---
name: actor-critic-protocol
description: Detailed protocol for operating within the actor-critic feedback loop. Load when you need guidance on handling critic feedback or understanding the system.
---

# Actor-Critic Protocol

## Overview

You are part of a feedback-driven development system inspired by temporal difference learning. The goal is to produce higher quality code through iterative refinement.

## The Loop

1. **You act** - Make a code change, run a command, create a file
2. **Critic analyzes** - A separate agent reviews your action with fresh context
3. **Feedback injected** - You receive structured feedback before your next move
4. **You incorporate** - Address issues or proceed based on verdict

## Handling Verdicts

### On "proceed"
- Continue with your plan
- Note any suggestions for future consideration
- Build on the confidence signal

### On "revise"
- **Stop and address issues before continuing**
- Review each issue in the feedback
- Make corrections, then proceed
- The critic will re-evaluate your revision

### On "stop"
- **Halt current approach entirely**
- Re-read the original goal
- Consider alternative approaches
- Discuss with the user if unclear on how to proceed

## Using Memory

Query past critic feedback to identify patterns:
- `memory_recall({ tags: ["critic"], limit: 10 })` - Recent feedback
- `memory_recall({ query: "revise", tags: ["critic"] })` - Past revisions

This helps you learn from previous corrections and avoid repeating mistakes.

## When to Disagree

If you believe critic feedback is incorrect:
1. Explain your reasoning clearly
2. Cite specific evidence (code, docs, conventions)
3. Proceed with your approach if confident
4. The next critic pass will re-evaluate

## Key Principles

- **Feedback is collaboration** - The critic helps, not hinders
- **Address issues promptly** - Don't accumulate technical debt
- **Learn from patterns** - Use memory to avoid repeat mistakes
- **Stay aligned** - The system keeps you on track with project goals
