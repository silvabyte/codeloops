//! One-time migration script to migrate NDJSON session files to SQLite.
//!
//! Run with: cargo run --manifest-path scripts/migrate_sessions_crate/Cargo.toml
//!
//! This script:
//! 1. Reads all .jsonl files from ~/.local/share/codeloops/sessions/
//! 2. Parses the SessionStart, Iteration, and SessionEnd lines
//! 3. Inserts them into the SQLite database at ~/.local/share/codeloops/codeloops.db
//! 4. Reports success/failure for each file

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use codeloops_db::{Database, Iteration, SessionEnd, SessionStart};
use serde::Deserialize;

/// JSONL line types (mirrors the old SessionLine enum)
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum SessionLine {
    SessionStart {
        timestamp: DateTime<Utc>,
        prompt: String,
        working_dir: PathBuf,
        actor_agent: String,
        critic_agent: String,
        actor_model: Option<String>,
        critic_model: Option<String>,
        max_iterations: Option<usize>,
    },
    Iteration {
        iteration_number: usize,
        actor_output: String,
        actor_stderr: String,
        actor_exit_code: i32,
        actor_duration_secs: f64,
        git_diff: String,
        git_files_changed: usize,
        critic_decision: String,
        feedback: Option<String>,
        timestamp: DateTime<Utc>,
    },
    SessionEnd {
        outcome: String,
        iterations: usize,
        summary: Option<String>,
        confidence: Option<f64>,
        duration_secs: f64,
        #[allow(dead_code)]
        timestamp: DateTime<Utc>,
    },
}

fn main() -> anyhow::Result<()> {
    // Find sessions directory
    let data_dir = dirs::data_dir().expect("Could not determine data directory");
    let sessions_dir = data_dir.join("codeloops").join("sessions");

    if !sessions_dir.exists() {
        println!("No sessions directory found at {:?}", sessions_dir);
        println!("Nothing to migrate.");
        return Ok(());
    }

    // Open database
    let db = Database::open()?;
    println!("Opened database at {:?}", Database::default_path());

    // Find all .jsonl files
    let mut files: Vec<PathBuf> = fs::read_dir(&sessions_dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|s| s.to_str()) == Some("jsonl"))
        .collect();

    files.sort();

    if files.is_empty() {
        println!("No .jsonl files found in {:?}", sessions_dir);
        return Ok(());
    }

    println!("Found {} session files to migrate", files.len());
    println!();

    let mut success_count = 0;
    let mut skip_count = 0;
    let mut error_count = 0;

    for file_path in &files {
        let file_name = file_path.file_name().unwrap().to_string_lossy();

        match migrate_file(&db, file_path) {
            Ok(MigrateResult::Success(id)) => {
                println!("✓ {} -> {}", file_name, id);
                success_count += 1;
            }
            Ok(MigrateResult::Skipped(reason)) => {
                println!("- {} (skipped: {})", file_name, reason);
                skip_count += 1;
            }
            Err(e) => {
                println!("✗ {} (error: {})", file_name, e);
                error_count += 1;
            }
        }
    }

    println!();
    println!("Migration complete:");
    println!("  Migrated: {}", success_count);
    println!("  Skipped:  {}", skip_count);
    println!("  Errors:   {}", error_count);

    if success_count > 0 {
        println!();
        println!("You can now archive the old JSONL files:");
        println!("  mkdir -p ~/.local/share/codeloops/sessions_archive");
        println!("  mv ~/.local/share/codeloops/sessions/*.jsonl ~/.local/share/codeloops/sessions_archive/");
    }

    Ok(())
}

enum MigrateResult {
    Success(String),
    Skipped(String),
}

fn migrate_file(db: &Database, file_path: &PathBuf) -> anyhow::Result<MigrateResult> {
    let file = fs::File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut session_start: Option<SessionLine> = None;
    let mut iterations: Vec<SessionLine> = Vec::new();
    let mut session_end: Option<SessionLine> = None;

    // Parse all lines
    for line_result in reader.lines() {
        let line = line_result?;
        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<SessionLine>(&line) {
            Ok(parsed) => match &parsed {
                SessionLine::SessionStart { .. } => session_start = Some(parsed),
                SessionLine::Iteration { .. } => iterations.push(parsed),
                SessionLine::SessionEnd { .. } => session_end = Some(parsed),
            },
            Err(e) => {
                eprintln!("  Warning: Failed to parse line: {}", e);
            }
        }
    }

    // Must have a session start
    let start = match session_start {
        Some(SessionLine::SessionStart {
            prompt,
            working_dir,
            actor_agent,
            critic_agent,
            actor_model,
            critic_model,
            max_iterations,
            ..
        }) => SessionStart {
            prompt,
            working_dir,
            actor_agent,
            critic_agent,
            actor_model,
            critic_model,
            max_iterations,
        },
        _ => return Ok(MigrateResult::Skipped("no session_start found".to_string())),
    };

    // Create session in database
    let session_id = db.sessions().create(&start)?;

    // Add iterations
    for iter_line in iterations {
        if let SessionLine::Iteration {
            iteration_number,
            actor_output,
            actor_stderr,
            actor_exit_code,
            actor_duration_secs,
            git_diff,
            git_files_changed,
            critic_decision,
            feedback,
            timestamp,
        } = iter_line
        {
            let iter = Iteration {
                iteration_number,
                actor_output,
                actor_stderr,
                actor_exit_code,
                actor_duration_secs,
                git_diff,
                git_files_changed,
                critic_decision,
                feedback,
                timestamp,
            };
            db.sessions().add_iteration(&session_id, &iter)?;
        }
    }

    // End session if we have an end record
    if let Some(SessionLine::SessionEnd {
        outcome,
        iterations,
        summary,
        confidence,
        duration_secs,
        ..
    }) = session_end
    {
        let end = SessionEnd {
            outcome,
            iterations,
            summary,
            confidence,
            duration_secs,
        };
        db.sessions().end(&session_id, &end)?;
    }

    Ok(MigrateResult::Success(session_id))
}
