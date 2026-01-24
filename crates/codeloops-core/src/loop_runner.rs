use chrono::Utc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{debug, info, warn};

use codeloops_agent::{Agent, AgentConfig, OutputCallback, OutputType};
use codeloops_critic::{CriticDecision, CriticEvaluationInput, CriticEvaluator};
use codeloops_git::DiffCapture;
use codeloops_logging::{AgentRole, LogEvent, Logger, SessionWriter, StreamType};

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
    session_writer: Option<Arc<SessionWriter>>,
    interrupted: Arc<AtomicBool>,
    actor_model: Option<String>,
    critic_model: Option<String>,
}

impl<'a> LoopRunner<'a> {
    pub fn new(
        actor: &'a dyn Agent,
        critic: &'a dyn Agent,
        diff_capture: DiffCapture,
        logger: Arc<Logger>,
        session_writer: Option<Arc<SessionWriter>>,
        actor_model: Option<String>,
        critic_model: Option<String>,
    ) -> Self {
        Self {
            actor,
            critic,
            diff_capture,
            logger,
            session_writer,
            interrupted: Arc::new(AtomicBool::new(false)),
            actor_model,
            critic_model,
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

        if let Some(ref sw) = self.session_writer {
            sw.write_start(
                &context.prompt,
                &context.working_dir,
                self.actor.name(),
                self.critic.name(),
                self.actor_model.as_deref(),
                self.critic_model.as_deref(),
                context.max_iterations,
            );
        }

        // Create separate configs for actor and critic
        let mut actor_config = AgentConfig::new(context.working_dir.clone());
        if let Some(ref model) = self.actor_model {
            actor_config = actor_config.with_model(model.clone());
        }

        let mut critic_config = AgentConfig::new(context.working_dir.clone());
        if let Some(ref model) = self.critic_model {
            critic_config = critic_config.with_model(model.clone());
        }

        loop {
            // Check for interruption
            if self.interrupted.load(Ordering::SeqCst) {
                info!("Loop interrupted by user");
                let duration = context.total_duration();
                let outcome =
                    LoopOutcome::interrupted(context.iteration, context.history, duration);
                self.write_session_end(&outcome);
                return Ok(outcome);
            }

            // Check iteration limit
            if !context.should_continue() {
                self.logger.log(&LogEvent::MaxIterationsReached {
                    iterations: context.iteration,
                });
                let duration = context.total_duration();
                let outcome = LoopOutcome::max_iterations_reached(
                    context.iteration,
                    context.history,
                    duration,
                );
                self.write_session_end(&outcome);
                return Ok(outcome);
            }

            // Run one iteration
            match self
                .run_iteration(&mut context, &actor_config, &critic_config)
                .await
            {
                Ok(Some(outcome)) => {
                    self.write_session_end(&outcome);
                    return Ok(outcome);
                }
                Ok(None) => {
                    // Continue to next iteration
                    context.increment_iteration();
                }
                Err(e) => {
                    warn!(error = %e, "Error during iteration");
                    let duration = context.total_duration();
                    let outcome = LoopOutcome::failed(
                        context.iteration + 1,
                        e.to_string(),
                        context.history,
                        duration,
                    );
                    self.write_session_end(&outcome);
                    return Ok(outcome);
                }
            }
        }
    }

    /// Run a single iteration of the actor-critic loop
    /// Returns Some(outcome) if loop should terminate, None to continue
    async fn run_iteration(
        &self,
        context: &mut LoopContext,
        actor_config: &AgentConfig,
        critic_config: &AgentConfig,
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
            .execute_with_callback(&actor_prompt, actor_config, Some(actor_callback))
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
            .evaluate_with_callback(evaluation_input, critic_config, Some(critic_callback))
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
        context.push_record(record.clone());

        // Write to session log
        if let Some(ref sw) = self.session_writer {
            let fb = match &decision {
                CriticDecision::Continue { feedback, .. } => Some(feedback.clone()),
                CriticDecision::Error {
                    error_description,
                    recovery_suggestion,
                } => Some(format!(
                    "Error encountered: {}\n\nRecovery suggestion: {}",
                    error_description, recovery_suggestion
                )),
                CriticDecision::Done { .. } => None,
            };
            sw.write_iteration(
                record.iteration_number,
                &record.actor_output,
                &record.actor_stderr,
                record.actor_exit_code,
                record.actor_duration_secs,
                &record.git_diff,
                record.git_files_changed,
                &record.critic_decision,
                fb.as_deref(),
                record.timestamp,
            );
        }

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

    /// Write the session end line to the JSONL session log.
    fn write_session_end(&self, outcome: &LoopOutcome) {
        if let Some(ref sw) = self.session_writer {
            let (outcome_str, iterations, summary, confidence, duration_secs) = match outcome {
                LoopOutcome::Success {
                    iterations,
                    summary,
                    confidence,
                    total_duration_secs,
                    ..
                } => (
                    "success",
                    *iterations,
                    Some(summary.as_str()),
                    Some(*confidence),
                    *total_duration_secs,
                ),
                LoopOutcome::MaxIterationsReached {
                    iterations,
                    total_duration_secs,
                    ..
                } => (
                    "max_iterations_reached",
                    *iterations,
                    None,
                    None,
                    *total_duration_secs,
                ),
                LoopOutcome::UserInterrupted {
                    iterations,
                    total_duration_secs,
                    ..
                } => (
                    "user_interrupted",
                    *iterations,
                    None,
                    None,
                    *total_duration_secs,
                ),
                LoopOutcome::Failed {
                    iterations,
                    error,
                    total_duration_secs,
                    ..
                } => (
                    "failed",
                    *iterations,
                    Some(error.as_str()),
                    None,
                    *total_duration_secs,
                ),
            };
            sw.write_end(outcome_str, iterations, summary, confidence, duration_secs);
        }
    }
}
