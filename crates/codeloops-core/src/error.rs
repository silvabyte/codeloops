use thiserror::Error;

#[derive(Error, Debug)]
pub enum LoopError {
    #[error("Agent error: {0}")]
    AgentError(#[from] codeloops_agent::AgentError),

    #[error("Git error: {0}")]
    GitError(#[from] codeloops_git::GitError),

    #[error("Critic evaluation error: {0}")]
    CriticError(#[from] codeloops_critic::evaluator::EvaluationError),

    #[error("Loop was interrupted")]
    Interrupted,

    #[error("Configuration error: {0}")]
    ConfigError(String),
}
