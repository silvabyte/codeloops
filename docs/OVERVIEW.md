![CodeLoops](../media/svg/codeloops_banner.svg)

# CodeLoops - Overview

A lightweight, persistent memory layer for AI coding agents.

## The Problem

AI coding agents are powerful but suffer from **session amnesia**:

| Problem              | Symptom                                          |
| -------------------- | ------------------------------------------------ |
| **Context loss**     | Forgotten APIs, duplicated components, dead code |
| **No learning**      | Same mistakes repeated across sessions           |
| **Lost decisions**   | Rationale for choices not preserved              |
| **Preference drift** | User preferences forgotten                       |

Every new session starts from scratch. The agent has no memory of what worked, what failed, or what you prefer.

## The Solution

CodeLoops provides a simple, append-only memory store that persists across sessions:

```
┌─────────────────────────────────────────────────────┐
│                    AI Coding Agent                   │
└──────────────────────┬──────────────────────────────┘
                       │
         ┌─────────────┴─────────────┐
         │                           │
         ▼                           ▼
┌─────────────────┐       ┌─────────────────┐
│   MCP Server    │       │ OpenCode Plugin │
└────────┬────────┘       └────────┬────────┘
         │                         │
         └───────────┬─────────────┘
                     │
                     ▼
          ┌─────────────────┐
          │   MemoryStore   │
          │  (NDJSON file)  │
          └─────────────────┘
```

## Core Concepts

### Memory Entries

Each memory is a simple JSON object:

```json
{
  "id": "abc123",
  "project": "my-app",
  "content": "User prefers functional components over class components",
  "tags": ["preference", "react"],
  "createdAt": "2024-01-15T10:30:00.000Z",
  "sessionId": "session-xyz",
  "source": "user-input"
}
```

### Project Scoping

Memories are scoped by project name (derived from directory path). This keeps memories relevant to the current codebase.

### Tags

Use tags for flexible categorization:

- `decision` - Architectural or design decisions
- `preference` - User preferences and conventions
- `error` - Error patterns and solutions
- `context` - Important project context
- `file-edit` - Auto-captured file changes
- `todo` - Auto-captured todo updates

### Auto-Capture (OpenCode Plugin)

The plugin automatically captures:

- **File edits** - Every saved file is logged
- **Todo updates** - Todo list changes are tracked
- **Session start** - Recent memories load automatically

## How It Works

### Storing Memories

```
Agent: "We decided to use PostgreSQL for ACID compliance"
→ memory_store(content="...", tags=["decision", "database"])
→ Saved to memory.ndjson
```

### Recalling Context

```
Agent: "What database did we choose?"
→ memory_recall(query="database", tags=["decision"])
→ Returns matching memories
```

### Session Continuity

```
New session starts
→ memory_context(limit=5)
→ Agent receives recent project context
→ Continues where it left off
```

## Integration Options

### MCP Server

For Claude Desktop, Cursor, and other MCP clients:

- Stdio transport (default)
- HTTP transport (for web clients)

### OpenCode Plugin

For OpenCode users:

- Native tool integration
- Event hooks for auto-capture
- Session-aware context loading

## Data Storage

All data is stored locally as NDJSON (newline-delimited JSON):

| Platform | Location                                   |
| -------- | ------------------------------------------ |
| Linux    | `~/.local/share/codeloops/`                |
| macOS    | `~/Library/Application Support/codeloops/` |
| Windows  | `%APPDATA%/codeloops/`                     |

Files:

- `memory.ndjson` - Active memories
- `memory.deleted.ndjson` - Soft-deleted memories

## Design Principles

1. **Simple** - Just an append-only log, no complex database
2. **Local** - Your data stays on your machine
3. **Portable** - NDJSON is human-readable and easy to backup
4. **Lightweight** - Minimal dependencies, fast startup
5. **Flexible** - Tags over rigid schemas

## License

MIT - see [LICENSE](../LICENSE)
