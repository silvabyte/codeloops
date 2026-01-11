/**
 * Critic module - Provides structured feedback for code actions
 *
 * This barrel file provides a clean public API for the critic module.
 * The re-exports are intentional and acceptable here as:
 * 1. It's a single-level re-export (not deeply nested)
 * 2. All exports are actively used by plugin/index.ts
 * 3. It provides a stable API boundary for the module
 */

// biome-ignore lint/performance/noBarrelFile: Intentional public API for critic module
export {
  formatCriticPrompt,
  formatFeedbackForActor,
} from "./feedback.ts";

// Export parser functions
export {
  createDefaultFeedback,
  extractFromCodeBlocks,
  extractJsonObject,
  parseCriticResponse,
  tryParseCriticJson,
} from "./parser.ts";
// Export session management functions and types
export {
  cleanupAllCriticSessions,
  cleanupCriticSession,
  getOrCreateCriticSession,
  type InvokeCriticOptions,
  invokeCritic,
  isCriticSession,
} from "./session.ts";
// Export all types
export type {
  CriticConfig,
  CriticContext,
  CriticFeedback,
} from "./types.ts";
