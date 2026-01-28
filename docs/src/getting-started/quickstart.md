# Quickstart

This guide walks you through running codeloops for the first time.

## Step 1: Initialize Configuration

If you haven't already, run the interactive setup:

```bash
codeloops init
```

Select your preferred agent (Claude, OpenCode, or Cursor) when prompted.

## Step 2: Create a Prompt File

Navigate to a git repository where you want to make changes:

```bash
cd /path/to/your/project
```

Create a `prompt.md` file describing your task:

```markdown
Fix the typo in the greeting message. The word "Helo" should be "Hello".
```

Keep your first prompt simple and specific. Complex tasks work better once you're familiar with how the loop operates.

## Step 3: Run Codeloops

Execute the loop:

```bash
codeloops
```

Codeloops will:
1. Read `prompt.md` from the current directory
2. Start the actor agent with your prompt
3. Capture the git diff after the actor completes
4. Run the critic agent to evaluate the changes
5. Either finish (if the critic approves) or loop back with feedback

## Step 4: Understand the Output

During execution, you'll see:

```
[codeloops] Starting actor-critic loop
[codeloops] Prompt: Fix the typo in the greeting message...
[codeloops] Working directory: /path/to/your/project
[codeloops] Actor: Claude Code | Critic: Claude Code

[iteration 1]
[actor] Running Claude Code...
[actor] Completed in 12.3s (exit code: 0)
[git] 1 file changed, 1 insertion(+), 1 deletion(-)
[critic] Evaluating changes...
[critic] Decision: DONE
[critic] Summary: The typo has been fixed. "Helo" is now "Hello".

[codeloops] Session complete: success (1 iteration)
```

### Decision Types

The critic returns one of three decisions:

| Decision | Meaning |
|----------|---------|
| **DONE** | Task is complete, loop ends |
| **CONTINUE** | More work needed, actor runs again with feedback |
| **ERROR** | Something went wrong, actor gets recovery instructions |

## Step 5: View the Result

Check what changed:

```bash
git diff
```

Or use the sessions command:

```bash
codeloops sessions show
```

This opens an interactive viewer to browse the session details.

## Alternative: Inline Prompt

Instead of a file, you can pass the prompt directly:

```bash
codeloops --prompt "Add a --verbose flag to the CLI"
```

## Alternative: Specify Agent

Override the configured agent:

```bash
codeloops --agent opencode
```

Or use different agents for actor and critic:

```bash
codeloops --actor-agent opencode --critic-agent claude
```

## Common First-Run Issues

### Agent not found

```
Error: Agent 'claude' not found in PATH
```

Install the agent or ensure its binary is in your PATH.

### No prompt provided

```
Error: No prompt provided. Create a prompt.md file or use --prompt
```

Create a `prompt.md` file or pass `--prompt "your task"`.

### Not a git repository

```
Error: Not a git repository
```

Initialize git or navigate to an existing repository:

```bash
git init
```

## Next Steps

- Read [Your First Session](./first-session.md) for a detailed walkthrough
- See [CLI Reference](../user-guide/cli-reference.md) for all available options
- Learn about [Configuration](../user-guide/configuration.md) to customize behavior
