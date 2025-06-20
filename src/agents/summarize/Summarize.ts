import { getInstance as getLogger } from '../../logger.ts';
import { v4 as uuid } from 'uuid';
import { DagNode, KnowledgeGraphManager, SummaryNode } from '../../engine/KnowledgeGraph.ts';
import { SummarizerAgent } from './SummarizerAgent.ts';

/**
 * SummarizationAgent provides an interface to the TypeScript-based summarization agent.
 * It handles summarization logic for the knowledge graph using the new SummarizerAgent.
 */
export class SummarizationAgent {
  private readonly summarizerAgent: SummarizerAgent;

  // Number of nodes after which to trigger summarization
  private static SUMMARIZATION_THRESHOLD = 20;

  /**
   * Creates a new SummarizationAgent.
   * @param knowledgeGraph The knowledge graph manager instance
   */
  constructor(private readonly knowledgeGraph: KnowledgeGraphManager) {
    this.summarizerAgent = new SummarizerAgent({ logger: getLogger() });
  }

  /**
   * Summarizes a segment of nodes from the knowledge graph.
   * @param nodes Array of DagNode objects to summarize
   * @returns A promise that resolves to an object containing the summary text and any error
   */
  async summarize(nodes: DagNode[]): Promise<{ summary: string; error?: string }> {
    try {
      // Log input for debugging
      getLogger().info({ nodeCount: nodes.length }, 'Summarization agent input');

      // Use the TypeScript SummarizerAgent
      const result = await this.summarizerAgent.summarize(nodes);

      getLogger().info({ summaryLength: result.summary.length }, 'Summarization agent output');

      return result;
    } catch (error) {
      const err = error as Error;
      getLogger().error({ error: err }, 'Summarization agent error');
      return {
        summary: '',
        error: `Summarization failed: ${err.message}`,
      };
    }
  }

  /**
   * Checks if summarization is needed and triggers it if necessary.
   * This should be called after adding new nodes to the graph.
   */
  async checkAndTriggerSummarization({
    project,
    projectContext,
  }: {
    project: string;
    projectContext: string;
  }): Promise<void> {
    const nodes = await this.knowledgeGraph.resume({
      project,
      limit: SummarizationAgent.SUMMARIZATION_THRESHOLD,
    });

    const lastSummaryIndex = nodes.findIndex((node) => node.role === 'summary');
    const nodesToSummarize = nodes.slice(lastSummaryIndex + 1);

    // Only summarize branches that have enough nodes
    if (nodesToSummarize.length >= SummarizationAgent.SUMMARIZATION_THRESHOLD) {
      await this.createSummary({
        project,
        projectContext,
        nodes: nodesToSummarize,
      });
    }
  }

  /**
   * Creates a summary for a segment of nodes.
   * @param nodes Nodes to summarize
   * @throws Error if summarization fails
   */
  async createSummary({
    nodes,
    projectContext,
    project,
  }: {
    nodes: DagNode[];
    projectContext: string;
    project: string;
  }): Promise<SummaryNode> {
    if (!nodes || nodes.length === 0) {
      throw new Error('Cannot create summary: No nodes provided');
    }

    getLogger().info(`[createSummary] Creating summary for ${nodes.length} nodes`);

    const result = await this.summarize(nodes);

    // Check for errors in the summarization result
    if (result.error) {
      getLogger().error({ error: result.error }, `[createSummary] Summarization agent error:`);
      throw new Error(`Summarization failed: ${result.error}`);
    }

    // Validate the summary content
    if (!result.summary || result.summary.trim() === '') {
      getLogger().error(`[createSummary] Summarization agent returned empty summary`);
      throw new Error('Summarization failed: Empty summary returned');
    }

    // Create a summary node
    const summaryNode: SummaryNode = {
      id: uuid(),
      project,
      thought: result.summary,
      role: 'summary',
      parents: [nodes[nodes.length - 1].id], // Link to the newest node in the segment
      children: [],
      createdAt: '', // Will be set by appendEntity
      projectContext,
      summarizedSegment: nodes.map((node) => node.id),
      tags: ['summary'],
      artifacts: [],
    };

    getLogger().info(`[createSummary] Created summary node with ID ${summaryNode.id}`);

    // Persist the summary node
    await this.knowledgeGraph.appendEntity(summaryNode);

    // Update the last node to include the summary node in its children
    const lastNode = nodes[nodes.length - 1];
    if (lastNode && !lastNode.children.includes(summaryNode.id)) {
      lastNode.children.push(summaryNode.id);
      await this.knowledgeGraph.appendEntity(lastNode);
    }

    return summaryNode;
  }
}
