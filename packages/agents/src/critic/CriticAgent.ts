import { BaseAgent, type AgentConfig } from '../BaseAgent.js';
import {
  createModel,
  getModelReference,
  getModelConfigFromPath,
  getConfig,
} from '@codeloops/config';
import type { Logger } from 'pino';
import type { DagNode } from '@codeloops/core';
import { z } from 'zod';

/**
 * Output schema for CriticAgent responses
 * Based on the critic agent's evaluation requirements
 */
export const CriticOutputSchema = z.object({
  verdict: z.enum(['approved', 'needs_revision', 'reject']),
  verdictReason: z.string().optional(),
  recommendations: z.array(z.string()).optional(),
});

/**
 * Type for CriticAgent responses
 */
export type CriticResponse = z.infer<typeof CriticOutputSchema>;

/**
 * AgentDeps interface for dependency injection
 */
interface AgentDeps {
  logger: Logger;
}

/**
 * CriticAgent - TypeScript implementation of the CodeLoops Quality Critic
 *
 * This agent evaluates actor thoughts and provides iterative feedback to ensure
 * code quality and clarity. It replaces the Python-based critic agent while
 * maintaining backward compatibility with the existing system.
 */
export class CriticAgent extends BaseAgent {
  private static readonly CRITIC_INSTRUCTIONS = `You are the Quality Critic in the CodeLoops system, responsible for evaluating and improving the quality of code generation.

## System Architecture
You are part of the CodeLoops system with these key components:
- Actor: Generates new thoughts and code
- Critic (you): Evaluates actor thoughts and provides iterative feedback

## Actor Requirements
Every \`thought\` **must** satisfy **all** of the following rules:

1. **Non‑Empty & Descriptive** – A clear statement of completed **or proposed** work; boiler‑plate or empty thoughts are invalid.
2. **Intent + Action + Rationale** – Explain *what* was/will be done, *why* it is/was done, and the intended outcome.
3. **Specific & Unambiguous** – Use concrete nouns/verbs; eliminate vague terms ("stuff", "things", "various"). No ambiguity.
4. **Comprehensive & Focused** – Provide enough detail to stand on its own while covering coherent ideas. Brevity is **not** required if it sacrifices clarity.
5. **Professional Tone** – Avoid slang, profanity, meme language, and excessive emojis.
6. **No TODO / FIXME** – The thought cannot contain TODOs, placeholders, or apologies. If more work is needed, describe next steps explicitly.
7. **Sensitive Content Handling** – If PII, credentials, or other sensitive data appear, explicitly prompt for security implications and request user guidance rather than exposing the data.
8. **Duplication Awareness** – The thought should indicate that existing code/logic has been reviewed to avoid reinventing the wheel or duplicating solutions already in the project.
9. **Code Mentions** – When referencing code, describe it conceptually (e.g. "Added async retry wrapper for HTTP calls") and flag any problematic patterns such as @ts‑expect‑error usage.

## Review Process
1. Set \`verdict\` to **approved**, **needs_revision**, or **reject**.
2. If not approved, include a short \`verdictReason\`.
3. Provide any helpful \`recommendations\` as an array of specific improvement suggestions.

## Verdict Definitions
- **approved**: Meets clarity and quality expectations.
- **needs_revision**: Requires improvements (explain why).
- **reject**: Fundamentally flawed or exceeds revision attempts (2).`;

  constructor(deps: AgentDeps) {
    const config = getConfig();
    const modelRef =
      getModelReference('agents.critic.model') || (config.get('default_model') as string);
    const modelSettings = getModelConfigFromPath('agents.critic');

    const agentConfig: AgentConfig = {
      name: 'critic',
      instructions: CriticAgent.CRITIC_INSTRUCTIONS,
      outputSchema: CriticOutputSchema,
      model: createModel(modelRef),
      markdown: true,
      temperature: modelSettings.temperature,
      maxTokens: modelSettings.maxTokens,
    };

    super(agentConfig, deps);

    deps.logger.info(
      {
        modelRef,
        temperature: modelSettings.temperature,
        maxTokens: modelSettings.maxTokens,
        enabled: modelSettings.enabled,
      },
      'CriticAgent initialized',
    );
  }

  /**
   * Review an actor node and provide feedback
   * @param actorNode The actor node to review
   * @returns Promise<CriticResponse> with verdict and optional feedback
   */
  async reviewActorNode(actorNode: DagNode): Promise<CriticResponse> {
    // Validate that the node is an actor node
    if (actorNode.role !== 'actor') {
      throw new Error(`Cannot review non-actor node. Node role: ${actorNode.role}`);
    }

    // Build the review prompt
    const prompt = this.buildReviewPrompt(actorNode);

    // Get the critic's evaluation
    const response = await this.send<CriticResponse>(prompt);

    return response;
  }

  /**
   * Build a review prompt for the critic to evaluate an actor node
   * @param actorNode The actor node to review
   * @returns The formatted prompt string
   */
  private buildReviewPrompt(actorNode: DagNode): string {
    const nodeContext = {
      id: actorNode.id,
      thought: actorNode.thought,
      tags: actorNode.tags,
      artifacts: actorNode.artifacts,
      createdAt: actorNode.createdAt,
    };

    return `Please review the following actor thought and provide your evaluation:

## Actor Node Information
${JSON.stringify(nodeContext, null, 2)}

## Your Task
Evaluate the actor's thought against all the requirements listed in your instructions. Consider:
1. Is the thought clear and descriptive?
2. Does it explain intent, action, and rationale?
3. Is it specific and unambiguous?
4. Does it maintain professional tone?
5. Are there any TODOs or placeholders?
6. Does it handle sensitive content appropriately?
7. Does it show awareness of existing code/patterns?
8. If code is mentioned, is it described conceptually?

Provide your verdict (approved/needs_revision/reject) and explain your reasoning if not approved.`;
  }

  /**
   * Helper method to check if the agent is enabled
   * @returns boolean indicating if the critic agent is enabled
   */
  isEnabled(): boolean {
    const modelSettings = getModelConfigFromPath('agents.critic');
    return modelSettings.enabled;
  }
}

// Factory function for convenience
export function createCriticAgent(deps: AgentDeps): CriticAgent {
  return new CriticAgent(deps);
}
