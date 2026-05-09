//! Pure state for the TUI. No terminal I/O — unit-testable in isolation.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use codeloops_logging::FileChangeType;

use crate::spinner::BRAILLE_FRAMES;

/// Maximum file events retained in `recent` before older ones are dropped.
const RECENT_CAP: usize = 64;

/// A file change event captured during an actor phase.
#[derive(Debug, Clone)]
pub struct FileEvent {
    pub path: String,
    pub change_type: FileChangeType,
}

/// The current execution phase (drives spinner label and visibility).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Actor,
    Critic,
    Done,
}

/// What the critic decided in the most recent iteration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CriticVerdict {
    Done,
    Continue { feedback: Option<String> },
    Error { message: Option<String> },
}

/// Terminal state for a finished loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinalKind {
    Success,
    MaxIterations,
    Interrupted,
    Failed,
}

/// Diff stats across an iteration or full loop.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DiffStats {
    pub insertions: usize,
    pub modifications: usize,
    pub deletions: usize,
    pub files_changed: usize,
}

/// Lines emitted into terminal scrollback (above the live viewport).
#[derive(Debug, Clone)]
pub enum ScrollbackLine {
    Header {
        prompt: String,
        working_dir: String,
        actor: Option<String>,
        critic: Option<String>,
        max_iterations: Option<usize>,
    },
    IterationDone {
        n: usize,
        elapsed: Duration,
        stats: DiffStats,
        status: char, // '✓' or '✗'
    },
    CriticContinue {
        n: usize,
        feedback: Option<String>,
    },
    CriticError {
        n: usize,
        message: Option<String>,
    },
    Final {
        kind: FinalKind,
        total_elapsed: Duration,
        iterations: usize,
        prompt: String,
        error: Option<String>,
        summary: Option<String>,
        confidence: Option<f64>,
    },
}

/// Render-side events. Produced by translating `LogEvent`s on the orchestrator
/// side and consumed by the render task to mutate `AppState`.
#[derive(Debug, Clone)]
pub enum RenderEvent {
    Header {
        prompt: String,
        working_dir: PathBuf,
    },
    SetMaxIterations(Option<usize>),
    SetAgentNames {
        actor: String,
        critic: String,
    },
    IterationStart {
        iteration: usize,
    },
    ActorStart,
    FileChange(FileEvent),
    ActorCompleted {
        exit_code: i32,
        duration_secs: f64,
    },
    GitDiff {
        files_changed: usize,
        insertions: usize,
        deletions: usize,
    },
    CriticStart,
    CriticDone,
    CriticContinue {
        feedback: Option<String>,
    },
    CriticError {
        message: Option<String>,
    },
    FinalSuccess {
        iterations: usize,
        total_duration_secs: f64,
        summary: Option<String>,
        confidence: Option<f64>,
    },
    FinalMaxIterations {
        iterations: usize,
        total_duration_secs: f64,
    },
    FinalInterrupted {
        iterations: usize,
        total_duration_secs: f64,
    },
    FinalFailed {
        iterations: usize,
        total_duration_secs: f64,
        error: Option<String>,
    },
}

/// All TUI state. Lives inside the render task; `apply` is the only mutator.
pub struct AppState {
    pub prompt: String,
    pub working_dir: String,
    pub actor_name: Option<String>,
    pub critic_name: Option<String>,
    pub max_iterations: Option<usize>,
    pub current_iteration: usize,
    pub phase: Phase,
    pub spinner_frame: usize,
    pub phase_started_at: Option<Instant>,
    pub loop_started_at: Option<Instant>,
    pub recent: VecDeque<FileEvent>,
    pub total_events_this_iter: usize,
    pub last_critic: Option<CriticVerdict>,

