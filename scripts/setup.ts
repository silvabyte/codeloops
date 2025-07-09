#!/usr/bin/env tsx
// Run this script with: npx tsx scripts/setup.ts
// Interactive setup for CodeLoops configuration

import fs from 'fs';
import path from 'path';
import inquirer from 'inquirer';
import { createFreshConfig, getConfig, CodeLoopsConfig, ModelConfig } from '../src/config/index.ts';

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
      return JSON.parse(configContent);
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
  try {
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

        const { apiKey } = await inquirer.prompt([{
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
        }]);
        
        // Only update if user provided a value
        if (apiKey && apiKey.trim()) {
          if (!config.providers[provider]) {
            config.providers[provider] = { models: {} };
          }
          config.providers[provider].api_key = apiKey.trim();
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
  } catch (error) {
    console.error('Error in configureApiKeys:', error);
    throw error;
  }
}

/**
 * Main setup function
 */
async function main() {
  console.log(`
╔═══════════════════════════════════════════════════════════════╗
║                    CodeLoops Interactive Setup                ║
╚═══════════════════════════════════════════════════════════════╝
`);

  // Load existing config or create fresh one
  const config = loadConfig();
  const availableModels = getAvailableModels(config);
  
  const configExists = fs.existsSync(CONFIG_FILE_PATH);
  if (configExists) {
    console.log(`📄 Found existing configuration at: ${CONFIG_FILE_PATH}`);
  } else {
    console.log(`🆕 Creating new configuration at: ${CONFIG_FILE_PATH}`);
  }

  // Main configuration menu
  const { configSections } = await inquirer.prompt([{
    type: 'checkbox',
    name: 'configSections',
    message: 'What would you like to configure?',
    choices: [
      { name: '🎯 Default model', value: 'default_model', checked: !configExists },
      { name: '🤖 Agent models (critic, summarizer, actor)', value: 'agents', checked: !configExists },
      { name: '🔑 API keys', value: 'api_keys', checked: !configExists },
      { name: '🔧 Advanced settings (telemetry, logging, features)', value: 'advanced', checked: false },
    ],
  }]);

  // Configure default model
  if (configSections.includes('default_model')) {
    console.log(`\n🎯 Current default model: ${config.default_model}`);
    
    const { defaultModel } = await inquirer.prompt([{
      type: 'list',
      name: 'defaultModel',
      message: 'Select the default model for CodeLoops:',
      choices: availableModels,
      default: config.default_model,
    }]);
    
    config.default_model = defaultModel;
  }

  // Configure agent models
  if (configSections.includes('agents')) {
    console.log('\n🤖 Agent Configuration');
    console.log(`Current settings:
  • Critic: ${config.agents.critic.model} (enabled: ${config.agents.critic.enabled})
  • Summarizer: ${config.agents.summarizer.model} (enabled: ${config.agents.summarizer.enabled})
  • Actor: ${config.agents.actor.model} (enabled: ${config.agents.actor.enabled})`);

    const { criticModel } = await inquirer.prompt([{
      type: 'list',
      name: 'criticModel',
      message: 'Select model for Critic agent (code review):',
      choices: availableModels,
      default: config.agents.critic.model,
    }]);

    const { summarizerEnabled } = await inquirer.prompt([{
      type: 'confirm',
      name: 'summarizerEnabled',
      message: 'Enable Summarizer agent (project summaries)?',
      default: config.agents.summarizer.enabled,
    }]);

    let summarizerModel = config.agents.summarizer.model;
    if (summarizerEnabled) {
      const result = await inquirer.prompt([{
        type: 'list',
        name: 'summarizerModel',
        message: 'Select model for Summarizer agent:',
        choices: availableModels,
        default: config.agents.summarizer.model,
      }]);
      summarizerModel = result.summarizerModel;
    }

    // Update agent configurations
    config.agents.critic.model = criticModel;
    config.agents.summarizer.enabled = summarizerEnabled;
    config.agents.summarizer.model = summarizerModel;
  }

  // Configure API keys
  if (configSections.includes('api_keys')) {
    try {
      await configureApiKeys(config);
    } catch (error) {
      console.error('Failed to configure API keys:', error);
      throw error;
    }
  }

  // Configure advanced settings
  if (configSections.includes('advanced')) {
    console.log('\n🔧 Advanced Configuration');

    const { telemetryEnabled } = await inquirer.prompt([{
      type: 'confirm',
      name: 'telemetryEnabled',
      message: 'Enable telemetry (helps improve CodeLoops)?',
      default: config.telemetry.enabled,
    }]);

    const { logLevel } = await inquirer.prompt([{
      type: 'list',
      name: 'logLevel',
      message: 'Select log level:',
      choices: [
        { name: 'Error only', value: 'error' },
        { name: 'Warnings and errors', value: 'warn' },
        { name: 'Info, warnings, and errors', value: 'info' },
        { name: 'Debug (verbose)', value: 'debug' },
      ],
      default: config.logging.level,
    }]);

    const { fileLogging } = await inquirer.prompt([{
      type: 'confirm',
      name: 'fileLogging',
      message: 'Enable file logging?',
      default: config.logging.file_logging.enabled,
    }]);

    // Update advanced settings
    config.telemetry.enabled = telemetryEnabled;
    config.features.telemetry_enabled = telemetryEnabled;
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
🤖 Critic agent: ${config.agents.critic.model}
🤖 Summarizer agent: ${config.agents.summarizer.enabled ? config.agents.summarizer.model : 'disabled'}

Next steps:
1. Review the configuration file if needed
2. Run CodeLoops with your new settings
3. Use 'npx tsx scripts/setup.ts' anytime to modify settings

Happy coding! 🚀
`);
}

main().catch((error) => {
  console.error('\n❌ Setup failed:', error);
  process.exit(1);
});