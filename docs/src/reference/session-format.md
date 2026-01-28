# Session Format

This document provides a complete reference for the codeloops session file format.

## Overview

Sessions are stored as JSONL (JSON Lines) files. Each line is a valid JSON object representing one event in the session.

**Location**: `~/.local/share/codeloops/sessions/`

**Filename format**: `<timestamp>_<hash>.jsonl`

Example: `2025-01-27T15-30-45Z_a3f2c1.jsonl`

- `timestamp`: ISO 8601 format with hyphens replacing colons (filesystem-safe)
- `hash`: First 6 characters of SHA256(prompt)

## Line Types

Every line has a `type` field indicating its kind:

| Type | Occurrence | Description |
|------|------------|-------------|
| `session_start` | Exactly once | First line, session metadata |
| `iteration` | Zero or more | One per actor-critic cycle |
| `session_end` | Zero or once | Last line, final outcome |

## SessionStart

The first line of every session file.

### Schema

```json
{
  "type": "session_start",
  "timestamp": "<ISO 8601 datetime>",
  "prompt": "<string>",
  "working_dir": "<path>",
  "actor_agent": "<string>",
  "critic_agent": "<string>",
  "actor_model": "<string | null>",
  "critic_model": "<string | null>",
  "max_iterations": "<integer | null>"
}
```

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | string | Yes | Always `"session_start"` |
| `timestamp` | string | Yes | ISO 8601 datetime when session started |
| `prompt` | string | Yes | Full task prompt text |
| `working_dir` | string | Yes | Absolute path to working directory |
| `actor_agent` | string | Yes | Actor agent name (e.g., "Claude Code") |
| `critic_agent` | string | Yes | Critic agent name (e.g., "Claude Code") |
| `actor_model` | string/null | Yes | Actor model name or null if not specified |
| `critic_model` | string/null | Yes | Critic model name or null if not specified |
| `max_iterations` | integer/null | Yes | Iteration limit or null if unlimited |

### Example

```json
{
  "type": "session_start",
  "timestamp": "2025-01-27T15:30:45Z",
  "prompt": "Add input validation to the user registration endpoint.\n\nRequirements:\n- Email must be valid format\n- Password must be at least 8 characters",
  "working_dir": "/home/user/projects/myapp",
  "actor_agent": "Claude Code",
  "critic_agent": "Claude Code",
  "actor_model": "sonnet",
  "critic_model": null,
  "max_iterations": 10
}
```

## Iteration

One line per actor-critic cycle. Sessions may have zero iterations (if the actor fails immediately).

### Schema

```json
{
  "type": "iteration",
  "iteration_number": "<integer>",
  "actor_output": "<string>",
  "actor_stderr": "<string>",
  "actor_exit_code": "<integer>",
  "actor_duration_secs": "<float>",
  "git_diff": "<string>",
  "git_files_changed": "<integer>",
  "critic_decision": "<string>",
  "feedback": "<string | null>",
  "timestamp": "<ISO 8601 datetime>"
}
```

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | string | Yes | Always `"iteration"` |
| `iteration_number` | integer | Yes | 1-indexed iteration number |
| `actor_output` | string | Yes | Actor's stdout output |
| `actor_stderr` | string | Yes | Actor's stderr output |
| `actor_exit_code` | integer | Yes | Actor's process exit code |
| `actor_duration_secs` | float | Yes | Actor execution time in seconds |
| `git_diff` | string | Yes | Unified diff of changes |
| `git_files_changed` | integer | Yes | Number of files modified |
| `critic_decision` | string | Yes | `"DONE"`, `"CONTINUE"`, or `"ERROR"` |
| `feedback` | string/null | Yes | Critic feedback (null for DONE) |
| `timestamp` | string | Yes | ISO 8601 datetime when iteration completed |

### Critic Decision Values

| Value | Meaning |
|-------|---------|
| `DONE` | Task complete, no more iterations |
| `CONTINUE` | More work needed, feedback provided |
| `ERROR` | Error occurred, recovery suggestion provided |

### Example (CONTINUE)

```json
{
  "type": "iteration",
  "iteration_number": 1,
  "actor_output": "I've added email validation using a regex pattern. The validation is now in place for the registration endpoint.",
  "actor_stderr": "",
  "actor_exit_code": 0,
  "actor_duration_secs": 45.2,
  "git_diff": "diff --git a/src/api/users.rs b/src/api/users.rs\nindex 1234567..abcdefg 100644\n--- a/src/api/users.rs\n+++ b/src/api/users.rs\n@@ -10,6 +10,12 @@ pub async fn register(data: Json<RegisterRequest>) {\n+    if !is_valid_email(&data.email) {\n+        return Err(ApiError::BadRequest(\"Invalid email format\"));\n+    }\n",
  "git_files_changed": 1,
  "critic_decision": "CONTINUE",
  "feedback": "Email validation is implemented correctly. However, password validation is missing. Please add:\n1. Minimum 8 character length check\n2. Error message for invalid passwords",
  "timestamp": "2025-01-27T15:31:30Z"
}
```

### Example (DONE)

```json
{
  "type": "iteration",
  "iteration_number": 2,
  "actor_output": "I've added password validation with minimum length check. Both email and password are now validated.",
  "actor_stderr": "",
  "actor_exit_code": 0,
  "actor_duration_secs": 32.1,
  "git_diff": "diff --git a/src/api/users.rs b/src/api/users.rs\n...",
  "git_files_changed": 1,
  "critic_decision": "DONE",
  "feedback": null,
  "timestamp": "2025-01-27T15:32:02Z"
}
```

