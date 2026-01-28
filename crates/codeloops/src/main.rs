mod api;
mod config;
mod init;
mod prompt;
mod sessions;
mod ui;

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;

use codeloops_agent::{create_agent, AgentType};
use codeloops_core::{LoopContext, LoopOutcome, LoopRunner};
use codeloops_git::DiffCapture;
use codeloops_logging::{LogFormat, Logger, SessionWriter};

use config::{GlobalConfig, ProjectConfig};

#[derive(Parser, Debug)]
#[command(
    name = "codeloops",
    about = "Actor-critic harness for coding agents",
    version,
    author
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

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

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run the actor-critic loop (default behavior)
    Run {
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
    },

    /// Browse and inspect sessions
    Sessions {
        #[command(subcommand)]
        action: sessions::SessionsAction,
    },

    /// Start the web UI for browsing sessions
    Ui {
        /// Run in development mode (uses bun dev server)
        #[arg(long)]
        dev: bool,

        /// Port for the API server
        #[arg(long, default_value = "3100")]
        api_port: u16,

        /// Port for the UI server
        #[arg(long, default_value = "3101")]
        ui_port: u16,
    },

    /// Set up codeloops with interactive configuration
    Init,

    /// Interactively generate a prompt.md file with AI assistance
    Prompt {
        /// Output file path (default: ./prompt.md)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Working directory for project scanning (default: current directory)
        #[arg(short = 'd', long)]
        working_dir: Option<PathBuf>,

        /// Agent to use for the interview
        #[arg(short, long, value_enum)]
        agent: Option<AgentChoice>,

        /// Model to use (if agent supports it)
        #[arg(short, long)]
        model: Option<String>,

        /// Resume a previous interview session
        #[arg(long)]
        resume: Option<PathBuf>,
    },
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

    // First-run hint (non-blocking) for commands that benefit from config
    if init::is_first_run() {
        let should_hint = matches!(
            &cli.command,
            None | Some(Commands::Run { .. }) | Some(Commands::Sessions { .. })
        );
        if should_hint {
            eprintln!(
                "{} First time? Run {} to set up your defaults.\n",
                "->".dimmed(),
                "codeloops init".bright_cyan()
            );
        }
    }

    match cli.command {
        Some(Commands::Init) => init::handle_init().await,
        Some(Commands::Sessions { action }) => sessions::handle_sessions_command(action).await,
        Some(Commands::Ui {
            dev,
            api_port,
            ui_port,
        }) => ui::handle_ui_command(dev, api_port, ui_port).await,
        Some(Commands::Prompt {
            output,
            working_dir,
            agent,
            model,
            resume,
        }) => {
            prompt::handle_prompt_command(prompt::PromptArgs {
                output,
                working_dir,
                agent: agent.map(Into::into),
                model,
                resume,
            })
            .await
        }
        Some(Commands::Run {
            prompt,
            prompt_file,
            working_dir,
            agent,
            actor_agent,
            critic_agent,
            max_iterations,
            log_format,
            log_file,
            model,
            json_output,
            dry_run,
            no_color,
        }) => {
            run_loop(RunArgs {
                prompt,
                prompt_file,
                working_dir,
                agent,
                actor_agent,
                critic_agent,
                max_iterations,
                log_format,
                log_file,
                model,
                json_output,
                dry_run,
                no_color,
            })
            .await
        }
        None => {
            // Backwards compatibility: treat top-level args as implicit `run`
            run_loop(RunArgs {
                prompt: cli.prompt,
                prompt_file: cli.prompt_file,
                working_dir: cli.working_dir,
                agent: cli.agent,
                actor_agent: cli.actor_agent,
                critic_agent: cli.critic_agent,
                max_iterations: cli.max_iterations,
                log_format: cli.log_format,
                log_file: cli.log_file,
                model: cli.model,
                json_output: cli.json_output,
                dry_run: cli.dry_run,
                no_color: cli.no_color,
            })
            .await
        }
    }
}

struct RunArgs {
    prompt: Option<String>,
    prompt_file: PathBuf,
    working_dir: Option<PathBuf>,
    agent: Option<AgentChoice>,
    actor_agent: Option<AgentChoice>,
    critic_agent: Option<AgentChoice>,
    max_iterations: Option<usize>,
    log_format: LogFormatChoice,
    log_file: Option<PathBuf>,
    model: Option<String>,
    json_output: bool,
    dry_run: bool,
    no_color: bool,
}

