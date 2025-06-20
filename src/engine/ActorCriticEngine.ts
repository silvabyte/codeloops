import { Actor } from '../agents/Actor.ts';
import { Critic } from '../agents/critic/Critic.ts';
import { SummarizationAgent } from '../agents/summarize/Summarize.ts';
import { KnowledgeGraphManager, type DagNode, FILE_REF } from './KnowledgeGraph.ts';
import { z } from 'zod';
import { getModelConfigFromPath } from '../config/models.ts';
// -----------------------------------------------------------------------------
// Actor–Critic engine ----------------------------------------------------------
// -----------------------------------------------------------------------------

const THOUGHT_DESCRIPTION = `
Add a new thought node to the knowledge‑graph.

• Use for any creative / planning step, requirement capture, task break‑down, etc.
• **Always include at least one 'tag'** so future searches can find this node
  – e.g. requirement, task, risk, design, definition.
• **If your thought references a file you just created or modified**, list it in
  the 'artifacts' array so the graph stores a durable link.
• Think of 'tags' + 'artifacts' as the breadcrumbs that future you (or another
  agent) will follow to avoid duplicate work or forgotten decisions.
`.trim();

export const ActorThinkSchema = {
  thought: z.string().describe(THOUGHT_DESCRIPTION),

  projectContext: z
    .string()
    .describe(
      'Full path to the currently open directory in the code editor. Used to infer the project name from the last item in the path.',
    ),

  tags: z
    .array(z.string())
    .min(1, 'Add at least one semantic tag – requirement, task, risk, design …')
    .describe('Semantic categories used for later search and deduping.'),

  /** Actual files produced or updated by this step.*/
  artifacts: z
    .array(FILE_REF)
    .describe(
      'Declare the file set this thought will affect so the critic can ' +
        'verify coverage before code is written.' +
        'graph has durable pointers to the exact revision.',
    ),
};

export const ActorThinkSchemaZodObject = z.object(ActorThinkSchema);
export type ActorThinkInput = z.infer<typeof ActorThinkSchemaZodObject>;

export class ActorCriticEngine {
  constructor(
    private readonly kg: KnowledgeGraphManager,
    private readonly critic: Critic,
    private readonly actor: Actor,
    private readonly summarizationAgent: SummarizationAgent,
  ) {}

  // Use the centralized extractProjectName function from utils
  /* --------------------------- public API --------------------------- */
  /**
   * Adds a new thought node to the knowledge graph and automatically triggers
   * critic review
   *
   * @param input The actor thought input
   * @returns Either the actor node (if no review was triggered) or the critic node (if review was triggered)
   */
  async actorThink(input: ActorThinkInput & { project: string; diff?: string }): Promise<DagNode> {
    // Actor.think will handle project switching based on projectContext
    const { node } = await this.actor.think(input);

    const criticNode = await this.criticReview({
      actorNodeId: node.id,
      projectContext: input.projectContext,
      project: input.project,
    });

    return criticNode;
  }

  /**
   * Manually triggers a critic review for a specific actor node.
   *
   * NOTE: In most cases, you don't need to call this directly as actorThink
   * automatically triggers critic reviews when appropriate.
   *
   * This method is primarily useful for:
   * - Manual intervention in the workflow
   * - Forcing a review of a specific previous node
   * - Debugging or testing purposes
   *
   * @param actorNodeId The ID of the actor node to review
   * @param projectContext The project context for the review
   * @returns The critic node
   */
  async criticReview({
    actorNodeId,
    projectContext,
    project,
  }: {
    actorNodeId: string;
    projectContext: string;
    project: string;
  }): Promise<DagNode> {
    const criticNode = await this.critic.review({ actorNodeId, projectContext, project });

    // Trigger summarization check after adding a critic node
    const summarizerConfig = getModelConfigFromPath('agents.summarizer');
    if (summarizerConfig.enabled) {
      await this.summarizationAgent.checkAndTriggerSummarization({
        project,
        projectContext,
      });
    } else {
      // Optionally log or handle the disabled state
      // console.info('Summarizer agent is disabled; skipping summarization check.');
    }

    return criticNode;
  }
}
