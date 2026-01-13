use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::debug;

use crate::{
    Agent, AgentConfig, AgentError, AgentOutput, AgentType, OutputCallback, ProcessSpawner,
};

/// Cursor agent CLI implementation
pub struct CursorAgent {
    binary_path: PathBuf,
}

impl CursorAgent {
    pub fn new() -> Self {
        Self {
            binary_path: PathBuf::from("cursor"),
        }
    }

    pub fn with_binary_path(path: PathBuf) -> Self {
        Self { binary_path: path }
    }
}

impl Default for CursorAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Agent for CursorAgent {
    fn name(&self) -> &str {
        "Cursor"
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Cursor
    }

    fn binary_path(&self) -> &Path {
        &self.binary_path
    }

    async fn is_available(&self) -> bool {
        // Check if the cursor CLI agent subcommand is available
        Command::new(&self.binary_path)
            .args(["agent", "--help"])
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

        // Build args for cursor agent CLI
        // Usage: cursor agent -p "prompt" --model "model" --output-format text
        let mut args = vec!["agent", "-p", prompt];

        // Add model if specified (default to opus-4.5-thinking per requirements)
        let model_arg;
        if let Some(ref model) = config.model {
            model_arg = model.clone();
        } else {
            // Default to opus-4.5-thinking as requested
            model_arg = "opus-4.5-thinking".to_string();
        }
        args.push("--model");
        args.push(&model_arg);

        // Use text output format for non-interactive mode
        args.push("--output-format");
        args.push("text");

        ProcessSpawner::spawn_with_callback(&self.binary_path, &args, config, on_output).await
    }
}
