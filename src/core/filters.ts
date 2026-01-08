import type { MemoryEntry, MemoryRole, QueryOptions } from "./types.ts";

/**
 * Check if entry matches the project filter.
 */
export function matchesProject(entry: MemoryEntry, project?: string): boolean {
  return !project || entry.project === project;
}

/**
 * Check if entry matches the role filter.
 */
export function matchesRole(entry: MemoryEntry, role?: MemoryRole): boolean {
  return !role || entry.role === role;
}

/**
 * Check if entry matches the session ID filter.
 */
export function matchesSessionId(
  entry: MemoryEntry,
  sessionId?: string
): boolean {
  return !sessionId || entry.sessionId === sessionId;
}

/**
 * Check if entry matches all specified tags.
 */
export function matchesTags(entry: MemoryEntry, tags?: string[]): boolean {
  if (!tags || tags.length === 0) {
    return true;
  }
  return Boolean(entry.tags && tags.every((tag) => entry.tags?.includes(tag)));
}

/**
 * Check if entry content matches the search query (case-insensitive).
 */
export function matchesQuery(
  entry: MemoryEntry,
  searchQuery?: string
): boolean {
  if (!searchQuery) {
    return true;
  }
  return entry.content.toLowerCase().includes(searchQuery.toLowerCase());
}

/**
 * Check if entry matches all filters in the query options.
 */
export function entryMatchesFilters(
  entry: MemoryEntry,
  options: QueryOptions
): boolean {
  return (
    matchesProject(entry, options.project) &&
    matchesSessionId(entry, options.sessionId) &&
    matchesTags(entry, options.tags) &&
    matchesQuery(entry, options.query) &&
    matchesRole(entry, options.role)
  );
}
