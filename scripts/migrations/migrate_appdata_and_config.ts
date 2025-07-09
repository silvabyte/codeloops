#!/usr/bin/env ts-node

import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { APP_PATHS } from '../../src/config/index';

// Default model configurations for commonly referenced models
const DEFAULT_MODELS = {
  anthropic: {
    haiku: {
      id: 'claude-3-haiku-20240307',
      max_tokens: 4096,
      description: 'Fast and efficient model for quick responses',
    },
    sonnet: {
      id: 'claude-3-5-sonnet-20241022',
      max_tokens: 8192,
      description: 'Balanced model for most tasks',
    },
    opus: {
      id: 'claude-3-opus-20240229',
      max_tokens: 4096,
      description: 'Most capable model for complex tasks',
    },
  },
  openai: {
    'gpt-4o-mini': {
      id: 'gpt-4o-mini',
      max_tokens: 16384,
      description: 'Fast and affordable model',
    },
    'gpt-4o': {
      id: 'gpt-4o',
      max_tokens: 4096,
      description: 'Advanced reasoning model',
    },
  },
};

function ensureDirSync(dir: string) {
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }
}

function copyFileIfNotExists(src: string, dest: string) {
  if (!fs.existsSync(src)) {
    console.log(`Source file does not exist: ${src}`);
    return;
  }
  if (fs.existsSync(dest)) {
    console.log(`Destination file already exists, skipping: ${dest}`);
    return;
  }
  ensureDirSync(path.dirname(dest));
  fs.copyFileSync(src, dest);
  console.log(`Copied file: ${src} -> ${dest}`);
}

function copyDirIfNotExists(srcDir: string, destDir: string) {
  if (!fs.existsSync(srcDir)) {
    console.log(`Source directory does not exist: ${srcDir}`);
    return;
  }
  if (fs.existsSync(destDir)) {
    console.log(`Destination directory already exists, skipping: ${destDir}`);
    return;
  }
  ensureDirSync(destDir);
  // Recursively copy
  const entries = fs.readdirSync(srcDir, { withFileTypes: true });
  for (const entry of entries) {
    const srcPath = path.join(srcDir, entry.name);
    const destPath = path.join(destDir, entry.name);
    if (entry.isDirectory()) {
      copyDirIfNotExists(srcPath, destPath);
    } else {
      copyFileIfNotExists(srcPath, destPath);
    }
  }
  console.log(`Copied directory: ${srcDir} -> ${destDir}`);
}

function fixConfigModelReferences(configPath: string) {
  if (!fs.existsSync(configPath)) {
    console.log(`Config file does not exist: ${configPath}`);
    return;
  }

  try {
    const configContent = fs.readFileSync(configPath, 'utf8');
    const config = JSON.parse(configContent);
    
    let configModified = false;

    // Ensure providers exist
    if (!config.providers) {
      config.providers = {};
    }

    // Check each provider and populate missing models
    for (const [providerName, defaultProviderModels] of Object.entries(DEFAULT_MODELS)) {
      if (!config.providers[providerName]) {
        config.providers[providerName] = {};
      }
      
      if (!config.providers[providerName].models) {
        config.providers[providerName].models = {};
      }

      // Add missing models
      for (const [modelName, modelConfig] of Object.entries(defaultProviderModels)) {
        if (!config.providers[providerName].models[modelName]) {
          config.providers[providerName].models[modelName] = modelConfig;
          configModified = true;
          console.log(`Added missing model: ${providerName}.${modelName}`);
        }
      }
    }

    if (configModified) {
      fs.writeFileSync(configPath, JSON.stringify(config, null, '\t'));
      console.log(`Updated config file with missing model definitions: ${configPath}`);
    } else {
      console.log(`Config file already has all required models: ${configPath}`);
    }
  } catch (error) {
    console.error(`Error fixing config file ${configPath}:`, error);
  }
}

try {
  // Config migration
  const oldConfigPath = path.join(
    os.homedir(),
    'Library',
    'Preferences',
    'codeloops-nodejs',
    'codeloops.config.json',
  );
  const newConfigPath = APP_PATHS.config;

  console.log('Migrating config file...');
  copyFileIfNotExists(oldConfigPath, newConfigPath);
  
  // Fix model references in the migrated config
  console.log('Fixing model references in config...');
  fixConfigModelReferences(newConfigPath);
  
  // Also fix the old config file if it exists
  console.log('Fixing model references in old config...');
  fixConfigModelReferences(oldConfigPath);

  // Data migration
  const oldDataDir = path.resolve('data');
  const newDataDir = APP_PATHS.data;

  console.log('Migrating data directory...');
  copyDirIfNotExists(oldDataDir, newDataDir);

  console.log('Migration complete.');
} catch (error) {
  console.error('Migration error:', error);
  process.exit(1);
}
