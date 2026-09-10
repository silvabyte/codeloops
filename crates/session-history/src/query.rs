use crate::{
    Error, History, Result,
    artifacts::hash,
    model::{EXCERPT_CHARS, Filter, MAX_PAGE_SIZE, Page, Query},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rusqlite::{OptionalExtension, params, params_from_iter, types::Value as SqlValue};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Serialize, Deserialize)]
struct Cursor {
    query: String,
    after: i64,
    through: i64,
}

pub(crate) fn query(h: &History, q: Query) -> Result<Value> {
    match q {
        Query::Checkpoint { checkpoint_id } => crate::checkpoints::get(h, &checkpoint_id),
        Query::Compare {
            before,
            after,
            before_layer,
            after_layer,
            page,
        } => crate::checkpoints::compare(h, &before, &after, before_layer, after_layer, page),
        Query::File {
            checkpoint_id,
            path,
            layer,
            offset,
            limit,
        } => crate::checkpoints::file(h, &checkpoint_id, &path, layer, offset, limit),
        Query::Changes {
            session_id,
            entry_id,
            workspace_id,
            page,
        } => changes(h, session_id, entry_id, &workspace_id, page),
        Query::Artifact {
            hash,
            offset,
            limit,
        } => {
            if limit == 0 || limit > 65536 {
                return Err(Error::Invalid("artifact limit must be 1..65536".into()));
            }
            let bytes = h.artifacts.get(&hash)?;
            if offset > bytes.len() {
                return Err(Error::Invalid("artifact offset exceeds length".into()));
            }
            let end = offset.saturating_add(limit).min(bytes.len());
            let next_offset = if end < bytes.len() { Some(end) } else { None };
            Ok(json!({
                "hash": hash,
                "encoding": "base64",
                "data": base64::engine::general_purpose::STANDARD.encode(&bytes[offset..end]),
                "offset": offset,
                "total_bytes": bytes.len(),
                "next_offset": next_offset,
                "truncated": end < bytes.len(),
            }))
        }
        Query::Entry { entry_id } => {
            let value = h
                .connection
                .query_row(
                    &format!("{ENTRY_SELECT} WHERE e.id=?"),
                    [&entry_id],
                    entry_row,
                )
                .optional()?;
            value.ok_or(Error::Unknown(entry_id))
        }
        Query::Show { session_id, page } => {
            ensure_session(h, &session_id)?;
            entries(
                h,
                None,
                Filter {
                    session_id: Some(session_id),
                    ..Default::default()
                },
                page,
            )
        }
        Query::Search { text, filter, page } => {
            if text.trim().is_empty() || text.len() > 1024 {
                return Err(Error::Invalid("search text must be 1..1024 bytes".into()));
            }
            entries(h, Some(text), filter, page)
        }
        Query::List { filter, page } => sessions(h, filter, page),
        Query::Captures { session_id, page } => captures(h, session_id, page),
    }
}

fn ensure_session(h: &History, id: &str) -> Result<()> {
    if !h.connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sessions WHERE id=?)",
        [id],
        |r| r.get::<_, bool>(0),
    )? {
        return Err(Error::Unknown(id.into()));
    }
    Ok(())
}

fn bounds(h: &History, table: &str, page: &Page, identity: &Value) -> Result<Cursor> {
    if page.limit == 0 || page.limit > MAX_PAGE_SIZE {
        return Err(Error::Invalid("page limit must be 1..100".into()));
    }
    let fingerprint = hash(&serde_json::to_vec(identity)?);
    if let Some(cursor) = &page.cursor {
        if cursor.len() > 1024 {
            return Err(Error::Invalid("oversized cursor".into()));
        }
        let parse = || -> Option<Cursor> {
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(cursor).ok()?).ok()
        };
        let c = parse().ok_or_else(|| Error::Invalid("invalid cursor".into()))?;
        if c.query != fingerprint || c.after < 0 || c.through < c.after {
            return Err(Error::Invalid("cursor does not match query".into()));
        }
        return Ok(c);
    }
    let through = h.connection.query_row(
        &format!("SELECT COALESCE(MAX(ordinal),0) FROM {table}"),
        [],
        |r| r.get(0),
    )?;
    Ok(Cursor {
        query: fingerprint,
        after: 0,
        through,
    })
}

fn page_result(mut rows: Vec<(i64, Value)>, page: &Page, mut cursor: Cursor) -> Result<Value> {
    let has_more = rows.len() > page.limit;
    rows.truncate(page.limit);
    let next = if has_more {
        cursor.after = rows
            .last()
            .map(|(ordinal, _)| *ordinal)
            .unwrap_or(cursor.after);
        Some(URL_SAFE_NO_PAD.encode(serde_json::to_vec(&cursor)?))
    } else {
        None
    };
    Ok(json!({
        "items": rows.into_iter().map(|(_, value)| value).collect::<Vec<_>>(),
        "limit": page.limit,
        "next_cursor": next,
        "has_more": has_more,
        "order": "receipt",
        "snapshot": "membership_bounded_current_projection",
    }))
}

