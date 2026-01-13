use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::IterationRecord;

/// The final outcome of an actor-critic loop
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum LoopOutcome {
    /// Task completed successfully
    Success {
        iterations: usize,
        summary: String,
        confidence: f64,
        #[serde(skip)]
        history: Vec<IterationRecord>,
        total_duration_secs: f64,
    },
    /// Hit maximum iteration limit
    MaxIterationsReached {
        iterations: usize,
        #[serde(skip)]
        history: Vec<IterationRecord>,
        total_duration_secs: f64,
    },
    /// User requested stop (e.g., Ctrl+C)
    UserInterrupted {
        iterations: usize,
        #[serde(skip)]
        history: Vec<IterationRecord>,
        total_duration_secs: f64,
    },
    /// Unrecoverable error
    Failed {
        iterations: usize,
        error: String,
        #[serde(skip)]
        history: Vec<IterationRecord>,
        total_duration_secs: f64,
    },
}

impl LoopOutcome {
    pub fn success(
        iterations: usize,
        summary: String,
        confidence: f64,
        history: Vec<IterationRecord>,
        duration: Duration,
    ) -> Self {
        Self::Success {
            iterations,
            summary,
            confidence,
            history,
            total_duration_secs: duration.as_secs_f64(),
        }
    }

    pub fn max_iterations_reached(
        iterations: usize,
        history: Vec<IterationRecord>,
        duration: Duration,
    ) -> Self {
        Self::MaxIterationsReached {
            iterations,
            history,
            total_duration_secs: duration.as_secs_f64(),
        }
    }

    pub fn interrupted(
        iterations: usize,
        history: Vec<IterationRecord>,
        duration: Duration,
    ) -> Self {
        Self::UserInterrupted {
            iterations,
            history,
            total_duration_secs: duration.as_secs_f64(),
        }
    }

    pub fn failed(
        iterations: usize,
        error: String,
        history: Vec<IterationRecord>,
        duration: Duration,
    ) -> Self {
        Self::Failed {
            iterations,
            error,
            history,
            total_duration_secs: duration.as_secs_f64(),
        }
    }

    pub fn iterations(&self) -> usize {
        match self {
            Self::Success { iterations, .. } => *iterations,
            Self::MaxIterationsReached { iterations, .. } => *iterations,
            Self::UserInterrupted { iterations, .. } => *iterations,
            Self::Failed { iterations, .. } => *iterations,
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Success { .. } => 0,
            Self::MaxIterationsReached { .. } => 1,
            Self::UserInterrupted { .. } => 130,
            Self::Failed { .. } => 2,
        }
    }
}
