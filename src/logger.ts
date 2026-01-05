import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import type { Logger } from "pino";
import {
  createLogger as createLoggerFromLib,
  getLoggerInstance as getLoggerInstanceFromLib,
  type CreateLoggerOptions as LibCreateLoggerOptions,
  setGlobalLogger as setGlobalLoggerFromLib,
} from "../lib/logger.ts";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Legacy logs directory for backwards compatibility
const logsDir = path.resolve(__dirname, "../logs");

// Ensure logs directory exists
if (!fs.existsSync(logsDir)) {
  fs.mkdirSync(logsDir, { recursive: true });
}

// Types
export type CodeLoopsLogger = Logger;
export type CreateLoggerOptions = LibCreateLoggerOptions;

/**
 * Creates and returns a new pino logger instance with the given options.
 * Also sets the global logger if not already set.
 */
export function createLogger(options?: CreateLoggerOptions): CodeLoopsLogger {
  return createLoggerFromLib({
    logsDir,
    ...options,
  });
}

/**
 * Returns the global singleton logger instance.
 * If not created, creates with default options.
 */
export function getInstance(options?: CreateLoggerOptions): CodeLoopsLogger {
  return getLoggerInstanceFromLib({
    logsDir,
    ...options,
  });
}

/**
 * Set the global logger instance.
 */
export function setGlobalLogger(logger: CodeLoopsLogger): void {
  setGlobalLoggerFromLib(logger);
}
