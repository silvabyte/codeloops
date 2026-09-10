//! Shared local collector persistence. Keep the original filename and identity keys
//! so upgrading an OpenCode preview preserves its queued events and archive identity.
use crate::{AppResult, config::Config};
use rusqlite::{Connection, Transaction, params};
use serde_json::{Value, json};
use session_history::{
    History,
    model::{Capture, MAX_CAPTURE_BYTES},
    now_ms,
};
use uuid::Uuid;

const OPEN_RETRIES: usize = 100;
const OPEN_RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(10);

pub fn open(config: &Config) -> AppResult<Connection> {
    let path = config.root.join("opencode-spool.sqlite3");
    for attempt in 0..OPEN_RETRIES {
        match open_once(&path) {
            Ok(db) => return Ok(db),
            Err(error) if is_lock_contention(&error) && attempt + 1 < OPEN_RETRIES => {
                std::thread::sleep(OPEN_RETRY_DELAY);
            }
            Err(error) => return Err(error.into()),
        }
    }
    unreachable!("the outbox open loop always returns")
}

fn open_once(path: &std::path::Path) -> rusqlite::Result<Connection> {
    let db = Connection::open(path)?;
    db.busy_timeout(std::time::Duration::from_secs(10))?;
    db.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = FULL;

         CREATE TABLE IF NOT EXISTS identities(key TEXT PRIMARY KEY, id TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS counter(value INTEGER NOT NULL);
         INSERT INTO counter SELECT 0 WHERE NOT EXISTS(SELECT 1 FROM counter);

         CREATE TABLE IF NOT EXISTS parts(
             session TEXT, message TEXT, part TEXT, kind TEXT, text TEXT,
             PRIMARY KEY(session, message, part)
         );
         CREATE TABLE IF NOT EXISTS queue(
             sequence INTEGER PRIMARY KEY, envelope TEXT NOT NULL, error TEXT
         );
         CREATE TABLE IF NOT EXISTS health(
             id INTEGER PRIMARY KEY CHECK(id = 1),
             delivered INTEGER NOT NULL DEFAULT 0, last_error TEXT
         );
         CREATE TABLE IF NOT EXISTS failures(
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             observed INTEGER NOT NULL, message TEXT NOT NULL
         );
         INSERT OR IGNORE INTO health(id) VALUES(1);

         CREATE TABLE IF NOT EXISTS adapter_health(
             source TEXT PRIMARY KEY, delivered INTEGER NOT NULL,
             last_error TEXT, enqueue_failures INTEGER NOT NULL
         );
         INSERT OR IGNORE INTO adapter_health
             SELECT 'opencode', delivered, last_error, (SELECT COUNT(*) FROM failures)
             FROM health WHERE id = 1;
         INSERT OR IGNORE INTO adapter_health VALUES('cursor', 0, NULL, 0);

         CREATE TABLE IF NOT EXISTS cursor_prompts(
             session TEXT, generation TEXT, message TEXT NOT NULL,
             PRIMARY KEY(session, generation)
         );
         CREATE TABLE IF NOT EXISTS cursor_sessions(session TEXT PRIMARY KEY);
         CREATE TABLE IF NOT EXISTS snapshot_state(scope TEXT PRIMARY KEY, link TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS tool_boundaries(
             scope TEXT, call TEXT, checkpoint TEXT, PRIMARY KEY(scope, call)
         );
         CREATE TABLE IF NOT EXISTS tool_activity(
             scope TEXT, call TEXT, workspace TEXT, overlap INTEGER NOT NULL,
             PRIMARY KEY(scope, call)
         );",
    )?;
    Ok(db)
}

fn is_lock_contention(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(failure, _)
            if matches!(
                failure.code,
                rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
            )
    )
}

pub fn identity(tx: &Transaction<'_>, key: &str) -> AppResult<String> {
    tx.execute(
        "INSERT OR IGNORE INTO identities(key,id) VALUES(?,?)",
        params![key, Uuid::new_v4().to_string()],
    )?;
    Ok(tx.query_row("SELECT id FROM identities WHERE key=?", [key], |r| r.get(0))?)
}

pub fn sequence(tx: &Transaction<'_>) -> AppResult<u64> {
    Ok(tx.query_row(
        "UPDATE counter SET value=value+1 RETURNING value",
        [],
        |r| r.get(0),
    )?)
}

pub fn queue(tx: &Transaction<'_>, capture: &Capture) -> AppResult<()> {
    let envelope = serde_json::to_string(capture)?;
    if envelope.len() > MAX_CAPTURE_BYTES {
        return Err("capture exceeds 4 MiB".into());
    }
    tx.execute(
        "INSERT INTO queue(sequence,envelope) VALUES(?,?)",
        params![capture.sequence, envelope],
    )?;
    Ok(())
}

pub fn flush(config: &Config) -> AppResult<()> {
    // Crash after archive commit but before deletion replays the same delivery ID.
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
        let source = capture.origin.source.clone();
        match history.ingest(capture) {
            Ok(_) => {
                tx.execute("DELETE FROM queue WHERE sequence=?", [sequence])?;
                tx.execute("UPDATE health SET delivered=delivered+1 WHERE id=1", [])?;
                tx.execute(
                    "UPDATE adapter_health SET delivered=delivered+1 WHERE source=?",
                    [&source],
                )?;
            }
            Err(error) => {
                let message = error.to_string();
                tx.execute("UPDATE health SET last_error=? WHERE id=1", [&message])?;
                tx.execute(
                    "UPDATE adapter_health SET last_error=? WHERE source=?",
                    params![message, source],
                )?;
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
    let checkpoint_failures: u64 = db.query_row(
        "SELECT COUNT(*) FROM snapshot_state WHERE json_extract(link,'$.status')='failed'",
        [],
        |r| r.get(0),
    )?;
    let mut sources = serde_json::Map::new();
    for source in ["opencode", "cursor"] {
        let (delivered, error, failures): (u64, Option<String>, u64) = db.query_row(
            "SELECT delivered,last_error,enqueue_failures FROM adapter_health WHERE source=?",
            [source],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        let (pending, rejected): (u64, u64) = db.query_row(
            "SELECT COUNT(*), COUNT(error) FROM queue
             WHERE json_extract(envelope, '$.origin.source') = ?",
            [source],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        sources.insert(
            source.into(),
            json!({
                "pending": pending,
                "rejected": rejected,
                "delivered": delivered,
                "last_error": error,
                "enqueue_failures": failures,
            }),
        );
    }
    Ok(json!({
        "source": "all",
        "sources": sources,
        "pending": pending,
        "rejected": rejected,
        "delivered": delivered,
        "last_error": last_error,
        "enqueue_failures": failures,
        "checkpoint_capture": "boundary_observations",
        "workspaces_with_failed_checkpoint": checkpoint_failures,
        "attachment_capture": "metadata_only",
    }))
}

pub fn record_failure(config: &Config, source: &str, error: &str) -> AppResult<()> {
    let mut db = open(config)?;
    let tx = db.transaction()?;
    tx.execute(
        "INSERT INTO failures(observed,message) VALUES(?,?)",
        params![now_ms(), error],
    )?;
    tx.execute("UPDATE health SET last_error=? WHERE id=1", [error])?;
    tx.execute(
        "UPDATE adapter_health SET last_error=?,enqueue_failures=enqueue_failures+1 WHERE source=?",
        params![error, source],
    )?;
    tx.commit()?;
    Ok(())
}
