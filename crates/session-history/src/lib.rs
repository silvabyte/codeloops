//! Durable session history. Transports and source adapters use this boundary;
//! SQLite, projections, indexing and artifact publication remain private.
mod artifacts;
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
        let connection = Connection::open(root.as_ref().join("history.sqlite3"))?;
        connection.busy_timeout(Duration::from_secs(10))?;
        connection.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;",
        )?;
        let version: u32 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version > 1 {
            return Err(Error::Invalid(
                "database requires a newer CodeLoops version".into(),
            ));
        }
        if version == 0 {
            let transaction = connection.unchecked_transaction()?;
            transaction.execute_batch(include_str!("schema.sql"))?;
            transaction.commit()?;
        }
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
}
