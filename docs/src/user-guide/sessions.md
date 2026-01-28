# Sessions

Every codeloops run is recorded as a session. This guide covers how to view, filter, and analyze your sessions.

## Session Storage

Sessions are stored as JSONL files in:

```
~/.local/share/codeloops/sessions/
```

Each session is a single file named:

```
<timestamp>_<hash>.jsonl
```

For example: `2025-01-27T15-30-45Z_a3f2c1.jsonl`

The hash is derived from the prompt, making it easy to identify related sessions.

## Listing Sessions

### Basic List

```bash
codeloops sessions list
```

Output:

```
ID                            Project    Outcome   Iters  Duration  Prompt
2025-01-27T15-30-45Z_a3f2c1   myapp      success   2      89.4s     Add input validation to...
2025-01-27T14-22-10Z_b5d3e2   myapp      success   1      45.1s     Fix the typo in greeting...
2025-01-26T09-15-33Z_c7f4a9   api-svc    failed    5      312.8s    Implement OAuth flow...
```

### Filtering by Outcome

```bash
# Only successful sessions
codeloops sessions list --outcome success

# Only failed sessions
codeloops sessions list --outcome failed

# Interrupted sessions (Ctrl+C)
codeloops sessions list --outcome interrupted

# Max iterations reached
codeloops sessions list --outcome max_iterations_reached
```

### Filtering by Date

```bash
# Sessions after a date
codeloops sessions list --after 2025-01-01

# Sessions before a date
codeloops sessions list --before 2025-01-31

# Date range
codeloops sessions list --after 2025-01-01 --before 2025-01-31
```

### Filtering by Project

```bash
codeloops sessions list --project myapp
```

The project name is the basename of the working directory where the session ran.

### Searching Prompts

```bash
codeloops sessions list --search "authentication"
```

This searches the prompt text for the given substring.

### Combining Filters

```bash
codeloops sessions list --outcome success --project myapp --after 2025-01-01
```

## Viewing Session Details

### Interactive Picker

```bash
codeloops sessions show
```

This opens an interactive picker to browse and select a session.

### Specific Session

```bash
codeloops sessions show 2025-01-27T15-30-45Z_a3f2c1
```

Output includes:
- Session metadata (timestamp, agents, working directory)
- Full prompt text
- Each iteration with actor output and critic feedback
- Final outcome and summary

## Viewing Session Diffs

### Interactive Picker

```bash
codeloops sessions diff
```

### Specific Session

```bash
codeloops sessions diff 2025-01-27T15-30-45Z_a3f2c1
```

This shows the cumulative git diff from all iterations. The diff is syntax-highlighted in the terminal.

## Session Statistics

```bash
codeloops sessions stats
```

Output:

```
Session Statistics
==================

Total Sessions:    47
Success Rate:      78.7%
Avg Iterations:    2.3
Avg Duration:      94.2s

By Project:
  myapp:           23 sessions (82.6% success)
  api-svc:         15 sessions (73.3% success)
  web-frontend:    9 sessions (77.8% success)

Sessions Over Time:
  2025-01-27:      5 sessions
  2025-01-26:      8 sessions
  2025-01-25:      12 sessions
  ...
```

## Session Outcomes

| Outcome | Description |
|---------|-------------|
| `success` | Critic approved the work (DONE decision) |
| `failed` | Error during execution |
| `interrupted` | User pressed Ctrl+C |
| `max_iterations_reached` | Hit the iteration limit without completion |

## Understanding Session Content

### Session Start

Contains initial configuration:
- Timestamp
- Prompt
- Working directory
- Actor and critic agents
- Models (if specified)
- Max iterations (if set)

### Iterations

Each iteration records:
- Iteration number
- Actor output (stdout)
- Actor stderr
- Actor exit code
- Actor duration
- Git diff
- Files changed count
- Critic decision (DONE, CONTINUE, or ERROR)
- Critic feedback (if CONTINUE)
- Timestamp

### Session End

Final status:
- Outcome
- Total iterations
- Summary (from critic)
- Confidence score (0-1)
- Total duration

## Programmatic Access

### Using jq

Sessions are JSONL (one JSON object per line), making them easy to parse:

```bash
# Get the prompt from a session
head -1 ~/.local/share/codeloops/sessions/2025-01-27T15-30-45Z_a3f2c1.jsonl | jq -r '.prompt'

# Get all critic decisions
cat ~/.local/share/codeloops/sessions/2025-01-27T15-30-45Z_a3f2c1.jsonl | jq -r 'select(.type == "iteration") | .critic_decision'

# Get the final outcome
tail -1 ~/.local/share/codeloops/sessions/2025-01-27T15-30-45Z_a3f2c1.jsonl | jq -r '.outcome'
```

### Using Python

```python
import json
from pathlib import Path

session_file = Path.home() / ".local/share/codeloops/sessions/2025-01-27T15-30-45Z_a3f2c1.jsonl"

with open(session_file) as f:
    lines = [json.loads(line) for line in f]

start = lines[0]
iterations = [l for l in lines if l.get("type") == "iteration"]
end = lines[-1] if lines[-1].get("type") == "session_end" else None

print(f"Prompt: {start['prompt']}")
print(f"Iterations: {len(iterations)}")
if end:
    print(f"Outcome: {end['outcome']}")
```

### Using Rust

The `codeloops-sessions` crate provides session parsing:

```rust
use codeloops_sessions::{SessionStore, SessionFilter};

let store = SessionStore::new()?;
let sessions = store.list_sessions(&SessionFilter::default())?;

for summary in sessions {
    println!("{}: {}", summary.id, summary.prompt_preview);
}

// Load a full session
let session = store.load_session("2025-01-27T15-30-45Z_a3f2c1")?;
println!("Iterations: {}", session.iterations.len());
```

## Managing Sessions

### Deleting Sessions

Sessions are plain files. Delete them directly:

```bash
# Delete a specific session
rm ~/.local/share/codeloops/sessions/2025-01-27T15-30-45Z_a3f2c1.jsonl

# Delete all sessions older than 30 days
find ~/.local/share/codeloops/sessions -name "*.jsonl" -mtime +30 -delete
```

### Backing Up Sessions

```bash
# Copy all sessions to a backup location
cp -r ~/.local/share/codeloops/sessions ~/backups/codeloops-sessions-$(date +%Y%m%d)
```

### Session Size

Sessions can grow large if:
- The actor produces verbose output
- Many iterations occur
- Large diffs are generated

Check session sizes:

```bash
ls -lh ~/.local/share/codeloops/sessions/
```

## Web UI for Sessions

For visual session browsing:

```bash
codeloops ui
```

The web UI provides:
- Session list with filters
- Iteration timeline
- Syntax-highlighted diffs
- Critic feedback trail
- Statistics and charts

See [Web UI Overview](../web-ui/overview.md) for details.
