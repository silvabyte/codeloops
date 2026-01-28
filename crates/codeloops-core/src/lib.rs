//! # codeloops-core
//!
//! Core loop orchestration for the codeloops actor-critic system.
//!
//! This crate provides the main execution loop that coordinates actor execution,
//! git diff capture, and critic evaluation.
//!
//! ## Overview
//!
//! The core crate orchestrates the actor-critic feedback loop:
//!
//! 1. Actor executes the task prompt
//! 2. Git diff is captured
//! 3. Critic evaluates the changes
//! 4. Loop continues or terminates based on critic decision
//!
//! ## Key Types
//!
//! - [`LoopRunner`] - Main orchestrator that runs the loop
//! - [`LoopContext`] - Shared state across iterations
//! - [`IterationRecord`] - Record of a single iteration
//! - [`LoopOutcome`] - Terminal states (Success, Failed, etc.)
//!
//! ## Usage
//!
//! ```rust,ignore
//! use codeloops_core::{LoopRunner, LoopContext, LoopOutcome};
//! use codeloops_agent::{create_agent, AgentType};
//! use std::path::PathBuf;
//!
//! // Create agents
//! let actor = create_agent(AgentType::ClaudeCode);
//! let critic = create_agent(AgentType::ClaudeCode);
//!
//! // Create the loop runner
//! let runner = LoopRunner::new(actor, critic, working_dir, logger, writer);
//!
//! // Create context
//! let context = LoopContext::new(
//!     "Fix the bug in main.rs".to_string(),
//!     PathBuf::from("."),
//! );
//!
//! // Run the loop
//! let outcome = runner.run(context).await?;
//!
//! match outcome {
//!     LoopOutcome::Success { iterations, summary, .. } => {
//!         println!("Completed in {} iterations: {}", iterations, summary);
//!     }
//!     LoopOutcome::MaxIterationsReached { iterations, .. } => {
//!         println!("Reached max iterations: {}", iterations);
//!     }
//!     _ => {}
//! }
//! ```
//!
//! ## Loop Termination
//!
//! The loop terminates when:
//! - Critic returns DONE (success)
//! - Max iterations reached
//! - User interrupts (Ctrl+C)
//! - Unrecoverable error occurs

mod context;
mod error;
mod loop_runner;
mod outcome;

pub use context::{IterationRecord, LoopContext};
pub use error::LoopError;
pub use loop_runner::LoopRunner;
pub use outcome::LoopOutcome;
