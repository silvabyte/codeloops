# codeloops

An actor-critic harness for coding agents.

[![Documentation](https://img.shields.io/badge/docs-mdbook-blue)](https://silvabyte.github.io/codeloops)

Codeloops orchestrates coding agents (Claude Code, OpenCode, Cursor) in an actor-critic feedback loop. The **actor** executes coding tasks while the **critic** evaluates the work via git diff and stdout/stderr logs, continuing iterations until the task is complete.

## The Problem

Current coding agents lack a fundamental feedback mechanism: they execute tasks but cannot systematically verify their own work, despite all the extension points provided(plugins, skills, agent files, .rules, etc). Codeloops implements an actor-critic system where:

1. **Actor**: Executes the coding task (writes code, runs commands)
2. **Critic**: Reviews the actor's output and git changes
3. **Loop**: Continues with feedback until the critic approves

This creates a self-correcting loop that DRAMATICALLY improves task completion quality.

## Installation

```bash
# Clone and build
git clone https://github.com/silvabyte/codeloops
cd codeloops
cargo build --release

# Binary is at ./target/release/codeloops - I like to symlink this to my user bin.

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
codeloops --agent cursor --prompt "add unit tests for the API"

# Use different agents for actor and critic
codeloops --actor-agent claude --critic-agent opencode --prompt "optimize performance"
codeloops --actor-agent cursor --critic-agent claude --prompt "optimize performance"

# Limit iterations (default: unlimited)
codeloops --max-iterations 5 --prompt "complex refactoring task"

# JSON output for scripting
codeloops --json-output --prompt "task description"

# Dry run to see configuration
codeloops --dry-run --prompt "test"
```

After using all the coding agents in the world. Having any sort of ui or tui ends up being... overly complex and ultimitly restrictive. Why not just a prompt.md file? It's so much simpler, easier... all the things...

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
┌───────────────────────────┐                 ┌───────────────────────────────┐
│          ACTOR            │                 │           CRITIC              │
│  (claude/opencode/cursor) │────────────────▶│   (claude/opencode/cursor)    │
│  Executes task            │   git diff +    │   Evaluates work              │
│                           │   stdout/stderr │   Decides: done/continue      │
└───────────────────────────┘                 └───────────────────────────────┘
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

## Session Viewer

Codeloops persists every session as a JSONL file. You can browse, filter, and inspect sessions using the built-in CLI or web UI.

### CLI Session Commands

```bash
# List all sessions
codeloops sessions list

# Filter by outcome
codeloops sessions list --outcome success

# Search prompt text
codeloops sessions list --search "auth bug"

# Show detailed session info (interactive picker if no ID given)
codeloops sessions show

# Show cumulative diff from a session
codeloops sessions diff <session-id>

# Aggregate statistics
codeloops sessions stats
```

### Web UI

```bash
# Start the web UI (opens browser automatically)
codeloops ui

# Development mode (uses Vite dev server with HMR)
codeloops ui --dev

# Custom ports
codeloops ui --api-port 4000 --ui-port 4001
```

The web UI provides a dashboard with session list, filters, statistics charts, iteration timelines, critic feedback trails, and syntax-highlighted diffs.

## CLI Options

### Run (default subcommand)

| Option | Description |
|--------|-------------|
| `-p, --prompt <PROMPT>` | Task prompt (or reads from prompt.md) |
| `--prompt-file <FILE>` | Path to prompt file (default: prompt.md) |
| `-d, --working-dir <DIR>` | Working directory (default: current) |
| `-a, --agent <AGENT>` | Agent for both actor and critic (claude/opencode/cursor) |
| `--actor-agent <AGENT>` | Agent specifically for actor role |
| `--critic-agent <AGENT>` | Agent specifically for critic role |
| `-n, --max-iterations <N>` | Maximum iterations (default: unlimited) |
| `--log-format <FORMAT>` | Output format: pretty, json, compact |
| `-m, --model <MODEL>` | Model to use (if agent supports it) |
| `--json-output` | Output final result as JSON |
| `--dry-run` | Show configuration without executing |

### Sessions

| Subcommand | Description |
|------------|-------------|
| `sessions list` | List sessions with optional filters |
| `sessions show [ID]` | Show session detail (interactive picker if omitted) |
| `sessions diff [ID]` | Show cumulative git diff |
| `sessions stats` | Aggregate statistics |

### UI

| Option | Description |
|--------|-------------|
| `--dev` | Development mode (Vite HMR) |
| `--api-port <PORT>` | API server port (default: 3100) |
| `--ui-port <PORT>` | UI server port (default: 3101) |

## Supported Agents

| Agent | CLI | Status |
|-------|-----|--------|
| Claude Code | `claude` | Supported |
| OpenCode | `opencode` | Supported |
| Cursor | `cursor` | Supported |

Agents must be installed and available in your PATH.

## Project Structure

```
codeloops/
├── crates/
│   ├── codeloops/           # CLI binary + API server
│   ├── codeloops-core/      # Loop orchestration
│   ├── codeloops-agent/     # Agent abstraction layer
│   ├── codeloops-critic/    # Critic evaluation & decision parsing
│   ├── codeloops-git/       # Git diff capture
│   ├── codeloops-logging/   # Structured logging + session writer
│   └── codeloops-sessions/  # Session reading, parsing, store, watcher
├── ui/                      # Web UI (React + Vite + Tailwind)
└── docs/                    # Design documents
```

## Documentation

Full documentation is available at [silvabyte.github.io/codeloops](https://silvabyte.github.io/codeloops)

- [Getting Started](https://silvabyte.github.io/codeloops/getting-started/installation.html) - Installation and first session
- [CLI Reference](https://silvabyte.github.io/codeloops/user-guide/cli-reference.html) - Complete command reference
- [Configuration](https://silvabyte.github.io/codeloops/user-guide/configuration.html) - Global and project configuration
- [Architecture](https://silvabyte.github.io/codeloops/architecture/overview.html) - How codeloops works
- [Contributing](https://silvabyte.github.io/codeloops/contributing/development.html) - Development setup

## Adding New Agents

See the [Adding New Agents](https://silvabyte.github.io/codeloops/contributing/adding-agents.html) guide for detailed instructions.

Quick overview:
1. Implement the `Agent` trait in `codeloops-agent/src/`
2. Add the variant to `AgentType` enum
3. Update the CLI's `AgentChoice` enum
4. Update `create_agent()` factory function

## License

MIT
