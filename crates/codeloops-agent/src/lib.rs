//! # codeloops-agent
//!
//! Agent abstraction layer for the codeloops actor-critic system.
//!
//! This crate defines the [`Agent`] trait and provides implementations for
//! various coding agents (Claude Code, OpenCode, Cursor).
//!
//! ## Overview
//!
//! The agent abstraction allows codeloops to work with different coding agents
//! through a unified interface. Each agent can execute prompts and return
//! structured output including stdout, stderr, and exit code.
//!
//! ## Supported Agents
//!
//! | Agent | Type | Binary |
//! |-------|------|--------|
//! | Claude Code | [`AgentType::ClaudeCode`] | `claude` |
//! | OpenCode | [`AgentType::OpenCode`] | `opencode` |
//! | Cursor | [`AgentType::Cursor`] | `cursor` |
//!
//! ## Usage
//!
//! ```rust,ignore
//! use codeloops_agent::{create_agent, AgentType, AgentConfig};
//! use std::path::PathBuf;
//!
//! // Create an agent
//! let agent = create_agent(AgentType::ClaudeCode);
//!
//! // Configure the execution context
//! let config = AgentConfig::new(PathBuf::from("."));
//!
//! // Execute a prompt
//! let output = agent.execute("Fix the bug in main.rs", &config).await?;
//!
//! println!("Exit code: {}", output.exit_code);
//! println!("Output: {}", output.stdout);
//! ```
//!
//! ## Adding New Agents
//!
//! To add a new agent, implement the [`Agent`] trait. See the contributing
//! guide for detailed instructions.

mod claude;
mod cursor;
mod opencode;
mod output;
mod spawner;
mod traits;

pub use claude::ClaudeCodeAgent;
pub use cursor::CursorAgent;
pub use opencode::OpenCodeAgent;
pub use output::AgentOutput;
pub use spawner::{OutputCallback, OutputType, ProcessSpawner};
pub use traits::{Agent, AgentConfig, AgentError, AgentType};

/// Create an agent by type
pub fn create_agent(agent_type: AgentType) -> Box<dyn Agent> {
    match agent_type {
        AgentType::ClaudeCode => Box::new(ClaudeCodeAgent::new()),
        AgentType::OpenCode => Box::new(OpenCodeAgent::new()),
        AgentType::Cursor => Box::new(CursorAgent::new()),
    }
}
