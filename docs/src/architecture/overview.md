# Architecture Overview

This document provides a high-level overview of codeloops' architecture, design philosophy, and component structure.

## Design Philosophy

### Simplicity

Codeloops embraces simplicity at every level:

- **Interface**: A `prompt.md` file is the primary interface. No complex configuration required.
- **Execution**: Run `codeloops` and the loop handles the rest.
- **Output**: JSONL session files are human-readable and tool-friendly.

### Composability

The system is designed for flexibility:

- **Agent-agnostic**: Works with any coding agent that has a CLI.
- **Mixed configurations**: Use different agents for actor and critic roles.
- **Extensible**: Adding new agents requires implementing a simple trait.

### Observability

Every action is recorded:

- **Full session logging**: All inputs, outputs, and decisions are captured.
- **Iteration history**: See exactly what happened at each step.
- **Statistics**: Analyze patterns across sessions.

## System Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              User Interface                                 │
│  ┌───────────────┐  ┌────────────────┐  ┌───────────────────────────────┐  │
│  │   prompt.md   │  │  CLI (codeloops) │  │         Web UI              │  │
│  │               │  │                  │  │  (sessions, stats, diffs)   │  │
│  └───────┬───────┘  └────────┬─────────┘  └──────────────┬──────────────┘  │
└──────────┼───────────────────┼───────────────────────────┼──────────────────┘
           │                   │                           │
           ▼                   ▼                           ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                            Core System                                      │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                         LoopRunner                                  │   │
│  │                    (codeloops-core crate)                           │   │
│  │                                                                     │   │
│  │   ┌─────────────┐          ┌─────────────┐          ┌───────────┐  │   │
│  │   │   Actor     │ ──────▶  │   Git Diff   │ ──────▶  │  Critic   │  │   │
│  │   │   Agent     │          │   Capture    │          │  Agent    │  │   │
│  │   └─────────────┘          └─────────────┘          └───────────┘  │   │
│  │         │                                                  │       │   │
│  │         │                  ┌─────────────┐                 │       │   │
│  │         └──────────────────│  Feedback   │◀────────────────┘       │   │
│  │                            │    Loop     │                         │   │
│  │                            └─────────────┘                         │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│                                    ▼                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                      Session Writer                                 │   │
│  │                  (codeloops-logging crate)                          │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
└────────────────────────────────────┼────────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Storage Layer                                     │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │             ~/.local/share/codeloops/sessions/*.jsonl               │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Component Overview

### CLI Binary (`codeloops` crate)

The main entry point. Responsibilities:
- Parse command-line arguments
- Load configuration (global and project)
- Initialize the loop runner
- Handle session commands
- Serve the web UI and API

### Loop Runner (`codeloops-core` crate)

Orchestrates the actor-critic loop. Responsibilities:
- Execute actor agents
- Capture git diffs
- Execute critic agents
- Parse critic decisions
- Manage iteration flow
- Handle interrupts (Ctrl+C)

### Agent Abstraction (`codeloops-agent` crate)

Provides a unified interface for coding agents. Responsibilities:
- Define the `Agent` trait
- Implement agents (Claude, OpenCode, Cursor)
- Spawn agent processes
- Capture output (stdout, stderr, exit code)

### Critic Evaluation (`codeloops-critic` crate)

Handles critic logic. Responsibilities:
- Build evaluation prompts
- Parse critic decisions (DONE, CONTINUE, ERROR)
- Extract feedback and confidence scores

### Git Operations (`codeloops-git` crate)

Manages git interactions. Responsibilities:
- Capture diffs between iterations
- Track changed files
- Provide diff summaries

### Logging (`codeloops-logging` crate)

Handles output and session recording. Responsibilities:
- Format log output (pretty, JSON, compact)
- Write session JSONL files
- Handle structured events

### Session Management (`codeloops-sessions` crate)

Reads and queries sessions. Responsibilities:
- Parse JSONL session files
- Provide session summaries (fast)
- Filter and search sessions
- Calculate statistics
- Watch for session changes

## Data Flow

1. **Input**: User provides prompt via `prompt.md` or `--prompt`

2. **Configuration**: CLI loads global and project config, resolves agent choices

3. **Initialization**: LoopRunner created with actor/critic agents

4. **Session Start**: SessionWriter creates JSONL file, writes start line

5. **Actor Execution**: Actor agent spawned with prompt, output captured

6. **Diff Capture**: Git diff computed between before/after states

7. **Critic Evaluation**: Critic agent spawned with actor output + diff

8. **Decision Parsing**: Critic response parsed for decision and feedback

9. **Session Update**: Iteration recorded to JSONL file

10. **Loop Control**: If DONE, end session. If CONTINUE, feed back to actor.

11. **Completion**: SessionEnd line written, process exits

## Why This Architecture?

### Separation of Concerns

Each crate has a single responsibility:
- `codeloops-agent`: Knows how to run agents
- `codeloops-critic`: Knows how to evaluate
- `codeloops-git`: Knows how to diff
- `codeloops-core`: Orchestrates everything

This makes testing and modification easier.

### Extensibility

Adding a new agent:
1. Implement the `Agent` trait
2. Add it to the agent factory
3. No changes to core loop logic

### Observability

JSONL session files provide:
- Complete audit trail
- Easy parsing (standard JSON)
- Append-only writes (crash-safe)
- Human readability

### Performance

Design choices for performance:
- Fast session summaries (read first/last lines only)
- Streaming output from agents
- Minimal overhead in the loop

## Next Steps

- [The Actor-Critic Loop](./actor-critic.md) - Detailed loop mechanics
- [Crate Structure](./crates.md) - Deep dive into each crate
- [Data Flow](./data-flow.md) - Sequence diagrams and data paths
