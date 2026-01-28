# Web UI Overview

Codeloops includes a web-based interface for browsing and analyzing sessions. This guide provides an overview of what the UI offers.

## What the Web UI Provides

### Session List

Browse all recorded sessions with:
- Sortable columns (timestamp, project, outcome, iterations, duration)
- Quick filters for outcome and project
- Search functionality for prompts
- Date range filtering

### Session Detail View

Examine individual sessions with:
- Full prompt display
- Session metadata (agents, working directory, duration)
- Iteration-by-iteration breakdown

### Iteration Timeline

Visualize the actor-critic loop:
- Timeline showing each iteration
- Duration bars for actor execution
- Color-coded critic decisions (DONE, CONTINUE, ERROR)
- Click to expand iteration details

### Syntax-Highlighted Diffs

View code changes with:
- Syntax highlighting for common languages
- Side-by-side or unified diff view
- File-by-file navigation
- Line number display

### Statistics and Charts

Analyze patterns across sessions:
- Success rate over time
- Average iterations per project
- Session duration trends
- Project-wise breakdown

### Real-Time Updates

When a session is in progress:
- Live iteration updates via Server-Sent Events
- Progress indicators
- Auto-refresh of session list

## When to Use CLI vs UI

### Use the CLI when:

- Doing quick lookups (`codeloops sessions show <id>`)
- Scripting and automation
- Working in a terminal-only environment
- Piping output to other tools

### Use the UI when:

- Browsing multiple sessions
- Comparing iterations visually
- Analyzing patterns and statistics
- Reviewing complex diffs
- Sharing session details with others

## Technology Stack

The web UI is built with:

| Technology | Purpose |
|------------|---------|
| React 19 | UI framework |
| React Router 7 | Client-side routing |
| TypeScript | Type safety |
| Vite | Build tool and dev server |
| Tailwind CSS 4 | Styling |
| Recharts | Charts and visualizations |
| Bun | JavaScript runtime and bundler |

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Web Browser                            │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                    React UI                           │  │
│  │   Dashboard │ Session Detail │ Stats │ Diff Viewer   │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ HTTP / SSE
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    API Server (Rust)                        │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │  /sessions  │  │    /stats    │  │  /sessions/live  │   │
│  │   REST API  │  │   REST API   │  │       SSE        │   │
│  └─────────────┘  └──────────────┘  └──────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              Session Store (JSONL Files)                    │
│        ~/.local/share/codeloops/sessions/*.jsonl            │
└─────────────────────────────────────────────────────────────┘
```

## Ports

By default:
- **API Server**: Port 3100
- **UI Server**: Port 3101 (development) or served by API (production)

Both are configurable via CLI flags.

## Browser Support

The UI works in modern browsers:
- Chrome/Chromium (recommended)
- Firefox
- Safari
- Edge

## Next Steps

- [Usage Guide](./usage.md) - How to use the UI
- [Development Guide](./development.md) - Contributing to the UI
