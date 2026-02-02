//! Database module for persistent storage.
//!
//! Uses SQLite to store prompt sessions and other persistent data.

mod prompt_store;

pub use prompt_store::{PromptFilter, PromptRecord, PromptStore};
