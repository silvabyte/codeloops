//! Projects store for managing registered project directories.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::MutexGuard;
use uuid::Uuid;

/// A registered project record.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRecord {
    pub id: String,
    pub path: String,
    pub name: String,
    #[serde(default)]
    pub config_overrides: Option<ProjectConfigOverrides>,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub last_accessed_at: DateTime<Utc>,
}

/// Typed config overrides stored per-project.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfigOverrides {
    pub default_agent: Option<String>,
    pub default_model: Option<String>,
}

/// Data for creating a new project.
pub struct NewProject {
    pub path: String,
    pub name: String,
    pub config_overrides: Option<ProjectConfigOverrides>,
}

/// Data for updating an existing project.
#[derive(Debug, Default)]
pub struct ProjectUpdate {
    pub name: Option<String>,
    pub config_overrides: Option<Option<ProjectConfigOverrides>>,
    pub is_default: Option<bool>,
}

/// Projects store with a borrowed connection.
pub struct Projects<'db> {
    conn: MutexGuard<'db, Connection>,
}

impl<'db> Projects<'db> {
    pub(crate) fn new(conn: MutexGuard<'db, Connection>) -> Self {
        Self { conn }
    }

    /// Add a new project. Returns the created ProjectRecord.
    pub fn add(&self, new: &NewProject) -> Result<ProjectRecord, rusqlite::Error> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let config_json = new
            .config_overrides
            .as_ref()
            .map(|c| serde_json::to_string(c).unwrap_or_default());

        self.conn.execute(
            r#"
            INSERT INTO projects (id, path, name, config_overrides, is_default, created_at, last_accessed_at)
            VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6)
            "#,
            params![
                id,
                new.path,
                new.name,
                config_json,
                now.to_rfc3339(),
                now.to_rfc3339(),
            ],
        )?;

        Ok(ProjectRecord {
            id,
            path: new.path.clone(),
            name: new.name.clone(),
            config_overrides: new.config_overrides.clone(),
            is_default: false,
            created_at: now,
            last_accessed_at: now,
        })
    }

    /// Get a project by ID.
    pub fn get(&self, id: &str) -> Result<Option<ProjectRecord>, rusqlite::Error> {
        self.conn
            .query_row(
                "SELECT id, path, name, config_overrides, is_default, created_at, last_accessed_at FROM projects WHERE id = ?1",
                params![id],
                Self::row_to_record,
            )
            .optional()
    }

    /// Get a project by its filesystem path.
    pub fn get_by_path(&self, path: &str) -> Result<Option<ProjectRecord>, rusqlite::Error> {
        self.conn
            .query_row(
                "SELECT id, path, name, config_overrides, is_default, created_at, last_accessed_at FROM projects WHERE path = ?1",
                params![path],
                Self::row_to_record,
            )
            .optional()
    }

    /// Get the default project.
    pub fn get_default(&self) -> Result<Option<ProjectRecord>, rusqlite::Error> {
        self.conn
            .query_row(
                "SELECT id, path, name, config_overrides, is_default, created_at, last_accessed_at FROM projects WHERE is_default = 1",
                [],
                Self::row_to_record,
            )
            .optional()
    }

    /// List all projects, sorted by last_accessed_at descending.
    pub fn list(&self) -> Result<Vec<ProjectRecord>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path, name, config_overrides, is_default, created_at, last_accessed_at FROM projects ORDER BY last_accessed_at DESC",
        )?;
        let rows = stmt.query_map([], Self::row_to_record)?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }
        Ok(records)
    }

    /// Update a project. Returns the updated record or None if not found.
    pub fn update(
        &self,
        id: &str,
        update: &ProjectUpdate,
    ) -> Result<Option<ProjectRecord>, rusqlite::Error> {
        let existing = self.get(id)?;
        if existing.is_none() {
            return Ok(None);
        }

        if let Some(ref name) = update.name {
            self.conn.execute(
                "UPDATE projects SET name = ?1 WHERE id = ?2",
                params![name, id],
            )?;
        }

        if let Some(ref config) = update.config_overrides {
            let json = config
                .as_ref()
                .map(|c| serde_json::to_string(c).unwrap_or_default());
            self.conn.execute(
                "UPDATE projects SET config_overrides = ?1 WHERE id = ?2",
                params![json, id],
            )?;
        }

        if let Some(is_default) = update.is_default {
            if is_default {
                self.set_default(id)?;
            } else {
                self.conn.execute(
                    "UPDATE projects SET is_default = 0 WHERE id = ?1",
                    params![id],
                )?;
            }
        }

        // Update last_accessed_at on write
        self.conn.execute(
            "UPDATE projects SET last_accessed_at = ?1 WHERE id = ?2",
            params![Utc::now().to_rfc3339(), id],
        )?;

        self.get(id)
    }

    /// Remove a project by ID. Returns true if deleted.
    /// If the removed project was the default, promotes the most recently accessed one.
    pub fn remove(&self, id: &str) -> Result<bool, rusqlite::Error> {
        let existing = self.get(id)?;
        let was_default = existing.as_ref().map(|p| p.is_default).unwrap_or(false);

        let rows = self
            .conn
            .execute("DELETE FROM projects WHERE id = ?1", params![id])?;

        if rows > 0 && was_default {
            // Promote most recently accessed project to default
            self.conn.execute(
                "UPDATE projects SET is_default = 1 WHERE id = (SELECT id FROM projects ORDER BY last_accessed_at DESC LIMIT 1)",
                [],
            )?;
        }

        Ok(rows > 0)
    }

    /// Remove a project by path. Returns true if deleted.
    pub fn remove_by_path(&self, path: &str) -> Result<bool, rusqlite::Error> {
        if let Some(project) = self.get_by_path(path)? {
            self.remove(&project.id)
        } else {
            Ok(false)
        }
    }

    /// Atomically set one project as the default (unset all others first).
    pub fn set_default(&self, id: &str) -> Result<(), rusqlite::Error> {
        self.conn
            .execute("UPDATE projects SET is_default = 0 WHERE is_default = 1", [])?;
        self.conn.execute(
            "UPDATE projects SET is_default = 1 WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    /// Update last_accessed_at for a project (call on write actions only).
    pub fn touch(&self, id: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE projects SET last_accessed_at = ?1 WHERE id = ?2",
            params![Utc::now().to_rfc3339(), id],
        )?;
        Ok(())
    }

    fn row_to_record(row: &rusqlite::Row) -> Result<ProjectRecord, rusqlite::Error> {
        let config_json: Option<String> = row.get(3)?;
        let is_default_int: i32 = row.get(4)?;
        let created_at_str: String = row.get(5)?;
        let accessed_at_str: String = row.get(6)?;

        let config_overrides = config_json.and_then(|json| serde_json::from_str(&json).ok());

        Ok(ProjectRecord {
            id: row.get(0)?,
            path: row.get(1)?,
            name: row.get(2)?,
            config_overrides,
            is_default: is_default_int != 0,
            created_at: DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            last_accessed_at: DateTime::parse_from_rfc3339(&accessed_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        })
    }
}
