import { BaseAgent } from '../BaseAgent.js';
import { Logger } from 'pino';
import { DagNode } from '@codeloops/core';
import { z } from 'zod';
/**
 * Output schema for CriticAgent responses
 * Based on the critic agent's evaluation requirements
 */
export declare const CriticOutputSchema: z.ZodObject<
  {
    verdict: z.ZodEnum<['approved', 'needs_revision', 'reject']>;
    verdictReason: z.ZodOptional<z.ZodString>;
    recommendations: z.ZodOptional<z.ZodArray<z.ZodString, 'many'>>;
  },
  'strip',
  z.ZodTypeAny,
  {
    verdict: 'approved' | 'needs_revision' | 'reject';
    verdictReason?: string | undefined;
    recommendations?: string[] | undefined;
  },
  {
    verdict: 'approved' | 'needs_revision' | 'reject';
    verdictReason?: string | undefined;
    recommendations?: string[] | undefined;
  }
>;
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
export declare class CriticAgent extends BaseAgent {
  private static readonly CRITIC_INSTRUCTIONS;
  constructor(deps: AgentDeps);
  /**
   * Review an actor node and provide feedback
   * @param actorNode The actor node to review
   * @returns Promise<CriticResponse> with verdict and optional feedback
   */
  reviewActorNode(actorNode: DagNode): Promise<CriticResponse>;
  /**
   * Build a review prompt for the critic to evaluate an actor node
   * @param actorNode The actor node to review
   * @returns The formatted prompt string
   */
  private buildReviewPrompt;
  /**
   * Helper method to check if the agent is enabled
   * @returns boolean indicating if the critic agent is enabled
   */
  isEnabled(): boolean;
}
export declare function createCriticAgent(deps: AgentDeps): CriticAgent;
export {};
//# sourceMappingURL=CriticAgent.d.ts.map
