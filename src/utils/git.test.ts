import { exec } from "node:child_process";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createLogger } from "../logger.ts";
import { getGitDiff } from "./git.ts";

// Mock child_process exec
vi.mock("node:child_process", () => ({
  exec: vi.fn(),
}));

describe("getGitDiff", () => {
  const mockLogger = createLogger({ withFile: false, withDevStdout: false });
  const mockExec = vi.mocked(exec);

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("happy path scenarios", () => {
    it("returns formatted diff with all sections when all git commands succeed", async () => {
      // Mock all three git commands in sequence
      mockExec
        .mockImplementationOnce((_cmd, callback) => {
          (
            callback as (
              error: Error | null,
              output: { stdout: string; stderr: string }
            ) => void
          )(null, {
            stdout: "diff --git a/file1.ts b/file1.ts\n+added line",
            stderr: "",
          });
          return {} as ReturnType<typeof exec>;
        })
        .mockImplementationOnce((_cmd, callback) => {
          (
            callback as (
              error: Error | null,
              output: { stdout: string; stderr: string }
            ) => void
          )(null, {
            stdout: "diff --git a/file2.ts b/file2.ts\n-removed line",
            stderr: "",
          });
          return {} as ReturnType<typeof exec>;
        })
        .mockImplementationOnce((_cmd, callback) => {
          (
            callback as (
              error: Error | null,
              output: { stdout: string; stderr: string }
            ) => void
          )(null, { stdout: "newfile.ts\nanother.ts", stderr: "" });
          return {} as ReturnType<typeof exec>;
        });

      const result = await getGitDiff(mockLogger);

      expect(result).toBe(
        "--- Staged Changes ---\ndiff --git a/file1.ts b/file1.ts\n+added line\n\n" +
          "--- Unstaged Changes ---\ndiff --git a/file2.ts b/file2.ts\n-removed line\n\n" +
          "--- Untracked Files ---\nnewfile.ts\nanother.ts"
      );

      expect(mockExec).toHaveBeenCalledTimes(3);
    });

    it("returns only staged changes when only staged files exist", async () => {
      mockExec
        .mockImplementationOnce((_cmd, callback) => {
          (
            callback as (
              error: Error | null,
              output: { stdout: string; stderr: string }
            ) => void
          )(null, {
            stdout: "diff --git a/staged.ts b/staged.ts\n+staged change",
            stderr: "",
          });
          return {} as ReturnType<typeof exec>;
        })
        .mockImplementationOnce((_cmd, callback) => {
          (
            callback as (
              error: Error | null,
              output: { stdout: string; stderr: string }
            ) => void
          )(null, { stdout: "", stderr: "" });
          return {} as ReturnType<typeof exec>;
        })
        .mockImplementationOnce((_cmd, callback) => {
          (
            callback as (
              error: Error | null,
              output: { stdout: string; stderr: string }
            ) => void
          )(null, { stdout: "", stderr: "" });
          return {} as ReturnType<typeof exec>;
        });

      const result = await getGitDiff(mockLogger);

      expect(result).toBe(
        "--- Staged Changes ---\ndiff --git a/staged.ts b/staged.ts\n+staged change"
      );
    });

    it("returns empty string when no changes exist", async () => {
      mockExec.mockImplementation((_cmd, callback) => {
        (
          callback as (
            error: Error | null,
            output: { stdout: string; stderr: string }
          ) => void
        )(null, { stdout: "", stderr: "" });
        return {} as ReturnType<typeof exec>;
      });

      const result = await getGitDiff(mockLogger);

      expect(result).toBe("");
      expect(mockExec).toHaveBeenCalledTimes(3);
    });

    it("handles errors gracefully", async () => {
      mockExec.mockImplementation((_cmd, callback) => {
        (
          callback as (
            error: Error | null,
            output: { stdout: string; stderr: string }
          ) => void
        )(new Error("git not found"), { stdout: "", stderr: "" });
        return {} as ReturnType<typeof exec>;
      });

      const result = await getGitDiff(mockLogger);

      expect(result).toBe("");
    });
  });
});