async fn run_loop(args: RunArgs) -> Result<()> {
    // Handle no-color flag
    if args.no_color {
        colored::control::set_override(false);
    }

    // Determine working directory
    let working_dir = args
        .working_dir
        .clone()
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));

    // Load global config (hard error if file exists but is invalid)
    let global_config = GlobalConfig::load().context("Failed to load global configuration")?;

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
    let prompt = get_prompt(&args.prompt, &args.prompt_file, &working_dir)?;

    // Create logger (with optional file output)
    let log_format: LogFormat = args.log_format.into();
    let logger = if let Some(ref log_path) = args.log_file {
        Logger::with_file(log_format, log_path).context("Failed to create file logger")?
    } else {
        Logger::new(log_format)
    };

    // Determine actor agent
    // Precedence: CLI flags > project config > global config > default (Claude)
    let actor_agent = args
        .actor_agent
        .or(args.agent)
        .or_else(|| {
            project_config
                .as_ref()
                .and_then(|c| c.actor_agent())
                .and_then(parse_agent_choice)
        })
        .or_else(|| {
            global_config
                .as_ref()
                .and_then(|c| c.actor_agent())
                .and_then(parse_agent_choice)
        })
        .unwrap_or(AgentChoice::Claude);

    // Determine critic agent
    // Precedence: CLI flags > project config > global config > default (Claude)
    let critic_agent = args
        .critic_agent
        .or(args.agent)
        .or_else(|| {
            project_config
                .as_ref()
                .and_then(|c| c.critic_agent())
                .and_then(parse_agent_choice)
        })
        .or_else(|| {
            global_config
                .as_ref()
                .and_then(|c| c.critic_agent())
                .and_then(parse_agent_choice)
        })
        .unwrap_or(AgentChoice::Claude);

    let actor_type: AgentType = actor_agent.into();
    let critic_type: AgentType = critic_agent.into();

    // Determine models
    // Precedence: CLI flags > project config > global config > None
    let actor_model = args
        .model
        .clone()
        .or_else(|| {
            project_config
                .as_ref()
                .and_then(|c| c.actor_model())
                .map(String::from)
        })
        .or_else(|| {
            global_config
                .as_ref()
                .and_then(|c| c.actor_model())
                .map(String::from)
        });

    let critic_model = args
        .model
        .clone()
        .or_else(|| {
            project_config
                .as_ref()
                .and_then(|c| c.critic_model())
                .map(String::from)
        })
        .or_else(|| {
            global_config
                .as_ref()
                .and_then(|c| c.critic_model())
                .map(String::from)
        });

    if args.dry_run {
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
            args.max_iterations
                .map(|n| n.to_string())
                .unwrap_or_else(|| "unlimited".to_string())
        );
        if let Some(ref log_path) = args.log_file {
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
            "Agent '{}' is not available.\n\n  \
             Install it or choose a different agent:\n    \
             codeloops --agent opencode\n\n  \
             Available agents: claude, opencode, cursor",
            actor.name()
        );
    }
    if !critic.is_available().await {
        anyhow::bail!(
            "Agent '{}' is not available.\n\n  \
             Install it or choose a different agent:\n    \
             codeloops --critic-agent opencode\n\n  \
             Available agents: claude, opencode, cursor",
            critic.name()
        );
    }

    // Create session writer
    let session_writer = match SessionWriter::new(&prompt) {
        Ok(sw) => Some(Arc::new(sw)),
        Err(e) => {
            eprintln!(
                "{} Failed to create session writer: {}",
                "⚠".bright_yellow(),
                e
            );
            None
        }
    };

    // Create loop context
    let mut context = LoopContext::new(prompt, working_dir.clone());
    if let Some(max) = args.max_iterations {
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
        session_writer.clone(),
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
    if args.json_output {
        let json = serde_json::to_string_pretty(&outcome)?;
        println!("{}", json);
    } else {
        print_outcome(&outcome);
    }

    // Print session log path and hints
    if let Some(ref sw) = session_writer {
        eprintln!("{} Session log: {}", "->".dimmed(), sw.path().display());

        // Extract the session ID from the path and show helpful commands
        if let Some(stem) = sw.path().file_stem().and_then(|s| s.to_str()) {
            eprintln!();
            eprintln!(
                "  View this session: {}",
                format!("codeloops sessions show {}", stem).bright_cyan()
            );
            eprintln!("  Browse all sessions: {}", "codeloops ui".bright_cyan());
        }
    }

    // Exit with appropriate code
    std::process::exit(outcome.exit_code());
}

fn get_prompt(prompt: &Option<String>, prompt_file: &Path, working_dir: &Path) -> Result<String> {
    if let Some(ref prompt) = prompt {
        return Ok(prompt.clone());
    }

    let prompt_path = if prompt_file.is_absolute() {
        prompt_file.to_path_buf()
    } else {
        working_dir.join(prompt_file)
    };

    if prompt_path.exists() {
        let content =
            std::fs::read_to_string(&prompt_path).context("Failed to read prompt file")?;
        Ok(content.trim().to_string())
    } else {
        anyhow::bail!(
            "No prompt provided.\n\n  \
             Create a prompt.md in your project directory:\n    \
             echo \"Your task description\" > prompt.md\n\n  \
             Or pass it directly:\n    \
             codeloops --prompt \"Fix the auth bug in login.rs\""
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
