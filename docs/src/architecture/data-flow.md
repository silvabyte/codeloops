# Data Flow

This document details how data flows through codeloops during execution.

## High-Level Flow

```
User Input          Processing           Storage           Output
───────────         ──────────           ───────           ──────

prompt.md ────────▶ CLI Parser ─────────────────────────▶ Console
                        │
                        ▼
                   Config Loader ───────────────────────▶ Console
                        │
                        ▼
                   Agent Factory
                        │
                        ▼
                   LoopRunner ──────────▶ SessionWriter ─▶ JSONL File
                        │                      │
              ┌─────────┴─────────┐           │
              ▼                   ▼           │
           Actor              Critic ─────────┤
              │                   │           │
              ▼                   ▼           │
          Git Diff ───────────────────────────┤
                                              │
                                              ▼
                                        Session File
```

## Detailed Sequence: Running a Session

### Phase 1: Initialization

```
┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│   User   │     │   CLI    │     │  Config  │     │ Agents   │
└────┬─────┘     └────┬─────┘     └────┬─────┘     └────┬─────┘
     │                │                │                │
     │ codeloops      │                │                │
     │────────────────▶                │                │
     │                │                │                │
     │                │ load global    │                │
     │                │───────────────▶│                │
     │                │                │                │
     │                │◀───────────────│                │
     │                │                │                │
     │                │ load project   │                │
     │                │───────────────▶│                │
     │                │                │                │
     │                │◀───────────────│                │
     │                │                │                │
     │                │ merge configs  │                │
     │                │───────▶        │                │
     │                │                │                │
     │                │ create agents  │                │
     │                │────────────────────────────────▶│
     │                │                │                │
     │                │◀────────────────────────────────│
     │                │                │                │
```

### Phase 2: Session Start

```
┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│   CLI    │     │  Runner  │     │  Writer  │     │   File   │
└────┬─────┘     └────┬─────┘     └────┬─────┘     └────┬─────┘
     │                │                │                │
     │ run(context)   │                │                │
     │───────────────▶│                │                │
     │                │                │                │
     │                │ create writer  │                │
     │                │───────────────▶│                │
     │                │                │                │
     │                │                │ create file    │
     │                │                │───────────────▶│
     │                │                │                │
     │                │ write_start()  │                │
     │                │───────────────▶│                │
     │                │                │                │
     │                │                │ append line    │
     │                │                │───────────────▶│
     │                │                │                │
```

### Phase 3: Actor Execution

```
┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│  Runner  │     │  Actor   │     │ Process  │     │   Git    │
└────┬─────┘     └────┬─────┘     └────┬─────┘     └────┬─────┘
     │                │                │                │
     │ build prompt   │                │                │
     │───────▶        │                │                │
     │                │                │                │
     │ execute()      │                │                │
     │───────────────▶│                │                │
     │                │                │                │
     │                │ spawn process  │                │
     │                │───────────────▶│                │
     │                │                │                │
     │                │                │ run agent CLI  │
     │                │                │ (claude/opencode)
     │                │                │                │
     │                │◀───────────────│ (stdout/stderr)
     │                │                │                │
     │◀───────────────│ AgentOutput    │                │
     │                │                │                │
     │ capture diff   │                │                │
     │─────────────────────────────────────────────────▶│
     │                │                │                │
     │◀─────────────────────────────────────────────────│
     │                │                │                │
```

### Phase 4: Critic Evaluation

```
┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│  Runner  │     │  Critic  │     │  Agent   │     │  Parser  │
└────┬─────┘     └────┬─────┘     └────┬─────┘     └────┬─────┘
     │                │                │                │
     │ evaluate()     │                │                │
     │───────────────▶│                │                │
     │                │                │                │
     │                │ build prompt   │                │
     │                │───────▶        │                │
     │                │                │                │
     │                │ execute()      │                │
     │                │───────────────▶│                │
     │                │                │                │
     │                │◀───────────────│ AgentOutput    │
     │                │                │                │
     │                │ parse response │                │
     │                │───────────────────────────────▶│
     │                │                │                │
     │                │◀───────────────────────────────│
     │                │                │  CriticDecision│
     │◀───────────────│                │                │
     │                │                │                │
```

### Phase 5: Iteration Recording

```
┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│  Runner  │     │  Writer  │     │   File   │     │  Logger  │
└────┬─────┘     └────┬─────┘     └────┬─────┘     └────┬─────┘
     │                │                │                │
     │ write_iteration()               │                │
     │───────────────▶│                │                │
     │                │                │                │
     │                │ serialize JSON │                │
     │                │───────▶        │                │
     │                │                │                │
     │                │ append line    │                │
     │                │───────────────▶│                │
     │                │                │                │
     │ log iteration  │                │                │
     │─────────────────────────────────────────────────▶│
     │                │                │                │
     │                │                │                │ console
     │                │                │                │ output
     │                │                │                │
```

### Phase 6: Loop Decision

