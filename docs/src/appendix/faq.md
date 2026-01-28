# FAQ

Frequently asked questions about codeloops.

## General

### What is codeloops?

Codeloops is a command-line tool that orchestrates an actor-critic feedback loop for AI coding agents. It runs a coding agent to execute tasks, evaluates the results with another agent instance, and loops until the task is complete.

### Why use codeloops instead of running an agent directly?

AI coding agents lack built-in self-correction. They may produce incomplete work, miss edge cases, or make mistakes without knowing. Codeloops adds a feedback loop where a critic evaluates the work and provides guidance for improvement, leading to higher quality results.

### What agents are supported?

Currently supported:
- Claude Code (`claude`)
- OpenCode (`opencode`)
- Cursor (`cursor`)

### Can I use my own LLM or a custom agent?

Yes, by implementing the `Agent` trait. See [Adding New Agents](../contributing/adding-agents.md) for a step-by-step guide.

### Is codeloops free?

Codeloops itself is open source and free. However, the agents it uses (Claude, OpenCode, Cursor) may have their own pricing. You'll need accounts with those services.

## Usage

### Why does the loop keep running?

The loop continues when the critic returns `CONTINUE`, meaning the task isn't complete. Possible reasons:

1. **Requirements not fully met**: Check the critic's feedback to see what's missing
2. **Vague prompt**: Add explicit acceptance criteria
3. **Error in implementation**: Actor may be making mistakes

Review the session to see the feedback:
```bash
codeloops sessions show
```

### How do I stop a running session?

Press `Ctrl+C`. The session will be recorded with outcome `interrupted`.

### How do I reduce iterations?

1. Write more specific prompts with clear acceptance criteria
2. Provide context (file paths, function names)
3. Break complex tasks into smaller ones
4. Set a limit: `--max-iterations 5`

### Why is the critic rejecting my changes?

Common reasons:
- Requirements in the prompt aren't fully implemented
- Edge cases not handled
- Tests failing (if mentioned in prompt)
- Code quality issues

Check the feedback in the session:
```bash
codeloops sessions show
```

### Can I run multiple sessions simultaneously?

Yes, in different terminals or directories. Each session writes to its own file.

## Configuration

### Where is the configuration file?

- Global: `~/.config/codeloops/config.toml`
- Project: `./codeloops.toml` (in working directory)

### How do I see what configuration is being used?

Use dry run:
```bash
codeloops --dry-run
```

### How do I use different agents for actor and critic?

```bash
codeloops --actor-agent opencode --critic-agent claude
```

Or in configuration:
```toml
[actor]
agent = "opencode"

[critic]
agent = "claude"
```

## Sessions

### Where are my sessions stored?

`~/.local/share/codeloops/sessions/`

### Can I delete old sessions?

Yes, they're just files:
```bash
rm ~/.local/share/codeloops/sessions/session-id.jsonl
```

### How do I export session data?

Sessions are JSONL files. Parse them with jq, Python, or any JSON tool:
```bash
cat ~/.local/share/codeloops/sessions/session.jsonl | jq
```

### Why is my session file missing session_end?

The session was either:
- Still in progress when you checked
- Interrupted (Ctrl+C, crash)
- The process was killed

## Troubleshooting

### Agent not found in PATH

```
Error: Agent 'claude' not found in PATH
```

**Solution**: Install the agent and ensure its binary is in your PATH.

```bash
# Check if agent is installed
which claude

# Verify it works
claude --version
```

### No prompt provided

```
Error: No prompt provided
```

**Solution**: Create a `prompt.md` file or use `--prompt`:

```bash
echo "Fix the bug" > prompt.md
codeloops
```

Or:
```bash
codeloops --prompt "Fix the bug"
```

### Not a git repository

```
Error: Not a git repository
```

**Solution**: Initialize git in your working directory:

```bash
git init
```

### Permission denied

```
Error: Permission denied
```

**Solution**: Check file and directory permissions:
```bash
ls -la ~/.local/share/codeloops/
```

### Session directory doesn't exist

The session directory is created automatically. If it fails, create it manually:
```bash
mkdir -p ~/.local/share/codeloops/sessions
```

### UI won't start

**Port in use**:
```bash
codeloops ui --api-port 4000 --ui-port 4001
```

**UI directory not found**: Build the UI:
```bash
cd ui && bun install && bun run build
```

### Slow performance

Possible causes:
- Agent response time (depends on the service)
- Large diffs (many files changed)
- Complex prompts

Tips:
- Use faster agents for simple tasks
- Break large tasks into smaller ones

## Development

### How do I contribute?

See [Development Setup](../contributing/development.md) for getting started.

### How do I report bugs?

Open an issue on GitHub with:
- Steps to reproduce
- Expected vs actual behavior
- Session file (if applicable)
- System information

### How do I request features?

Open an issue on GitHub describing:
- The use case
- Proposed solution
- Alternatives considered

## Miscellaneous

### Does codeloops modify my code?

No, codeloops doesn't modify code directly. The agents do. Codeloops orchestrates the loop and captures the results.

### Does codeloops send my code anywhere?

Codeloops itself doesn't send code anywhere. The agents you use may send code to their services (Claude API, OpenAI API, etc.) according to their terms of service.

### Can I use codeloops offline?

Codeloops itself runs locally, but the agents typically require internet access to their respective APIs.

### How is this different from GitHub Copilot?

Copilot is an inline code completion tool. Codeloops is a task execution framework with feedback loops. They solve different problems and can be used together.

### Does codeloops work with any language?

Yes, codeloops is language-agnostic. It works with whatever languages your chosen agent supports.

### Can I use codeloops in CI/CD?

Yes. Use `--json-output` for machine-readable output and exit codes for status:
```bash
codeloops --max-iterations 3 --json-output > result.json
```

### Where can I get help?

- Documentation: This site
- Issues: [GitHub Issues](https://github.com/matsilva/codeloops/issues)
- Discussions: [GitHub Discussions](https://github.com/matsilva/codeloops/discussions)
