import Fastify, { FastifyInstance } from 'fastify';
import { StreamableHTTPServerTransport } from '@modelcontextprotocol/sdk/server/streamableHttp.js';
import { isInitializeRequest } from '@modelcontextprotocol/sdk/types.js';
import { FastifyRequest, FastifyReply } from 'fastify';
import { nanoid } from 'nanoid';
import { CodeLoopsLogger } from '../logger.js';
import { createMcpServerInstance } from './index.js';
import { SSEServerTransport } from '@modelcontextprotocol/sdk/server/sse.js';
import { registerTools } from './tools.js';

declare module 'fastify' {
  interface FastifyRequest {
    currentTransport?: StreamableHTTPServerTransport;
    sessionId?: string;
  }
}

export const buildServer = async ({ logger, port }: { logger: CodeLoopsLogger; port: number }) => {
  logger.info('Building CodeLoops MCP HTTP server');

  const fastify = Fastify({
    logger: {
      level: 'info',
    },
  });

  // Scopes the onRequest hook to the mcp routes in this register handler
  await fastify.register(async (fastify) => {
    const routePrefix = '/api';
    // Store transports for each session type
    const transports = {
      streamable: {} as Record<string, StreamableHTTPServerTransport>,
      sse: {} as Record<string, SSEServerTransport>,
    };

    // Handle POST requests for client-to-server communication
    fastify.post(`${routePrefix}/mcp`, async (request, reply) => {
      const sessionId = request.headers['mcp-session-id'];
      let transport: StreamableHTTPServerTransport;

      request.log.info('Processing onRequest transport configuration');

      if (sessionId && transports.streamable[sessionId as string]) {
        request.log.info(`Using existing transport for session ID: ${sessionId}`);
        transport = transports.streamable[sessionId as string];
      } else if (!sessionId && isInitializeRequest(request.body)) {
        request.log.info('Initializing new transport');
        transport = new StreamableHTTPServerTransport({
          sessionIdGenerator: () => nanoid(),
          onsessioninitialized: (sessionId) => {
            transports.streamable[sessionId] = transport;
          },
        });

        transport.onclose = () => {
          request.log.info('MCP HTTP server closed');
          if (transport.sessionId) {
            request.log.info('Removing transport for session ID: ' + transport.sessionId);
            delete transports.streamable[transport.sessionId];
          }
        };

        request.log.info('Creating MCP server instance');
        const server = createMcpServerInstance();

        registerTools({ server });

        request.currentTransport = transport;

        request.log.info('Connecting MCP server to transport');
        await server.connect(transport);
        request.log.info('MCP server connected and ready to receive requests');
      } else {
        request.log.error('No valid session ID provided');
        return reply.status(400).send({
          jsonrpc: '2.0',
          error: {
            code: -32000,
            message: 'Bad Request: No valid session ID provided',
          },
          id: null,
        });
      }
      //if for some reason this does not exist, error
      if (!transport) {
        request.log.error('No transport found for session ID');
        return reply.status(400).send('Invalid or missing session ID');
      }

      request.log.info({ body: request.body }, 'Processing MCP request');
      await transport.handleRequest(request.raw, reply.raw, request.body);
    });

    // Reusable handler for GET and DELETE
    /**
     * Handles session requests by verifying the session ID and utilizing
     * the respective transport to process the request. Responds with an error
     * if the session ID is invalid or missing.
     *
     * @param {FastifyRequest} request - The incoming Fastify request object.
     * @param {FastifyReply} reply - The Fastify reply object to send responses.
     */
    const handleSessionRequest = async (request: FastifyRequest, reply: FastifyReply) => {
      const sessionId = request.headers['mcp-session-id'];
      if (!sessionId || !transports.streamable[sessionId as string]) {
        request.log.error('No transport found for session ID');
        return reply.status(400).send('Invalid or missing session ID');
      }

      const transport = transports.streamable[sessionId as string];
      await transport.handleRequest(request.raw, reply.raw);
    };

    // Handle GET for SSE
    fastify.get(`${routePrefix}/mcp`, handleSessionRequest);

    // Handle DELETE for session termination
    fastify.delete(`${routePrefix}/mcp`, handleSessionRequest);

    //add legacy routes
    await addLegacyRoutes({ transports, fastify });
  });

  try {
    const address = await fastify.listen({ port, host: '0.0.0.0' });
    fastify.log.info(`Server listening at ${address}`);
  } catch (err) {
    fastify.log.error(err);
    process.exit(1);
  }
};

export const addLegacyRoutes = async ({
  transports,
  fastify,
}: {
  transports: {
    streamable: Record<string, StreamableHTTPServerTransport>;
    sse: Record<string, SSEServerTransport>;
  };
  fastify: FastifyInstance;
}) => {
  //creating dedicated server instance for legacy routes
  const server = createMcpServerInstance();

  registerTools({ server });

  fastify.get('/sse', async (request, reply) => {
    const transport = new SSEServerTransport('/messages', reply.raw);
    transports.sse[transport.sessionId as string] = transport;
    reply.raw.on('close', () => {
      delete transports.sse[transport.sessionId as string];
    });
    await server.connect(transport);
  });
  fastify.post('/messages', async (request, reply) => {
    //
    const sessionId = (request?.query as { sessionId?: string })?.sessionId as string;
    const transport = transports.sse[sessionId];
    if (!transport) {
      reply.status(400).send('Invalid or missing session ID');
      return;
    }
    await transport.handlePostMessage(request.raw, reply.raw, request.body);
  });
};

export const http = {
  buildServer,
};
