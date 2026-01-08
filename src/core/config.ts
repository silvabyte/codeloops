import path from "node:path";
import envPaths from "env-paths";

/**
 * Cross-platform paths for codeloops data storage.
 * Uses XDG Base Directory specification on Linux.
 *
 * Locations:
 * - Linux: ~/.local/share/codeloops
 * - macOS: ~/Library/Application Support/codeloops
 * - Windows: %APPDATA%/codeloops
 */
const paths = envPaths("codeloops", { suffix: "" });

/**
 * Get the cross-platform data directory for codeloops.
 */
export function getDataDir(): string {
  return paths.data;
}

/**
 * Get the cross-platform logs directory for codeloops.
 */
export function getLogsDir(): string {
  return path.join(paths.data, "logs");
}

/**
 * Get the path to the memory log file.
 */
export function getMemoryLogPath(): string {
  return path.resolve(paths.data, "memory.ndjson");
}

/**
 * Get the path to the deleted memory log file.
 */
export function getDeletedMemoryLogPath(): string {
  return path.resolve(paths.data, "memory.deleted.ndjson");
}

export { paths };
