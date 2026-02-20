/// Non-TTY fallback renderer.
///
/// When stderr is not a TTY (piped, CI), we output sequential lines
/// with no cursor movement, no spinner animation. Same layout structure
/// but static output only. Respects NO_COLOR.
use std::io::{self, Write};

use codeloops_logging::FileChangeType;

use crate::layout::{self, content_indent, pad_label, rule, shorten_home, wrap_text, MARGIN};
use crate::renderer::RenderEvent;
use crate::spinner::format_elapsed;

/// Whether color is enabled (respects NO_COLOR env var and --no-color flag).
fn use_color() -> bool {
    // colored crate already respects NO_COLOR and CLICOLOR
    colored::control::SHOULD_COLORIZE.should_colorize()
}

/// Apply dim styling if color is enabled
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


/// Fallback renderer for non-TTY environments.
pub struct FallbackRenderer {
    max_iterations: Option<usize>,
    actor_agent_name: Option<String>,
    critic_agent_name: Option<String>,
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
            actor_agent_name: None,
            critic_agent_name: None,
        }
    }

    pub fn set_max_iterations(&mut self, max: Option<usize>) {
        self.max_iterations = max;
    }

    pub fn set_agent_names(&mut self, actor: &str, critic: &str) {
        self.actor_agent_name = Some(actor.to_string());
        self.critic_agent_name = Some(critic.to_string());
    }

    pub fn render(&self, event: &RenderEvent) {
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
                if let (Some(actor), Some(critic)) = (&self.actor_agent_name, &self.critic_agent_name) {
                    let agents_str = if let Some(max) = self.max_iterations {
                        format!(
                            "{} → {} · {} iterations max",
                            actor, critic, max
                        )
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
                let _ = writeln!(
                    w,
                    "{}{}...",
                    MARGIN,
                    dim(&pad_label("actor")),
                );
            }

            RenderEvent::FileChange(file_event) => {
                let sigil = match file_event.change_type {
                    FileChangeType::Created => green("+"),
                    FileChangeType::Modified => yellow("~"),
                    FileChangeType::Deleted => red("-"),
                };
                let _ = writeln!(w, "{}{} {}", content_indent(), sigil, file_event.path);
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
                    let _ = writeln!(
                        w,
                        "{}{}{} exit {}",
                        MARGIN,
                        dim(&pad_label("actor")),
                        red("✗"),
                        exit_code,
                    );
                }
                let _ = writeln!(w);

                // File events
                for fe in file_events {
                    let sigil = match fe.change_type {
                        FileChangeType::Created => green("+"),
                        FileChangeType::Modified => yellow("~"),
                        FileChangeType::Deleted => red("-"),
                    };
                    let _ = writeln!(w, "{}{} {}", content_indent(), sigil, fe.path);
                }
                if !file_events.is_empty() {
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
                let _ = writeln!(
                    w,
                    "{}{}...",
                    MARGIN,
                    dim(&pad_label("critic")),
                );
            }

            RenderEvent::CriticDone {
                duration_secs,
                decision_text: _,
                feedback,
            } => {
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
                        "{}{}{}",
                        MARGIN,
                        dim(&pad_label("confidence")),
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
    }
}
