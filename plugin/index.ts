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
import {
  type ExtractedTodo,
  extractTodosFromDiff,
} from "../src/utils/todo-extractor.ts";

// -----------------------------------------------------------------------------
// Regex Constants
// -----------------------------------------------------------------------------

const TRAILING_SLASHES_REGEX = /\/+$/;
const JSON_EXTRACT_REGEX = /\{[\s\S]*\}/;

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
  critic?: {
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
        criticModel: cachedConfig.critic?.model,
        criticEnabled: cachedConfig.critic?.enabled,
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

// -----------------------------------------------------------------------------
// Critic Agent Configuration
// -----------------------------------------------------------------------------

/**
 * Configuration for the Critic agent in the actor-critic loop.
 *
 * Loaded from (in order of priority):
 * 1. Environment variables (highest priority, overrides file config)
 *    - CODELOOPS_CRITIC_MODEL: Model to use (defaults to actor's model if not set)
 *    - CODELOOPS_CRITIC_ENABLED: Set to "false" to disable critic
 *
 * 2. Config file: ~/.config/codeloops/config.json
 *    {
 *      "critic": {
 *        "model": "anthropic/claude-haiku-4-20250514",
 *        "enabled": true
 *      }
 *    }
 */
type CriticConfig = {
  /** Model in provider/model format (null = use actor's model) */
  model: string | undefined;
  /** Whether critic is enabled */
  enabled: boolean;
};

function getCriticConfig(): CriticConfig {
  const fileConfig = loadConfigFile();

  // Environment variables take precedence over file config
  const model = process.env.CODELOOPS_CRITIC_MODEL ?? fileConfig.critic?.model;

  // For enabled: env var "false" disables, otherwise check file config, default true
  let enabled = true;
  if (process.env.CODELOOPS_CRITIC_ENABLED === "false") {
    enabled = false;
  } else if (fileConfig.critic?.enabled === false) {
    enabled = false;
  }

  return { model, enabled };
}

// -----------------------------------------------------------------------------
// Critic Types
// -----------------------------------------------------------------------------

/**
 * Structured feedback from the critic agent.
 */
type CriticFeedback = {
  verdict: "proceed" | "revise" | "stop";
  confidence: number;
  issues: string[];
  suggestions: string[];
  context: string;
  reasoning: string;
};

/**
 * Context provided to the critic for analysis.
 */
type CriticContext = {
  action: {
    tool: string;
    args: Record<string, unknown>;
    result: string;
  };
  diff?: string;
  conversationContext: string;
  project: {
    name: string;
    workdir: string;
  };
};

// Create memory store functions using the shared lib
const memoryStore = createMemoryStoreFunctions(pluginLogger);

// -----------------------------------------------------------------------------
// Critic Implementation
// -----------------------------------------------------------------------------

/**
 * Format the context into a prompt for the critic agent.
 */
function formatCriticPrompt(ctx: CriticContext): string {
  const parts: string[] = [
    "## Action Taken",
    "",
    `**Tool:** ${ctx.action.tool}`,
    "**Arguments:**",
    "```json",
    JSON.stringify(ctx.action.args, null, 2),
    "```",
    "",
    "**Result:**",
    "```",
    ctx.action.result.slice(0, 2000), // Truncate very long results
    "```",
  ];

  if (ctx.diff) {
    parts.push(
      "",
      "## File Changes",
      "```diff",
      ctx.diff.slice(0, 3000),
      "```"
    );
  }

  if (ctx.conversationContext) {
    parts.push("", "## Conversation Context", ctx.conversationContext);
  }

  parts.push(
    "",
    "## Your Task",
    "",
    "Analyze this action and provide structured JSON feedback.",
    "Use your tools to read files, search code, or gather additional context as needed."
  );

  return parts.join("\n");
}

/**
 * Parse the critic's JSON response.
 * Falls back to a default "proceed" response if parsing fails.
 */
function parseCriticResponse(responseText: string): CriticFeedback {
  try {
    // Try to extract JSON from the response (critic might include extra text)
    const jsonMatch = responseText.match(JSON_EXTRACT_REGEX);
    if (jsonMatch) {
      const parsed = JSON.parse(jsonMatch[0]);
      return {
        verdict: parsed.verdict || "proceed",
        confidence:
          typeof parsed.confidence === "number" ? parsed.confidence : 0.5,
        issues: Array.isArray(parsed.issues) ? parsed.issues : [],
        suggestions: Array.isArray(parsed.suggestions)
          ? parsed.suggestions
          : [],
        context: parsed.context || "",
        reasoning: parsed.reasoning || "",
      };
    }
  } catch (err) {
    pluginLogger.warn({
      msg: "Failed to parse critic response",
      error: err instanceof Error ? err.message : String(err),
      responsePreview: responseText.slice(0, 200),
    });
  }

  // Default fallback
  return {
    verdict: "proceed",
    confidence: 0.5,
    issues: [],
    suggestions: [],
    context: "",
    reasoning: "Unable to parse critic response, defaulting to proceed.",
  };
}

/**
 * Format critic feedback for injection into the actor's context.
 */
function formatFeedbackForActor(feedback: CriticFeedback): string {
  const verdictSymbol: Record<string, string> = {
    proceed: "[PROCEED]",
    revise: "[REVISE]",
    stop: "[STOP]",
  };

  const parts: string[] = [
    "---",
    `## Critic Feedback ${verdictSymbol[feedback.verdict] || ""}`,
    "",
    `**Verdict:** ${feedback.verdict.toUpperCase()} (confidence: ${Math.round(feedback.confidence * 100)}%)`,
  ];

  if (feedback.issues.length > 0) {
    parts.push("", "### Issues");
    for (const issue of feedback.issues) {
      parts.push(`- ${issue}`);
    }
  }

  if (feedback.suggestions.length > 0) {
    parts.push("", "### Suggestions");
    for (const suggestion of feedback.suggestions) {
      parts.push(`- ${suggestion}`);
    }
  }

  if (feedback.context) {
    parts.push("", "### Context", feedback.context);
  }

  if (feedback.reasoning) {
    parts.push("", "### Reasoning", feedback.reasoning);
  }

  parts.push("---");

  return parts.join("\n");
}

/**
 * Set of session IDs that are critic sessions (to avoid critic critiquing itself).
 */
const criticSessionIds = new Set<string>();

/**
 * Tools that should trigger critic analysis.
 * We focus on tools that make changes or could affect the codebase.
 */
const CRITIC_TRIGGER_TOOLS = new Set(["edit", "write", "bash", "multiEdit"]);

/**
 * Get or create a critic session for the current actor session.
 * Reuses existing session if available, creates new one if needed.
 */
async function getOrCreateCriticSession(
  // biome-ignore lint/suspicious/noExplicitAny: SDK client types are complex
  client: any,
  actorSessionId: string
): Promise<string | null> {
  // If actor session changed, clean up old critic session
  if (
    criticState.actorSessionId &&
    criticState.actorSessionId !== actorSessionId
  ) {
    await cleanupCriticSession(client);
  }

  // Reuse existing critic session if available
  if (criticState.criticSessionId) {
    return criticState.criticSessionId;
  }

  // Create new critic session
  const sessionResult = await client.session.create({
    body: { title: `critic-for-${actorSessionId}` },
  });

  const sessionId = sessionResult.data?.id;
  if (!sessionId) {
    pluginLogger.error({ msg: "Failed to create critic session" });
    return null;
  }

  // Track the new session
  criticState.criticSessionId = sessionId;
  criticState.actorSessionId = actorSessionId;
  criticSessionIds.add(sessionId);

  pluginLogger.info({
    msg: "Created reusable critic session",
    criticSessionId: sessionId,
    actorSessionId,
  });

  return sessionId;
}

/**
 * Clean up the current critic session.
 * Called when actor session changes or critic is disabled.
 */
async function cleanupCriticSession(
  // biome-ignore lint/suspicious/noExplicitAny: SDK client types are complex
  client: any
): Promise<void> {
  if (!criticState.criticSessionId) {
    return;
  }

  const sessionId = criticState.criticSessionId;
  criticSessionIds.delete(sessionId);
  criticState.criticSessionId = null;
  criticState.actorSessionId = null;

  try {
    await client.session.delete({ path: { id: sessionId } });
    pluginLogger.info({
      msg: "Cleaned up critic session",
      sessionId,
    });
  } catch (err) {
    pluginLogger.warn({
      msg: "Failed to delete critic session",
      sessionId,
      error: err instanceof Error ? err.message : String(err),
    });
  }
}

/**
 * Options for invoking the critic agent.
 */
type InvokeCriticOptions = {
  // biome-ignore lint/suspicious/noExplicitAny: SDK client types are complex
  client: any;
  context: CriticContext;
  config: CriticConfig;
  actorSessionId: string;
  actorModel?: { providerID: string; modelID: string };
};

/**
 * Invoke the critic agent to analyze an action.
 * Returns structured feedback.
 *
 * Uses a reusable critic session per actor session to:
 * 1. Reduce session creation overhead
 * 2. Allow critic to build context across multiple analyses
 * 3. Enable continuity in feedback within an actor session
 */
async function invokeCritic(
  opts: InvokeCriticOptions
): Promise<CriticFeedback> {
  // Get or create a reusable critic session
  const sessionId = await getOrCreateCriticSession(
    opts.client,
    opts.actorSessionId
  );
  if (!sessionId) {
    return parseCriticResponse("");
  }

  try {
    // Determine model to use
    let model: { providerID: string; modelID: string } | undefined;
    if (opts.config.model) {
      const [providerID, ...modelParts] = opts.config.model.split("/");
      model = { providerID, modelID: modelParts.join("/") };
    } else if (opts.actorModel) {
      model = opts.actorModel;
    }

    // Send context to critic and get response
    const response = await opts.client.session.prompt({
      path: { id: sessionId },
      body: {
        model,
        agent: "critic",
        parts: [
          { type: "text" as const, text: formatCriticPrompt(opts.context) },
        ],
      },
    });

    // Extract text from response parts
    const responseParts = response.data?.parts || [];
    const textParts = responseParts
      // biome-ignore lint/suspicious/noExplicitAny: SDK response types
      .filter((p: any) => p.type === "text" && p.text)
      // biome-ignore lint/suspicious/noExplicitAny: SDK response types
      .map((p: any) => p.text)
      .join("\n");

    return parseCriticResponse(textParts);
  } catch (err) {
    // If session became invalid, clear it so next call creates a new one
    pluginLogger.error({
      msg: "Critic session error, will recreate on next call",
      sessionId,
      error: err instanceof Error ? err.message : String(err),
    });
    criticState.criticSessionId = null;
    criticSessionIds.delete(sessionId);
    return parseCriticResponse("");
  }
}

/**
 * Inject feedback into the actor's session without triggering a response.
 */
async function injectFeedbackIntoSession(
  // biome-ignore lint/suspicious/noExplicitAny: SDK client types are complex
  client: any,
  sessionId: string,
  feedback: CriticFeedback
): Promise<void> {
  const formatted = formatFeedbackForActor(feedback);

  await client.session.prompt({
    path: { id: sessionId },
    body: {
      noReply: true,
      parts: [{ type: "text" as const, text: formatted }],
    },
  });
}

/**
 * Critic state tracking.
 * - inProgress: Mutex to prevent nested critic invocations
 * - sessionEnabled: Per-session toggle, controlled via /critic command
 * - criticSessionId: Reusable critic session for the current actor session
 * - actorSessionId: Track which actor session the critic session belongs to
 */
const criticState: {
  inProgress: boolean;
  sessionEnabled: boolean;
  criticSessionId: string | null;
  actorSessionId: string | null;
} = {
  inProgress: false,
  sessionEnabled: false, // Default to disabled, user must run /critic on
  criticSessionId: null,
  actorSessionId: null,
};

/**
 * Determines if critic analysis should run for this tool execution.
 *
 * Note: We cannot reliably detect which agent is active via the SDK,
 * so critic runs based on configuration alone. Users should set
 * critic.enabled = false when not using the actor agent.
 */
function shouldRunCritic(
  // biome-ignore lint/suspicious/noExplicitAny: SDK client types
  client: any,
  toolName: string,
  sessionId: string | undefined
): boolean {
  if (!client) {
    return false;
  }
  if (!sessionId) {
    return false;
  }
  if (shouldSkipCritic(toolName, sessionId)) {
    return false;
  }
  return true;
}

/**
 * Check if critic should be skipped for this tool execution.
 * Assumes client and sessionId have already been validated.
 */
function shouldSkipCritic(toolName: string, sessionId: string): boolean {
  const criticConfig = getCriticConfig();

  // Config must have critic enabled (global kill switch)
  if (!criticConfig.enabled) {
    return true;
  }

  // Session must have critic enabled via /critic on command
  if (!criticState.sessionEnabled) {
    return true;
  }

  // Skip if a critic is already running (prevents infinite loops)
  if (criticState.inProgress) {
    return true;
  }

  if (criticSessionIds.has(sessionId)) {
    return true;
  }

  if (!CRITIC_TRIGGER_TOOLS.has(toolName)) {
    return true;
  }

  return false;
}

/**
 * Options for the critic hook handler.
 */
type CriticHookOptions = {
  toolName: string;
  inputArgs: Record<string, unknown>;
  toolOutput: string;
  sessionId: string | undefined;
  projectName: string;
  workdir: string;
  currentSessionId: string | undefined;
  currentModel: { providerID: string; modelID: string } | undefined;
  conversationBuffer: ConversationBuffer;
  // biome-ignore lint/suspicious/noExplicitAny: SDK client types
  client: any;
};

/**
 * Handle critic analysis for a tool execution.
 * Extracted to reduce complexity in the main hook.
 */
async function handleCriticAnalysis(opts: CriticHookOptions): Promise<void> {
  // Set flag to prevent nested critic invocations
  criticState.inProgress = true;

  try {
    await performCriticAnalysis(opts);
  } finally {
    criticState.inProgress = false;
  }
}

/**
 * Perform the actual critic analysis.
 * Separated to allow wrapping with the inProgress flag.
 */
async function performCriticAnalysis(opts: CriticHookOptions): Promise<void> {
  const criticConfig = getCriticConfig();

  // Get conversation context
  const conversationContext = getRecentContext(opts.conversationBuffer);

  // Get diff if this was a file edit
  let diff: string | undefined;
  if (opts.toolName === "edit" || opts.toolName === "write") {
    const filePath =
      (opts.inputArgs.filePath as string) || (opts.inputArgs.file as string);
    if (filePath) {
      const fileDiff = await getFileDiff(filePath, opts.workdir);
      if (fileDiff) {
        diff = fileDiff;
      }
    }
  }

  // Build context for critic
  const criticContext: CriticContext = {
    action: {
      tool: opts.toolName,
      args: opts.inputArgs,
      result: opts.toolOutput,
    },
    diff,
    conversationContext,
    project: {
      name: opts.projectName,
      workdir: opts.workdir,
    },
  };

  // Invoke critic (blocking) - use sessionId from tool execution as actor session
  const actorSessionId = opts.sessionId || opts.currentSessionId || "unknown";
  const feedback = await invokeCritic({
    client: opts.client,
    context: criticContext,
    config: criticConfig,
    actorSessionId,
    actorModel: opts.currentModel,
  });

  // Store feedback in memory with role: "critic"
  await memoryStore.append({
    content: JSON.stringify(feedback),
    project: opts.projectName,
    tags: ["critic", "feedback", feedback.verdict],
    source: "actor-critic",
    sessionId: opts.currentSessionId,
    role: "critic",
  });

  // Inject formatted feedback into actor's session
  if (opts.sessionId) {
    await injectFeedbackIntoSession(opts.client, opts.sessionId, feedback);
  }

  pluginLogger.info({
    msg: "Critic analysis complete",
    verdict: feedback.verdict,
    confidence: feedback.confidence,
    issueCount: feedback.issues.length,
  });
}

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

export const CodeLoopsMemory: Plugin = async ({
  project,
  directory,
  client,
}) => {
  // Use project id or extract from directory path
  const workdir = project?.worktree || directory;
  const projectName = project?.id
    ? extractProjectName(project.worktree)
    : extractProjectName(directory);
  let currentSessionId: string | undefined;
  // Note: currentModel tracking would require intercepting model selection events
  // For now, critic uses its configured model or falls back to a default
  const currentModel: { providerID: string; modelID: string } | undefined =
    undefined;

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

      critic_toggle: tool({
        description:
          "Toggle the actor-critic feedback system on or off for this session. Use 'on' to enable critic feedback after each action, 'off' to disable, or 'status' to check current state.",
        args: {
          action: tool.schema
            .enum(["on", "off", "status"])
            .describe(
              "Action: 'on' to enable, 'off' to disable, 'status' to check"
            ),
        },
        async execute(args: { action: "on" | "off" | "status" }) {
          if (args.action === "status") {
            const configEnabled = getCriticConfig().enabled;
            // Use await to satisfy async requirement
            await Promise.resolve();
            return JSON.stringify({
              sessionEnabled: criticState.sessionEnabled,
              configEnabled,
              active: criticState.sessionEnabled && configEnabled,
              message: criticState.sessionEnabled
                ? "Critic is ON for this session"
                : "Critic is OFF for this session",
            });
          }

          criticState.sessionEnabled = args.action === "on";

          // Clean up critic session when disabled
          if (!criticState.sessionEnabled && client) {
            await cleanupCriticSession(client);
          }

          pluginLogger.info({
            msg: "Critic toggled",
            enabled: criticState.sessionEnabled,
          });

          return criticState.sessionEnabled
            ? "Critic enabled. I will now receive feedback after each action."
            : "Critic disabled. No feedback will be provided.";
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

    // -------------------------------------------------------------------------
    // Tool Execution Hooks (Actor-Critic System)
    // -------------------------------------------------------------------------

    "tool.execute.after": async (input, output) => {
      const toolName = input.tool as string;
      // biome-ignore lint/suspicious/noExplicitAny: Plugin hook types
      const inputAny = input as any;
      // Note: OpenCode uses sessionID (capital ID), not sessionId
      const sessionId = (inputAny.sessionID ?? inputAny.sessionId) as
        | string
        | undefined;

      // Check all conditions for running critic
      if (!shouldRunCritic(client, toolName, sessionId)) {
        return;
      }

      pluginLogger.info({
        msg: "Triggering critic analysis",
        tool: toolName,
        sessionId,
      });

      // biome-ignore lint/suspicious/noExplicitAny: Plugin hook types vary
      const inputArgs = ((input as any).args || {}) as Record<string, unknown>;
      const toolOutput = output.output || "";

      try {
        await handleCriticAnalysis({
          toolName,
          inputArgs,
          toolOutput,
          sessionId,
          projectName,
          workdir,
          currentSessionId,
          currentModel,
          conversationBuffer,
          client,
        });
      } catch (err) {
        pluginLogger.error({
          msg: "Critic analysis failed",
          error: err instanceof Error ? err.message : String(err),
        });
      }
    },
  };
};

export default CodeLoopsMemory;
