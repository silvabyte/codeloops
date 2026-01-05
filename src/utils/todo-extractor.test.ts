import { describe, expect, it } from "vitest";
import { extractTodosFromDiff } from "./todo-extractor.ts";

describe("extractTodosFromDiff", () => {
  describe("basic extraction", () => {
    it("extracts single TODO from added line", () => {
      const diff = `@@ -1,3 +1,4 @@
 existing line
+// TODO: implement this feature
 another line`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(1);
      expect(todos[0]).toEqual({
        lineNumber: 2,
        todoText: "implement this feature",
        fullLine: "// TODO: implement this feature",
      });
    });

    it("extracts multiple TODOs from same diff", () => {
      const diff = `@@ -1,2 +1,5 @@
 existing line
+// TODO: first task
+// some other code
+// TODO: second task
 end`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(2);
      expect(todos[0].todoText).toBe("first task");
      expect(todos[0].lineNumber).toBe(2);
      expect(todos[1].todoText).toBe("second task");
      expect(todos[1].lineNumber).toBe(4);
    });

    it("extracts TODO with colon separator", () => {
      const diff = `@@ -1,1 +1,2 @@
 line
+// TODO: with colon`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(1);
      expect(todos[0].todoText).toBe("with colon");
    });

    it("extracts TODO with space separator", () => {
      const diff = `@@ -1,1 +1,2 @@
 line
+// TODO fix this bug`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(1);
      expect(todos[0].todoText).toBe("fix this bug");
    });

    it("extracts TODO with both colon and space", () => {
      const diff = `@@ -1,1 +1,2 @@
 line
+// TODO: fix this: important`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(1);
      expect(todos[0].todoText).toBe("fix this: important");
    });
  });

  describe("comment syntax variations", () => {
    it("extracts TODO from // comment (JavaScript/TypeScript)", () => {
      const diff = `@@ -1,1 +1,2 @@
 code
+// TODO: js style`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(1);
      expect(todos[0].fullLine).toBe("// TODO: js style");
    });

    it("extracts TODO from # comment (Python/Shell/YAML)", () => {
      const diff = `@@ -1,1 +1,2 @@
 code
+# TODO: python style`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(1);
      expect(todos[0].fullLine).toBe("# TODO: python style");
    });

    it("extracts TODO from /* comment (C-style block)", () => {
      const diff = `@@ -1,1 +1,2 @@
 code
+/* TODO: c style block comment */`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(1);
      expect(todos[0].todoText).toBe("c style block comment */");
    });

    it("extracts TODO from /** comment (JSDoc)", () => {
      const diff = `@@ -1,1 +1,2 @@
 code
+/** TODO: jsdoc style */`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(1);
      expect(todos[0].todoText).toBe("jsdoc style */");
    });

    it("extracts TODO from <!-- comment (HTML)", () => {
      const diff = `@@ -1,1 +1,2 @@
 <div>
+<!-- TODO: html comment -->`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(1);
      expect(todos[0].todoText).toBe("html comment -->");
    });

    it("extracts TODO from -- comment (SQL)", () => {
      const diff = `@@ -1,1 +1,2 @@
 SELECT *
+-- TODO: optimize this query`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(1);
      expect(todos[0].fullLine).toBe("-- TODO: optimize this query");
    });

    it("extracts TODO from inline code with leading spaces", () => {
      const diff = `@@ -1,1 +1,2 @@
 function foo() {
+    // TODO: handle edge case`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(1);
      expect(todos[0].todoText).toBe("handle edge case");
      expect(todos[0].fullLine).toBe("// TODO: handle edge case");
    });
  });

  describe("filtering behavior", () => {
    it("skips TODOs already tracked with [bd-xxx]", () => {
      const diff = `@@ -1,1 +1,3 @@
 code
+// TODO: not tracked yet
+// TODO: already tracked [bd-abc123]`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(1);
      expect(todos[0].todoText).toBe("not tracked yet");
    });

    it("skips TODOs with [BD-XXX] (case insensitive)", () => {
      const diff = `@@ -1,1 +1,2 @@
 code
+// TODO: tracked [BD-ABC123]`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(0);
    });

    it("only processes added lines (+ prefix)", () => {
      const diff = `@@ -1,3 +1,4 @@
 // TODO: context line (not added)
+// TODO: added line
-// TODO: removed line
 end`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(1);
      expect(todos[0].todoText).toBe("added line");
    });

    it("ignores removed lines (- prefix)", () => {
      const diff = `@@ -1,2 +1,1 @@
-// TODO: this was removed
 remaining line`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(0);
    });

    it("ignores context lines (space prefix)", () => {
      const diff = `@@ -1,3 +1,4 @@
 // TODO: existing todo (context)
 other line
+new line without todo
 // TODO: another existing (context)`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(0);
    });

    it("ignores +++ file header", () => {
      const diff = `--- a/file.ts
+++ b/file.ts
@@ -1,1 +1,2 @@
 code
+// TODO: real todo`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(1);
      expect(todos[0].todoText).toBe("real todo");
    });

    it("ignores --- file header", () => {
      const diff = `--- a/TODO: not a real todo
+++ b/file.ts
@@ -1,1 +1,2 @@
 code
+// TODO: real one`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(1);
    });
  });

  describe("line number tracking", () => {
    it("correctly parses hunk header for starting line", () => {
      const diff = `@@ -10,3 +15,4 @@
 context
+// TODO: at line 16`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(1);
      expect(todos[0].lineNumber).toBe(16);
    });

    it("tracks line numbers across context lines", () => {
      const diff = `@@ -1,5 +1,6 @@
 line 1
 line 2
 line 3
+// TODO: this should be line 4
 line 5`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(1);
      expect(todos[0].lineNumber).toBe(4);
    });

    it("handles multiple hunks", () => {
      const diff = `@@ -1,2 +1,3 @@
 first section
+// TODO: first hunk
 end first
@@ -10,2 +11,3 @@
 second section
+// TODO: second hunk
 end second`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(2);
      expect(todos[0].lineNumber).toBe(2);
      expect(todos[0].todoText).toBe("first hunk");
      expect(todos[1].lineNumber).toBe(12);
      expect(todos[1].todoText).toBe("second hunk");
    });

    it("handles hunk with no count (single line change)", () => {
      const diff = `@@ -5 +5,2 @@
 existing
+// TODO: single line hunk`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(1);
      expect(todos[0].lineNumber).toBe(6);
    });

    it("correctly counts lines with mixed additions and deletions", () => {
      const diff = `@@ -1,5 +1,5 @@
 line 1
-old line 2
+new line 2
 line 3
-old line 4
+// TODO: replaced line 4
 line 5`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(1);
      expect(todos[0].lineNumber).toBe(4);
    });

    it("handles consecutive additions", () => {
      const diff = `@@ -1,1 +1,4 @@
 existing
+// TODO: line 2
+// TODO: line 3
+// TODO: line 4`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(3);
      expect(todos[0].lineNumber).toBe(2);
      expect(todos[1].lineNumber).toBe(3);
      expect(todos[2].lineNumber).toBe(4);
    });
  });

  describe("edge cases", () => {
    it("returns empty array for empty diff", () => {
      const todos = extractTodosFromDiff("");
      expect(todos).toEqual([]);
    });

    it("returns empty array for diff with no TODOs", () => {
      const diff = `@@ -1,2 +1,3 @@
 regular code
+more regular code
 end`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toEqual([]);
    });

    it("ignores TODO with no content after separator", () => {
      const diff = `@@ -1,1 +1,2 @@
 code
+// TODO:`;

      const todos = extractTodosFromDiff(diff);

      // Regex requires at least one character after TODO: so empty TODOs are ignored
      expect(todos).toHaveLength(0);
    });

    it("is case insensitive for TODO keyword", () => {
      const diff = `@@ -1,1 +1,4 @@
 code
+// TODO: uppercase
+// todo: lowercase
+// Todo: mixed case`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(3);
      expect(todos[0].todoText).toBe("uppercase");
      expect(todos[1].todoText).toBe("lowercase");
      expect(todos[2].todoText).toBe("mixed case");
    });

    it("handles TODO with special characters in text", () => {
      const diff = `@@ -1,1 +1,2 @@
 code
+// TODO: handle "quotes" and 'apostrophes' & <special> chars!`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(1);
      expect(todos[0].todoText).toBe(
        `handle "quotes" and 'apostrophes' & <special> chars!`
      );
    });

    it("handles TODO with unicode characters", () => {
      const diff = `@@ -1,1 +1,2 @@
 code
+// TODO: support internationalization`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(1);
      expect(todos[0].todoText).toBe("support internationalization");
    });

    it("handles TODO with numbers and symbols", () => {
      const diff = `@@ -1,1 +1,2 @@
 code
+// TODO: fix bug #123 (priority: P0)`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(1);
      expect(todos[0].todoText).toBe("fix bug #123 (priority: P0)");
    });

    it("handles very long TODO text", () => {
      const longText = "a".repeat(500);
      const diff = `@@ -1,1 +1,2 @@
 code
+// TODO: ${longText}`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(1);
      expect(todos[0].todoText).toBe(longText);
    });

    it("handles diff with only header lines", () => {
      const diff = `diff --git a/file.ts b/file.ts
index abc123..def456 100644
--- a/file.ts
+++ b/file.ts`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toEqual([]);
    });

    it("trims whitespace from fullLine", () => {
      const diff = `@@ -1,1 +1,2 @@
 code
+    // TODO: indented    `;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(1);
      expect(todos[0].fullLine).toBe("// TODO: indented");
    });

    it("trims whitespace from todoText", () => {
      const diff = `@@ -1,1 +1,2 @@
 code
+// TODO:    lots of spaces    `;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(1);
      expect(todos[0].todoText).toBe("lots of spaces");
    });
  });

  describe("bd tracking detection", () => {
    it("detects various bd ID formats", () => {
      const diff = `@@ -1,1 +1,5 @@
 code
+// TODO: tracked [bd-a1b2c3]
+// TODO: tracked [bd-xyz]
+// TODO: tracked [bd-123abc]
+// TODO: not tracked`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(1);
      expect(todos[0].todoText).toBe("not tracked");
    });

    it("does not match bd ID in wrong format", () => {
      const diff = `@@ -1,1 +1,3 @@
 code
+// TODO: this has bd-123 but not in brackets
+// TODO: this has [bd_123] wrong separator`;

      const todos = extractTodosFromDiff(diff);

      // Both should be extracted since they don't match [bd-xxx] pattern
      expect(todos).toHaveLength(2);
    });

    it("handles bd ID at end of line", () => {
      const diff = `@@ -1,1 +1,2 @@
 code
+// TODO: do something [bd-end]`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(0);
    });

    it("handles bd ID in middle of TODO text", () => {
      const diff = `@@ -1,1 +1,2 @@
 code
+// TODO: [bd-mid] do something else`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(0);
    });
  });

  describe("real-world diff scenarios", () => {
    it("handles typical TypeScript file diff", () => {
      const diff = `diff --git a/src/utils.ts b/src/utils.ts
index abc123..def456 100644
--- a/src/utils.ts
+++ b/src/utils.ts
@@ -15,6 +15,10 @@ export function processData(input: string): Result {
   const parsed = JSON.parse(input);
+  
+  // TODO: add input validation
+  // TODO: handle edge cases for empty objects [bd-existing]
+  
   return transform(parsed);
 }`;

      const todos = extractTodosFromDiff(diff);

      // Line counting: hunk starts at 15, context line is 15, empty added is 16, TODO is 17
      expect(todos).toHaveLength(1);
      expect(todos[0].todoText).toBe("add input validation");
      expect(todos[0].lineNumber).toBe(17);
    });

    it("handles Python file diff", () => {
      const diff = `@@ -1,5 +1,8 @@
 def main():
+    # TODO: refactor this function
     print("hello")
+    
+    # TODO: add error handling
     return 0`;

      const todos = extractTodosFromDiff(diff);

      expect(todos).toHaveLength(2);
      expect(todos[0].todoText).toBe("refactor this function");
      expect(todos[1].todoText).toBe("add error handling");
    });

    it("handles multi-file diff (only processes hunks)", () => {
      const diff = `diff --git a/file1.ts b/file1.ts
--- a/file1.ts
+++ b/file1.ts
@@ -1,2 +1,3 @@
 code
+// TODO: in file1
 end
diff --git a/file2.ts b/file2.ts
--- a/file2.ts
+++ b/file2.ts
@@ -1,2 +1,3 @@
 code
+// TODO: in file2
 end`;

      const todos = extractTodosFromDiff(diff);

      // Note: This extracts from both files in the diff
      // In practice, we call this per-file, but the function handles multi-file diffs
      expect(todos).toHaveLength(2);
    });
  });
});
