import fs from 'node:fs/promises';
import { lock, unlock } from 'proper-lockfile';
import * as fsSync from 'node:fs';
import path from 'node:path';
import { z } from 'zod';
import readline from 'node:readline';
import { getDataDir } from './config.ts';
import { CodeLoopsLogger } from './logger.ts';
import { nanoid } from 'nanoid';

// -----------------------------------------------------------------------------
// Interfaces & Schemas --------------------------------------------------------
// -----------------------------------------------------------------------------

export const MemoryEntrySchema = z.object({
  id: z.string(),
  project: z.string(),
  content: z.string(),
  tags: z.array(z.string()).optional(),
  createdAt: z.string().datetime(),
  sessionId: z.string().optional(),
  source: z.string().optional(),
});

export type MemoryEntry = z.infer<typeof MemoryEntrySchema>;

export interface DeletedMemoryEntry extends MemoryEntry {
  deletedAt: string;
  deletedReason?: string;
}

export interface QueryOptions {
  project?: string;
  tags?: string[];
  query?: string;
  limit?: number;
  sessionId?: string;
}

export interface AppendInput {
  content: string;
  project: string;
  tags?: string[];
  sessionId?: string;
  source?: string;
}

export interface MemoryStoreOptions {
  /** Custom data directory (for testing) */
  dataDir?: string;
}

// -----------------------------------------------------------------------------
// MemoryStore -----------------------------------------------------------------
// -----------------------------------------------------------------------------

export class MemoryStore {
  private logFilePath: string;
  private deletedLogFilePath: string;
  private logger: CodeLoopsLogger;

  constructor(logger: CodeLoopsLogger, options?: MemoryStoreOptions) {
    this.logger = logger;
    const dataDir = options?.dataDir ?? getDataDir();
    this.logFilePath = path.resolve(dataDir, 'memory.ndjson');
    this.deletedLogFilePath = path.resolve(dataDir, 'memory.deleted.ndjson');
  }

  async init(): Promise<void> {
    this.logger.info(`[MemoryStore] Initializing from ${this.logFilePath}`);
    await this.ensureLogFile();
  }

  private async ensureLogFile(): Promise<void> {
    if (!(await fs.stat(this.logFilePath).catch(() => null))) {
      this.logger.info(`[MemoryStore] Creating new log file at ${this.logFilePath}`);
      await fs.mkdir(path.dirname(this.logFilePath), { recursive: true });
      await fs.writeFile(this.logFilePath, '');
    }
  }

  private parseMemoryEntry(line: string): MemoryEntry | null {
    try {
      const parsed = JSON.parse(line);
      const validated = MemoryEntrySchema.parse(parsed);
      return validated;
    } catch (err) {
      this.logger.error({ err, line }, 'Invalid MemoryEntry');
      return null;
    }
  }

  /**
   * Append a new memory entry to the log
   */
  async append(input: AppendInput, retries = 3): Promise<MemoryEntry> {
    const entry: MemoryEntry = {
      id: nanoid(),
      project: input.project,
      content: input.content,
      tags: input.tags,
      createdAt: new Date().toISOString(),
      sessionId: input.sessionId,
      source: input.source,
    };

    const line = JSON.stringify(entry) + '\n';
    let err: Error | null = null;

    for (let attempt = 1; attempt <= retries; attempt++) {
      try {
        await lock(this.logFilePath, { retries: 0 });
        await fs.appendFile(this.logFilePath, line, 'utf8');
        return entry;
      } catch (e: unknown) {
        err = e as Error;
        this.logger.warn({ err, attempt }, `Retry ${attempt} failed appending entry`);
        if (attempt === retries) break;
        await new Promise((resolve) => setTimeout(resolve, 100 * attempt));
      } finally {
        try {
          await unlock(this.logFilePath);
        } catch (unlockErr) {
          this.logger.error({ err: unlockErr }, 'Failed to unlock file');
        }
      }
    }

    this.logger.error({ err }, 'Error appending entry after retries');
    throw err;
  }

