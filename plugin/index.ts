/**
 * CodeLoops Memory Plugin for OpenCode
 *
 * Provides persistent memory across coding sessions with:
 * - Custom tools: memory_store, memory_recall, memory_forget, memory_context
 * - Event hooks: file.edited, todo.updated, session.created, message.part.updated
 *
 * Conversation Context Capture:
 * - Captures user and assistant messages in a rolling buffer
 * - Associates conversation context with file edits
 * - Enables richer memory recall with "why" context, not just "what" changed
 *
 * bd (beads) Integration:
 * - Detects TODO comments in code changes (any syntax: //, #, /*, etc.)
 * - When bd is initialized (.beads/ exists), spawns an agent to:
 *   1. Gather context about the TODO
 *   2. Create a bd issue with that context
 *   3. Update the TODO with [bd-xxx] suffix to prevent duplicates
 *
 * Configuration:
 * 1. Config file (~/.config/codeloops/config.json):
 *    {
 *      "todo": {
 *        "model": "anthropic/claude-sonnet-4-20250514",
 *        "enabled": true
 *      }
 *    }
 *
 * 2. Environment variables (override file config):
 *    - CODELOOPS_TODO_MODEL: Model for TODO analysis
 *    - CODELOOPS_TODO_ENABLED: Set to "false" to disable
 */

import { existsSync } from "node:fs";
import path from "node:path";
import type { Plugin } from "@opencode-ai/plugin";
import { tool } from "@opencode-ai/plugin/tool";
import { nanoid } from "nanoid";
import { createLogger } from "../lib/logger.ts";
import { createMemoryStoreFunctions } from "../lib/memory-store.ts";
import { type ExtractedTodo, extractTodosFromDiff } from "./todo-extractor.ts";

// -----------------------------------------------------------------------------
// Regex Constants
// -----------------------------------------------------------------------------

const TRAILING_SLASHES_REGEX = /\/+$/;

// -----------------------------------------------------------------------------
// Plugin Logger
// -----------------------------------------------------------------------------

const pluginLogger = createLogger({
  withFile: true,
  logFileName: "plugin.log",
  retentionDays: 7,
});

// Maximum number of TODO analysis agents to spawn concurrently
const TODO_AGENT_CONCURRENCY = 3;

// -----------------------------------------------------------------------------
// TODO Agent Configuration
// -----------------------------------------------------------------------------

/**
 * Configuration for the TODO analysis agent.
 *
 * Loaded from (in order of priority):
 * 1. Environment variables (highest priority, overrides file config)
 *    - CODELOOPS_TODO_MODEL: Model to use (e.g., "anthropic/claude-sonnet-4-20250514")
 *    - CODELOOPS_TODO_ENABLED: Set to "false" to disable TODO detection
 *
 * 2. Config file: ~/.config/codeloops/config.json
 *    {
 *      "todo": {
 *        "model": "anthropic/claude-sonnet-4-20250514",
 *        "enabled": true
 *      }
 *    }
 */
type TodoAgentConfig = {
  /** Model in provider/model format (e.g., "anthropic/claude-sonnet-4-20250514") */
  model: string | undefined;
  /** Whether TODO detection is enabled */
  enabled: boolean;
};

type CodeLoopsConfig = {
  todo?: {
    model?: string;
    enabled?: boolean;
  };
};

/** Cached config to avoid re-reading file on every TODO */
let cachedConfig: CodeLoopsConfig | null = null;

function getConfigPath(): string {
  // Use XDG config dir: ~/.config/codeloops/config.json
  const configDir =
    process.env.XDG_CONFIG_HOME || path.join(process.env.HOME || "", ".config");
  return path.join(configDir, "codeloops", "config.json");
}

function loadConfigFile(): CodeLoopsConfig {
  if (cachedConfig !== null) {
    return cachedConfig;
  }

  const configPath = getConfigPath();
  try {
    if (existsSync(configPath)) {
      const content = require("node:fs").readFileSync(configPath, "utf-8");
      cachedConfig = JSON.parse(content) as CodeLoopsConfig;
      pluginLogger.info({
        msg: "Loaded codeloops config",
        path: configPath,
        todoModel: cachedConfig.todo?.model,
        todoEnabled: cachedConfig.todo?.enabled,
      });
      return cachedConfig;
    }
  } catch (err) {
    pluginLogger.warn({
      msg: "Failed to load codeloops config",
      path: configPath,
      error: err instanceof Error ? err.message : String(err),
    });
  }

  cachedConfig = {};
  return cachedConfig;
}

