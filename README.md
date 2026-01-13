# codeloops

An actor-critic harness for coding agents.

Codeloops orchestrates coding agents (Claude Code, OpenCode) in an actor-critic feedback loop. The **actor** executes coding tasks while the **critic** evaluates the work via git diff and stdout/stderr logs, continuing iterations until the task is complete.

## Why?

Current coding agents lack a fundamental feedback mechanism: they execute tasks but don't systematically verify their own work. Codeloops implements an actor-critic system where:

1. **Actor**: Executes the coding task (writes code, runs commands)
2. **Critic**: Reviews the actor's output and git changes
3. **Loop**: Continues with feedback until the critic approves

This creates a self-correcting loop that dramatically improves task completion quality.

## Installation

```bash
# Clone and build
git clone https://github.com/matsilva/codeloops
cd codeloops
cargo build --release

# Binary is at ./target/release/codeloops
```

## Usage

```bash
# Basic usage with inline prompt
codeloops --prompt "fix the authentication bug in login.rs"

# Using a prompt.md file (default)
echo "implement user registration with email verification" > prompt.md
codeloops

# Specify which agent to use
codeloops --agent claude --prompt "add unit tests for the API"
codeloops --agent opencode --prompt "refactor the database layer"

# Use different agents for actor and critic
codeloops --actor-agent claude --critic-agent opencode --prompt "optimize performance"

# Limit iterations (default: unlimited)
codeloops --max-iterations 5 --prompt "complex refactoring task"

# JSON output for scripting
codeloops --json-output --prompt "task description"

# Dry run to see configuration
codeloops --dry-run --prompt "test"
```

## How It Works

```
┌─────────────────────────────────────────────────────────────────┐
│                         codeloops CLI                           │
│  --prompt "task" | prompt.md                                    │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                        Loop Runner                              │
│  Orchestrates iterations until task complete                    │
└─────────────────────────────────────────────────────────────────┘
                                │
           ┌────────────────────┴────────────────────┐
           ▼                                         ▼
┌─────────────────────┐                 ┌─────────────────────────┐
│       ACTOR         │                 │        CRITIC           │
│  (claude/opencode)  │────────────────▶│   (claude/opencode)     │
│  Executes task      │   git diff +    │   Evaluates work        │
│                     │   stdout/stderr │   Decides: done/continue│
└─────────────────────┘                 └─────────────────────────┘
                                                     │
                                        ┌────────────┴────────────┐
                                        ▼                         ▼
                                   [DONE]                   [CONTINUE]
                                   Exit with               Feed back to
                                   success                 actor, repeat
```

### Iteration Flow

1. **Actor executes** the task prompt (or feedback from previous iteration)
2. **Git diff captured** showing all file changes
3. **Critic evaluates** the actor's stdout, stderr, and git diff
4. **Decision made**:
   - `DONE`: Task complete, exit successfully
   - `CONTINUE`: Provide feedback, run another iteration
   - `ERROR`: Provide recovery suggestion, continue

## CLI Options

| Option | Description |
|--------|-------------|
| `-p, --prompt <PROMPT>` | Task prompt (or reads from prompt.md) |
| `--prompt-file <FILE>` | Path to prompt file (default: prompt.md) |
| `-d, --working-dir <DIR>` | Working directory (default: current) |
| `-a, --agent <AGENT>` | Agent for both actor and critic (claude/opencode) |
| `--actor-agent <AGENT>` | Agent specifically for actor role |
| `--critic-agent <AGENT>` | Agent specifically for critic role |
| `-n, --max-iterations <N>` | Maximum iterations (default: unlimited) |
| `--log-format <FORMAT>` | Output format: pretty, json, compact |
| `-m, --model <MODEL>` | Model to use (if agent supports it) |
| `--json-output` | Output final result as JSON |
| `--dry-run` | Show configuration without executing |

## Supported Agents

| Agent | CLI | Status |
|-------|-----|--------|
| Claude Code | `claude` | Supported |
| OpenCode | `opencode` | Supported |

Agents must be installed and available in your PATH.

## Project Structure

```
codeloops/
├── crates/
│   ├── codeloops/           # CLI binary
│   ├── codeloops-core/      # Loop orchestration
│   ├── codeloops-agent/     # Agent abstraction layer
│   ├── codeloops-critic/    # Critic evaluation & decision parsing
│   ├── codeloops-git/       # Git diff capture
│   └── codeloops-logging/   # Structured logging
```

## Adding New Agents

1. Implement the `Agent` trait in `codeloops-agent/src/`
2. Add the variant to `AgentType` enum
3. Update the CLI's `AgentChoice` enum
4. Update `create_agent()` factory function

## License

MIT
