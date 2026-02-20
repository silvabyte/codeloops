//! # codeloops-tui
//!
//! Terminal UI renderer for the codeloops session loop output.
//!
//! Provides a clean, spacious layout with:
//! - Live braille spinner animation during agent execution
//! - Real-time file change events with git-style sigils
//! - Graceful fallback for non-TTY environments
//!
//! ## Usage
//!
//! ```rust,ignore
//! use codeloops_tui::SessionRenderer;
//!
//! let mut renderer = SessionRenderer::new();
//! renderer.set_max_iterations(Some(5));
//! renderer.set_agent_names("claude", "claude");
//!
//! // Feed events...
//! renderer.render(&event);
//!
//! // Called periodically by a ticker task
//! renderer.tick();
//! ```

pub mod fallback;
pub mod layout;
pub mod renderer;
pub mod spinner;
pub mod watcher;

use std::io::IsTerminal;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use codeloops_logging::LogEvent;

pub use renderer::{FileEvent, RenderEvent, TuiRenderer};
pub use fallback::FallbackRenderer;

/// Unified session renderer that selects TTY or fallback mode.
pub struct SessionRenderer {
    inner: RendererInner,
    /// Track the actor start time for computing durations on completion
    actor_start: Option<Instant>,
    /// Track the critic start time
    critic_start: Option<Instant>,
    /// Accumulated file events for the current actor phase
    file_events: Vec<FileEvent>,
    /// Stored actor completion data (exit_code, duration_secs) waiting for GitDiffCaptured
    pending_actor_done: Option<(i32, f64)>,
}

enum RendererInner {
    Tui(TuiRenderer),
    Fallback(FallbackRenderer),
}

impl Default for SessionRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionRenderer {
    /// Create a new renderer, auto-detecting TTY vs pipe.
    pub fn new() -> Self {
        let is_tty = std::io::stderr().is_terminal();
        let inner = if is_tty {
            RendererInner::Tui(TuiRenderer::new())
        } else {
            RendererInner::Fallback(FallbackRenderer::new())
        };
        Self {
            inner,
            actor_start: None,
            critic_start: None,
            file_events: Vec::new(),
            pending_actor_done: None,
        }
    }

    /// Force TTY mode (for testing).
    pub fn new_tty() -> Self {
        Self {
            inner: RendererInner::Tui(TuiRenderer::new()),
            actor_start: None,
            critic_start: None,
            file_events: Vec::new(),
            pending_actor_done: None,
        }
    }

    /// Force fallback mode (for testing).
    pub fn new_fallback() -> Self {
        Self {
            inner: RendererInner::Fallback(FallbackRenderer::new()),
            actor_start: None,
            critic_start: None,
            file_events: Vec::new(),
            pending_actor_done: None,
        }
    }

    pub fn set_max_iterations(&mut self, max: Option<usize>) {
        match &mut self.inner {
            RendererInner::Tui(r) => r.set_max_iterations(max),
            RendererInner::Fallback(r) => r.set_max_iterations(max),
        }
    }

    pub fn set_agent_names(&mut self, actor: &str, critic: &str) {
        match &mut self.inner {
            RendererInner::Tui(r) => r.set_agent_names(actor, critic),
            RendererInner::Fallback(r) => r.set_agent_names(actor, critic),
        }
    }

