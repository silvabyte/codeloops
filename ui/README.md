# Codeloops Session Viewer UI

Web interface for browsing, filtering, and visualizing codeloops actor-critic sessions.

## Prerequisites

- [Bun](https://bun.sh/) (v1.0+)

## Setup

```bash
bun install
```

## Development

Start the Vite dev server with hot module replacement:

```bash
bun dev
```

By default the dev server runs on `http://localhost:5173`. It expects the API server to be running at `http://localhost:3100` (or the value of `VITE_API_URL`).

To start both the API server and UI together in dev mode:

```bash
codeloops ui --dev
```

## Building for Production

Build the Vite production bundle:

```bash
bun run build
```

Compile the static file server into a standalone binary:

```bash
bun run compile
```

This produces a `codeloops-ui` binary that serves the `dist/` directory as a static SPA.

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `VITE_API_URL` | `http://localhost:3100` | Base URL for the codeloops API server |
| `PORT` | `3101` | Port for the compiled static file server |

## Tech Stack

- **React** with TypeScript
- **Vite** for bundling and dev server
- **Tailwind CSS** for styling (dark mode by default)
- **React Router v6** for client-side routing
- **shadcn/ui** for core UI components (table, card, badge, tabs, etc.)
- **Recharts** for charts (via shadcn chart component)

## Project Structure

```
src/
├── api/           # API client, types, SSE connection
├── components/    # Reusable UI components
├── hooks/         # React hooks for data fetching
├── lib/           # Utilities
├── pages/         # Route-level page components
├── App.tsx        # Router setup
└── main.tsx       # Entry point
```
