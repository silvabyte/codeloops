use crate::{
    Error, History, Result, artifacts,
    model::{Capture, Change, MAX_CAPTURE_BYTES, Receipt},
    now_ms,
};
use rusqlite::{OptionalExtension, Transaction, params};
use uuid::Uuid;

fn validate(c: &Capture) -> Result<()> {
    if c.schema_version != 1
        || c.sequence == 0
        || c.sequence > i64::MAX as u64
        || c.observed_at > i64::MAX as u64
        || c.occurred_at.is_some_and(|t| t > i64::MAX as u64)
    {
        return Err(Error::Invalid(
            "unsupported schema, sequence, or timestamp".into(),
        ));
    }
    for id in [
        &c.delivery_id,
        &c.origin.device_id,
        &c.origin.installation_id,
        &c.project_id,
        &c.workspace_id,
    ] {
        let canonical = Uuid::parse_str(id)
            .map_err(|_| Error::Invalid("archive identities must be UUIDs".into()))?
            .to_string();
        if &canonical != id {
            return Err(Error::Invalid(
                "archive identities must be canonical UUIDs".into(),
            ));
        }
    }
    for field in [
        &c.native_session_id,
        &c.origin.source,
        &c.origin.source_version,
    ] {
        if field.is_empty() || field.len() > 1024 {
            return Err(Error::Invalid("empty or oversized source identity".into()));
        }
    }
    match &c.change {
        Change::Tool {
            native_id,
            name,
            status,
            ..
        } if native_id.is_empty()
            || name.is_empty()
            || name.len() > 1024
            || !["pending", "running", "completed", "failed", "unknown"]
                .contains(&status.as_str()) =>
        {
            return Err(Error::Invalid("invalid tool identity or status".into()));
        }
        Change::Message {
            native_id, role, ..
        } if native_id.is_empty()
            || !["user", "assistant", "system", "unknown"].contains(&role.as_str()) =>
        {
            return Err(Error::Invalid("invalid message identity or role".into()));
        }
        Change::Part {
            message_id,
            native_id,
            ..
        } if message_id.is_empty() || native_id.is_empty() => {
            return Err(Error::Invalid("empty part identity".into()));
        }
        Change::Lifecycle { state, .. }
            if state.as_ref().is_some_and(|state| {
                !["active", "idle", "ended", "interrupted", "unknown"].contains(&state.as_str())
            }) =>
        {
            return Err(Error::Invalid("invalid lifecycle state".into()));
        }
        _ => {}
    }
    let (native, parent) = match &c.change {
        Change::Tool {
            native_id,
            parent_native_id,
            ..
        } => (native_id, parent_native_id.as_deref()),
        Change::Message {
            native_id,
            parent_native_id,
            ..
        } => (native_id, parent_native_id.as_deref()),
        Change::Part {
            native_id,
            message_id,
            ..
        } => (native_id, Some(message_id.as_str())),
        Change::Lifecycle {
            title,
            parent_native_id,
            ..
        } => {
            if title.as_ref().is_some_and(|t| t.len() > 4096) {
                return Err(Error::Invalid("session title exceeds 4096 bytes".into()));
            }
            (&c.native_session_id, parent_native_id.as_deref())
        }
    };
    if native.len() > 1024 || parent.is_some_and(|p| p.len() > 1024) {
        return Err(Error::Invalid("source identity exceeds 1024 bytes".into()));
    }
    Ok(())
}

