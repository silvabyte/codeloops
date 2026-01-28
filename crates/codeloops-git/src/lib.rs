//! # codeloops-git
//!
//! Git operations for the codeloops actor-critic system.
//!
//! This crate handles git diff capture between actor iterations,
//! allowing the critic to evaluate what changes were made.
//!
//! ## Overview
//!
//! After each actor execution, a git diff is captured to show:
//! - What files were modified
//! - What lines were added/removed
//! - How many changes occurred
//!
//! ## Key Types
//!
//! - [`DiffCapture`] - Utility for capturing git diffs
//! - [`DiffSummary`] - Summary of captured diff
//! - [`GitStatus`] - Current git repository status
//!
//! ## Usage
//!
//! ```rust,ignore
//! use codeloops_git::{DiffCapture, DiffSummary};
//! use std::path::PathBuf;
//!
//! // Create diff capture for a working directory
//! let mut capture = DiffCapture::new(PathBuf::from("."));
//!
//! // Set baseline before actor runs
//! capture.set_baseline()?;
//!
//! // ... actor makes changes ...
//!
//! // Capture the diff
//! let summary: DiffSummary = capture.capture()?;
//!
//! println!("Files changed: {}", summary.files_changed);
//! println!("Diff:\n{}", summary.diff);
//! ```
//!
//! ## Diff Format
//!
//! Diffs are captured in unified diff format, compatible with
//! standard diff tools and easy for the critic agent to parse.

mod diff;
mod status;

pub use diff::{DiffCapture, DiffSummary, GitError};
pub use status::GitStatus;
