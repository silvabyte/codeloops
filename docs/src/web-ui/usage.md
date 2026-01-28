# Web UI Usage

This guide covers how to use the codeloops web interface.

## Starting the UI

### Basic Start

```bash
codeloops ui
```

This:
1. Starts the API server on port 3100
2. Serves the UI on port 3101
3. Opens your default browser automatically

### Custom Ports

```bash
codeloops ui --api-port 4000 --ui-port 4001
```

### Development Mode

For UI development with hot reloading:

```bash
codeloops ui --dev
```

## Navigating the Interface

### Sidebar

The sidebar provides navigation:

| Section | Description |
|---------|-------------|
| Dashboard | Session list with filters |
| Stats | Statistics and charts |

### Dashboard (Session List)

The main view shows all sessions in a table:

| Column | Description |
|--------|-------------|
| Timestamp | When the session started |
| Project | Project name (working directory basename) |
| Outcome | success, failed, interrupted, or max_iterations_reached |
| Iterations | Number of actor-critic loops |
| Duration | Total session time |
| Prompt | First 100 characters of the prompt |

**Sorting**: Click column headers to sort.

**Row click**: Opens the session detail view.

### Filters

Above the session table:

**Outcome Filter**: Dropdown to filter by outcome type.

**Project Filter**: Dropdown to filter by project name.

**Search**: Text input to search prompts.

**Date Range**: Start and end date pickers.

**Clear Filters**: Reset all filters.

### Session Detail View

Click a session to open its detail view:

#### Header Section

- Session ID
- Timestamp
- Working directory
- Actor agent and model
- Critic agent and model
- Total duration
- Final outcome

#### Prompt Section

The full prompt text, rendered as markdown.

#### Iteration Timeline

Visual timeline of iterations:

```
[===========] 45s  ──▶  CONTINUE
[=======] 32s      ──▶  DONE
```

Each bar represents an iteration:
- Width proportional to duration
- Color indicates critic decision:
  - Green: DONE
  - Yellow: CONTINUE
  - Red: ERROR

#### Iteration Details

Click an iteration to expand:

**Actor Output**: What the agent produced (stdout)

**Git Diff**: Syntax-highlighted diff of changes

**Critic Feedback**: Feedback text (for CONTINUE decisions)

**Metadata**: Duration, exit code, files changed

### Diff Viewer

The diff viewer shows code changes:

**File Tabs**: If multiple files changed, tabs let you switch between them.

**Syntax Highlighting**: Code is highlighted based on file extension.

**Line Numbers**: Both old and new line numbers shown.

**Change Indicators**:
- Green background: Added lines
- Red background: Removed lines
- No background: Unchanged context lines

### Stats Page

The statistics page shows:

#### Summary Cards

- Total Sessions
- Success Rate
- Average Iterations
- Average Duration

#### Charts

**Sessions Over Time**: Bar chart showing sessions per day.

**Success Rate Trend**: Line chart of success rate over time.

**Iterations Distribution**: Histogram of iteration counts.

**Duration Distribution**: Histogram of session durations.

#### By Project

Table breakdown per project:
- Session count
- Success rate
- Average iterations
- Average duration

### Real-Time Updates

When sessions are running:

**Session List**: New sessions appear automatically.

**Active Session**: If viewing an active session, iterations appear as they complete.

**Status Indicator**: Shows "Live" badge for in-progress sessions.

The UI uses Server-Sent Events (SSE) for real-time updates without polling.

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `/` | Focus search input |
| `Esc` | Close detail view / clear search |
| `j` | Next session in list |
| `k` | Previous session in list |
| `Enter` | Open selected session |

## URL Structure

The UI uses client-side routing:

| URL | View |
|-----|------|
| `/` | Dashboard (session list) |
| `/sessions/:id` | Session detail |
| `/stats` | Statistics page |

You can bookmark or share these URLs directly.

## Troubleshooting

### UI Won't Start

**Port already in use**:
```
Error: Address already in use (port 3100)
```

Solution: Use different ports:
```bash
codeloops ui --api-port 4000 --ui-port 4001
```

**UI directory not found**:
```
Error: UI directory not found
```

Solution: Build the UI or set `CODELOOPS_UI_DIR`:
```bash
cd ui && bun run build
# or
export CODELOOPS_UI_DIR=/path/to/ui/dist
```

### No Sessions Showing

Check that sessions exist:
```bash
ls ~/.local/share/codeloops/sessions/
```

If empty, run some codeloops sessions first.

### Slow Loading

Large sessions (many iterations, large diffs) may load slowly. The API paginates where possible, but individual session detail views load the full session.

### Browser Console Errors

Open browser developer tools (F12) and check the console for errors. Common issues:
- CORS errors: Ensure the API server is running
- 404 errors: API endpoint issues
- Network errors: Port/firewall problems

## Tips

1. **Use filters**: Large session lists load faster with filters applied.

2. **Bookmark searches**: The URL includes filter state, so bookmark filtered views.

3. **Compare iterations**: Use the iteration timeline to quickly see where changes happened.

4. **Export data**: Use the CLI for programmatic access to session data.