function getTodoAgentConfig(): TodoAgentConfig {
  const fileConfig = loadConfigFile();

  // Environment variables take precedence over file config
  const model = process.env.CODELOOPS_TODO_MODEL ?? fileConfig.todo?.model;

  // For enabled: env var "false" disables, otherwise check file config, default true
  let enabled = true;
  if (process.env.CODELOOPS_TODO_ENABLED === "false") {
    enabled = false;
  } else if (fileConfig.todo?.enabled === false) {
    enabled = false;
  }

  return { model, enabled };
}

// Create memory store functions using the shared lib
const memoryStore = createMemoryStoreFunctions(pluginLogger);

// -----------------------------------------------------------------------------
// Git Diff Helper
// -----------------------------------------------------------------------------

async function getFileDiff(
  filePath: string,
  workdir: string
): Promise<string | null> {
  const { exec } = await import("node:child_process");
  const { promisify } = await import("node:util");
  const execAsync = promisify(exec);

  if (!filePath?.trim()) {
    return null;
  }

  try {
    // Get diff for the specific file (staged + unstaged)
    const { stdout } = await execAsync(`git diff HEAD -- "${filePath}"`, {
      cwd: workdir,
    });
    return stdout.trim() || null;
  } catch {
    // Git not available or file not tracked
    return null;
  }
}

// -----------------------------------------------------------------------------
// TODO Comment Detection & bd Integration
// -----------------------------------------------------------------------------

/**
 * Check if bd (beads) is initialized in the given directory.
 */
function isBdInitialized(workdir: string): boolean {
  const beadsDir = path.join(workdir, ".beads");
  return existsSync(beadsDir);
}

/**
 * Build the prompt for the spawned opencode agent to analyze the TODO
 * and create a bd issue.
 */
function buildTodoAnalysisPrompt(ctx: {
  file: string;
  lineNumber: number;
  todoText: string;
  fullLine: string;
}): string {
  return `A TODO comment was added to the codebase:

File: ${ctx.file}
Line: ${ctx.lineNumber}
Full line: ${ctx.fullLine}
TODO text: ${ctx.todoText}

Your task:
1. First, verify bd (beads issue tracker) is initialized by checking for a .beads/ directory
2. If not initialized, respond with "bd not initialized, skipping" and exit immediately
3. If initialized, thoroughly gather context about this TODO:
   - Read the file and understand what the TODO is referring to
   - Look at surrounding code to understand the context
   - If the TODO references other files/functions, explore them
   - Understand WHY this TODO exists based on the code context
4. Create a bd issue that captures this TODO with the context you gathered:
   - Use: bd create "<title>" -d "<description with context>"
   - Choose appropriate type (-t bug/feature/task/chore) based on the TODO content
   - The title should be concise but descriptive
   - The description should include the file path, line number, and gathered context
5. After creating the issue, note the bd issue ID from the output (e.g., bd-abc123)
6. Update the TODO comment in the source file to include the bd identifier:
   - Find the exact line and add the bd ID in brackets at the end
   - Example: // TODO: fix this -> // TODO: fix this [bd-abc123]
   - This prevents duplicate issue creation for this TODO

IMPORTANT:
- Do NOT invent or assume information you cannot verify from the code
- If unsure about something, note it as "unclear" or "needs investigation" in the issue description
- Be thorough in gathering context but stick to verifiable facts from the codebase
- If the TODO already has a [bd-xxx] suffix, skip it entirely - it's already tracked`;
}

/**
 * Spawn an opencode run process in the background to analyze a TODO
 * and create a bd issue.
 */
async function spawnTodoAnalysisAgent(
  prompt: string,
  workdir: string,
  todoContext: { todoText: string; lineNumber: number; file: string },
  config: TodoAgentConfig
): Promise<void> {
  const { spawn } = await import("node:child_process");

  return new Promise((resolve, reject) => {
    try {
      // Build command args: opencode run [--model provider/model] "prompt"
      const args = ["run"];
      if (config.model) {
        args.push("--model", config.model);
      }
      args.push(prompt);

      const child = spawn("opencode", args, {
        cwd: workdir,
        detached: true,
        stdio: "ignore",
      });

      child.on("error", (err) => {
        pluginLogger.error({
          msg: "opencode agent process error",
          file: todoContext.file,
          line: todoContext.lineNumber,
          todo: todoContext.todoText,
          error: err.message,
        });
        reject(err);
      });

      // Consider spawn successful once the process starts
      child.on("spawn", () => {
        pluginLogger.info({
          msg: "Spawned TODO analysis agent",
          file: todoContext.file,
          line: todoContext.lineNumber,
          todo: todoContext.todoText,
          model: config.model ?? "default",
        });
        child.unref();
        resolve();
      });
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      pluginLogger.error({
        msg: "Failed to spawn opencode agent",
        file: todoContext.file,
        line: todoContext.lineNumber,
        todo: todoContext.todoText,
        error: errorMessage,
      });
      reject(err);
    }
  });
}

