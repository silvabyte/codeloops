//! Native event translation and the adapter-owned durable outbox.
use crate::{
    AppResult,
    config::Config,
    outbox::{self, identity},
};
use rusqlite::{OptionalExtension, params};
use serde::Deserialize;
use serde_json::{Value, json};
use session_history::{
    model::{Capture, Change, MAX_CAPTURE_BYTES, Origin},
    now_ms,
};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct NativeInput {
    #[serde(default = "now_ms")]
    pub observed_at: u64,
    pub source_version: String,
    pub directory: String,
    pub project: String,
    pub event: Value,
}

fn string<'a>(v: &'a Value, key: &str) -> AppResult<&'a str> {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("missing {key} in native event").into())
}

pub fn enqueue(config: &Config, input: NativeInput) -> AppResult<Value> {
    let mut db = outbox::open(config)?;
    let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let event_type = string(&input.event, "type")?;
    let p = &input.event["properties"];
    let (session, change, occurred_at) = match event_type {
        "history.prompt" => (
            string(p, "sessionID")?.into(),
            Change::Lifecycle {
                state: Some("active".into()),
                title: None,
                parent_native_id: None,
            },
            None,
        ),
        "history.tool.before" | "history.tool.after" => (
            string(p, "sessionID")?.into(),
            Change::Tool {
                native_id: string(p, "callID")?.into(),
                name: string(p, "tool")?.into(),
                parent_native_id: None,
                status: if event_type == "history.tool.before" {
                    "running"
                } else {
                    "completed"
                }
                .into(),
                input: p.get("args").cloned(),
                output: p.get("output").cloned(),
                error: None,
            },
            None,
        ),
        "message.updated" => {
            let info = &p["info"];
            (
                string(info, "sessionID")?.to_string(),
                Change::Message {
                    native_id: string(info, "id")?.into(),
                    role: string(info, "role")?.into(),
                    parent_native_id: info["parentID"].as_str().map(String::from),
                    removed: false,
                },
                info["time"]["created"].as_u64(),
            )
        }
        "message.removed" => (
            string(p, "sessionID")?.into(),
            Change::Message {
                native_id: string(p, "messageID")?.into(),
                role: "unknown".into(),
                parent_native_id: None,
                removed: true,
            },
            None,
        ),
        "message.part.updated" => {
            let part = &p["part"];
            let session = string(part, "sessionID")?;
            let message = string(part, "messageID")?;
            let native = string(part, "id")?;
            let kind = string(part, "type")?;
            let text = part["text"].as_str().unwrap_or("");
            tx.execute("INSERT INTO parts VALUES(?,?,?,?,?) ON CONFLICT(session,message,part) DO UPDATE SET kind=excluded.kind,text=excluded.text",params![session,message,native,kind,text])?;
            (
                session.into(),
                if kind == "tool" {
                    Change::Tool {
                        native_id: string(part, "callID")?.into(),
                        name: string(part, "tool")?.into(),
                        parent_native_id: Some(message.into()),
                        status: match part["state"]["status"].as_str() {
                            Some("pending") => "pending",
                            Some("running") => "running",
                            Some("completed") => "completed",
                            Some("error") => "failed",
                            _ => "unknown",
                        }
                        .into(),
                        input: part["state"].get("input").cloned(),
                        output: part["state"].get("output").cloned(),
                        error: part["state"].get("error").cloned(),
                    }
                } else {
                    Change::Part {
                        message_id: message.into(),
                        native_id: native.into(),
                        kind: kind.into(),
                        text: text.into(),
                        removed: false,
                    }
                },
                part["time"]["start"].as_u64(),
            )
        }
        "message.part.delta" => {
            let session = string(p, "sessionID")?;
            let message = string(p, "messageID")?;
            let native = string(p, "partID")?;
            if string(p, "field")? != "text" {
                return Err("unsupported delta field".into());
            }
            let previous: Option<(String, String)> = tx
                .query_row(
                    "SELECT kind,text FROM parts WHERE session=? AND message=? AND part=?",
                    params![session, message, native],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?;
            let (kind, mut text) =
                previous.ok_or("delta arrived without a captured part baseline")?;
            text.push_str(string(p, "delta")?);
            if text.len() > MAX_CAPTURE_BYTES {
                return Err("part exceeds 4 MiB".into());
            }
            tx.execute(
                "UPDATE parts SET text=? WHERE session=? AND message=? AND part=?",
                params![text, session, message, native],
            )?;
            (
                session.into(),
                Change::Part {
                    message_id: message.into(),
                    native_id: native.into(),
                    kind,
                    text,
                    removed: false,
                },
                None,
            )
        }
        "message.part.removed" => (
            string(p, "sessionID")?.into(),
            Change::Part {
                message_id: string(p, "messageID")?.into(),
                native_id: string(p, "partID")?.into(),
                kind: "removed".into(),
                text: String::new(),
                removed: true,
            },
            None,
        ),
        name if name.starts_with("session.") => {
            let info = &p["info"];
            let session = p["sessionID"]
                .as_str()
                .or_else(|| info["id"].as_str())
                .ok_or("session event has no session identity")?;
            let state = match name {
                "session.created" => "active",
                "session.idle" => "idle",
                "session.error" => "interrupted",
                "session.status" => match p["status"]["type"].as_str() {
                    Some("busy" | "retry") => "active",
                    Some("idle") => "idle",
                    _ => "unknown",
                },
                _ => "unknown",
            };
            // Deleted/compacted events are observations, not proof of successful completion.
            (
                session.into(),
                Change::Lifecycle {
                    state: if state == "unknown" {
                        None
                    } else {
                        Some(state.into())
                    },
                    title: info["title"].as_str().map(String::from),
                    parent_native_id: info["parentID"].as_str().map(String::from),
                },
                info["time"]["updated"].as_u64(),
            )
        }
        _ => return Err(format!("unsupported source event: {event_type}").into()),
    };
    let sequence = outbox::sequence(&tx)?;
    let workspace_id = identity(&tx, &format!("workspace:{}", input.directory))?;
    use crate::boundary::{self, Boundary};
    let boundary = match event_type {
        "history.prompt" => Boundary::Prompt,
        "history.tool.before" => Boundary::Before(string(p, "callID")?),
        "history.tool.after" => Boundary::After {
            call: string(p, "callID")?,
            late: false,
        },
        "message.part.updated"
            if p["part"]["type"] == "tool" && p["part"]["state"]["status"] == "error" =>
        {
            Boundary::After {
                call: string(&p["part"], "callID")?,
                late: true,
            }
        }
        "session.idle" | "session.error" => Boundary::Turn,
        _ => Boundary::Reuse,
    };
    let checkpoint = boundary::observe(
        config,
        &tx,
        "opencode",
        &session,
        &workspace_id,
        Some(&input.directory),
        boundary,
    )?;
    let capture = Capture {
        schema_version: 1,
        delivery_id: Uuid::new_v4().to_string(),
        origin: Origin {
            device_id: identity(&tx, "device")?,
            installation_id: identity(&tx, "opencode")?,
            source: "opencode".into(),
            source_version: input.source_version,
        },
        sequence,
        native_session_id: session,
        // Native project IDs are hints scoped to this installation. Non-Git projects use their full workspace path.
        project_id: identity(
            &tx,
            &format!(
                "project:{}",
                if input.project == "global" {
                    &input.directory
                } else {
                    &input.project
                }
            ),
        )?,
        workspace_id,
        observed_at: input.observed_at,
        occurred_at,
        change,
        source_payload: input.event,
        checkpoints: vec![checkpoint],
    };
    outbox::queue(&tx, &capture)?;
    tx.commit()?;
    Ok(json!({"queued":true,"delivery_id":capture.delivery_id}))
}
