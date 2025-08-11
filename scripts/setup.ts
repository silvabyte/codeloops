#!/usr/bin/env bun
// Run this script with: bun run setup
// Interactive setup for CodeLoops configuration
// Non-interactive mode: bun run setup --non-interactive --anthropic-key="your-key" --model="anthropic.haiku"

import fs from 'fs';
import path from 'path';
import inquirer from 'inquirer';
import { createFreshConfig, getConfig, CodeLoopsConfig, ModelConfig } from '@codeloops/config';
import { parseArgs } from 'util';

// Get the proper config file path
const configInstance = getConfig();
const CONFIG_FILE_PATH = configInstance.path;

interface ModelChoice {
  name: string;
  value: string;
  provider: string;
}

/**
 * Extract available models from the config system
 */
function getAvailableModels(config: CodeLoopsConfig): ModelChoice[] {
  const models: ModelChoice[] = [];

  for (const [providerName, providerConfig] of Object.entries(config.providers)) {
    if (providerConfig?.models) {
      for (const [modelKey, modelConfig] of Object.entries(providerConfig.models)) {
        const typedModelConfig = modelConfig as ModelConfig;
        models.push({
          name: `${providerName.charAt(0).toUpperCase() + providerName.slice(1)} - ${typedModelConfig.description || modelKey}`,
          value: `${providerName}.${modelKey}`,
          provider: providerName,
        });
      }
    }
  }

  return models;
}

/**
 * Load existing config or create fresh one
 */
function loadConfig(): CodeLoopsConfig {
  if (fs.existsSync(CONFIG_FILE_PATH)) {
    try {
      const configContent = fs.readFileSync(CONFIG_FILE_PATH, 'utf8');
      const parsed = JSON.parse(configContent);
      
      // Ensure agents structure exists with only critic and actor
      if (!parsed.agents) {
        parsed.agents = {};
      }
      if (!parsed.agents.critic) {
        parsed.agents.critic = {
          model: parsed.default_model || 'anthropic.haiku',
          enabled: true,
          temperature: 0.3,
          maxTokens: 2000,
        };
      }
      if (!parsed.agents.actor) {
        parsed.agents.actor = {
          model: parsed.default_model || 'anthropic.haiku',
          enabled: true,
          temperature: 0.7,
          maxTokens: 4000,
        };
      }
      
      return parsed;
    } catch (error) {
      console.log(`⚠️  Could not read existing config: ${error}`);
      console.log('Creating fresh configuration...');
    }
  }

  return createFreshConfig();
}

/**
 * Mask API key for display
 */
function maskApiKey(apiKey: string): string {
  if (!apiKey || apiKey.length < 8) return '[not set]';
  return `${apiKey.substring(0, 6)}...${apiKey.substring(apiKey.length - 4)}`;
}

/**
 * Get all available providers from config
 */
function getAllProviders(config: CodeLoopsConfig): string[] {
  return Object.keys(config.providers);
}

/**
 * Configure API keys for providers
 */
async function configureApiKeys(config: CodeLoopsConfig): Promise<void> {
  console.log('\n🔑 API Key Configuration');
  console.log('Enter API keys for each provider (leave blank to skip)\n');

  const allProviders = getAllProviders(config);

  for (const provider of allProviders) {
    try {
      const currentKey = config.providers[provider]?.api_key;
      const maskedKey = maskApiKey(currentKey || '');

      console.log(`\n${provider.charAt(0).toUpperCase() + provider.slice(1)} API Key:`);
      if (currentKey) {
        console.log(`  Current: ${maskedKey}`);
      }

      // Special handling for generic provider
      const isGeneric = provider === 'generic';
      const promptMessage = isGeneric
        ? `Enter ${provider} API key or custom value (often 'ollama', or press Enter to ${currentKey ? 'keep current' : 'skip'}):`
        : `Enter ${provider} API key (or press Enter to ${currentKey ? 'keep current' : 'skip'}):`;

      const { apiKey } = await inquirer.prompt([
        {
          type: isGeneric ? 'input' : 'password',
          name: 'apiKey',
          message: promptMessage,
          mask: isGeneric ? undefined : '*',
          validate: (input: string) => {
            // Allow empty input to skip/keep current
            if (!input.trim()) return true;
            // Less strict validation for generic provider
            if (isGeneric) return true;
            if (input.length < 10) return 'API key seems too short (minimum 10 characters)';
            return true;
          },
        },
      ]);

      // Only update if user provided a value
      if (apiKey && apiKey.trim()) {
        if (!config.providers[provider]) {
          config.providers[provider] = { models: {} };
        }
        // Ensure the provider object exists and set the API key
        config.providers[provider] = {
          ...config.providers[provider],
          api_key: apiKey.trim()
        };
        console.log(`  ✅ Updated ${provider} API key`);
      } else if (currentKey) {
        console.log(`  ℹ️  Keeping current ${provider} API key`);
      } else {
        console.log(`  ⏭️  Skipped ${provider} API key`);
      }
    } catch (providerError) {
      console.error(`Error configuring ${provider} API key:`, providerError);
    }
  }
}

