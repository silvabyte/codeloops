import { Agent as VoltAgent, createHooks } from '@voltagent/core';
import { VercelAIProvider } from '@voltagent/vercel-ai';
import type { LanguageModelV1 } from 'ai';
import { getProviderApiKey, getConfig } from '@codeloops/config';
import type { ZodType } from 'zod';
import type { Logger } from 'pino';

export type OutputSchema = unknown;

export interface AgentConfig<> {
  name: string;
  instructions: string;
  outputSchema: ZodType;
  model: LanguageModelV1;
  maxRetries?: number;
  temperature?: number;
  maxTokens?: number;
  // Extension points for VoltAgent features (keeping things simple for now)
  markdown?: boolean;
}

export interface AgentSendOptions {
  stream?: boolean;
  temperature?: number;
  maxTokens?: number;
  signal?: AbortSignal;
}

export class AgentError extends Error {
  readonly agentName: string;
  readonly cause?: unknown;

  constructor(message: string, agentName: string, cause?: unknown) {
    super(message);
    this.name = 'AgentError';
    this.agentName = agentName;
    this.cause = cause;
  }
}

interface AgentDeps {
  logger: Logger;
}

export class Agent {
  private readonly _agent: VoltAgent<{ llm: VercelAIProvider }>; // Using any for now to avoid complex type issues
  private readonly logger: Logger;
  private readonly name: string;
  private readonly instructions: string;
  private readonly outputSchema: ZodType;
  private readonly maxRetries: number;
  private readonly temperature?: number;
  private readonly maxTokens?: number;

  constructor(config: AgentConfig, { logger }: AgentDeps) {
    this.logger = logger.child({ agentName: config.name });

    // Validate config
    if (!config.name?.trim()) {
      throw new AgentError('Agent name is required', config.name || 'unknown');
    }
    if (!config.instructions?.trim()) {
      throw new AgentError('Agent instructions are required', config.name);
    }
    if (!config.outputSchema) {
      throw new AgentError('Output schema is required', config.name);
    }
    if (!config.model) {
      throw new AgentError('Model is required', config.name);
    }

    // Store config for external access
    this.name = config.name;
    this.instructions = config.instructions;
    this.outputSchema = config.outputSchema;
    this.maxRetries = config.maxRetries ?? 3;
    this.temperature = config.temperature ?? 0.7;
    this.maxTokens = config.maxTokens;

    // Create VoltAgent hooks for logging integration
    const hooks = createHooks({
      onStart: async ({ agent, context }) => {
        //TODO: add a metrics context tracker for hooks
        this.logger.info(
          {
            startTime: new Date().toISOString(),
            hookName: 'onStart',
            operationId: context.operationId,
            agentName: agent.name,
          },
          'VoltAgent operation started',
        );
      },
      onEnd: async ({ agent, output, error, context }) => {
        if (error) {
          this.logger.error(
            {
              endTime: new Date().toISOString(),
              hookName: 'onEnd',
              operationId: context.operationId,
              agentName: agent.name,
              error,
            },
            'VoltAgent operation failed',
          );
        } else {
          this.logger.info(
            {
              endTime: new Date().toISOString(),
              hookName: 'onEnd',
              operationId: context.operationId,
              agentName: agent.name,
              hasOutput: !!output,
            },
            'VoltAgent operation completed',
          );
        }
      },
      onToolStart: async ({ agent, tool, context }) => {
        this.logger.info(
          {
            startTime: new Date().toISOString(),
            hookName: 'onToolStart',
            operationId: context.operationId,
            agentName: agent.name,
            toolName: tool.name,
          },
          'VoltAgent tool execution started',
        );
      },
      onToolEnd: async ({ agent, tool, output, error, context }) => {
        if (error) {
          this.logger.error(
            {
              endTime: new Date().toISOString(),
              hookName: 'onToolEnd',
              operationId: context.operationId,
              agentName: agent.name,
              toolName: tool.name,
              error,
            },
            'VoltAgent tool execution failed',
          );
        } else {
          this.logger.info(
            {
              endTime: new Date().toISOString(),
              hookName: 'onToolEnd',
              operationId: context.operationId,
              agentName: agent.name,
              toolName: tool.name,
              hasOutput: !!output,
            },
            'VoltAgent tool execution completed',
          );
        }
      },
    });

    // Initialize VoltAgent with basic configuration
    try {
      this._agent = new VoltAgent({
        name: config.name,
        instructions: config.instructions,
        llm: new VercelAIProvider(),
        model: config.model,
        hooks,
        markdown: config.markdown ?? false,
        // Keep it simple for now - advanced features can be added later
        memory: false,
        tools: [],
        subAgents: [],
      });

      this.logger.info(
        {
          agentName: config.name,
          markdownEnabled: config.markdown ?? false,
        },
        'Agent initialized with VoltAgent',
      );
    } catch (error) {
      this.logger.error(
        {
          agentName: config.name,
          error,
        },
        'Failed to initialize VoltAgent',
      );

      // For now, if VoltAgent fails, we'll also fail the agent initialization
      throw error;
    }
  }

