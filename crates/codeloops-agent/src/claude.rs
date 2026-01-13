use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::debug;

use crate::{
    Agent, AgentConfig, AgentError, AgentOutput, AgentType, OutputCallback, ProcessSpawner,
};

/// Claude Code agent implementation
pub struct ClaudeCodeAgent {
    binary_path: PathBuf,
}

impl ClaudeCodeAgent {
    pub fn new() -> Self {
        Self {
            binary_path: PathBuf::from("claude"),
        }
    }

    pub fn with_binary_path(path: PathBuf) -> Self {
        Self { binary_path: path }
    }
}

impl Default for ClaudeCodeAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Agent for ClaudeCodeAgent {
    fn name(&self) -> &str {
        "Claude Code"
    }

    fn agent_type(&self) -> AgentType {
        AgentType::ClaudeCode
    }

    fn binary_path(&self) -> &Path {
        &self.binary_path
    }

    async fn is_available(&self) -> bool {
        Command::new(&self.binary_path)
            .arg("--version")
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    async fn execute_with_callback(
        &self,
        prompt: &str,
        config: &AgentConfig,
        on_output: Option<OutputCallback>,
    ) -> Result<AgentOutput, AgentError> {
        debug!(
            agent = self.name(),
            prompt_len = prompt.len(),
            "Executing agent"
        );

        let mut args = vec![
            "--print",                        // Non-interactive mode, output only
            "--dangerously-skip-permissions", // Skip permission prompts
        ];

        // Add model if specified
        let model_arg;
        if let Some(ref model) = config.model {
            args.push("--model");
            model_arg = model.clone();
            args.push(&model_arg);
        }

        // Add -- to signal end of options, then the prompt as positional argument
        // This prevents prompts starting with '-' from being interpreted as options
        args.push("--");
        args.push(prompt);

        ProcessSpawner::spawn_with_callback(&self.binary_path, &args, config, on_output).await
    }
}
