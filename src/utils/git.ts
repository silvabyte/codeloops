import { exec } from "node:child_process";
import { promisify } from "node:util";
import type { CodeLoopsLogger } from "../core/logger.ts";

const execAsync = promisify(exec);

/**
 * Git operations utility for CodeLoops
 */

/**
 * Helper to safely execute a git command
 */
async function execGit(
  command: string,
  logger: CodeLoopsLogger
): Promise<string> {
  try {
    const { stdout } = await execAsync(command);
    return stdout.trim();
  } catch (error) {
    logger.debug({ error, command }, "Git command failed");
    return "";
  }
}

/**
 * Helper to execute a git command with a specific working directory.
 * Returns null on error instead of empty string.
 */
async function execGitInDir(
  command: string,
  workdir: string
): Promise<string | null> {
  try {
    const { stdout } = await execAsync(command, { cwd: workdir });
    return stdout.trim() || null;
  } catch {
    return null;
  }
}

/**
 * Generate a complete git diff for all changes in the repository.
 * This captures both staged and unstaged changes.
 * Gracefully fails and returns empty string if git is not available.
 */
export async function getGitDiff(logger: CodeLoopsLogger): Promise<string> {
  const diffs: string[] = [];

  // Get staged changes
  const staged = await execGit("git diff --cached", logger);
  if (staged) {
    diffs.push(`--- Staged Changes ---\n${staged}`);
  }

  // Get unstaged changes
  const unstaged = await execGit("git diff", logger);
  if (unstaged) {
    diffs.push(`--- Unstaged Changes ---\n${unstaged}`);
  }

  // Get untracked files
  const untracked = await execGit(
    "git ls-files --others --exclude-standard",
    logger
  );
  if (untracked) {
    diffs.push(`--- Untracked Files ---\n${untracked}`);
  }

  return diffs.join("\n\n");
}

/**
 * Get git diff for a specific file, trying multiple strategies:
 * 1. diff HEAD (staged + unstaged vs last commit)
 * 2. diff --cached (staged only, for new files)
 * 3. diff (unstaged only)
 * 4. show file content for untracked files
 *
 * @param filePath - Path to the file (relative or absolute)
 * @param workdir - Working directory for git commands
 * @returns The diff string, or null if no diff available
 */
export async function getFileDiff(
  filePath: string,
  workdir: string
): Promise<string | null> {
  if (!filePath?.trim()) {
    return null;
  }

  // Strategy 1: Try diff against HEAD (most common case)
  const headDiff = await execGitInDir(
    `git diff HEAD -- "${filePath}"`,
    workdir
  );
  if (headDiff) {
    return headDiff;
  }

  // Strategy 2: Try staged diff (for newly added files)
  const stagedDiff = await execGitInDir(
    `git diff --cached -- "${filePath}"`,
    workdir
  );
  if (stagedDiff) {
    return stagedDiff;
  }

  // Strategy 3: Try unstaged diff
  const unstagedDiff = await execGitInDir(`git diff -- "${filePath}"`, workdir);
  if (unstagedDiff) {
    return unstagedDiff;
  }

  // Strategy 4: Check if file is untracked and show content preview
  const status = await execGitInDir(
    `git status --porcelain -- "${filePath}"`,
    workdir
  );
  if (status?.startsWith("??")) {
    // Untracked file - show first 100 lines as context
    const content = await execGitInDir(`head -100 "${filePath}"`, workdir);
    if (content) {
      return `[New untracked file]\n${content}`;
    }
  }

  return null;
}

/**
 * Get diffs for multiple files.
 * Useful for multi-file edit operations.
 *
 * @param filePaths - Array of file paths
 * @param workdir - Working directory for git commands
 * @returns Combined diff string with file headers, or null if no diffs
 */
export async function getMultiFileDiff(
  filePaths: string[],
  workdir: string
): Promise<string | null> {
  const diffs: string[] = [];

  for (const filePath of filePaths) {
    const diff = await getFileDiff(filePath, workdir);
    if (diff) {
      diffs.push(`### ${filePath}\n${diff}`);
    }
  }

  return diffs.length > 0 ? diffs.join("\n\n") : null;
}
