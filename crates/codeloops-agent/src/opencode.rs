use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::debug;

use crate::{
    Agent, AgentConfig, AgentError, AgentOutput, AgentType, OutputCallback, ProcessSpawner,
};

/// OpenCode agent implementation
pub struct OpenCodeAgent {
    binary_path: PathBuf,
}

impl OpenCodeAgent {
    pub fn new() -> Self {
        Self {
            binary_path: PathBuf::from("opencode"),
        }
    }

    pub fn with_binary_path(path: PathBuf) -> Self {
        Self { binary_path: path }
    }
}

impl Default for OpenCodeAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Agent for OpenCodeAgent {
    fn name(&self) -> &str {
        "OpenCode"
    }

    fn agent_type(&self) -> AgentType {
        AgentType::OpenCode
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

        // OpenCode uses the "run" subcommand for non-interactive execution
        let mut args = vec!["run"];

        // Add model if specified
        let model_arg;
        if let Some(ref model) = config.model {
            args.push("--model");
            model_arg = model.clone();
            args.push(&model_arg);
        }

        // Add the prompt
        args.push("--prompt");
        args.push(prompt);

        ProcessSpawner::spawn_with_callback(&self.binary_path, &args, config, on_output).await
    }
}
