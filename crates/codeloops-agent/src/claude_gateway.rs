use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::debug;

use crate::{
    Agent, AgentConfig, AgentError, AgentOutput, AgentType, OutputCallback, ProcessSpawner,
};

/// Claude Gateway agent implementation
///
/// Standalone agent for the `claude-gateway` binary used by enterprise
/// users who route through a custom gateway instead of Anthropic's API
/// directly. Mirrors the Claude Code CLI interface but uses a separate
/// binary, allowing independent evolution as enterprise needs arise.
pub struct ClaudeGatewayAgent {
    binary_path: PathBuf,
}

impl ClaudeGatewayAgent {
    pub fn new() -> Self {
        Self {
            binary_path: PathBuf::from("claude-gateway"),
        }
    }

    pub fn with_binary_path(path: PathBuf) -> Self {
        Self { binary_path: path }
    }
}

impl Default for ClaudeGatewayAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Agent for ClaudeGatewayAgent {
    fn name(&self) -> &str {
        "Claude Gateway"
    }

    fn agent_type(&self) -> AgentType {
        AgentType::ClaudeGateway
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn new_has_correct_binary() {
        let agent = ClaudeGatewayAgent::new();
        assert_eq!(agent.binary_path(), Path::new("claude-gateway"));
    }

    #[test]
    fn name_returns_claude_gateway() {
        let agent = ClaudeGatewayAgent::new();
        assert_eq!(agent.name(), "Claude Gateway");
    }

    #[test]
    fn agent_type_returns_claude_gateway() {
        let agent = ClaudeGatewayAgent::new();
        assert_eq!(agent.agent_type(), AgentType::ClaudeGateway);
    }

    #[test]
    fn with_binary_path_overrides_default() {
        let agent = ClaudeGatewayAgent::with_binary_path(PathBuf::from("/usr/local/bin/cg"));
        assert_eq!(agent.binary_path(), Path::new("/usr/local/bin/cg"));
    }

    #[tokio::test]
    async fn is_available_false_when_binary_missing() {
        let agent = ClaudeGatewayAgent::with_binary_path(PathBuf::from("nonexistent-binary-12345"));
        assert!(!agent.is_available().await);
    }
}
