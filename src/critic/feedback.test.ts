import { describe, expect, it } from "vitest";
import {
  createDefaultFeedback,
  formatCriticPrompt,
  formatFeedbackForActor,
} from "./feedback.ts";
import type { CriticContext, CriticFeedback } from "./types.ts";

describe("formatCriticPrompt", () => {
  it("formats basic action context", () => {
    const ctx: CriticContext = {
      action: {
        tool: "edit",
        args: { file: "test.ts", content: "hello" },
        result: "File edited successfully",
      },
      project: { name: "test-project", workdir: "/tmp/test" },
    };

    const prompt = formatCriticPrompt(ctx);

    expect(prompt).toContain("## Action Taken");
    expect(prompt).toContain("**Tool:** edit");
    expect(prompt).toContain('"file": "test.ts"');
    expect(prompt).toContain("File edited successfully");
    expect(prompt).toContain("## Your Task");
  });

  it("includes diff when provided", () => {
    const ctx: CriticContext = {
      action: {
        tool: "edit",
        args: { file: "test.ts" },
        result: "OK",
      },
      diff: "+added line\n-removed line",
      project: { name: "test-project", workdir: "/tmp/test" },
    };

    const prompt = formatCriticPrompt(ctx);

    expect(prompt).toContain("## File Changes");
    expect(prompt).toContain("+added line");
    expect(prompt).toContain("-removed line");
  });

  it("truncates long results", () => {
    const longResult = "x".repeat(3000);
    const ctx: CriticContext = {
      action: {
        tool: "read",
        args: {},
        result: longResult,
      },
      project: { name: "test-project", workdir: "/tmp/test" },
    };

    const prompt = formatCriticPrompt(ctx);

    // Result should be truncated to 2000 chars
    expect(prompt.length).toBeLessThan(longResult.length);
  });

  it("truncates long diffs", () => {
    const longDiff = "d".repeat(5000);
    const ctx: CriticContext = {
      action: {
        tool: "edit",
        args: {},
        result: "OK",
      },
      diff: longDiff,
      project: { name: "test-project", workdir: "/tmp/test" },
    };

    const prompt = formatCriticPrompt(ctx);

    // Diff should be truncated to 3000 chars
    expect(prompt.length).toBeLessThan(longDiff.length);
  });
});

describe("formatFeedbackForActor", () => {
  it("formats proceed verdict", () => {
    const feedback: CriticFeedback = {
      verdict: "proceed",
      confidence: 0.9,
      issues: [],
      suggestions: [],
      context: "",
      reasoning: "",
    };

    const formatted = formatFeedbackForActor(feedback);

    expect(formatted).toContain("[PROCEED]");
    expect(formatted).toContain("**Verdict:** PROCEED");
    expect(formatted).toContain("90%");
  });

  it("formats revise verdict with issues", () => {
    const feedback: CriticFeedback = {
      verdict: "revise",
      confidence: 0.75,
      issues: ["Missing null check", "No error handling"],
      suggestions: [],
      context: "",
      reasoning: "",
    };

    const formatted = formatFeedbackForActor(feedback);

    expect(formatted).toContain("[REVISE]");
    expect(formatted).toContain("### Issues");
    expect(formatted).toContain("- Missing null check");
    expect(formatted).toContain("- No error handling");
  });

  it("formats stop verdict with suggestions", () => {
    const feedback: CriticFeedback = {
      verdict: "stop",
      confidence: 0.95,
      issues: [],
      suggestions: ["Reconsider approach", "Check requirements"],
      context: "",
      reasoning: "",
    };

    const formatted = formatFeedbackForActor(feedback);

    expect(formatted).toContain("[STOP]");
    expect(formatted).toContain("### Suggestions");
    expect(formatted).toContain("- Reconsider approach");
    expect(formatted).toContain("- Check requirements");
  });

  it("includes context when provided", () => {
    const feedback: CriticFeedback = {
      verdict: "proceed",
      confidence: 0.8,
      issues: [],
      suggestions: [],
      context: "Additional context here",
      reasoning: "",
    };

    const formatted = formatFeedbackForActor(feedback);

    expect(formatted).toContain("### Context");
    expect(formatted).toContain("Additional context here");
  });

  it("includes reasoning when provided", () => {
    const feedback: CriticFeedback = {
      verdict: "proceed",
      confidence: 0.8,
      issues: [],
      suggestions: [],
      context: "",
      reasoning: "My detailed reasoning",
    };

    const formatted = formatFeedbackForActor(feedback);

    expect(formatted).toContain("### Reasoning");
    expect(formatted).toContain("My detailed reasoning");
  });

  it("formats complete feedback with all fields", () => {
    const feedback: CriticFeedback = {
      verdict: "revise",
      confidence: 0.65,
      issues: ["Issue 1", "Issue 2"],
      suggestions: ["Suggestion 1"],
      context: "Some context",
      reasoning: "Because reasons",
    };

    const formatted = formatFeedbackForActor(feedback);

    expect(formatted).toContain("## Critic Feedback [REVISE]");
    expect(formatted).toContain("65%");
    expect(formatted).toContain("### Issues");
    expect(formatted).toContain("### Suggestions");
    expect(formatted).toContain("### Context");
    expect(formatted).toContain("### Reasoning");
  });
});

describe("createDefaultFeedback", () => {
  it("creates default feedback with reason", () => {
    const feedback = createDefaultFeedback("Something went wrong");

    expect(feedback.verdict).toBe("proceed");
    expect(feedback.confidence).toBe(0);
    expect(feedback.issues).toEqual([]);
    expect(feedback.suggestions).toEqual([]);
    expect(feedback.context).toBe("Something went wrong");
    expect(feedback.reasoning).toContain("Unable to analyze");
  });
});
