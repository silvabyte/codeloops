/// Main TUI renderer with in-place spinner updates.
///
/// Uses crossterm for cursor manipulation on stderr. Output stays in
/// the terminal scrollback — no alternate screen.
use std::collections::VecDeque;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Instant;

use crossterm::{cursor, execute, terminal};

use codeloops_logging::FileChangeType;

use crate::layout::{
    self, content_indent, fit_path_to_width, pad_label, rule, shorten_home, stream_box_height,
    wrap_text, MARGIN, MAX_FILE_EVENTS,
};
use crate::spinner::{format_elapsed, Spinner};

/// A file change event detected during actor execution.
#[derive(Debug, Clone)]
pub struct FileEvent {
    pub path: String,
    pub change_type: FileChangeType,
}

/// High-level render events produced by the orchestrator.
/// These map from LogEvent but are decoupled from the logging crate's
/// serialization concerns.
pub enum RenderEvent {
    Header {
        prompt: String,
        working_dir: PathBuf,
    },
    IterationStart {
        iteration: usize,
    },
    ActorStart,
    FileChange(FileEvent),
    ActorDone {
        duration_secs: f64,
        exit_code: i32,
        files_changed: usize,
        insertions: usize,
        deletions: usize,
        summary: Option<String>,
        file_events: Vec<FileEvent>,
    },
    CriticStart,
    CriticDone {
        duration_secs: f64,
        decision_text: String,
        feedback: Option<String>,
    },
    CriticContinue {
        duration_secs: f64,
        feedback: Option<String>,
    },
    CriticError {
        duration_secs: f64,
        error: Option<String>,
    },
    FinalSuccess {
        iterations: usize,
        total_duration_secs: f64,
        confidence: Option<f64>,
        summary: Option<String>,
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

/// Whether color output is enabled.
fn use_color() -> bool {
    colored::control::SHOULD_COLORIZE.should_colorize()
}

fn dim(s: &str) -> String {
    if use_color() {
        use colored::Colorize;
        s.dimmed().to_string()
    } else {
        s.to_string()
    }
}

fn bold(s: &str) -> String {
    if use_color() {
        use colored::Colorize;
        s.bold().bright_white().to_string()
    } else {
        s.to_string()
    }
}

fn green(s: &str) -> String {
    if use_color() {
        use colored::Colorize;
        s.green().to_string()
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

fn yellow(s: &str) -> String {
    if use_color() {
        use colored::Colorize;
        s.yellow().to_string()
    } else {
        s.to_string()
    }
}

fn dim_cyan(s: &str) -> String {
    if use_color() {
        use colored::Colorize;
        s.cyan().dimmed().to_string()
    } else {
        s.to_string()
    }
}

fn cyan_char(s: &str) -> String {
    if use_color() {
        use colored::Colorize;
        s.cyan().to_string()
    } else {
        s.to_string()
    }
}

/// State for the in-place spinner line during an active phase.
struct SpinnerState {
    spinner: Spinner,
    start: Instant,
    label: String,
    /// Fixed height of the streaming file box above the spinner.
    /// 0 for phases without a box (e.g. critic).
    box_height: usize,
    /// Most-recent file events for the streaming view (tail). Never exceeds box_height.
    recent: VecDeque<FileEvent>,
    /// Whether we've printed the box+spinner at least once (so cursor moves are valid).
    printed: bool,
}

/// TTY-aware renderer with in-place spinner updates.
pub struct TuiRenderer {
    max_iterations: Option<usize>,
    actor_agent_name: Option<String>,
    critic_agent_name: Option<String>,
    /// Active spinner state (during actor/critic execution)
    active_spinner: Option<SpinnerState>,
}

impl Default for TuiRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl TuiRenderer {
    pub fn new() -> Self {
        Self {
            max_iterations: None,
            actor_agent_name: None,
            critic_agent_name: None,
            active_spinner: None,
        }
    }

    pub fn set_max_iterations(&mut self, max: Option<usize>) {
        self.max_iterations = max;
    }

    pub fn set_agent_names(&mut self, actor: &str, critic: &str) {
        self.actor_agent_name = Some(actor.to_string());
        self.critic_agent_name = Some(critic.to_string());
    }

    /// Called by the spinner tick task (~100ms interval) to update in-place.
    ///
    /// Layout (top to bottom): exactly `box_height` rows of streaming file events
    /// (most recent at the bottom; padded with blanks if fewer events have arrived),
    /// then a single spinner status row directly underneath.
    pub fn tick(&mut self) {
        if let Some(ref mut state) = self.active_spinner {
            let frame = state.spinner.tick();
            let elapsed_secs = state.start.elapsed().as_secs();
            let elapsed_str = format_elapsed(elapsed_secs);

            let mut w = io::stderr();

            // Move cursor back to the top of the box on every redraw.
            if state.printed {
                let total = state.box_height + 1;
                let _ = execute!(w, cursor::MoveUp(total as u16));
            }

            let term_w = layout::term_width();

            // Render the file-event box (always exactly box_height rows).
            // Tail-align: pad the *top* with blanks so newest events sit just above
            // the spinner.
            let pad = state.box_height.saturating_sub(state.recent.len());
            for _ in 0..pad {
                let _ = execute!(w, terminal::Clear(terminal::ClearType::CurrentLine));
                let _ = writeln!(w, "\r");
            }
            for fe in state.recent.iter() {
                let sigil = match fe.change_type {
                    FileChangeType::Created => green("+"),
                    FileChangeType::Modified => yellow("~"),
                    FileChangeType::Deleted => red("-"),
                };
                let path = fit_path_to_width(&fe.path, term_w);
                let _ = execute!(w, terminal::Clear(terminal::ClearType::CurrentLine));
                let _ = writeln!(w, "\r{}{} {}", content_indent(), sigil, path);
            }

            // Spinner status row, directly under the box.
            let _ = execute!(w, terminal::Clear(terminal::ClearType::CurrentLine));
            let _ = writeln!(
                w,
                "\r{}{}{} {}",
                MARGIN,
                dim(&pad_label(&state.label)),
                cyan_char(&frame.to_string()),
                dim_cyan(&elapsed_str),
            );

            let _ = w.flush();
            state.printed = true;
        }
    }

    /// Render a high-level event.
    pub fn render(&mut self, event: &RenderEvent) {
        let mut w = io::stderr();
        match event {
            RenderEvent::Header {
                prompt,
                working_dir,
            } => {
                let _ = writeln!(w);
                let title_rule = rule("codeloops ".len() + 2);
                let _ = writeln!(w, "{}{} {}", MARGIN, bold("codeloops"), dim(&title_rule));
                let _ = writeln!(w);

                let dir_display = shorten_home(&working_dir.display().to_string());
                let width = layout::term_width();

                // prompt (may wrap)
                let prompt_lines = wrap_text(prompt, width);
                for (i, line) in prompt_lines.iter().enumerate() {
                    if i == 0 {
                        let _ = writeln!(w, "{}{}{}", MARGIN, dim(&pad_label("prompt")), line);
                    } else {
                        let _ = writeln!(w, "{}{}", content_indent(), line);
                    }
                }

                let _ = writeln!(w, "{}{}{}", MARGIN, dim(&pad_label("dir")), dir_display);

                // agents line
                if let (Some(actor), Some(critic)) =
                    (&self.actor_agent_name, &self.critic_agent_name)
                {
                    let agents_str = if let Some(max) = self.max_iterations {
                        format!("{} → {} · {} iterations max", actor, critic, max)
                    } else {
                        format!("{} → {}", actor, critic)
                    };
                    let _ = writeln!(w, "{}{}{}", MARGIN, dim(&pad_label("agents")), agents_str);
                }
                let _ = writeln!(w);
                let _ = writeln!(w);
            }

            RenderEvent::IterationStart { iteration } => {
                let iter_display = if let Some(max) = self.max_iterations {
                    format!("{} of {}", iteration, max)
                } else {
                    format!("{}", iteration)
                };
                let iter_rule = rule(iter_display.len() + 3);
                let _ = writeln!(w, "{}{} {}", MARGIN, bold(&iter_display), dim(&iter_rule));
                let _ = writeln!(w);
            }

            RenderEvent::ActorStart => {
                let h = stream_box_height();
                self.active_spinner = Some(SpinnerState {
                    spinner: Spinner::new(),
                    start: Instant::now(),
                    label: "actor".to_string(),
                    box_height: h,
                    recent: VecDeque::with_capacity(h),
                    printed: false,
                });
                // Print the initial (empty) box + spinner line
                self.tick();
            }

            RenderEvent::FileChange(file_event) => {
                if let Some(ref mut state) = self.active_spinner {
                    state.recent.push_back(file_event.clone());
                    while state.recent.len() > state.box_height {
                        state.recent.pop_front();
                    }
                }
                // The next tick() will redraw including the new file event
            }

            RenderEvent::ActorDone {
                duration_secs,
                exit_code,
                files_changed,
                insertions,
                deletions,
                summary,
                file_events,
            } => {
                // Clear the spinner area
                self.clear_spinner_area(&mut w);
                self.active_spinner = None;

                let events_to_show: &[FileEvent] = file_events;

                if *exit_code == 0 {
                    let elapsed = format_elapsed(*duration_secs as u64);
                    let stats = if *files_changed > 0 {
                        format!(
                            "{} · {} {} {} {}",
                            elapsed,
                            files_changed,
                            if *files_changed == 1 { "file" } else { "files" },
                            green(&format!("+{}", insertions)),
                            red(&format!("-{}", deletions)),
                        )
                    } else {
                        format!("{} · no changes", elapsed)
                    };
                    let _ = writeln!(
                        w,
                        "{}{}{} {}",
                        MARGIN,
                        dim(&pad_label("actor")),
                        green("✓"),
                        dim(&stats),
                    );
                } else {
                    let elapsed = format_elapsed(*duration_secs as u64);
                    let _ = writeln!(
                        w,
                        "{}{}{} {} · exit {}",
                        MARGIN,
                        dim(&pad_label("actor")),
                        red("✗"),
                        dim(&elapsed),
                        exit_code,
                    );
                }
                let _ = writeln!(w);

                // File events: cap to MAX_FILE_EVENTS rows + summary, width-truncate paths.
                if !events_to_show.is_empty() {
                    let term_w = layout::term_width();
                    let cap = MAX_FILE_EVENTS;
                    for fe in events_to_show.iter().take(cap) {
                        let sigil = match fe.change_type {
                            FileChangeType::Created => green("+"),
                            FileChangeType::Modified => yellow("~"),
                            FileChangeType::Deleted => red("-"),
                        };
                        let path = fit_path_to_width(&fe.path, term_w);
                        let _ = writeln!(w, "{}{} {}", content_indent(), sigil, path);
                    }
                    if events_to_show.len() > cap {
                        let remaining = events_to_show.len() - cap;
                        let _ = writeln!(
                            w,
                            "{}{}",
                            content_indent(),
                            dim(&format!("… and {} more files", remaining))
                        );
                    }
                    let _ = writeln!(w);
                }

                // Summary
                if let Some(text) = summary {
                    if !text.is_empty() {
                        let width = layout::term_width();
                        let lines = wrap_text(text, width);
                        for line in &lines {
                            let _ = writeln!(w, "{}{}", content_indent(), dim(line));
                        }
                        let _ = writeln!(w);
                    }
                }
                let _ = writeln!(w);
            }

            RenderEvent::CriticStart => {
                self.active_spinner = Some(SpinnerState {
                    spinner: Spinner::new(),
                    start: Instant::now(),
                    label: "critic".to_string(),
                    box_height: 0,
                    recent: VecDeque::new(),
                    printed: false,
                });
                self.tick();
            }

            RenderEvent::CriticDone {
                duration_secs,
                feedback,
                ..
            } => {
                self.clear_spinner_area(&mut w);
                self.active_spinner = None;

                let elapsed = format_elapsed(*duration_secs as u64);
                let _ = writeln!(
                    w,
                    "{}{}{} {} · {}",
                    MARGIN,
                    dim(&pad_label("critic")),
                    green("✓"),
                    dim("done"),
                    dim(&elapsed),
                );
                let _ = writeln!(w);

                if let Some(text) = feedback {
                    if !text.is_empty() {
                        let width = layout::term_width();
                        let lines = wrap_text(text, width);
                        for line in &lines {
                            let _ = writeln!(w, "{}{}", content_indent(), dim(line));
                        }
                        let _ = writeln!(w);
                    }
                }
            }

            RenderEvent::CriticContinue {
                duration_secs,
                feedback,
            } => {
                self.clear_spinner_area(&mut w);
                self.active_spinner = None;

                let elapsed = format_elapsed(*duration_secs as u64);
                let _ = writeln!(
                    w,
                    "{}{}{} {} · {}",
                    MARGIN,
                    dim(&pad_label("critic")),
                    yellow("→"),
                    yellow("continue"),
                    dim(&elapsed),
                );
                let _ = writeln!(w);

                if let Some(text) = feedback {
                    if !text.is_empty() {
                        let width = layout::term_width();
                        let lines = wrap_text(text, width);
                        for line in &lines {
                            let _ = writeln!(w, "{}{}", content_indent(), dim(line));
                        }
                        let _ = writeln!(w);
                    }
                }
                let _ = writeln!(w);
            }

            RenderEvent::CriticError {
                duration_secs,
                error,
            } => {
                self.clear_spinner_area(&mut w);
                self.active_spinner = None;

                let elapsed = format_elapsed(*duration_secs as u64);
                let _ = writeln!(
                    w,
                    "{}{}{} {} · {}",
                    MARGIN,
                    dim(&pad_label("critic")),
                    red("✗"),
                    red("error"),
                    dim(&elapsed),
                );
                let _ = writeln!(w);

                if let Some(text) = error {
                    let width = layout::term_width();
                    let lines = wrap_text(text, width);
                    for line in &lines {
                        let _ = writeln!(w, "{}{}", content_indent(), dim(line));
                    }
                    let _ = writeln!(w);
                }
            }

            RenderEvent::FinalSuccess {
                iterations,
                total_duration_secs,
                confidence,
                summary,
            } => {
                let final_rule = rule(2);
                let _ = writeln!(w, "{}{}", MARGIN, dim(&final_rule));
                let _ = writeln!(w);

                let elapsed = format_elapsed(*total_duration_secs as u64);
                let iter_word = if *iterations == 1 {
                    "iteration"
                } else {
                    "iterations"
                };
                let _ = writeln!(
                    w,
                    "{}{} · {} {} · {}",
                    MARGIN,
                    green(&bold("✓ done")),
                    iterations,
                    iter_word,
                    dim(&elapsed),
                );

                if let Some(conf) = confidence {
                    let conf_text = if *conf >= 0.9 {
                        "high"
                    } else if *conf >= 0.7 {
                        "medium"
                    } else {
                        "low"
                    };
                    let _ = writeln!(w, "{}confidence {}", MARGIN, dim(conf_text),);
                }
                let _ = writeln!(w);

                if let Some(text) = summary {
                    let width = layout::term_width();
                    let lines = wrap_text(text, width);
                    for line in &lines {
                        let _ = writeln!(w, "{}{}", content_indent(), dim(line));
                    }
                    let _ = writeln!(w);
                }
            }

            RenderEvent::FinalMaxIterations {
                iterations,
                total_duration_secs,
            } => {
                let final_rule = rule(2);
                let _ = writeln!(w, "{}{}", MARGIN, dim(&final_rule));
                let _ = writeln!(w);

                let elapsed = format_elapsed(*total_duration_secs as u64);
                let _ = writeln!(
                    w,
                    "{}{} · {} iterations · {}",
                    MARGIN,
                    yellow("⚠ incomplete"),
                    iterations,
                    dim(&elapsed),
                );
                let _ = writeln!(w);
                let _ = writeln!(
                    w,
                    "{}{}",
                    content_indent(),
                    dim("The task may not be fully complete.")
                );
                let _ = writeln!(w);
            }

            RenderEvent::FinalInterrupted {
                iterations,
                total_duration_secs,
            } => {
                let final_rule = rule(2);
                let _ = writeln!(w, "{}{}", MARGIN, dim(&final_rule));
                let _ = writeln!(w);

                let elapsed = format_elapsed(*total_duration_secs as u64);
                let iter_word = if *iterations == 1 {
                    "iteration"
                } else {
                    "iterations"
                };
                let _ = writeln!(
                    w,
                    "{}{} · {} {} · {}",
                    MARGIN,
                    yellow("⏸ interrupted"),
                    iterations,
                    iter_word,
                    dim(&elapsed),
                );
                let _ = writeln!(w);
            }

            RenderEvent::FinalFailed {
                iterations,
                total_duration_secs,
                error,
            } => {
                let final_rule = rule(2);
                let _ = writeln!(w, "{}{}", MARGIN, dim(&final_rule));
                let _ = writeln!(w);

                let elapsed = format_elapsed(*total_duration_secs as u64);
                let iter_word = if *iterations == 1 {
                    "iteration"
                } else {
                    "iterations"
                };
                let _ = writeln!(
                    w,
                    "{}{} · {} {} · {}",
                    MARGIN,
                    red("✗ failed"),
                    iterations,
                    iter_word,
                    dim(&elapsed),
                );
                let _ = writeln!(w);

                if let Some(text) = error {
                    let width = layout::term_width();
                    let lines = wrap_text(text, width);
                    for line in &lines {
                        let _ = writeln!(w, "{}{}", content_indent(), dim(line));
                    }
                    let _ = writeln!(w);
                }
            }
        }
        let _ = w.flush();
    }

    /// Clear the streaming box + spinner row from the terminal.
    /// Layout total height = box_height + 1 (the spinner row).
    fn clear_spinner_area(&self, _w: &mut impl Write) {
        if let Some(ref state) = self.active_spinner {
            if state.printed {
                let total_lines = state.box_height + 1;
                let _ = execute!(io::stderr(), cursor::MoveUp(total_lines as u16));
                for _ in 0..total_lines {
                    let _ = execute!(
                        io::stderr(),
                        terminal::Clear(terminal::ClearType::CurrentLine),
                        cursor::MoveDown(1),
                    );
                }
                let _ = execute!(io::stderr(), cursor::MoveUp(total_lines as u16));
            }
        }
    }

    /// Check if a spinner is currently active.
    pub fn has_active_spinner(&self) -> bool {
        self.active_spinner.is_some()
    }

    /// Ensure terminal state is clean (restore cursor visibility, etc.)
    pub fn cleanup(&mut self) {
        let mut w = io::stderr();
        let _ = execute!(w, cursor::Show);
        let _ = w.flush();
    }
}

impl Drop for TuiRenderer {
    fn drop(&mut self) {
        self.cleanup();
    }
}
