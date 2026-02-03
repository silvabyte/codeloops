use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;

use codeloops_db::{Database, Session, SessionFilter, SessionStats, SessionSummary};

#[derive(Subcommand, Debug)]
pub enum SessionsAction {
    /// List all sessions
    List {
        /// Filter by outcome (success, failed, interrupted, max_iterations_reached)
        #[arg(long)]
        outcome: Option<String>,

        /// Show sessions after this date (YYYY-MM-DD)
        #[arg(long)]
        after: Option<String>,

        /// Show sessions before this date (YYYY-MM-DD)
        #[arg(long)]
        before: Option<String>,

        /// Search prompt text
        #[arg(long)]
        search: Option<String>,

        /// Filter by project name
        #[arg(long)]
        project: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show detailed session info
    Show {
        /// Session ID (launches interactive picker if omitted)
        id: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show cumulative git diff from a session
    Diff {
        /// Session ID (launches interactive picker if omitted)
        id: Option<String>,
    },

    /// Show aggregate statistics
    Stats {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

pub async fn handle_sessions_command(action: SessionsAction) -> Result<()> {
    let db = Database::open()?;

    match action {
        SessionsAction::List {
            outcome,
            after,
            before,
            search,
            project,
            json,
        } => {
            let filter = build_filter(outcome, after, before, search, project)?;
            let summaries = db.sessions().list(&filter)?;

            if json {
                println!("{}", serde_json::to_string_pretty(&summaries)?);
            } else if summaries.is_empty() {
                eprintln!("{}", "No sessions found.".dimmed());
                eprintln!();
                eprintln!("  Run your first loop to create a session:");
                eprintln!("    {}", "codeloops --prompt \"Your task\"".bright_cyan());
            } else {
                print_sessions_table(&summaries);
            }
        }
        SessionsAction::Show { id, json } => {
            let id = resolve_session_id(&db, id)?;
            let session = db.sessions().get(&id)?.ok_or_else(|| {
                anyhow::anyhow!("Session not found: {}", id)
            })?;

            if json {
                println!("{}", serde_json::to_string_pretty(&session)?);
            } else {
                print_session_detail(&session);
            }
        }
        SessionsAction::Diff { id } => {
            let id = resolve_session_id(&db, id)?;
            let diff = db.sessions().get_diff(&id)?;

            match diff {
                Some(d) if !d.is_empty() => println!("{}", d),
                _ => println!("{}", "No diffs found for this session.".dimmed()),
            }
        }
        SessionsAction::Stats { json } => {
            let stats = db.sessions().stats(&SessionFilter::default())?;

            if json {
                println!("{}", serde_json::to_string_pretty(&stats)?);
            } else {
                print_stats(&stats);
            }
        }
    }

    Ok(())
}

fn build_filter(
    outcome: Option<String>,
    after: Option<String>,
    before: Option<String>,
    search: Option<String>,
    project: Option<String>,
) -> Result<SessionFilter> {
    use chrono::{NaiveDate, TimeZone, Utc};

    let after = after
        .map(|s| {
            NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                .map(|d| Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0).unwrap()))
                .map_err(|e| anyhow::anyhow!("Invalid --after date: {}", e))
        })
        .transpose()?;

    let before = before
        .map(|s| {
            NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                .map(|d| Utc.from_utc_datetime(&d.and_hms_opt(23, 59, 59).unwrap()))
                .map_err(|e| anyhow::anyhow!("Invalid --before date: {}", e))
        })
        .transpose()?;

    Ok(SessionFilter {
        outcome,
        after,
        before,
        search,
        project,
    })
}

fn resolve_session_id(db: &Database, id: Option<String>) -> Result<String> {
    if let Some(id) = id {
        return Ok(id);
    }

    // Interactive picker
    let summaries = db.sessions().list(&SessionFilter::default())?;
    if summaries.is_empty() {
        anyhow::bail!("No sessions found.");
    }

    let items: Vec<String> = summaries
        .iter()
        .map(|s| {
            let ts = s.timestamp.format("%Y-%m-%d %H:%M");
            let outcome = s.outcome.as_deref().unwrap_or("active").to_string();
            let duration = s
                .duration_secs
                .map(format_duration)
                .unwrap_or_else(|| "...".to_string());
            let prompt = if s.prompt_preview.len() > 60 {
                format!("{}...", &s.prompt_preview[..60])
            } else {
                s.prompt_preview.clone()
            };
            format!(
                "{} | {:8} ({} iters, {}) | \"{}\"",
                ts, outcome, s.iterations, duration, prompt
            )
        })
        .collect();

    let selection = dialoguer::FuzzySelect::new()
        .with_prompt("Select a session")
        .items(&items)
        .default(0)
        .interact()?;

    Ok(summaries[selection].id.clone())
}

fn print_sessions_table(summaries: &[SessionSummary]) {
    println!(
        "{:<20} {:<10} {:<6} {:<8} {:<12} {}",
        "TIMESTAMP".dimmed(),
        "OUTCOME".dimmed(),
        "ITERS".dimmed(),
        "DURATION".dimmed(),
        "PROJECT".dimmed(),
        "PROMPT".dimmed(),
    );

    for s in summaries {
        let ts = s.timestamp.format("%Y-%m-%d %H:%M").to_string();
        let outcome = s.outcome.as_deref().unwrap_or("active");
        let outcome_colored = match outcome {
            "success" => outcome.bright_green().to_string(),
            "failed" => outcome.bright_red().to_string(),
            "active" => outcome.bright_cyan().to_string(),
            _ => outcome.bright_yellow().to_string(),
        };
        let duration = s
            .duration_secs
            .map(format_duration)
            .unwrap_or_else(|| "...".to_string());
        let prompt = if s.prompt_preview.len() > 50 {
            format!("{}...", &s.prompt_preview[..50])
        } else {
            s.prompt_preview.clone()
        };

        println!(
            "{:<20} {:<10} {:<6} {:<8} {:<12} {}",
            ts, outcome_colored, s.iterations, duration, s.project, prompt
        );
    }
}

fn print_session_detail(session: &Session) {
    println!("{}", "=== Session Detail ===".bright_blue().bold());
    println!("{}  {}", "ID:".dimmed(), session.id);
    println!(
        "{}  {}",
        "Started:".dimmed(),
        session.started_at.format("%Y-%m-%d %H:%M:%S UTC")
    );
    println!(
        "{}  {}",
        "Working Dir:".dimmed(),
        session.working_dir.display()
    );
    println!("{}  {}", "Actor:".dimmed(), session.actor_agent);
    println!("{}  {}", "Critic:".dimmed(), session.critic_agent);
    if let Some(ref model) = session.actor_model {
        println!("{}  {}", "Actor Model:".dimmed(), model);
    }
    if let Some(ref model) = session.critic_model {
        println!("{}  {}", "Critic Model:".dimmed(), model);
    }
    println!();
    println!("{}", "Prompt:".dimmed());
    println!("  {}", session.prompt);
    println!();

    if let Some(ref outcome) = session.outcome {
        println!(
            "{}  {}",
            "Outcome:".dimmed(),
            match outcome.as_str() {
                "success" => outcome.bright_green().to_string(),
                "failed" => outcome.bright_red().to_string(),
                _ => outcome.bright_yellow().to_string(),
            }
        );
        if let Some(iterations) = session.iteration_count {
            println!("{}  {}", "Iterations:".dimmed(), iterations);
        }
        if let Some(duration) = session.duration_secs {
            println!(
                "{}  {}",
                "Duration:".dimmed(),
                format_duration(duration)
            );
        }
        if let Some(confidence) = session.confidence {
            println!("{}  {:.0}%", "Confidence:".dimmed(), confidence * 100.0);
        }
        if let Some(ref summary) = session.summary {
            println!("{}  {}", "Summary:".dimmed(), summary);
        }
    } else {
        println!("{}  {}", "Status:".dimmed(), "ACTIVE".bright_cyan());
    }

    if !session.iterations.is_empty() {
        println!();
        println!(
            "{}",
            format!("--- Iterations ({}) ---", session.iterations.len()).dimmed()
        );
        for iter in &session.iterations {
            println!();
            println!(
                "  {} {}",
                format!("[{}]", iter.iteration_number).bright_blue(),
                iter.timestamp.format("%H:%M:%S")
            );
            println!(
                "    {} {} (exit: {}, {:.1}s)",
                "Actor:".dimmed(),
                if iter.actor_exit_code == 0 {
                    "OK".bright_green().to_string()
                } else {
                    format!("ERR({})", iter.actor_exit_code)
                        .bright_red()
                        .to_string()
                },
                iter.actor_exit_code,
                iter.actor_duration_secs
            );
            println!(
                "    {} {} files changed",
                "Diff:".dimmed(),
                iter.git_files_changed
            );
            println!(
                "    {} {}",
                "Decision:".dimmed(),
                match iter.critic_decision.as_str() {
                    "DONE" => iter.critic_decision.bright_green().to_string(),
                    "CONTINUE" => iter.critic_decision.bright_yellow().to_string(),
                    _ => iter.critic_decision.bright_red().to_string(),
                }
            );
            if let Some(ref feedback) = iter.feedback {
                let preview = if feedback.len() > 120 {
                    format!("{}...", &feedback[..120])
                } else {
                    feedback.clone()
                };
                println!("    {} {}", "Feedback:".dimmed(), preview);
            }
        }
    }
}

fn print_stats(stats: &SessionStats) {
    println!("{}", "=== Session Statistics ===".bright_blue().bold());
    println!("{}  {}", "Total Sessions:".dimmed(), stats.total_sessions);
    println!(
        "{}  {:.1}%",
        "Success Rate:".dimmed(),
        stats.success_rate * 100.0
    );
    println!(
        "{}  {:.1}",
        "Avg Iterations:".dimmed(),
        stats.avg_iterations
    );
    println!(
        "{}  {}",
        "Avg Duration:".dimmed(),
        format_duration(stats.avg_duration_secs)
    );

    if !stats.by_project.is_empty() {
        println!();
        println!("{}", "By Project:".dimmed());
        for p in &stats.by_project {
            println!(
                "  {:<20} {} sessions, {:.0}% success",
                p.project,
                p.total,
                p.success_rate * 100.0
            );
        }
    }
}

fn format_duration(secs: f64) -> String {
    if secs < 60.0 {
        format!("{:.0}s", secs)
    } else {
        let mins = (secs / 60.0).floor() as u64;
        let remaining_secs = (secs % 60.0) as u64;
        format!("{}m {}s", mins, remaining_secs)
    }
}
