import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import packageJson from '../package.json' with { type: 'json' };

export const createMcpServerInstance = () => {
  const server = new McpServer({
    name: packageJson.name,
    version: packageJson.version,
  });

  return server;
};
