# Configuration Schema

This document provides a complete reference for all configuration options.

## Configuration Files

| File | Location | Scope |
|------|----------|-------|
| Global | `~/.config/codeloops/config.toml` | All projects |
| Project | `<working-dir>/codeloops.toml` | Single project |

## Precedence

Settings are resolved in order (highest priority first):

1. CLI flags
2. Project configuration
3. Global configuration
4. Built-in defaults

## Global Configuration

**File**: `~/.config/codeloops/config.toml`

### Complete Schema

```toml
# Default settings applied to all sessions
[defaults]
# Default agent for both actor and critic roles
# Values: "claude", "opencode", "cursor"
# Default: "claude"
agent = "claude"

# Default model for both roles (optional)
# Value depends on agent (e.g., "sonnet", "opus", "gpt-4o")
# Default: none (uses agent default)
model = "sonnet"

# Actor-specific overrides (optional section)
[defaults.actor]
# Agent for actor role (overrides defaults.agent for actor)
agent = "opencode"

# Model for actor role (overrides defaults.model for actor)
model = "gpt-4o"

# Critic-specific overrides (optional section)
[defaults.critic]
# Agent for critic role (overrides defaults.agent for critic)
agent = "claude"

# Model for critic role (overrides defaults.model for critic)
model = "opus"
```

### Section Reference

#### `[defaults]`

Base defaults for all sessions.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `agent` | string | `"claude"` | Default agent for both roles |
| `model` | string | none | Default model for both roles |

#### `[defaults.actor]`

Override defaults for the actor role.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `agent` | string | inherit | Agent for actor |
| `model` | string | inherit | Model for actor |

#### `[defaults.critic]`

Override defaults for the critic role.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `agent` | string | inherit | Agent for critic |
| `model` | string | inherit | Model for critic |

### Example Configurations

**Minimal (use defaults):**
```toml
[defaults]
agent = "claude"
```

**With model:**
```toml
[defaults]
agent = "claude"
model = "sonnet"
```

**Different agents per role:**
```toml
[defaults]
agent = "claude"

[defaults.actor]
agent = "opencode"
model = "gpt-4o"

[defaults.critic]
model = "opus"
```

## Project Configuration

**File**: `<working-dir>/codeloops.toml`

### Complete Schema

```toml
# Default agent for this project
# Values: "claude", "opencode", "cursor"
agent = "claude"

# Default model for this project (optional)
model = "sonnet"

# Actor-specific settings (optional section)
[actor]
agent = "opencode"
model = "gpt-4o"

# Critic-specific settings (optional section)
[critic]
agent = "claude"
model = "opus"
```

### Field Reference

#### Root Level

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `agent` | string | inherit | Default agent for this project |
| `model` | string | inherit | Default model for this project |

#### `[actor]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `agent` | string | inherit | Agent for actor |
| `model` | string | inherit | Model for actor |

#### `[critic]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `agent` | string | inherit | Agent for critic |
| `model` | string | inherit | Model for critic |

### Example Configurations

**Simple project config:**
```toml
agent = "claude"
```

**Mixed agents:**
```toml
[actor]
agent = "opencode"
model = "gpt-4o-mini"

[critic]
agent = "claude"
model = "sonnet"
```

## Valid Values

### Agent Values

| Value | Agent |
|-------|-------|
| `"claude"` | Claude Code |
| `"opencode"` | OpenCode |
| `"cursor"` | Cursor |

### Model Values

Model values depend on the agent:

**Claude Code:**
- `"sonnet"` - Claude Sonnet
- `"opus"` - Claude Opus
- `"haiku"` - Claude Haiku

**OpenCode:**
- `"gpt-4o"` - GPT-4o
- `"gpt-4o-mini"` - GPT-4o Mini
- Other OpenAI models

**Cursor:**
- Uses Cursor's configured model

## Resolution Examples

### Example 1: No config files

Running `codeloops` with no config:

| Setting | Value | Source |
|---------|-------|--------|
| Actor agent | claude | default |
| Critic agent | claude | default |
| Actor model | (none) | default |
| Critic model | (none) | default |

### Example 2: Global config only

`~/.config/codeloops/config.toml`:
```toml
[defaults]
agent = "opencode"
model = "gpt-4o"
```

Running `codeloops`:

| Setting | Value | Source |
|---------|-------|--------|
| Actor agent | opencode | global |
| Critic agent | opencode | global |
| Actor model | gpt-4o | global |
| Critic model | gpt-4o | global |

### Example 3: Global + Project config

`~/.config/codeloops/config.toml`:
```toml
[defaults]
agent = "opencode"
```

`codeloops.toml`:
```toml
agent = "claude"
```

Running `codeloops`:

| Setting | Value | Source |
|---------|-------|--------|
| Actor agent | claude | project |
| Critic agent | claude | project |

### Example 4: CLI overrides all

`~/.config/codeloops/config.toml`:
```toml
[defaults]
agent = "opencode"
```

`codeloops.toml`:
```toml
agent = "claude"
```

Running `codeloops --agent cursor`:

| Setting | Value | Source |
|---------|-------|--------|
| Actor agent | cursor | CLI |
| Critic agent | cursor | CLI |

### Example 5: Per-role settings

`~/.config/codeloops/config.toml`:
```toml
[defaults]
agent = "claude"
model = "sonnet"

[defaults.actor]
agent = "opencode"
model = "gpt-4o"
```

Running `codeloops`:

| Setting | Value | Source |
|---------|-------|--------|
| Actor agent | opencode | global.actor |
| Critic agent | claude | global.defaults |
| Actor model | gpt-4o | global.actor |
| Critic model | sonnet | global.defaults |

## Environment Variables

| Variable | Description |
|----------|-------------|
| `CODELOOPS_UI_DIR` | Override UI assets directory |
| `NO_COLOR` | Disable colored output when set |

## Creating Configuration

### Using init

```bash
codeloops init
```

Interactive prompts create `~/.config/codeloops/config.toml`.

### Manual creation

```bash
mkdir -p ~/.config/codeloops
cat > ~/.config/codeloops/config.toml << 'EOF'
[defaults]
agent = "claude"
EOF
```

## Validating Configuration

Use `--dry-run` to see resolved configuration:

```bash
codeloops --dry-run
```

Output shows the effective settings without executing.
