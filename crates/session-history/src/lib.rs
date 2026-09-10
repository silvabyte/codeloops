//! Durable session history. Transports and source adapters use this boundary;
//! SQLite, projections, indexing and artifact publication remain private.
mod artifacts;
mod checkpoints;
mod export;
mod ingest;
pub mod model;
mod query;

use artifacts::Artifacts;
use model::{Capture, Query, Receipt};
use rusqlite::Connection;
use serde_json::Value;
use std::{
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid request: {0}")]
    Invalid(String),
    #[error("unknown ID: {0}")]
    Unknown(String),
    #[error("conflicting delivery ID: {0}")]
    Conflict(String),
    #[error("artifact unavailable or corrupt: {0}")]
    UnavailableArtifact(String),
    #[error("storage IO: {0}")]
    Io(#[from] std::io::Error),
    #[error("database: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("serialization: {0}")]
    Json(#[from] serde_json::Error),
}

impl Error {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Invalid(_) | Self::Json(_) => "invalid_request",
            Self::Unknown(_) => "unknown_id",
            Self::Conflict(_) => "delivery_conflict",
            Self::UnavailableArtifact(_) => "unavailable_artifact",
            Self::Io(_) | Self::Database(_) => "storage_failure",
        }
    }
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub struct History {
    connection: Connection,
    artifacts: Artifacts,
}

impl History {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        std::fs::create_dir_all(root.as_ref())?;
        let mut connection = Connection::open(root.as_ref().join("history.sqlite3"))?;
        connection.busy_timeout(Duration::from_secs(10))?;
        connection.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;",
        )?;
        let transaction =
            connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let version: u32 = transaction.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version > 2 {
            return Err(Error::Invalid(
                "database requires a newer CodeLoops version".into(),
            ));
        }
        if version == 0 {
            transaction.execute_batch(include_str!("schema.sql"))?;
        }
        if version < 2 {
            transaction.execute_batch(include_str!("schema-v2.sql"))?;
        }
        transaction.commit()?;
        let artifacts = Artifacts::open(&root.as_ref().join("artifacts"))?;
        Ok(Self {
            connection,
            artifacts,
        })
    }

    pub fn ingest(&mut self, capture: Capture) -> Result<Receipt> {
        ingest::ingest(self, capture)
    }
    pub fn query(&self, query: Query) -> Result<Value> {
        query::query(self, query)
    }

    /// Freeze portable records and verify their complete artifact graph.
    /// Returns an immutable manifest reference readable through artifact queries.
    pub fn export(&self, session_id: &str) -> Result<Value> {
        export::export(self, session_id)
    }

    /// Observe now and publish only after all referenced file artifacts are durable.
    /// Call at the action boundary, never while retrying delivery of an old event.
    pub fn checkpoint(&mut self, directory: &Path, workspace_id: &str) -> Result<Value> {
        checkpoints::capture(self, directory, workspace_id)
    }
}
