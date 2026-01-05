import path from "node:path";
import { fileURLToPath } from "node:url";
import { getDataDir as getDataDirFromLib } from "../lib/config.ts";

// -----------------------------------------------------------------------------
// Path Configuration
// -----------------------------------------------------------------------------

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

/**
 * @deprecated Use getDataDir() instead.
 * Legacy dataDir for backwards compatibility during migration.
 */
export const dataDir = path.resolve(__dirname, "..", "data");

/**
 * Get the cross-platform data directory for codeloops.
 * Re-exported from lib/config.ts for convenience.
 */
export const getDataDir = getDataDirFromLib;
