use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Output captured from an agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    /// Combined stdout output
    pub stdout: String,
    /// Combined stderr output
    pub stderr: String,
    /// Exit code from the process
    pub exit_code: i32,
    /// Duration of execution
    #[serde(with = "humantime_serde_compat")]
    pub duration: Duration,
}

impl AgentOutput {
    pub fn new(stdout: String, stderr: String, exit_code: i32, duration: Duration) -> Self {
        Self {
            stdout,
            stderr,
            exit_code,
            duration,
        }
    }

    /// Check if the agent exited successfully
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }

    /// Get combined output (stdout + stderr)
    pub fn combined_output(&self) -> String {
        if self.stderr.is_empty() {
            self.stdout.clone()
        } else if self.stdout.is_empty() {
            self.stderr.clone()
        } else {
            format!("{}\n\n--- stderr ---\n{}", self.stdout, self.stderr)
        }
    }

    /// Count lines in stdout
    pub fn stdout_lines(&self) -> usize {
        self.stdout.lines().count()
    }

    /// Count lines in stderr
    pub fn stderr_lines(&self) -> usize {
        self.stderr.lines().count()
    }
}

mod humantime_serde_compat {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs_f64().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = f64::deserialize(deserializer)?;
        Ok(Duration::from_secs_f64(secs))
    }
}
