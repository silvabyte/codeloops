import { describe, expect, it } from "vitest";
import {
  createDefaultFeedback,
  extractFromCodeBlocks,
  extractJsonObject,
  parseCriticResponse,
  tryParseCriticJson,
} from "./parser.ts";

describe("tryParseCriticJson", () => {
  it("parses valid critic JSON", () => {
    const json = JSON.stringify({
      verdict: "proceed",
      confidence: 0.9,
      issues: ["issue1"],
      suggestions: ["suggestion1"],
      context: "some context",
      reasoning: "some reasoning",
    });

    const result = tryParseCriticJson(json);

    expect(result).toEqual({
      verdict: "proceed",
      confidence: 0.9,
      issues: ["issue1"],
      suggestions: ["suggestion1"],
      context: "some context",
      reasoning: "some reasoning",
    });
  });

  it("returns null for invalid JSON", () => {
    expect(tryParseCriticJson("not json")).toBeNull();
    expect(tryParseCriticJson("{invalid}")).toBeNull();
  });

  it("returns null for JSON without verdict field", () => {
    expect(tryParseCriticJson('{"foo": "bar"}')).toBeNull();
  });

  it("provides defaults for missing optional fields", () => {
    const result = tryParseCriticJson('{"verdict": "revise"}');

    expect(result).toEqual({
      verdict: "revise",
      confidence: 0.5,
      issues: [],
      suggestions: [],
      context: "",
      reasoning: "",
    });
  });
});

describe("extractFromCodeBlocks", () => {
  it("extracts content from markdown code blocks", () => {
    const text = '```json\n{"verdict": "proceed"}\n```';
    expect(extractFromCodeBlocks(text)).toEqual(['{"verdict": "proceed"}']);
  });

  it("extracts from code blocks without language specifier", () => {
    const text = '```\n{"verdict": "stop"}\n```';
    expect(extractFromCodeBlocks(text)).toEqual(['{"verdict": "stop"}']);
  });

  it("extracts multiple code blocks", () => {
    const text = "```\nblock1\n```\ntext\n```\nblock2\n```";
    expect(extractFromCodeBlocks(text)).toEqual(["block1", "block2"]);
  });

  it("returns empty array when no code blocks", () => {
    expect(extractFromCodeBlocks("no code blocks here")).toEqual([]);
  });
});

describe("extractJsonObject", () => {
  it("extracts simple JSON object", () => {
    const text = 'prefix {"key": "value"} suffix';
    expect(extractJsonObject(text)).toBe('{"key": "value"}');
  });

  it("handles nested objects", () => {
    const text = '{"outer": {"inner": "value"}}';
    expect(extractJsonObject(text)).toBe('{"outer": {"inner": "value"}}');
  });

  it("handles strings with braces", () => {
    const text = '{"message": "use { and } carefully"}';
    expect(extractJsonObject(text)).toBe(
      '{"message": "use { and } carefully"}'
    );
  });

  it("handles escaped quotes in strings", () => {
    const text = '{"message": "say \\"hello\\""}';
    expect(extractJsonObject(text)).toBe('{"message": "say \\"hello\\""}');
  });

  it("returns null when no object found", () => {
    expect(extractJsonObject("no json here")).toBeNull();
    expect(extractJsonObject("")).toBeNull();
  });

  it("handles complex nested structures", () => {
    const json = '{"a": {"b": {"c": 1}}, "d": [1, 2, {"e": 3}]}';
    const text = `Some text ${json} more text`;
    expect(extractJsonObject(text)).toBe(json);
  });
});

describe("parseCriticResponse", () => {
  it("parses direct JSON response", () => {
    const response = JSON.stringify({
      verdict: "proceed",
      confidence: 0.85,
      issues: [],
      suggestions: ["Consider adding tests"],
      context: "",
      reasoning: "Code looks good",
    });

    const result = parseCriticResponse(response);

    expect(result.verdict).toBe("proceed");
    expect(result.confidence).toBe(0.85);
    expect(result.suggestions).toContain("Consider adding tests");
  });

  it("extracts JSON from markdown code blocks", () => {
    const response = `
Here is my analysis:

\`\`\`json
{
  "verdict": "revise",
  "confidence": 0.7,
  "issues": ["Missing error handling"],
  "suggestions": [],
  "context": "",
  "reasoning": "Add try-catch"
}
\`\`\`
`;

    const result = parseCriticResponse(response);

    expect(result.verdict).toBe("revise");
    expect(result.issues).toContain("Missing error handling");
  });

  it("handles embedded JSON in prose", () => {
    const response = `
Let me analyze this action.

{"verdict": "stop", "confidence": 0.95, "issues": ["Critical bug"], "suggestions": [], "context": "Stop immediately", "reasoning": "Bug found"}

Please address the above.
`;

    const result = parseCriticResponse(response);

    expect(result.verdict).toBe("stop");
    expect(result.issues).toContain("Critical bug");
  });

  it("returns default feedback for empty response", () => {
    const result = parseCriticResponse("");

    expect(result.verdict).toBe("proceed");
    expect(result.confidence).toBe(0);
    expect(result.context).toContain("Empty");
  });

  it("returns default feedback for unparseable response", () => {
    const result = parseCriticResponse("Just some text with no JSON at all");

    expect(result.verdict).toBe("proceed");
    expect(result.confidence).toBe(0);
    expect(result.context).toContain("Unable to parse");
  });
});

describe("createDefaultFeedback", () => {
  it("creates feedback with given reason", () => {
    const result = createDefaultFeedback("Test reason");

    expect(result).toEqual({
      verdict: "proceed",
      confidence: 0,
      issues: [],
      suggestions: [],
      context: "Test reason",
      reasoning: "Unable to analyze due to parsing error",
    });
  });
});
