use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Shared context for the actor-critic loop
#[derive(Debug, Clone)]
pub struct LoopContext {
    /// Original task prompt
    pub prompt: String,
    /// Working directory
    pub working_dir: PathBuf,
    /// Current iteration number (0-indexed)
    pub iteration: usize,
    /// History of all iterations
    pub history: Vec<IterationRecord>,
    /// When the loop started
    started_at: Instant,
    /// Maximum iterations (None = unlimited)
    pub max_iterations: Option<usize>,
    /// Last feedback from critic (for next actor iteration)
    pub last_feedback: Option<String>,
}

/// Record of a single iteration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IterationRecord {
    pub iteration_number: usize,
    pub actor_output: String,
    pub actor_stderr: String,
    pub actor_exit_code: i32,
    pub actor_duration_secs: f64,
    pub git_diff: String,
    pub git_files_changed: usize,
    pub critic_output: String,
    pub critic_decision: String,
    pub timestamp: DateTime<Utc>,
}

impl LoopContext {
    pub fn new(prompt: String, working_dir: PathBuf) -> Self {
        Self {
            prompt,
            working_dir,
            iteration: 0,
            history: Vec::new(),
            started_at: Instant::now(),
            max_iterations: None,
            last_feedback: None,
        }
    }

    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = Some(max);
        self
    }

    pub fn increment_iteration(&mut self) {
        self.iteration += 1;
    }

    pub fn push_record(&mut self, record: IterationRecord) {
        self.history.push(record);
    }

    pub fn set_feedback(&mut self, feedback: String) {
        self.last_feedback = Some(feedback);
    }

    pub fn total_duration(&self) -> Duration {
        self.started_at.elapsed()
    }

    pub fn should_continue(&self) -> bool {
        match self.max_iterations {
            Some(max) => self.iteration < max,
            None => true,
        }
    }

    /// Get the prompt for the current iteration
    /// First iteration uses original prompt, subsequent use feedback
    pub fn current_prompt(&self) -> String {
        if self.iteration == 0 {
            self.prompt.clone()
        } else if let Some(ref feedback) = self.last_feedback {
            codeloops_critic::CriticPrompts::build_continuation_prompt(&self.prompt, feedback)
        } else {
            // Fallback to original prompt if no feedback
            self.prompt.clone()
        }
    }
}
