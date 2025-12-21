import path from "node:path";
import { fileURLToPath } from "node:url";
import envPaths from "env-paths";

// -----------------------------------------------------------------------------
// Path Configuration ----------------------------------------------------------
// -----------------------------------------------------------------------------

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Legacy dataDir for backwards compatibility during migration
export const dataDir = path.resolve(__dirname, "..", "data");

// Cross-platform data directory using env-paths
const paths = envPaths("codeloops", { suffix: "" });

/**
 * Get the cross-platform data directory for codeloops
 * - Linux: ~/.local/share/codeloops
 * - macOS: ~/Library/Application Support/codeloops
 * - Windows: %APPDATA%/codeloops
 */
export function getDataDir(): string {
  return paths.data;
}