/**
 * Non-interactive setup function
 */
function nonInteractiveSetup(config: CodeLoopsConfig, args: any): void {
  // Set default model
  if (args.model) {
    config.default_model = args.model;
  }

  // Configure API keys
  if (args['anthropic-key']) {
    if (!config.providers.anthropic) {
      config.providers.anthropic = { models: {} };
    }
    config.providers.anthropic.api_key = args['anthropic-key'];
  }

  if (args['openai-key']) {
    if (!config.providers.openai) {
      config.providers.openai = { models: {} };
    }
    config.providers.openai.api_key = args['openai-key'];
  }

  if (args['generic-key']) {
    if (!config.providers.generic) {
      config.providers.generic = { models: {} };
    }
    config.providers.generic.api_key = args['generic-key'];
  }

  // Configure agents
  const model = args.model || config.default_model || 'anthropic.haiku';
  
  if (!config.agents) {
    config.agents = {};
  }

  // Set critic agent
  config.agents.critic = {
    model: args['critic-model'] || model,
    enabled: args['disable-critic'] !== 'true',
    temperature: parseFloat(args['critic-temperature'] || '0.3'),
    maxTokens: parseInt(args['critic-max-tokens'] || '2000'),
  };

  // Set actor agent
  config.agents.actor = {
    model: args['actor-model'] || model,
    enabled: true,
    temperature: parseFloat(args['actor-temperature'] || '0.7'),
    maxTokens: parseInt(args['actor-max-tokens'] || '4000'),
  };

  // Configure telemetry and logging
  if (!config.telemetry) config.telemetry = {};
  config.telemetry.enabled = args.telemetry === 'true';
  
  if (!config.features) config.features = {};
  config.features.telemetry_enabled = args.telemetry === 'true';
  
  if (!config.logging) config.logging = { file_logging: {} };
  config.logging.level = args['log-level'] || 'info';
  config.logging.file_logging.enabled = args['file-logging'] === 'true';
}

/**
 * Main setup function
 */
