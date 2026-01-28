# AI Agent Instructions for Codeloops

**Codeloops** is a Rust-based actor-critic harness for orchestrating AI coding agents in a feedback loop.

## Project Overview

This project automates AI-assisted coding with built-in verification:
1. An **actor** (coding agent like Claude Code) executes a task
2. A **critic** (another agent) reviews the changes via git diff and output logs
3. The loop continues with feedback until the task is approved

### Architecture

- **codeloops** - CLI binary, API server (axum), and `ui` command
- **codeloops-core** - Loop orchestration logic
- **codeloops-agent** - Agent abstraction layer
- **codeloops-critic** - Critic evaluation and decision parsing
- **codeloops-git** - Git diff capture
- **codeloops-logging** - Structured logging and JSONL session writer
- **codeloops-sessions** - Session reading/parsing/store/watcher (shared by CLI and API)
- **ui/** - Web UI (React + Vite + Tailwind CSS). Run `codeloops ui --dev` for development

## Issue Tracking with bd (beads)

**IMPORTANT**: This project uses **bd (beads)** for ALL issue tracking. Do NOT use markdown TODOs, task lists, or other tracking methods.

### Why bd?

- Dependency-aware: Track blockers and relationships between issues
- Git-friendly: Auto-syncs to JSONL for version control
- Agent-optimized: JSON output, ready work detection, discovered-from links
- Prevents duplicate tracking systems and confusion

### Quick Start

**Check for ready work:**
```bash
bd ready --json
```

**Create new issues:**
```bash
bd create "Issue title" -t bug|feature|task -p 0-4 --json
bd create "Issue title" -p 1 --deps discovered-from:cl-123 --json
bd create "Subtask" --parent <epic-id> --json  # Hierarchical subtask
```

**Claim and update:**
```bash
bd update cl-42 --status in_progress --json
bd update cl-42 --priority 1 --json
```

**Complete work:**
```bash
bd close cl-42 --reason "Completed" --json
```

### Issue Types

- `bug` - Something broken
- `feature` - New functionality
- `task` - Work item (tests, docs, refactoring)
- `epic` - Large feature with subtasks
- `chore` - Maintenance (dependencies, tooling)

### Priorities

- `0` - Critical (security, data loss, broken builds)
- `1` - High (major features, important bugs)
- `2` - Medium (default, nice-to-have)
- `3` - Low (polish, optimization)
- `4` - Backlog (future ideas)

### Workflow for AI Agents

1. **Check ready work**: `bd ready` shows unblocked issues
2. **Claim your task**: `bd update <id> --status in_progress`
3. **Work on it**: Implement, test, document
4. **Discover new work?** Create linked issue:
   - `bd create "Found bug" -p 1 --deps discovered-from:<parent-id>`
5. **Complete**: `bd close <id> --reason "Done"`
6. **Commit together**: Always commit the `.beads/issues.jsonl` file together with the code changes so issue state stays in sync with code state

### Auto-Sync

bd automatically syncs with git:
- Exports to `.beads/issues.jsonl` after changes (5s debounce)
- Imports from JSONL when newer (e.g., after `git pull`)
- No manual export/import needed!

### CLI Help

Run `bd <command> --help` to see all available flags for any command.
For example: `bd create --help` shows `--parent`, `--deps`, `--assignee`, etc.

### Important Rules

- Always use bd for ALL task tracking
- Always use `--json` flag for programmatic use
- Link discovered work with `discovered-from` dependencies
- Check `bd ready` before asking "what should I work on?"
- Do NOT create markdown TODO lists
- Do NOT duplicate tracking systems

## Development

### Building

```bash
cargo build
cargo build --release
```

### Testing

```bash
cargo test
```

### Running

```bash
cargo run -- --help
cargo run -- sessions list
cargo run -- ui --dev
```

### Frontend Development

```bash
cd ui
bun install
bun dev          # Vite dev server with HMR
bun run build    # Production build
```

### Workspace Dependencies

Key additions for the session viewer:
- `axum` 0.8 - API server
- `tower-http` 0.6 - CORS middleware
- `tokio-stream` 0.1 - SSE streaming
- `notify` 7 - File system watching
- `dialoguer` 0.11 - Interactive CLI picker
- `open` 5 - Browser opening

## Prompt Generator Interview Protocol

The `codeloops prompt` command uses a JSON protocol for communication between the agent and the TUI. This section documents the protocol for agent developers implementing interview support.

### Overview

When conducting an interview for `codeloops prompt`, agents receive a system prompt explaining the protocol and must respond with valid JSON (not wrapped in markdown code blocks). The TUI parses these messages to update the UI and build the draft prompt.

### Message Types

Agents send messages to the TUI using these JSON formats:

#### Question

Ask the user a question. The TUI will display appropriate input widgets based on `input_type`.

```json
{
  "type": "question",
  "text": "What is the main goal of this feature?",
  "context": "Understanding the core objective helps define scope",
  "input_type": "text",
  "options": [],
  "section": "goal"
}
```

Fields:
- `text` (required): The question to ask
- `context` (optional): Helpful context or explanation
- `input_type` (required): One of `"text"`, `"select"`, `"multi_select"`, `"confirm"`, `"editor"`
- `options` (required for select types): Array of option objects
- `section` (optional): Which draft section this relates to

#### Select Options

For `select` and `multi_select` input types, provide options:

```json
{
  "type": "question",
  "text": "What type of project is this?",
  "context": null,
  "input_type": "select",
  "options": [
    {
      "value": "web",
      "label": "Web Application",
      "description": "Frontend or full-stack web app"
    },
    {
      "value": "cli",
      "label": "CLI Tool",
      "description": "Command-line application"
    },
    {
      "value": "library",
      "label": "Library",
      "description": "Reusable code library"
    }
  ],
  "section": "context"
}
```

Option fields:
- `value` (required): Value stored when selected
- `label` (required): Display text for the option
- `description` (optional): Additional help text

#### Draft Update

Update a section of the draft prompt. Use this to incrementally build the prompt as information is gathered.

```json
{
  "type": "draft_update",
  "section": "requirements",
  "content": "- Must support authentication via JWT\n- Must handle rate limiting",
  "append": true
}
```

Fields:
- `section` (required): One of `"title"`, `"goal"`, `"context"`, `"requirements"`, `"constraints"`, `"files_to_modify"`, `"acceptance_criteria"`, `"notes"`
- `content` (required): Content to add/replace
- `append` (optional, default: false): If true, appends to existing content; if false, replaces

#### Thinking

Show the user that processing is happening. Use sparingly.

```json
{
  "type": "thinking",
  "message": "Analyzing the technical requirements..."
}
```

#### Clarification

Ask for clarification on a vague or ambiguous answer.

```json
{
  "type": "clarification",
  "text": "When you say 'fast', do you mean response time or throughput?",
  "original_answer": "It needs to be fast",
  "input_type": "select",
  "options": [
    {"value": "response_time", "label": "Response Time", "description": "Low latency for individual requests"},
    {"value": "throughput", "label": "Throughput", "description": "High volume of requests per second"},
    {"value": "both", "label": "Both", "description": "Optimize for both metrics"}
  ]
}
```

#### Draft Complete

Signal that the interview is complete and the draft is ready.

```json
{
  "type": "draft_complete",
  "summary": "Created comprehensive prompt covering authentication feature with JWT, role-based access, and session management"
}
```

#### Error

Report an error condition.

```json
{
  "type": "error",
  "message": "Unable to parse the previous response. Please try again."
}
```

### User Responses

The TUI sends user responses back to the agent in this format:

```json
{
  "answer": "Build a REST API for user management",
  "feedback": null
}
```

Answer types vary based on input type:
- `text` / `editor`: String value
- `select`: String value (the selected option's `value`)
- `multi_select`: Array of strings (selected option values)
- `confirm`: Boolean (`true` or `false`)

The optional `feedback` field allows users to add context to their answer.

### Interview Strategy Guidelines

When implementing interview support, follow these guidelines:

1. **Start Broad**: Begin with the overall goal before diving into details
2. **Probe Vague Answers**: If the user gives vague answers like "make it work", ask specifically what "working" means
3. **Confirm Understanding**: Periodically summarize what you've learned with draft updates
4. **Think About Edge Cases**: Ask about error scenarios, invalid inputs, edge cases
5. **Consider Dependencies**: Ask about existing code, libraries, or constraints
6. **Define Success**: Ensure clear acceptance criteria exist
7. **One Question at a Time**: Ask only one question per message
8. **Update Draft Incrementally**: Don't wait until the end to write the draft

### Draft Sections

The generated prompt.md should include these sections:

| Section | Description |
|---------|-------------|
| `title` | Clear, concise name for the task (e.g., "Add User Authentication") |
| `goal` | Primary objective in 1-3 sentences |
| `context` | Background, motivation, and relevant existing state |
| `requirements` | Specific, actionable requirements (use bullet points) |
| `constraints` | Technical limitations, time constraints, must-not-do items |
| `files_to_modify` | Specific files that will need changes (if known) |
| `acceptance_criteria` | Measurable criteria for completion (checkboxes) |
| `notes` | Additional context, references, or considerations |

### Project Context

The system prompt includes project context that agents should use to ask informed questions:

- Project type (Rust, Node, Python, Go, Unknown)
- Languages and frameworks detected
- Key files (entry points, config files)
- Directory structure summary
- Project name and description (if available from manifest files)

### Example Conversation Flow

```
System: [System prompt with protocol instructions and project context]

Agent: {"type": "question", "text": "What feature or task would you like to implement?", "input_type": "text", "options": [], "section": "goal"}

User: {"answer": "Add user authentication", "feedback": null}

Agent: {"type": "draft_update", "section": "title", "content": "Add User Authentication", "append": false}
Agent: {"type": "question", "text": "What authentication method should be used?", "input_type": "select", "options": [{"value": "jwt", "label": "JWT Tokens", "description": "Stateless authentication with JSON Web Tokens"}, {"value": "session", "label": "Session-based", "description": "Server-side sessions with cookies"}, {"value": "oauth", "label": "OAuth 2.0", "description": "Third-party authentication providers"}], "section": "requirements"}

User: {"answer": "jwt", "feedback": null}

Agent: {"type": "draft_update", "section": "requirements", "content": "- Implement JWT-based authentication", "append": false}
Agent: {"type": "question", "text": "Should the JWT include refresh token support?", "input_type": "confirm", "options": [], "section": "requirements"}

...

Agent: {"type": "draft_complete", "summary": "Created prompt for JWT authentication with refresh tokens, password reset, and role-based access control"}
```

### Implementation Notes

- Always output valid JSON without markdown code blocks
- The TUI attempts to extract JSON from agent responses even if wrapped in other text
- Send draft updates frequently to give users visual feedback
- Use appropriate input types to reduce user typing effort
- Include helpful context with questions when useful
