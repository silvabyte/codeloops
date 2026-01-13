use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Role of the agent producing output
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    Actor,
    Critic,
}

/// Type of output stream
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamType {
    Stdout,
    Stderr,
}

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
    /// Streaming output line from an agent
    AgentStreamLine {
        iteration: usize,
        role: AgentRole,
        stream: StreamType,
        line: String,
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

impl LogEvent {
    /// Add a timestamp to serialize with the event
    fn with_timestamp(&self) -> serde_json::Value {
        let mut value = serde_json::to_value(self).unwrap_or_default();
        if let Some(obj) = value.as_object_mut() {
            obj.insert(
                "timestamp".to_string(),
                serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
            );
        }
        value
    }
}

/// Log output format
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LogFormat {
    /// Human-readable format with colors and visual structure
    #[default]
    Pretty,
    /// JSON lines format for machine consumption
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

/// Logger for codeloops events - handles both console output and file logging
pub struct Logger {
    format: LogFormat,
    file_writer: Option<Mutex<File>>,
}

impl Logger {
    pub fn new(format: LogFormat) -> Self {
        Self {
            format,
            file_writer: None,
        }
    }

    /// Create a logger with file output in addition to console
    pub fn with_file(format: LogFormat, log_path: &Path) -> std::io::Result<Self> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;

        Ok(Self {
            format,
            file_writer: Some(Mutex::new(file)),
        })
    }

    pub fn log(&self, event: &LogEvent) {
        // Log to file if configured (always JSON format for file)
        if let Some(ref writer) = self.file_writer {
            if let Ok(mut file) = writer.lock() {
                let json = event.with_timestamp();
                let _ = writeln!(file, "{}", json);
            }
        }

        // Log to console based on format
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
                // Top banner
                let _ = writeln!(stderr);
                let _ = writeln!(
                    stderr,
                    "{}",
                    "╭─────────────────────────────────────────────────────────────────────╮"
                        .bright_blue()
                );
                let _ = writeln!(
                    stderr,
                    "{}  {}{}",
                    "│".bright_blue(),
                    "codeloops".bold().bright_white(),
                    " ".repeat(58) + &"│".bright_blue().to_string()
                );
                let _ = writeln!(
                    stderr,
                    "{}  {} {}",
                    "│".bright_blue(),
                    "Prompt:".dimmed(),
                    Self::truncate_with_padding(prompt, 60, 68).dimmed()
                );
                let _ = writeln!(
                    stderr,
                    "{}  {} {}",
                    "│".bright_blue(),
                    "Dir:".dimmed(),
                    Self::truncate_with_padding(&working_dir.display().to_string(), 63, 68)
                        .dimmed()
                );
                let _ = writeln!(
                    stderr,
                    "{}",
                    "╰─────────────────────────────────────────────────────────────────────╯"
                        .bright_blue()
                );
                let _ = writeln!(stderr);
            }
            LogEvent::ActorStarted { iteration, .. } => {
                // Iteration header
                let iter_text = format!("─ Iteration {} ", iteration + 1);
                let padding = "─".repeat(67 - iter_text.len());
                let _ = writeln!(
                    stderr,
                    "{}{}{}",
                    "┌".bright_blue(),
                    iter_text.bright_blue().bold(),
                    padding.bright_blue()
                );
                let _ = writeln!(stderr);

                // Actor section header
                let _ = writeln!(
                    stderr,
                    "  {} {}",
                    "▶".bright_cyan(),
                    "ACTOR".bright_cyan().bold()
                );
            }
            LogEvent::ActorCompleted {
                exit_code,
                duration_secs,
                ..
            } => {
                // Actor completion
                if *exit_code == 0 {
                    let _ = writeln!(
                        stderr,
                        "    {} Done ({:.1}s)",
                        "✓".bright_green(),
                        duration_secs
                    );
                } else {
                    let _ = writeln!(
                        stderr,
                        "    {} Exit {} ({:.1}s)",
                        "✗".bright_red(),
                        exit_code,
                        duration_secs
                    );
                }
                let _ = writeln!(stderr);
            }
            LogEvent::GitDiffCaptured {
                files_changed,
                insertions,
                deletions,
                ..
            } => {
                if *files_changed > 0 {
                    let _ = writeln!(
                        stderr,
                        "    {} {} {} {}, {} {}, {} {}",
                        "📁".dimmed(),
                        "Git:".dimmed(),
                        files_changed,
                        if *files_changed == 1 { "file" } else { "files" },
                        format!("+{}", insertions).green(),
                        if *insertions == 1 { "line" } else { "lines" },
                        format!("-{}", deletions).red(),
                        if *deletions == 1 { "line" } else { "lines" }
                    );
                } else {
                    let _ = writeln!(
                        stderr,
                        "    {} {}",
                        "📁".dimmed(),
                        "Git: no changes".dimmed()
                    );
                }
                let _ = writeln!(stderr);
            }
            LogEvent::CriticStarted { .. } => {
                let _ = writeln!(
                    stderr,
                    "  {} {}",
                    "▶".bright_magenta(),
                    "CRITIC".bright_magenta().bold()
                );
            }
            LogEvent::CriticCompleted { decision, .. } => {
                // Parse decision to show appropriate styling
                let styled_decision = if decision.contains("DONE") {
                    format!("✓ Decision: {}", decision)
                        .bright_green()
                        .to_string()
                } else if decision.contains("ERROR") {
                    format!("✗ Decision: {}", decision).bright_red().to_string()
                } else {
                    format!("→ Decision: {}", decision)
                        .bright_yellow()
                        .to_string()
                };
                let _ = writeln!(stderr, "    {}", styled_decision);
                let _ = writeln!(stderr);

                // Iteration footer
                let _ = writeln!(
                    stderr,
                    "{}",
                    "└─────────────────────────────────────────────────────────────────────┘"
                        .bright_blue()
                );
                let _ = writeln!(stderr);
            }
            LogEvent::LoopCompleted { .. } => {
                // This is handled by the final outcome printing in main.rs
                // We skip it here to avoid duplication
            }
            LogEvent::MaxIterationsReached { iterations } => {
                let _ = writeln!(stderr);
                let _ = writeln!(
                    stderr,
                    "{} Maximum iterations reached ({})",
                    "⚠".bright_yellow(),
                    iterations
                );
            }
            LogEvent::ErrorEncountered { iteration, error } => {
                let _ = writeln!(stderr);
                let _ = writeln!(
                    stderr,
                    "{} Error in iteration {}: {}",
                    "✗".bright_red(),
                    iteration + 1,
                    error.bright_red()
                );
            }
            LogEvent::AgentStreamLine { line, stream, .. } => {
                // Stream output with visual indent
                let prefix = "    │".dimmed();
                let styled_line = match stream {
                    StreamType::Stdout => line.normal(),
                    StreamType::Stderr => line.dimmed(),
                };
                let _ = writeln!(stderr, "{} {}", prefix, styled_line);
            }
            LogEvent::ActorOutput { .. } => {
                // Skip this in pretty mode - it's debug info
            }
        }
    }

    fn log_compact(&self, event: &LogEvent) {
        let mut stderr = std::io::stderr();
        let timestamp = chrono::Utc::now().format("%H:%M:%S");
        let msg = match event {
            LogEvent::LoopStarted { .. } => format!("[{}] loop:start", timestamp),
            LogEvent::ActorStarted { iteration, .. } => {
                format!("[{}] actor:start:{}", timestamp, iteration + 1)
            }
            LogEvent::ActorCompleted {
                iteration,
                exit_code,
                duration_secs,
            } => format!(
                "[{}] actor:done:{} exit={} {:.1}s",
                timestamp,
                iteration + 1,
                exit_code,
                duration_secs
            ),
            LogEvent::CriticStarted { iteration } => {
                format!("[{}] critic:start:{}", timestamp, iteration + 1)
            }
            LogEvent::CriticCompleted {
                iteration,
                decision,
            } => format!("[{}] critic:done:{} {}", timestamp, iteration + 1, decision),
            LogEvent::LoopCompleted {
                iterations,
                duration_secs,
                ..
            } => format!(
                "[{}] loop:done:{} {:.1}s",
                timestamp, iterations, duration_secs
            ),
            LogEvent::MaxIterationsReached { iterations } => {
                format!("[{}] loop:limit:{}", timestamp, iterations)
            }
            LogEvent::ErrorEncountered { iteration, error } => {
                format!("[{}] error:{}:{}", timestamp, iteration + 1, error)
            }
            LogEvent::GitDiffCaptured {
                iteration,
                files_changed,
                insertions,
                deletions,
            } => format!(
                "[{}] git:{} {}f +{} -{}",
                timestamp, iteration, files_changed, insertions, deletions
            ),
            LogEvent::AgentStreamLine { role, line, .. } => {
                let role_str = match role {
                    AgentRole::Actor => "A",
                    AgentRole::Critic => "C",
                };
                format!("[{}] {}:{}", timestamp, role_str, line)
            }
            LogEvent::ActorOutput { .. } => return, // Skip in compact mode
        };
        let _ = writeln!(stderr, "{}", msg);
    }

    /// Truncate a string and pad to exact width
    fn truncate_with_padding(s: &str, max_len: usize, total_width: usize) -> String {
        let truncated = if s.len() > max_len {
            format!("{}...", &s[..max_len - 3])
        } else {
            s.to_string()
        };

        let padding_needed = total_width.saturating_sub(truncated.len() + 1); // +1 for trailing │
        format!("{}{}│", truncated, " ".repeat(padding_needed))
    }
}
