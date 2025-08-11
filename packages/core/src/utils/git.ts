import { execa } from 'execa';
import { to } from 'await-to-js';
import type { CodeLoopsLogger } from '../logger.js';

/**
 * Git operations utility for CodeLoops
 */

/**
 * Generate a complete git diff for all changes in the repository.
 * This captures both staged and unstaged changes.
 * Gracefully fails and returns empty string if git is not available.
 */
export async function getGitDiff(logger: CodeLoopsLogger): Promise<string> {
  const diffs: string[] = [];

  // Get staged changes
  const [stagedError, stagedResult] = await to(
    execa('git', ['diff', '--cached'], {
      reject: false,
    }),
  );

  if (stagedError || stagedResult.exitCode !== 0) {
    logger.debug({ error: stagedError }, 'Failed to get staged git diff');
  } else if (stagedResult.stdout.trim()) {
    diffs.push(`--- Staged Changes ---\n${stagedResult.stdout}`);
  }

  // Get unstaged changes
  const [unstagedError, unstagedResult] = await to(
    execa('git', ['diff'], {
      reject: false,
    }),
  );

  if (unstagedError || unstagedResult.exitCode !== 0) {
    logger.debug({ error: unstagedError }, 'Failed to get unstaged git diff');
  } else if (unstagedResult.stdout.trim()) {
    diffs.push(`--- Unstaged Changes ---\n${unstagedResult.stdout}`);
  }

  // Get untracked files
  const [untrackedError, untrackedResult] = await to(
    execa('git', ['ls-files', '--others', '--exclude-standard'], {
      reject: false,
    }),
  );

  if (untrackedError || untrackedResult.exitCode !== 0) {
    logger.debug({ error: untrackedError }, 'Failed to get untracked files');
  } else if (untrackedResult.stdout.trim()) {
    const untrackedFiles = untrackedResult.stdout.trim().split('\n');
    diffs.push(`--- Untracked Files ---\n${untrackedFiles.join('\n')}`);
  }

  return diffs.join('\n\n');
}
