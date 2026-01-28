# Introduction

Codeloops is a command-line tool that orchestrates an **actor-critic feedback loop** for AI coding agents. It runs your preferred coding agent (Claude Code, OpenCode, or Cursor) as the "actor" to execute tasks, then uses another agent as the "critic" to evaluate the work and provide feedback. This loop continues until the task is complete or a maximum iteration count is reached.

## The Problem

AI coding agents are powerful, but they lack a built-in mechanism for self-correction. When an agent makes a mistake or produces incomplete work, it has no way to know unless a human reviews the output. This leads to:

- Incomplete implementations that miss edge cases
- Code that doesn't fully address the original requirements
- Bugs that could have been caught with a second look

## How Codeloops Solves It

Codeloops introduces a feedback loop where:

1. The **actor** (a coding agent) executes your task
2. Git captures the changes made
3. The **critic** (another agent instance) evaluates the output against your original prompt
4. If the work is incomplete, the critic provides feedback and the actor tries again
5. The loop continues until the critic approves or max iterations are reached

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│   prompt.md ──▶ Actor ──▶ Git Diff ──▶ Critic ──▶ Done?    │
│                   ▲                        │                │
│                   │         feedback       │                │
│                   └────────────────────────┘                │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Key Benefits

- **Self-correcting loop**: Mistakes are caught and fixed automatically
- **Improved task completion**: Multiple iterations ensure requirements are fully met
- **Full observability**: Every iteration is logged to a session file for review
- **Agent flexibility**: Mix and match agents for actor and critic roles
- **Simple interface**: Just write a `prompt.md` file and run `codeloops`

## Who Should Use This

Codeloops is for developers who:

- Use AI coding agents for development tasks
- Want higher quality output from their AI tools
- Need a way to review and analyze AI agent behavior
- Want to experiment with different agent configurations

## Next Steps

Ready to get started? Head to the [Installation](./getting-started/installation.md) guide to set up codeloops, then follow the [Quickstart](./getting-started/quickstart.md) to run your first session.
