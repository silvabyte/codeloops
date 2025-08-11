import type { McpServer, ToolCallback } from '@modelcontextprotocol/sdk/server/mcp.js';
import { z } from 'zod';
import {
  KnowledgeGraphManager,
  type CodeLoopsLogger,
  getInstance as getLogger,
  setGlobalLogger,
} from '@codeloops/core';
import { extractProjectName, getGitDiff } from '@codeloops/core/utils';
import { Critic, Actor, ActorCriticEngine, ActorThinkSchema } from '@codeloops/agents';

// Shared dependencies interface
interface ToolDependencies {
  logger: CodeLoopsLogger;
  kg: KnowledgeGraphManager;
  engine: ActorCriticEngine;
  runOnce: (project: string) => void;
}

const handleToolError = (err: Error): ReturnType<ToolCallback> => {
  return {
    content: [
      {
        type: 'text',
        text: JSON.stringify(
          {
            error: err?.message,
            stack: err?.stack,
            name: err?.name,
          },
          null,
          2,
        ),
      },
    ],
  };
};

// Utility function to load project
const loadProjectOrThrow = async ({
  logger,
  args,
  onProjectLoad,
}: {
  logger: CodeLoopsLogger;
  args: { projectContext: string };
  onProjectLoad: (project: string) => void;
}) => {
  const projectName = extractProjectName(args.projectContext);
  if (!projectName) {
    logger.error({ projectContext: args.projectContext }, 'Invalid projectContext');
    throw new Error(`Invalid projectContext: ${args.projectContext}`);
  }
  onProjectLoad(projectName);
  return projectName;
};

export const createDependencies = async (): Promise<ToolDependencies> => {
  const logger = getLogger();

  // Create KnowledgeGraphManager
  const kg = new KnowledgeGraphManager(logger);
  await kg.init();

  // Create dependencies
  const critic = new Critic(kg);
  const actor = new Actor(kg);

  // Create ActorCriticEngine
  const engine = new ActorCriticEngine(kg, critic, actor);

  const runOnce = (project: string) => {
    const child = logger.child({ project });
    setGlobalLogger(child);
  };

  return { logger, kg, engine, runOnce };
};