async function main() {
  // Parse command line arguments
  const { values: args } = parseArgs({
    args: process.argv.slice(2),
    options: {
      'non-interactive': {
        type: 'boolean',
        default: false,
      },
      'anthropic-key': {
        type: 'string',
      },
      'openai-key': {
        type: 'string',
      },
      'generic-key': {
        type: 'string',
      },
      'model': {
        type: 'string',
        default: 'anthropic.haiku',
      },
      'critic-model': {
        type: 'string',
      },
      'actor-model': {
        type: 'string',
      },
      'disable-critic': {
        type: 'string',
      },
      'critic-temperature': {
        type: 'string',
      },
      'critic-max-tokens': {
        type: 'string',
      },
      'actor-temperature': {
        type: 'string',
      },
      'actor-max-tokens': {
        type: 'string',
      },
      'telemetry': {
        type: 'string',
        default: 'false',
      },
      'log-level': {
        type: 'string',
        default: 'info',
      },
      'file-logging': {
        type: 'string',
        default: 'false',
      },
      'help': {
        type: 'boolean',
        default: false,
      },
    },
  });

  // Show help if requested
  if (args.help) {
    console.log(`
CodeLoops Setup Script

Usage:
  bun run setup [options]

Options:
  --non-interactive        Run in non-interactive mode
  --anthropic-key <key>    Set Anthropic API key
  --openai-key <key>       Set OpenAI API key
  --generic-key <key>      Set Generic provider API key
  --model <model>          Set default model (default: anthropic.haiku)
  --critic-model <model>   Set critic agent model
  --actor-model <model>    Set actor agent model
  --disable-critic <bool>  Disable critic agent (true/false)
  --critic-temperature <n> Set critic temperature (default: 0.3)
  --critic-max-tokens <n>  Set critic max tokens (default: 2000)
  --actor-temperature <n>  Set actor temperature (default: 0.7)
  --actor-max-tokens <n>   Set actor max tokens (default: 4000)
  --telemetry <bool>       Enable telemetry (true/false, default: false)
  --log-level <level>      Set log level (error/warn/info/debug, default: info)
  --file-logging <bool>    Enable file logging (true/false, default: false)
  --help                   Show this help message

Examples:
  # Interactive mode (default)
  bun run setup

  # Non-interactive with Anthropic
  bun run setup --non-interactive --anthropic-key="sk-ant-..." --model="anthropic.haiku"

  # Non-interactive with custom agent settings
  bun run setup --non-interactive --anthropic-key="sk-ant-..." --critic-model="anthropic.haiku" --actor-model="anthropic.sonnet"
`);
    process.exit(0);
  }

  // Load existing config or create fresh one
  const config = loadConfig();

  if (args['non-interactive']) {
    console.log('\n📋 Running in non-interactive mode...');
    nonInteractiveSetup(config, args);
    
    // Ensure config directory exists
    const configDir = path.dirname(CONFIG_FILE_PATH);
    if (!fs.existsSync(configDir)) {
      fs.mkdirSync(configDir, { recursive: true });
    }

    // Write configuration
    fs.writeFileSync(CONFIG_FILE_PATH, JSON.stringify(config, null, 2));

    console.log(`
✅ Configuration saved successfully!

📍 Location: ${CONFIG_FILE_PATH}
🎯 Default model: ${config.default_model}
🤖 Critic agent: ${config.agents.critic?.enabled ? config.agents.critic.model : 'disabled'}
🤖 Actor agent: ${config.agents.actor?.model || 'not configured'}

Happy coding! 🚀
`);
    process.exit(0);
  }
  console.log(`
╔═══════════════════════════════════════════════════════════════╗
║                    CodeLoops Interactive Setup                ║
╚═══════════════════════════════════════════════════════════════╝
`);

  const availableModels = getAvailableModels(config);

  const configExists = fs.existsSync(CONFIG_FILE_PATH);
  if (configExists) {
    console.log(`📄 Found existing configuration at: ${CONFIG_FILE_PATH}`);
  } else {
    console.log(`🆕 Creating new configuration at: ${CONFIG_FILE_PATH}`);
  }

  // Main configuration menu
  const { configSections } = await inquirer.prompt([
    {
      type: 'checkbox',
      name: 'configSections',
      message: 'What would you like to configure?',
      choices: [
        { name: '🎯 Default model', value: 'default_model', checked: !configExists },
        {
          name: '🤖 Agent models (critic and actor)',
          value: 'agents',
          checked: !configExists,
        },
        { name: '🔑 API keys', value: 'api_keys', checked: !configExists },
        {
          name: '🔧 Advanced settings (telemetry, logging)',
          value: 'advanced',
          checked: false,
        },
      ],
    },
  ]);

  // Configure default model
  if (configSections.includes('default_model')) {
    console.log(`\n🎯 Current default model: ${config.default_model}`);

    const { defaultModel } = await inquirer.prompt([
      {
        type: 'list',
        name: 'defaultModel',
        message: 'Select the default model for CodeLoops:',
        choices: availableModels,
        default: config.default_model,
      },
    ]);

    config.default_model = defaultModel;
  }

  // Configure agent models (only critic and actor now)
  if (configSections.includes('agents')) {
    console.log('\n🤖 Agent Configuration');
    console.log(`Current settings:
  • Critic: ${config.agents.critic?.model || 'not set'} (enabled: ${config.agents.critic?.enabled ?? true})
  • Actor: ${config.agents.actor?.model || 'not set'} (enabled: ${config.agents.actor?.enabled ?? true})`);

    // Configure Critic
    const { criticEnabled } = await inquirer.prompt([
      {
        type: 'confirm',
        name: 'criticEnabled',
        message: 'Enable Critic agent (code review)?',
        default: config.agents.critic?.enabled ?? true,
      },
    ]);

    let criticModel = config.agents.critic?.model || config.default_model;
    if (criticEnabled) {
      const result = await inquirer.prompt([
        {
          type: 'list',
          name: 'criticModel',
          message: 'Select model for Critic agent (code review):',
          choices: availableModels,
          default: criticModel,
        },
      ]);
      criticModel = result.criticModel;
    }

    // Configure Actor
    const { actorModel } = await inquirer.prompt([
      {
        type: 'list',
        name: 'actorModel',
        message: 'Select model for Actor agent (code generation):',
        choices: availableModels,
        default: config.agents.actor?.model || config.default_model,
      },
    ]);

    // Update agent configurations
    if (!config.agents) {
      config.agents = {};
    }
    
    config.agents.critic = {
      model: criticModel,
      enabled: criticEnabled,
      temperature: config.agents.critic?.temperature || 0.3,
      maxTokens: config.agents.critic?.maxTokens || 2000,
    };

    config.agents.actor = {
      model: actorModel,
      enabled: true,
      temperature: config.agents.actor?.temperature || 0.7,
      maxTokens: config.agents.actor?.maxTokens || 4000,
    };
  }

  // Configure API keys
  if (configSections.includes('api_keys')) {
    await configureApiKeys(config);
  }

  // Configure advanced settings
  if (configSections.includes('advanced')) {
    console.log('\n🔧 Advanced Configuration');

    const { telemetryEnabled } = await inquirer.prompt([
      {
        type: 'confirm',
        name: 'telemetryEnabled',
        message: 'Enable telemetry (helps improve CodeLoops)?',
        default: config.telemetry?.enabled ?? false,
      },
    ]);

    const { logLevel } = await inquirer.prompt([
      {
        type: 'list',
        name: 'logLevel',
        message: 'Select log level:',
        choices: [
          { name: 'Error only', value: 'error' },
          { name: 'Warnings and errors', value: 'warn' },
          { name: 'Info, warnings, and errors', value: 'info' },
          { name: 'Debug (verbose)', value: 'debug' },
        ],
        default: config.logging?.level || 'info',
      },
    ]);

    const { fileLogging } = await inquirer.prompt([
      {
        type: 'confirm',
        name: 'fileLogging',
        message: 'Enable file logging?',
        default: config.logging?.file_logging?.enabled ?? false,
      },
    ]);

    // Update advanced settings
    if (!config.telemetry) config.telemetry = {};
    config.telemetry.enabled = telemetryEnabled;
    
    if (!config.features) config.features = {};
    config.features.telemetry_enabled = telemetryEnabled;
    
    if (!config.logging) config.logging = { file_logging: {} };
    config.logging.level = logLevel;
    config.logging.file_logging.enabled = fileLogging;
  }

  // Ensure config directory exists
  const configDir = path.dirname(CONFIG_FILE_PATH);
  if (!fs.existsSync(configDir)) {
    fs.mkdirSync(configDir, { recursive: true });
  }

  // Write configuration
  fs.writeFileSync(CONFIG_FILE_PATH, JSON.stringify(config, null, 2));

  console.log(`
✅ Configuration saved successfully!

📍 Location: ${CONFIG_FILE_PATH}
🎯 Default model: ${config.default_model}
🤖 Critic agent: ${config.agents.critic?.enabled ? config.agents.critic.model : 'disabled'}
🤖 Actor agent: ${config.agents.actor?.model || 'not configured'}

Next steps:
1. Review the configuration file if needed
2. Run CodeLoops with your new settings
3. Use 'bun run setup' anytime to modify settings

Happy coding! 🚀
`);
}

main().catch((error) => {
  console.error('\n❌ Setup failed:', error);
  console.error('\nIf you continue to have issues, you can manually edit the config file at:');
  console.error(CONFIG_FILE_PATH);
  process.exit(1);
});