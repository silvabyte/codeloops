use std::path::Path;
use std::process::Stdio;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{debug, trace};

use crate::{AgentConfig, AgentError, AgentOutput};

/// Utility for spawning agent processes
pub struct ProcessSpawner;

impl ProcessSpawner {
    /// Spawn a process and capture its output
    pub async fn spawn(
        binary: &Path,
        args: &[&str],
        config: &AgentConfig,
    ) -> Result<AgentOutput, AgentError> {
        let start = Instant::now();

        debug!(
            binary = %binary.display(),
            args = ?args,
            working_dir = %config.working_dir.display(),
            "Spawning agent process"
        );

        let mut cmd = Command::new(binary);
        cmd.args(args)
            .current_dir(&config.working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null()); // Non-interactive

        // Add environment variables
        for (key, value) in &config.env_vars {
            cmd.env(key, value);
        }

        let mut child = cmd.spawn()?;

        // Capture stdout and stderr
        let stdout_handle = child.stdout.take().expect("stdout not captured");
        let stderr_handle = child.stderr.take().expect("stderr not captured");

        let mut stdout_reader = BufReader::new(stdout_handle).lines();
        let mut stderr_reader = BufReader::new(stderr_handle).lines();

        let mut stdout = String::new();
        let mut stderr = String::new();

        // Read both streams concurrently
        loop {
            tokio::select! {
                biased;

                result = stdout_reader.next_line() => {
                    match result {
                        Ok(Some(line)) => {
                            trace!(line = %line, "stdout");
                            if !stdout.is_empty() {
                                stdout.push('\n');
                            }
                            stdout.push_str(&line);
                        }
                        Ok(None) => {
                            // stdout closed, wait for stderr to close too
                            while let Ok(Some(line)) = stderr_reader.next_line().await {
                                trace!(line = %line, "stderr");
                                if !stderr.is_empty() {
                                    stderr.push('\n');
                                }
                                stderr.push_str(&line);
                            }
                            break;
                        }
                        Err(e) => {
                            return Err(AgentError::ExecutionFailed(format!(
                                "Failed to read stdout: {}",
                                e
                            )));
                        }
                    }
                }
                result = stderr_reader.next_line() => {
                    match result {
                        Ok(Some(line)) => {
                            trace!(line = %line, "stderr");
                            if !stderr.is_empty() {
                                stderr.push('\n');
                            }
                            stderr.push_str(&line);
                        }
                        Ok(None) => {
                            // stderr closed, continue reading stdout
                        }
                        Err(e) => {
                            return Err(AgentError::ExecutionFailed(format!(
                                "Failed to read stderr: {}",
                                e
                            )));
                        }
                    }
                }
            }
        }

        let status = child.wait().await?;
        let duration = start.elapsed();

        debug!(
            exit_code = status.code().unwrap_or(-1),
            duration_ms = duration.as_millis(),
            "Agent process completed"
        );

        Ok(AgentOutput::new(
            stdout,
            stderr,
            status.code().unwrap_or(-1),
            duration,
        ))
    }
}
