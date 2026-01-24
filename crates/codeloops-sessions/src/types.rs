use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Mirrors the SessionLine enum from codeloops-logging but with Deserialize.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionLine {
    SessionStart(SessionStart),
    Iteration(Iteration),
    SessionEnd(SessionEnd),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStart {
    pub timestamp: DateTime<Utc>,
    pub prompt: String,
    pub working_dir: PathBuf,
    pub actor_agent: String,
    pub critic_agent: String,
    pub actor_model: Option<String>,
    pub critic_model: Option<String>,
    pub max_iterations: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Iteration {
    pub iteration_number: usize,
    pub actor_output: String,
    pub actor_stderr: String,
    pub actor_exit_code: i32,
    pub actor_duration_secs: f64,
    pub git_diff: String,
    pub git_files_changed: usize,
    pub critic_decision: String,
    pub feedback: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEnd {
    pub outcome: String,
    pub iterations: usize,
    pub summary: Option<String>,
    pub confidence: Option<f64>,
    pub duration_secs: f64,
    pub timestamp: DateTime<Utc>,
}

/// A fully parsed session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub start: SessionStart,
    pub iterations: Vec<Iteration>,
    pub end: Option<SessionEnd>,
}

/// Summary for list views.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub prompt_preview: String,
    pub working_dir: PathBuf,
    pub project: String,
    pub outcome: Option<String>,
    pub iterations: usize,
    pub duration_secs: Option<f64>,
    pub confidence: Option<f64>,
    pub actor_agent: String,
    pub critic_agent: String,
}

/// Filter parameters for listing sessions.
#[derive(Debug, Default)]
pub struct SessionFilter {
    pub outcome: Option<String>,
    pub after: Option<DateTime<Utc>>,
    pub before: Option<DateTime<Utc>>,
    pub search: Option<String>,
    pub project: Option<String>,
}

/// Aggregate statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    pub total_sessions: usize,
    pub success_rate: f64,
    pub avg_iterations: f64,
    pub avg_duration_secs: f64,
    pub sessions_over_time: Vec<DayCount>,
    pub by_project: Vec<ProjectStats>,
}

/// Sessions count for a single day.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayCount {
    pub date: String,
    pub count: usize,
}

/// Per-project statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStats {
    pub project: String,
    pub total: usize,
    pub success_rate: f64,
}