const ENTRY_SELECT: &str = "
    SELECT e.id, e.session, e.native, e.kind, e.role, e.parent_native,
           e.removed, e.observed, e.text, e.content_hash, e.ordinal,
           e.tool, e.checkpoints
    FROM entries e
    JOIN sessions s ON s.id = e.session
";

fn entry_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Value> {
    let text: String = r.get(8)?;
    let excerpt: String = text.chars().take(EXCERPT_CHARS).collect();
    Ok(json!({
        "id": r.get::<_, String>(0)?,
        "session_id": r.get::<_, String>(1)?,
        "native_id": r.get::<_, String>(2)?,
        "kind": r.get::<_, String>(3)?,
        "role": r.get::<_, String>(4)?,
        "parent_native_id": r.get::<_, Option<String>>(5)?,
        "removed": r.get::<_, bool>(6)?,
        "observed_at": r.get::<_, u64>(7)?,
        "text": excerpt,
        "text_bytes": text.len(),
        "truncated": excerpt.len() < text.len(),
        "content_hash": r.get::<_, Option<String>>(9)?,
        "checkpoints": serde_json::from_str::<Value>(&r.get::<_, String>(12)?)
            .unwrap_or(json!([])),
        "tool": r.get::<_, Option<String>>(11)?
            .and_then(|stored| serde_json::from_str::<Value>(&stored).ok()),
        "coverage": {
            "attachments": "metadata_only",
            "tools": "source_exposed",
        },
    }))
}

fn filters(
    filter: &Filter,
    sql: &mut String,
    values: &mut Vec<SqlValue>,
    entries: bool,
) -> Result<()> {
    for (column, value) in [
        ("s.project", &filter.project_id),
        ("s.source", &filter.source),
        ("s.device", &filter.device_id),
        ("s.id", &filter.session_id),
    ] {
        if let Some(value) = value {
            sql.push_str(&format!(" AND {column}=?"));
            values.push(value.clone().into());
        }
    }
    if entries {
        for (column, value) in [("e.role", &filter.role), ("e.kind", &filter.kind)] {
            if let Some(value) = value {
                sql.push_str(&format!(" AND {column}=?"));
                values.push(value.clone().into());
            }
        }
    } else if filter.role.is_some() || filter.kind.is_some() {
        return Err(Error::Invalid(
            "role and kind filters apply to entry search".into(),
        ));
    }
    let column = if entries { "e.observed" } else { "s.observed" };
    for (op, value) in [(">=", filter.since), ("<", filter.until)] {
        if let Some(value) = value {
            let value = i64::try_from(value)
                .map_err(|_| Error::Invalid("timestamp out of range".into()))?;
            sql.push_str(&format!(" AND {column}{op}?"));
            values.push(value.into());
        }
    }
    Ok(())
}

