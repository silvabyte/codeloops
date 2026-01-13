use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};

use codeloops_agent::{create_agent, AgentType};
use codeloops_core::{LoopContext, LoopOutcome, LoopRunner};
use codeloops_git::DiffCapture;
use codeloops_logging::{LogFormat, Logger};

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

    /// Agent to use for both actor and critic
    #[arg(short, long, value_enum, default_value = "claude")]
    agent: AgentChoice,

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

    /// Model to use (if agent supports it)
    #[arg(short, long)]
    model: Option<String>,

    /// Output final result as JSON
    #[arg(long)]
    json_output: bool,

    /// Dry run: show what would happen without executing
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum AgentChoice {
    Claude,
    Opencode,
}

impl From<AgentChoice> for AgentType {
    fn from(choice: AgentChoice) -> Self {
        match choice {
            AgentChoice::Claude => AgentType::ClaudeCode,
            AgentChoice::Opencode => AgentType::OpenCode,
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

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Determine working directory
    let working_dir = cli
        .working_dir
        .clone()
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));

    // Get prompt
    let prompt = get_prompt(&cli, &working_dir)?;

    // Create logger
    let log_format: LogFormat = cli.log_format.into();
    let logger = Logger::new(log_format);

    // Determine agent types
    let actor_type: AgentType = cli.actor_agent.unwrap_or(cli.agent).into();
    let critic_type: AgentType = cli.critic_agent.unwrap_or(cli.agent).into();

    if cli.dry_run {
        println!("=== Dry Run ===");
        println!(
            "Prompt: {}",
            if prompt.len() > 100 {
                format!("{}...", &prompt[..100])
            } else {
                prompt.clone()
            }
        );
        println!("Working dir: {}", working_dir.display());
        println!("Actor: {}", actor_type);
        println!("Critic: {}", critic_type);
        if let Some(max) = cli.max_iterations {
            println!("Max iterations: {}", max);
        } else {
            println!("Max iterations: unlimited");
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
    let runner = LoopRunner::new(actor.as_ref(), critic.as_ref(), diff_capture, logger);

    // Handle Ctrl+C gracefully
    let interrupt_handle = runner.interrupt_handle();
    ctrlc::set_handler(move || {
        eprintln!("\nInterrupted. Finishing current iteration...");
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
    match outcome {
        LoopOutcome::Success {
            iterations,
            summary,
            confidence,
            total_duration_secs,
            ..
        } => {
            eprintln!();
            eprintln!("=== SUCCESS ===");
            eprintln!("Iterations: {}", iterations);
            eprintln!("Confidence: {:.0}%", confidence * 100.0);
            eprintln!("Duration: {:.1}s", total_duration_secs);
            eprintln!("Summary: {}", summary);
        }
        LoopOutcome::MaxIterationsReached {
            iterations,
            total_duration_secs,
            ..
        } => {
            eprintln!();
            eprintln!("=== INCOMPLETE ===");
            eprintln!("Reached maximum iterations ({})", iterations);
            eprintln!("Duration: {:.1}s", total_duration_secs);
            eprintln!("The task may not be fully complete.");
        }
        LoopOutcome::UserInterrupted {
            iterations,
            total_duration_secs,
            ..
        } => {
            eprintln!();
            eprintln!("=== INTERRUPTED ===");
            eprintln!("User stopped after {} iteration(s)", iterations);
            eprintln!("Duration: {:.1}s", total_duration_secs);
        }
        LoopOutcome::Failed {
            iterations,
            error,
            total_duration_secs,
            ..
        } => {
            eprintln!();
            eprintln!("=== FAILED ===");
            eprintln!("Error after {} iteration(s): {}", iterations, error);
            eprintln!("Duration: {:.1}s", total_duration_secs);
        }
    }
}
