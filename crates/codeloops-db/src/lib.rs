//! Database layer for codeloops.
//!
//! Provides a unified `Database` struct that owns the SQLite connection
//! and provides access to domain-specific stores.

mod prompts;

pub use prompts::{PromptFilter, PromptRecord, Prompts};

use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

/// The main database struct that owns the SQLite connection.
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Open or create a database at the default location.
    ///
    /// The default location is `~/.local/share/codeloops/codeloops.db`.
    pub fn open() -> Result<Self, rusqlite::Error> {
        let db_path = Self::default_path();

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        Self::open_at(&db_path)
    }

    /// Open or create a database at a specific path.
    pub fn open_at(path: &std::path::Path) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        Self::init_schema(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Open an in-memory database (useful for testing).
    pub fn open_in_memory() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_in_memory()?;
        Self::init_schema(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Get the default database path.
    pub fn default_path() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("codeloops")
            .join("codeloops.db")
    }

    /// Access the prompts store.
    pub fn prompts(&self) -> Prompts<'_> {
        let conn = self.conn.lock().expect("Database lock poisoned");
        Prompts::new(conn)
    }

    /// Initialize the database schema.
    fn init_schema(conn: &Connection) -> Result<(), rusqlite::Error> {
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

            CREATE TABLE IF NOT EXISTS prompt_parents (
                child_id TEXT NOT NULL REFERENCES prompts(id) ON DELETE CASCADE,
                parent_id TEXT NOT NULL REFERENCES prompts(id) ON DELETE CASCADE,
                position INTEGER NOT NULL,
                PRIMARY KEY (child_id, parent_id)
            );

            CREATE INDEX IF NOT EXISTS idx_prompt_parents_child ON prompt_parents(child_id);
            CREATE INDEX IF NOT EXISTS idx_prompt_parents_parent ON prompt_parents(parent_id);
            "#,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_save_and_get() {
        let db = Database::open_in_memory().unwrap();
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

        db.prompts().save(&record).unwrap();

        let retrieved = db.prompts().get("test-1").unwrap().unwrap();
        assert_eq!(retrieved.id, "test-1");
        assert_eq!(retrieved.title, Some("Test Prompt".to_string()));
        assert_eq!(retrieved.work_type, "feature");
    }

    #[test]
    fn test_list_with_filter() {
        let db = Database::open_in_memory().unwrap();
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

        db.prompts().save(&record1).unwrap();
        db.prompts().save(&record2).unwrap();

        // List all
        let all = db.prompts().list(&PromptFilter::default()).unwrap();
        assert_eq!(all.len(), 2);

        // Filter by project
        let filtered = db
            .prompts()
            .list(&PromptFilter {
                project_name: Some("project-a".to_string()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].project_name, "project-a");

        // Search
        let searched = db
            .prompts()
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
        let db = Database::open_in_memory().unwrap();
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

        db.prompts().save(&record).unwrap();
        assert!(db.prompts().get("test-1").unwrap().is_some());

        let deleted = db.prompts().delete("test-1").unwrap();
        assert!(deleted);
        assert!(db.prompts().get("test-1").unwrap().is_none());

        // Deleting again returns false
        let deleted_again = db.prompts().delete("test-1").unwrap();
        assert!(!deleted_again);
    }

    #[test]
    fn test_list_projects() {
        let db = Database::open_in_memory().unwrap();
        let now = Utc::now();

        let record1 = PromptRecord {
            id: "test-1".to_string(),
            title: None,
            work_type: "feature".to_string(),
            project_path: "/path/a".to_string(),
            project_name: "project-a".to_string(),
            content: None,
            session_state: "{}".to_string(),
            created_at: now,
            updated_at: now,
        };

        let record2 = PromptRecord {
            id: "test-2".to_string(),
            title: None,
            work_type: "feature".to_string(),
            project_path: "/path/b".to_string(),
            project_name: "project-b".to_string(),
            content: None,
            session_state: "{}".to_string(),
            created_at: now,
            updated_at: now,
        };

        db.prompts().save(&record1).unwrap();
        db.prompts().save(&record2).unwrap();

        let projects = db.prompts().list_projects().unwrap();
        assert_eq!(projects.len(), 2);
        assert!(projects.contains(&"project-a".to_string()));
        assert!(projects.contains(&"project-b".to_string()));
    }

    #[test]
    fn test_parent_ids() {
        let db = Database::open_in_memory().unwrap();
        let now = Utc::now();

        // Create parent and child prompts
        let parent1 = PromptRecord {
            id: "parent-1".to_string(),
            title: Some("Parent 1".to_string()),
            work_type: "feature".to_string(),
            project_path: "/path".to_string(),
            project_name: "project".to_string(),
            content: Some("Parent 1 content".to_string()),
            session_state: "{}".to_string(),
            created_at: now,
            updated_at: now,
        };

        let parent2 = PromptRecord {
            id: "parent-2".to_string(),
            title: Some("Parent 2".to_string()),
            work_type: "feature".to_string(),
            project_path: "/path".to_string(),
            project_name: "project".to_string(),
            content: Some("Parent 2 content".to_string()),
            session_state: "{}".to_string(),
            created_at: now,
            updated_at: now,
        };

        let child = PromptRecord {
            id: "child-1".to_string(),
            title: Some("Child".to_string()),
            work_type: "feature".to_string(),
            project_path: "/path".to_string(),
            project_name: "project".to_string(),
            content: Some("Child content".to_string()),
            session_state: "{}".to_string(),
            created_at: now,
            updated_at: now,
        };

        db.prompts().save(&parent1).unwrap();
        db.prompts().save(&parent2).unwrap();
        db.prompts().save(&child).unwrap();

        // Initially no parents
        let parents = db.prompts().get_parent_ids("child-1").unwrap();
        assert!(parents.is_empty());

        // Set parents
        db.prompts()
            .set_parent_ids("child-1", &["parent-1".to_string(), "parent-2".to_string()])
            .unwrap();

        // Verify parents in order
        let parents = db.prompts().get_parent_ids("child-1").unwrap();
        assert_eq!(parents, vec!["parent-1", "parent-2"]);

        // Update parents (change order)
        db.prompts()
            .set_parent_ids("child-1", &["parent-2".to_string(), "parent-1".to_string()])
            .unwrap();

        let parents = db.prompts().get_parent_ids("child-1").unwrap();
        assert_eq!(parents, vec!["parent-2", "parent-1"]);

        // Clear parents
        db.prompts().set_parent_ids("child-1", &[]).unwrap();
        let parents = db.prompts().get_parent_ids("child-1").unwrap();
        assert!(parents.is_empty());
    }

    #[test]
    fn test_resolve_chain_simple() {
        let db = Database::open_in_memory().unwrap();
        let now = Utc::now();

        // Create A -> B -> C chain
        let a = PromptRecord {
            id: "a".to_string(),
            title: Some("A".to_string()),
            work_type: "feature".to_string(),
            project_path: "/path".to_string(),
            project_name: "project".to_string(),
            content: Some("A content".to_string()),
            session_state: "{}".to_string(),
            created_at: now,
            updated_at: now,
        };

        let b = PromptRecord {
            id: "b".to_string(),
            title: Some("B".to_string()),
            work_type: "feature".to_string(),
            project_path: "/path".to_string(),
            project_name: "project".to_string(),
            content: Some("B content".to_string()),
            session_state: "{}".to_string(),
            created_at: now,
            updated_at: now,
        };

        let c = PromptRecord {
            id: "c".to_string(),
            title: Some("C".to_string()),
            work_type: "feature".to_string(),
            project_path: "/path".to_string(),
            project_name: "project".to_string(),
            content: Some("C content".to_string()),
            session_state: "{}".to_string(),
            created_at: now,
            updated_at: now,
        };

        db.prompts().save(&a).unwrap();
        db.prompts().save(&b).unwrap();
        db.prompts().save(&c).unwrap();

        // B extends A
        db.prompts()
            .set_parent_ids("b", &["a".to_string()])
            .unwrap();
        // C extends B
        db.prompts()
            .set_parent_ids("c", &["b".to_string()])
            .unwrap();

        // Resolve C's chain: should be A, B, C
        let chain = db.prompts().resolve_chain("c").unwrap();
        let ids: Vec<&str> = chain.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_resolve_chain_diamond() {
        let db = Database::open_in_memory().unwrap();
        let now = Utc::now();

        // Diamond: D extends B and C, both B and C extend A
        //      A
        //     / \
        //    B   C
        //     \ /
        //      D
        let create_record = |id: &str| PromptRecord {
            id: id.to_string(),
            title: Some(id.to_string()),
            work_type: "feature".to_string(),
            project_path: "/path".to_string(),
            project_name: "project".to_string(),
            content: Some(format!("{} content", id)),
            session_state: "{}".to_string(),
            created_at: now,
            updated_at: now,
        };

        db.prompts().save(&create_record("a")).unwrap();
        db.prompts().save(&create_record("b")).unwrap();
        db.prompts().save(&create_record("c")).unwrap();
        db.prompts().save(&create_record("d")).unwrap();

        db.prompts()
            .set_parent_ids("b", &["a".to_string()])
            .unwrap();
        db.prompts()
            .set_parent_ids("c", &["a".to_string()])
            .unwrap();
        db.prompts()
            .set_parent_ids("d", &["b".to_string(), "c".to_string()])
            .unwrap();

        // Resolve D: A should only appear once
        let chain = db.prompts().resolve_chain("d").unwrap();
        let ids: Vec<&str> = chain.iter().map(|r| r.id.as_str()).collect();

        // With depth-first and dedup: A, B, C, D (A appears once, before B)
        assert_eq!(ids.len(), 4);
        assert_eq!(ids[0], "a"); // A first (root)
        assert_eq!(ids[3], "d"); // D last (target)
                                 // B and C are in between (order depends on D's parent order)
        assert!(ids.contains(&"b"));
        assert!(ids.contains(&"c"));
    }

    #[test]
    fn test_delete_cascade_parent() {
        let db = Database::open_in_memory().unwrap();
        let now = Utc::now();

        let parent = PromptRecord {
            id: "parent".to_string(),
            title: Some("Parent".to_string()),
            work_type: "feature".to_string(),
            project_path: "/path".to_string(),
            project_name: "project".to_string(),
            content: Some("Parent content".to_string()),
            session_state: "{}".to_string(),
            created_at: now,
            updated_at: now,
        };

        let child = PromptRecord {
            id: "child".to_string(),
            title: Some("Child".to_string()),
            work_type: "feature".to_string(),
            project_path: "/path".to_string(),
            project_name: "project".to_string(),
            content: Some("Child content".to_string()),
            session_state: "{}".to_string(),
            created_at: now,
            updated_at: now,
        };

        db.prompts().save(&parent).unwrap();
        db.prompts().save(&child).unwrap();
        db.prompts()
            .set_parent_ids("child", &["parent".to_string()])
            .unwrap();

        // Verify relationship exists
        let parents = db.prompts().get_parent_ids("child").unwrap();
        assert_eq!(parents, vec!["parent"]);

        // Delete parent - CASCADE should remove relationship
        db.prompts().delete("parent").unwrap();

        // Child still exists
        assert!(db.prompts().get("child").unwrap().is_some());

        // But relationship is gone
        let parents = db.prompts().get_parent_ids("child").unwrap();
        assert!(parents.is_empty());
    }
}
