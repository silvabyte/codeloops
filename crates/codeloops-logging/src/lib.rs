//! # codeloops-logging
//!
//! Logging for the codeloops actor-critic system.
//!
//! This crate provides structured logging for execution events.
//!
//! ## Key Types
//!
//! - [`Logger`] - Structured event logging
//! - [`LogEvent`] - Log event types
//! - [`LogFormat`] - Output formats (Pretty, JSON, Compact)
//!
//! ## Log Formats
//!
//! - `Pretty` - Human-readable colored output
//! - `JSON` - Structured JSON lines
//! - `Compact` - Minimal text output

mod events;

pub use events::{AgentRole, LogEvent, LogFormat, Logger, StreamType};

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
