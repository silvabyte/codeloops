//! Prompt storage using SQLite.
//!
//! Stores prompt sessions persistently at ~/.local/share/codeloops/codeloops.db

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

/// A stored prompt record.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptRecord {
    pub id: String,
    pub title: Option<String>,
    pub work_type: String,
    pub project_path: String,
    pub project_name: String,
    pub content: Option<String>,
    pub session_state: String, // JSON blob of session state
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Filter options for listing prompts.
#[derive(Debug, Default, Clone)]
pub struct PromptFilter {
    pub project_name: Option<String>,
    pub search: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// SQLite-backed prompt storage.
pub struct PromptStore {
    conn: Arc<Mutex<Connection>>,
}

impl PromptStore {
    /// Create a new PromptStore, initializing the database if needed.
    pub fn new() -> Result<Self, rusqlite::Error> {
        let db_path = Self::db_path();

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(&db_path)?;

        // Initialize schema
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS prompts (
                id TEXT PRIMARY KEY,
                title TEXT,
                work_type TEXT NOT NULL,
                project_path TEXT NOT NULL,
                project_name TEXT NOT NULL,
                content TEXT,
                session_state TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_prompts_project_name ON prompts(project_name);
            CREATE INDEX IF NOT EXISTS idx_prompts_updated_at ON prompts(updated_at DESC);
            "#,
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Get the database path.
    fn db_path() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("codeloops")
            .join("codeloops.db")
    }

    /// Save a prompt record (insert or update).
    pub fn save(&self, record: &PromptRecord) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
                Some("Lock error".to_string()),
            )
        })?;

        conn.execute(
            r#"
            INSERT INTO prompts (id, title, work_type, project_path, project_name, content, session_state, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                work_type = excluded.work_type,
                project_path = excluded.project_path,
                project_name = excluded.project_name,
                content = excluded.content,
                session_state = excluded.session_state,
                updated_at = excluded.updated_at
            "#,
            params![
                record.id,
                record.title,
                record.work_type,
                record.project_path,
                record.project_name,
                record.content,
                record.session_state,
                record.created_at.to_rfc3339(),
                record.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    /// Get a prompt by ID.
    pub fn get(&self, id: &str) -> Result<Option<PromptRecord>, rusqlite::Error> {
        let conn = self.conn.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
                Some("Lock error".to_string()),
            )
        })?;

        conn.query_row(
            "SELECT id, title, work_type, project_path, project_name, content, session_state, created_at, updated_at FROM prompts WHERE id = ?1",
            params![id],
            |row| Self::row_to_record(row),
        )
        .optional()
    }

    /// List prompts with optional filtering.
    pub fn list(&self, filter: &PromptFilter) -> Result<Vec<PromptRecord>, rusqlite::Error> {
        let conn = self.conn.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
                Some("Lock error".to_string()),
            )
        })?;

        let mut sql =
            String::from("SELECT id, title, work_type, project_path, project_name, content, session_state, created_at, updated_at FROM prompts WHERE 1=1");
        let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref project_name) = filter.project_name {
            sql.push_str(" AND project_name = ?");
            param_values.push(Box::new(project_name.clone()));
        }

        if let Some(ref search) = filter.search {
            sql.push_str(" AND (title LIKE ? OR content LIKE ? OR project_name LIKE ?)");
            let pattern = format!("%{}%", search);
            param_values.push(Box::new(pattern.clone()));
            param_values.push(Box::new(pattern.clone()));
            param_values.push(Box::new(pattern));
        }

        sql.push_str(" ORDER BY updated_at DESC");

        if let Some(limit) = filter.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = filter.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        let params: Vec<&dyn rusqlite::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params.as_slice(), |row| Self::row_to_record(row))?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }

        Ok(records)
    }

    /// Get all unique project names.
    pub fn list_projects(&self) -> Result<Vec<String>, rusqlite::Error> {
        let conn = self.conn.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
                Some("Lock error".to_string()),
            )
        })?;

        let mut stmt =
            conn.prepare("SELECT DISTINCT project_name FROM prompts ORDER BY project_name")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;

        let mut projects = Vec::new();
        for row in rows {
            projects.push(row?);
        }

        Ok(projects)
    }

    /// Delete a prompt by ID.
    pub fn delete(&self, id: &str) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
                Some("Lock error".to_string()),
            )
        })?;

        let rows_affected = conn.execute("DELETE FROM prompts WHERE id = ?1", params![id])?;
        Ok(rows_affected > 0)
    }

    fn row_to_record(row: &rusqlite::Row) -> Result<PromptRecord, rusqlite::Error> {
        let created_at_str: String = row.get(7)?;
        let updated_at_str: String = row.get(8)?;

        Ok(PromptRecord {
            id: row.get(0)?,
            title: row.get(1)?,
            work_type: row.get(2)?,
            project_path: row.get(3)?,
            project_name: row.get(4)?,
            content: row.get(5)?,
            session_state: row.get(6)?,
            created_at: DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> PromptStore {
        // Use in-memory database for tests
        let conn = Connection::open_in_memory().unwrap();

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS prompts (
                id TEXT PRIMARY KEY,
                title TEXT,
                work_type TEXT NOT NULL,
                project_path TEXT NOT NULL,
                project_name TEXT NOT NULL,
                content TEXT,
                session_state TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            "#,
        )
        .unwrap();

        PromptStore {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    #[test]
    fn test_save_and_get() {
        let store = test_store();
        let now = Utc::now();

        let record = PromptRecord {
            id: "test-1".to_string(),
            title: Some("Test Prompt".to_string()),
            work_type: "feature".to_string(),
            project_path: "/path/to/project".to_string(),
            project_name: "my-project".to_string(),
            content: Some("# Test\n\nThis is a test.".to_string()),
            session_state: r#"{"messages":[]}"#.to_string(),
            created_at: now,
            updated_at: now,
        };

        store.save(&record).unwrap();

        let retrieved = store.get("test-1").unwrap().unwrap();
        assert_eq!(retrieved.id, "test-1");
        assert_eq!(retrieved.title, Some("Test Prompt".to_string()));
        assert_eq!(retrieved.work_type, "feature");
    }

    #[test]
    fn test_list_with_filter() {
        let store = test_store();
        let now = Utc::now();

        // Create two prompts with different project names
        let record1 = PromptRecord {
            id: "test-1".to_string(),
            title: Some("Auth Feature".to_string()),
            work_type: "feature".to_string(),
            project_path: "/path/to/project1".to_string(),
            project_name: "project-a".to_string(),
            content: Some("# Auth".to_string()),
            session_state: "{}".to_string(),
            created_at: now,
            updated_at: now,
        };

        let record2 = PromptRecord {
            id: "test-2".to_string(),
            title: Some("Bug Fix".to_string()),
            work_type: "defect".to_string(),
            project_path: "/path/to/project2".to_string(),
            project_name: "project-b".to_string(),
            content: Some("# Bug".to_string()),
            session_state: "{}".to_string(),
            created_at: now,
            updated_at: now,
        };

        store.save(&record1).unwrap();
        store.save(&record2).unwrap();

        // List all
        let all = store.list(&PromptFilter::default()).unwrap();
        assert_eq!(all.len(), 2);

        // Filter by project
        let filtered = store
            .list(&PromptFilter {
                project_name: Some("project-a".to_string()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].project_name, "project-a");

        // Search
        let searched = store
            .list(&PromptFilter {
                search: Some("Auth".to_string()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(searched.len(), 1);
        assert_eq!(searched[0].id, "test-1");
    }

    #[test]
    fn test_delete() {
        let store = test_store();
        let now = Utc::now();

        let record = PromptRecord {
            id: "test-1".to_string(),
            title: None,
            work_type: "feature".to_string(),
            project_path: "/path".to_string(),
            project_name: "project".to_string(),
            content: None,
            session_state: "{}".to_string(),
            created_at: now,
            updated_at: now,
        };

        store.save(&record).unwrap();
        assert!(store.get("test-1").unwrap().is_some());

        let deleted = store.delete("test-1").unwrap();
        assert!(deleted);
        assert!(store.get("test-1").unwrap().is_none());

        // Deleting again returns false
        let deleted_again = store.delete("test-1").unwrap();
        assert!(!deleted_again);
    }
}
