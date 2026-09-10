//! Snapshot export owns record selection and reference traversal. Transport clients
//! copy immutable, hash-verified bytes; they never inspect SQLite or source paths.
use crate::{
    Error, History, Result, artifacts, checkpoints,
    model::{CheckpointLink, Filter, MAX_CAPTURE_BYTES, Page, Query},
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

struct Bundle<'a> {
    history: &'a History,
    artifacts: BTreeMap<String, usize>,
    pages: Vec<Value>,
    checkpoints: BTreeSet<String>,
}

impl Bundle<'_> {
    fn artifact(&mut self, hash: &str) -> Result<()> {
        if !self.artifacts.contains_key(hash) {
            let bytes = self.history.artifacts.get(hash)?;
            self.artifacts.insert(hash.to_owned(), bytes.len());
        }
        Ok(())
    }

    fn optional_artifact(&mut self, value: &Value) -> Result<()> {
        if let Some(hash) = value.as_str() {
            self.artifact(hash)?;
        }
        Ok(())
    }

    fn links(&mut self, value: &Value) -> Result<()> {
        let links: Vec<CheckpointLink> = serde_json::from_value(value.clone())?;
        for link in links {
            for id in [link.checkpoint_id, link.before_id, link.baseline_id]
                .into_iter()
                .flatten()
            {
                self.checkpoints.insert(id);
            }
        }
        Ok(())
    }

    fn page(&mut self, kind: &str, items: Vec<Value>) -> Result<()> {
        let bytes = serde_json::to_vec(&json!({
            "kind": kind,
            "items": items,
        }))?;
        if bytes.len() > MAX_CAPTURE_BYTES * 2 {
            return Err(Error::Invalid("export record page exceeds 8 MiB".into()));
        }
        let hash = self.history.artifacts.put(&bytes)?;
        self.artifacts.insert(hash.clone(), bytes.len());
        self.pages.push(json!({
            "kind": kind,
            "hash": hash,
        }));
        Ok(())
    }

    fn entries(&mut self, session_id: &str) -> Result<()> {
        let mut cursor = None;
        loop {
            let result = self.history.query(Query::Show {
                session_id: session_id.into(),
                page: Page { limit: 100, cursor },
            })?;
            let items = result["items"].as_array().ok_or_else(invalid)?.clone();
            for item in &items {
                self.optional_artifact(&item["content_hash"])?;
                for field in ["input_hash", "output_hash", "error_hash"] {
                    self.optional_artifact(&item["tool"][field])?;
                }
                self.links(&item["checkpoints"])?;
            }
            self.page("entries", items)?;
            cursor = result["next_cursor"].as_str().map(String::from);
            if cursor.is_none() {
                return Ok(());
            }
        }
    }

    fn captures(&mut self, session_id: &str) -> Result<()> {
        let mut statement = self.history.connection.prepare(
            "SELECT id, delivery, hash, envelope_hash, payload_hash, entry, recorded, checkpoints
             FROM captures WHERE session = ? ORDER BY ordinal",
        )?;
        let mut rows = statement.query([session_id])?;
        let mut page = Vec::new();
        while let Some(row) = rows.next()? {
            let envelope_hash: String = row.get(3)?;
            let payload_hash: String = row.get(4)?;
            self.artifact(&envelope_hash)?;
            self.artifact(&payload_hash)?;
            let envelope: Value =
                serde_json::from_slice(&self.history.artifacts.get(&envelope_hash)?)?;
            if envelope["change"]["type"] == "tool" {
                for field in ["input", "output", "error"] {
                    let value = &envelope["change"][field];
                    if !value.is_null() {
                        self.artifact(&artifacts::hash(&serde_json::to_vec(value)?))?;
                    }
                }
            }
            let links: Value = serde_json::from_str(&row.get::<_, String>(7)?)?;
            self.links(&links)?;
            page.push(json!({
                "id": row.get::<_, String>(0)?,
                "delivery_id": row.get::<_, String>(1)?,
                "capture_hash": row.get::<_, String>(2)?,
                "envelope_hash": envelope_hash,
                "source_payload_hash": payload_hash,
                "session_id": session_id,
                "entry_id": row.get::<_, Option<String>>(5)?,
                "recorded_at": row.get::<_, u64>(6)?,
                "checkpoints": links,
            }));
            if page.len() == 100 {
                self.page("captures", std::mem::take(&mut page))?;
            }
        }
        if !page.is_empty() {
            self.page("captures", page)?;
        }
        Ok(())
    }
}

fn invalid() -> Error {
    Error::Invalid("invalid export record".into())
}

pub(crate) fn export(history: &History, session_id: &str) -> Result<Value> {
    // One WAL read snapshot covers projections, revisions, workspace associations
    // and checkpoint selection. Concurrent ingestion cannot mix record versions.
    let transaction = history.connection.unchecked_transaction()?;
    let sessions = history.query(Query::List {
        filter: Filter {
            session_id: Some(session_id.into()),
            ..Default::default()
        },
        page: Page::default(),
    })?;
    let session = sessions["items"]
        .as_array()
        .and_then(|items| items.first())
        .ok_or_else(|| Error::Unknown(session_id.into()))?
        .clone();
    let mut bundle = Bundle {
        history,
        artifacts: BTreeMap::new(),
        pages: Vec::new(),
        checkpoints: BTreeSet::new(),
    };
    bundle.entries(session_id)?;
    bundle.captures(session_id)?;
    for id in bundle.checkpoints.clone() {
        let record = checkpoints::get(history, &id)?;
        for hash in checkpoints::export_hashes(history, &record)? {
            bundle.artifact(&hash)?;
        }
        bundle.page("checkpoints", vec![record])?;
    }
    let manifest = json!({
        "format": "codeloops-session-export",
        "schema_version": 1,
        "session": session,
        "snapshot": "consistent_committed_archive",
        "record_order": "receipt",
        "record_pages": bundle.pages,
        "artifacts": bundle.artifacts,
        "artifact_encoding": "raw",
        "artifact_hash": "sha256",
        "coverage": {
            "pending_deliveries": "not_included",
            "related_sessions": "references_only",
            "attachments": "source_reported_metadata_unless_archived",
            "patches": "derivable_from_captured_layers",
        },
    });
    let bytes = serde_json::to_vec_pretty(&manifest)?;
    if bytes.len() > MAX_CAPTURE_BYTES * 2 {
        return Err(Error::Invalid("export manifest exceeds 8 MiB".into()));
    }
    // No descriptor is returned for a partial graph or failed publication.
    let hash = history.artifacts.put(&bytes)?;
    transaction.commit()?;
    Ok(json!({
        "format": "codeloops-session-export",
        "schema_version": 1,
        "session_id": session_id,
        "manifest_hash": hash,
        "manifest_bytes": bytes.len(),
        "artifact_count": bundle.artifacts.len(),
    }))
}
