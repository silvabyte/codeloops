import { Agent as VoltAgent } from '@voltagent/core';
import { VercelAIProvider } from '@voltagent/vercel-ai';
import { LanguageModelV1 } from 'ai';
import { ZodType } from 'zod';
import { Logger } from 'pino';
export type OutputSchema = unknown;
export interface AgentConfig {
  name: string;
  instructions: string;
  outputSchema: ZodType;
  model: LanguageModelV1;
  maxRetries?: number;
  temperature?: number;
  maxTokens?: number;
  markdown?: boolean;
}
export interface AgentSendOptions {
  stream?: boolean;
  temperature?: number;
  maxTokens?: number;
  signal?: AbortSignal;
}
export declare class AgentError extends Error {
  readonly agentName: string;
  readonly cause?: unknown;
  constructor(message: string, agentName: string, cause?: unknown);
}
interface AgentDeps {
  logger: Logger;
}
export declare class Agent {
  private readonly _agent;
  private readonly logger;
  private readonly name;
  private readonly instructions;
  private readonly outputSchema;
  private readonly maxRetries;
  private readonly temperature?;
  private readonly maxTokens?;
  constructor(config: AgentConfig, { logger }: AgentDeps);
  send<T>(prompt: string, options?: AgentSendOptions): Promise<T>;
  sendText(prompt: string, options?: AgentSendOptions): Promise<string>;
  streamObject<T>(prompt: string, options?: AgentSendOptions): AsyncGenerator<Partial<T>>;
  streamText(prompt: string, options?: AgentSendOptions): AsyncGenerator<string>;
  /**
   * Get the underlying agent framework instance for advanced usage
   * Note: May be null if agent framework failed to initialize
   */
  getUnderlyingAgent(): VoltAgent<{
    llm: VercelAIProvider;
  }> | null;
  getName(): string;
  getInstructions(): string;
  getSchema(): ZodType<any, import('zod').ZodTypeDef, any>;
  private executeWithRetry;
}
export declare const createAgent: (config: AgentConfig, deps: AgentDeps) => Agent;
export declare const createOpenAIAgent: (
  config: Omit<AgentConfig, 'model'> & {
    model?: LanguageModelV1;
  },
  deps: AgentDeps,
) => Agent;
export declare const createAzureAgent: (
  config: Omit<AgentConfig, 'model'> & {
    model: LanguageModelV1;
  },
  deps: AgentDeps,
) => Agent;
export { Agent as BaseAgent };
export type { Tool, Memory, BaseRetriever } from '@voltagent/core';
export { createTool } from '@voltagent/core';
//# sourceMappingURL=BaseAgent.d.ts.map
