# Adding New Agents

This guide walks through adding support for a new coding agent to codeloops.

## Overview

To add a new agent, you need to:

1. Implement the `Agent` trait
2. Add the agent type to the `AgentType` enum
3. Update the agent factory function
4. Add CLI support
5. Update documentation and tests

## Step 1: Implement the Agent Trait

Create a new file in `crates/codeloops-agent/src/agents/`:

```rust
// crates/codeloops-agent/src/agents/aider.rs

use crate::{Agent, AgentConfig, AgentError, AgentOutput, AgentType, OutputCallback};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};

/// Aider coding agent implementation.
pub struct AiderAgent {
    binary_path: PathBuf,
}

impl AiderAgent {
    pub fn new() -> Self {
        Self {
            binary_path: PathBuf::from("aider"),
        }
    }
}

#[async_trait]
impl Agent for AiderAgent {
    fn name(&self) -> &str {
        "Aider"
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Aider
    }

    // Note: execute() has a default implementation that calls execute_with_callback(),
    // so you only need to implement execute_with_callback().

    async fn execute_with_callback(
        &self,
        prompt: &str,
        config: &AgentConfig,
        on_output: Option<OutputCallback>,
    ) -> Result<AgentOutput, AgentError> {
        let start = std::time::Instant::now();

        // Build command
        let mut cmd = Command::new(&self.binary_path);
        cmd.current_dir(&config.working_dir);

        // Add agent-specific arguments
        cmd.arg("--message").arg(prompt);
        cmd.arg("--yes");  // Auto-confirm changes
        cmd.arg("--no-git");  // Let codeloops handle git

        // Add model if specified
        if let Some(model) = &config.model {
            cmd.arg("--model").arg(model);
        }

        // Set up I/O
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Spawn process
        let mut child = cmd.spawn().map_err(|e| {
            AgentError::SpawnError(format!("Failed to spawn aider: {}", e))
        })?;

        // Capture output
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        let mut stdout_output = String::new();
        let mut stderr_output = String::new();

        // Read output streams
        loop {
            tokio::select! {
                line = stdout_reader.next_line() => {
                    match line {
                        Ok(Some(line)) => {
                            if let Some(ref callback) = on_output {
                                callback(&line);
                            }
                            stdout_output.push_str(&line);
                            stdout_output.push('\n');
                        }
                        Ok(None) => break,
                        Err(e) => {
                            stderr_output.push_str(&format!("Read error: {}\n", e));
                            break;
                        }
                    }
                }
                line = stderr_reader.next_line() => {
                    if let Ok(Some(line)) = line {
                        stderr_output.push_str(&line);
                        stderr_output.push('\n');
                    }
                }
            }
        }

        // Wait for process
        let status = child.wait().await.map_err(|e| {
            AgentError::ExecutionError(format!("Failed to wait for aider: {}", e))
        })?;

        let duration = start.elapsed();

        Ok(AgentOutput {
            stdout: stdout_output,
            stderr: stderr_output,
            exit_code: status.code().unwrap_or(-1),
            duration,
        })
    }

    async fn is_available(&self) -> bool {
        Command::new(&self.binary_path)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn binary_path(&self) -> &Path {
        &self.binary_path
    }
}

impl Default for AiderAgent {
    fn default() -> Self {
        Self::new()
    }
}
```

## Step 2: Add the Agent Type

Edit `crates/codeloops-agent/src/lib.rs`:

```rust
/// Supported agent types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentType {
    ClaudeCode,
    OpenCode,
    Cursor,
    Aider,  // Add new variant
}

impl AgentType {
    pub fn display_name(&self) -> &'static str {
        match self {
            AgentType::ClaudeCode => "Claude Code",
            AgentType::OpenCode => "OpenCode",
            AgentType::Cursor => "Cursor",
            AgentType::Aider => "Aider",  // Add display name
        }
    }
}
```

## Step 3: Update the Factory Function

Edit `crates/codeloops-agent/src/lib.rs`:

```rust
mod agents;

pub use agents::aider::AiderAgent;  // Export new agent
pub use agents::claude::ClaudeCodeAgent;
pub use agents::cursor::CursorAgent;
pub use agents::opencode::OpenCodeAgent;

/// Create an agent instance from the agent type.
pub fn create_agent(agent_type: AgentType) -> Box<dyn Agent> {
    match agent_type {
        AgentType::ClaudeCode => Box::new(ClaudeCodeAgent::new()),
        AgentType::OpenCode => Box::new(OpenCodeAgent::new()),
        AgentType::Cursor => Box::new(CursorAgent::new()),
        AgentType::Aider => Box::new(AiderAgent::new()),  // Add factory case
    }
}
```

Don't forget to add the module declaration:

```rust
// crates/codeloops-agent/src/agents/mod.rs
pub mod aider;
pub mod claude;
pub mod cursor;
pub mod opencode;
```

## Step 4: Add CLI Support

Edit `crates/codeloops/src/main.rs`:

