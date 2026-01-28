# Background & Inspiration

Codeloops started in April 2026 as an attempt to bring systematic feedback loops to AI coding agents. This page covers the project's origins and the key insight that shaped its architecture.

## The Early Days

The initial vision for codeloops was clear: coding agents needed a way to verify their own work. But where should this feedback loop live? The original assumption was that the loop needed to be embedded within the coding agent layer itself—perhaps as a plugin, extension, or modification to the agent's core behavior.

This approach proved frustrating. Coding agents have their own architectures, extension points, and constraints. Trying to inject a feedback loop at that level meant fighting against each agent's design rather than working with it.

## Enter Ralph Wiggum

The [Ralph Wiggum loop](https://ghuntley.com/ralph/) changed everything.

Ralph Wiggum is a technique popularized by Geoffrey Huntley that implements a simple but powerful idea: wrap your coding agent in an external while loop that keeps running until a task is complete. The agent doesn't need to know it's in a loop—it just executes, and the external harness decides whether to continue.

The brilliance of Ralph Wiggum was showing that the feedback loop belongs **outside** the agent, not inside it. The agent is a black box that takes a prompt and produces output. The orchestration layer—the loop, the evaluation, the decision to continue—lives above it.

This insight unlocked codeloops. Instead of modifying agents, codeloops wraps them. Any agent with a CLI becomes a valid actor or critic. The loop is agent-agnostic because it operates at a higher architectural layer.

## How Codeloops Differs

While Ralph Wiggum uses naive persistence (loop until a completion promise appears), codeloops adds structure:

| Aspect | Ralph Wiggum | Codeloops |
|--------|--------------|-----------|
| **Evaluation** | Self-evaluation (agent decides when done) | Explicit critic agent reviews the actor's work |
| **Feedback source** | Context window + file state | Git diff + stdout/stderr |
| **Roles** | Single agent | Separate actor and critic (can be different agents) |
| **Decision protocol** | Completion promise string | Structured DONE/CONTINUE/ERROR response |

The actor-critic separation means the agent doing the work isn't the same one judging it. This provides a second perspective and catches issues the actor might miss.

## Acknowledgments

Thanks to Geoffrey Huntley for the Ralph Wiggum technique. It provided the key architectural insight that made codeloops possible: the feedback loop belongs in the orchestration layer, not the agent layer.

## Further Reading

- [Ralph Wiggum - Original Post](https://ghuntley.com/ralph/)
- [The Actor-Critic Loop](../architecture/actor-critic.md) - How codeloops implements its feedback loop
- [Architecture Overview](../architecture/overview.md) - System design and component structure
