mod config;

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use colored::Colorize;

use codeloops_agent::{create_agent, AgentType};
use codeloops_core::{LoopContext, LoopOutcome, LoopRunner};
use codeloops_git::DiffCapture;
use codeloops_logging::{LogFormat, Logger};

use config::ProjectConfig;

#[derive(Parser, Debug)]
#[command(
    name = "codeloops",
    about = "Actor-critic harness for coding agents",
    version,
    author
)]
struct Cli {
    /// Task prompt (or reads from prompt.md if not provided)
    #[arg(short, long)]
    prompt: Option<String>,

    /// Path to prompt file (default: ./prompt.md)
    #[arg(long, default_value = "prompt.md")]
    prompt_file: PathBuf,

    /// Working directory (default: current directory)
    #[arg(short = 'd', long)]
    working_dir: Option<PathBuf>,

    /// Agent to use for both actor and critic (overrides config file)
    #[arg(short, long, value_enum)]
    agent: Option<AgentChoice>,

    /// Agent to use specifically for the actor role
    #[arg(long, value_enum)]
    actor_agent: Option<AgentChoice>,

    /// Agent to use specifically for the critic role
    #[arg(long, value_enum)]
    critic_agent: Option<AgentChoice>,

    /// Maximum iterations (default: unlimited)
    #[arg(short = 'n', long)]
    max_iterations: Option<usize>,

    /// Log output format
    #[arg(long, value_enum, default_value = "pretty")]
    log_format: LogFormatChoice,

    /// Write structured logs to a file (JSON format)
    #[arg(long)]
    log_file: Option<PathBuf>,

    /// Model to use (if agent supports it)
    #[arg(short, long)]
    model: Option<String>,

    /// Output final result as JSON
    #[arg(long)]
    json_output: bool,

    /// Dry run: show what would happen without executing
    #[arg(long)]
    dry_run: bool,

