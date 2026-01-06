# Actor-Critic System Design for OpenCode

**Date:** 2026-01-06
**Status:** Approved

## Overview

An actor-critic feedback system for OpenCode that provides continuous, iterative feedback to coding agents. Inspired by temporal difference learning, the system uses a critic agent to analyze each action the actor takes, providing structured feedback that gets injected back into the actor's context before its next move.

## Core Concepts

- **Actor**: The primary coding agent that generates code/plans
- **Critic**: A subagent with fresh context that evaluates actions for correctness, best practices, and alignment
- **Knowledge Graph**: Memory store that persists feedback with role attribution
- **Key Insight**: Inverts typical orchestration - user interfaces with Actor, Actor receives feedback from Critic

## Architecture

### Components

1. **Actor Agent** (`agents/actor.md`)
   - Primary agent that replaces/extends "build"
   - Knows it operates within an actor-critic feedback loop
   - Instructed to incorporate critic feedback before each action

2. **Critic Agent** (`agents/critic.md`)
   - Subagent with fresh context per invocation
   - Analyzes what the actor just did
   - Has read-only tool access to dig deeper (read files, grep, etc.)
   - Outputs structured JSON feedback

3. **Protocol Skill** (`skills/actor-critic-protocol/SKILL.md`)
   - Detailed guidance on handling feedback
   - Loaded on-demand when actor needs protocol details

4. **Plugin Enhancement** (`plugin/index.ts`)
   - Intercepts `tool.execute.after` events
   - Spawns critic synchronously (blocking)
   - Injects formatted feedback via `client.session.prompt({ noReply: true })`
   - Stores feedback in MemoryStore with `role: "critic"` property

### Data Flow

```
Actor takes action → Plugin intercepts → Critic analyzes (blocking)
                                              ↓
                         Structured JSON feedback
                                              ↓
                    Plugin formats & injects into actor session
                    Plugin stores in memory (role: "critic")
                                              ↓
                         Actor sees feedback, takes next action
```

## Schema Changes

### MemoryEntry Enhancement

Add `role` property to distinguish entry sources:

```typescript
type MemoryEntry = {
  id: string;
  project: string;
  content: string;
  tags?: string[];
  createdAt: string;
  sessionId?: string;
  source?: string;
  role?: "actor" | "critic";  // NEW: extensible to "human" later
}
```

## Critic Output Format

Critic outputs structured JSON:

```json
{
  "verdict": "proceed" | "revise" | "stop",
  "confidence": 0.0-1.0,
  "issues": ["list of specific problems found"],
  "suggestions": ["actionable improvements"],
  "context": "any relevant context the actor should know",
  "reasoning": "brief explanation of your analysis"
}
```

### Verdicts

- **proceed**: Action looks good, actor can continue
- **revise**: Issues found, actor should address before continuing
- **stop**: Critical problem, actor should stop and reassess approach

## Context Building

What the critic receives:

```typescript
type CriticContext = {
  action: {
    tool: string;
    args: Record<string, unknown>;
    result: string;
  };
  diff?: string;
  conversationContext: string;
  project: {
    name: string;
    workdir: string;
  };
}
```

## Configuration

In `~/.config/codeloops/config.json`:

```json
{
  "critic": {
    "enabled": true,
    "model": null
  }
}
```

- `enabled`: Toggle critic system on/off
- `model`: Model for critic (null = use actor's model)

## Installation

The install script creates symlinks from codeloops repo to OpenCode config:

```
~/.config/opencode/agent/actor.md -> <repo>/agents/actor.md
~/.config/opencode/agent/critic.md -> <repo>/agents/critic.md
~/.config/opencode/skill/actor-critic-protocol -> <repo>/skills/actor-critic-protocol
```

Benefits:
- Edit files in repo, changes apply immediately
- Version controlled with codeloops
- Updates via `git pull`, no reinstall needed

## File Structure

```
codeloops/
├── agents/
│   ├── actor.md              # Actor agent definition
│   └── critic.md             # Critic agent definition
├── skills/
│   └── actor-critic-protocol/
│       └── SKILL.md          # Detailed protocol skill
├── plugin/
│   └── index.ts              # Enhanced with critic invocation
├── lib/
│   └── types.ts              # MemoryEntry with role property
├── scripts/
│   ├── install-plugin.ts     # Extended to symlink agents/skills
│   └── build-plugin.ts
```

## Implementation Tasks

1. Create `agents/actor.md` - Actor agent definition
2. Create `agents/critic.md` - Critic agent definition
3. Create `skills/actor-critic-protocol/SKILL.md` - Protocol skill
4. Modify `lib/types.ts` - Add `role` to MemoryEntry schema
5. Modify `plugin/index.ts` - Add critic invocation and feedback injection
6. Modify `scripts/install-plugin.ts` - Add symlink creation for agents/skills
7. Update documentation

## Future Considerations

- Specialized sub-critics (duplicate code, component interface, best practices)
- `role: "human"` for human-in-the-loop clarification
- Configurable trigger points (before writes vs after all actions)
- Async mode option for speed over strict ordering
