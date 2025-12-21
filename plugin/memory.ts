/**
 * CodeLoops Memory Plugin for OpenCode
 *
 * Provides persistent memory across coding sessions with:
 * - Custom tools: memory_store, memory_recall, memory_forget, memory_context
 * - Event hooks: file.edited, todo.updated, session.created
 */

import { createReadStream, existsSync } from "node:fs";
import fs from "node:fs/promises";
import path from "node:path";
import readline from "node:readline";
import type { Plugin } from "@opencode-ai/plugin";
import { tool } from "@opencode-ai/plugin";
import envPaths from "env-paths";
import { nanoid } from "nanoid";
import { lock, unlock } from "proper-lockfile";
import { z } from "zod";

// -----------------------------------------------------------------------------
// Regex Constants
// -----------------------------------------------------------------------------

const TRAILING_SLASHES_REGEX = /\/+$/;

// -----------------------------------------------------------------------------
// Schema & Types
// -----------------------------------------------------------------------------

const MemoryEntrySchema = z.object({
  id: z.string(),
  project: z.string(),
  content: z.string(),
  tags: z.array(z.string()).optional(),
  createdAt: z.string().datetime(),
  sessionId: z.string().optional(),
  source: z.string().optional(),
});

type MemoryEntry = z.infer<typeof MemoryEntrySchema>;

interface DeletedMemoryEntry extends MemoryEntry {
  deletedAt: string;
  deletedReason?: string;
}

type QueryOptions = {
  project?: string;
  tags?: string[];
  query?: string;
  limit?: number;
  sessionId?: string;
};

// -----------------------------------------------------------------------------
// Filter Helper Functions
// -----------------------------------------------------------------------------

function matchesProject(entry: MemoryEntry, project?: string): boolean {
  return !project || entry.project === project;
}

function matchesSessionId(entry: MemoryEntry, sessionId?: string): boolean {
  return !sessionId || entry.sessionId === sessionId;
}

function matchesTags(entry: MemoryEntry, tags?: string[]): boolean {
  if (!tags || tags.length === 0) {
    return true;
  }
  return Boolean(entry.tags && tags.every((tag) => entry.tags?.includes(tag)));
}

function matchesQueryText(entry: MemoryEntry, searchQuery?: string): boolean {
  if (!searchQuery) {
    return true;
  }
  return entry.content.toLowerCase().includes(searchQuery.toLowerCase());
}

function entryMatchesFilters(
  entry: MemoryEntry,
  options: QueryOptions
): boolean {
  return (
    matchesProject(entry, options.project) &&
    matchesSessionId(entry, options.sessionId) &&
    matchesTags(entry, options.tags) &&
    matchesQueryText(entry, options.query)
  );
}

// -----------------------------------------------------------------------------
// MemoryStore (embedded for standalone plugin)
// -----------------------------------------------------------------------------

const paths = envPaths("codeloops", { suffix: "" });
const dataDir = paths.data;
const logFilePath = path.resolve(dataDir, "memory.ndjson");
const deletedLogFilePath = path.resolve(dataDir, "memory.deleted.ndjson");

async function ensureLogFile(): Promise<void> {
  if (!(await fs.stat(logFilePath).catch(() => null))) {
    await fs.mkdir(path.dirname(logFilePath), { recursive: true });
    await fs.writeFile(logFilePath, "");
  }
}

function parseMemoryEntry(line: string): MemoryEntry | null {
  try {
    const parsed = JSON.parse(line);
    return MemoryEntrySchema.parse(parsed);
  } catch {
    return null;
  }
}

async function append(input: {
  content: string;
  project: string;
  tags?: string[];
  sessionId?: string;
  source?: string;
}): Promise<MemoryEntry> {
  await ensureLogFile();

  const entry: MemoryEntry = {
    id: nanoid(),
    project: input.project,
    content: input.content,
    tags: input.tags,
    createdAt: new Date().toISOString(),
    sessionId: input.sessionId,
    source: input.source,
  };

  const line = `${JSON.stringify(entry)}\n`;

  try {
    await lock(logFilePath, { retries: 3 });
    await fs.appendFile(logFilePath, line, "utf8");
  } finally {
    try {
      await unlock(logFilePath);
    } catch {
      // Ignore unlock errors
    }
  }

  return entry;
}

