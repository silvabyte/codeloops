# Agents

Codeloops works with multiple AI coding agents. This guide covers supported agents and how to choose between them.

## Supported Agents

| Agent | CLI Value | Binary | Description |
|-------|-----------|--------|-------------|
| Claude Code | `claude` | `claude` | Anthropic's Claude-powered coding agent |
| OpenCode | `opencode` | `opencode` | Multi-model coding agent |
| Cursor | `cursor-agent` | `cursor-agent` | Cursor IDE's agent CLI |

## Agent Details

### Claude Code

Claude Code is Anthropic's official coding agent, powered by Claude models.

**Binary**: `claude`

**Strengths**:
- Excellent reasoning and planning
- Strong understanding of complex codebases
- Good at following detailed instructions
- Reliable critic evaluation

**Installation**: Visit [claude.ai/code](https://claude.ai/code)

**Verify installation**:
```bash
which claude
claude --version
```

### OpenCode

OpenCode is a multi-model coding agent that supports various LLM backends.

**Binary**: `opencode`

**Strengths**:
- Supports multiple models (GPT-4, etc.)
- Fast execution for straightforward tasks
- Good for rapid iteration

**Installation**: Visit [opencode.ai/docs](https://opencode.ai/docs/#install)

**Verify installation**:
```bash
which opencode
opencode --version
```

### Cursor

Cursor's CLI provides access to its coding capabilities outside the IDE.

**Binary**: `cursor-agent` (or `agent`)

**Strengths**:
- Integrates with Cursor IDE workflow
- Familiar for Cursor users

**Installation**: Visit [cursor.com/cli](https://cursor.com/cli)

**Verify installation**:
```bash
which cursor-agent  # or 'agent'
```

## Choosing Agents

### Same Agent for Both Roles

The simplest configuration uses one agent for both actor and critic:

```bash
codeloops --agent claude
```

This is recommended when:
- You're starting out and want simplicity
- The agent performs well for your use case
- You want consistent behavior

### Different Agents for Actor and Critic

You can use different agents for each role:

```bash
codeloops --actor-agent opencode --critic-agent claude
```

This is useful when:
- You want fast iteration with a thorough reviewer
- Different agents have different strengths
- You're experimenting with agent combinations

### Configuration Recommendations

**For complex tasks** (refactoring, architecture changes):
```bash
codeloops --agent claude
```
Use Claude for both roles when the task requires deep understanding.

**For fast iteration** (simple fixes, small changes):
```bash
codeloops --actor-agent opencode --critic-agent claude
```
Use a fast actor with a thorough critic.

**For Cursor users**:
```bash
codeloops --agent cursor-agent
```
Use Cursor if you're already in the Cursor ecosystem.

## Model Selection

Some agents support model selection:

```bash
# Specify model for both roles
codeloops --agent claude --model opus

# Specify model per role (via config file)
```

Model support depends on the agent:
- Claude Code: Supports Claude models (sonnet, opus, etc.)
- OpenCode: Supports multiple backends (gpt-4o, etc.)
- Cursor: Uses Cursor's configured model

## Agent Availability

Codeloops checks agent availability before running. If an agent isn't found:

```
Error: Agent 'claude' not found in PATH
```

To fix this:
1. Install the agent
2. Ensure the binary is in your PATH
3. Verify with `which <agent-name>`

### Checking All Agents

Check which agents are available:

```bash
which claude opencode cursor-agent
```

## Agent Configuration

### In Global Config

```toml
# ~/.config/codeloops/config.toml

[defaults]
agent = "claude"

[defaults.actor]
agent = "opencode"

[defaults.critic]
agent = "claude"
```

### In Project Config

```toml
# codeloops.toml

[actor]
agent = "opencode"
model = "gpt-4o"

[critic]
agent = "claude"
model = "sonnet"
```

## How Agents Are Invoked

When codeloops runs an agent, it:

1. Spawns the agent binary as a subprocess
2. Passes the prompt via stdin or command-line arguments
3. Sets the working directory
4. Captures stdout, stderr, and exit code
5. Waits for completion

The agent runs with full access to the filesystem within the working directory, allowing it to read and modify files as needed.

## Agent Output

Agent output is captured and passed to the critic. The output typically includes:
- What the agent did
- Files modified
- Any errors encountered

This output, combined with the git diff, forms the basis for critic evaluation.
