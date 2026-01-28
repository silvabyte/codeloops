# Configuration

Codeloops supports configuration at two levels: global (user-wide) and project-level. This guide explains how to configure both.

## Configuration Precedence

Settings are resolved in this order (highest to lowest priority):

1. CLI flags (e.g., `--agent claude`)
2. Project configuration (`codeloops.toml` in working directory)
3. Global configuration (`~/.config/codeloops/config.toml`)
4. Built-in defaults

For example, if you set `agent = "opencode"` in your global config but run `codeloops --agent claude`, Claude will be used.

## Global Configuration

Location: `~/.config/codeloops/config.toml`

The global configuration sets your user-wide defaults. Create it with `codeloops init` or manually.

### Schema

```toml
[defaults]
agent = "claude"           # Default agent for both roles
model = "sonnet"           # Default model (optional)

[defaults.actor]
agent = "opencode"         # Override agent for actor role
model = "gpt-4o"           # Override model for actor

[defaults.critic]
agent = "claude"           # Override agent for critic role
model = "opus"             # Override model for critic
```

### Fields

| Section | Field | Type | Description |
|---------|-------|------|-------------|
| `[defaults]` | `agent` | String | Default agent: `claude`, `opencode`, or `cursor` |
| `[defaults]` | `model` | String | Default model name (optional) |
| `[defaults.actor]` | `agent` | String | Actor-specific agent override |
| `[defaults.actor]` | `model` | String | Actor-specific model override |
| `[defaults.critic]` | `agent` | String | Critic-specific agent override |
| `[defaults.critic]` | `model` | String | Critic-specific model override |

### Example Configurations

**Simple setup (same agent for everything):**

```toml
[defaults]
agent = "claude"
```

**Mixed agents (fast actor, thorough critic):**

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

Location: `codeloops.toml` in the project root (working directory)

Project configuration overrides global settings for a specific project. This is useful when different projects need different agent configurations.

### Schema

```toml
agent = "claude"           # Default agent for this project
model = "sonnet"           # Default model (optional)

[actor]
agent = "opencode"         # Actor agent for this project
model = "gpt-4o"           # Actor model for this project

[critic]
agent = "claude"           # Critic agent for this project
model = "opus"             # Critic model for this project
```

### Fields

| Section | Field | Type | Description |
|---------|-------|------|-------------|
| (root) | `agent` | String | Default agent for this project |
| (root) | `model` | String | Default model for this project |
| `[actor]` | `agent` | String | Actor agent override |
| `[actor]` | `model` | String | Actor model override |
| `[critic]` | `agent` | String | Critic agent override |
| `[critic]` | `model` | String | Critic model override |

### Example Configurations

**Simple project config:**

```toml
agent = "claude"
```

**Project using OpenCode for fast iteration:**

```toml
[actor]
agent = "opencode"
model = "gpt-4o-mini"

[critic]
agent = "claude"
model = "sonnet"
```

## Resolution Examples

### Example 1: No configuration

With no config files and running `codeloops`:
- Actor: Claude (default)
- Critic: Claude (default)

### Example 2: Global config only

`~/.config/codeloops/config.toml`:
```toml
[defaults]
agent = "opencode"
```

Running `codeloops`:
- Actor: OpenCode (from global)
- Critic: OpenCode (from global)

### Example 3: Global + project config

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
- Actor: Claude (project overrides global)
- Critic: Claude (project overrides global)

### Example 4: CLI overrides everything

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
- Actor: Cursor (CLI overrides all)
- Critic: Cursor (CLI overrides all)

### Example 5: Per-role configuration

`~/.config/codeloops/config.toml`:
```toml
[defaults]
agent = "claude"

[defaults.actor]
agent = "opencode"
```

Running `codeloops`:
- Actor: OpenCode (from defaults.actor)
- Critic: Claude (from defaults)

### Example 6: CLI role-specific override

`~/.config/codeloops/config.toml`:
```toml
[defaults]
agent = "opencode"
```

Running `codeloops --critic-agent claude`:
- Actor: OpenCode (from global)
- Critic: Claude (CLI override for critic only)

## Creating Configuration

### Using init

The simplest way to create global configuration:

```bash
codeloops init
```

This runs an interactive setup and creates the config file.

### Manual Creation

Create the file manually:

```bash
mkdir -p ~/.config/codeloops
cat > ~/.config/codeloops/config.toml << 'EOF'
[defaults]
agent = "claude"
EOF
```

For project config, create `codeloops.toml` in your project root.

## Validating Configuration

Use `--dry-run` to see the resolved configuration without executing:

```bash
codeloops --dry-run
```

Output shows the effective agent, model, and other settings that would be used.
