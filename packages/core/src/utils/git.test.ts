import { describe, it, expect, vi, beforeEach } from 'vitest';
import { getGitDiff } from './git.js';
import { createLogger } from '../logger.js';
import type { Result } from 'execa';

// Mock execa
vi.mock('execa', () => ({
  execa: vi.fn(),
}));

import { execa } from 'execa';

describe('getGitDiff', () => {
  const mockLogger = createLogger({ withFile: false, withDevStdout: false });
  const mockExeca = vi.mocked(execa);

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('happy path scenarios', () => {
    it('returns formatted diff with all sections when all git commands succeed', async () => {
      // Mock staged changes
      mockExeca.mockResolvedValueOnce({
        stdout: 'diff --git a/file1.ts b/file1.ts\n+added line',
        stderr: '',
        exitCode: 0,
      } as Result);

      // Mock unstaged changes
      mockExeca.mockResolvedValueOnce({
        stdout: 'diff --git a/file2.ts b/file2.ts\n-removed line',
        stderr: '',
        exitCode: 0,
      } as Result);

      // Mock untracked files
      mockExeca.mockResolvedValueOnce({
        stdout: 'newfile.ts\nanother.ts',
        stderr: '',
        exitCode: 0,
      } as Result);

      const result = await getGitDiff(mockLogger);

      expect(result).toBe(
        '--- Staged Changes ---\ndiff --git a/file1.ts b/file1.ts\n+added line\n\n' +
          '--- Unstaged Changes ---\ndiff --git a/file2.ts b/file2.ts\n-removed line\n\n' +
          '--- Untracked Files ---\nnewfile.ts\nanother.ts',
      );

      expect(mockExeca).toHaveBeenCalledTimes(3);
      expect(mockExeca).toHaveBeenNthCalledWith(1, 'git', ['diff', '--cached'], { reject: false });
      expect(mockExeca).toHaveBeenNthCalledWith(2, 'git', ['diff'], { reject: false });
      expect(mockExeca).toHaveBeenNthCalledWith(
        3,
        'git',
        ['ls-files', '--others', '--exclude-standard'],
        { reject: false },
      );
    });

    it('returns only staged changes when only staged files exist', async () => {
      // Mock staged changes
      mockExeca.mockResolvedValueOnce({
        stdout: 'diff --git a/staged.ts b/staged.ts\n+staged change',
        stderr: '',
        exitCode: 0,
      } as Result);

      // Mock empty unstaged changes
      mockExeca.mockResolvedValueOnce({
        stdout: '',
        stderr: '',
        exitCode: 0,
      } as Result);

      // Mock empty untracked files
      mockExeca.mockResolvedValueOnce({
        stdout: '',
        stderr: '',
        exitCode: 0,
      } as Result);

      const result = await getGitDiff(mockLogger);

      expect(result).toBe(
        '--- Staged Changes ---\ndiff --git a/staged.ts b/staged.ts\n+staged change',
      );
    });

    it('returns only unstaged changes when only unstaged files exist', async () => {
      // Mock empty staged changes
      mockExeca.mockResolvedValueOnce({
        stdout: '',
        stderr: '',
        exitCode: 0,
      } as Result);

      // Mock unstaged changes
      mockExeca.mockResolvedValueOnce({
        stdout: 'diff --git a/unstaged.ts b/unstaged.ts\n-unstaged change',
        stderr: '',
        exitCode: 0,
      } as Result);

      // Mock empty untracked files
      mockExeca.mockResolvedValueOnce({
        stdout: '',
        stderr: '',
        exitCode: 0,
      } as Result);

      const result = await getGitDiff(mockLogger);

      expect(result).toBe(
        '--- Unstaged Changes ---\ndiff --git a/unstaged.ts b/unstaged.ts\n-unstaged change',
      );
    });

    it('returns only untracked files when only untracked files exist', async () => {
      // Mock empty staged changes
      mockExeca.mockResolvedValueOnce({
        stdout: '',
        stderr: '',
        exitCode: 0,
      } as Result);

      // Mock empty unstaged changes
      mockExeca.mockResolvedValueOnce({
        stdout: '',
        stderr: '',
        exitCode: 0,
      } as Result);

      // Mock untracked files
      mockExeca.mockResolvedValueOnce({
        stdout: 'untracked1.ts\nuntracked2.ts',
        stderr: '',
        exitCode: 0,
      } as Result);

      const result = await getGitDiff(mockLogger);

      expect(result).toBe('--- Untracked Files ---\nuntracked1.ts\nuntracked2.ts');
    });

    it('returns empty string when no changes exist', async () => {
      // Mock all empty results
      mockExeca.mockResolvedValue({
        stdout: '',
        stderr: '',
        exitCode: 0,
      } as Result);

      const result = await getGitDiff(mockLogger);

      expect(result).toBe('');
      expect(mockExeca).toHaveBeenCalledTimes(3);
    });

    it('handles whitespace-only output correctly', async () => {
      // Mock results with whitespace
      mockExeca.mockResolvedValueOnce({
        stdout: '   \n  \t  ',
        stderr: '',
        exitCode: 0,
      } as Result);

      mockExeca.mockResolvedValueOnce({
        stdout: '\n\n',
        stderr: '',
        exitCode: 0,
      } as Result);

      mockExeca.mockResolvedValueOnce({
        stdout: '  ',
        stderr: '',
        exitCode: 0,
      } as Result);

      const result = await getGitDiff(mockLogger);

      expect(result).toBe('');
    });
  });
});
