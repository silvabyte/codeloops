import { BaseAgent, type AgentConfig } from '../BaseAgent.ts';
import { createModel, getModelReference, getModelConfigFromPath } from '../../config/models.ts';
import { getConfig } from '../../config/index.ts';
import { Logger } from 'pino';
import { DagNode } from '../../engine/KnowledgeGraph.ts';
import { z } from 'zod';

/**
 * Output schema for SummarizerAgent responses
 * Based on the summarizer agent's condensation requirements
 */
export const SummaryOutputSchema = z.object({
  summary: z.string(),
  keyPoints: z.array(z.string()),
  actionItems: z.array(z.string()).optional(),
});

/**
 * Type for SummarizerAgent responses
 */
export type SummaryResponse = z.infer<typeof SummaryOutputSchema>;

/**
 * AgentDeps interface for dependency injection
 */
interface AgentDeps {
  logger: Logger;
}

/**
 * SummarizerAgent - TypeScript implementation of the CodeLoops Summarizer
 *
 * This agent analyzes sequences of nodes and provides structured summaries
 * to help track progress, key decisions, and important artifacts across
 * actor-critic loops.
 */
export class SummarizerAgent extends BaseAgent {
  private static readonly SUMMARIZER_INSTRUCTIONS = `You are the Summarizer in the CodeLoops system, responsible for analyzing sequences of thoughts and providing structured summaries.

## System Architecture
You are part of the CodeLoops system with these key components:
- KnowledgeGraphManager: Manages graph of interconnected thought nodes
- Actor: Generates new thoughts and code implementations  
- Critic: Evaluates actor thoughts and provides feedback
- Summarizer (you): Analyzes sequences of nodes and creates summaries
- ActorCriticEngine: Orchestrates the feedback loop between components

## DagNode Schema
Each node in the knowledge graph contains:
- id: Unique identifier for the node
- project: Project name/identifier
- thought: The main content/reasoning of the node
- role: Type of node ("actor", "critic", "summary")
- verdict: For critic nodes ("approved", "needs_revision", "reject")
- verdictReason: Optional explanation for critic decisions
- target: For critic nodes, the ID of the node being critiqued
- parents: Array of parent node IDs (nodes this depends on)
- children: Array of child node IDs (nodes that depend on this)
- createdAt: ISO timestamp of creation
- projectContext: Full path to the project directory
- tags: Array of semantic tags for categorization
- artifacts: Array of file references with durable links

## Summarization Process
When analyzing a sequence of nodes, follow this process:

1. **Analyze the sequence** for key decisions, code artifacts, and conceptual developments
2. **Identify main themes** and the progression of work across the actor-critic loops
3. **Produce a structured summary** with:
   - A concise overview (1-2 paragraphs) of what was accomplished
   - Key points highlighting major decisions and implementations
   - Optional action items for future work or improvements
4. **Include references** to important artifacts and definitions mentioned
5. **Focus on high-level insights** rather than low-level implementation details

## Guidelines
- Keep summaries brief yet comprehensive
- Highlight important artifacts and their purposes
- Focus on the "why" and "what" rather than implementation details
- Identify patterns and themes across multiple nodes
- Note significant critic feedback and how it influenced development
- Include key decisions and their rationale

Your response should provide clear insight into the progress and evolution of the codebase.`;

  constructor(deps: AgentDeps) {
    const config = getConfig();
    const modelRef =
      getModelReference('agents.summarizer.model') || (config.get('default_model') as string);
    const modelSettings = getModelConfigFromPath('agents.summarizer');

    const agentConfig: AgentConfig = {
      name: 'summarizer',
      instructions: SummarizerAgent.SUMMARIZER_INSTRUCTIONS,
      outputSchema: SummaryOutputSchema,
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
      'SummarizerAgent initialized',
    );
  }

  /**
   * Summarize a sequence of nodes from the knowledge graph
   * @param nodes Array of DagNode objects to summarize
   * @returns Promise<SummaryResponse> with structured summary
   */
  async summarizeNodes(nodes: DagNode[]): Promise<SummaryResponse> {
    if (!nodes || nodes.length === 0) {
      throw new Error('Cannot summarize: No nodes provided');
    }

    // Build the summarization prompt
    const prompt = this.buildSummarizationPrompt(nodes);

    // Get the summarizer's analysis
    const response = await this.send<SummaryResponse>(prompt);

    return response;
  }

  /**
   * Legacy method for backward compatibility with existing SummarizationAgent API
   * @param nodes Array of DagNode objects to summarize
   * @returns Promise with summary string and optional error
   */
  async summarize(nodes: DagNode[]): Promise<{ summary: string; error?: string }> {
    try {
      const response = await this.summarizeNodes(nodes);
      return {
        summary: `${response.summary}\n\nKey Points:\n${response.keyPoints.map((p) => `• ${p}`).join('\n')}${
          response.actionItems?.length
            ? `\n\nAction Items:\n${response.actionItems.map((a) => `• ${a}`).join('\n')}`
            : ''
        }`,
      };
    } catch (error) {
      return {
        summary: '',
        error: error instanceof Error ? error.message : 'Unknown summarization error',
      };
    }
  }

  /**
   * Build a summarization prompt for the given nodes
   * @param nodes The nodes to summarize
   * @returns The formatted prompt string
   */
  private buildSummarizationPrompt(nodes: DagNode[]): string {
    // Create a simplified representation of the nodes for analysis
    const nodesSummary = nodes.map((node, index) => ({
      position: index + 1,
      id: node.id,
      role: node.role,
      thought: node.thought,
      tags: node.tags,
      artifacts: node.artifacts?.map((a) => a.name) || [],
      verdict: node.verdict,
      verdictReason: node.verdictReason,
      createdAt: node.createdAt,
    }));

    return `Please analyze and summarize the following sequence of ${nodes.length} nodes from the CodeLoops knowledge graph:

## Node Sequence
${JSON.stringify(nodesSummary, null, 2)}

## Your Task
Analyze this sequence and provide a structured summary that captures:
1. The main progression of work and key decisions made
2. Important artifacts created or modified
3. Critical feedback from critics and how it influenced development
4. Overall themes and patterns in the actor-critic loops
5. Any significant insights or learnings

Focus on the high-level narrative of what was accomplished rather than implementation details.

Provide your analysis as a structured summary with key points and optional action items.`;
  }

  /**
   * Helper method to check if the agent is enabled
   * @returns boolean indicating if the summarizer agent is enabled
   */
  isEnabled(): boolean {
    const modelSettings = getModelConfigFromPath('agents.summarizer');
    return modelSettings.enabled;
  }
}

// Factory function for convenience
export function createSummarizerAgent(deps: AgentDeps): SummarizerAgent {
  return new SummarizerAgent(deps);
}
