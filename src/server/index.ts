import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import pkg from "../../package.json" with { type: "json" };

export const createMcpServerInstance = () => {
  const server = new McpServer({
    name: "codeloops",
    version: pkg.version,
  });

  return server;
};
