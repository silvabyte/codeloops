# Actor-Critic System

This directory contains legacy stubs. The actual implementation is now in:

## Agent Definitions
- `/agents/actor.md` - Primary actor agent
- `/agents/critic.md` - Critic subagent

## Skills
- `/skills/actor-critic-protocol/SKILL.md` - Detailed protocol guidance

## Plugin Implementation
- `/plugin/index.ts` - Contains the `tool.execute.after` hook that:
  1. Intercepts tool executions
  2. Spawns a critic agent with fresh context
  3. Injects feedback into the actor's session
  4. Stores feedback in memory with `role: "critic"`

## Design Documentation
- `/docs/plans/2026-01-06-actor-critic-system-design.md` - Full system design

## Key Concept

This system inverts typical agent orchestration. Instead of the user interfacing
with an orchestrator that delegates to workers, the user interfaces with an Actor
that solicits feedback from a Critic. The Critic provides guidance to keep the
Actor aligned with project goals and best practices.

## Installation

Run `bun run scripts/install-plugin.ts` to install:
- Plugin to `~/.config/opencode/plugin/codeloops.js`
- Agents to `~/.config/opencode/agent/`
- Skills to `~/.config/opencode/skill/`

## Configuration

Configure in `~/.config/codeloops/config.json`:

```json
{
  "critic": {
    "enabled": true,
    "model": "anthropic/claude-haiku-4-20250514"
  }
}
```

If `model` is not set, the critic uses the same model as the actor.
