/**
 * TODO Comment Extraction from Git Diffs
 *
 * Extracts TODO comments from git diff output for bd (beads) integration.
 */

// -----------------------------------------------------------------------------
// Regex Constants
// -----------------------------------------------------------------------------

// Match TODO comments that appear after common comment prefixes
// Supported prefixes:
//   // (JS/TS/C/Go)    # (Python/Shell/YAML)    /* (C-style block)
//   /** (JSDoc)        * (block comment line)   -- (SQL)
//   ;; (Lisp/Clojure)  <!-- (HTML/XML)
// This prevents false positives from TODOs in prose or variable names
const TODO_COMMENT_REGEX =
  /^[\s]*(\/\/|#|\/\*\*?|\*|--|;{1,2}|<!--)\s*TODO[:\s]+(.+)/i;

// Detect if a TODO is already tracked by bd (has [bd-xxx] suffix)
const BD_TRACKED_REGEX = /\[bd-[a-z0-9]+\]/i;

// Hunk header regex: @@ -old,count +new,count @@
const HUNK_HEADER_REGEX = /^@@\s+-\d+(?:,\d+)?\s+\+(\d+)(?:,\d+)?\s+@@/;

// -----------------------------------------------------------------------------
// Types
// -----------------------------------------------------------------------------

export type ExtractedTodo = {
  lineNumber: number;
  todoText: string;
  fullLine: string;
};

// -----------------------------------------------------------------------------
// Core Function
// -----------------------------------------------------------------------------

/**
 * Extract TODO comments from a git diff.
 * Only looks at added lines (starting with +) and skips already-tracked TODOs.
 */
export function extractTodosFromDiff(diff: string): ExtractedTodo[] {
  const todos: ExtractedTodo[] = [];
  const lines = diff.split("\n");

  // Track approximate line number from diff hunk headers
  let currentLineNumber = 0;

  for (const line of lines) {
    // Parse hunk header to get line number: @@ -old,count +new,count @@
    const hunkMatch = line.match(HUNK_HEADER_REGEX);
    if (hunkMatch) {
      currentLineNumber = Number.parseInt(hunkMatch[1], 10) - 1;
      continue;
    }

    // Only process added lines (start with +, but not +++ header)
    if (line.startsWith("+") && !line.startsWith("+++")) {
      currentLineNumber += 1;
      const content = line.slice(1); // Remove the + prefix

      // Check for TODO pattern (must be in a comment, not prose)
      const todoMatch = content.match(TODO_COMMENT_REGEX);
      if (todoMatch) {
        // Skip if already tracked by bd
        if (BD_TRACKED_REGEX.test(content)) {
          continue;
        }

        todos.push({
          lineNumber: currentLineNumber,
          todoText: todoMatch[2].trim(), // Group 2 is the TODO text (group 1 is comment prefix)
          fullLine: content.trim(),
        });
      }
    } else if (line.startsWith(" ")) {
      // Context line (unchanged)
      currentLineNumber += 1;
    }
    // Lines starting with - are deletions, don't increment line number
  }

  return todos;
}