  /**
   * Get a single entry by ID
   */
  async getById(id: string): Promise<MemoryEntry | undefined> {
    const fileStream = fsSync.createReadStream(this.logFilePath);
    const rl = readline.createInterface({ input: fileStream, crlfDelay: Infinity });
    try {
      for await (const line of rl) {
        const entry = this.parseMemoryEntry(line);
        if (entry?.id === id) {
          return entry;
        }
      }
      return undefined;
    } finally {
      rl.close();
      fileStream.close();
    }
  }

  /**
   * Query entries with filters
   */
  async query(options: QueryOptions = {}): Promise<MemoryEntry[]> {
    const { project, tags, query, limit, sessionId } = options;
    const entries: MemoryEntry[] = [];

    const fileStream = fsSync.createReadStream(this.logFilePath);
    const rl = readline.createInterface({ input: fileStream, crlfDelay: Infinity });

    try {
      for await (const line of rl) {
        const entry = this.parseMemoryEntry(line);
        if (!entry) continue;

        // Filter by project
        if (project && entry.project !== project) continue;

        // Filter by sessionId
        if (sessionId && entry.sessionId !== sessionId) continue;

        // Filter by tags (all specified tags must be present)
        if (tags && tags.length > 0) {
          if (!entry.tags || !tags.every((tag) => entry.tags?.includes(tag))) {
            continue;
          }
        }

        // Filter by query (simple text search in content)
        if (query && !entry.content.toLowerCase().includes(query.toLowerCase())) {
          continue;
        }

        entries.push(entry);

        // Keep only the most recent entries if limit is set
        if (limit && entries.length > limit) {
          entries.shift();
        }
      }

      return entries;
    } finally {
      rl.close();
      fileStream.close();
    }
  }

  /**
   * List all unique projects
   */
  async listProjects(): Promise<string[]> {
    const projects = new Set<string>();
    const fileStream = fsSync.createReadStream(this.logFilePath);
    const rl = readline.createInterface({ input: fileStream, crlfDelay: Infinity });

    try {
      for await (const line of rl) {
        const entry = this.parseMemoryEntry(line);
        if (entry?.project && !projects.has(entry.project)) {
          projects.add(entry.project);
        }
      }
      return Array.from(projects);
    } finally {
      rl.close();
      fileStream.close();
    }
  }

  /**
   * Soft delete an entry by moving it to the deleted log
   */
  async forget(id: string, reason?: string): Promise<DeletedMemoryEntry | undefined> {
    const entry = await this.getById(id);
    if (!entry) {
      return undefined;
    }

    const deletedEntry: DeletedMemoryEntry = {
      ...entry,
      deletedAt: new Date().toISOString(),
      deletedReason: reason,
    };

    // Append to deleted log
    const deletedLine = JSON.stringify(deletedEntry) + '\n';
    await fs.mkdir(path.dirname(this.deletedLogFilePath), { recursive: true });
    await fs.appendFile(this.deletedLogFilePath, deletedLine, 'utf8');

    // Rebuild main log without deleted entry
    await this.rebuildWithoutEntry(id);

    this.logger.info(`[MemoryStore] Soft deleted entry ${id}`);
    return deletedEntry;
  }

  private async rebuildWithoutEntry(entryId: string): Promise<void> {
    const entries: MemoryEntry[] = [];
    const fileStream = fsSync.createReadStream(this.logFilePath);
    const rl = readline.createInterface({ input: fileStream, crlfDelay: Infinity });

    try {
      for await (const line of rl) {
        const entry = this.parseMemoryEntry(line);
        if (entry && entry.id !== entryId) {
          entries.push(entry);
        }
      }
    } finally {
      rl.close();
      fileStream.close();
    }

    // Write back all non-deleted entries
    const tempPath = `${this.logFilePath}.tmp`;
    const lines = entries.map((entry) => JSON.stringify(entry) + '\n').join('');
    await fs.writeFile(tempPath, lines, 'utf8');

    // Atomic replace
    await fs.rename(tempPath, this.logFilePath);
  }
}
