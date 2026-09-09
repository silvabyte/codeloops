//! Native event translation and the adapter-owned durable outbox.
use crate::{AppResult, config::Config};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::Deserialize;
use serde_json::{Value, json};
use session_history::{
    History,
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

fn open(config: &Config) -> AppResult<Connection> {
    let db = Connection::open(config.root.join("opencode-spool.sqlite3"))?;
    db.busy_timeout(std::time::Duration::from_secs(10))?;
    db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
      CREATE TABLE IF NOT EXISTS identities(key TEXT PRIMARY KEY,id TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS counter(value INTEGER NOT NULL);
      INSERT INTO counter SELECT 0 WHERE NOT EXISTS(SELECT 1 FROM counter);
      CREATE TABLE IF NOT EXISTS parts(session TEXT,message TEXT,part TEXT,kind TEXT,text TEXT,PRIMARY KEY(session,message,part));
      CREATE TABLE IF NOT EXISTS queue(sequence INTEGER PRIMARY KEY,envelope TEXT NOT NULL,error TEXT);
      CREATE TABLE IF NOT EXISTS health(id INTEGER PRIMARY KEY CHECK(id=1),delivered INTEGER NOT NULL DEFAULT 0,last_error TEXT);
      CREATE TABLE IF NOT EXISTS failures(id INTEGER PRIMARY KEY AUTOINCREMENT,observed INTEGER NOT NULL,message TEXT NOT NULL);
      INSERT OR IGNORE INTO health(id) VALUES(1);")?;
    Ok(db)
}

fn identity(tx: &Transaction<'_>, key: &str) -> AppResult<String> {
    tx.execute(
        "INSERT OR IGNORE INTO identities(key,id) VALUES(?,?)",
        params![key, Uuid::new_v4().to_string()],
    )?;
    Ok(tx.query_row("SELECT id FROM identities WHERE key=?", [key], |r| r.get(0))?)
}

fn string<'a>(v: &'a Value, key: &str) -> AppResult<&'a str> {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("missing {key} in native event").into())
}

pub fn enqueue(config: &Config, input: NativeInput) -> AppResult<Value> {
    let mut db = open(config)?;
    let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let event_type = string(&input.event, "type")?;
    let p = &input.event["properties"];
    let (session, change, occurred_at) = match event_type {
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
                Change::Part {
                    message_id: message.into(),
                    native_id: native.into(),
                    kind: kind.into(),
                    text: text.into(),
                    removed: false,
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
    let sequence: u64 = tx.query_row(
        "UPDATE counter SET value=value+1 RETURNING value",
        [],
        |r| r.get(0),
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
        workspace_id: identity(&tx, &format!("workspace:{}", input.directory))?,
        observed_at: input.observed_at,
        occurred_at,
        change,
        source_payload: input.event,
    };
    let envelope = serde_json::to_string(&capture)?;
    if envelope.len() > MAX_CAPTURE_BYTES {
        return Err("capture exceeds 4 MiB".into());
    }
    tx.execute(
        "INSERT INTO queue(sequence,envelope) VALUES(?,?)",
        params![sequence, envelope],
    )?;
    tx.commit()?;
    Ok(json!({"queued":true,"delivery_id":capture.delivery_id}))
}

pub fn flush(config: &Config) -> AppResult<()> {
    // One SQLite write lock serializes flushers and producers. Crash after archive commit
    // but before queue deletion is recovered by the archive's delivery-id check.
    let mut db = open(config)?;
    let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let queued = tx
        .prepare(
            "SELECT sequence,envelope FROM queue WHERE error IS NULL ORDER BY sequence LIMIT 100",
        )?
        .query_map([], |r| Ok((r.get::<_, u64>(0)?, r.get::<_, String>(1)?)))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if queued.is_empty() {
        return Ok(());
    }
    let mut history = History::open(config.archive())?;
    for (sequence, envelope) in queued {
        let capture: Capture = serde_json::from_str(&envelope)?;
        match history.ingest(capture) {
            Ok(_) => {
                tx.execute("DELETE FROM queue WHERE sequence=?", [sequence])?;
                tx.execute("UPDATE health SET delivered=delivered+1 WHERE id=1", [])?;
            }
            Err(error) => {
                let message = error.to_string();
                tx.execute("UPDATE health SET last_error=? WHERE id=1", [&message])?;
                if matches!(
                    error,
                    session_history::Error::Invalid(_)
                        | session_history::Error::Conflict(_)
                        | session_history::Error::Json(_)
                ) {
                    tx.execute(
                        "UPDATE queue SET error=? WHERE sequence=?",
                        params![message, sequence],
                    )?;
                } else {
                    break;
                }
            }
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn health(config: &Config) -> AppResult<Value> {
    let db = open(config)?;
    let (pending, rejected): (u64, u64) =
        db.query_row("SELECT COUNT(*),COUNT(error) FROM queue", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })?;
    let (delivered, last_error): (u64, Option<String>) = db.query_row(
        "SELECT delivered,last_error FROM health WHERE id=1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let failures: u64 = db.query_row("SELECT COUNT(*) FROM failures", [], |r| r.get(0))?;
    Ok(
        json!({"source":"opencode","pending":pending,"rejected":rejected,"delivered":delivered,"last_error":last_error,"enqueue_failures":failures,"checkpoint_capture":"not_implemented","attachment_capture":"metadata_only"}),
    )
}

pub fn record_failure(config: &Config, error: &str) -> AppResult<()> {
    let mut db = open(config)?;
    let tx = db.transaction()?;
    tx.execute(
        "INSERT INTO failures(observed,message) VALUES(?,?)",
        params![now_ms(), error],
    )?;
    tx.execute("UPDATE health SET last_error=? WHERE id=1", [error])?;
    tx.commit()?;
    Ok(())
}
