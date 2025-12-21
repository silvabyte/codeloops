![CodeLoops](media/svg/codeloops_banner.svg)

# CodeLoops: Memory Layer for AI Coding Agents

CodeLoops provides **persistent memory** for AI coding agents, enabling them to remember context, decisions, and learnings across sessions. It works as both an **MCP server** (for Claude Desktop, Cursor, etc.) and an **OpenCode plugin**.

> **Note**: CodeLoops is in active development. Back up your data before upgrading.

## Why CodeLoops?

AI coding agents are powerful but forgetful. They lose context between sessions, repeat mistakes, and forget your preferences. CodeLoops solves this with:

- **Persistent Memory**: Store decisions, context, preferences, and learnings
- **Cross-Session Recall**: Query past memories to inform current work
- **Auto-Capture**: Automatically log file edits and todo updates
- **Dual Integration**: Works with MCP clients (Claude, Cursor) and OpenCode

## Quick Start

### Option 1: OpenCode Plugin

```bash
# Clone and install
git clone https://github.com/silvabyte/codeloops.git
cd codeloops
npm install

# Install the plugin
npm run plugin:install
```

The plugin provides these tools in OpenCode:

- `memory_store` - Save a memory
- `memory_recall` - Query memories
- `memory_forget` - Delete a memory
- `memory_context` - Load recent context
- `memory_projects` - List all projects

Plus auto-capture of file edits and todo updates.

### Option 2: MCP Server

Add to your MCP client configuration:

**Stdio Transport (Claude Desktop, Cursor)**

```json
{
  "mcpServers": {
    "codeloops": {
      "command": "npx",
      "args": ["-y", "tsx", "/path/to/codeloops/src"]
    }
  }
}
```

**HTTP Transport**

```bash
# Start the server
npm run start:http

# Then configure your client to connect to http://localhost:3000
```

## Available Tools

| Tool             | Description                                            |
| ---------------- | ------------------------------------------------------ |
| `memory_store`   | Store a memory with content, tags, and optional source |
| `memory_recall`  | Query memories by text search, tags, or project        |
| `memory_forget`  | Soft-delete a memory (moves to deleted log)            |
| `memory_context` | Quick retrieval of recent project memories             |
| `list_projects`  | List all projects with stored memories                 |
| `resume`         | Load recent memories to continue where you left off    |

## Usage Examples

### Storing Memories

```
Use memory_store to save this decision: "We chose PostgreSQL over MongoDB for ACID compliance"
Tags: database, architecture, decision
```

### Recalling Context

```
Use memory_recall to find any memories about database decisions
```

### Starting a Session

```
Use memory_context to load recent context for this project
```

## Data Storage

Memories are stored as NDJSON (newline-delimited JSON) in:

- **Linux**: `~/.local/share/codeloops/memory.ndjson`
- **macOS**: `~/Library/Application Support/codeloops/memory.ndjson`
- **Windows**: `%APPDATA%/codeloops/memory.ndjson`

Each memory entry contains:

```json
{
  "id": "abc123",
  "project": "my-project",
  "content": "The memory content",
  "tags": ["decision", "architecture"],
  "createdAt": "2024-01-15T10:30:00.000Z",
  "sessionId": "session-xyz",
  "source": "manual"
}
```

## CLI Options

```bash
# Start MCP server (stdio, default)
npm start

# Start HTTP server
npm run start:http

# Custom port/host
npx -y tsx src --http --port 8080 --host 127.0.0.1

# Install OpenCode plugin
npm run plugin:install
```

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                    AI Coding Agent                   │
│              (Claude, Cursor, OpenCode)              │
└──────────────────────┬──────────────────────────────┘
                       │
         ┌─────────────┴─────────────┐
         │                           │
         ▼                           ▼
┌─────────────────┐       ┌─────────────────┐
│   MCP Server    │       │ OpenCode Plugin │
│  (stdio/HTTP)   │       │   (memory.ts)   │
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

## Contributing

This project is experimental. Contributions welcome!

- [GitHub Issues](https://github.com/silvabyte/codeloops/issues)
- Email: [mat@silvabyte.com](mailto:mat@silvabyte.com)
- X: [@MatSilva](https://x.com/MatSilva)

## License

MIT - see [LICENSE](./LICENSE)
