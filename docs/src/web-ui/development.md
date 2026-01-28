# Web UI Development

This guide covers how to develop and contribute to the codeloops web UI.

## Prerequisites

- [Bun](https://bun.sh/) (recommended) or Node.js 18+
- Rust toolchain (for the API server)

## Development Setup

### Clone and Install

```bash
git clone https://github.com/matsilva/codeloops
cd codeloops/ui
bun install
```

### Start Development Servers

Option 1: Use the integrated dev mode:

```bash
codeloops ui --dev
```

This starts both the API server and Vite dev server with hot reloading.

Option 2: Run servers separately:

```bash
# Terminal 1: API server
cargo run -- ui --dev

# Terminal 2: Vite dev server (if needed separately)
cd ui && bun dev
```

### Development URLs

- **UI**: http://localhost:3101
- **API**: http://localhost:3100

In dev mode, the UI proxies API requests to the backend automatically.

## Project Structure

```
ui/
├── src/
│   ├── main.tsx              # Application entry point
│   ├── App.tsx               # Root component with routing
│   │
│   ├── api/
│   │   ├── types.ts          # TypeScript interfaces
│   │   └── client.ts         # API client functions
│   │
│   ├── pages/
│   │   ├── Dashboard.tsx     # Session list view
│   │   ├── SessionDetail.tsx # Single session view
│   │   ├── Stats.tsx         # Statistics page
│   │   └── NotFound.tsx      # 404 page
│   │
│   ├── components/
│   │   ├── Layout.tsx        # Main layout with sidebar
│   │   ├── Welcome.tsx       # Empty state component
│   │   ├── SessionTable.tsx  # Sessions list table
│   │   ├── SessionFilters.tsx# Filter controls
│   │   ├── StatsBar.tsx      # Statistics summary
│   │   ├── IterationTimeline.tsx # Visual timeline
│   │   ├── CriticTrail.tsx   # Critic feedback display
│   │   └── DiffViewer.tsx    # Code diff viewer
│   │
│   ├── hooks/
│   │   ├── useSessions.ts    # Fetch sessions list
│   │   ├── useSession.ts     # Fetch single session
│   │   ├── useStats.ts       # Fetch statistics
│   │   └── useSSE.ts         # Server-Sent Events
│   │
│   └── lib/
│       └── utils.ts          # Utility functions
│
├── package.json
├── vite.config.ts            # Vite configuration
├── tsconfig.json             # TypeScript configuration
├── tailwind.config.js        # Tailwind CSS configuration
└── serve.ts                  # Standalone server entry
```

## Key Components

### API Client (`src/api/client.ts`)

Functions for fetching data from the backend:

```typescript
// Fetch all sessions with optional filters
export async function fetchSessions(filters?: SessionFilter): Promise<SessionSummary[]>

// Fetch a single session by ID
export async function fetchSession(id: string): Promise<Session>

// Fetch session diff
export async function fetchSessionDiff(id: string): Promise<string>

// Fetch statistics
export async function fetchStats(): Promise<SessionStats>
```

### Types (`src/api/types.ts`)

TypeScript interfaces matching the Rust types:

```typescript
interface SessionSummary {
  id: string
  timestamp: string
  prompt_preview: string
  working_dir: string
  project: string
  outcome: string | null
  iterations: number
  duration_secs: number | null
  confidence: number | null
  actor_agent: string
  critic_agent: string
}

interface Session {
  id: string
  start: SessionStart
  iterations: Iteration[]
  end: SessionEnd | null
}
```

### Hooks

Custom React hooks for data fetching:

```typescript
// useSessions.ts
export function useSessions(filters?: SessionFilter) {
  const [sessions, setSessions] = useState<SessionSummary[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<Error | null>(null)
  // ... fetch logic
  return { sessions, loading, error, refetch }
}
```

### SSE Hook (`src/hooks/useSSE.ts`)

Handles real-time updates:

```typescript
export function useSSE(onEvent: (event: SessionEvent) => void) {
  useEffect(() => {
    const eventSource = new EventSource('/api/sessions/live')
    eventSource.onmessage = (e) => {
      const event = JSON.parse(e.data)
      onEvent(event)
    }
    return () => eventSource.close()
  }, [onEvent])
}
```

## Adding a New Page

1. Create the page component in `src/pages/`:

```typescript
// src/pages/MyPage.tsx
export default function MyPage() {
  return (
    <div>
      <h1>My Page</h1>
    </div>
  )
}
```

2. Add the route in `src/App.tsx`:

```typescript
import MyPage from './pages/MyPage'

// In the Routes:
<Route path="/my-page" element={<MyPage />} />
```

3. Add navigation in `src/components/Layout.tsx`:

```typescript
<NavLink to="/my-page">My Page</NavLink>
```

## Adding a New Component

1. Create the component in `src/components/`:

```typescript
// src/components/MyComponent.tsx
interface MyComponentProps {
  title: string
  children: React.ReactNode
}

export function MyComponent({ title, children }: MyComponentProps) {
  return (
    <div className="p-4 border rounded">
      <h2 className="text-lg font-bold">{title}</h2>
      {children}
    </div>
  )
}
```

2. Import and use it:

```typescript
import { MyComponent } from '../components/MyComponent'

<MyComponent title="Hello">
  Content here
</MyComponent>
```

## Styling

The UI uses Tailwind CSS. Add classes directly to elements:

```tsx
<div className="flex items-center gap-4 p-4 bg-gray-100 rounded-lg">
  <span className="text-sm text-gray-600">Label</span>
  <button className="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600">
    Click me
  </button>
</div>
```

## Building for Production

```bash
cd ui
bun run build
```

Output goes to `ui/dist/`.

## Creating a Standalone Binary

```bash
cd ui
bun run compile
```

This creates a `codeloops-ui` binary that serves the UI without needing Bun installed.

## Testing

### Type Checking

```bash
bun run typecheck
# or
npx tsc --noEmit
```

### Linting

```bash
bun run lint
# or
npx eslint src/
```

## API Development

The API server is in Rust at `crates/codeloops/src/api/`. Key files:

| File | Purpose |
|------|---------|
| `mod.rs` | Router setup |
| `sessions.rs` | Session endpoints |
| `stats.rs` | Statistics endpoint |
| `sse.rs` | Server-Sent Events |

To add a new API endpoint:

1. Add the handler function in the appropriate file
2. Add the route in `mod.rs`:

```rust
.route("/api/my-endpoint", get(my_handler))
```

3. Add the corresponding client function in `ui/src/api/client.ts`

## Common Tasks

### Update API Types

When Rust types change:

1. Update `ui/src/api/types.ts` to match
2. Update any affected components

### Add a New Filter

1. Add the filter field to `SessionFilter` in `types.ts`
2. Update `SessionFilters.tsx` to include the new control
3. Update `fetchSessions()` to send the filter parameter
4. Update the Rust API to handle the new filter

### Add a Chart

1. Add the data field to the appropriate type
2. Import from Recharts:

```typescript
import { BarChart, Bar, XAxis, YAxis, Tooltip } from 'recharts'
```

3. Add the chart component:

```tsx
<BarChart data={data}>
  <XAxis dataKey="name" />
  <YAxis />
  <Tooltip />
  <Bar dataKey="value" fill="#3b82f6" />
</BarChart>
```

## Debugging

### Browser DevTools

- **Network tab**: Check API requests
- **Console**: View errors and logs
- **React DevTools**: Inspect component state

### API Debugging

```bash
# Test API endpoints directly
curl http://localhost:3100/api/sessions
curl http://localhost:3100/api/stats
```

### SSE Debugging

In browser console:

```javascript
const es = new EventSource('http://localhost:3100/api/sessions/live')
es.onmessage = (e) => console.log(JSON.parse(e.data))
```
