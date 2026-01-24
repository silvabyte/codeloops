use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};

/// Represents each line type in the session JSONL file.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionLine {
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
        timestamp: DateTime<Utc>,
    },
}

/// Writes session data as JSONL to a file in ~/.local/share/codeloops/sessions/.
pub struct SessionWriter {
    file: Mutex<BufWriter<File>>,
    path: PathBuf,
}

impl SessionWriter {
    /// Create a new SessionWriter. Computes the session file path from the current
    /// UTC timestamp and a hash of the prompt, creates parent directories, and opens
    /// the file for writing.
    pub fn new(prompt: &str) -> io::Result<Self> {
        let sessions_dir = Self::sessions_dir()?;
        fs::create_dir_all(&sessions_dir)?;

        let now = Utc::now();
        let timestamp_str = now.format("%Y-%m-%dT%H-%M-%SZ").to_string();

        let mut hasher = Sha256::new();
        hasher.update(prompt.as_bytes());
        let hash = hex::encode(hasher.finalize());
        let short_hash = &hash[..6];

        let filename = format!("{}_{}.jsonl", timestamp_str, short_hash);
        let path = sessions_dir.join(filename);

        let file = File::create(&path)?;
        let writer = BufWriter::new(file);

        Ok(Self {
            file: Mutex::new(writer),
            path,
        })
    }

    /// Returns the path to the session file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Write the session start line.
    pub fn write_start(
        &self,
        prompt: &str,
        working_dir: &Path,
        actor_agent: &str,
        critic_agent: &str,
        actor_model: Option<&str>,
        critic_model: Option<&str>,
        max_iterations: Option<usize>,
    ) {
        let line = SessionLine::SessionStart {
            timestamp: Utc::now(),
            prompt: prompt.to_string(),
            working_dir: working_dir.to_path_buf(),
            actor_agent: actor_agent.to_string(),
            critic_agent: critic_agent.to_string(),
            actor_model: actor_model.map(String::from),
            critic_model: critic_model.map(String::from),
            max_iterations,
        };
        self.write_line(&line);
    }

    /// Write an iteration line. Accepts individual fields to avoid circular
    /// dependency on codeloops-core's IterationRecord.
    pub fn write_iteration(
        &self,
        iteration_number: usize,
        actor_output: &str,
        actor_stderr: &str,
        actor_exit_code: i32,
        actor_duration_secs: f64,
        git_diff: &str,
        git_files_changed: usize,
        critic_decision: &str,
        feedback: Option<&str>,
        timestamp: DateTime<Utc>,
    ) {
        let line = SessionLine::Iteration {
            iteration_number,
            actor_output: actor_output.to_string(),
            actor_stderr: actor_stderr.to_string(),
            actor_exit_code,
            actor_duration_secs,
            git_diff: git_diff.to_string(),
            git_files_changed,
            critic_decision: critic_decision.to_string(),
            feedback: feedback.map(String::from),
            timestamp,
        };
        self.write_line(&line);
    }

    /// Write the session end line.
    pub fn write_end(
        &self,
        outcome: &str,
        iterations: usize,
        summary: Option<&str>,
        confidence: Option<f64>,
        duration_secs: f64,
    ) {
        let line = SessionLine::SessionEnd {
            outcome: outcome.to_string(),
            iterations,
            summary: summary.map(String::from),
            confidence,
            duration_secs,
            timestamp: Utc::now(),
        };
        self.write_line(&line);
    }

    fn write_line(&self, line: &SessionLine) {
        if let Ok(json) = serde_json::to_string(line) {
            if let Ok(mut writer) = self.file.lock() {
                let _ = writeln!(writer, "{}", json);
                let _ = writer.flush();
            }
        }
    }

    fn sessions_dir() -> io::Result<PathBuf> {
        let data_dir = dirs::data_dir().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "Could not determine data directory",
            )
        })?;
        Ok(data_dir.join("codeloops").join("sessions"))
    }
}