  async send<T>(prompt: string, options?: AgentSendOptions): Promise<T> {
    if (!prompt?.trim()) {
      throw new AgentError('Prompt cannot be empty', this.name);
    }

    const startTime = Date.now();
    this.logger.info(
      {
        promptLength: prompt.length,
        prompt,
        options,
      },
      `Agent ${this.name} processing prompt`,
    );

    try {
      const response = await this.executeWithRetry(async () => {
        if (this._agent) {
          // Use agent framework if available
          try {
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            return await this._agent.generateObject(prompt, this.outputSchema as any, {
              provider: {
                temperature: options?.temperature ?? this.temperature,
                maxTokens: options?.maxTokens ?? this.maxTokens,
                onError: async (error: unknown) => {
                  this.logger.error({ error }, 'Agent provider error');
                },
              },
            });
          } catch (voltError) {
            this.logger.warn(
              { error: voltError },
              'Agent framework failed, falling back to direct AI SDK',
            );
            // Fallback to direct AI SDK usage
            throw voltError;
          }
        } else {
          throw new Error('Agent framework not available');
        }
      }, this.maxRetries);

      const duration = Date.now() - startTime;
      this.logger.info(
        {
          duration,
          hasObject: !!response.object,
        },
        `Agent ${this.name} completed successfully`,
      );

      return response.object as T;
    } catch (error) {
      this.logger.error(
        {
          error,
          duration: Date.now() - startTime,
        },
        `Agent ${this.name} failed`,
      );
      throw new AgentError(
        `Failed to generate response: ${error instanceof Error ? error.message : 'Unknown error'}`,
        this.name,
        error,
      );
    }
  }

  async sendText(prompt: string, options?: AgentSendOptions): Promise<string> {
    if (!prompt?.trim()) {
      throw new AgentError('Prompt cannot be empty', this.name);
    }

    const startTime = Date.now();
    this.logger.info(
      {
        promptLength: prompt.length,
        prompt,
        options,
      },
      `Agent ${this.name} processing text prompt`,
    );

    try {
      const response = await this.executeWithRetry(async () => {
        if (this._agent) {
          try {
            return await this._agent.generateText(prompt, {
              provider: {
                temperature: options?.temperature ?? this.temperature,
                maxTokens: options?.maxTokens ?? this.maxTokens,
                onError: async (error: unknown) => {
                  this.logger.error({ error }, 'Agent provider error');
                },
              },
            });
          } catch (voltError) {
            this.logger.warn({ error: voltError }, 'Agent framework failed for text generation');
            throw voltError;
          }
        } else {
          throw new Error('Agent framework not available');
        }
      }, this.maxRetries);

      const duration = Date.now() - startTime;
      this.logger.info(
        {
          duration,
          textLength: response.text.length,
        },
        `Agent ${this.name} text generation completed`,
      );

      return response.text;
    } catch (error) {
      this.logger.error(
        {
          error,
          duration: Date.now() - startTime,
        },
        `Agent ${this.name} text generation failed`,
      );
      throw new AgentError(
        `Failed to generate text response: ${error instanceof Error ? error.message : 'Unknown error'}`,
        this.name,
        error,
      );
    }
  }

  async *streamObject<T>(prompt: string, options?: AgentSendOptions): AsyncGenerator<Partial<T>> {
    if (!prompt?.trim()) {
      throw new AgentError('Prompt cannot be empty', this.name);
    }

    const startTime = Date.now();
    this.logger.info(
      {
        promptLength: prompt.length,
        prompt,
      },
      `Agent ${this.name} starting object stream`,
    );

    try {
      if (this._agent) {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const response = await this._agent.streamObject(prompt, this.outputSchema as any, {
          provider: {
            temperature: options?.temperature ?? this.temperature,
            maxTokens: options?.maxTokens ?? this.maxTokens,
            onError: async (error: unknown) => {
              this.logger.error({ error }, 'Agent stream error');
            },
          },
        });

        for await (const partial of response.objectStream) {
          this.logger.debug(
            {
              duration: Date.now() - startTime,
              hasPartial: !!partial,
            },
            `Agent ${this.name} streaming partial object`,
          );
          yield partial as Partial<T>;
        }
      } else {
        throw new AgentError('Agent framework not available for streaming', this.name);
      }

      this.logger.info(
        {
          duration: Date.now() - startTime,
        },
        `Agent ${this.name} object stream completed`,
      );
    } catch (error) {
      this.logger.error(
        {
          error,
          duration: Date.now() - startTime,
        },
        `Agent ${this.name} object stream failed`,
      );
      throw new AgentError(
        `Failed to stream object response: ${error instanceof Error ? error.message : 'Unknown error'}`,
        this.name,
        error,
      );
    }
  }

