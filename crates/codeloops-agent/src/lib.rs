mod traits;
mod output;
mod claude;
mod opencode;
mod spawner;

pub use traits::{Agent, AgentConfig, AgentError, AgentType};
pub use output::AgentOutput;
pub use claude::ClaudeCodeAgent;
pub use opencode::OpenCodeAgent;
pub use spawner::ProcessSpawner;

/// Create an agent by type
pub fn create_agent(agent_type: AgentType) -> Box<dyn Agent> {
    match agent_type {
        AgentType::ClaudeCode => Box::new(ClaudeCodeAgent::new()),
        AgentType::OpenCode => Box::new(OpenCodeAgent::new()),
    }
}
