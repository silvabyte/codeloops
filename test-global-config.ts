#!/usr/bin/env node

import { getConfig } from './src/config/index.ts';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';

try {
  console.log('Testing global config...');

  const expectedConfigDir = path.join(os.homedir(), '.config', 'codeloops');
  const expectedConfigPath = path.join(expectedConfigDir, 'codeloops.config.json');

  console.log('Expected config directory:', expectedConfigDir);
  console.log('Expected config file path:', expectedConfigPath);

  const config = getConfig();
  const actualConfigPath = config.path;

  console.log('Actual config path:', actualConfigPath);
  console.log('Config directory exists:', fs.existsSync(path.dirname(actualConfigPath)));
  console.log('Config file exists:', fs.existsSync(actualConfigPath));

  // Test that it's indeed using the global location
  if (actualConfigPath === expectedConfigPath) {
    console.log('✅ SUCCESS: Config is using global location');
  } else {
    console.log('❌ FAILURE: Config is not using expected global location');
    console.log('  Expected:', expectedConfigPath);
    console.log('  Actual:  ', actualConfigPath);
  }

  // Test basic config functionality
  console.log('Default model:', config.get('default_model'));
  console.log('Critic agent enabled:', config.get('agents.critic.enabled'));
} catch (error) {
  console.error('Error testing config:', error);
  process.exit(1);
}
