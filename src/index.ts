import { createLogger } from "./logger.js";
import { http } from "./server/http.js";
import { stdio } from "./server/stdio.js";

// -----------------------------------------------------------------------------
// CLI Configuration -----------------------------------------------------------
// -----------------------------------------------------------------------------

type ServerConfig = {
  protocol: "stdio" | "http";
  port?: number;
  host?: string;
};

const parseArgs = (): ServerConfig => {
  const args = process.argv.slice(2);
  const config: ServerConfig = {
    protocol: "stdio", // Default to stdio for backward compatibility
    port: 3000,
    host: "0.0.0.0",
  };

  let skipNext = false;
  for (const [index, arg] of args.entries()) {
    if (skipNext) {
      skipNext = false;
      continue;
    }

    switch (arg) {
      case "--http":
        config.protocol = "http";
        break;
      case "--stdio":
        config.protocol = "stdio";
        break;
      case "--port": {
        const port = Number.parseInt(args[index + 1], 10);
        if (Number.isNaN(port)) {
          throw new Error("Invalid port number");
        }
        config.port = port;
        skipNext = true;
        break;
      }
      case "--host":
        config.host = args[index + 1];
        skipNext = true;
        break;
      case "--help":
      case "-h":
        console.log(`
CodeLoops MCP Server

Usage: npm start [options]

Options:
  --stdio              Use stdio transport (default)
  --http               Use HTTP transport
  --port <number>      HTTP server port (default: 3000)
  --host <string>      HTTP server host (default: 0.0.0.0)
  -h, --help           Show this help message

Examples:
  npm start                    # Use stdio transport
  npm start -- --http         # Use HTTP transport on default port 3000
  npm start -- --http --port 8080  # Use HTTP transport on port 8080
`);
        process.exit(0);
        break;
      default:
        if (arg.startsWith("--")) {
          throw new Error(`Unknown option: ${arg}`);
        }
        break;
    }
  }

  return config;
};

// -----------------------------------------------------------------------------
// Main Entry Point ------------------------------------------------------------
// -----------------------------------------------------------------------------
const logger = createLogger();

/**
 * Main entry point for the CodeLoops MCP server.
 */
async function main() {
  let config: ServerConfig;

  try {
    config = parseArgs();
  } catch (error) {
    console.error("Error parsing arguments:", error);
    process.exit(1);
  }

  logger.info(
    `Starting CodeLoops MCP server with ${config.protocol} transport...`
  );

  try {
    switch (config.protocol) {
      case "stdio":
        await stdio.buildServer({ logger });
        logger.info("CodeLoops MCP server running on stdio");
        break;
      case "http":
        if (!config.port) {
          throw new Error("Port is required for HTTP transport");
        }
        await http.buildServer({ logger, port: config.port });
        logger.info(
          `CodeLoops MCP HTTP server running on ${config.host}:${config.port}`
        );
        break;
      default:
        throw new Error(`Unsupported protocol: ${config.protocol}`);
    }
  } catch (error) {
    logger.error({ error }, "Failed to start server");
    process.exit(1);
  }
}

main().catch((err) => {
  logger.error({ err }, "Fatal error in main");
  process.exit(1);
});
