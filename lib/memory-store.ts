import { createReadStream, existsSync } from "node:fs";
import fs from "node:fs/promises";
import path from "node:path";
import readline from "node:readline";
import { nanoid } from "nanoid";
import { lock, unlock } from "proper-lockfile";
import { getDataDir } from "./config.ts";
import { entryMatchesFilters } from "./filters.ts";
import type { CodeLoopsLogger } from "./logger.ts";
import {
  type AppendInput,
  type DeletedMemoryEntry,
  type MemoryEntry,
  MemoryEntrySchema,
  type MemoryStoreOptions,
  type QueryOptions,
} from "./types.ts";

/**
 * Parse a single line from the NDJSON log file into a MemoryEntry.
 * Returns null if the line is invalid.
 */
export function parseMemoryEntry(line: string): MemoryEntry | null {
  try {
    const parsed = JSON.parse(line);
    return MemoryEntrySchema.parse(parsed);
  } catch {
    return null;
  }
}

/**
 * MemoryStore provides persistent storage for memory entries.
 *
 * Uses NDJSON format for append-only logging with file locking
 * for concurrent access safety.
 */
export class MemoryStore {
  private readonly logFilePath: string;
  private readonly deletedLogFilePath: string;
  private readonly logger: CodeLoopsLogger;

  constructor(logger: CodeLoopsLogger, options?: MemoryStoreOptions) {
    this.logger = logger;
    const dataDir = options?.dataDir ?? getDataDir();
    this.logFilePath = path.resolve(dataDir, "memory.ndjson");
    this.deletedLogFilePath = path.resolve(dataDir, "memory.deleted.ndjson");
  }

  /**
   * Initialize the memory store, ensuring the log file exists.
   */
  async init(): Promise<void> {
    this.logger.info(`[MemoryStore] Initializing from ${this.logFilePath}`);
    await this.ensureLogFile();
  }

  private async ensureLogFile(): Promise<void> {
    if (!(await fs.stat(this.logFilePath).catch(() => null))) {
      this.logger.info(
        `[MemoryStore] Creating new log file at ${this.logFilePath}`
      );
      await fs.mkdir(path.dirname(this.logFilePath), { recursive: true });
      await fs.writeFile(this.logFilePath, "");
    }
  }

  /**
   * Append a new memory entry to the log.
   * Includes retry logic for handling concurrent access.
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

    const line = `${JSON.stringify(entry)}\n`;
    let err: Error | null = null;

    for (let attempt = 1; attempt <= retries; attempt++) {
      try {
        await lock(this.logFilePath, { retries: 0 });
        await fs.appendFile(this.logFilePath, line, "utf8");
        return entry;
      } catch (e: unknown) {
        err = e as Error;
        this.logger.warn(
          { err, attempt },
          `Retry ${attempt} failed appending entry`
        );
        if (attempt === retries) {
          break;
        }
        await new Promise((resolve) => setTimeout(resolve, 100 * attempt));
      } finally {
        try {
          await unlock(this.logFilePath);
        } catch (unlockErr) {
          this.logger.error({ err: unlockErr }, "Failed to unlock file");
        }
      }
    }

    this.logger.error({ err }, "Error appending entry after retries");
    throw err;
  }

  /**
   * Get a single entry by ID.
   */
  async getById(id: string): Promise<MemoryEntry | undefined> {
    if (!existsSync(this.logFilePath)) {
      return;
    }

    const fileStream = createReadStream(this.logFilePath);
    const rl = readline.createInterface({
      input: fileStream,
      crlfDelay: Number.POSITIVE_INFINITY,
    });

    try {
      for await (const line of rl) {
        const entry = parseMemoryEntry(line);
        if (entry?.id === id) {
          return entry;
        }
      }
      return;
    } finally {
      rl.close();
      fileStream.close();
    }
  }

  /**
   * Query entries with filters.
   * Returns the most recent entries matching the criteria.
   */
  async query(options: QueryOptions = {}): Promise<MemoryEntry[]> {
    if (!existsSync(this.logFilePath)) {
      return [];
    }

    const { limit } = options;
    const entries: MemoryEntry[] = [];

    const fileStream = createReadStream(this.logFilePath);
    const rl = readline.createInterface({
      input: fileStream,
      crlfDelay: Number.POSITIVE_INFINITY,
    });

    try {
      for await (const line of rl) {
        const entry = parseMemoryEntry(line);
        if (entry && entryMatchesFilters(entry, options)) {
          entries.push(entry);
          // Keep only the most recent entries if limit is set
          if (limit && entries.length > limit) {
            entries.shift();
          }
        }
      }

      return entries;
    } finally {
      rl.close();
      fileStream.close();
    }
  }

  /**
   * List all unique projects.
   */
  async listProjects(): Promise<string[]> {
    if (!existsSync(this.logFilePath)) {
      return [];
    }

    const projects = new Set<string>();
    const fileStream = createReadStream(this.logFilePath);
    const rl = readline.createInterface({
      input: fileStream,
      crlfDelay: Number.POSITIVE_INFINITY,
    });

    try {
      for await (const line of rl) {
        const entry = parseMemoryEntry(line);
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
   * Soft delete an entry by moving it to the deleted log.
   */
  async forget(
    id: string,
    reason?: string
  ): Promise<DeletedMemoryEntry | undefined> {
    const entry = await this.getById(id);
    if (!entry) {
      return;
    }

    const deletedEntry: DeletedMemoryEntry = {
      ...entry,
      deletedAt: new Date().toISOString(),
      deletedReason: reason,
    };

    // Append to deleted log
    const deletedLine = `${JSON.stringify(deletedEntry)}\n`;
    await fs.mkdir(path.dirname(this.deletedLogFilePath), { recursive: true });
    await fs.appendFile(this.deletedLogFilePath, deletedLine, "utf8");

    // Rebuild main log without deleted entry
    await this.rebuildWithoutEntry(id);

    this.logger.info(`[MemoryStore] Soft deleted entry ${id}`);
    return deletedEntry;
  }

  private async rebuildWithoutEntry(entryId: string): Promise<void> {
    const entries: MemoryEntry[] = [];
    const fileStream = createReadStream(this.logFilePath);
    const rl = readline.createInterface({
      input: fileStream,
      crlfDelay: Number.POSITIVE_INFINITY,
    });

    try {
      for await (const line of rl) {
        const entry = parseMemoryEntry(line);
        if (entry && entry.id !== entryId) {
          entries.push(entry);
        }
      }
    } finally {
      rl.close();
      fileStream.close();
    }

    // Write back all non-deleted entries atomically
    const tempPath = `${this.logFilePath}.tmp`;
    const lines = entries.map((entry) => `${JSON.stringify(entry)}\n`).join("");
    await fs.writeFile(tempPath, lines, "utf8");
    await fs.rename(tempPath, this.logFilePath);
  }
}

// -----------------------------------------------------------------------------
// Standalone Functions (for plugin use without class instantiation)
// -----------------------------------------------------------------------------

/**
 * Create standalone memory store functions that don't require a class instance.
 * Useful for the plugin where we want simpler function-based API.
 */
export function createMemoryStoreFunctions(
  logger: CodeLoopsLogger,
  options?: MemoryStoreOptions
) {
  const store = new MemoryStore(logger, options);

  return {
    init: () => store.init(),
    append: (input: AppendInput) => store.append(input),
    query: (opts?: QueryOptions) => store.query(opts),
    getById: (id: string) => store.getById(id),
    forget: (id: string, reason?: string) => store.forget(id, reason),
    listProjects: () => store.listProjects(),
  };
}