export const registerTools = ({ server }: { server: McpServer }) => {
  // This will be called from each transport, so we need to initialize dependencies here
  let deps: ToolDependencies | null = null;

  const getDeps = async (): Promise<ToolDependencies> => {
    if (!deps) {
      deps = await createDependencies();
    }
    return deps;
  };

  const ACTOR_THINK_DESCRIPTION = `
  Add a new thought node to the CodeLoops knowledge graph to plan, execute, or document coding tasks.
  
  **Purpose**: This is the **primary tool** for interacting with the actor-critic system. It records your work, triggers critic reviews when needed, and guides you through iterative development. **You must call 'actor_think' iteratively** after every significant action to ensure your work is reviewed and refined.
  
  **Instructions**:
  1. **Call 'actor_think' for all actions**:
     - Planning, requirement capture, task breakdown, or coding steps.
     - Use the 'projectContext' property to specify the full path to the currently open directory.
  2. **Always include at least one semantic tag** (e.g., 'requirement', 'task', 'file-modification', 'task-complete') to enable searchability and trigger appropriate reviews.
  3. **Iterative Workflow**:
     - File modifications or task completions automatically trigger critic reviews.
     - Use the critic's feedback (in 'criticNode') to refine your next thought.
  4. **Tags and artifacts are critical for tracking decisions and avoiding duplicate work**.
  
  **Example Workflow**:
  - Step 1: Call 'actor_think' with thought: "Create main.ts with initial setup", projectContext: "/path/to/project", artifacts: ['src/main.ts'], tags: ['file-modification'].
      - Response: Includes feedback from the critic
  - Step 2:  Make any necessary changes and call 'actor_think' again with the updated thought.
  - Repeat until the all work is completed.
  
  **Note**: Critic reviews are automatically triggered by 'actor_think' - no manual intervention needed.
  `;

  /**
   * actor_think - Add a new thought node to the knowledge graph.
   */
  server.tool('actor_think', ACTOR_THINK_DESCRIPTION, ActorThinkSchema, async (args) => {
    try {
      const { logger, engine, runOnce } = await getDeps();
      const projectName = await loadProjectOrThrow({ logger, args, onProjectLoad: runOnce });

      // Auto-generate comprehensive git diff
      const diff = await getGitDiff(logger);

      const node = await engine.actorThink({
        ...args,
        project: projectName,
        diff,
      });
      return {
        content: [
          {
            type: 'text',
            text: JSON.stringify(node, null, 2),
          },
        ],
      };
    } catch (err) {
      return handleToolError(err as Error);
    }
  });

  server.tool(
    'get_node',
    'Get a specific node by ID',
    {
      id: z.string().describe('ID of the node to retrieve.'),
    },
    async (a) => {
      try {
        const { kg } = await getDeps();
        const node = await kg.getNode(a.id);
        return {
          content: [
            {
              type: 'text',
              text: JSON.stringify(node, null, 2),
            },
          ],
        };
      } catch (err) {
        return handleToolError(err as Error);
      }
    },
  );

  server.tool(
    'resume',
    'Pick up where you left off by fetching the most recent nodes from the knowledge graph for this project. Use limit to control the number of nodes returned. Increase it if you need more context.',
    {
      projectContext: z.string().describe('Full path to the project directory.'),
      limit: z
        .number()
        .optional()
        .describe('Limit the number of nodes returned. Increase it if you need more context.'),
      includeDiffs: z
        .enum(['all', 'latest', 'none'])
        .optional()
        .default('none')
        .describe(
          'Control diff inclusion: "all" includes all diffs, "latest" includes only the most recent diff, "none" excludes all diffs. Defaults to "none" to avoid context overflow.',
        ),
    },
    async (a) => {
      try {
        const { logger, kg, runOnce } = await getDeps();
        const projectName = await loadProjectOrThrow({ logger, args: a, onProjectLoad: runOnce });
        const text = await kg.resume({
          project: projectName,
          limit: a.limit,
          includeDiffs: a.includeDiffs || 'latest',
        });
        return {
          content: [{ type: 'text', text: JSON.stringify(text, null, 2) }],
        };
      } catch (err) {
        return handleToolError(err as Error);
      }
    },
  );

  /** export – dump the current graph */
  server.tool(
    'export',
    'dump the current knowledge graph, with optional limit',
    {
      limit: z.number().optional().describe('Limit the number of nodes returned.'),
      projectContext: z.string().describe('Full path to the project directory.'),
    },
    async (a) => {
      try {
        const { logger, kg, runOnce } = await getDeps();
        const projectName = await loadProjectOrThrow({ logger, args: a, onProjectLoad: runOnce });
        const nodes = await kg.export({ project: projectName, limit: a.limit });
        return {
          content: [
            {
              type: 'text',
              text: JSON.stringify(nodes, null, 2),
            },
          ],
        };
      } catch (err) {
        return handleToolError(err as Error);
      }
    },
  );

  /** list_projects – list all available knowledge graph projects */
  server.tool(
    'list_projects',
    {
      projectContext: z
        .string()
        .optional()
        .describe(
          'Optional full path to the project directory. If provided, the project name will be extracted and highlighted as current.',
        ),
    },
    async (a) => {
      try {
        const { logger, kg } = await getDeps();
        let activeProject: string | null = null;
        if (a.projectContext) {
          const projectName = extractProjectName(a.projectContext);
          if (!projectName) {
            throw new Error('Invalid projectContext');
          }
          activeProject = projectName;
        }
        const projects = await kg.listProjects();

        logger.info(
          `[list_projects] Current project: ${activeProject}, Available projects: ${projects.join(', ')}`,
        );

        return {
          content: [
            {
              type: 'text',
              text: JSON.stringify(
                {
                  activeProject,
                  projects,
                },
                null,
                2,
              ),
            },
          ],
        };
      } catch (err) {
        return handleToolError(err as Error);
      }
    },
  );

  /** delete_thoughts – safely soft-delete one or more knowledge graph nodes */
  server.tool(
    'delete_thoughts',
    'Safely soft-delete one or more knowledge graph nodes within a project. Creates backup, checks dependencies, and rebuilds clean graph.',
    {
      nodeIds: z
        .array(z.string())
        .min(1)
        .describe('Array of node IDs to delete. Must contain at least one ID.'),
      projectContext: z.string().describe('Full path to the project directory.'),
      reason: z
        .string()
        .optional()
        .describe('Optional reason for deletion (e.g., "accidental entry", "experimental spike").'),
      checkDependents: z
        .boolean()
        .default(true)
        .describe('Check for dependent nodes before deletion. Defaults to true.'),
      confirm: z
        .boolean()
        .default(false)
        .describe('Set to true to proceed with deletion after reviewing dependencies.'),
    },
    async (a) => {
      try {
        const { logger, kg, runOnce } = await getDeps();
        const projectName = await loadProjectOrThrow({ logger, args: a, onProjectLoad: runOnce });

        // Check if nodes exist
        const nodeChecks = await Promise.all(
          a.nodeIds.map(async (id) => {
            const node = await kg.getNode(id);
            return { id, exists: !!node, node };
          }),
        );

        const nonExistentNodes = nodeChecks
          .filter((check) => !check.exists)
          .map((check) => check.id);
        if (nonExistentNodes.length > 0) {
          throw new Error(`Nodes not found: ${nonExistentNodes.join(', ')}`);
        }

        // Filter nodes by project
        const projectNodes = nodeChecks.filter((check) => check.node?.project === projectName);
        const wrongProjectNodes = nodeChecks
          .filter((check) => check.node?.project !== projectName)
          .map((check) => check.id);

        if (wrongProjectNodes.length > 0) {
          throw new Error(`Nodes not in project ${projectName}: ${wrongProjectNodes.join(', ')}`);
        }

        if (a.checkDependents && !a.confirm) {
          // Check for dependent nodes
          const dependentsMap = await kg.findDependentNodes(a.nodeIds, projectName);
          const affectedSummaries = await kg.findAffectedSummaryNodes(a.nodeIds, projectName);

          const hasDependents = Array.from(dependentsMap.values()).some((deps) => deps.length > 0);
          const hasSummaries = affectedSummaries.length > 0;

          if (hasDependents || hasSummaries) {
            return {
              content: [
                {
                  type: 'text',
                  text: JSON.stringify(
                    {
                      action: 'confirmation_required',
                      nodesToDelete: projectNodes.map((check) => ({
                        id: check.id,
                        thought: check.node?.thought,
                        role: check.node?.role,
                        tags: check.node?.tags,
                      })),
                      dependentNodes: Object.fromEntries(dependentsMap),
                      affectedSummaries: affectedSummaries.map((node) => ({
                        id: node.id,
                        thought: node.thought,
                        summarizedSegment: node.summarizedSegment,
                      })),
                      message:
                        'These nodes have dependencies or are referenced in summaries. Set confirm=true to proceed with deletion.',
                    },
                    null,
                    2,
                  ),
                },
              ],
            };
          }
        }

        // Proceed with deletion
        const result = await kg.softDeleteNodes(a.nodeIds, projectName, a.reason, 'mcp-tool');

        logger.info(
          `[delete_thoughts] Successfully deleted ${result.deletedNodes.length} nodes from project ${projectName}`,
        );

        return {
          content: [
            {
              type: 'text',
              text: JSON.stringify(
                {
                  action: 'deletion_completed',
                  deletedNodes: result.deletedNodes.map((node) => ({
                    id: node.id,
                    thought: node.thought,
                    role: node.role,
                    deletedAt: node.deletedAt,
                    deletedReason: node.deletedReason,
                  })),
                  backupPath: result.backupPath,
                  affectedSummaries: result.affectedSummaries.map((node) => ({
                    id: node.id,
                    thought: node.thought,
                    summarizedSegment: node.summarizedSegment,
                  })),
                  message: `Successfully deleted ${result.deletedNodes.length} nodes. Backup created at ${result.backupPath}`,
                },
                null,
                2,
              ),
            },
          ],
        };
      } catch (err) {
        return handleToolError(err as Error);
      }
    },
  );
};
