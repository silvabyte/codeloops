import { createLogger } from '@codeloops/core';
import { stdio } from './stdio.js';
import { http } from './http.js';

// -----------------------------------------------------------------------------
// CLI Configuration -----------------------------------------------------------
// -----------------------------------------------------------------------------

interface ServerConfig {
  protocol: 'stdio' | 'http';
  port?: number;
  host?: string;
}

const parseArgs = (): ServerConfig => {
  const args = process.argv.slice(2);
  const config: ServerConfig = {
    protocol: 'stdio', // Default to stdio for backward compatibility
    port: 3000,
    host: '0.0.0.0',
  };

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    switch (arg) {
      case '--http':
        config.protocol = 'http';
        break;
      case '--stdio':
        config.protocol = 'stdio';
        break;
      case '--port': {
        const portStr = args[++i];
        if (!portStr) {
          throw new Error('--port requires a value');
        }
        const port = parseInt(portStr);
        if (isNaN(port)) {
          throw new Error('Invalid port number');
        }
        config.port = port;
        break;
      }
      case '--host': {
        const host = args[++i];
        if (!host) {
          throw new Error('--host requires a value');
        }
        config.host = host;
        break;
      }
      case '--help':
      case '-h':
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
        if (arg && arg.startsWith('--')) {
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
    console.error('Error parsing arguments:', error);
    process.exit(1);
  }

  logger.info(`Starting CodeLoops MCP server with ${config.protocol} transport...`);

  try {
    switch (config.protocol) {
      case 'stdio':
        await stdio.buildServer({ logger });
        logger.info('CodeLoops MCP server running on stdio');
        break;
      case 'http':
        if (!config.port) {
          throw new Error('Port is required for HTTP transport');
        }
        await http.buildServer({ logger, port: config.port });
        logger.info(`CodeLoops MCP HTTP server running on ${config.host}:${config.port}`);
        break;
      default:
        throw new Error(`Unsupported protocol: ${config.protocol}`);
    }
  } catch (error) {
    logger.error({ error }, 'Failed to start server');
    process.exit(1);
  }
}

main().catch((err) => {
  logger.error({ err }, 'Fatal error in main');
  process.exit(1);
});
