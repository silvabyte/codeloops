# Codeloops Session Viewer — Design

## Overview

A CLI + Web UI for browsing, filtering, and visualizing codeloops actor-critic sessions persisted as JSONL files.

## Architecture

```
codeloops CLI binary
├── codeloops run ...           (existing actor-critic loop)
├── codeloops sessions list     (text output, interactive picker)
├── codeloops sessions show     (detailed session dump)
├── codeloops sessions diff     (cumulative diff output)
├── codeloops sessions stats    (aggregate metrics)
└── codeloops ui [--dev]        (starts API + frontend)
      │
      ├── Rust API server (axum, localhost:PORT)
      │     ├── GET /api/sessions         (list/filter)
      │     ├── GET /api/sessions/:id     (full session)
      │     ├── GET /api/sessions/:id/diff (cumulative diff)
      │     ├── GET /api/stats            (aggregates)
      │     └── GET /api/sessions/live    (SSE stream)
      │
      └── Frontend server
            ├── --dev:  spawns `bun dev` (Vite HMR)
            └── prod:   spawns bundled Bun binary serving static build
```

## Data Source

JSONL files in `~/.local/share/codeloops/sessions/` with filename pattern `{timestamp}_{hash}.jsonl`.

Each file contains:
- Line 1: `session_start` — prompt, working_dir, agents, models, max_iterations
- Lines 2..N-1: `iteration` — iteration_number, actor_output, actor_stderr, actor_exit_code, actor_duration_secs, git_diff, git_files_changed, critic_decision, feedback, timestamp
- Line N: `session_end` — outcome, iterations, summary, confidence, duration_secs

## Session Identity

ID = filename stem (e.g. `2026-01-24T13-50-14Z_e171a1`)

## CLI Subcommands

### `codeloops sessions list`
- Tabular output: date, project (working_dir basename), prompt snippet, outcome, iterations, duration
- `--json` flag for scripting
- Interactive fuzzy-select picker when run without arguments or with `--pick`

### `codeloops sessions show [id]`
- If no id: launches interactive picker
- Prints formatted session: header, each iteration with actor output + critic feedback, end summary

### `codeloops sessions diff [id]`
- Same picker behavior
- Outputs concatenated git diffs across all iterations

### `codeloops sessions stats`
- Aggregate: total sessions, success rate, avg iterations, avg duration, sessions by project

### `codeloops ui [--dev]`
- Starts axum API server on a free port
- With `--dev`: spawns `bun dev` in the `ui/` directory (Vite HMR)
- Without `--dev`: spawns the compiled Bun binary serving the production build
- Auto-opens browser to frontend URL

## Rust API

### `GET /api/sessions`
Query params: `?outcome=success|failed|interrupted`, `?after=YYYY-MM-DD`, `?before=YYYY-MM-DD`, `?search=<prompt text>`

Response: array of session summaries (id, timestamp, prompt_preview, working_dir, outcome, iterations, duration_secs, confidence)

### `GET /api/sessions/:id`
Response: full parsed session (start + iterations + end)

### `GET /api/sessions/:id/diff`
Response: concatenated diffs as plain text

### `GET /api/stats`
Response: { total_sessions, success_rate, avg_iterations, avg_duration_secs, sessions_over_time: [{date, count}], by_project: [{project, count, success_rate}] }

### `GET /api/sessions/live` (SSE)
Uses `notify` crate to watch sessions directory. Events:
- `session_created` — new file appears
- `session_updated` — file modified (active session writing iterations)
- `session_completed` — session_end line detected

## Frontend

### Stack
- Bun + React + Vite + Tailwind CSS + shadcn/ui
- Scaffolded with `bun init --react=shadcn`
- Project location: `ui/` at repo root

### Component Libraries

**Core shadcn/ui:**
- Table/Data Table — session list with sorting, filtering, pagination
- Card — session summary cards, stat cards
- Badge — outcome badges (success/failed/interrupted), critic decision badges (DONE/CONTINUE)
- Tabs — switching between timeline/diff/feedback views
- Collapsible — expandable actor output and critic feedback per iteration
- Sidebar — app navigation
- Chart (Recharts) — success rate line chart, iteration bar chart, duration trends
- Skeleton — loading states
- Command — Cmd+K session search/jump
- Select / Date Picker — filters
- Progress — active session progress
- Scroll Area — long diffs and output
- Resizable — split panes for diff + feedback
- Tooltip — hover details on timeline nodes

**@kibo-ui:**
- `code-block` — syntax-highlighted git diffs with built-in diff mode (`[!code ++]`/`[!code --]`), Shiki-powered
- `gantt` — timeline visualization of iterations within a session
- `status` — live session status indicator with ping animation
- `relative-time` — "5 minutes ago" timestamps
- `contribution-graph` — GitHub-style activity heatmap on stats page

**@blocks:**
- `stats` — pre-built stat card layouts for dashboard metrics

### Pages

#### Dashboard (`/`)
- Quick stats bar (total sessions, success rate, avg iterations, avg duration)
- Session list table with filters (outcome, date range, prompt search)
- Live indicator for any active sessions via SSE

#### Session Detail (`/sessions/:id`)
- Header: full prompt, working_dir, agents, total duration
- Gantt timeline of iterations (horizontal time bars)
- For each iteration (expandable):
  - Actor output (collapsible, formatted)
  - Git diff (Kibo Code Block with diff highlighting)
  - Critic decision badge + feedback
  - Duration and timestamp
- Critic feedback trail: conversation-style view across iterations

#### Stats (`/stats`)
- Success rate over time (line chart)
- Iterations per session (bar chart)
- Duration trends (area chart)
- Contribution graph (activity heatmap)
- Per-project breakdown table

### Real-time
- SSE connection to `/api/sessions/live`
- Dashboard: new sessions animate into list, active sessions show live status
- Session detail: active session streams new iterations in real-time

### Styling
- Dark mode by default
- shadcn/ui theming variables
- Tailwind for layout
