//! Project configuration file support for codeloops.
//!
//! Loads configuration from `codeloops.toml` in the working directory.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

/// Project-level configuration loaded from `codeloops.toml`
#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    /// Global default agent (applies to both actor and critic)
    pub agent: Option<String>,
    /// Global default model (applies to both actor and critic)
    pub model: Option<String>,
    /// Actor-specific configuration
    #[serde(default)]
    pub actor: RoleConfig,
    /// Critic-specific configuration
    #[serde(default)]
    pub critic: RoleConfig,
}

/// Configuration for a specific role (actor or critic)
#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct RoleConfig {
    /// Agent to use for this role
    pub agent: Option<String>,
    /// Model to use for this role
    pub model: Option<String>,
}

/// The config file name
pub const CONFIG_FILE_NAME: &str = "codeloops.toml";

impl ProjectConfig {
    /// Load configuration from the working directory.
    ///
    /// Returns:
    /// - `Ok(Some(config))` if file exists and parses successfully
    /// - `Ok(None)` if file does not exist
    /// - `Err(...)` if file exists but fails to parse (hard error)
    pub fn load(working_dir: &Path) -> Result<Option<Self>> {
        let config_path = working_dir.join(CONFIG_FILE_NAME);

        if !config_path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read {}", config_path.display()))?;

        let config: ProjectConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", config_path.display()))?;

        Ok(Some(config))
    }

    /// Get the effective agent for the actor role.
    /// Priority: [actor].agent > global agent > None
    pub fn actor_agent(&self) -> Option<&str> {
        self.actor.agent.as_deref().or(self.agent.as_deref())
    }

    /// Get the effective model for the actor role.
    /// Priority: [actor].model > global model > None
    pub fn actor_model(&self) -> Option<&str> {
        self.actor.model.as_deref().or(self.model.as_deref())
    }

    /// Get the effective agent for the critic role.
    /// Priority: [critic].agent > global agent > None
    pub fn critic_agent(&self) -> Option<&str> {
        self.critic.agent.as_deref().or(self.agent.as_deref())
    }

    /// Get the effective model for the critic role.
    /// Priority: [critic].model > global model > None
    pub fn critic_model(&self) -> Option<&str> {
        self.critic.model.as_deref().or(self.model.as_deref())
    }
}