pub(crate) fn ingest(history: &mut History, c: Capture) -> Result<Receipt> {
    validate(&c)?;
    let bytes = serde_json::to_vec(&c)?;
    if bytes.len() > MAX_CAPTURE_BYTES {
        return Err(Error::Invalid("capture exceeds 4 MiB".into()));
    }
    let hash = artifacts::hash(&bytes);
    let tx = history
        .connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let existing: Option<(String, String)> = tx
        .query_row(
            "SELECT hash,receipt FROM captures WHERE delivery=?",
            [&c.delivery_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if let Some((old_hash, receipt)) = existing {
        if hash != old_hash {
            return Err(Error::Conflict(c.delivery_id));
        }
        return Ok(serde_json::from_str(&receipt)?);
    }
    if c.checkpoints.len() > 32 {
        return Err(Error::Invalid("too many checkpoint links".into()));
    }
    for link in &c.checkpoints {
        for id in [&link.checkpoint_id, &link.before_id, &link.baseline_id]
            .into_iter()
            .flatten()
        {
            let workspace: Option<String> = tx
                .query_row("SELECT workspace FROM checkpoints WHERE id=?", [id], |r| {
                    r.get(0)
                })
                .optional()?;
            if workspace.as_deref() != Some(&link.workspace_id) {
                return Err(Error::Invalid(
                    "checkpoint workspace missing or mismatched".into(),
                ));
            }
        }
    }
    // Publish and fsync bytes before the transaction can reference them.
    let payload_hash = history
        .artifacts
        .put(&serde_json::to_vec(&c.source_payload)?)?;
    let serde_json::Value::Object(mut stored) = serde_json::to_value(&c)? else {
        return Err(Error::Invalid("capture must be an object".into()));
    };
    stored.remove("source_payload");
    stored.insert("source_payload_hash".into(), payload_hash.clone().into());
    let envelope_hash = history.artifacts.put(&serde_json::to_vec(&stored)?)?;
    let proposed_session = Uuid::new_v4().to_string();
    tx.execute("INSERT OR IGNORE INTO sessions(id,device,installation,source,native,project,workspace,source_version,observed) VALUES(?,?,?,?,?,?,?,?,?)",
        params![proposed_session, c.origin.device_id, c.origin.installation_id, c.origin.source,
            c.native_session_id, c.project_id, c.workspace_id, c.origin.source_version, c.observed_at])?;
    let session: String = tx.query_row(
        "SELECT id FROM sessions WHERE device=? AND installation=? AND source=? AND native=?",
        params![
            c.origin.device_id,
            c.origin.installation_id,
            c.origin.source,
            c.native_session_id
        ],
        |r| r.get(0),
    )?;
    let (native, kind) = match &c.change {
        Change::Tool { native_id, .. } => (native_id.as_str(), "tool"),
        Change::Message { native_id, .. } => (native_id.as_str(), "message"),
        Change::Part { message_id, .. } => (message_id.as_str(), "message"),
        Change::Lifecycle { .. } => (c.delivery_id.as_str(), "lifecycle"),
    };
    tx.execute(
        "INSERT OR IGNORE INTO session_workspaces(session,workspace) VALUES(?,?)",
        params![session, c.workspace_id],
    )?;
    tx.execute(
        "INSERT OR IGNORE INTO entries(id,session,native,kind,observed) VALUES(?,?,?,?,?)",
        params![
            Uuid::new_v4().to_string(),
            session,
            native,
            kind,
            c.observed_at
        ],
    )?;
    let entry: String = tx.query_row(
        "SELECT id FROM entries WHERE session=? AND native=? AND kind=?",
        params![session, native, kind],
        |r| r.get(0),
    )?;
    match &c.change {
        Change::Tool {
            name,
            parent_native_id,
            status,
            input,
            output,
            error,
            ..
        } => {
            let mut tool: serde_json::Value = tx
                .query_row("SELECT tool FROM entries WHERE id=?", [&entry], |r| {
                    r.get::<_, Option<String>>(0)
                })?
                .map(|s| serde_json::from_str(&s))
                .transpose()?
                .unwrap_or_else(|| serde_json::json!({}));
            tool["name"] = name.clone().into();
            tool["status"] = status.clone().into();
            for (key, value) in [
                ("input_hash", input),
                ("output_hash", output),
                ("error_hash", error),
            ] {
                if let Some(value) = value {
                    tool[key] = history.artifacts.put(&serde_json::to_vec(value)?)?.into();
                }
            }
            let text = format!(
                "{name} {status}\n{}\n{}\n{}",
                input.as_ref().map(ToString::to_string).unwrap_or_default(),
                output.as_ref().map(ToString::to_string).unwrap_or_default(),
                error.as_ref().map(ToString::to_string).unwrap_or_default()
            );
            let content_hash = history.artifacts.put(text.as_bytes())?;
            tx.execute("UPDATE entries SET role='assistant',parent_native=COALESCE(?,parent_native),tool=?,text=?,content_hash=?,sequence=? WHERE id=? AND sequence<?",
                params![parent_native_id,serde_json::to_string(&tool)?,text,content_hash,c.sequence,entry,c.sequence])?;
        }
        Change::Message {
            role,
            parent_native_id,
            removed,
            ..
        } => {
            tx.execute("UPDATE entries SET role=?,parent_native=?,removed=?,sequence=? WHERE id=? AND sequence<?",
                params![role, parent_native_id, removed, c.sequence, entry, c.sequence])?;
        }
        Change::Part {
            native_id,
            kind,
            text,
            removed,
            ..
        } => {
            tx.execute("INSERT INTO parts(entry,native,kind,text,removed,sequence) VALUES(?,?,?,?,?,?) ON CONFLICT(entry,native) DO UPDATE SET kind=excluded.kind,text=excluded.text,removed=excluded.removed,sequence=excluded.sequence WHERE parts.sequence<excluded.sequence",
                params![entry, native_id, kind, text, removed, c.sequence])?;
        }
        Change::Lifecycle {
            state,
            title,
            parent_native_id,
        } => {
            if let Some(state) = state {
                tx.execute(
                    "UPDATE sessions SET state=?,state_sequence=? WHERE id=? AND state_sequence<?",
                    params![state, c.sequence, session, c.sequence],
                )?;
            }
            if title.is_some() || parent_native_id.is_some() {
                tx.execute("UPDATE sessions SET title=COALESCE(?,title),parent_native=COALESCE(?,parent_native),sequence=?,source_version=? WHERE id=? AND sequence<?",
                    params![title, parent_native_id, c.sequence, c.origin.source_version, session, c.sequence])?;
            }
            tx.execute(
                "UPDATE entries SET role='system',text=?,sequence=? WHERE id=?",
                params![state.as_deref().unwrap_or("metadata"), c.sequence, entry],
            )?;
        }
    }
    project(&tx, &history.artifacts, &entry, kind)?;
    if !c.checkpoints.is_empty() {
        tx.execute("UPDATE entries SET checkpoints=?,checkpoint_sequence=? WHERE id=? AND checkpoint_sequence<?", params![serde_json::to_string(&c.checkpoints)?,c.sequence,entry,c.sequence])?;
    }
    let receipt = Receipt {
        capture_id: Uuid::new_v4().to_string(),
        session_id: session,
        entry_id: Some(entry),
        recorded_at: now_ms(),
    };
    tx.execute("INSERT INTO captures(id,delivery,hash,envelope_hash,payload_hash,session,entry,recorded,receipt) VALUES(?,?,?,?,?,?,?,?,?)",
        params![receipt.capture_id, c.delivery_id, hash, envelope_hash, payload_hash, receipt.session_id, receipt.entry_id, receipt.recorded_at, serde_json::to_string(&receipt)?])?;
    tx.execute(
        "UPDATE captures SET checkpoints=? WHERE id=?",
        params![serde_json::to_string(&c.checkpoints)?, receipt.capture_id],
    )?;
    tx.commit()?;
    Ok(receipt)
}

fn project(
    tx: &Transaction<'_>,
    artifacts: &artifacts::Artifacts,
    entry: &str,
    kind: &str,
) -> Result<()> {
    if kind == "message" {
        let mut statement = tx.prepare("SELECT text FROM parts WHERE entry=? AND removed=0 AND kind IN ('text','reasoning') ORDER BY native")?;
        let texts = statement
            .query_map([entry], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let text = texts.join("\n");
        if text.len() > MAX_CAPTURE_BYTES {
            return Err(Error::Invalid("message projection exceeds 4 MiB".into()));
        }
        let content_hash = artifacts.put(text.as_bytes())?;
        tx.execute(
            "UPDATE entries SET text=?,content_hash=? WHERE id=?",
            params![text, content_hash, entry],
        )?;
    }
    tx.execute("DELETE FROM search_index WHERE entry=?", [entry])?;
    tx.execute(
        "INSERT INTO search_index(entry,text) SELECT id,text FROM entries WHERE id=? AND removed=0",
        [entry],
    )?;
    Ok(())
}