type TodoCommentContext = {
  file: string;
  lineNumber: number;
  todoText: string;
  fullLine: string;
  workdir: string;
  projectName: string;
};

/**
 * Handle a detected TODO comment by spawning an agent to create a bd issue.
 */
async function handleTodoCommentDetected(
  ctx: TodoCommentContext,
  config: TodoAgentConfig
): Promise<void> {
  // Only proceed if bd is initialized in this project
  if (!isBdInitialized(ctx.workdir)) {
    return;
  }

  const prompt = buildTodoAnalysisPrompt({
    file: ctx.file,
    lineNumber: ctx.lineNumber,
    todoText: ctx.todoText,
    fullLine: ctx.fullLine,
  });

  await spawnTodoAnalysisAgent(
    prompt,
    ctx.workdir,
    {
      file: ctx.file,
      lineNumber: ctx.lineNumber,
      todoText: ctx.todoText,
    },
    config
  );
}

/**
 * Process multiple TODOs with concurrency limit.
 * Spawns up to TODO_AGENT_CONCURRENCY agents at a time.
 */
async function processTodosWithConcurrency(
  todos: ExtractedTodo[],
  ctx: { file: string; workdir: string; projectName: string }
): Promise<void> {
  const config = getTodoAgentConfig();

  // Skip if TODO detection is disabled
  if (!config.enabled) {
    pluginLogger.info({ msg: "TODO detection disabled via config" });
    return;
  }

  for (let i = 0; i < todos.length; i += TODO_AGENT_CONCURRENCY) {
    const batch = todos.slice(i, i + TODO_AGENT_CONCURRENCY);
    const results = await Promise.allSettled(
      batch.map((todo) =>
        handleTodoCommentDetected(
          {
            file: ctx.file,
            lineNumber: todo.lineNumber,
            todoText: todo.todoText,
            fullLine: todo.fullLine,
            workdir: ctx.workdir,
            projectName: ctx.projectName,
          },
          config
        )
      )
    );

    // Log any failures from this batch
    for (const [index, result] of results.entries()) {
      if (result.status === "rejected") {
        const todo = batch[index];
        pluginLogger.error({
          msg: "Failed to process TODO for bd issue creation",
          file: ctx.file,
          line: todo.lineNumber,
          todo: todo.todoText,
          error: String(result.reason),
        });
      }
    }
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
// Conversation Context Buffer
// -----------------------------------------------------------------------------

/**
 * Buffer to accumulate recent conversation context.
 * This allows us to associate what was said with file edits.
 */
type ConversationBuffer = {
  /** Recent user messages */
  userMessages: Array<{ text: string; timestamp: number }>;
  /** Recent assistant messages */
  assistantMessages: Array<{ text: string; timestamp: number }>;
  /** Track message IDs to their roles */
  messageRoles: Map<string, "user" | "assistant">;
  /** Maximum age of messages to keep (ms) */
  maxAge: number;
  /** Maximum number of messages to keep per role */
  maxMessages: number;
};

function createConversationBuffer(): ConversationBuffer {
  return {
    userMessages: [],
    assistantMessages: [],
    messageRoles: new Map(),
    maxAge: 60_000, // 1 minute
    maxMessages: 5,
  };
}

function addToBuffer(
  buffer: ConversationBuffer,
  role: "user" | "assistant",
  text: string
): void {
  const messages =
    role === "user" ? buffer.userMessages : buffer.assistantMessages;
  const now = Date.now();

  // Add new message
  messages.push({ text, timestamp: now });

  // Clean up old messages
  const cutoff = now - buffer.maxAge;
  while (messages.length > 0 && messages[0].timestamp < cutoff) {
    messages.shift();
  }

  // Trim to max size
  while (messages.length > buffer.maxMessages) {
    messages.shift();
  }
}

function getRecentContext(buffer: ConversationBuffer): string {
  const now = Date.now();
  const cutoff = now - buffer.maxAge;

  const recentUser = buffer.userMessages
    .filter((m) => m.timestamp >= cutoff)
    .map((m) => m.text)
    .join("\n");

  const recentAssistant = buffer.assistantMessages
    .filter((m) => m.timestamp >= cutoff)
    .map((m) => m.text)
    .join("\n");

  const parts: string[] = [];
  if (recentUser) {
    parts.push(`**User said:**\n${recentUser}`);
  }
  if (recentAssistant) {
    parts.push(`**Assistant said:**\n${recentAssistant}`);
  }

  return parts.join("\n\n");
}

function clearBuffer(buffer: ConversationBuffer): void {
  buffer.userMessages = [];
  buffer.assistantMessages = [];
  buffer.messageRoles.clear();
}

function setMessageRole(
  buffer: ConversationBuffer,
  messageId: string,
  role: "user" | "assistant"
): void {
  buffer.messageRoles.set(messageId, role);
}

function getMessageRole(
  buffer: ConversationBuffer,
  messageId: string
): "user" | "assistant" | undefined {
  return buffer.messageRoles.get(messageId);
}

// -----------------------------------------------------------------------------
// Event Handlers
// -----------------------------------------------------------------------------

type DedupFn = (key: string) => boolean;
type SetSessionFn = (id: string) => void;

type FileEditContext = {
  file: string;
  projectName: string;
  sessionId: string | undefined;
  isDuplicate: DedupFn;
  workdir: string;
  conversationContext: string;
};

async function handleFileEdited(ctx: FileEditContext): Promise<void> {
  const eventKey = `file.edited:${ctx.file}`;
  if (ctx.isDuplicate(eventKey)) {
    return;
  }

  // Get git diff for the file
  const diff = await getFileDiff(ctx.file, ctx.workdir);

  // Build content with conversation context
  const contentParts: string[] = [];

  // Add conversation context if available
  if (ctx.conversationContext) {
    contentParts.push("## Conversation Context\n");
    contentParts.push(ctx.conversationContext);
    contentParts.push("\n");
  }

  // Add file edit info
  contentParts.push("## File Edit\n");
  contentParts.push(`Edited file: ${ctx.file}`);

  if (diff) {
    contentParts.push(`\n\n\`\`\`diff\n${diff}\n\`\`\``);
  }

  const content = contentParts.join("");

  await memoryStore.append({
    content,
    project: ctx.projectName,
    tags: ["file-edit", "auto-capture"],
    source: "file.edited",
    sessionId: ctx.sessionId,
  });

  // Check for TODO comments in the diff and spawn bd agents with concurrency limit
  if (diff) {
    const todos = extractTodosFromDiff(diff);
    if (todos.length > 0) {
      await processTodosWithConcurrency(todos, {
        file: ctx.file,
        workdir: ctx.workdir,
        projectName: ctx.projectName,
      });
    }
  }
}

async function handleTodoUpdated(
  todos: unknown[],
  projectName: string,
  sessionId: string | undefined,
  isDuplicate: DedupFn
): Promise<void> {
  const eventKey = `todo.updated:${JSON.stringify(todos)}`;
  if (isDuplicate(eventKey)) {
    return;
  }
  const todoCount = todos?.length || 0;
  await memoryStore.append({
    content: `Todo list updated (${todoCount} items)`,
    project: projectName,
    tags: ["todo", "auto-capture"],
    source: "todo.updated",
    sessionId,
  });
}

function handleSessionCreated(
  projectName: string,
  isDuplicate: DedupFn,
  setSession: SetSessionFn
) {
  const eventKey = `session.created:${projectName}`;
  if (isDuplicate(eventKey)) {
    return;
  }
  setSession(nanoid());
}

type EventContext = {
  projectName: string;
  sessionId: string | undefined;
  isDuplicate: DedupFn;
  setSession: SetSessionFn;
  workdir: string;
  conversationBuffer: ConversationBuffer;
};

/**
 * Message part type from OpenCode SDK
 */
type MessagePart = {
  type: string;
  text?: string;
  messageID?: string;
  sessionID?: string;
};

/**
 * Message info type from OpenCode SDK
 */
type MessageInfo = {
  role?: string;
  id?: string;
  sessionID?: string;
};

function handleMessageUpdated(
  properties: Record<string, unknown> | undefined,
  buffer: ConversationBuffer
): void {
  const info = properties?.info as MessageInfo | undefined;
  if (info?.id && info.role) {
    const role = info.role as "user" | "assistant";
    setMessageRole(buffer, info.id, role);
  }
}

function handleMessagePartUpdated(
  properties: Record<string, unknown> | undefined,
  buffer: ConversationBuffer
): void {
  const part = properties?.part as MessagePart | undefined;
  if (part?.type === "text" && part.text && part.messageID) {
    const role = getMessageRole(buffer, part.messageID) ?? "assistant";
    addToBuffer(buffer, role, part.text);
  }
}

async function handleEvent(
  event: { type: string; properties?: Record<string, unknown> },
  ctx: EventContext
): Promise<void> {
  // Capture full messages to get role information
  if (event.type === "message.updated") {
    handleMessageUpdated(event.properties, ctx.conversationBuffer);
    return;
  }

  // Capture message parts (conversation text)
  if (event.type === "message.part.updated") {
    handleMessagePartUpdated(event.properties, ctx.conversationBuffer);
    return;
  }

  if (event.type === "file.edited") {
    const file = (event.properties?.file as string) || "";
    const conversationContext = getRecentContext(ctx.conversationBuffer);

    await handleFileEdited({
      file,
      projectName: ctx.projectName,
      sessionId: ctx.sessionId,
      isDuplicate: ctx.isDuplicate,
      workdir: ctx.workdir,
      conversationContext,
    });

    // Clear buffer after capturing context for a file edit
    clearBuffer(ctx.conversationBuffer);
    return;
  }

  if (event.type === "todo.updated") {
    const todos = (event.properties?.todos as unknown[]) || [];
    await handleTodoUpdated(
      todos,
      ctx.projectName,
      ctx.sessionId,
      ctx.isDuplicate
    );
    return;
  }

  if (event.type === "session.created") {
    clearBuffer(ctx.conversationBuffer);
    handleSessionCreated(ctx.projectName, ctx.isDuplicate, ctx.setSession);
  }
}

// -----------------------------------------------------------------------------
// Tool Argument Types
// -----------------------------------------------------------------------------

type MemoryStoreArgs = {
  content: string;
  tags?: string[];
  source?: string;
};

type MemoryRecallArgs = {
  query?: string;
  tags?: string[];
  limit: number;
};

type MemoryForgetArgs = {
  id: string;
  reason?: string;
};

type MemoryContextArgs = {
  limit: number;
};

// -----------------------------------------------------------------------------
// Plugin Export
// -----------------------------------------------------------------------------

export const CodeLoopsMemory: Plugin = async ({ project, directory }) => {
  // Use project id or extract from directory path
  const workdir = project?.worktree || directory;
  const projectName = project?.id
    ? extractProjectName(project.worktree)
    : extractProjectName(directory);
  let currentSessionId: string | undefined;

  // Conversation buffer to capture context around file edits
  const conversationBuffer = createConversationBuffer();

  // Deduplication: track recent events to prevent duplicates
  const recentEvents = new Map<string, number>();
  const DEDUP_WINDOW_MS = 1000; // Ignore duplicate events within 1 second

  function isDuplicateEvent(eventKey: string): boolean {
    const now = Date.now();
    const lastSeen = recentEvents.get(eventKey);

    // Clean up old entries
    for (const [key, timestamp] of recentEvents) {
      if (now - timestamp > DEDUP_WINDOW_MS) {
        recentEvents.delete(key);
      }
    }

    if (lastSeen && now - lastSeen < DEDUP_WINDOW_MS) {
      return true;
    }

    recentEvents.set(eventKey, now);
    return false;
  }

  // Initialize memory store
  await memoryStore.init();

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
        },
        async execute(args: MemoryStoreArgs) {
          const entry = await memoryStore.append({
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
        async execute(args: MemoryRecallArgs) {
          const entries = await memoryStore.query({
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
        async execute(args: MemoryForgetArgs) {
          const deleted = await memoryStore.forget(args.id, args.reason);
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
        async execute(args: MemoryContextArgs) {
          const entries = await memoryStore.query({
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
          const projects = await memoryStore.listProjects();
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
      await handleEvent(event, {
        projectName,
        sessionId: currentSessionId,
        isDuplicate: isDuplicateEvent,
        setSession: (id) => {
          currentSessionId = id;
        },
        workdir,
        conversationBuffer,
      });
    },
  };
};

export default CodeLoopsMemory;
