/**
 * Critic session management utilities
 */

import { formatCriticPrompt } from "./feedback.ts";
import { parseCriticResponse } from "./parser.ts";
import type { CriticConfig, CriticContext, CriticFeedback } from "./types.ts";

/**
 * State tracking for critic sessions.
 */
type CriticState = {
  criticSessionId: string | null;
  actorSessionId: string | null;
};

// Global state for critic sessions
const criticState: CriticState = {
  criticSessionId: null,
  actorSessionId: null,
};

// Track all created critic sessions for cleanup
const criticSessionIds = new Set<string>();

/**
 * Check if a session belongs to the critic.
 */
export function isCriticSession(sessionId: string): boolean {
  return criticSessionIds.has(sessionId);
}

// Logger interface for session operations
type SessionLogger = {
  info: (msg: unknown) => void;
  error: (msg: unknown) => void;
};

// Extended logger for invoke operations
type InvokeLogger = SessionLogger & {
  warn: (msg: unknown) => void;
};

// OpenCode client interface (minimal type for session operations)
type OpenCodeClient = {
  session: {
    create: (opts: { body: { title: string } }) => Promise<{
      data?: { id?: string };
    }>;
    delete: (opts: { path: { id: string } }) => Promise<unknown>;
    prompt: (opts: {
      path: { id: string };
      body: {
        model?: { providerID: string; modelID: string };
        agent: string;
        parts: Array<{ type: "text"; text: string }>;
      };
    }) => Promise<{
      data?: { parts?: Array<{ type?: string; text?: string }> };
    }>;
  };
};

/**
 * Get or create a critic session for the current actor session.
 * Reuses existing session if available, creates new one if needed.
 */
export async function getOrCreateCriticSession(
  client: OpenCodeClient,
  actorSessionId: string,
  logger?: SessionLogger
): Promise<string | null> {
  // If actor session changed, clean up old critic session
  if (
    criticState.actorSessionId &&
    criticState.actorSessionId !== actorSessionId
  ) {
    await cleanupCriticSession(client, logger);
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
    logger?.error({ msg: "Failed to create critic session" });
    return null;
  }

  // Track the new session
  criticState.criticSessionId = sessionId;
  criticState.actorSessionId = actorSessionId;
  criticSessionIds.add(sessionId);

  logger?.info({
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
export async function cleanupCriticSession(
  client: OpenCodeClient,
  logger?: SessionLogger
): Promise<void> {
  if (!criticState.criticSessionId) {
    return;
  }

  const sessionId = criticState.criticSessionId;

  try {
    await client.session.delete({ path: { id: sessionId } });
    logger?.info({
      msg: "Cleaned up critic session",
      sessionId,
    });
  } catch (err) {
    logger?.error({
      msg: "Failed to delete critic session",
      sessionId,
      error: err instanceof Error ? err.message : String(err),
    });
  }

  // Clear state regardless of deletion success
  criticState.criticSessionId = null;
  criticState.actorSessionId = null;
  criticSessionIds.delete(sessionId);
}

/**
 * Clean up all critic sessions on shutdown.
 */
export async function cleanupAllCriticSessions(
  client: OpenCodeClient,
  logger?: SessionLogger
): Promise<void> {
  if (criticSessionIds.size === 0) {
    return;
  }

  logger?.info({
    msg: "Cleaning up all critic sessions",
    count: criticSessionIds.size,
  });

  const cleanupPromises = Array.from(criticSessionIds).map(
    async (sessionId) => {
      try {
        await client.session.delete({ path: { id: sessionId } });
      } catch (err) {
        logger?.error({
          msg: "Failed to delete critic session during cleanup",
          sessionId,
          error: err instanceof Error ? err.message : String(err),
        });
      }
    }
  );

  await Promise.all(cleanupPromises);
  criticSessionIds.clear();
  criticState.criticSessionId = null;
  criticState.actorSessionId = null;
}

/**
 * Options for invoking the critic.
 */
export type InvokeCriticOptions = {
  context: CriticContext;
  client: OpenCodeClient;
  actorSessionId: string;
  actorModel?: { providerID: string; modelID: string };
  config: CriticConfig;
};

/**
 * Invoke the critic agent to analyze an action.
 * Uses a reusable critic session per actor session to:
 * 1. Reduce session creation overhead
 * 2. Allow critic to build context across multiple analyses
 * 3. Enable continuity in feedback within an actor session
 */
export async function invokeCritic(
  opts: InvokeCriticOptions,
  logger?: InvokeLogger
): Promise<CriticFeedback> {
  // Get or create a reusable critic session
  const sessionId = await getOrCreateCriticSession(
    opts.client,
    opts.actorSessionId,
    logger
  );
  if (!sessionId) {
    return parseCriticResponse("", logger);
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
      .filter(
        (p): p is { type: string; text: string } =>
          p.type === "text" && !!p.text
      )
      .map((p) => p.text)
      .join("\n");

    return parseCriticResponse(textParts, logger);
  } catch (err) {
    // If session became invalid, clear it so next call creates a new one
    logger?.error({
      msg: "Critic session error, will recreate on next call",
      sessionId,
      error: err instanceof Error ? err.message : String(err),
    });
    criticState.criticSessionId = null;
    criticSessionIds.delete(sessionId);
    return parseCriticResponse("", logger);
  }
}
