import { LanguageModelV1 } from 'ai';
import { anthropic } from '@ai-sdk/anthropic';
import { openai } from '@ai-sdk/openai';
import { azure } from '@ai-sdk/azure';
import { getConfig, getModelConfig, getProviderApiKey, getProviderConfig } from './index.ts';
import { env } from './env.ts';

/**
 * Creates a model instance from a model reference string
 * @param modelRef Model reference in format "provider.model" (e.g., "openai.gpt-4o")
 * @returns Configured LanguageModelV1 instance
 */
export function createModel(modelRef: string): LanguageModelV1 {
  if (!modelRef || typeof modelRef !== 'string') {
    throw new Error(`Invalid model reference: ${modelRef}`);
  }

  // Parse model reference (format: "provider.model")
  const [provider, modelName] = modelRef.split('.');
  if (!provider || !modelName) {
    throw new Error(`Invalid model reference format: ${modelRef}. Expected "provider.model"`);
  }

  // Get model configuration
  const modelConfig = getModelConfig(modelRef);
  if (!modelConfig) {
    throw new Error(`Model configuration not found for: ${modelRef}`);
  }

  // Create model instance based on provider
  switch (provider) {
    case 'anthropic':
      return createAnthropicModel(modelConfig.model.id);

    case 'openai':
      return createOpenAIModel(modelConfig.model.id);

    case 'azure':
      return createAzureModel(modelConfig.model.id);

    case 'deepseek':
      return createDeepSeekModel(modelConfig.model.id);

    case 'google':
      return createGoogleModel(modelConfig.model.id);

    case 'generic':
      return createGenericModel(modelConfig.model.id);

    default:
      throw new Error(`Unsupported model provider: ${provider}`);
  }
}

/**
 * Create Anthropic model instance
 */
function createAnthropicModel(modelId: string): LanguageModelV1 {
  const apiKey = getProviderApiKey('anthropic');
  if (!apiKey) {
    throw new Error(
      'Anthropic API key not configured. Add api_key to providers.anthropic in config.',
    );
  }

  // Set environment variable for AI SDK compatibility
  env.set('ANTHROPIC_API_KEY', apiKey);

  return anthropic(modelId);
}

/**
 * Create OpenAI model instance
 */
function createOpenAIModel(modelId: string): LanguageModelV1 {
  const apiKey = getProviderApiKey('openai');
  if (!apiKey) {
    throw new Error('OpenAI API key not configured. Add api_key to providers.openai in config.');
  }

  // Set environment variable for AI SDK compatibility
  env.set('OPENAI_API_KEY', apiKey);

  return openai(modelId);
}

/**
 * Create Azure OpenAI model instance
 */
function createAzureModel(modelId: string): LanguageModelV1 {
  const azureConfig = getProviderConfig('azure');
  const apiKey = azureConfig?.api_key;
  const resourceName = azureConfig?.resource_name;

  if (!apiKey || !resourceName) {
    throw new Error(
      'Azure OpenAI configuration incomplete. Add api_key and resource_name to providers.azure in config.',
    );
  }

  // Set environment variables for AI SDK compatibility
  env.set('AZURE_OPENAI_API_KEY', apiKey);
  env.set('AZURE_OPENAI_RESOURCE_NAME', resourceName);

  return azure(modelId);
}

/**
 * Create DeepSeek model instance (OpenAI-compatible)
 */
function createDeepSeekModel(modelId: string): LanguageModelV1 {
  const apiKey = getProviderApiKey('deepseek');
  if (!apiKey) {
    throw new Error(
      'DeepSeek API key not configured. Add api_key to providers.deepseek in config.',
    );
  }

  // @ts-expect-error - OpenAI SDK types are strict but this works at runtime
  return openai(modelId, {
    baseURL: 'https://api.deepseek.com/v1',
    apiKey,
  });
}

/**
 * Create Google model instance (via OpenAI-compatible API)
 */
function createGoogleModel(modelId: string): LanguageModelV1 {
  const apiKey = getProviderApiKey('google');
  if (!apiKey) {
    throw new Error('Google API key not configured. Add api_key to providers.google in config.');
  }

  // @ts-expect-error - OpenAI SDK types are strict but this works at runtime
  return openai(modelId, {
    baseURL: 'https://generativelanguage.googleapis.com/v1beta',
    apiKey,
  });
}

/**
 * Create generic/Ollama model instance
 */
function createGenericModel(modelId: string): LanguageModelV1 {
  const genericConfig = getProviderConfig('generic');

  const baseURL = genericConfig?.base_url || 'http://localhost:11434/v1';
  const apiKey = genericConfig?.api_key || 'ollama';

  // @ts-expect-error - OpenAI SDK types are strict but this works at runtime
  return openai(modelId, {
    baseURL,
    apiKey,
  });
}

/**
 * Gets a model reference from configuration path
 * @param configPath Configuration path (e.g., "agents.critic.model" or "default_model")
 * @returns Model reference string or null if not found
 */
export function getModelReference(configPath: string): string | null {
  const config = getConfig();
  const modelRef = config.get(configPath);

  if (typeof modelRef === 'string' && modelRef === 'default') {
    return config.get('default_model') as string;
  }

  return typeof modelRef === 'string' ? modelRef : null;
}

/**
 * Get model configuration from a config path
 * @param configPath Configuration path (e.g., "agents.critic" or "agents.actor")
 * @returns Model configuration including temperature, max_tokens, etc.
 */
export function getModelConfigFromPath(configPath: string) {
  const config = getConfig();
  const agentConfig = config.get(configPath) as Record<string, unknown> | undefined;

  return {
    temperature: (agentConfig?.temperature as number) || 0.7,
    maxTokens: (agentConfig?.max_tokens as number) || 2000,
    enabled: (agentConfig?.enabled as boolean) ?? true,
  };
}
