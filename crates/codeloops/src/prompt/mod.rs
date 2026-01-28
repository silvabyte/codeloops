//! Interactive prompt.md generator with TUI.
//!
//! This module provides a TUI-based interview system that uses the configured
//! coding agent to generate comprehensive prompt.md files through a series
//! of probing questions.

mod protocol;
mod scanner;
mod session;
mod system_prompt;
mod tui;

use std::path::PathBuf;

use anyhow::{Context, Result};
use colored::Colorize;

use codeloops_agent::{create_agent, AgentConfig, AgentType};

use crate::config::{GlobalConfig, ProjectConfig};

// Re-export for use by tui module
pub(crate) use session::*;

/// Arguments for the prompt command
pub struct PromptArgs {
    pub output: Option<PathBuf>,
    pub working_dir: Option<PathBuf>,
    pub agent: Option<AgentType>,
    pub model: Option<String>,
    pub resume: Option<PathBuf>,
}

/// Handle the `codeloops prompt` command
pub async fn handle_prompt_command(args: PromptArgs) -> Result<()> {
    // Determine working directory
    let working_dir = args
        .working_dir
        .clone()
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));

    // Load configs
    let global_config = GlobalConfig::load().context("Failed to load global configuration")?;
    let project_config =
        ProjectConfig::load(&working_dir).context("Failed to load project configuration")?;

    // Resolve agent
    // Precedence: CLI flags > project config > global config > default (Claude)
    let agent_type = args
        .agent
        .or_else(|| {
            project_config
                .as_ref()
                .and_then(|c| c.actor_agent())
                .and_then(parse_agent_type)
        })
        .or_else(|| {
            global_config
                .as_ref()
                .and_then(|c| c.actor_agent())
                .and_then(parse_agent_type)
        })
        .unwrap_or(AgentType::ClaudeCode);

    // Resolve model
    let model = args
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

    // Create agent and verify availability
    let agent = create_agent(agent_type);
    if !agent.is_available().await {
        anyhow::bail!(
            "Agent '{}' is not available.\n\n  \
             Install it or choose a different agent:\n    \
             codeloops prompt --agent opencode\n\n  \
             Available agents: claude, opencode, cursor",
            agent.name()
        );
    }

    // Determine output path
    let output_path = args
        .output
        .clone()
        .unwrap_or_else(|| working_dir.join("prompt.md"));

    // Create agent config
    let agent_config = AgentConfig::new(working_dir.clone());
    let agent_config = if let Some(ref m) = model {
        agent_config.with_model(m.clone())
    } else {
        agent_config
    };

    // Load or create session
    let session = if let Some(ref resume_path) = args.resume {
        eprintln!(
            "{} Resuming session from {}",
            "->".dimmed(),
            resume_path.display()
        );
        InterviewSession::load(resume_path).context("Failed to load session")?
    } else {
        eprintln!(
            "{} Starting new interview with {}",
            "->".dimmed(),
            agent.name().bright_cyan()
        );

        // Scan project for context
        let project_context = scanner::scan_project(&working_dir)?;
        InterviewSession::new(project_context, output_path.clone())
    };

    // Run the TUI
    let mut app = tui::App::new(session, agent, agent_config)?;
    let result = app.run().await;

    // Handle the result
    match result {
        Ok(Some(final_draft)) => {
            // Write the final prompt.md
            std::fs::write(&output_path, final_draft.to_markdown())
                .context("Failed to write prompt.md")?;

            eprintln!();
            eprintln!(
                "{} Created {}",
                "✅".bright_green(),
                output_path.display().to_string().bright_cyan()
            );
            eprintln!();
            eprintln!(
                "  Run your prompt: {}",
                format!("codeloops --prompt-file {}", output_path.display()).bright_cyan()
            );
        }
        Ok(None) => {
            // User cancelled or session saved for later
            eprintln!();
            eprintln!("{} Session saved. Resume with:", "->".dimmed());
            if let Some(session_path) = app.session_path() {
                eprintln!(
                    "  {}",
                    format!("codeloops prompt --resume {}", session_path.display()).bright_cyan()
                );
            }
        }
        Err(e) => {
            // Error occurred - try to save session
            if let Some(session_path) = app.session_path() {
                eprintln!(
                    "\n{} Session saved to {}",
                    "->".dimmed(),
                    session_path.display()
                );
            }
            return Err(e);
        }
    }

    Ok(())
}

/// Parse agent type from config string
fn parse_agent_type(s: &str) -> Option<AgentType> {
    match s.to_lowercase().as_str() {
        "claude" | "claude-code" => Some(AgentType::ClaudeCode),
        "opencode" | "open-code" => Some(AgentType::OpenCode),
        "cursor" => Some(AgentType::Cursor),
        _ => None,
    }
}
