import { z } from "zod";

/**
 * Role of the entity that created the memory entry.
 * - "actor": The primary coding agent
 * - "critic": The feedback/analysis agent
 * - "human": User-provided clarification (future)
 */
export const MemoryRoleSchema = z.enum(["actor", "critic", "human"]);
export type MemoryRole = z.infer<typeof MemoryRoleSchema>;

/**
 * Schema for validating memory entries stored in NDJSON format.
 */
export const MemoryEntrySchema = z.object({
  id: z.string(),
  project: z.string(),
  content: z.string(),
  tags: z.array(z.string()).optional(),
  createdAt: z.string().datetime(),
  sessionId: z.string().optional(),
  source: z.string().optional(),
  role: MemoryRoleSchema.optional(),
});

/**
 * A single memory entry stored in the memory log.
 */
export type MemoryEntry = z.infer<typeof MemoryEntrySchema>;

/**
 * A memory entry that has been soft-deleted.
 * Stored in a separate deleted log for potential recovery.
 */
export interface DeletedMemoryEntry extends MemoryEntry {
  deletedAt: string;
  deletedReason?: string;
}

/**
 * Options for querying memory entries.
 */
export type QueryOptions = {
  /** Filter by project name */
  project?: string;
  /** Filter by tags (all must match) */
  tags?: string[];
  /** Search text in content */
  query?: string;
  /** Maximum number of entries to return */
  limit?: number;
  /** Filter by session ID */
  sessionId?: string;
  /** Filter by role (actor, critic, human) */
  role?: MemoryRole;
};

/**
 * Input for appending a new memory entry.
 */
export type AppendInput = {
  content: string;
  project: string;
  tags?: string[];
  sessionId?: string;
  source?: string;
  role?: MemoryRole;
};

/**
 * Options for creating a MemoryStore instance.
 */
export type MemoryStoreOptions = {
  /** Custom data directory (for testing or custom configurations) */
  dataDir?: string;
};