```
┌──────────┐
│  Runner  │
└────┬─────┘
     │
     │ match decision
     │───────▶
     │
     ├─── DONE ───────────▶ write_end() ─▶ return Success
     │
     ├─── CONTINUE ───────▶ set_feedback() ─▶ loop back
     │
     └─── ERROR ──────────▶ set_feedback() ─▶ loop back
```

## Data Structures at Each Stage

### Input Data

```
prompt: String
working_dir: PathBuf
config: {
    actor_agent: AgentType,
    critic_agent: AgentType,
    actor_model: Option<String>,
    critic_model: Option<String>,
    max_iterations: Option<usize>,
}
```

### After Actor Execution

```
actor_output: {
    stdout: String,
    stderr: String,
    exit_code: i32,
    duration: Duration,
}
```

### After Diff Capture

```
diff_summary: {
    diff: String,
    files_changed: usize,
    insertions: usize,
    deletions: usize,
}
```

### After Critic Evaluation

```
decision: {
    type: "DONE" | "CONTINUE" | "ERROR",
    summary?: String,       // for DONE
    confidence?: f64,       // for DONE
    feedback?: String,      // for CONTINUE
    recovery?: String,      // for ERROR
}
```

### Session File (JSONL)

```
Line 1 (SessionStart):
{
    "type": "session_start",
    "timestamp": "2025-01-27T15:30:45Z",
    "prompt": "...",
    "working_dir": "/path/to/project",
    "actor_agent": "Claude Code",
    "critic_agent": "Claude Code",
    "actor_model": "sonnet",
    "critic_model": null,
    "max_iterations": 10
}

Line 2..N (Iteration):
{
    "type": "iteration",
    "iteration_number": 1,
    "actor_output": "...",
    "actor_stderr": "",
    "actor_exit_code": 0,
    "actor_duration_secs": 45.2,
    "git_diff": "...",
    "git_files_changed": 3,
    "critic_decision": "CONTINUE",
    "feedback": "...",
    "timestamp": "2025-01-27T15:31:30Z"
}

Final Line (SessionEnd):
{
    "type": "session_end",
    "outcome": "success",
    "iterations": 2,
    "summary": "...",
    "confidence": 0.95,
    "duration_secs": 89.4,
    "timestamp": "2025-01-27T15:32:14Z"
}
```

## API Data Flow

### List Sessions

```
Browser                  API                    SessionStore
   │                      │                          │
   │ GET /api/sessions    │                          │
   │─────────────────────▶│                          │
   │                      │                          │
   │                      │ list_sessions(filter)    │
   │                      │─────────────────────────▶│
   │                      │                          │
   │                      │                          │ read first/last
   │                      │                          │ lines of each file
   │                      │                          │
   │                      │◀─────────────────────────│
   │                      │   Vec<SessionSummary>    │
   │                      │                          │
   │◀─────────────────────│                          │
   │   JSON response      │                          │
   │                      │                          │
```

### Get Session Detail

```
Browser                  API                    SessionStore
   │                      │                          │
   │ GET /api/sessions/id │                          │
   │─────────────────────▶│                          │
   │                      │                          │
   │                      │ load_session(id)         │
   │                      │─────────────────────────▶│
   │                      │                          │
   │                      │                          │ read entire
   │                      │                          │ JSONL file
   │                      │                          │
   │                      │◀─────────────────────────│
   │                      │     Session              │
   │                      │                          │
   │◀─────────────────────│                          │
   │   JSON response      │                          │
   │                      │                          │
```

### Live Updates (SSE)

```
Browser                  API                    SessionWatcher
   │                      │                          │
   │ GET /api/sessions/live (SSE)                    │
   │─────────────────────▶│                          │
   │                      │                          │
   │                      │ subscribe()              │
   │                      │─────────────────────────▶│
   │                      │                          │
   │◀ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─│ (connection held open)  │
   │                      │                          │
   │                      │                          │ file change
   │                      │                          │ detected
   │                      │◀ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─│
   │                      │    SessionEvent         │
   │                      │                          │
   │◀─────────────────────│                          │
   │   SSE event          │                          │
   │                      │                          │
```

## File System Layout

```
~/.config/codeloops/
└── config.toml                    # Global configuration

~/.local/share/codeloops/
├── sessions/                      # Session storage
│   ├── 2025-01-27T15-30-45Z_a3f2c1.jsonl
│   ├── 2025-01-27T14-22-10Z_b5d3e2.jsonl
│   └── ...
└── ui/                           # UI assets (optional)
    ├── index.html
    ├── assets/
    └── ...

/path/to/project/
├── codeloops.toml                # Project configuration (optional)
├── prompt.md                     # Default prompt file
└── ...                           # Project files
```

## Concurrency Model

- **Actor execution**: Single-threaded, synchronous process spawn
- **Critic execution**: Single-threaded, synchronous process spawn
- **Session writing**: Append-only, no concurrent writers
- **API server**: Multi-threaded (tokio async runtime)
- **SSE**: Async broadcast channel for events

The loop itself is sequential (actor → diff → critic → decision → repeat), ensuring consistent state and predictable behavior.