async function query(options: QueryOptions): Promise<MemoryEntry[]> {
  await ensureLogFile();

  const { limit } = options;
  const entries: MemoryEntry[] = [];

  if (!existsSync(logFilePath)) {
    return entries;
  }

  const fileStream = createReadStream(logFilePath);
  const rl = readline.createInterface({
    input: fileStream,
    crlfDelay: Number.POSITIVE_INFINITY,
  });

  try {
    for await (const line of rl) {
      const entry = parseMemoryEntry(line);
      if (entry && entryMatchesFilters(entry, options)) {
        entries.push(entry);
        if (limit && entries.length > limit) {
          entries.shift();
        }
      }
    }

    return entries;
  } finally {
    rl.close();
    fileStream.close();
  }
}

async function getById(id: string): Promise<MemoryEntry | undefined> {
  await ensureLogFile();

  if (!existsSync(logFilePath)) {
    return;
  }

  const fileStream = createReadStream(logFilePath);
  const rl = readline.createInterface({
    input: fileStream,
    crlfDelay: Number.POSITIVE_INFINITY,
  });

  try {
    for await (const line of rl) {
      const entry = parseMemoryEntry(line);
      if (entry?.id === id) {
        return entry;
      }
    }
    return;
  } finally {
    rl.close();
    fileStream.close();
  }
}

async function forget(
  id: string,
  reason?: string
): Promise<DeletedMemoryEntry | undefined> {
  const entry = await getById(id);
  if (!entry) {
    return;
  }

  const deletedEntry: DeletedMemoryEntry = {
    ...entry,
    deletedAt: new Date().toISOString(),
    deletedReason: reason,
  };

  // Append to deleted log
  await fs.mkdir(path.dirname(deletedLogFilePath), { recursive: true });
  await fs.appendFile(
    deletedLogFilePath,
    `${JSON.stringify(deletedEntry)}\n`,
    "utf8"
  );

  // Rebuild main log without deleted entry
  const entries: MemoryEntry[] = [];
  const fileStream = createReadStream(logFilePath);
  const rl = readline.createInterface({
    input: fileStream,
    crlfDelay: Number.POSITIVE_INFINITY,
  });

  try {
    for await (const line of rl) {
      const e = parseMemoryEntry(line);
      if (e && e.id !== id) {
        entries.push(e);
      }
    }
  } finally {
    rl.close();
    fileStream.close();
  }

  const tempPath = `${logFilePath}.tmp`;
  const lines = entries.map((e) => `${JSON.stringify(e)}\n`).join("");
  await fs.writeFile(tempPath, lines, "utf8");
  await fs.rename(tempPath, logFilePath);

  return deletedEntry;
}

async function listProjects(): Promise<string[]> {
  await ensureLogFile();

  const projects = new Set<string>();

  if (!existsSync(logFilePath)) {
    return [];
  }

  const fileStream = createReadStream(logFilePath);
  const rl = readline.createInterface({
    input: fileStream,
    crlfDelay: Number.POSITIVE_INFINITY,
  });

  try {
    for await (const line of rl) {
      const entry = parseMemoryEntry(line);
      if (entry?.project) {
        projects.add(entry.project);
      }
    }
    return Array.from(projects);
  } finally {
    rl.close();
    fileStream.close();
  }
}

// -----------------------------------------------------------------------------
// Helper to extract project name
// -----------------------------------------------------------------------------

function extractProjectName(projectPath: string): string {
  // Extract last directory name from path
  const normalized = projectPath
    .replace(/\\/g, "/")
    .replace(TRAILING_SLASHES_REGEX, "");
  const parts = normalized.split("/");
  return parts.at(-1) || "unknown";
}

// -----------------------------------------------------------------------------
// Plugin Export
// -----------------------------------------------------------------------------

