use chrono::Utc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{debug, info, warn};

use codeloops_agent::{Agent, AgentConfig, OutputCallback, OutputType};
use codeloops_critic::{CriticDecision, CriticEvaluationInput, CriticEvaluator};
use codeloops_git::DiffCapture;
use codeloops_logging::{AgentRole, LogEvent, Logger, StreamType};

use crate::context::IterationRecord;
use crate::error::LoopError;
use crate::outcome::LoopOutcome;
use crate::LoopContext;

/// Orchestrates the actor-critic loop
pub struct LoopRunner<'a> {
    actor: &'a dyn Agent,
    critic: &'a dyn Agent,
    diff_capture: DiffCapture,
    logger: Arc<Logger>,
    interrupted: Arc<AtomicBool>,
}

impl<'a> LoopRunner<'a> {
    pub fn new(
        actor: &'a dyn Agent,
        critic: &'a dyn Agent,
        diff_capture: DiffCapture,
        logger: Arc<Logger>,
    ) -> Self {
        Self {
            actor,
            critic,
            diff_capture,
            logger,
            interrupted: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get a handle to signal interruption
    pub fn interrupt_handle(&self) -> Arc<AtomicBool> {
        self.interrupted.clone()
    }

    /// Create an output callback for streaming agent output
    fn create_output_callback(&self, iteration: usize, role: AgentRole) -> OutputCallback {
        let logger = self.logger.clone();
        Arc::new(move |line: &str, output_type: OutputType| {
            let stream = match output_type {
                OutputType::Stdout => StreamType::Stdout,
                OutputType::Stderr => StreamType::Stderr,
            };
            logger.log(&LogEvent::AgentStreamLine {
                iteration,
                role,
                stream,
                line: line.to_string(),
            });
        })
    }

    /// Run the actor-critic loop until completion
    pub async fn run(&self, mut context: LoopContext) -> Result<LoopOutcome, LoopError> {
        self.logger.log(&LogEvent::LoopStarted {
            prompt: context.prompt.clone(),
            working_dir: context.working_dir.clone(),
        });

        let config = AgentConfig::new(context.working_dir.clone());

        loop {
            // Check for interruption
            if self.interrupted.load(Ordering::SeqCst) {
                info!("Loop interrupted by user");
                let duration = context.total_duration();
                return Ok(LoopOutcome::interrupted(
                    context.iteration,
                    context.history,
                    duration,
                ));
            }

            // Check iteration limit
            if !context.should_continue() {
                self.logger.log(&LogEvent::MaxIterationsReached {
                    iterations: context.iteration,
                });
                let duration = context.total_duration();
                return Ok(LoopOutcome::max_iterations_reached(
                    context.iteration,
                    context.history,
                    duration,
                ));
            }

            // Run one iteration
            match self.run_iteration(&mut context, &config).await {
                Ok(Some(outcome)) => return Ok(outcome),
                Ok(None) => {
                    // Continue to next iteration
                    context.increment_iteration();
                }
                Err(e) => {
                    warn!(error = %e, "Error during iteration");
                    let duration = context.total_duration();
                    return Ok(LoopOutcome::failed(
                        context.iteration + 1,
                        e.to_string(),
                        context.history,
                        duration,
                    ));
                }
            }
        }
    }

    /// Run a single iteration of the actor-critic loop
    /// Returns Some(outcome) if loop should terminate, None to continue
    async fn run_iteration(
        &self,
        context: &mut LoopContext,
        config: &AgentConfig,
    ) -> Result<Option<LoopOutcome>, LoopError> {
        let iteration = context.iteration;

        // Get the prompt for this iteration
        let actor_prompt = context.current_prompt();

        self.logger.log(&LogEvent::ActorStarted {
            iteration,
            prompt_preview: actor_prompt.chars().take(100).collect(),
        });

        // Run actor with streaming output
        debug!(iteration, "Running actor");
        let actor_callback = self.create_output_callback(iteration, AgentRole::Actor);
        let actor_output = self
            .actor
            .execute_with_callback(&actor_prompt, config, Some(actor_callback))
            .await?;

        self.logger.log(&LogEvent::ActorCompleted {
            iteration,
            exit_code: actor_output.exit_code,
            duration_secs: actor_output.duration.as_secs_f64(),
        });

        self.logger.log(&LogEvent::ActorOutput {
            iteration,
            stdout_lines: actor_output.stdout_lines(),
            stderr_lines: actor_output.stderr_lines(),
        });

        // Capture git diff
        let git_diff = self
            .diff_capture
            .capture_diff(&context.working_dir)
            .unwrap_or_else(|e| {
                warn!(error = %e, "Failed to capture git diff");
                String::new()
            });

        let diff_summary = self
            .diff_capture
            .capture_summary(&context.working_dir)
            .unwrap_or_default();

        self.logger.log(&LogEvent::GitDiffCaptured {
            iteration,
            files_changed: diff_summary.files_changed,
            insertions: diff_summary.insertions,
            deletions: diff_summary.deletions,
        });

        // Run critic with streaming output
        self.logger.log(&LogEvent::CriticStarted { iteration });

        let critic_callback = self.create_output_callback(iteration, AgentRole::Critic);
        let evaluator = CriticEvaluator::new(self.critic);
        let evaluation_input = CriticEvaluationInput {
            original_task: &context.prompt,
            actor_stdout: &actor_output.stdout,
            actor_stderr: &actor_output.stderr,
            git_diff: &git_diff,
            iteration,
        };
        let decision = evaluator
            .evaluate_with_callback(evaluation_input, config, Some(critic_callback))
            .await?;

        self.logger.log(&LogEvent::CriticCompleted {
            iteration,
            decision: decision.short_description(),
        });

        // Record this iteration
        let record = IterationRecord {
            iteration_number: iteration,
            actor_output: actor_output.stdout.clone(),
            actor_stderr: actor_output.stderr.clone(),
            actor_exit_code: actor_output.exit_code,
            actor_duration_secs: actor_output.duration.as_secs_f64(),
            git_diff: git_diff.clone(),
            git_files_changed: diff_summary.files_changed,
            critic_output: String::new(), // We don't store full critic output
            critic_decision: decision.short_description(),
            timestamp: Utc::now(),
        };
        context.push_record(record);

        // Process decision
        match decision {
            CriticDecision::Done {
                summary,
                confidence,
            } => {
                self.logger.log(&LogEvent::LoopCompleted {
                    iterations: iteration + 1,
                    summary: summary.clone(),
                    duration_secs: context.total_duration().as_secs_f64(),
                });

                Ok(Some(LoopOutcome::success(
                    iteration + 1,
                    summary,
                    confidence,
                    context.history.clone(),
                    context.total_duration(),
                )))
            }
            CriticDecision::Continue {
                feedback,
                remaining_issues,
            } => {
                info!(
                    iteration = iteration + 1,
                    issues = remaining_issues.len(),
                    "Continuing to next iteration"
                );
                context.set_feedback(feedback);
                Ok(None)
            }
            CriticDecision::Error {
                error_description,
                recovery_suggestion,
            } => {
                self.logger.log(&LogEvent::ErrorEncountered {
                    iteration,
                    error: error_description.clone(),
                });

                // Treat errors as feedback for next iteration
                let feedback = format!(
                    "Error encountered: {}\n\nRecovery suggestion: {}",
                    error_description, recovery_suggestion
                );
                context.set_feedback(feedback);
                Ok(None)
            }
        }
    }
}
