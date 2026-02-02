//! # codeloops-sessions
//!
//! Session reading, parsing, and querying for the codeloops system.
//!
//! This crate provides tools to read and analyze recorded sessions,
//! including filtering, statistics, and real-time watching.
//!
//! ## Overview
//!
//! Sessions are stored as JSONL files. This crate provides:
//! - Fast session listing (reads only first/last lines)
//! - Full session loading
//! - Filtering by outcome, date, project, search text
//! - Aggregate statistics
//! - File watching for live updates
//!
//! ## Key Types
//!
//! - [`SessionStore`] - Main interface for session access
//! - [`Session`] - Full session with all iterations
//! - [`SessionSummary`] - Lightweight summary for listings
//! - [`SessionFilter`] - Filter criteria
//! - [`SessionStats`] - Aggregate statistics
//! - [`SessionWatcher`] - File system watcher for live updates
//!
//! ## Usage
//!
//! ```rust,ignore
//! use codeloops_sessions::{SessionStore, SessionFilter, Session};
//!
//! // Create store
//! let store = SessionStore::new()?;
//!
//! // List sessions with filters
//! let filter = SessionFilter {
//!     outcome: Some("success".to_string()),
//!     project: Some("myapp".to_string()),
//!     ..Default::default()
//! };
//! let summaries = store.list_sessions(&filter)?;
//!
//! for summary in &summaries {
//!     println!("{}: {} ({} iterations)",
//!         summary.id,
//!         summary.prompt_preview,
//!         summary.iterations
//!     );
//! }
//!
//! // Load full session
//! if let Some(first) = summaries.first() {
//!     let session: Session = store.load_session(&first.id)?;
//!     println!("Prompt: {}", session.start.prompt);
//!     for iter in &session.iterations {
//!         println!("  Iteration {}: {}", iter.iteration_number, iter.critic_decision);
//!     }
//! }
//!
//! // Get statistics
//! let stats = store.get_stats()?;
//! println!("Total: {}, Success rate: {:.1}%",
//!     stats.total_sessions,
//!     stats.success_rate * 100.0
//! );
//! ```
//!
//! ## Filtering
//!
//! ```rust,ignore
//! let filter = SessionFilter {
//!     outcome: Some("success".to_string()),
//!     after: Some(parse_date("2025-01-01")),
//!     before: Some(parse_date("2025-01-31")),
//!     search: Some("authentication".to_string()),
//!     project: Some("myapp".to_string()),
//! };
//! ```
//!
//! ## Session Format
//!
//! Sessions are JSONL files with three line types:
//! - `session_start` - Initial metadata
//! - `iteration` - One per actor-critic cycle
//! - `session_end` - Final outcome

pub mod parser;
pub mod store;
pub mod types;
pub mod watcher;

pub use parser::{parse_session, parse_session_summary};
pub use store::SessionStore;
pub use types::{
    AgenticMetrics, DayCount, Iteration, ProjectStats, Session, SessionEnd, SessionFilter,
    SessionLine, SessionStart, SessionStats, SessionSummary,
};
pub use watcher::{SessionEvent, SessionWatcher};
