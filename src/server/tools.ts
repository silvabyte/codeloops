import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import {
  type CodeLoopsLogger,
  getInstance as getLogger,
  setGlobalLogger,
} from "../logger.js";
import { MemoryStore } from "../memory-store.js";
import { extractProjectName } from "../utils/project.js";

// Shared dependencies interface
type ToolDependencies = {
  logger: CodeLoopsLogger;
  store: MemoryStore;
  runOnce: (project: string) => void;
};

// Utility function to load project
const loadProjectOrThrow = ({
  logger,
  args,
  onProjectLoad,
}: {
  logger: CodeLoopsLogger;
  args: Record<string, unknown>;
  onProjectLoad: (project: string) => void;
}): string => {
  const projectContext = args.projectContext as string;
  const projectName = extractProjectName(projectContext);
  if (!projectName) {
    logger.error({ projectContext }, "Invalid projectContext");
    throw new Error(`Invalid projectContext: ${projectContext}`);
  }
  onProjectLoad(projectName);
  return projectName;
};

export const createDependencies = async (): Promise<ToolDependencies> => {
  const logger = getLogger();

  // Create MemoryStore
  const store = new MemoryStore(logger);
  await store.init();

  const runOnce = (project: string) => {
    const child = logger.child({ project });
    setGlobalLogger(child);
  };

  return { logger, store, runOnce };
};

