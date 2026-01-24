# AI Agent Instructions for Codeloops

**Codeloops** is a Rust-based actor-critic harness for orchestrating AI coding agents in a feedback loop.

## Project Overview

This project automates AI-assisted coding with built-in verification:
1. An **actor** (coding agent like Claude Code) executes a task
2. A **critic** (another agent) reviews the changes via git diff and output logs
3. The loop continues with feedback until the task is approved

### Architecture

- **codeloops** - CLI binary, API server (axum), and `ui` command
- **codeloops-core** - Loop orchestration logic
- **codeloops-agent** - Agent abstraction layer
- **codeloops-critic** - Critic evaluation and decision parsing
- **codeloops-git** - Git diff capture
- **codeloops-logging** - Structured logging and JSONL session writer
- **codeloops-sessions** - Session reading/parsing/store/watcher (shared by CLI and API)
- **ui/** - Web UI (React + Vite + Tailwind CSS). Run `codeloops ui --dev` for development

## Issue Tracking with bd (beads)

**IMPORTANT**: This project uses **bd (beads)** for ALL issue tracking. Do NOT use markdown TODOs, task lists, or other tracking methods.

### Why bd?

- Dependency-aware: Track blockers and relationships between issues
- Git-friendly: Auto-syncs to JSONL for version control
- Agent-optimized: JSON output, ready work detection, discovered-from links
- Prevents duplicate tracking systems and confusion

### Quick Start

**Check for ready work:**
```bash
bd ready --json
```

**Create new issues:**
```bash
bd create "Issue title" -t bug|feature|task -p 0-4 --json
bd create "Issue title" -p 1 --deps discovered-from:cl-123 --json
bd create "Subtask" --parent <epic-id> --json  # Hierarchical subtask
```

**Claim and update:**
```bash
bd update cl-42 --status in_progress --json
bd update cl-42 --priority 1 --json
```

**Complete work:**
```bash
bd close cl-42 --reason "Completed" --json
```

### Issue Types

- `bug` - Something broken
- `feature` - New functionality
- `task` - Work item (tests, docs, refactoring)
- `epic` - Large feature with subtasks
- `chore` - Maintenance (dependencies, tooling)

### Priorities

- `0` - Critical (security, data loss, broken builds)
- `1` - High (major features, important bugs)
- `2` - Medium (default, nice-to-have)
- `3` - Low (polish, optimization)
- `4` - Backlog (future ideas)

### Workflow for AI Agents

1. **Check ready work**: `bd ready` shows unblocked issues
2. **Claim your task**: `bd update <id> --status in_progress`
3. **Work on it**: Implement, test, document
4. **Discover new work?** Create linked issue:
   - `bd create "Found bug" -p 1 --deps discovered-from:<parent-id>`
5. **Complete**: `bd close <id> --reason "Done"`
6. **Commit together**: Always commit the `.beads/issues.jsonl` file together with the code changes so issue state stays in sync with code state

### Auto-Sync

bd automatically syncs with git:
- Exports to `.beads/issues.jsonl` after changes (5s debounce)
- Imports from JSONL when newer (e.g., after `git pull`)
- No manual export/import needed!

### CLI Help

Run `bd <command> --help` to see all available flags for any command.
For example: `bd create --help` shows `--parent`, `--deps`, `--assignee`, etc.

### Important Rules

- Always use bd for ALL task tracking
- Always use `--json` flag for programmatic use
- Link discovered work with `discovered-from` dependencies
- Check `bd ready` before asking "what should I work on?"
- Do NOT create markdown TODO lists
- Do NOT duplicate tracking systems

## Development

### Building

```bash
cargo build
cargo build --release
```

### Testing

```bash
cargo test
```

### Running

```bash
cargo run -- --help
cargo run -- sessions list
cargo run -- ui --dev
```

### Frontend Development

```bash
cd ui
bun install
bun dev          # Vite dev server with HMR
bun run build    # Production build
```

### Workspace Dependencies

Key additions for the session viewer:
- `axum` 0.8 - API server
- `tower-http` 0.6 - CORS middleware
- `tokio-stream` 0.1 - SSE streaming
- `notify` 7 - File system watching
- `dialoguer` 0.11 - Interactive CLI picker
- `open` 5 - Browser opening
