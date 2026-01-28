//! Interactive initialization for codeloops.
//!
//! Sets up the global config file with user-selected defaults.

use anyhow::Result;
use colored::Colorize;
use dialoguer::Select;
use std::fs;

use codeloops_agent::{create_agent, AgentType};

use crate::config::{GlobalConfig, GLOBAL_CONFIG_DIR, GLOBAL_CONFIG_FILE};

/// Agent info for display and config
struct AgentInfo {
    display_name: &'static str,
    config_name: &'static str,
    agent_type: AgentType,
}

const AGENTS: &[AgentInfo] = &[
    AgentInfo {
        display_name: "Claude Code",
        config_name: "claude",
        agent_type: AgentType::ClaudeCode,
    },
    AgentInfo {
        display_name: "Opencode",
        config_name: "opencode",
        agent_type: AgentType::OpenCode,
    },
    AgentInfo {
        display_name: "Cursor",
        config_name: "cursor",
        agent_type: AgentType::Cursor,
    },
];

pub async fn handle_init() -> Result<()> {
    eprintln!("{}", "Setting up codeloops...".bold());
    eprintln!();

    // Step 1: Detect available agents
    eprintln!("{}", "Checking for available agents...".dimmed());

    let mut available: Vec<&AgentInfo> = Vec::new();

    for info in AGENTS {
        let agent = create_agent(info.agent_type);
        if agent.is_available().await {
            eprintln!(
                "  {} {} ({})",
                "✓".bright_green(),
                info.display_name,
                info.config_name
            );
            available.push(info);
        } else {
            eprintln!("  {} {} (not found)", "✗".dimmed(), info.display_name);
        }
    }

    eprintln!();

    if available.is_empty() {
        eprintln!(
            "{} No agents found in PATH. Install at least one:",
            "⚠".bright_yellow()
        );
        eprintln!("  Claude Code: https://docs.anthropic.com/claude-code");
        eprintln!("  Opencode:    https://opencode.ai/docs/#install");
        eprintln!("  Cursor:      https://cursor.com/cli");
        eprintln!();
        eprintln!(
            "After installing, run {} again.",
            "codeloops init".bright_cyan()
        );
        return Ok(());
    }

    // Step 2: Pick default agent
    let selection = if available.len() == 1 {
        eprintln!(
            "Using {} as your default agent (only one available).",
            available[0].display_name.bright_cyan()
        );
        0
    } else {
        let items: Vec<&str> = available.iter().map(|a| a.display_name).collect();
        Select::new()
            .with_prompt("Select your default agent")
            .items(&items)
            .default(0)
            .interact()?
    };

    let agent_info = available[selection];

    // Step 3: Write global config
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
        .join(GLOBAL_CONFIG_DIR);

    fs::create_dir_all(&config_dir)?;

    let config_path = config_dir.join(GLOBAL_CONFIG_FILE);

    let config_content = format!(
        r#"[defaults]
agent = "{}"
# model = ""  # Optional: set a default model

# Override per-role:
# [defaults.actor]
# agent = "claude"
# model = "sonnet"

# [defaults.critic]
# agent = "claude"
# model = "opus"
"#,
        agent_info.config_name
    );

    // Check if config already exists
    if config_path.exists() {
        eprintln!(
            "{} Config already exists at {}",
            "⚠".bright_yellow(),
            config_path.display()
        );

        let overwrite = Select::new()
            .with_prompt("Overwrite existing config?")
            .items(&["No, keep existing", "Yes, replace it"])
            .default(0)
            .interact()?;

        if overwrite == 0 {
            eprintln!();
            eprintln!("Keeping existing config. Edit it manually if needed:");
            eprintln!("  {}", config_path.display().to_string().dimmed());
            return Ok(());
        }
    }

    fs::write(&config_path, &config_content)?;

    eprintln!();
    eprintln!(
        "{} Config saved to {}",
        "✓".bright_green(),
        config_path.display()
    );

    // Step 4: Quick-start tips
    print_getting_started();

    Ok(())
}

/// Print the getting started guide
pub fn print_getting_started() {
    eprintln!();
    eprintln!("{}", "Getting started:".bold());
    eprintln!("  {} Navigate to a git repo", "1.".dimmed());
    eprintln!(
        "  {} Create a prompt.md describing your task (or use {})",
        "2.".dimmed(),
        "--prompt \"...\"".bright_cyan()
    );
    eprintln!("  {} Run: {}", "3.".dimmed(), "codeloops".bright_cyan());
    eprintln!(
        "  {} View sessions: {}",
        "4.".dimmed(),
        "codeloops sessions list".bright_cyan()
    );
    eprintln!(
        "  {} Launch the UI: {}",
        "5.".dimmed(),
        "codeloops ui".bright_cyan()
    );
}

/// Check if this appears to be first run (no global config)
pub fn is_first_run() -> bool {
    !GlobalConfig::exists()
}
