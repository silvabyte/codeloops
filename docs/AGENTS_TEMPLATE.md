# AGENTS.md Template for CodeLoops Memory Integration

Copy this section into your project's AGENTS.md file to enable memory-aware AI coding.

---

## Memory System (CodeLoops)

This project uses **CodeLoops** for persistent memory across AI coding sessions.

### Available Memory Tools

| Tool             | Usage                                              |
| ---------------- | -------------------------------------------------- |
| `memory_store`   | Save decisions, context, preferences, or learnings |
| `memory_recall`  | Query past memories by text search or tags         |
| `memory_forget`  | Remove outdated or incorrect memories              |
| `memory_context` | Load recent project context at session start       |

### When to Store Memories

Store memories for:

- **Decisions**: "We chose X over Y because..."
- **Preferences**: "User prefers functional components"
- **Errors**: "This error was caused by X, fixed with Y"
- **Context**: "The auth system uses JWT tokens stored in..."
- **Learnings**: "Discovered that API X requires header Y"

### Recommended Tags

Use consistent tags for better recall:

- `decision` - Architectural and design decisions
- `preference` - User/project preferences
- `error` - Error patterns and solutions
- `api` - API behavior and quirks
- `pattern` - Code patterns to follow
- `avoid` - Anti-patterns to avoid

### Example Usage

**At session start:**

```
Use memory_context to load recent context for this project
```

**Before making decisions:**

```
Use memory_recall to check if we have any memories about [topic]
```

**After important decisions:**

```
Use memory_store to save: "We decided to [decision] because [rationale]"
Tags: decision, [relevant-area]
```

**When encountering errors:**

```
Use memory_store to save: "Error [X] was caused by [Y], fixed with [Z]"
Tags: error, [relevant-area]
```

### Auto-Captured Events (OpenCode Plugin)

If using the OpenCode plugin, these are captured automatically:

- File edits (tagged: `file-edit`, `auto-capture`)
- Todo updates (tagged: `todo`, `auto-capture`)

---

_Memory data is stored locally in `~/.local/share/codeloops/` (Linux) or equivalent._
