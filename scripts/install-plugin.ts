#!/usr/bin/env bun

/**
 * CodeLoops Plugin Installation Script
 * Installs the bundled memory plugin, agents, and skills for OpenCode
 */

import {
  existsSync,
  lstatSync,
  mkdirSync,
  readdirSync,
  rmSync,
  symlinkSync,
} from "node:fs";
import { dirname, join } from "node:path";
import { $ } from "bun";

// Text formatting (ANSI codes)
const BOLD = "\x1b[1m";
const GREEN = "\x1b[0;32m";
const YELLOW = "\x1b[0;33m";
const BLUE = "\x1b[0;34m";
const RED = "\x1b[0;31m";
const NC = "\x1b[0m"; // No Color

const PLUGIN_NAME = "codeloops.js";
const ROOT_DIR = join(dirname(import.meta.path), "..");
const SOURCE_DIR = join(ROOT_DIR, "dist");
const OPENCODE_CONFIG = join(process.env.HOME || "", ".config/opencode");
const TARGET_DIR = join(OPENCODE_CONFIG, "plugin");
const AGENTS_SOURCE = join(ROOT_DIR, "agents");
const AGENTS_TARGET = join(OPENCODE_CONFIG, "agent");
const SKILLS_SOURCE = join(ROOT_DIR, "skills");
const SKILLS_TARGET = join(OPENCODE_CONFIG, "skill");

/**
 * Create a symlink, removing any existing file/symlink first.
 */
function createSymlink(source: string, target: string): void {
  if (existsSync(target) || lstatSync(target, { throwIfNoEntry: false })) {
    rmSync(target, { force: true });
  }
  symlinkSync(source, target);
}

/**
 * Install symlinks for all files in a directory.
 */
function installDirectorySymlinks(
  sourceDir: string,
  targetDir: string,
  label: string
): void {
  if (!existsSync(sourceDir)) {
    console.log(`${YELLOW}No ${label} found at ${sourceDir}${NC}`);
    return;
  }

  if (!existsSync(targetDir)) {
    console.log(`Creating ${label} directory: ${BOLD}${targetDir}${NC}`);
    mkdirSync(targetDir, { recursive: true });
  }

  const items = readdirSync(sourceDir);
  for (const item of items) {
    const sourcePath = join(sourceDir, item);
    const targetPath = join(targetDir, item);

    console.log(
      `  Linking ${label}: ${BOLD}${item}${NC} -> ${BOLD}${sourcePath}${NC}`
    );
    createSymlink(sourcePath, targetPath);
  }
}

console.log(`${BOLD}${BLUE}CodeLoops Plugin Installer${NC}\n`);

// Check if bundled plugin exists, build if not
if (!existsSync(join(SOURCE_DIR, PLUGIN_NAME))) {
  console.log(`${YELLOW}Plugin bundle not found. Building...${NC}`);

  const result = await $`bun run scripts/build-plugin.ts`.cwd(ROOT_DIR).quiet();

  if (result.exitCode !== 0 || !existsSync(join(SOURCE_DIR, PLUGIN_NAME))) {
    console.log(
      `${RED}Error: Build failed. Plugin not found at ${SOURCE_DIR}/${PLUGIN_NAME}${NC}`
    );
    process.exit(1);
  }

  console.log(`${GREEN}Build complete!${NC}\n`);
}

// Create target directory if it doesn't exist
if (!existsSync(TARGET_DIR)) {
  console.log(`Creating plugin directory: ${BOLD}${TARGET_DIR}${NC}`);
  mkdirSync(TARGET_DIR, { recursive: true });
}

const targetPath = join(TARGET_DIR, PLUGIN_NAME);
const sourcePath = join(SOURCE_DIR, PLUGIN_NAME);

// Check if plugin already exists (file or symlink)
if (
  existsSync(targetPath) ||
  lstatSync(targetPath, { throwIfNoEntry: false })
) {
  console.log(`${YELLOW}Plugin already exists. Removing old version...${NC}`);
  rmSync(targetPath, { force: true });
}

// Create symlink to bundled plugin
console.log(
  `Creating symlink: ${BOLD}${targetPath}${NC} -> ${BOLD}${sourcePath}${NC}`
);
symlinkSync(sourcePath, targetPath);

console.log(`\n${GREEN}Plugin installed successfully!${NC}`);

// Install agents
console.log(`\n${BOLD}Installing agents...${NC}`);
installDirectorySymlinks(AGENTS_SOURCE, AGENTS_TARGET, "agent");

// Install skills
console.log(`\n${BOLD}Installing skills...${NC}`);
installDirectorySymlinks(SKILLS_SOURCE, SKILLS_TARGET, "skill");

console.log(`\n${BOLD}Available tools:${NC}`);
console.log("  - memory_store   Store a memory for later recall");
console.log("  - memory_recall  Query stored memories");
console.log("  - memory_forget  Soft-delete a memory");
console.log("  - memory_context Get recent memories for context");
console.log("  - memory_projects List all projects with memories");

console.log(`\n${BOLD}Auto-capture events:${NC}`);
console.log("  - file.edited    Captures file edits with conversation context");
console.log("  - todo.updated   Captures todo list changes");
console.log("  - session.created Initializes session tracking");

console.log(`\n${BOLD}Actor-Critic system:${NC}`);
console.log("  - actor agent    Primary agent with feedback awareness");
console.log("  - critic agent   Analyzes actions and provides feedback");
console.log("  - actor-critic-protocol skill   Detailed protocol guidance");
console.log("  - Configure in ~/.config/codeloops/config.json");

console.log(`\n${BOLD}bd (beads) integration:${NC}`);
console.log("  - Detects TODO comments in file edits");
console.log("  - Spawns agent to create bd issues with context");
console.log("  - Configure model in ~/.config/codeloops/config.json");

console.log(`\n${BOLD}Data location:${NC}`);
if (process.platform === "darwin") {
  console.log("  ~/Library/Application Support/codeloops/");
} else if (process.platform === "win32") {
  console.log("  %APPDATA%/codeloops/");
} else {
  console.log("  ~/.local/share/codeloops/");
}