  async *streamText(prompt: string, options?: AgentSendOptions): AsyncGenerator<string> {
    if (!prompt?.trim()) {
      throw new AgentError('Prompt cannot be empty', this.name);
    }

    const startTime = Date.now();
    this.logger.info(
      {
        promptLength: prompt.length,
        prompt,
      },
      `Agent ${this.name} starting text stream`,
    );

    try {
      if (this._agent) {
        const response = await this._agent.streamText(prompt, {
          provider: {
            temperature: options?.temperature ?? this.temperature,
            maxTokens: options?.maxTokens ?? this.maxTokens,
            onError: async (error: unknown) => {
              this.logger.error({ error }, 'Agent stream error');
            },
          },
        });

        for await (const delta of response.textStream) {
          this.logger.debug(
            {
              duration: Date.now() - startTime,
              deltaLength: delta.length,
            },
            `Agent ${this.name} streaming text`,
          );
          yield delta;
        }
      } else {
        throw new AgentError('Agent framework not available for streaming', this.name);
      }

      this.logger.info(
        {
          duration: Date.now() - startTime,
        },
        `Agent ${this.name} text stream completed`,
      );
    } catch (error) {
      this.logger.error(
        {
          error,
          duration: Date.now() - startTime,
        },
        `Agent ${this.name} text stream failed`,
      );
      throw new AgentError(
        `Failed to stream text response: ${error instanceof Error ? error.message : 'Unknown error'}`,
        this.name,
        error,
      );
    }
  }

  /**
   * Get the underlying agent framework instance for advanced usage
   * Note: May be null if agent framework failed to initialize
   */
  getUnderlyingAgent(): VoltAgent<{ llm: VercelAIProvider }> | null {
    return this._agent;
  }

  // Existing API compatibility methods
  getName(): string {
    return this.name;
  }

  getInstructions(): string {
    return this.instructions;
  }

  getSchema() {
    return this.outputSchema;
  }

  private async executeWithRetry<R>(
    fn: () => Promise<R>,
    retriesLeft: number,
    lastError?: unknown,
  ): Promise<R> {
    if (retriesLeft <= 0) {
      throw lastError || new Error('Max retries exceeded');
    }

    try {
      return await fn();
    } catch (error) {
      this.logger.warn(
        {
          retriesLeft: retriesLeft - 1,
          error,
        },
        `Agent ${this.name} attempt failed, retrying...`,
      );

      await new Promise((resolve) =>
        setTimeout(resolve, 1000 * (this.maxRetries - retriesLeft + 1)),
      );

      return this.executeWithRetry(fn, retriesLeft - 1, error);
    }
  }
}

// Factory functions for convenience
export const createAgent = (config: AgentConfig, deps: AgentDeps) => {
  return new Agent(config, deps);
};

export const createOpenAIAgent = (
  config: Omit<AgentConfig, 'model'> & { model?: LanguageModelV1 },
  deps: AgentDeps,
): Agent => {
  const apiKey = getProviderApiKey('openai');

  if (!apiKey) {
    throw new AgentError(
      'Missing OpenAI API key. Please add api_key to providers.openai in config.',
      config.name,
    );
  }

  if (!config.model) {
    throw new AgentError('Model must be provided for OpenAI agent', config.name);
  }

  return createAgent(
    {
      ...config,
      model: config.model,
    },
    deps,
  );
};

export const createAzureAgent = (
  config: Omit<AgentConfig, 'model'> & { model: LanguageModelV1 },
  deps: AgentDeps,
): Agent => {
  const conf = getConfig();
  const azureConfig = conf.get('providers.azure');
  const apiKey = getProviderApiKey('azure');
  const resourceName = azureConfig?.resource_name;

  if (!apiKey || !resourceName) {
    throw new AgentError(
      'Missing Azure OpenAI credentials. Please add api_key and resource_name to providers.azure in config.',
      config.name,
    );
  }

  return createAgent(
    {
      ...config,
      model: config.model,
    },
    deps,
  );
};

// Export as BaseAgent for clarity when used in inheritance
export { Agent as BaseAgent };

// Re-export VoltAgent types for future advanced usage
export type { Tool, Memory, BaseRetriever } from '@voltagent/core';
export { createTool } from '@voltagent/core';
