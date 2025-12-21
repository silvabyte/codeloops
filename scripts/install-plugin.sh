#!/bin/bash

# CodeLoops Plugin Installation Script
# Installs the memory plugin for OpenCode

set -e

# Text formatting
BOLD="\033[1m"
GREEN="\033[0;32m"
YELLOW="\033[0;33m"
BLUE="\033[0;34m"
RED="\033[0;31m"
NC="\033[0m"

PLUGIN_NAME="memory.ts"
SOURCE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/plugin"
TARGET_DIR="$HOME/.config/opencode/plugin"

echo -e "${BOLD}${BLUE}CodeLoops Plugin Installer${NC}\n"

# Check if source plugin exists
if [ ! -f "$SOURCE_DIR/$PLUGIN_NAME" ]; then
  echo -e "${RED}Error: Plugin not found at $SOURCE_DIR/$PLUGIN_NAME${NC}"
  exit 1
fi

# Create target directory if it doesn't exist
if [ ! -d "$TARGET_DIR" ]; then
  echo -e "Creating plugin directory: ${BOLD}$TARGET_DIR${NC}"
  mkdir -p "$TARGET_DIR"
fi

# Check if plugin already exists
if [ -f "$TARGET_DIR/$PLUGIN_NAME" ] || [ -L "$TARGET_DIR/$PLUGIN_NAME" ]; then
  echo -e "${YELLOW}Plugin already exists. Removing old version...${NC}"
  rm -f "$TARGET_DIR/$PLUGIN_NAME"
fi

# Create symlink
echo -e "Creating symlink: ${BOLD}$TARGET_DIR/$PLUGIN_NAME${NC} -> ${BOLD}$SOURCE_DIR/$PLUGIN_NAME${NC}"
ln -s "$SOURCE_DIR/$PLUGIN_NAME" "$TARGET_DIR/$PLUGIN_NAME"

echo -e "\n${GREEN}✅ Plugin installed successfully!${NC}"
echo -e "\n${BOLD}Available tools:${NC}"
echo -e "  - memory_store   Store a memory for later recall"
echo -e "  - memory_recall  Query stored memories"
echo -e "  - memory_forget  Soft-delete a memory"
echo -e "  - memory_context Get recent memories for context"
echo -e "  - memory_projects List all projects with memories"
echo -e "\n${BOLD}Auto-capture events:${NC}"
echo -e "  - file.edited    Captures file edits"
echo -e "  - todo.updated   Captures todo list changes"
echo -e "  - session.created Loads recent memories on start"
echo -e "\n${BOLD}Data location:${NC}"
if [[ "$OSTYPE" == "darwin"* ]]; then
  echo -e "  ~/Library/Application Support/codeloops/"
elif [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "win32" ]]; then
  echo -e "  %APPDATA%/codeloops/"
else
  echo -e "  ~/.local/share/codeloops/"
fi
