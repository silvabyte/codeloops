# Background & Inspiration

Codeloops has gone through multiple iterations since its inception. This page covers the project's origins, the challenges faced, and the key insight that shaped its current architecture.

## Origins (March/April 2025)

Codeloops started in March/April 2025, inspired by the actor-critic model from neuroscience. The idea came from reading Max Bennett's *A Brief History of Intelligence*, which introduced concepts about temporal difference learning and how the brain uses separate systems for action and evaluation.

The problem was clear: coding agents produce what could be called "code slop"—unstructured, error-prone output. Memory gaps cause agents to forget APIs they've just created. Context lapses lead them to ignore prior configurations. For serious software development, the results were unreliable.

The original architecture had three components:

1. **The Actor**: The coding agent generates code/plans
2. **The Critic**: An LLM evaluates outputs for accuracy and best practices
3. **The Knowledge Graph**: Stores feedback and context for future iterations

Early testing used Anthropic's Haiku 3.5 as the critic, paired with Augment's agent. The approach showed promise—a week of testing cost under $0.40.

## The Architectural Challenge

But where should this feedback loop live? The initial assumption was that the loop needed to be embedded within the coding agent layer itself—perhaps as a plugin, extension, or modification to the agent's core behavior.

This approach proved frustrating. Coding agents have their own architectures, extension points, and constraints. Trying to inject a feedback loop at that level meant fighting against each agent's design rather than working with it. The project stalled.

## Enter Ralph Wiggum (2026)

The [Ralph Wiggum loop](https://ghuntley.com/ralph/) changed everything.

Ralph Wiggum is a technique popularized by Geoffrey Huntley that implements a simple but powerful idea: wrap your coding agent in an external while loop that keeps running until a task is complete. The agent doesn't need to know it's in a loop—it just executes, and the external harness decides whether to continue.

The brilliance of Ralph Wiggum was showing that the feedback loop belongs **outside** the agent, not inside it. The agent is a black box that takes a prompt and produces output. The orchestration layer—the loop, the evaluation, the decision to continue—lives above it.

This insight unlocked codeloops. In April 2026, the project was revived with a new architecture. Instead of modifying agents, codeloops wraps them. Any agent with a CLI becomes a valid actor or critic. The loop is agent-agnostic because it operates at a higher architectural layer.

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

Thanks to Max Bennett for *A Brief History of Intelligence*, which introduced the actor-critic concepts from neuroscience that inspired this project.

Thanks to Geoffrey Huntley for the Ralph Wiggum technique. It provided the key architectural insight that made codeloops viable: the feedback loop belongs in the orchestration layer, not the agent layer.

## Further Reading

- [Improving Coding Agents: An Early Look at Codeloops](https://bytes.silvabyte.com/improving-coding-agents-an-early-look-at-codeloops-for-building-more-reliable-software/) - Original blog post from May 2025
- [Ralph Wiggum - Original Post](https://ghuntley.com/ralph/)
- [The Actor-Critic Loop](../architecture/actor-critic.md) - How codeloops implements its feedback loop
- [Architecture Overview](../architecture/overview.md) - System design and component structure