export const registerTools = ({ server }: { server: McpServer }) => {
  let deps: ToolDependencies | null = null;

  const getDeps = async (): Promise<ToolDependencies> => {
    if (!deps) {
      deps = await createDependencies();
    }
    return deps;
  };

  /**
   * memory_store - Store a new memory entry
   */
  server.tool(
    "memory_store",
    `Store a memory entry for later recall. Use this to persist important context, decisions, errors, or learnings across sessions.

**When to use:**
- Record decisions and their rationale
- Save error patterns and solutions
- Capture user preferences
- Note important context for future sessions

**Tips:**
- Use descriptive tags for easier recall
- Include enough context to understand the memory later
- Set source to indicate where the memory came from`,
    {
      content: z.string().describe("The memory content to store"),
      projectContext: z.string().describe("Full path to the project directory"),
      tags: z
        .array(z.string())
        .optional()
        .describe("Tags for categorization and filtering"),
      source: z
        .string()
        .optional()
        .describe(
          'Source of the memory (e.g., "user-input", "file-edit", "error")'
        ),
      sessionId: z.string().optional().describe("Session ID for correlation"),
    },
    async (args) => {
      const { logger, store, runOnce } = await getDeps();
      const projectName = await loadProjectOrThrow({
        logger,
        args,
        onProjectLoad: runOnce,
      });

      const entry = await store.append({
        content: args.content,
        project: projectName,
        tags: args.tags,
        source: args.source,
        sessionId: args.sessionId,
      });

      return {
        content: [
          {
            type: "text",
            text: JSON.stringify(entry, null, 2),
          },
        ],
      };
    }
  );

  /**
   * memory_recall - Query stored memories
   */
  server.tool(
    "memory_recall",
    `Recall stored memories by searching content and filtering by tags.

**When to use:**
- Start of session to load relevant context
- Before making decisions to check past patterns
- When troubleshooting to find previous error solutions
- To retrieve user preferences

**Tips:**
- Use query for text search in content
- Use tags for precise filtering
- Combine both for best results`,
    {
      projectContext: z.string().describe("Full path to the project directory"),
      query: z
        .string()
        .optional()
        .describe("Text to search for in memory content"),
      tags: z
        .array(z.string())
        .optional()
        .describe("Filter by tags (all specified tags must be present)"),
      limit: z
        .number()
        .optional()
        .default(10)
        .describe("Maximum number of entries to return"),
    },
    async (args) => {
      const { logger, store, runOnce } = await getDeps();
      const projectName = await loadProjectOrThrow({
        logger,
        args,
        onProjectLoad: runOnce,
      });

      const entries = await store.query({
        project: projectName,
        query: args.query,
        tags: args.tags,
        limit: args.limit,
      });

      return {
        content: [
          {
            type: "text",
            text: JSON.stringify(entries, null, 2),
          },
        ],
      };
    }
  );

  /**
   * memory_forget - Soft delete a memory entry
   */
  server.tool(
    "memory_forget",
    `Soft-delete a memory entry. The entry is moved to a deleted log and can be recovered if needed.

**When to use:**
- Remove outdated or incorrect information
- Clean up experimental or temporary memories
- Remove memories that are no longer relevant`,
    {
      id: z.string().describe("ID of the memory entry to delete"),
      reason: z.string().optional().describe("Reason for deletion"),
    },
    async (args) => {
      const { store } = await getDeps();

      const deleted = await store.forget(args.id, args.reason);

      if (!deleted) {
        return {
          content: [
            {
              type: "text",
              text: JSON.stringify(
                { error: `Memory entry ${args.id} not found` },
                null,
                2
              ),
            },
          ],
        };
      }

      return {
        content: [
          {
            type: "text",
            text: JSON.stringify(
              {
                message: `Memory entry ${args.id} deleted`,
                deleted,
              },
              null,
              2
            ),
          },
        ],
      };
    }
  );

  /**
   * memory_context - Quick project context retrieval
   */
  server.tool(
    "memory_context",
    `Get recent memories for a project. Use this at session start to quickly load context.

**When to use:**
- At the beginning of a session
- When switching between projects
- To get a quick overview of recent activity`,
    {
      projectContext: z.string().describe("Full path to the project directory"),
      limit: z
        .number()
        .optional()
        .default(5)
        .describe("Number of recent entries to return (default: 5)"),
    },
    async (args) => {
      const { logger, store, runOnce } = await getDeps();
      const projectName = await loadProjectOrThrow({
        logger,
        args,
        onProjectLoad: runOnce,
      });

      const entries = await store.query({
        project: projectName,
        limit: args.limit,
      });

      return {
        content: [
          {
            type: "text",
            text: JSON.stringify(
              {
                project: projectName,
                recentMemories: entries,
                count: entries.length,
              },
              null,
              2
            ),
          },
        ],
      };
    }
  );

  /**
   * list_projects - List all available projects
   */
  server.tool(
    "list_projects",
    "List all projects that have stored memories",
    {
      projectContext: z
        .string()
        .optional()
        .describe("Optional current project path to highlight as active"),
    },
    async (args) => {
      const { logger, store } = await getDeps();
      let activeProject: string | null = null;

      if (args.projectContext) {
        const projectName = extractProjectName(args.projectContext);
        if (projectName) {
          activeProject = projectName;
        }
      }

      const projects = await store.listProjects();

      logger.info(
        `[list_projects] Current project: ${activeProject}, Available projects: ${projects.join(", ")}`
      );

      return {
        content: [
          {
            type: "text",
            text: JSON.stringify(
              {
                activeProject,
                projects,
              },
              null,
              2
            ),
          },
        ],
      };
    }
  );

  /**
   * resume - Pick up where you left off (convenience wrapper around memory_context)
   */
  server.tool(
    "resume",
    "Pick up where you left off by fetching recent memories for this project",
    {
      projectContext: z.string().describe("Full path to the project directory"),
      limit: z
        .number()
        .optional()
        .default(10)
        .describe("Number of recent entries to return"),
    },
    async (args) => {
      const { logger, store, runOnce } = await getDeps();
      const projectName = await loadProjectOrThrow({
        logger,
        args,
        onProjectLoad: runOnce,
      });

      const entries = await store.query({
        project: projectName,
        limit: args.limit,
      });

      return {
        content: [
          {
            type: "text",
            text: JSON.stringify(entries, null, 2),
          },
        ],
      };
    }
  );
};
