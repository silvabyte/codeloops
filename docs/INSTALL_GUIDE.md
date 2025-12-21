# CodeLoops: Installation Guide

CodeLoops can be installed as an **OpenCode plugin** or as an **MCP server** for other clients.

## Prerequisites

- **Node.js**: Version 18 or higher
  - Download from [nodejs.org](https://nodejs.org) or use a version manager like `nvm`
  - Verify with: `node --version`

## Installation Steps

### Step 1: Clone the Repository

```bash
git clone https://github.com/silvabyte/codeloops.git
cd codeloops
npm install
```

---

## Option A: OpenCode Plugin Installation

The OpenCode plugin provides memory tools directly in your OpenCode sessions.

### Install the Plugin

```bash
npm run plugin:install
```

This creates a symlink from `~/.config/opencode/plugin/memory.ts` to your local plugin.

### Verify Installation

Start OpenCode in any project. You should see the memory tools available:

- `memory_store`
- `memory_recall`
- `memory_forget`
- `memory_context`
- `memory_projects`

### Auto-Capture Events

The plugin automatically captures:

- **File edits** - Every file.edited event is logged
- **Todo updates** - Todo list changes are tracked
- **Session start** - Loads recent memories when a session begins

---

## Option B: MCP Server Installation

The MCP server works with Claude Desktop, Cursor, and other MCP-compatible clients.

### Stdio Transport (Recommended)

Add to your MCP client configuration:

**Claude Desktop** (`~/Library/Application Support/Claude/claude_desktop_config.json` on macOS):

```json
{
  "mcpServers": {
    "codeloops": {
      "command": "npx",
      "args": ["-y", "tsx", "/absolute/path/to/codeloops/src"]
    }
  }
}
```

**Cursor** (`.cursor/mcp.json` in your project):

```json
{
  "mcpServers": {
    "codeloops": {
      "command": "npx",
      "args": ["-y", "tsx", "/absolute/path/to/codeloops/src"]
    }
  }
}
```

### HTTP Transport

For clients that support HTTP transport:

1. Start the server:

   ```bash
   npm run start:http
   # or with custom port
   npx -y tsx src --http --port 8080
   ```

2. Configure your client to connect to `http://localhost:3000` (or your custom port)

### Available MCP Tools

| Tool             | Description                                |
| ---------------- | ------------------------------------------ |
| `memory_store`   | Store a memory with content, project, tags |
| `memory_recall`  | Query memories by text, tags, project      |
| `memory_forget`  | Soft-delete a memory by ID                 |
| `memory_context` | Get recent memories for current project    |
| `list_projects`  | List all projects with memories            |
| `resume`         | Load recent memories to continue work      |

---

## Data Storage

All memories are stored locally as NDJSON files:

| Platform | Location                                                |
| -------- | ------------------------------------------------------- |
| Linux    | `~/.local/share/codeloops/memory.ndjson`                |
| macOS    | `~/Library/Application Support/codeloops/memory.ndjson` |
| Windows  | `%APPDATA%/codeloops/memory.ndjson`                     |

Deleted memories are moved to `memory.deleted.ndjson` in the same directory.

---

## Troubleshooting

### Plugin not loading in OpenCode

1. Verify the symlink exists:

   ```bash
   ls -la ~/.config/opencode/plugin/
   ```

2. Check that the source file exists:

   ```bash
   ls -la /path/to/codeloops/plugin/memory.ts
   ```

3. Reinstall:
   ```bash
   npm run plugin:install
   ```

### MCP server connection issues

1. Check the server is running:

   ```bash
   npm start
   # or for HTTP
   npm run start:http
   ```

2. Verify the path in your MCP config is absolute (starts with `/`)

3. Check logs in the `logs/` directory

### Data not persisting

1. Verify the data directory exists:

   ```bash
   # Linux
   ls ~/.local/share/codeloops/

   # macOS
   ls ~/Library/Application\ Support/codeloops/
   ```

2. Check file permissions

---

## Uninstalling

### Remove OpenCode Plugin

```bash
rm ~/.config/opencode/plugin/memory.ts
```

### Remove MCP Server Config

Remove the `codeloops` entry from your MCP client configuration.

### Remove Data (Optional)

```bash
# Linux
rm -rf ~/.local/share/codeloops/

# macOS
rm -rf ~/Library/Application\ Support/codeloops/
```