```rust
/// Agent choice for CLI arguments.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum AgentChoice {
    Claude,
    Opencode,
    Cursor,
    Aider,  // Add new variant
}

impl From<AgentChoice> for AgentType {
    fn from(choice: AgentChoice) -> Self {
        match choice {
            AgentChoice::Claude => AgentType::ClaudeCode,
            AgentChoice::Opencode => AgentType::OpenCode,
            AgentChoice::Cursor => AgentType::Cursor,
            AgentChoice::Aider => AgentType::Aider,  // Add mapping
        }
    }
}
```

## Step 5: Update Configuration

Edit `crates/codeloops/src/config.rs` to recognize the new agent string:

```rust
fn parse_agent(s: &str) -> Option<AgentType> {
    match s.to_lowercase().as_str() {
        "claude" => Some(AgentType::ClaudeCode),
        "opencode" => Some(AgentType::OpenCode),
        "cursor" => Some(AgentType::Cursor),
        "aider" => Some(AgentType::Aider),  // Add parsing
        _ => None,
    }
}
```

## Step 6: Write Tests

Create test file `crates/codeloops-agent/src/agents/aider_test.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_agent_name() {
        let agent = AiderAgent::new();
        assert_eq!(agent.name(), "Aider");
    }

    #[test]
    fn test_agent_type() {
        let agent = AiderAgent::new();
        assert_eq!(agent.agent_type(), AgentType::Aider);
    }

    #[test]
    fn test_binary_path() {
        let agent = AiderAgent::new();
        assert_eq!(agent.binary_path(), Path::new("aider"));
    }

    #[tokio::test]
    async fn test_execute_not_available() {
        // Test behavior when agent is not installed
        let agent = AiderAgent {
            binary_path: PathBuf::from("/nonexistent/aider"),
        };
        assert!(!agent.is_available().await);
    }
}
```

## Step 7: Update Documentation

### CLI Reference

Update `docs/src/user-guide/cli-reference.md`:

```markdown
Agent values: `claude`, `opencode`, `cursor`, `aider`
```

### Agents Guide

Update `docs/src/user-guide/agents.md`:

```markdown
## Supported Agents

| Agent | CLI Value | Binary | Description |
|-------|-----------|--------|-------------|
| Claude Code | `claude` | `claude` | Anthropic's Claude-powered coding agent |
| OpenCode | `opencode` | `opencode` | Multi-model coding agent |
| Cursor | `cursor` | `cursor` | Cursor IDE's agent CLI |
| Aider | `aider` | `aider` | AI pair programming in your terminal |

### Aider

Aider is an AI pair programming tool that works in your terminal.

**Binary**: `aider`

**Strengths**:
- Works with many LLM providers
- Good for iterative editing
- Supports multiple files

**Installation**: Visit [aider.chat](https://aider.chat/)
```

### Configuration Schema

Update `docs/src/reference/config-schema.md`:

```markdown
### Agent Values

| Value | Agent |
|-------|-------|
| `"claude"` | Claude Code |
| `"opencode"` | OpenCode |
| `"cursor"` | Cursor |
| `"aider"` | Aider |
```

## Step 8: Test the Integration

```bash
# Build
cargo build

# Run tests
cargo test -p codeloops-agent

# Test CLI
./target/debug/codeloops --agent aider --dry-run

# Test actual execution (requires aider installed)
./target/debug/codeloops --agent aider --prompt "Fix typo"
```

## Agent Implementation Tips

### Handle Different Output Formats

Agents may produce output in different formats. Normalize output for the critic:

```rust
fn normalize_output(&self, raw_output: &str) -> String {
    // Remove agent-specific noise
    // Standardize formatting
    raw_output.to_string()
}
```

### Handle Model Selection

Different agents support models differently:

```rust
// Some agents use --model
cmd.arg("--model").arg(model);

// Some use environment variables
cmd.env("MODEL_NAME", model);

// Some use different flag names
cmd.arg("-m").arg(model);
```

### Handle Working Directory

Agents should run in the specified working directory:

```rust
cmd.current_dir(&config.working_dir);
```

### Handle Prompts

Different agents accept prompts differently:

```rust
// Via argument
cmd.arg("--message").arg(prompt);

// Via stdin
cmd.stdin(Stdio::piped());
// Then write prompt to stdin after spawn

// Via file
let prompt_file = config.working_dir.join(".prompt");
std::fs::write(&prompt_file, prompt)?;
cmd.arg("--prompt-file").arg(&prompt_file);
```

### Handle Errors Gracefully

```rust
if !status.success() {
    // Don't fail immediately - let the critic handle it
    // The critic can provide recovery suggestions
}
```

## Checklist

Before submitting your PR:

- [ ] Agent trait implemented
- [ ] AgentType enum updated
- [ ] Factory function updated
- [ ] CLI AgentChoice added
- [ ] Configuration parsing updated
- [ ] Tests written
- [ ] Documentation updated (agents guide, CLI reference, config schema)
- [ ] All tests pass (`cargo test --workspace`)
- [ ] Code formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy --workspace`)
