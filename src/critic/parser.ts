/**
 * Critic response parsing utilities
 */

import type { CriticFeedback } from "./types.ts";

// Regex patterns for JSON extraction
const MARKDOWN_CODE_BLOCK_REGEX = /```(?:json)?\s*([\s\S]*?)```/g;
const JSON_OBJECT_REGEX = /\{[\s\S]*\}/;

/**
 * Try to parse a string as JSON and validate it has expected critic fields.
 */
export function tryParseCriticJson(text: string): CriticFeedback | null {
  try {
    const parsed = JSON.parse(text);
    // Validate it looks like a critic response (has verdict field)
    if (parsed && typeof parsed === "object" && "verdict" in parsed) {
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
  } catch {
    // Not valid JSON, return null
  }
  return null;
}

/**
 * Extract JSON from markdown code blocks.
 */
export function extractFromCodeBlocks(text: string): string[] {
  const blocks: string[] = [];
  const matches = text.matchAll(MARKDOWN_CODE_BLOCK_REGEX);
  for (const match of matches) {
    if (match[1]) {
      blocks.push(match[1].trim());
    }
  }
  return blocks;
}

/**
 * Process a single character in JSON extraction.
 * Returns updated state and optional end position if object is complete.
 */
function processJsonChar(
  char: string,
  state: { depth: number; inString: boolean; isEscaped: boolean }
): { endFound: boolean } {
  if (state.isEscaped) {
    state.isEscaped = false;
    return { endFound: false };
  }

  if (char === "\\") {
    state.isEscaped = true;
    return { endFound: false };
  }

  if (char === '"') {
    state.inString = !state.inString;
    return { endFound: false };
  }

  if (state.inString) {
    return { endFound: false };
  }

  if (char === "{") {
    state.depth += 1;
  } else if (char === "}") {
    state.depth -= 1;
    if (state.depth === 0) {
      return { endFound: true };
    }
  }

  return { endFound: false };
}

/**
 * Extract JSON object from text using balanced brace matching.
 * More reliable than greedy regex for nested objects.
 */
export function extractJsonObject(text: string): string | null {
  const start = text.indexOf("{");
  if (start === -1) {
    return null;
  }

  const state = { depth: 0, inString: false, isEscaped: false };

  for (let i = start; i < text.length; i += 1) {
    const { endFound } = processJsonChar(text[i], state);
    if (endFound) {
      return text.slice(start, i + 1);
    }
  }

  return null;
}

/**
 * Try parsing with a specific extraction strategy.
 */
function tryParseWithExtraction(
  text: string,
  extractor: (t: string) => string | null
): CriticFeedback | null {
  const extracted = extractor(text);
  if (extracted) {
    return tryParseCriticJson(extracted);
  }
  return null;
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

/**
 * Parse the critic's JSON response.
 * Tries multiple extraction strategies:
 * 1. Direct JSON parse (if response is pure JSON)
 * 2. Extract from markdown code blocks
 * 3. Extract JSON object with balanced brace matching
 * Falls back to a default "proceed" response if all strategies fail.
 */
export function parseCriticResponse(
  responseText: string,
  logger?: { warn: (msg: unknown) => void }
): CriticFeedback {
  if (!responseText?.trim()) {
    logger?.warn({ msg: "Empty critic response" });
    return createDefaultFeedback("Empty critic response");
  }

  // Strategy 1: Try direct parse (response might be pure JSON)
  const directParse = tryParseCriticJson(responseText.trim());
  if (directParse) {
    return directParse;
  }

  // Strategy 2: Try extracting from markdown code blocks
  const codeBlocks = extractFromCodeBlocks(responseText);
  for (const block of codeBlocks) {
    const fromBlock = tryParseCriticJson(block);
    if (fromBlock) {
      return fromBlock;
    }
    // Try extracting JSON object from within the code block
    const fromBlockExtract = tryParseWithExtraction(block, extractJsonObject);
    if (fromBlockExtract) {
      return fromBlockExtract;
    }
  }

  // Strategy 3: Try balanced brace extraction from full text
  const fromBalanced = tryParseWithExtraction(responseText, extractJsonObject);
  if (fromBalanced) {
    return fromBalanced;
  }

  // Strategy 4: Fallback to greedy regex (last resort)
  const greedyMatch = responseText.match(JSON_OBJECT_REGEX);
  if (greedyMatch) {
    const fromGreedy = tryParseCriticJson(greedyMatch[0]);
    if (fromGreedy) {
      return fromGreedy;
    }
  }

  logger?.warn({
    msg: "Failed to parse critic response after all strategies",
    responsePreview: responseText.slice(0, 300),
  });

  return createDefaultFeedback("Unable to parse critic response");
}
