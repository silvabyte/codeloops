mod claude;
mod opencode;
mod output;
mod spawner;
mod traits;

pub use claude::ClaudeCodeAgent;
pub use opencode::OpenCodeAgent;
pub use output::AgentOutput;
pub use spawner::ProcessSpawner;
pub use traits::{Agent, AgentConfig, AgentError, AgentType};

/// Create an agent by type
pub fn create_agent(agent_type: AgentType) -> Box<dyn Agent> {
    match agent_type {
        AgentType::ClaudeCode => Box::new(ClaudeCodeAgent::new()),
        AgentType::OpenCode => Box::new(OpenCodeAgent::new()),
    }
}
