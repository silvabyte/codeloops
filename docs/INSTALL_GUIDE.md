# CodeLoops: Installation Guide

## Prerequisites

Before starting, ensure you have the following dependencies

- **Node.js**: Version 18 or higher
  - Download from [nodejs.org](https://nodejs.org) or use a version manager like `nvm`
  - Verify with: `node --version`
- **API Keys**: Required for your chosen LLM provider (e.g., Anthropic, OpenAI)
  - Obtain keys from your provider’s dashboard

## Installation Steps

### Step 1: Clone the Repository

1. Open a terminal and clone the CodeLoops repository:
   ```bash
   git clone https://github.com/SilvaByte/codeloops.git
   ```
2. Navigate to the project directory:
   ```bash
   cd codeloops
   ```
3. Verify the repository structure:
   ```bash
   ls
   ```
   You should see directories like `src`, `agents`, and files like `package.json`.

### Step 2: Understand the Project Structure

CodeLoops is a unified TypeScript application with integrated components:

- **MCP Server** (TypeScript): Manages the CodeLoops system and Knowledge Graph.
- **Agent Components** (TypeScript): Includes integrated Critic and Summarizer agents for evaluating and condensing information.

Key directories:

```
codeloops/
├── src/                # Core TypeScript application
│   ├── engine/         # Actor-Critic engine and Knowledge Graph
│   ├── agents/         # TypeScript agent implementations
│   │   ├── CriticAgent.ts     # Quality evaluation agent
│   │   ├── SummarizerAgent.ts # Branch summarization agent
│   │   └── BaseAgent.ts       # Shared agent framework
│   ├── config/         # Configuration management
│   └── server/         # MCP server implementations
├── package.json        # Node.js dependencies
└── README.md           # Project documentation
```

### Step 3: Install Node.js Dependencies

1. From the project root (`codeloops/`), install Node.js dependencies:
   ```bash
   npm install
   ```
2. Verify installation:
   ```bash
   npm list
   ```
   Ensure no errors appear, and dependencies like `typescript` and `tsx` are listed.

### Step 4: Configure API Keys

CodeLoops requires API keys for LLM providers. You can configure these using the setup script or manually:

#### Option 1: Using Setup Script (Recommended)

```bash
npm run setup
```

This script will guide you through API key configuration.

#### Option 2: Manual Configuration

Create or edit `codeloops.config.json` in the project root:

```json
{
  "providers": {
    "anthropic": {
      "api_key": "your-anthropic-api-key"
    },
    "openai": {
      "api_key": "your-openai-api-key"
    }
  },
  "default_model": "anthropic.claude-3-haiku-20240307"
}
```

#### Option 3: Environment Variables

Set environment variables:

```bash
export ANTHROPIC_API_KEY="your-anthropic-api-key"
export OPENAI_API_KEY="your-openai-api-key"
```

### Step 5: Test the MCP Server

CodeLoops supports both stdio and HTTP transports. Test both to ensure proper functionality:

#### Option 1: Test Stdio Transport (Default)

1. Start the MCP server:
   ```bash
   npx -y tsx src
   ```
2. The server should start without any errors and wait for input via stdio

#### Option 2: Test HTTP Transport

1. Start the HTTP server:
   ```bash
   npm run start:http
   # or with custom options
   npx -y tsx src --http --port 3000
   ```
2. The server should start and display:
   ```
   CodeLoops HTTP server running on http://0.0.0.0:3000
   ```
3. Test the server health endpoint:
   ```bash
   curl http://localhost:3000/health
   ```
   You should receive a JSON response indicating the server is running.

To stop the HTTP server, use `Ctrl+C`.

### Step 6: Verify Installation

Test that the TypeScript compilation and configuration are working correctly:

```bash
# Run TypeScript type checking
npm run check-types

# Run tests (optional)
npm test

# Check configuration
npx tsx src --help
```

If all commands run without errors, your CodeLoops installation is complete and ready to use with your agent!
