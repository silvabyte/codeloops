use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::{AgentOutput, OutputCallback};

/// Errors that can occur during agent execution
#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Failed to spawn agent process: {0}")]
    SpawnFailed(#[from] std::io::Error),

    #[error("Agent execution timed out after {0:?}")]
    Timeout(std::time::Duration),

    #[error("Agent not found at path: {0}")]
    NotFound(String),

    #[error("Agent configuration error: {0}")]
    ConfigError(String),

    #[error("Agent execution failed: {0}")]
    ExecutionFailed(String),
}

/// Configuration for agent execution
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Working directory for the agent
    pub working_dir: PathBuf,
    /// Optional timeout (None = no limit)
    pub timeout: Option<std::time::Duration>,
    /// Additional environment variables
    pub env_vars: HashMap<String, String>,
    /// Model to use (if agent supports it)
    pub model: Option<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            timeout: None,
            env_vars: HashMap::new(),
            model: None,
        }
    }
}

impl AgentConfig {
    pub fn new(working_dir: PathBuf) -> Self {
        Self {
            working_dir,
            ..Default::default()
        }
    }

    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = Some(model);
        self
    }

    pub fn with_env(mut self, key: String, value: String) -> Self {
        self.env_vars.insert(key, value);
        self
    }
}

/// Supported agent types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AgentType {
    ClaudeCode,
    OpenCode,
    Cursor,
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentType::ClaudeCode => write!(f, "claude-code"),
            AgentType::OpenCode => write!(f, "opencode"),
            AgentType::Cursor => write!(f, "cursor"),
        }
    }
}

impl std::str::FromStr for AgentType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude" | "claude-code" | "claudecode" => Ok(AgentType::ClaudeCode),
            "opencode" | "open-code" => Ok(AgentType::OpenCode),
            "cursor" => Ok(AgentType::Cursor),
            _ => Err(format!("Unknown agent type: {}", s)),
        }
    }
}

/// The core abstraction for coding agents
#[async_trait]
pub trait Agent: Send + Sync {
    /// Human-readable name of the agent (e.g., "Claude Code", "OpenCode")
    fn name(&self) -> &str;

    /// The agent type
    fn agent_type(&self) -> AgentType;

    /// Execute a task with the given prompt
    async fn execute(&self, prompt: &str, config: &AgentConfig) -> Result<AgentOutput, AgentError> {
        self.execute_with_callback(prompt, config, None).await
    }

    /// Execute a task with the given prompt and optional output callback for streaming
    async fn execute_with_callback(
        &self,
        prompt: &str,
        config: &AgentConfig,
        on_output: Option<OutputCallback>,
    ) -> Result<AgentOutput, AgentError>;

    /// Check if the agent CLI is available on the system
    async fn is_available(&self) -> bool;

    /// Get the path to the agent binary
    fn binary_path(&self) -> &Path;
}