    /// Held until `GitDiff` arrives so iteration completion can include diff stats.
    pending_actor_done: Option<(i32, f64)>,
    /// Diff stats for the just-finished iteration, used in the IterationDone scrollback line.
    last_iter_stats: DiffStats,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            prompt: String::new(),
            working_dir: String::new(),
            actor_name: None,
            critic_name: None,
            max_iterations: None,
            current_iteration: 0,
            phase: Phase::Idle,
            spinner_frame: 0,
            phase_started_at: None,
            loop_started_at: None,
            recent: VecDeque::with_capacity(RECENT_CAP),
            total_events_this_iter: 0,
            last_critic: None,
            pending_actor_done: None,
            last_iter_stats: DiffStats::default(),
        }
    }

    /// Apply an event to the state. Returns any scrollback lines that should
    /// be inserted above the live viewport.
    pub fn apply(&mut self, ev: RenderEvent) -> Vec<ScrollbackLine> {
        let mut out = Vec::new();
        match ev {
            RenderEvent::Header {
                prompt,
                working_dir,
            } => {
                self.prompt = prompt.clone();
                self.working_dir = crate::layout::shorten_home(&working_dir.display().to_string());
                self.loop_started_at = Some(Instant::now());
                out.push(ScrollbackLine::Header {
                    prompt,
                    working_dir: self.working_dir.clone(),
                    actor: self.actor_name.clone(),
                    critic: self.critic_name.clone(),
                    max_iterations: self.max_iterations,
                });
            }

            RenderEvent::SetMaxIterations(max) => {
                self.max_iterations = max;
            }

            RenderEvent::SetAgentNames { actor, critic } => {
                self.actor_name = Some(actor);
                self.critic_name = Some(critic);
            }

            RenderEvent::IterationStart { iteration } => {
                self.current_iteration = iteration;
                self.recent.clear();
                self.total_events_this_iter = 0;
                self.last_iter_stats = DiffStats::default();
                self.last_critic = None;
            }

            RenderEvent::ActorStart => {
                self.phase = Phase::Actor;
                self.phase_started_at = Some(Instant::now());
            }

            RenderEvent::FileChange(fe) => {
                self.total_events_this_iter += 1;
                self.recent.push_back(fe);
                while self.recent.len() > RECENT_CAP {
                    self.recent.pop_front();
                }
            }

            RenderEvent::ActorCompleted {
                exit_code,
                duration_secs,
            } => {
                self.pending_actor_done = Some((exit_code, duration_secs));
            }

            RenderEvent::GitDiff {
                files_changed,
                insertions,
                deletions,
            } => {
                let (exit_code, duration_secs) = self.pending_actor_done.take().unwrap_or((0, 0.0));
                let modifications = self
                    .recent
                    .iter()
                    .filter(|fe| matches!(fe.change_type, FileChangeType::Modified))
                    .count();
                let stats = DiffStats {
                    insertions,
                    modifications,
                    deletions,
                    files_changed,
                };
                self.last_iter_stats = stats;
                let status = if exit_code == 0 { '✓' } else { '✗' };
                out.push(ScrollbackLine::IterationDone {
                    n: self.current_iteration,
                    elapsed: Duration::from_secs_f64(duration_secs.max(0.0)),
                    stats,
                    status,
                });
                // Actor phase is over; clear the live spinner until CriticStart
                // arrives so the status line doesn't keep ticking up.
                self.phase = Phase::Idle;
                self.phase_started_at = None;
            }

            RenderEvent::CriticStart => {
                self.phase = Phase::Critic;
                self.phase_started_at = Some(Instant::now());
            }

            RenderEvent::CriticDone => {
                self.last_critic = Some(CriticVerdict::Done);
                self.phase = Phase::Idle;
                self.phase_started_at = None;
            }

            RenderEvent::CriticContinue { feedback } => {
                self.last_critic = Some(CriticVerdict::Continue {
                    feedback: feedback.clone(),
                });
                self.phase = Phase::Idle;
                self.phase_started_at = None;
                out.push(ScrollbackLine::CriticContinue {
                    n: self.current_iteration,
                    feedback,
                });
            }

            RenderEvent::CriticError { message } => {
                self.last_critic = Some(CriticVerdict::Error {
                    message: message.clone(),
                });
                self.phase = Phase::Idle;
                self.phase_started_at = None;
                out.push(ScrollbackLine::CriticError {
                    n: self.current_iteration,
                    message,
                });
            }

            RenderEvent::FinalSuccess {
                iterations,
                total_duration_secs,
                summary,
                confidence,
            } => {
                self.phase = Phase::Done;
                out.push(ScrollbackLine::Final {
                    kind: FinalKind::Success,
                    total_elapsed: Duration::from_secs_f64(total_duration_secs.max(0.0)),
                    iterations,
                    prompt: self.prompt.clone(),
                    error: None,
                    summary,
                    confidence,
                });
            }

            RenderEvent::FinalMaxIterations {
                iterations,
                total_duration_secs,
            } => {
                self.phase = Phase::Done;
                out.push(ScrollbackLine::Final {
                    kind: FinalKind::MaxIterations,
                    total_elapsed: Duration::from_secs_f64(total_duration_secs.max(0.0)),
                    iterations,
                    prompt: self.prompt.clone(),
                    error: None,
                    summary: None,
                    confidence: None,
                });
            }

            RenderEvent::FinalInterrupted {
                iterations,
                total_duration_secs,
            } => {
                self.phase = Phase::Done;
                out.push(ScrollbackLine::Final {
                    kind: FinalKind::Interrupted,
                    total_elapsed: Duration::from_secs_f64(total_duration_secs.max(0.0)),
                    iterations,
                    prompt: self.prompt.clone(),
                    error: None,
                    summary: None,
                    confidence: None,
                });
            }

            RenderEvent::FinalFailed {
                iterations,
                total_duration_secs,
                error,
            } => {
                self.phase = Phase::Done;
                out.push(ScrollbackLine::Final {
                    kind: FinalKind::Failed,
                    total_elapsed: Duration::from_secs_f64(total_duration_secs.max(0.0)),
                    iterations,
                    prompt: self.prompt.clone(),
                    error,
                    summary: None,
                    confidence: None,
                });
            }
        }
        out
    }

    /// Advance the spinner frame index.
    pub fn tick(&mut self) {
        self.spinner_frame = (self.spinner_frame + 1) % BRAILLE_FRAMES.len();
    }

    pub fn elapsed_total(&self) -> Duration {
        self.loop_started_at
            .map(|t| t.elapsed())
            .unwrap_or_default()
    }

    pub fn elapsed_phase(&self) -> Duration {
        self.phase_started_at
            .map(|t| t.elapsed())
            .unwrap_or_default()
    }

    pub fn is_active(&self) -> bool {
        matches!(self.phase, Phase::Actor | Phase::Critic)
    }

    pub fn last_iter_stats(&self) -> DiffStats {
        self.last_iter_stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fe(path: &str, ct: FileChangeType) -> FileEvent {
        FileEvent {
            path: path.to_string(),
            change_type: ct,
        }
    }

    #[test]
    fn apply_iteration_start_resets_recent_and_increments_iter() {
        let mut s = AppState::new();
        s.apply(RenderEvent::Header {
            prompt: "p".into(),
            working_dir: PathBuf::from("/tmp"),
        });
        s.apply(RenderEvent::IterationStart { iteration: 1 });
        s.apply(RenderEvent::FileChange(fe("a.rs", FileChangeType::Created)));
        s.apply(RenderEvent::FileChange(fe(
            "b.rs",
            FileChangeType::Modified,
        )));
        s.apply(RenderEvent::FileChange(fe("c.rs", FileChangeType::Deleted)));
        assert_eq!(s.recent.len(), 3);
        assert_eq!(s.total_events_this_iter, 3);

        s.apply(RenderEvent::IterationStart { iteration: 2 });
        assert!(s.recent.is_empty());
        assert_eq!(s.current_iteration, 2);
        assert_eq!(s.total_events_this_iter, 0);
    }

    #[test]
    fn apply_file_change_pushes_and_caps_recent() {
        let mut s = AppState::new();
        s.apply(RenderEvent::IterationStart { iteration: 1 });
        for i in 0..100 {
            s.apply(RenderEvent::FileChange(fe(
                &format!("f{}.rs", i),
                FileChangeType::Modified,
            )));
        }
        assert!(s.recent.len() <= RECENT_CAP);
        assert_eq!(s.total_events_this_iter, 100);
    }

    #[test]
    fn apply_actor_done_emits_iteration_scrollback_line() {
        let mut s = AppState::new();
        s.apply(RenderEvent::IterationStart { iteration: 1 });
        s.apply(RenderEvent::ActorStart);
        s.apply(RenderEvent::FileChange(fe("a.rs", FileChangeType::Created)));
        s.apply(RenderEvent::FileChange(fe(
            "b.rs",
            FileChangeType::Modified,
        )));
        s.apply(RenderEvent::ActorCompleted {
            exit_code: 0,
            duration_secs: 12.0,
        });
        let lines = s.apply(RenderEvent::GitDiff {
            files_changed: 2,
            insertions: 5,
            deletions: 1,
        });
        assert_eq!(lines.len(), 1);
        match &lines[0] {
            ScrollbackLine::IterationDone {
                n,
                stats,
                status,
                elapsed,
            } => {
                assert_eq!(*n, 1);
                assert_eq!(stats.files_changed, 2);
                assert_eq!(stats.insertions, 5);
                assert_eq!(stats.deletions, 1);
                assert_eq!(*status, '✓');
                assert_eq!(elapsed.as_secs(), 12);
            }
            _ => panic!("expected IterationDone"),
        }
    }

    #[test]
    fn apply_git_diff_clears_actor_phase_to_idle() {
        let mut s = AppState::new();
        s.apply(RenderEvent::IterationStart { iteration: 1 });
        s.apply(RenderEvent::ActorStart);
        assert_eq!(s.phase, Phase::Actor);
        assert!(s.phase_started_at.is_some());
        s.apply(RenderEvent::ActorCompleted {
            exit_code: 0,
            duration_secs: 1.0,
        });
        // ActorCompleted alone must not flip the phase back to idle —
        // the IterationDone scrollback hasn't been pushed yet.
        assert_eq!(s.phase, Phase::Actor);
        s.apply(RenderEvent::GitDiff {
            files_changed: 0,
            insertions: 0,
            deletions: 0,
        });
        // After GitDiff, we've emitted IterationDone and the actor work
        // is over — the live status line should stop showing
        // "actor working · Ns" until CriticStart arrives.
        assert_eq!(s.phase, Phase::Idle);
        assert!(s.phase_started_at.is_none());
    }

    #[test]
    fn apply_critic_continue_records_verdict_and_emits_scrollback() {
        let mut s = AppState::new();
        s.apply(RenderEvent::IterationStart { iteration: 1 });
        s.apply(RenderEvent::CriticStart);
        let lines = s.apply(RenderEvent::CriticContinue {
            feedback: Some("missing tests".to_string()),
        });
        assert!(matches!(
            s.last_critic,
            Some(CriticVerdict::Continue { .. })
        ));
        assert_eq!(lines.len(), 1);
        match &lines[0] {
            ScrollbackLine::CriticContinue { n, feedback } => {
                assert_eq!(*n, 1);
                assert_eq!(feedback.as_deref(), Some("missing tests"));
            }
            _ => panic!("expected CriticContinue"),
        }
    }

    #[test]
    fn apply_final_success_sets_phase_done_and_emits_final() {
        let mut s = AppState::new();
        s.apply(RenderEvent::Header {
            prompt: "p".into(),
            working_dir: PathBuf::from("/tmp"),
        });
        let lines = s.apply(RenderEvent::FinalSuccess {
            iterations: 3,
            total_duration_secs: 60.0,
            summary: Some("done".into()),
            confidence: Some(0.95),
        });
        assert_eq!(s.phase, Phase::Done);
        assert_eq!(lines.len(), 1);
        match &lines[0] {
            ScrollbackLine::Final {
                kind, iterations, ..
            } => {
                assert_eq!(*kind, FinalKind::Success);
                assert_eq!(*iterations, 3);
            }
            _ => panic!("expected Final"),
        }
    }

    #[test]
    fn tick_advances_spinner_frame_modulo_len() {
        let mut s = AppState::new();
        let len = BRAILLE_FRAMES.len();
        for _ in 0..(len + 1) {
            s.tick();
        }
        assert_eq!(s.spinner_frame, 1);
    }

    #[test]
    fn elapsed_phase_returns_zero_when_phase_idle() {
        let s = AppState::new();
        assert_eq!(s.elapsed_phase(), Duration::ZERO);
    }
}
