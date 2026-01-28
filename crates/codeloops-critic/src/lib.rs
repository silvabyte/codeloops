//! # codeloops-critic
//!
//! Critic evaluation logic for the codeloops actor-critic system.
//!
//! This crate handles the critic phase of the loop, where actor output
//! is evaluated against the original task prompt.
//!
//! ## Overview
//!
//! The critic receives:
//! - Original task prompt
//! - Actor's output (stdout)
//! - Git diff of changes
//! - Iteration history
//!
//! And returns a decision:
//! - [`CriticDecision::Done`] - Task complete
//! - [`CriticDecision::Continue`] - More work needed, with feedback
//! - [`CriticDecision::Error`] - Error occurred, with recovery suggestion
//!
//! ## Key Types
//!
//! - [`CriticEvaluator`] - Runs evaluation using an agent
//! - [`CriticDecision`] - Parsed decision from critic response
//! - [`CriticEvaluationInput`] - Input data for evaluation
//!
//! ## Usage
//!
//! ```rust,ignore
//! use codeloops_critic::{CriticEvaluator, CriticEvaluationInput, CriticDecision};
//! use codeloops_agent::{create_agent, AgentType};
//!
//! // Create evaluator with a critic agent
//! let agent = create_agent(AgentType::ClaudeCode);
//! let evaluator = CriticEvaluator::new(agent);
//!
//! // Prepare evaluation input
//! let input = CriticEvaluationInput {
//!     prompt: "Fix the bug".to_string(),
//!     actor_output: "I fixed the null pointer issue...".to_string(),
//!     git_diff: "diff --git a/...".to_string(),
//!     iteration: 1,
//! };
//!
//! // Evaluate
//! let decision = evaluator.evaluate(&input, &config).await?;
//!
//! match decision {
//!     CriticDecision::Done { summary, confidence } => {
//!         println!("Task complete: {} (confidence: {})", summary, confidence);
//!     }
//!     CriticDecision::Continue { feedback } => {
//!         println!("Continue with feedback: {}", feedback);
//!     }
//!     CriticDecision::Error { recovery } => {
//!         println!("Error, try: {}", recovery);
//!     }
//! }
//! ```
//!
//! ## Decision Parsing
//!
//! The critic agent returns a structured response that is parsed into
//! a [`CriticDecision`]. The expected format includes:
//!
//! ```text
//! DECISION: DONE|CONTINUE|ERROR
//! SUMMARY: ... (for DONE)
//! FEEDBACK: ... (for CONTINUE)
//! RECOVERY: ... (for ERROR)
//! CONFIDENCE: 0.0-1.0 (for DONE)
//! ```

mod decision;
pub mod evaluator;
mod prompts;

pub use decision::{CriticDecision, DecisionParseError};
pub use evaluator::{CriticEvaluationInput, CriticEvaluator, EvaluationError};
pub use prompts::CriticPrompts;