    /// Process a LogEvent and render the appropriate output.
    pub fn on_log_event(&mut self, event: &LogEvent) {
        match event {
            LogEvent::LoopStarted {
                prompt,
                working_dir,
            } => {
                self.render(RenderEvent::Header {
                    prompt: prompt.clone(),
                    working_dir: working_dir.clone(),
                });
            }

            LogEvent::ActorStarted { iteration, .. } => {
                self.render(RenderEvent::IterationStart {
                    iteration: iteration + 1,
                });
                self.actor_start = Some(Instant::now());
                self.file_events.clear();
                self.render(RenderEvent::ActorStart);
            }

            LogEvent::FileChanged {
                path, change_type, ..
            } => {
                let fe = FileEvent {
                    path: path.to_string_lossy().to_string(),
                    change_type: *change_type,
                };
                self.file_events.push(fe.clone());
                self.render(RenderEvent::FileChange(fe));
            }

            LogEvent::ActorCompleted {
                exit_code,
                duration_secs,
                ..
            } => {
                // Store completion data; actual rendering deferred until GitDiffCaptured
                self.pending_actor_done = Some((*exit_code, *duration_secs));
                self.actor_start = None;
            }

            LogEvent::GitDiffCaptured {
                files_changed,
                insertions,
                deletions,
                ..
            } => {
                let (exit_code, duration_secs) = self.pending_actor_done.take().unwrap_or((0, 0.0));
                self.render(RenderEvent::ActorDone {
                    duration_secs,
                    exit_code,
                    files_changed: *files_changed,
                    insertions: *insertions,
                    deletions: *deletions,
                    summary: None,
                    file_events: self.file_events.clone(),
                });
                self.file_events.clear();
            }

            LogEvent::CriticStarted { .. } => {
                self.critic_start = Some(Instant::now());
                self.render(RenderEvent::CriticStart);
            }

            LogEvent::CriticCompleted { decision, .. } => {
                let duration = self
                    .critic_start
                    .map(|s| s.elapsed().as_secs_f64())
                    .unwrap_or(0.0);
                self.critic_start = None;

                if decision.contains("DONE") {
                    self.render(RenderEvent::CriticDone {
                        duration_secs: duration,
                        decision_text: decision.clone(),
                        feedback: None,
                    });
                } else if decision.contains("ERROR") {
                    self.render(RenderEvent::CriticError {
                        duration_secs: duration,
                        error: Some(decision.clone()),
                    });
                } else {
                    self.render(RenderEvent::CriticContinue {
                        duration_secs: duration,
                        feedback: Some(decision.clone()),
                    });
                }
            }

            LogEvent::LoopCompleted {
                iterations,
                summary,
                duration_secs,
            } => {
                self.render(RenderEvent::FinalSuccess {
                    iterations: *iterations,
                    total_duration_secs: *duration_secs,
                    confidence: None, // Will be passed from outcome
                    summary: Some(summary.clone()),
                });
            }

            LogEvent::MaxIterationsReached { iterations } => {
                self.render(RenderEvent::FinalMaxIterations {
                    iterations: *iterations,
                    total_duration_secs: 0.0, // Will be filled from outcome
                });
            }

            LogEvent::ErrorEncountered { .. } => {
                // Inline errors are part of the iteration flow, not final outcomes
            }

            // Stream lines and ActorOutput are handled by the existing logger
            LogEvent::AgentStreamLine { .. } | LogEvent::ActorOutput { .. } => {}
        }
    }

    /// Tick the spinner (call every ~100ms from a ticker task).
    pub fn tick(&mut self) {
        if let RendererInner::Tui(ref mut r) = self.inner {
            r.tick();
        }
        // No-op for fallback
    }

    /// Check if the renderer has an active spinner.
    pub fn has_active_spinner(&self) -> bool {
        match &self.inner {
            RendererInner::Tui(r) => r.has_active_spinner(),
            RendererInner::Fallback(_) => false,
        }
    }

    /// Clean up terminal state.
    pub fn cleanup(&mut self) {
        if let RendererInner::Tui(ref mut r) = self.inner {
            r.cleanup();
        }
    }

    /// Render a RenderEvent directly (public for use from main.rs).
    pub fn render_event(&mut self, event: RenderEvent) {
        self.render(event);
    }

    fn render(&mut self, event: RenderEvent) {
        match &mut self.inner {
            RendererInner::Tui(r) => r.render(&event),
            RendererInner::Fallback(r) => r.render(&event),
        }
    }
}

/// Create a thread-safe shared renderer for use with tokio tasks.
pub fn create_shared_renderer() -> Arc<Mutex<SessionRenderer>> {
    Arc::new(Mutex::new(SessionRenderer::new()))
}
