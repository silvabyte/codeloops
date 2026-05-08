//! # codeloops-tui
//!
//! Terminal UI renderer for the codeloops session loop.
//!
//! Built on [`ratatui`] with an inline viewport (no alternate screen). Older
//! content stays in the user's scrollback. Per-iteration summaries are pushed
//! into scrollback above the live region via [`ratatui::Terminal::insert_before`].
//!
//! Architecture: a single tokio task owns the [`ratatui::Terminal`] and an
//! [`AppState`]. Producers translate [`codeloops_logging::LogEvent`]s to
//! [`RenderEvent`]s and send them on an `mpsc::UnboundedSender`. The render
//! task pumps the channel plus a spinner-frame ticker and redraws.
//!
//! In non-TTY environments (pipes, dumb terminal, CI) the same channel feeds
//! a [`FallbackRenderer`] that emits plain stderr lines per event.

pub mod app;
pub mod fallback;
pub mod layout;
pub mod render;
pub mod spinner;

use std::io::{self, IsTerminal, Stderr};
use std::sync::Mutex;
use std::time::Duration;

use crossterm::{
    cursor,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    backend::CrosstermBackend,
    text::{Line, Span},
    Terminal, TerminalOptions, Viewport,
};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use codeloops_logging::LogEvent;

pub use app::{
    AppState, CriticVerdict, DiffStats, FileEvent, FinalKind, Phase, RenderEvent, ScrollbackLine,
};
pub use fallback::FallbackRenderer;

/// Initial inline viewport height: a sensible chunk of the terminal, capped at 24.
fn initial_viewport_height() -> u16 {
    let term_h = crossterm::terminal::size().map(|(_, h)| h).unwrap_or(24);
    term_h.saturating_sub(2).clamp(8, 24)
}

/// Front-end handle to the renderer. Call [`SessionRenderer::cleanup`] to
/// shut down the render task and restore the terminal.
pub struct SessionRenderer {
    tx: mpsc::UnboundedSender<Msg>,
    handle: Mutex<Option<JoinHandle<()>>>,
}

enum Msg {
    Event(RenderEvent),
    Shutdown,
}

impl Default for SessionRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionRenderer {
    /// Create a new renderer, auto-detecting TTY vs pipe.
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel::<Msg>();
        let handle = if io::stderr().is_terminal() && !is_dumb_terminal() {
            tokio::spawn(run_tty(rx))
        } else {
            tokio::spawn(run_fallback(rx))
        };
        Self {
            tx,
            handle: Mutex::new(Some(handle)),
        }
    }

    pub fn set_max_iterations(&self, max: Option<usize>) {
        let _ = self.tx.send(Msg::Event(RenderEvent::SetMaxIterations(max)));
    }

    pub fn set_agent_names(&self, actor: &str, critic: &str) {
        let _ = self.tx.send(Msg::Event(RenderEvent::SetAgentNames {
            actor: actor.to_string(),
            critic: critic.to_string(),
        }));
    }

    /// Translate a [`LogEvent`] into one or more [`RenderEvent`]s and dispatch.
    pub fn on_log_event(&self, event: &LogEvent) {
        match event {
            LogEvent::LoopStarted {
                prompt,
                working_dir,
            } => {
                let _ = self.tx.send(Msg::Event(RenderEvent::Header {
                    prompt: prompt.clone(),
                    working_dir: working_dir.clone(),
                }));
            }
            LogEvent::ActorStarted { iteration, .. } => {
                let _ = self.tx.send(Msg::Event(RenderEvent::IterationStart {
                    iteration: iteration + 1,
                }));
                let _ = self.tx.send(Msg::Event(RenderEvent::ActorStart));
            }
            LogEvent::FileChanged {
                path, change_type, ..
            } => {
                let _ = self.tx.send(Msg::Event(RenderEvent::FileChange(FileEvent {
                    path: path.to_string_lossy().to_string(),
                    change_type: *change_type,
                })));
            }
            LogEvent::ActorCompleted {
                exit_code,
                duration_secs,
                ..
            } => {
                let _ = self.tx.send(Msg::Event(RenderEvent::ActorCompleted {
                    exit_code: *exit_code,
                    duration_secs: *duration_secs,
                }));
            }
            LogEvent::GitDiffCaptured {
                files_changed,
                insertions,
                deletions,
                ..
            } => {
                let _ = self.tx.send(Msg::Event(RenderEvent::GitDiff {
                    files_changed: *files_changed,
                    insertions: *insertions,
                    deletions: *deletions,
                }));
            }
            LogEvent::CriticStarted { .. } => {
                let _ = self.tx.send(Msg::Event(RenderEvent::CriticStart));
            }
            LogEvent::CriticCompleted { decision, .. } => {
                let ev = if decision.contains("DONE") {
                    RenderEvent::CriticDone
                } else if decision.contains("ERROR") {
                    RenderEvent::CriticError {
                        message: Some(decision.clone()),
                    }
                } else {
                    RenderEvent::CriticContinue {
                        feedback: Some(decision.clone()),
                    }
                };
                let _ = self.tx.send(Msg::Event(ev));
            }
            // Final outcomes are dispatched by `main.rs` via `send_event` once
            // the runner returns — that path also covers UserInterrupted/Failed
            // which never emit log events.
            LogEvent::LoopCompleted { .. }
            | LogEvent::MaxIterationsReached { .. }
            | LogEvent::ErrorEncountered { .. }
            | LogEvent::AgentStreamLine { .. }
            | LogEvent::ActorOutput { .. } => {}
        }
    }

    /// Send a [`RenderEvent`] directly. Used for terminal outcomes constructed
    /// in `main.rs` that don't have a 1:1 [`LogEvent`].
    pub fn send_event(&self, ev: RenderEvent) {
        let _ = self.tx.send(Msg::Event(ev));
    }

    /// Shut down the render task and restore the terminal. Idempotent.
    pub async fn cleanup(&self) {
        let _ = self.tx.send(Msg::Shutdown);
        let h = self.handle.lock().ok().and_then(|mut g| g.take());
        if let Some(h) = h {
            let _ = h.await;
        }
    }
}

