/// Main TUI renderer with in-place spinner updates.
///
/// Uses crossterm for cursor manipulation on stderr. Output stays in
/// the terminal scrollback — no alternate screen.
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Instant;

use crossterm::{cursor, execute, terminal};

use codeloops_logging::FileChangeType;

use crate::layout::{
    self, content_indent, pad_label, rule, shorten_home, wrap_text, COLLAPSED_SHOW, MARGIN,
    MAX_FILE_EVENTS,
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
    /// Number of file event lines printed below the spinner
    file_lines_below: usize,
    /// Whether we've printed the spinner line at least once
    printed: bool,
}

/// TTY-aware renderer with in-place spinner updates.
pub struct TuiRenderer {
    max_iterations: Option<usize>,
    actor_agent_name: Option<String>,
    critic_agent_name: Option<String>,
    /// Active spinner state (during actor/critic execution)
    active_spinner: Option<SpinnerState>,
    /// File events accumulated during the current actor phase
    current_file_events: Vec<FileEvent>,
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
            current_file_events: Vec::new(),
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
    pub fn tick(&mut self) {
        if let Some(ref mut state) = self.active_spinner {
            let frame = state.spinner.tick();
            let elapsed_secs = state.start.elapsed().as_secs();
            let elapsed_str = format_elapsed(elapsed_secs);

            let mut w = io::stderr();

            // Move cursor up to the spinner line, overwrite it
            if state.printed {
                // Move up past file lines + the blank line separator (if files exist) + spinner line
                let lines_to_go_up = if state.file_lines_below > 0 {
                    state.file_lines_below + 1 + 1 // file lines + blank separator + spinner line itself
                } else {
                    1 // just the spinner line
                };
                let _ = execute!(w, cursor::MoveUp(lines_to_go_up as u16));
            }

            // Clear and rewrite spinner line
            let _ = execute!(w, terminal::Clear(terminal::ClearType::CurrentLine));
            let _ = write!(
                w,
                "\r{}{}{} {}",
                MARGIN,
                dim(&pad_label(&state.label)),
                cyan_char(&frame.to_string()),
                dim_cyan(&elapsed_str),
            );
            let _ = writeln!(w);

            // Reprint file events below
            if state.file_lines_below > 0 {
                let _ = writeln!(w); // blank separator
                let start = if self.current_file_events.len() > MAX_FILE_EVENTS {
                    // Show first COLLAPSED_SHOW, then "... and N more"
                    for fe in self.current_file_events.iter().take(COLLAPSED_SHOW) {
                        let sigil = match fe.change_type {
                            FileChangeType::Created => green("+"),
                            FileChangeType::Modified => yellow("~"),
                            FileChangeType::Deleted => red("-"),
                        };
                        let _ = writeln!(w, "{}{} {}", content_indent(), sigil, fe.path);
                    }
                    let remaining = self.current_file_events.len() - COLLAPSED_SHOW;
                    let _ = writeln!(
                        w,
                        "{}{}",
                        content_indent(),
                        dim(&format!("… and {} more files", remaining))
                    );
                    0 // sentinel, we handled it
                } else {
                    for fe in &self.current_file_events {
                        let sigil = match fe.change_type {
                            FileChangeType::Created => green("+"),
                            FileChangeType::Modified => yellow("~"),
                            FileChangeType::Deleted => red("-"),
                        };
                        let _ = writeln!(w, "{}{} {}", content_indent(), sigil, fe.path);
                    }
                    0
                };
                let _ = start; // suppress unused
            }

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
                let _ = writeln!(
                    w,
                    "{}{} {}",
                    MARGIN,
                    bold("codeloops"),
                    dim(&title_rule)
                );
                let _ = writeln!(w);

                let dir_display = shorten_home(&working_dir.display().to_string());
                let width = layout::term_width();

                // prompt (may wrap)
                let prompt_lines = wrap_text(prompt, width);
                for (i, line) in prompt_lines.iter().enumerate() {
                    if i == 0 {
                        let _ = writeln!(
                            w,
                            "{}{}{}",
                            MARGIN,
                            dim(&pad_label("prompt")),
                            line
                        );
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
                    let _ = writeln!(
                        w,
                        "{}{}{}",
                        MARGIN,
                        dim(&pad_label("agents")),
                        agents_str
                    );
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
                let _ = writeln!(
                    w,
                    "{}{} {}",
                    MARGIN,
                    bold(&iter_display),
                    dim(&iter_rule)
                );
                let _ = writeln!(w);
            }

            RenderEvent::ActorStart => {
                self.current_file_events.clear();
                self.active_spinner = Some(SpinnerState {
                    spinner: Spinner::new(),
                    start: Instant::now(),
                    label: "actor".to_string(),
                    file_lines_below: 0,
                    printed: false,
                });
                // Print initial spinner line
                self.tick();
            }

            RenderEvent::FileChange(file_event) => {
                self.current_file_events.push(file_event.clone());
                // Update the count of file lines below the spinner
                if let Some(ref mut state) = self.active_spinner {
                    if self.current_file_events.len() > MAX_FILE_EVENTS {
                        state.file_lines_below = COLLAPSED_SHOW + 1; // collapsed lines + "... and N more"
                    } else {
                        state.file_lines_below = self.current_file_events.len();
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

                // Use file_events from the event if provided, otherwise use accumulated
                let events_to_show = if !file_events.is_empty() {
                    file_events
                } else {
                    &self.current_file_events
                };

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

                // File events
                if !events_to_show.is_empty() {
                    for fe in events_to_show {
                        let sigil = match fe.change_type {
                            FileChangeType::Created => green("+"),
                            FileChangeType::Modified => yellow("~"),
                            FileChangeType::Deleted => red("-"),
                        };
                        let _ = writeln!(w, "{}{} {}", content_indent(), sigil, fe.path);
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

                self.current_file_events.clear();
            }

            RenderEvent::CriticStart => {
                self.active_spinner = Some(SpinnerState {
                    spinner: Spinner::new(),
                    start: Instant::now(),
                    label: "critic".to_string(),
                    file_lines_below: 0,
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
                    let _ = writeln!(
                        w,
                        "{}confidence {}",
                        MARGIN,
                        dim(conf_text),
                    );
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

    /// Clear the spinner + file event lines from the terminal.
    fn clear_spinner_area(&self, _w: &mut impl Write) {
        if let Some(ref state) = self.active_spinner {
            if state.printed {
                // Move up to the spinner line
                let total_lines = if state.file_lines_below > 0 {
                    state.file_lines_below + 1 + 1 // file lines + blank separator + spinner line
                } else {
                    1
                };
                let _ = execute!(
                    io::stderr(),
                    cursor::MoveUp(total_lines as u16),
                );
                // Clear all lines from spinner down
                for _ in 0..total_lines {
                    let _ = execute!(
                        io::stderr(),
                        terminal::Clear(terminal::ClearType::CurrentLine),
                        cursor::MoveDown(1),
                    );
                }
                // Move back up to where we started clearing
                let _ = execute!(
                    io::stderr(),
                    cursor::MoveUp(total_lines as u16),
                );
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
