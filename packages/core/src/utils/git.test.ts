import { describe, it, expect, beforeEach, mock, spyOn } from 'bun:test';
import { getGitDiff } from './git.js';
import { createLogger } from '../logger.js';
import * as execaModule from 'execa';

describe('getGitDiff', () => {
  const mockLogger = createLogger({ withFile: false, withDevStdout: false });
  let execaSpy: ReturnType<typeof spyOn>;

  beforeEach(() => {
    // Reset the spy before each test
    if (execaSpy) {
      execaSpy.mockRestore();
    }
    execaSpy = spyOn(execaModule, 'execa');
  });

  describe('happy path scenarios', () => {
    it('returns formatted diff with all sections when all git commands succeed', async () => {
      // Mock staged changes
      execaSpy.mockResolvedValueOnce({
        stdout: 'diff --git a/file1.ts b/file1.ts\n+added line',
        stderr: '',
        exitCode: 0,
        failed: false,
        command: '',
        escapedCommand: '',
        timedOut: false,
        isCanceled: false,
        killed: false,
      });

      // Mock unstaged changes
      execaSpy.mockResolvedValueOnce({
        stdout: 'diff --git a/file2.ts b/file2.ts\n-removed line',
        stderr: '',
        exitCode: 0,
        failed: false,
        command: '',
        escapedCommand: '',
        timedOut: false,
        isCanceled: false,
        killed: false,
      });

      // Mock untracked files
      execaSpy.mockResolvedValueOnce({
        stdout: 'newfile.ts\nanother.ts',
        stderr: '',
        exitCode: 0,
        failed: false,
        command: '',
        escapedCommand: '',
        timedOut: false,
        isCanceled: false,
        killed: false,
      });

      const result = await getGitDiff(mockLogger);

      expect(result).toBe(
        '--- Staged Changes ---\ndiff --git a/file1.ts b/file1.ts\n+added line\n\n' +
          '--- Unstaged Changes ---\ndiff --git a/file2.ts b/file2.ts\n-removed line\n\n' +
          '--- Untracked Files ---\nnewfile.ts\nanother.ts',
      );

      expect(execaSpy).toHaveBeenCalledTimes(3);
      expect(execaSpy).toHaveBeenCalledWith('git', ['diff', '--cached'], { reject: false });
      expect(execaSpy).toHaveBeenCalledWith('git', ['diff'], { reject: false });
      expect(execaSpy).toHaveBeenCalledWith('git', ['ls-files', '--others', '--exclude-standard'], {
        reject: false,
      });
    });

    it('returns only staged changes when only staged files exist', async () => {
      // Mock staged changes
      execaSpy.mockResolvedValueOnce({
        stdout: 'diff --git a/staged.ts b/staged.ts\n+staged change',
        stderr: '',
        exitCode: 0,
        failed: false,
        command: '',
        escapedCommand: '',
        timedOut: false,
        isCanceled: false,
        killed: false,
      });

      // Mock empty unstaged changes
      execaSpy.mockResolvedValueOnce({
        stdout: '',
        stderr: '',
        exitCode: 0,
        failed: false,
        command: '',
        escapedCommand: '',
        timedOut: false,
        isCanceled: false,
        killed: false,
      });

      // Mock empty untracked files
      execaSpy.mockResolvedValueOnce({
        stdout: '',
        stderr: '',
        exitCode: 0,
        failed: false,
        command: '',
        escapedCommand: '',
        timedOut: false,
        isCanceled: false,
        killed: false,
      });

      const result = await getGitDiff(mockLogger);

      expect(result).toBe(
        '--- Staged Changes ---\ndiff --git a/staged.ts b/staged.ts\n+staged change',
      );
    });

    it('returns only unstaged changes when only unstaged files exist', async () => {
      // Mock empty staged changes
      execaSpy.mockResolvedValueOnce({
        stdout: '',
        stderr: '',
        exitCode: 0,
        failed: false,
        command: '',
        escapedCommand: '',
        timedOut: false,
        isCanceled: false,
        killed: false,
      });

      // Mock unstaged changes
      execaSpy.mockResolvedValueOnce({
        stdout: 'diff --git a/unstaged.ts b/unstaged.ts\n-unstaged change',
        stderr: '',
        exitCode: 0,
        failed: false,
        command: '',
        escapedCommand: '',
        timedOut: false,
        isCanceled: false,
        killed: false,
      });

      // Mock empty untracked files
      execaSpy.mockResolvedValueOnce({
        stdout: '',
        stderr: '',
        exitCode: 0,
        failed: false,
        command: '',
        escapedCommand: '',
        timedOut: false,
        isCanceled: false,
        killed: false,
      });

      const result = await getGitDiff(mockLogger);

      expect(result).toBe(
        '--- Unstaged Changes ---\ndiff --git a/unstaged.ts b/unstaged.ts\n-unstaged change',
      );
    });

    it('returns only untracked files when only untracked files exist', async () => {
      // Mock empty staged changes
      execaSpy.mockResolvedValueOnce({
        stdout: '',
        stderr: '',
        exitCode: 0,
        failed: false,
        command: '',
        escapedCommand: '',
        timedOut: false,
        isCanceled: false,
        killed: false,
      });

      // Mock empty unstaged changes
      execaSpy.mockResolvedValueOnce({
        stdout: '',
        stderr: '',
        exitCode: 0,
        failed: false,
        command: '',
        escapedCommand: '',
        timedOut: false,
        isCanceled: false,
        killed: false,
      });

      // Mock untracked files
      execaSpy.mockResolvedValueOnce({
        stdout: 'untracked1.ts\nuntracked2.ts',
        stderr: '',
        exitCode: 0,
        failed: false,
        command: '',
        escapedCommand: '',
        timedOut: false,
        isCanceled: false,
        killed: false,
      });

      const result = await getGitDiff(mockLogger);

      expect(result).toBe('--- Untracked Files ---\nuntracked1.ts\nuntracked2.ts');
    });

    it('returns empty string when no changes exist', async () => {
      // Mock all empty results
      const emptyResult = {
        stdout: '',
        stderr: '',
        exitCode: 0,
        failed: false,
        command: '',
        escapedCommand: '',
        timedOut: false,
        isCanceled: false,
        killed: false,
      };

      execaSpy.mockResolvedValue(emptyResult);

      const result = await getGitDiff(mockLogger);

      expect(result).toBe('');
      expect(execaSpy).toHaveBeenCalledTimes(3);
    });

    it('handles whitespace-only output correctly', async () => {
      // Mock results with whitespace
      execaSpy.mockResolvedValueOnce({
        stdout: '   \n  \t  ',
        stderr: '',
        exitCode: 0,
        failed: false,
        command: '',
        escapedCommand: '',
        timedOut: false,
        isCanceled: false,
        killed: false,
      });

      execaSpy.mockResolvedValueOnce({
        stdout: '\n\n',
        stderr: '',
        exitCode: 0,
        failed: false,
        command: '',
        escapedCommand: '',
        timedOut: false,
        isCanceled: false,
        killed: false,
      });

      execaSpy.mockResolvedValueOnce({
        stdout: '  ',
        stderr: '',
        exitCode: 0,
        failed: false,
        command: '',
        escapedCommand: '',
        timedOut: false,
        isCanceled: false,
        killed: false,
      });

      const result = await getGitDiff(mockLogger);

      expect(result).toBe('');
    });
  });
});
