//! Prompts store for persistent prompt storage.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::MutexGuard;

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

/// Prompts store with a borrowed connection.
pub struct Prompts<'db> {
    conn: MutexGuard<'db, Connection>,
}

impl<'db> Prompts<'db> {
    /// Create a new Prompts store with a borrowed connection.
    pub(crate) fn new(conn: MutexGuard<'db, Connection>) -> Self {
        Self { conn }
    }

    /// Save a prompt record (insert or update).
    pub fn save(&self, record: &PromptRecord) -> Result<(), rusqlite::Error> {
        self.conn.execute(
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
        self.conn
            .query_row(
                "SELECT id, title, work_type, project_path, project_name, content, session_state, created_at, updated_at FROM prompts WHERE id = ?1",
                params![id],
                Self::row_to_record,
            )
            .optional()
    }

    /// List prompts with optional filtering.
    pub fn list(&self, filter: &PromptFilter) -> Result<Vec<PromptRecord>, rusqlite::Error> {
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

        let params: Vec<&dyn rusqlite::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params.as_slice(), Self::row_to_record)?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }

        Ok(records)
    }

    /// Get all unique project names.
    pub fn list_projects(&self) -> Result<Vec<String>, rusqlite::Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT project_name FROM prompts ORDER BY project_name")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;

        let mut projects = Vec::new();
        for row in rows {
            projects.push(row?);
        }

        Ok(projects)
    }

    /// Delete a prompt by ID.
    pub fn delete(&self, id: &str) -> Result<bool, rusqlite::Error> {
        let rows_affected = self
            .conn
            .execute("DELETE FROM prompts WHERE id = ?1", params![id])?;
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
