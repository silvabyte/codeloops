import { exec } from "node:child_process";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createLogger } from "../core/logger.ts";
import { getFileDiff, getGitDiff, getMultiFileDiff } from "./git.ts";

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

describe("getFileDiff", () => {
  const mockExec = vi.mocked(exec);

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("returns diff from HEAD when available", async () => {
    mockExec.mockImplementationOnce((_cmd, options, callback) => {
      const cb = callback || options;
      (cb as any)(null, {
        stdout: "diff --git a/file.ts b/file.ts\n+added line",
        stderr: "",
      });
      return {} as ReturnType<typeof exec>;
    });

    const result = await getFileDiff("file.ts", "/project");

    expect(result).toBe("diff --git a/file.ts b/file.ts\n+added line");
    expect(mockExec).toHaveBeenCalledWith(
      'git diff HEAD -- "file.ts"',
      { cwd: "/project" },
      expect.any(Function)
    );
  });

  it("returns staged diff when HEAD diff is empty", async () => {
    mockExec
      .mockImplementationOnce((_cmd, options, callback) => {
        const cb = callback || options;
        (cb as any)(null, { stdout: "", stderr: "" });
        return {} as ReturnType<typeof exec>;
      })
      .mockImplementationOnce((_cmd, options, callback) => {
        const cb = callback || options;
        (cb as any)(null, {
          stdout: "diff --git a/new.ts b/new.ts\n+new file",
          stderr: "",
        });
        return {} as ReturnType<typeof exec>;
      });

    const result = await getFileDiff("new.ts", "/project");

    expect(result).toBe("diff --git a/new.ts b/new.ts\n+new file");
    expect(mockExec).toHaveBeenCalledTimes(2);
  });

  it("returns null when no diff available", async () => {
    mockExec.mockImplementation((_cmd, options, callback) => {
      const cb = callback || options;
      (cb as any)(null, { stdout: "", stderr: "" });
      return {} as ReturnType<typeof exec>;
    });

    const result = await getFileDiff("unchanged.ts", "/project");

    expect(result).toBeNull();
  });

  it("shows content for untracked files", async () => {
    mockExec
      .mockImplementationOnce((_cmd, options, callback) => {
        const cb = callback || options;
        (cb as any)(null, { stdout: "", stderr: "" });
        return {} as ReturnType<typeof exec>;
      })
      .mockImplementationOnce((_cmd, options, callback) => {
        const cb = callback || options;
        (cb as any)(null, { stdout: "", stderr: "" });
        return {} as ReturnType<typeof exec>;
      })
      .mockImplementationOnce((_cmd, options, callback) => {
        const cb = callback || options;
        (cb as any)(null, { stdout: "", stderr: "" });
        return {} as ReturnType<typeof exec>;
      })
      .mockImplementationOnce((_cmd, options, callback) => {
        const cb = callback || options;
        (cb as any)(null, { stdout: "?? untracked.ts", stderr: "" });
        return {} as ReturnType<typeof exec>;
      })
      .mockImplementationOnce((_cmd, options, callback) => {
        const cb = callback || options;
        (cb as any)(null, { stdout: "console.log('new file');", stderr: "" });
        return {} as ReturnType<typeof exec>;
      });

    const result = await getFileDiff("untracked.ts", "/project");

    expect(result).toBe("[New untracked file]\nconsole.log('new file');");
    expect(mockExec).toHaveBeenCalledTimes(5);
  });
});

describe("getMultiFileDiff", () => {
  const mockExec = vi.mocked(exec);

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("combines diffs from multiple files", async () => {
    mockExec
      .mockImplementationOnce((_cmd, options, callback) => {
        const cb = callback || options;
        (cb as any)(null, { stdout: "diff for file1", stderr: "" });
        return {} as ReturnType<typeof exec>;
      })
      .mockImplementationOnce((_cmd, options, callback) => {
        const cb = callback || options;
        (cb as any)(null, { stdout: "diff for file2", stderr: "" });
        return {} as ReturnType<typeof exec>;
      });

    const result = await getMultiFileDiff(["file1.ts", "file2.ts"], "/project");

    expect(result).toBe(
      "### file1.ts\ndiff for file1\n\n### file2.ts\ndiff for file2"
    );
  });

  it("returns null when no files have diffs", async () => {
    mockExec.mockImplementation((_cmd, options, callback) => {
      const cb = callback || options;
      (cb as any)(null, { stdout: "", stderr: "" });
      return {} as ReturnType<typeof exec>;
    });

    const result = await getMultiFileDiff(["file1.ts", "file2.ts"], "/project");

    expect(result).toBeNull();
  });
});
