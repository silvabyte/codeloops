/**
 * Critic module types
 */

/**
 * Configuration for the critic system.
 */
export type CriticConfig = {
  /** Model in provider/model format (null = use actor's model) */
  model: string | undefined;
  /** Whether critic is enabled */
  enabled: boolean;
};

/**
 * Structured feedback from the critic agent.
 */
export type CriticFeedback = {
  verdict: "proceed" | "revise" | "stop";
  confidence: number;
  issues: string[];
  suggestions: string[];
  context: string;
  reasoning: string;
};

/**
 * Context provided to the critic for analysis.
 *
 * Note: conversationContext was removed because the conversation buffer
 * captures streaming/partial message updates that the critic misinterprets
 * as UI rendering issues, causing hallucinated feedback about "technical
 * problems" that don't exist. The critic should rely on tool args, result,
 * and git diff for context instead.
 */
export type CriticContext = {
  /** The action being analyzed */
  action: {
    /** Tool name that was executed */
    tool: string;
    /** Arguments passed to the tool */
    args: Record<string, unknown>;
    /** Result/output from the tool */
    result: string;
  };
  /** Git diff of changes (if applicable) */
  diff?: string;
  /** Project information */
  project: {
    /** Project name */
    name: string;
    /** Working directory path */
    workdir: string;
  };
};