impl Drop for SessionRenderer {
    fn drop(&mut self) {
        // Best-effort: tell the render task to stop; it'll restore the terminal.
        let _ = self.tx.send(Msg::Shutdown);
    }
}

fn is_dumb_terminal() -> bool {
    matches!(
        std::env::var("TERM").as_deref(),
        Ok("dumb") | Ok("") | Err(_)
    )
}

// ---- Render task ----

async fn run_tty(mut rx: mpsc::UnboundedReceiver<Msg>) {
    // Set up the inline viewport. Raw mode so cursor restoration is clean.
    if enable_raw_mode().is_err() {
        // Fall back to plain output if we can't grab the terminal.
        run_fallback(rx).await;
        return;
    }
    let backend = CrosstermBackend::new(io::stderr());
    let viewport = Viewport::Inline(initial_viewport_height());
    let mut terminal = match Terminal::with_options(backend, TerminalOptions { viewport }) {
        Ok(t) => t,
        Err(_) => {
            let _ = disable_raw_mode();
            run_fallback(rx).await;
            return;
        }
    };

    let mut state = AppState::new();
    let mut tick = tokio::time::interval(Duration::from_millis(100));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            biased;
            msg = rx.recv() => {
                match msg {
                    None | Some(Msg::Shutdown) => break,
                    Some(Msg::Event(ev)) => {
                        let scrollback = state.apply(ev);
                        for line in scrollback {
                            let lines = render_scrollback_line(&line);
                            let height = lines.len() as u16;
                            let _ = terminal.insert_before(height, |buf| {
                                use ratatui::widgets::{Paragraph, Widget};
                                let para = Paragraph::new(lines);
                                para.render(buf.area, buf);
                            });
                        }
                        let _ = terminal.draw(|f| render::draw(&state, f));
                        if matches!(state.phase, Phase::Done) {
                            break;
                        }
                    }
                }
            }
            _ = tick.tick() => {
                if state.is_active() {
                    state.tick();
                }
                let _ = terminal.draw(|f| render::draw(&state, f));
            }
        }
    }

    // Clear the inline viewport so it doesn't linger.
    let _ = terminal.clear();
    drop(terminal);
    let _ = disable_raw_mode();
    let mut stderr: Stderr = io::stderr();
    let _ = crossterm::execute!(stderr, cursor::Show);
}