fn entries(h: &History, text: Option<String>, filter: Filter, page: Page) -> Result<Value> {
    let cursor = bounds(h, "entries", &page, &json!(["entries", text, filter]))?;
    let mut sql = format!("{ENTRY_SELECT} WHERE e.ordinal>? AND e.ordinal<=?");
    let mut values = vec![cursor.after.into(), cursor.through.into()];
    filters(&filter, &mut sql, &mut values, true)?;
    if let Some(text) = text {
        sql.push_str(" AND e.id IN (SELECT entry FROM search_index WHERE search_index MATCH ?)");
        // Literal phrase search: user input cannot become FTS syntax.
        values.push(format!("\"{}\"", text.replace('"', "\"\"")).into());
    }
    sql.push_str(" ORDER BY e.ordinal LIMIT ?");
    values.push(((page.limit + 1) as i64).into());
    let rows = h
        .connection
        .prepare(&sql)?
        .query_map(params_from_iter(values), |r| {
            Ok((r.get(10)?, entry_row(r)?))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    page_result(rows, &page, cursor)
}

fn sessions(h: &History, filter: Filter, page: Page) -> Result<Value> {
    let cursor = bounds(h, "sessions", &page, &json!(["sessions", filter]))?;
    let mut sql = String::from(
        "SELECT s.ordinal, s.id, s.native, s.source, s.device, s.installation,
                s.project, s.workspace, s.source_version, s.title, s.state,
                s.parent_native, s.observed
         FROM sessions s
         WHERE s.ordinal > ? AND s.ordinal <= ?",
    );
    let mut values = vec![cursor.after.into(), cursor.through.into()];
    filters(&filter, &mut sql, &mut values, false)?;
    sql.push_str(" ORDER BY s.ordinal LIMIT ?");
    values.push(((page.limit + 1) as i64).into());
    let mut rows = h
        .connection
        .prepare(&sql)?
        .query_map(params_from_iter(values), |r| {
            Ok((
                r.get(0)?,
                json!({
                    "id": r.get::<_, String>(1)?,
                    "native_id": r.get::<_, String>(2)?,
                    "source": r.get::<_, String>(3)?,
                    "device_id": r.get::<_, String>(4)?,
                    "installation_id": r.get::<_, String>(5)?,
                    "project_id": r.get::<_, String>(6)?,
                    "workspace_id": r.get::<_, String>(7)?,
                    "source_version": r.get::<_, String>(8)?,
                    "title": r.get::<_, String>(9)?,
                    "state": r.get::<_, String>(10)?,
                    "parent_native_id": r.get::<_, Option<String>>(11)?,
                    "observed_at": r.get::<_, u64>(12)?,
                }),
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut workspaces = h
        .connection
        .prepare("SELECT workspace FROM session_workspaces WHERE session=? ORDER BY workspace")?;
    for (_, session) in &mut rows {
        let id = session["id"]
            .as_str()
            .ok_or_else(|| Error::Invalid("invalid stored session ID".into()))?;
        let ids = workspaces
            .query_map([id], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        session["workspace_ids"] = serde_json::to_value(ids)?;
    }
    page_result(rows, &page, cursor)
}

fn captures(h: &History, session: String, page: Page) -> Result<Value> {
    ensure_session(h, &session)?;
    let cursor = bounds(h, "captures", &page, &json!(["captures", session]))?;
    let rows = h
        .connection
        .prepare(
            "SELECT ordinal, id, entry, envelope_hash, recorded, payload_hash, checkpoints
             FROM captures
             WHERE session = ? AND ordinal > ? AND ordinal <= ?
             ORDER BY ordinal
             LIMIT ?",
        )?
        .query_map(
            params![
                session,
                cursor.after,
                cursor.through,
                (page.limit + 1) as i64
            ],
            |r| {
                Ok((
                    r.get(0)?,
                    json!({
                        "id": r.get::<_, String>(1)?,
                        "entry_id": r.get::<_, Option<String>>(2)?,
                        "envelope_hash": r.get::<_, String>(3)?,
                        "recorded_at": r.get::<_, u64>(4)?,
                        "source_payload_hash": r.get::<_, String>(5)?,
                        "checkpoints": serde_json::from_str::<Value>(&r.get::<_, String>(6)?)
                            .unwrap_or(json!([])),
                    }),
                ))
            },
        )?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    page_result(rows, &page, cursor)
}

fn changes(
    h: &History,
    session: Option<String>,
    entry: Option<String>,
    workspace: &str,
    page: Page,
) -> Result<Value> {
    if page.limit == 0 || page.limit > MAX_PAGE_SIZE {
        return Err(Error::Invalid("page limit must be 1..100".into()));
    }
    use crate::model::{CheckpointLink, Layer};
    let (sql, id, event) = match (&session, &entry) {
        (Some(id), None) => {
            ensure_session(h, id)?;
            (
                "SELECT checkpoints FROM captures WHERE session=? ORDER BY ordinal DESC",
                id,
                false,
            )
        }
        (None, Some(id)) => {
            if !h.connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM entries WHERE id=?)",
                [id],
                |r| r.get::<_, bool>(0),
            )? {
                return Err(Error::Unknown(id.clone()));
            }
            (
                "SELECT checkpoints FROM captures WHERE entry=? ORDER BY ordinal DESC",
                id,
                true,
            )
        }
        _ => {
            return Err(Error::Invalid(
                "choose exactly one of session_id or entry_id".into(),
            ));
        }
    };
    let mut stmt = h.connection.prepare(sql)?;
    let mut rows = stmt.query([id])?;
    while let Some(row) = rows.next()? {
        let links: Vec<CheckpointLink> = serde_json::from_str(&row.get::<_, String>(0)?)?;
        if let Some(link) = links.into_iter().find(|l| l.workspace_id == workspace) {
            if event && link.status == "reused" {
                continue;
            }
            let before = if event {
                &link.before_id
            } else {
                &link.baseline_id
            };
            if let (Some(before), Some(after)) = (before, &link.checkpoint_id) {
                let mut result = crate::checkpoints::compare(
                    h,
                    before,
                    after,
                    Layer::Worktree,
                    Layer::Worktree,
                    page,
                )?;
                result["boundary"] = serde_json::to_value(link)?;
                return Ok(result);
            }
            return Ok(json!({
                "status": "unavailable",
                "boundary": link,
                "items": [],
            }));
        }
    }
    Ok(json!({"status": "not_captured", "items": []}))
}
