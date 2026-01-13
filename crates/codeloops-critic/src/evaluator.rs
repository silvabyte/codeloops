use codeloops_agent::{Agent, AgentConfig, OutputCallback};
use tracing::{debug, info};

use crate::{CriticDecision, CriticPrompts, DecisionParseError};

/// Inputs required to evaluate the critic decision.
#[derive(Clone, Copy)]
pub struct CriticEvaluationInput<'a> {
    pub original_task: &'a str,
    pub actor_stdout: &'a str,
    pub actor_stderr: &'a str,
    pub git_diff: &'a str,
    pub iteration: usize,
}

/// Evaluator that runs the critic agent
pub struct CriticEvaluator<'a> {
    agent: &'a dyn Agent,
}

impl<'a> CriticEvaluator<'a> {
    pub fn new(agent: &'a dyn Agent) -> Self {
        Self { agent }
    }

    /// Evaluate the actor's work with optional streaming output
    pub async fn evaluate(
        &self,
        input: CriticEvaluationInput<'_>,
        config: &AgentConfig,
    ) -> Result<CriticDecision, EvaluationError> {
        self.evaluate_with_callback(input, config, None).await
    }

    /// Evaluate the actor's work with optional streaming output callback
    pub async fn evaluate_with_callback(
        &self,
        input: CriticEvaluationInput<'_>,
        config: &AgentConfig,
        on_output: Option<OutputCallback>,
    ) -> Result<CriticDecision, EvaluationError> {
        let prompt = CriticPrompts::build_evaluation_prompt(
            input.original_task,
            input.actor_stdout,
            input.actor_stderr,
            input.git_diff,
            input.iteration,
        );

        debug!(
            prompt_len = prompt.len(),
            iteration = input.iteration,
            "Running critic evaluation"
        );

        let output = self
            .agent
            .execute_with_callback(&prompt, config, on_output)
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