async fn run_fallback(mut rx: mpsc::UnboundedReceiver<Msg>) {
    let mut fb = FallbackRenderer::new();
    while let Some(msg) = rx.recv().await {
        match msg {
            Msg::Shutdown => break,
            Msg::Event(ev) => fb.render(&ev),
        }
    }
}

fn render_scrollback_line(line: &ScrollbackLine) -> Vec<Line<'static>> {
    use ratatui::style::{Color, Modifier, Style};

    let dim = Style::default().add_modifier(Modifier::DIM);
    let bold_white = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);

    match line {
        ScrollbackLine::Header {
            prompt,
            working_dir,
            actor,
            critic,
            max_iterations,
        } => {
            let mut lines = vec![
                Line::from(""),
                Line::from(vec![Span::raw("  "), Span::styled("codeloops", bold_white)]),
                Line::from(""),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(format!("\"{}\"", prompt), dim),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled("dir: ", dim),
                    Span::styled(working_dir.clone(), dim),
                ]),
            ];
            if let (Some(a), Some(c)) = (actor, critic) {
                let agents = match max_iterations {
                    Some(max) => format!("{} → {} · {} iterations max", a, c, max),
                    None => format!("{} → {}", a, c),
                };
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("agents: ", dim),
                    Span::styled(agents, dim),
                ]));
            }
            lines.push(Line::from(""));
            lines
        }
        ScrollbackLine::IterationDone {
            n,
            elapsed,
            stats,
            status,
        } => {
            let elapsed_s = spinner::format_elapsed(elapsed.as_secs());
            let status_color = if *status == '✓' {
                Color::Green
            } else {
                Color::Red
            };
            vec![Line::from(vec![
                Span::raw("  "),
                Span::styled(status.to_string(), Style::default().fg(status_color)),
                Span::raw(" "),
                Span::styled(format!("iter {} · {} · ", n, elapsed_s), dim),
                Span::styled(
                    format!("+{}", stats.insertions),
                    Style::default().fg(Color::Green),
                ),
                Span::raw(" "),
                Span::styled(
                    format!("~{}", stats.modifications),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(" "),
                Span::styled(
                    format!("-{}", stats.deletions),
                    Style::default().fg(Color::Red),
                ),
            ])]
        }
        ScrollbackLine::CriticContinue { n, feedback } => {
            let mut lines = vec![Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("→ iter {} continues", n),
                    Style::default().fg(Color::Yellow),
                ),
            ])];
            if let Some(text) = feedback {
                if !text.is_empty() {
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(format!("\"{}\"", text), dim),
                    ]));
                }
            }
            lines
        }
        ScrollbackLine::CriticError { n, message } => {
            let mut lines = vec![Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("✗ iter {} critic error", n),
                    Style::default().fg(Color::Red),
                ),
            ])];
            if let Some(text) = message {
                if !text.is_empty() {
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(text.clone(), dim),
                    ]));
                }
            }
            lines
        }
        ScrollbackLine::Final {
            kind,
            total_elapsed,
            iterations,
            prompt,
            error,
            summary,
        } => {
            let elapsed_s = spinner::format_elapsed(total_elapsed.as_secs());
            let (sigil, color, label) = match kind {
                FinalKind::Success => ("✓", Color::Green, "codeloops done"),
                FinalKind::MaxIterations => ("⚠", Color::Yellow, "codeloops incomplete"),
                FinalKind::Interrupted => ("⏸", Color::Yellow, "codeloops interrupted"),
                FinalKind::Failed => ("✗", Color::Red, "codeloops failed"),
            };
            let mut lines = vec![
                Line::from(""),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(sigil, Style::default().fg(color)),
                    Span::raw(" "),
                    Span::styled(
                        label,
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!(" · {} · {} iterations", elapsed_s, iterations), dim),
                ]),
            ];
            if !prompt.is_empty() {
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(format!("\"{}\"", prompt), dim),
                ]));
            }
            if let Some(text) = summary {
                if !text.is_empty() {
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(text.clone(), dim),
                    ]));
                }
            }
            if let Some(text) = error {
                if !text.is_empty() {
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(text.clone(), Style::default().fg(Color::Red)),
                    ]));
                }
            }
            lines.push(Line::from(""));
            lines
        }
    }
}
