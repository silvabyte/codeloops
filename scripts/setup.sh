#!/bin/bash

# CodeLoops Quick Setup Script
# This script automates the setup process for CodeLoops

# Text formatting
BOLD="\033[1m"
GREEN="\033[0;32m"
YELLOW="\033[0;33m"
BLUE="\033[0;34m"
ORANGE="\033[0;38;5;166m"
RED="\033[0;31m"
NC="\033[0m" # No Color

# Print header
echo -e "${BOLD}${ORANGE}
 ██████╗ ██████╗ ██████╗ ███████╗██╗      ██████╗  ██████╗ ██████╗ ███████╗
██╔════╝██╔═══██╗██╔══██╗██╔════╝██║     ██╔═══██╗██╔═══██╗██╔══██╗██╔════╝
██║     ██║   ██║██║  ██║█████╗  ██║     ██║   ██║██║   ██║██████╔╝███████╗
██║     ██║   ██║██║  ██║██╔══╝  ██║     ██║   ██║██║   ██║██╔═══╝ ╚════██║
╚██████╗╚██████╔╝██████╔╝███████╗███████╗╚██████╔╝╚██████╔╝██║     ███████║
 ╚═════╝ ╚═════╝ ╚═════╝ ╚══════╝╚══════╝ ╚═════╝  ╚═════╝ ╚═╝     ╚══════╝

${NC}${BOLD}Memory Layer for AI Coding Agents${NC}
"

echo -e "${BOLD}This script will set up CodeLoops on your system.${NC}"
echo -e "It will check for prerequisites and install dependencies.\n"

# Function to check if a command exists
command_exists() {
  command -v "$1" >/dev/null 2>&1
}

# Step 1: Check prerequisites
echo -e "${BOLD}${BLUE}Step 1: Checking prerequisites...${NC}"

# Check for Node.js
if command_exists node; then
  NODE_VERSION=$(node -v)
  echo -e "✅ ${GREEN}Node.js is installed:${NC} $NODE_VERSION"

  # Check Node.js version
  NODE_MAJOR_VERSION=$(echo $NODE_VERSION | cut -d. -f1 | tr -d 'v')
  if [ "$NODE_MAJOR_VERSION" -lt 18 ]; then
    echo -e "⚠️  ${YELLOW}Warning: Node.js version 18+ is recommended. You have $NODE_VERSION${NC}"
  fi
else
  echo -e "❌ ${RED}Node.js is not installed. Please install Node.js v18+ from https://nodejs.org/${NC}"
  exit 1
fi

echo -e "${GREEN}All prerequisites are satisfied!${NC}\n"

# Step 2: Install Node.js dependencies
echo -e "${BOLD}${BLUE}Step 2: Installing Node.js dependencies...${NC}"
npm install
if [ $? -eq 0 ]; then
  echo -e "✅ ${GREEN}Node.js dependencies installed successfully.${NC}\n"
else
  echo -e "❌ ${RED}Failed to install Node.js dependencies. Please check the error messages above.${NC}"
  exit 1
fi

echo -e "${GREEN}Setup completed successfully!${NC}\n"

# Step 3: Provide instructions for starting the server
echo -e "${BOLD}${BLUE}Step 3: Starting CodeLoops...${NC}"
echo -e "${BOLD}To start the CodeLoops MCP server, run:${NC}"
echo -e "  ${BOLD}npm start${NC}           # stdio mode (for Claude Desktop, etc.)"
echo -e "  ${BOLD}npm run start:http${NC}  # HTTP mode (for web clients)"
echo -e "\n${BOLD}To use with Claude Desktop, add to your config:${NC}"
echo -e '  "codeloops": {'
echo -e '    "command": "npx",'
echo -e '    "args": ["-y", "tsx", "/path/to/codeloops/src"]'
echo -e '  }'
