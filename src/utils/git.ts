import { exec } from "node:child_process";
import { promisify } from "node:util";
import type { CodeLoopsLogger } from "../logger.ts";

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
