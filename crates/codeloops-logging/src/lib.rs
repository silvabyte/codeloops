//! # codeloops-logging
//!
//! Logging and session recording for the codeloops actor-critic system.
//!
//! This crate provides structured logging and session file writing.
//! Sessions are recorded as JSONL files for later analysis.
//!
//! ## Overview
//!
//! Two main responsibilities:
//! 1. **Logging** - Structured console/file output during execution
//! 2. **Session Writing** - Persistent JSONL records of sessions
//!
//! ## Key Types
//!
//! - [`Logger`] - Structured event logging
//! - [`SessionWriter`] - JSONL session file writer
//! - [`LogEvent`] - Log event types
//! - [`LogFormat`] - Output formats (Pretty, JSON, Compact)
//!
//! ## Session Writing
//!
//! ```rust,ignore
//! use codeloops_logging::SessionWriter;
//! use std::path::PathBuf;
//!
//! // Create a session writer
//! let sessions_dir = PathBuf::from("~/.local/share/codeloops/sessions");
//! let mut writer = SessionWriter::new(&sessions_dir, "Fix the bug")?;
//!
//! // Write session start
//! writer.write_start(
//!     "Fix the bug",
//!     &working_dir,
//!     "Claude Code",
//!     "Claude Code",
//!     None, // actor_model
//!     None, // critic_model
//!     Some(10), // max_iterations
//! )?;
//!
//! // Write iteration
//! writer.write_iteration(
//!     1,
//!     "Actor output...",
//!     "",
//!     0,
//!     45.2,
//!     "diff --git...",
//!     3,
//!     "CONTINUE",
//!     Some("Please also fix..."),
//! )?;
//!
//! // Write session end
//! writer.write_end(
//!     "success",
//!     2,
//!     Some("Fixed the bug by..."),
//!     Some(0.95),
//!     89.4,
//! )?;
//! ```
//!
//! ## Log Formats
//!
//! - `Pretty` - Human-readable colored output
//! - `JSON` - Structured JSON lines
//! - `Compact` - Minimal text output

mod events;
pub mod session;

pub use events::{AgentRole, LogEvent, LogFormat, Logger, StreamType};
pub use session::SessionWriter;

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialize tracing for the application
pub fn init_tracing(level: &str, format: LogFormat) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));

    match format {
        LogFormat::Json => {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt::layer().json().with_target(false))
                .init();
        }
        LogFormat::Pretty | LogFormat::Compact => {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt::layer().with_target(false))
                .init();
        }
    }
}
