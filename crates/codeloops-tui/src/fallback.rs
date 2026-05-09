//! Non-TTY fallback renderer: plain stderr lines, one per RenderEvent.
//!
//! Used when stderr is piped, dumb terminal, or CI. NO_COLOR honored via the
//! `colored` crate.

use std::io::{self, Write};

use codeloops_logging::FileChangeType;

use crate::app::{FileEvent, RenderEvent};
use crate::layout::shorten_home;
use crate::spinner::format_elapsed;

fn use_color() -> bool {
    colored::control::SHOULD_COLORIZE.should_colorize()
}

fn green(s: &str) -> String {
    if use_color() {
        use colored::Colorize;
        s.green().to_string()
    } else {
        s.to_string()
    }
}

fn yellow(s: &str) -> String {
    if use_color() {
        use colored::Colorize;
        s.yellow().to_string()
    } else {
        s.to_string()
    }
}

fn red(s: &str) -> String {
    if use_color() {
        use colored::Colorize;
        s.red().to_string()
    } else {
        s.to_string()
    }
}

fn dim(s: &str) -> String {
    if use_color() {
        use colored::Colorize;
        s.dimmed().to_string()
    } else {
        s.to_string()
    }
}

/// Carries enough state for ActorCompleted+GitDiff merging in the same shape
/// the TTY path uses.
pub struct FallbackRenderer {
    max_iterations: Option<usize>,
    actor: Option<String>,
    critic: Option<String>,
    pending_actor: Option<(i32, f64)>,
    iter_files: Vec<FileEvent>,
    iter: usize,
}

impl Default for FallbackRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl FallbackRenderer {
    pub fn new() -> Self {
        Self {
            max_iterations: None,
            actor: None,
            critic: None,
            pending_actor: None,
            iter_files: Vec::new(),
            iter: 0,
        }
    }

    pub fn render(&mut self, ev: &RenderEvent) {
        let mut w = io::stderr();
        match ev {
            RenderEvent::Header {
                prompt,
                working_dir,
            } => {
                let dir = shorten_home(&working_dir.display().to_string());
                let _ = writeln!(w, "codeloops: {} ({})", prompt, dir);
                if let (Some(a), Some(c)) = (&self.actor, &self.critic) {
                    if let Some(max) = self.max_iterations {
                        let _ = writeln!(w, "agents: {} -> {} (max {} iterations)", a, c, max);
                    } else {
                        let _ = writeln!(w, "agents: {} -> {}", a, c);
                    }
                }
            }
            RenderEvent::SetMaxIterations(max) => {
                self.max_iterations = *max;
            }
            RenderEvent::SetAgentNames { actor, critic } => {
                self.actor = Some(actor.clone());
                self.critic = Some(critic.clone());
            }
            RenderEvent::IterationStart { iteration } => {
                self.iter = *iteration;
                self.iter_files.clear();
                if let Some(max) = self.max_iterations {
                    let _ = writeln!(w, "--- iteration {} of {} ---", iteration, max);
                } else {
                    let _ = writeln!(w, "--- iteration {} ---", iteration);
                }
            }
            RenderEvent::ActorStart => {
                let _ = writeln!(w, "actor: started");
            }
            RenderEvent::FileChange(fe) => {
                let sigil = match fe.change_type {
                    FileChangeType::Created => green("+"),
                    FileChangeType::Modified => yellow("~"),
                    FileChangeType::Deleted => red("-"),
                };
                self.iter_files.push(fe.clone());
                let _ = writeln!(w, "  {} {}", sigil, fe.path);
            }
            RenderEvent::ActorCompleted {
                exit_code,
                duration_secs,
            } => {
                self.pending_actor = Some((*exit_code, *duration_secs));
            }
            RenderEvent::GitDiff {
                files_changed,
                insertions,
                deletions,
            } => {
                let (exit_code, duration_secs) = self.pending_actor.take().unwrap_or((0, 0.0));
                let elapsed = format_elapsed(duration_secs as u64);
                if exit_code == 0 {
                    let _ = writeln!(
                        w,
                        "actor: done in {} · {} files {} {}",
                        elapsed,
                        files_changed,
                        green(&format!("+{}", insertions)),
                        red(&format!("-{}", deletions)),
                    );
                } else {
                    let _ = writeln!(w, "actor: failed exit {} in {}", exit_code, elapsed);
                }
            }
            RenderEvent::CriticStart => {
                let _ = writeln!(w, "critic: started");
            }
            RenderEvent::CriticDone => {
                let _ = writeln!(w, "critic: done");
            }
            RenderEvent::CriticContinue { feedback } => match feedback {
                Some(text) if !text.is_empty() => {
                    let _ = writeln!(w, "critic: continue · {}", text);
                }
                _ => {
                    let _ = writeln!(w, "critic: continue");
                }
            },
            RenderEvent::CriticError { message } => match message {
                Some(text) if !text.is_empty() => {
                    let _ = writeln!(w, "critic: error · {}", text);
                }
                _ => {
                    let _ = writeln!(w, "critic: error");
                }
            },
            RenderEvent::FinalSuccess {
                iterations,
                total_duration_secs,
                summary,
                confidence,
            } => {
                let _ = writeln!(
                    w,
                    "=== {} · {} iterations · {} ===",
                    green("complete"),
                    iterations,
                    format_elapsed(*total_duration_secs as u64),
                );
                if let Some(conf) = confidence {
                    let bucket = if *conf >= 0.9 {
                        "high"
                    } else if *conf >= 0.7 {
                        "medium"
                    } else {
                        "low"
                    };
                    let _ = writeln!(w, "{}", dim(&format!("confidence {}", bucket)));
                }
                if let Some(s) = summary {
                    if !s.is_empty() {
                        let _ = writeln!(w, "{}", dim(s));
                    }
                }
            }
            RenderEvent::FinalMaxIterations {
                iterations,
                total_duration_secs,
            } => {
                let _ = writeln!(
                    w,
                    "=== {} · {} iterations · {} ===",
                    yellow("incomplete"),
                    iterations,
                    format_elapsed(*total_duration_secs as u64),
                );
            }
            RenderEvent::FinalInterrupted {
                iterations,
                total_duration_secs,
            } => {
                let _ = writeln!(
                    w,
                    "=== {} · {} iterations · {} ===",
                    yellow("interrupted"),
                    iterations,
                    format_elapsed(*total_duration_secs as u64),
                );
            }
            RenderEvent::FinalFailed {
                iterations,
                total_duration_secs,
                error,
            } => {
                let _ = writeln!(
                    w,
                    "=== {} · {} iterations · {} ===",
                    red("failed"),
                    iterations,
                    format_elapsed(*total_duration_secs as u64),
                );
                if let Some(e) = error {
                    let _ = writeln!(w, "{}", red(e));
                }
            }
        }
        let _ = w.flush();
    }
}
