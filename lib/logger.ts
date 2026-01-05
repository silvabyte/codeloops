import { existsSync, mkdirSync } from "node:fs";
import path from "node:path";
import { type Logger, pino } from "pino";
import { getLogsDir } from "./config.ts";

/**
 * Logger type used throughout codeloops.
 */
export type CodeLoopsLogger = Logger;

/**
 * Options for creating a logger instance.
 */
export type CreateLoggerOptions = {
  /** Enable pretty stdout output (for development) */
  withDevStdout?: boolean;
  /** Enable file logging with rotation */
  withFile?: boolean;
  /** Custom log file name (default: "codeloops.log") */
  logFileName?: string;
  /** Custom logs directory (default: XDG data dir + /logs) */
  logsDir?: string;
  /** Number of days to retain logs (default: 14) */
  retentionDays?: number;
  /** Log rotation frequency (default: "daily") */
  frequency?: "daily" | "hourly";
  /** Set as global logger singleton */
  setGlobal?: boolean;
  /** Sync mode for logging */
  sync?: boolean;
};

let globalLogger: CodeLoopsLogger | null = null;

/**
 * Creates and returns a new pino logger instance with the given options.
 *
 * @example
 * // Server logger with file output
 * const serverLogger = createLogger({
 *   withFile: true,
 *   logFileName: "server.log",
 *   setGlobal: true,
 * });
 *
 * @example
 * // Plugin logger with custom directory
 * const pluginLogger = createLogger({
 *   withFile: true,
 *   logFileName: "plugin.log",
 *   retentionDays: 7,
 * });
 */
export function createLogger(options?: CreateLoggerOptions): CodeLoopsLogger {
  const logsDir = options?.logsDir ?? getLogsDir();
  const logFileName = options?.logFileName ?? "codeloops.log";
  const logFile = path.join(logsDir, logFileName);
  const retentionDays = options?.retentionDays ?? 14;
  const frequency = options?.frequency ?? "daily";

  // Ensure logs directory exists
  if (!existsSync(logsDir)) {
    mkdirSync(logsDir, { recursive: true });
  }

  const targets: pino.TransportTargetOptions[] = [];

  if (options?.withFile) {
    targets.push({
      target: "pino-roll",
      options: {
        file: logFile,
        frequency,
        limit: {
          count: retentionDays,
        },
      },
    });
  }

  if (options?.withDevStdout) {
    targets.push({
      target: "pino-pretty",
      options: {
        destination: 1,
      },
    });
  }

  // If no targets specified, default to noop
  if (targets.length === 0) {
    targets.push({
      target: "pino/file",
      options: { destination: "/dev/null" },
    });
  }

  const transports = pino.transport({
    targets,
    ...(options?.sync ? { sync: true } : {}),
  });

  const logger = pino(transports);

  if (options?.setGlobal && !globalLogger) {
    globalLogger = logger;
  }

  return logger;
}

/**
 * Returns the global singleton logger instance.
 * If not created, creates one with default options (file logging enabled).
 */
export function getLoggerInstance(
  options?: CreateLoggerOptions
): CodeLoopsLogger {
  if (!globalLogger) {
    globalLogger = createLogger({
      withFile: true,
      ...options,
      setGlobal: true,
    });
  }
  return globalLogger;
}

/**
 * Set the global logger instance.
 * Useful for testing or custom logger configurations.
 */
export function setGlobalLogger(logger: CodeLoopsLogger): void {
  globalLogger = logger;
}

/**
 * Create a child logger with additional context.
 */
export function createChildLogger(
  parent: CodeLoopsLogger,
  context: Record<string, unknown>
): CodeLoopsLogger {
  return parent.child(context);
}
