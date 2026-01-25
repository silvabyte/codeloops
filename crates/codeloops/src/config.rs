//! Configuration file support for codeloops.
//!
//! Supports two levels of configuration:
//! - Global: `~/.config/codeloops/config.toml`
//! - Project: `codeloops.toml` in the working directory
//!
//! Precedence: CLI flags > project config > global config > defaults

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

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
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct RoleConfig {
    /// Agent to use for this role
    pub agent: Option<String>,
    /// Model to use for this role
    pub model: Option<String>,
}

/// The project config file name
pub const CONFIG_FILE_NAME: &str = "codeloops.toml";

/// The global config directory name
pub const GLOBAL_CONFIG_DIR: &str = "codeloops";

/// The global config file name
pub const GLOBAL_CONFIG_FILE: &str = "config.toml";

/// Global-level configuration loaded from ~/.config/codeloops/config.toml
#[derive(Debug, Deserialize, Default)]
pub struct GlobalConfig {
    #[serde(default)]
    pub defaults: GlobalDefaults,
}

/// Default settings within the global config
#[derive(Debug, Deserialize, Default)]
pub struct GlobalDefaults {
    /// Default agent (applies to both actor and critic)
    pub agent: Option<String>,
    /// Default model (applies to both actor and critic)
    pub model: Option<String>,
    /// Actor-specific defaults
    pub actor: Option<RoleConfig>,
    /// Critic-specific defaults
    pub critic: Option<RoleConfig>,
}

impl GlobalConfig {
    /// Load global config from ~/.config/codeloops/config.toml.
    ///
    /// Returns:
    /// - `Ok(Some(config))` if file exists and parses successfully
    /// - `Ok(None)` if file does not exist
    /// - `Err(...)` if file exists but fails to parse (hard error)
    pub fn load() -> Result<Option<Self>> {
        let Some(path) = Self::config_path() else {
            return Ok(None);
        };

        if !path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;

        let config: GlobalConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))?;

        Ok(Some(config))
    }

    /// Returns the path where global config would be stored
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join(GLOBAL_CONFIG_DIR).join(GLOBAL_CONFIG_FILE))
    }

    /// Check if global config exists
    pub fn exists() -> bool {
        Self::config_path().map(|p| p.exists()).unwrap_or(false)
    }

    /// Get the effective agent for the actor role.
    /// Priority: [defaults.actor].agent > [defaults].agent > None
    pub fn actor_agent(&self) -> Option<&str> {
        self.defaults
            .actor
            .as_ref()
            .and_then(|a| a.agent.as_deref())
            .or(self.defaults.agent.as_deref())
    }

    /// Get the effective model for the actor role.
    /// Priority: [defaults.actor].model > [defaults].model > None
    pub fn actor_model(&self) -> Option<&str> {
        self.defaults
            .actor
            .as_ref()
            .and_then(|a| a.model.as_deref())
            .or(self.defaults.model.as_deref())
    }

    /// Get the effective agent for the critic role.
    /// Priority: [defaults.critic].agent > [defaults].agent > None
    pub fn critic_agent(&self) -> Option<&str> {
        self.defaults
            .critic
            .as_ref()
            .and_then(|c| c.agent.as_deref())
            .or(self.defaults.agent.as_deref())
    }

    /// Get the effective model for the critic role.
    /// Priority: [defaults.critic].model > [defaults].model > None
    pub fn critic_model(&self) -> Option<&str> {
        self.defaults
            .critic
            .as_ref()
            .and_then(|c| c.model.as_deref())
            .or(self.defaults.model.as_deref())
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_config_parse() {
        let toml = r#"
[defaults]
agent = "claude"
model = "sonnet"

[defaults.actor]
agent = "opencode"

[defaults.critic]
model = "opus"
"#;
        let config: GlobalConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.defaults.agent.as_deref(), Some("claude"));
        assert_eq!(config.defaults.model.as_deref(), Some("sonnet"));
        assert_eq!(config.actor_agent(), Some("opencode"));
        assert_eq!(config.actor_model(), Some("sonnet")); // falls back to defaults.model
        assert_eq!(config.critic_agent(), Some("claude")); // falls back to defaults.agent
        assert_eq!(config.critic_model(), Some("opus"));
    }

    #[test]
    fn test_global_config_minimal() {
        let toml = r#"
[defaults]
agent = "cursor"
"#;
        let config: GlobalConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.defaults.agent.as_deref(), Some("cursor"));
        assert_eq!(config.actor_agent(), Some("cursor"));
        assert_eq!(config.critic_agent(), Some("cursor"));
        assert_eq!(config.actor_model(), None);
        assert_eq!(config.critic_model(), None);
    }

    #[test]
    fn test_global_config_empty() {
        let toml = "";
        let config: GlobalConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.actor_agent(), None);
        assert_eq!(config.critic_agent(), None);
    }
}
