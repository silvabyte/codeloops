import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import type { CodeLoopsLogger } from '@codeloops/core';
import { createMcpServerInstance } from './server.js';
import { registerTools } from './tools.js';

export const buildServer = async ({ logger }: { logger: CodeLoopsLogger }) => {
  const server = createMcpServerInstance();

  registerTools({ server });

  logger.info('Initializing stdio transport');
  const transport = new StdioServerTransport();

  logger.info('Connecting MCP server to transport');
  await server.connect(transport);

  logger.info('MCP server connected and ready to receive requests');
};

export const stdio = {
  buildServer,
};
