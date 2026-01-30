# CLI Reference

Complete reference for all codeloops commands and options.

## Overview

```
codeloops [OPTIONS] [COMMAND]
```

When no command is specified, codeloops runs the actor-critic loop (equivalent to `codeloops run`).

## Commands

| Command | Description |
|---------|-------------|
| `run` | Run the actor-critic loop (default) |
| `sessions` | Browse and inspect sessions |
| `ui` | Start the web UI |
| `init` | Interactive configuration setup |
| `help` | Print help information |

## Run Command

Execute the actor-critic loop. This is the default command when none is specified.

```bash
codeloops run [OPTIONS]
codeloops [OPTIONS]  # Same as above
```

### Prompt Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `-p, --prompt <PROMPT>` | String | - | Task prompt (inline) |
| `--prompt-file <FILE>` | Path | `prompt.md` | Path to prompt file |

If neither `--prompt` nor `--prompt-file` is provided, codeloops looks for `prompt.md` in the working directory.

### Directory Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `-d, --working-dir <DIR>` | Path | Current directory | Working directory for the session |

### Agent Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `-a, --agent <AGENT>` | Enum | `claude` | Agent for both actor and critic |
| `--actor-agent <AGENT>` | Enum | - | Agent specifically for actor role |
| `--critic-agent <AGENT>` | Enum | - | Agent specifically for critic role |
| `-m, --model <MODEL>` | String | - | Model to use (if agent supports it) |

Agent values: `claude`, `opencode`, `cursor`

### Loop Control

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `-n, --max-iterations <N>` | Integer | Unlimited | Maximum loop iterations |

### Output Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `--log-format <FORMAT>` | Enum | `pretty` | Output format |
| `--log-file <PATH>` | Path | - | Write structured logs to file |
| `--json-output` | Flag | - | Output final result as JSON |
| `--no-color` | Flag | - | Disable colored output |

Log format values: `pretty`, `json`, `compact`

### Other Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `--dry-run` | Flag | - | Show configuration without executing |

### Examples

```bash
# Run with default prompt.md
codeloops

# Run with inline prompt
codeloops --prompt "Fix the bug in main.rs"

# Run with custom prompt file
codeloops --prompt-file tasks/feature.md

# Run with specific agent
codeloops --agent opencode

# Run with mixed agents
codeloops --actor-agent opencode --critic-agent claude

# Limit iterations
codeloops --max-iterations 5

# Output as JSON
codeloops --json-output

# Dry run to verify configuration
codeloops --dry-run
```

## Sessions Command

Browse and inspect recorded sessions.

```bash
codeloops sessions <SUBCOMMAND>
```

### Subcommands

#### list

List all sessions with optional filtering.

```bash
codeloops sessions list [OPTIONS]
```

| Option | Type | Description |
|--------|------|-------------|
| `--outcome <OUTCOME>` | String | Filter by outcome: `success`, `failed`, `interrupted`, `max_iterations_reached` |
| `--after <DATE>` | Date | Show sessions after date (YYYY-MM-DD) |
| `--before <DATE>` | Date | Show sessions before date (YYYY-MM-DD) |
| `--search <TEXT>` | String | Search in prompt text |
| `--project <NAME>` | String | Filter by project name |

Examples:

```bash
# List all sessions
codeloops sessions list

# Filter by outcome
codeloops sessions list --outcome success

# Filter by date range
codeloops sessions list --after 2025-01-01 --before 2025-01-31

# Search prompts
codeloops sessions list --search "authentication"

# Filter by project
codeloops sessions list --project myapp
```

#### show

Show detailed session information.

```bash
codeloops sessions show [ID]
```

If no ID is provided, opens an interactive picker to select a session.

Examples:

```bash
# Interactive picker
codeloops sessions show

# Show specific session
codeloops sessions show 2025-01-27T15-30-45Z_a3f2c1
```

#### diff

Show the cumulative git diff from a session.

```bash
codeloops sessions diff [ID]
```

If no ID is provided, opens an interactive picker.

Examples:

```bash
# Interactive picker
codeloops sessions diff

# Show diff for specific session
codeloops sessions diff 2025-01-27T15-30-45Z_a3f2c1
```

#### stats

Show aggregate statistics across all sessions.

```bash
codeloops sessions stats
```

Output includes:
- Total sessions
- Success rate
- Average iterations
- Average duration
- Sessions by project

## UI Command

Start the web UI for visual session browsing.

```bash
codeloops ui [OPTIONS]
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `--dev` | Flag | - | Development mode with hot reloading |
| `--api-port <PORT>` | Integer | 3100 | API server port |
| `--ui-port <PORT>` | Integer | 3101 | UI server port |

Examples:

```bash
# Start UI with defaults
codeloops ui

# Custom ports
codeloops ui --api-port 4000 --ui-port 4001

# Development mode
codeloops ui --dev
```

The UI opens automatically in your default browser.

## Init Command

Interactive first-time setup.

```bash
codeloops init
```

This command:
1. Prompts for your preferred default agent
2. Creates `~/.config/codeloops/config.toml`
3. Displays the generated configuration

Run this after installation to set up your defaults.

## Global Options

These options work with any command:

| Option | Description |
|--------|-------------|
| `-h, --help` | Print help information |
| `-V, --version` | Print version information |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Max iterations reached |
| 2 | Failed (error during execution) |
| 130 | User interrupted (Ctrl+C) |

## Environment Variables

| Variable | Description |
|----------|-------------|
| `CODELOOPS_UI_DIR` | Override the UI directory location |
| `NO_COLOR` | Disable colored output when set |
