#!/usr/bin/env ts-node

import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';

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

try {
  // Config migration
  const oldConfigPath = path.join(
    os.homedir(),
    'Library',
    'Preferences',
    'codeloops-nodejs',
    'codeloops.config.json',
  );
  const newConfigDir = path.join(os.homedir(), '.config', 'codeloops');
  const newConfigPath = path.join(newConfigDir, 'codeloops.config.json');

  console.log('Migrating config file...');
  copyFileIfNotExists(oldConfigPath, newConfigPath);

  // Data migration
  const oldDataDir = path.resolve('data');
  const newDataDir = path.join(os.homedir(), '.local', 'share', 'codeloops');

  console.log('Migrating data directory...');
  copyDirIfNotExists(oldDataDir, newDataDir);

  console.log('Migration complete.');
} catch (error) {
  console.error('Migration error:', error);
  process.exit(1);
}