    /// Disable colored output
    #[arg(long)]
    no_color: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum AgentChoice {
    Claude,
    Opencode,
    Cursor,
}

impl From<AgentChoice> for AgentType {
    fn from(choice: AgentChoice) -> Self {
        match choice {
            AgentChoice::Claude => AgentType::ClaudeCode,
            AgentChoice::Opencode => AgentType::OpenCode,
            AgentChoice::Cursor => AgentType::Cursor,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum LogFormatChoice {
    Pretty,
    Json,
    Compact,
}

impl From<LogFormatChoice> for LogFormat {
    fn from(choice: LogFormatChoice) -> Self {
        match choice {
            LogFormatChoice::Pretty => LogFormat::Pretty,
            LogFormatChoice::Json => LogFormat::Json,
            LogFormatChoice::Compact => LogFormat::Compact,
        }
    }
}

/// Parse agent string from config file to AgentChoice
fn parse_agent_choice(s: &str) -> Option<AgentChoice> {
    match s.to_lowercase().as_str() {
        "claude" | "claude-code" => Some(AgentChoice::Claude),
        "opencode" | "open-code" => Some(AgentChoice::Opencode),
        "cursor" => Some(AgentChoice::Cursor),
        _ => None,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle no-color flag
    if cli.no_color {
        colored::control::set_override(false);
    }

    // Determine working directory
    let working_dir = cli
        .working_dir
        .clone()
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));

    // Load project config (hard error if file exists but is invalid)
    let project_config =
        ProjectConfig::load(&working_dir).context("Failed to load project configuration")?;

    if project_config.is_some() {
        eprintln!(
            "{} Loaded config from {}",
            "->".dimmed(),
            working_dir.join(config::CONFIG_FILE_NAME).display()
        );
    }

    // Get prompt
    let prompt = get_prompt(&cli, &working_dir)?;

    // Create logger (with optional file output)
    let log_format: LogFormat = cli.log_format.into();
    let logger = if let Some(ref log_path) = cli.log_file {
        Logger::with_file(log_format, log_path).context("Failed to create file logger")?
    } else {
        Logger::new(log_format)
    };

    // Determine actor agent: CLI --actor-agent > CLI --agent > config [actor].agent > config agent > default
    let actor_agent = cli
        .actor_agent
        .or(cli.agent) // CLI --agent overrides config
        .or_else(|| {
            project_config
                .as_ref()
                .and_then(|c| c.actor_agent())
                .and_then(parse_agent_choice)
        })
        .unwrap_or(AgentChoice::Claude);

    // Determine critic agent: CLI --critic-agent > CLI --agent > config [critic].agent > config agent > default
    let critic_agent = cli
        .critic_agent
        .or(cli.agent) // CLI --agent overrides config
        .or_else(|| {
            project_config
                .as_ref()
                .and_then(|c| c.critic_agent())
                .and_then(parse_agent_choice)
        })
        .unwrap_or(AgentChoice::Claude);

    let actor_type: AgentType = actor_agent.into();
    let critic_type: AgentType = critic_agent.into();

    // Determine models: CLI --model > config [role].model > config model > None
    let actor_model = cli.model.clone().or_else(|| {
        project_config
            .as_ref()
            .and_then(|c| c.actor_model())
            .map(String::from)
    });

    let critic_model = cli.model.clone().or_else(|| {
        project_config
            .as_ref()
            .and_then(|c| c.critic_model())
            .map(String::from)
    });

    if cli.dry_run {
        println!("{}", "=== Dry Run ===".bright_blue().bold());
        println!(
            "{}  {}",
            "Prompt:".dimmed(),
            if prompt.len() > 100 {
                format!("{}...", &prompt[..100])
            } else {
                prompt.clone()
            }
        );
        println!("{}  {}", "Dir:".dimmed(), working_dir.display());
        println!("{}  {}", "Actor:".dimmed(), actor_type);
        if let Some(ref model) = actor_model {
            println!("{}  {}", "Actor model:".dimmed(), model);
        }
        println!("{}  {}", "Critic:".dimmed(), critic_type);
        if let Some(ref model) = critic_model {
            println!("{}  {}", "Critic model:".dimmed(), model);
        }
        println!(
            "{}  {}",
            "Max iterations:".dimmed(),
            cli.max_iterations
                .map(|n| n.to_string())
                .unwrap_or_else(|| "unlimited".to_string())
        );
        if let Some(ref log_path) = cli.log_file {
            println!("{}  {}", "Log file:".dimmed(), log_path.display());
        }
        return Ok(());
    }

    // Create agents
    let actor = create_agent(actor_type);
    let critic = create_agent(critic_type);

    // Verify agents are available
    if !actor.is_available().await {
        anyhow::bail!(
            "Actor agent '{}' is not available. Make sure it's installed and in PATH.",
            actor.name()
        );
    }
    if !critic.is_available().await {
        anyhow::bail!(
            "Critic agent '{}' is not available. Make sure it's installed and in PATH.",
            critic.name()
        );
    }

    // Create loop context
    let mut context = LoopContext::new(prompt, working_dir.clone());
    if let Some(max) = cli.max_iterations {
        context = context.with_max_iterations(max);
    }

    // Create loop runner
    let diff_capture = DiffCapture::new();
    let logger = Arc::new(logger);
    let runner = LoopRunner::new(
        actor.as_ref(),
        critic.as_ref(),
        diff_capture,
        logger,
        actor_model,
        critic_model,
    );

    // Handle Ctrl+C gracefully
    let interrupt_handle = runner.interrupt_handle();
    ctrlc::set_handler(move || {
        eprintln!(
            "\n{} Interrupted. Finishing current iteration...",
            "⚠".bright_yellow()
        );
        interrupt_handle.store(true, Ordering::SeqCst);
    })
    .context("Failed to set Ctrl+C handler")?;

    // Run the loop
    let outcome = runner.run(context).await?;

    // Output result
    if cli.json_output {
        let json = serde_json::to_string_pretty(&outcome)?;
        println!("{}", json);
    } else {
        print_outcome(&outcome);
    }

    // Exit with appropriate code
    std::process::exit(outcome.exit_code());
}

fn get_prompt(cli: &Cli, working_dir: &Path) -> Result<String> {
    // Prefer --prompt flag
    if let Some(ref prompt) = cli.prompt {
        return Ok(prompt.clone());
    }

    // Try to read from prompt file
    let prompt_path = if cli.prompt_file.is_absolute() {
        cli.prompt_file.clone()
    } else {
        working_dir.join(&cli.prompt_file)
    };

    if prompt_path.exists() {
        let content =
            std::fs::read_to_string(&prompt_path).context("Failed to read prompt file")?;
        Ok(content.trim().to_string())
    } else {
        anyhow::bail!(
            "No prompt provided. Use --prompt or create a {} file",
            cli.prompt_file.display()
        )
    }
}

fn print_outcome(outcome: &LoopOutcome) {
    let mut stderr = std::io::stderr();
    use std::io::Write;

    match outcome {
        LoopOutcome::Success {
            iterations,
            summary,
            confidence,
            total_duration_secs,
            ..
        } => {
            let _ = writeln!(stderr);
            let _ = writeln!(
                stderr,
                "{} {} in {} {} ({:.1}s)",
                "✅".bright_green(),
                "SUCCESS".bright_green().bold(),
                iterations,
                if *iterations == 1 {
                    "iteration"
                } else {
                    "iterations"
                },
                total_duration_secs
            );
            let _ = writeln!(
                stderr,
                "   {} {:.0}%",
                "Confidence:".dimmed(),
                confidence * 100.0
            );
            // Word-wrap the summary at ~70 chars
            let wrapped = wrap_text(summary, 70);
            for (i, line) in wrapped.iter().enumerate() {
                if i == 0 {
                    let _ = writeln!(stderr, "   {} {}", "Summary:".dimmed(), line);
                } else {
                    let _ = writeln!(stderr, "            {}", line);
                }
            }
        }
        LoopOutcome::MaxIterationsReached {
            iterations,
            total_duration_secs,
            ..
        } => {
            let _ = writeln!(stderr);
            let _ = writeln!(
                stderr,
                "{} {} after {} iterations ({:.1}s)",
                "⚠".bright_yellow(),
                "INCOMPLETE".bright_yellow().bold(),
                iterations,
                total_duration_secs
            );
            let _ = writeln!(
                stderr,
                "   {}",
                "The task may not be fully complete.".dimmed()
            );
        }
        LoopOutcome::UserInterrupted {
            iterations,
            total_duration_secs,
            ..
        } => {
            let _ = writeln!(stderr);
            let _ = writeln!(
                stderr,
                "{} {} after {} {} ({:.1}s)",
                "⏸".bright_yellow(),
                "INTERRUPTED".bright_yellow().bold(),
                iterations,
                if *iterations == 1 {
                    "iteration"
                } else {
                    "iterations"
                },
                total_duration_secs
            );
        }
        LoopOutcome::Failed {
            iterations,
            error,
            total_duration_secs,
            ..
        } => {
            let _ = writeln!(stderr);
            let _ = writeln!(
                stderr,
                "{} {} after {} {} ({:.1}s)",
                "❌".bright_red(),
                "FAILED".bright_red().bold(),
                iterations,
                if *iterations == 1 {
                    "iteration"
                } else {
                    "iterations"
                },
                total_duration_secs
            );
            let _ = writeln!(stderr, "   {} {}", "Error:".dimmed(), error.bright_red());
        }
    }
    let _ = writeln!(stderr);
}

/// Wrap text to a maximum line width
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() <= max_width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}
