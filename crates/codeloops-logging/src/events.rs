use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;

/// Structured log events for the actor-critic loop
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum LogEvent {
    LoopStarted {
        prompt: String,
        working_dir: PathBuf,
    },
    ActorStarted {
        iteration: usize,
        prompt_preview: String,
    },
    ActorCompleted {
        iteration: usize,
        exit_code: i32,
        duration_secs: f64,
    },
    ActorOutput {
        iteration: usize,
        stdout_lines: usize,
        stderr_lines: usize,
    },
    GitDiffCaptured {
        iteration: usize,
        files_changed: usize,
        insertions: usize,
        deletions: usize,
    },
    CriticStarted {
        iteration: usize,
    },
    CriticCompleted {
        iteration: usize,
        decision: String,
    },
    LoopCompleted {
        iterations: usize,
        summary: String,
        duration_secs: f64,
    },
    MaxIterationsReached {
        iterations: usize,
    },
    ErrorEncountered {
        iteration: usize,
        error: String,
    },
}

/// Log output format
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LogFormat {
    /// Human-readable format
    #[default]
    Pretty,
    /// JSON lines format
    Json,
    /// Compact single-line format
    Compact,
}

impl std::str::FromStr for LogFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pretty" => Ok(LogFormat::Pretty),
            "json" => Ok(LogFormat::Json),
            "compact" => Ok(LogFormat::Compact),
            _ => Err(format!("Unknown log format: {}", s)),
        }
    }
}

/// Logger for codeloops events
pub struct Logger {
    format: LogFormat,
}

impl Logger {
    pub fn new(format: LogFormat) -> Self {
        Self { format }
    }

    pub fn log(&self, event: &LogEvent) {
        match self.format {
            LogFormat::Json => self.log_json(event),
            LogFormat::Pretty => self.log_pretty(event),
            LogFormat::Compact => self.log_compact(event),
        }
    }

    fn log_json(&self, event: &LogEvent) {
        if let Ok(json) = serde_json::to_string(event) {
            let _ = writeln!(std::io::stderr(), "{}", json);
        }
    }

    fn log_pretty(&self, event: &LogEvent) {
        let mut stderr = std::io::stderr();
        match event {
            LogEvent::LoopStarted {
                prompt,
                working_dir,
            } => {
                let _ = writeln!(stderr, "=== Codeloops Started ===");
                let _ = writeln!(
                    stderr,
                    "Prompt: {}",
                    if prompt.len() > 80 {
                        format!("{}...", &prompt[..80])
                    } else {
                        prompt.clone()
                    }
                );
                let _ = writeln!(stderr, "Working dir: {}", working_dir.display());
            }
            LogEvent::ActorStarted { iteration, .. } => {
                let _ = writeln!(stderr, "\n--- Iteration {} ---", iteration + 1);
                let _ = writeln!(stderr, "[ACTOR] Starting...");
            }
            LogEvent::ActorCompleted {
                exit_code,
                duration_secs,
                ..
            } => {
                let _ = writeln!(
                    stderr,
                    "[ACTOR] Completed (exit: {}, took: {:.1}s)",
                    exit_code, duration_secs
                );
            }
            LogEvent::GitDiffCaptured {
                files_changed,
                insertions,
                deletions,
                ..
            } => {
                let _ = writeln!(
                    stderr,
                    "[GIT] {} files changed, +{} -{} lines",
                    files_changed, insertions, deletions
                );
            }
            LogEvent::CriticStarted { .. } => {
                let _ = writeln!(stderr, "[CRITIC] Evaluating...");
            }
            LogEvent::CriticCompleted { decision, .. } => {
                let _ = writeln!(stderr, "[CRITIC] Decision: {}", decision);
            }
            LogEvent::LoopCompleted {
                iterations,
                summary,
                duration_secs,
            } => {
                let _ = writeln!(stderr, "\n=== Loop Completed ===");
                let _ = writeln!(stderr, "Iterations: {}", iterations);
                let _ = writeln!(stderr, "Duration: {:.1}s", duration_secs);
                let _ = writeln!(stderr, "Summary: {}", summary);
            }
            LogEvent::MaxIterationsReached { iterations } => {
                let _ = writeln!(
                    stderr,
                    "\n[LIMIT] Maximum iterations reached ({})",
                    iterations
                );
            }
            LogEvent::ErrorEncountered { iteration, error } => {
                let _ = writeln!(stderr, "[ERROR] Iteration {}: {}", iteration + 1, error);
            }
            _ => {
                let _ = writeln!(stderr, "{:?}", event);
            }
        }
    }

    fn log_compact(&self, event: &LogEvent) {
        let mut stderr = std::io::stderr();
        let msg = match event {
            LogEvent::LoopStarted { .. } => "loop:start".to_string(),
            LogEvent::ActorStarted { iteration, .. } => format!("actor:start:{}", iteration + 1),
            LogEvent::ActorCompleted {
                iteration,
                exit_code,
                ..
            } => format!("actor:done:{}:exit={}", iteration + 1, exit_code),
            LogEvent::CriticStarted { iteration } => format!("critic:start:{}", iteration + 1),
            LogEvent::CriticCompleted {
                iteration,
                decision,
            } => format!("critic:done:{}:{}", iteration + 1, decision),
            LogEvent::LoopCompleted { iterations, .. } => format!("loop:done:{}", iterations),
            LogEvent::MaxIterationsReached { iterations } => format!("loop:limit:{}", iterations),
            LogEvent::ErrorEncountered { iteration, error } => {
                format!("error:{}:{}", iteration + 1, error)
            }
            _ => format!("{:?}", event),
        };
        let _ = writeln!(stderr, "{}", msg);
    }
}