### Example (ERROR)

```json
{
  "type": "iteration",
  "iteration_number": 1,
  "actor_output": "",
  "actor_stderr": "error[E0433]: failed to resolve: use of undeclared crate or module `validator`",
  "actor_exit_code": 101,
  "actor_duration_secs": 12.5,
  "git_diff": "",
  "git_files_changed": 0,
  "critic_decision": "ERROR",
  "feedback": "The actor encountered a compilation error. The `validator` crate is not in Cargo.toml. Please either:\n1. Add `validator = \"0.16\"` to Cargo.toml, or\n2. Implement validation manually without the external crate",
  "timestamp": "2025-01-27T15:31:00Z"
}
```

## SessionEnd

The last line of a completed session. May be absent if the session is still in progress or crashed.

### Schema

```json
{
  "type": "session_end",
  "outcome": "<string>",
  "iterations": "<integer>",
  "summary": "<string | null>",
  "confidence": "<float | null>",
  "duration_secs": "<float>",
  "timestamp": "<ISO 8601 datetime>"
}
```

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | string | Yes | Always `"session_end"` |
| `outcome` | string | Yes | Session outcome (see values below) |
| `iterations` | integer | Yes | Total number of iterations completed |
| `summary` | string/null | Yes | Task completion summary (for success) |
| `confidence` | float/null | Yes | Confidence score 0.0-1.0 (for success) |
| `duration_secs` | float | Yes | Total session duration in seconds |
| `timestamp` | string | Yes | ISO 8601 datetime when session ended |

### Outcome Values

| Value | Description |
|-------|-------------|
| `success` | Critic returned DONE, task complete |
| `failed` | Unrecoverable error occurred |
| `interrupted` | User pressed Ctrl+C |
| `max_iterations_reached` | Hit iteration limit without completion |

### Example (success)

```json
{
  "type": "session_end",
  "outcome": "success",
  "iterations": 2,
  "summary": "Input validation has been added to the user registration endpoint. Email addresses are validated using RFC 5321 compliant regex. Passwords require minimum 8 characters. Appropriate error messages are returned for invalid inputs.",
  "confidence": 0.95,
  "duration_secs": 89.4,
  "timestamp": "2025-01-27T15:32:14Z"
}
```

### Example (max_iterations_reached)

```json
{
  "type": "session_end",
  "outcome": "max_iterations_reached",
  "iterations": 10,
  "summary": null,
  "confidence": null,
  "duration_secs": 482.3,
  "timestamp": "2025-01-27T15:38:45Z"
}
```

## Complete Example

A full session file:

```json
{"type":"session_start","timestamp":"2025-01-27T15:30:45Z","prompt":"Fix the typo in greeting.rs","working_dir":"/home/user/myapp","actor_agent":"Claude Code","critic_agent":"Claude Code","actor_model":null,"critic_model":null,"max_iterations":null}
{"type":"iteration","iteration_number":1,"actor_output":"I found and fixed the typo. 'Helo' is now 'Hello'.","actor_stderr":"","actor_exit_code":0,"actor_duration_secs":23.4,"git_diff":"diff --git a/src/greeting.rs b/src/greeting.rs\n--- a/src/greeting.rs\n+++ b/src/greeting.rs\n@@ -1 +1 @@\n-println!(\"Helo, World!\");\n+println!(\"Hello, World!\");","git_files_changed":1,"critic_decision":"DONE","feedback":null,"timestamp":"2025-01-27T15:31:08Z"}
{"type":"session_end","outcome":"success","iterations":1,"summary":"Fixed the typo in greeting.rs. Changed 'Helo' to 'Hello'.","confidence":1.0,"duration_secs":23.4,"timestamp":"2025-01-27T15:31:08Z"}
```

## Parsing Sessions

### Using jq

```bash
# Get session prompt
head -1 session.jsonl | jq -r '.prompt'

# Get all critic decisions
jq -r 'select(.type == "iteration") | .critic_decision' session.jsonl

# Get final outcome
tail -1 session.jsonl | jq -r '.outcome'

# Count iterations
jq -s '[.[] | select(.type == "iteration")] | length' session.jsonl
```

### Using Python

```python
import json

def parse_session(filepath):
    with open(filepath) as f:
        lines = [json.loads(line) for line in f]

    start = lines[0]
    iterations = [l for l in lines if l.get("type") == "iteration"]
    end = lines[-1] if lines[-1].get("type") == "session_end" else None

    return {
        "start": start,
        "iterations": iterations,
        "end": end,
    }
```

### Using Rust

```rust
use codeloops_sessions::{SessionStore, Session};

let store = SessionStore::new()?;
let session: Session = store.load_session("2025-01-27T15-30-45Z_a3f2c1")?;

println!("Prompt: {}", session.start.prompt);
println!("Iterations: {}", session.iterations.len());
if let Some(end) = session.end {
    println!("Outcome: {}", end.outcome);
}
```

## Session Discovery

Sessions are stored in a flat directory. To discover sessions:

```bash
# List all session files
ls ~/.local/share/codeloops/sessions/*.jsonl

# Find sessions by date
ls ~/.local/share/codeloops/sessions/2025-01-27*.jsonl

# Find sessions by prompt hash
ls ~/.local/share/codeloops/sessions/*_a3f2c1.jsonl
```

## File Integrity

Sessions are written in append-only mode:
- Lines are written atomically (full line or nothing)
- No line is ever modified after writing
- Crashes leave the file in a consistent state

If a session file is missing `session_end`, the session was either:
- Interrupted before completion
- Crashed unexpectedly
- Still in progress
