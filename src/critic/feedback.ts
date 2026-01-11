/**
 * Critic feedback formatting utilities
 */

import type { CriticContext, CriticFeedback } from "./types.ts";

/**
 * Format the context into a prompt for the critic agent.
 */
export function formatCriticPrompt(ctx: CriticContext): string {
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
 * Format critic feedback for injection into the actor's context.
 */
export function formatFeedbackForActor(feedback: CriticFeedback): string {
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

  parts.push("---", "");

  return parts.join("\n");
}

/**
 * Create a default feedback response for error cases.
 */
export function createDefaultFeedback(reason: string): CriticFeedback {
  return {
    verdict: "proceed",
    confidence: 0.0,
    issues: [],
    suggestions: [],
    context: reason,
    reasoning: "Unable to analyze due to parsing error",
  };
}
