use codeloops_agent::{Agent, AgentConfig};
use tracing::{debug, info};

use crate::{CriticDecision, CriticPrompts, DecisionParseError};

/// Evaluator that runs the critic agent
pub struct CriticEvaluator<'a> {
    agent: &'a dyn Agent,
}

impl<'a> CriticEvaluator<'a> {
    pub fn new(agent: &'a dyn Agent) -> Self {
        Self { agent }
    }

    /// Evaluate the actor's work
    pub async fn evaluate(
        &self,
        original_task: &str,
        actor_stdout: &str,
        actor_stderr: &str,
        git_diff: &str,
        iteration: usize,
        config: &AgentConfig,
    ) -> Result<CriticDecision, EvaluationError> {
        let prompt = CriticPrompts::build_evaluation_prompt(
            original_task,
            actor_stdout,
            actor_stderr,
            git_diff,
            iteration,
        );

        debug!(
            prompt_len = prompt.len(),
            iteration = iteration,
            "Running critic evaluation"
        );

        let output = self
            .agent
            .execute(&prompt, config)
            .await
            .map_err(|e| EvaluationError::AgentError(e.to_string()))?;

        info!(
            exit_code = output.exit_code,
            duration_secs = output.duration.as_secs_f64(),
            "Critic completed"
        );

        if output.exit_code != 0 {
            return Err(EvaluationError::AgentError(format!(
                "Critic exited with code {}",
                output.exit_code
            )));
        }

        CriticDecision::parse(&output.stdout).map_err(EvaluationError::ParseError)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EvaluationError {
    #[error("Agent execution error: {0}")]
    AgentError(String),

    #[error("Failed to parse critic decision: {0}")]
    ParseError(#[from] DecisionParseError),
}