export const CodeLoopsMemory: Plugin = async ({ project, directory }) => {
  // Use project id or extract from directory path
  const projectName = project?.id
    ? extractProjectName(project.worktree)
    : extractProjectName(directory);
  let currentSessionId: string | undefined;

  // Ensure log file exists on plugin initialization
  await ensureLogFile();

  return {
    // -------------------------------------------------------------------------
    // Custom Tools
    // -------------------------------------------------------------------------
    tool: {
      memory_store: tool({
        description:
          "Store a memory for later recall. Use for decisions, preferences, errors, context.",
        args: {
          content: tool.schema.string().describe("The memory content to store"),
          tags: tool.schema
            .array(tool.schema.string())
            .optional()
            .describe("Tags for filtering"),
          source: tool.schema.string().optional().describe("Source of memory"),
          //todo: store git diff from project dir as well for context...
        },
        async execute(args) {
          const entry = await append({
            content: args.content,
            project: projectName,
            tags: args.tags,
            source: args.source,
            sessionId: currentSessionId,
          });
          return `Stored memory: ${entry.id}`;
        },
      }),

      memory_recall: tool({
        description: "Query stored memories. Filter by tags or search content.",
        args: {
          query: tool.schema
            .string()
            .optional()
            .describe("Text to search in content"),
          tags: tool.schema
            .array(tool.schema.string())
            .optional()
            .describe("Filter by tags"),
          limit: tool.schema
            .number()
            .optional()
            .default(10)
            .describe("Max entries to return"),
        },
        async execute(args) {
          const entries = await query({
            project: projectName,
            query: args.query,
            tags: args.tags,
            limit: args.limit,
          });
          return JSON.stringify(entries, null, 2);
        },
      }),

      memory_forget: tool({
        description: "Soft-delete a memory entry.",
        args: {
          id: tool.schema.string().describe("ID of memory to delete"),
          reason: tool.schema
            .string()
            .optional()
            .describe("Reason for deletion"),
        },
        async execute(args) {
          const deleted = await forget(args.id, args.reason);
          if (!deleted) {
            return `Memory ${args.id} not found`;
          }
          return `Deleted memory: ${args.id}`;
        },
      }),

      memory_context: tool({
        description: "Get recent memories for quick context loading.",
        args: {
          limit: tool.schema
            .number()
            .optional()
            .default(5)
            .describe("Number of recent entries"),
        },
        async execute(args) {
          const entries = await query({
            project: projectName,
            limit: args.limit,
          });
          return JSON.stringify(
            {
              project: projectName,
              count: entries.length,
              memories: entries,
            },
            null,
            2
          );
        },
      }),

      memory_projects: tool({
        description: "List all projects with stored memories.",
        args: {},
        async execute() {
          const projects = await listProjects();
          return JSON.stringify(
            {
              current: projectName,
              projects,
            },
            null,
            2
          );
        },
      }),
    },

    // -------------------------------------------------------------------------
    // Event Hooks
    // -------------------------------------------------------------------------

    event: async ({ event }) => {
      // Auto-capture file edits
      if (event.type === "file.edited") {
        const fileEvent = event as {
          type: "file.edited";
          properties: { file: string };
        };
        await append({
          content: `Edited file: ${fileEvent.properties.file}`,
          project: projectName,
          tags: ["file-edit", "auto-capture"],
          source: "file.edited",
          sessionId: currentSessionId,
        });
      }

      // Auto-capture todo updates
      if (event.type === "todo.updated") {
        const todoEvent = event as {
          type: "todo.updated";
          properties: { todos: unknown[] };
        };
        const todoCount = todoEvent.properties.todos?.length || 0;
        await append({
          content: `Todo list updated (${todoCount} items)`,
          project: projectName,
          tags: ["todo", "auto-capture"],
          source: "todo.updated",
          sessionId: currentSessionId,
        });
      }

      // Load context on session start
      if (event.type === "session.created") {
        currentSessionId = nanoid();
        const recentMemories = await query({
          project: projectName,
          limit: 5,
        });

        if (recentMemories.length > 0) {
          console.log(
            `[codeloops] Loaded ${recentMemories.length} recent memories for ${projectName}`
          );
        }
      }
    },
  };
};

export default CodeLoopsMemory;
