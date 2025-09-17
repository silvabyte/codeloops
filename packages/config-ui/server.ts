#!/usr/bin/env bun
import { getConfig } from '@codeloops/config';
import indexHtml from './public/index.html';

interface LMStudioModel {
  id: string;
  object: string;
  created?: number;
  owned_by?: string;
}

/**
 * Fetch available models from LM Studio
 */
async function fetchLMStudioModels(baseUrl: string): Promise<LMStudioModel[]> {
  try {
    const response = await fetch(`${baseUrl}/v1/models`);
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }
    const data = (await response.json()) as { data?: LMStudioModel[] };
    return data.data || [];
  } catch (error) {
    console.warn(
      `Could not fetch models from LM Studio (${baseUrl}):`,
      error instanceof Error ? error.message : String(error),
    );
    return [];
  }
}

interface ProviderConfig {
  base_url?: string;
  api_key?: string;
  models?: Record<string, unknown>;
}

/**
 * Test connection to a provider
 */
async function testConnection(provider: string, config: ProviderConfig) {
  switch (provider) {
    case 'lmstudio':
      if (config.base_url) {
        try {
          const response = await fetch(`${config.base_url}/v1/models`);
          const data = response.ok
            ? ((await response.json()) as { data?: LMStudioModel[] })
            : { data: [] };
          return { success: response.ok, models: data.data || [] };
        } catch {
          return { success: false, models: [] };
        }
      }
      return { success: false, models: [] };

    // Add other provider tests as needed
    default:
      return { success: true, models: [] };
  }
}

const server = Bun.serve({
  port: 3344,
  routes: {
    '/': indexHtml,

    // API Routes
    '/api/config': {
      GET: () => {
        try {
          const config = getConfig();
          const configData = {
            version: config.get('version'),
            default_model: config.get('default_model'),
            providers: config.get('providers'),
            agents: config.get('agents'),
            telemetry: config.get('telemetry'),
            logging: config.get('logging'),
            features: config.get('features'),
          };
          return Response.json(configData);
        } catch (error) {
          return Response.json(
            { error: error instanceof Error ? error.message : String(error) },
            { status: 500 },
          );
        }
      },

      POST: async (req) => {
        try {
          const newConfig = await req.json();
          const config = getConfig();

          // Update configuration
          for (const [key, value] of Object.entries(newConfig as Record<string, unknown>)) {
            config.set(key, value);
          }

          return Response.json({ success: true });
        } catch (error) {
          return Response.json(
            { error: error instanceof Error ? error.message : String(error) },
            { status: 500 },
          );
        }
      },
    },

    '/api/providers/:provider/test': {
      POST: async (req) => {
        try {
          const provider = req.params.provider;
          const providerConfig = (await req.json()) as ProviderConfig;
          const result = await testConnection(provider, providerConfig);
          return Response.json(result);
        } catch (error) {
          return Response.json(
            { error: error instanceof Error ? error.message : String(error) },
            { status: 500 },
          );
        }
      },
    },

    '/api/providers/:provider/models': {
      GET: async (req) => {
        try {
          const provider = req.params.provider;
          const config = getConfig();
          const providerConfig = config.get(`providers.${provider}`) as ProviderConfig;

          if (provider === 'lmstudio' && providerConfig?.base_url) {
            const models = await fetchLMStudioModels(providerConfig.base_url);
            return Response.json({ models });
          }

          return Response.json({ models: [] });
        } catch (error) {
          return Response.json(
            { error: error instanceof Error ? error.message : String(error) },
            { status: 500 },
          );
        }
      },
    },
  },

  development: {
    hmr: true,
    console: true,
  },
});

console.log(`🎨 CodeLoops Configuration UI running at http://localhost:${server.port}`);
