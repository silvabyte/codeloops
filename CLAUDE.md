# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

CodeLoops is an experimental actor-critic system for improving coding agent autonomy. It provides iterative feedback through a knowledge graph that stores context and enables agents to work more reliably across complex projects. The system integrates with coding agents via MCP (Model Context Protocol).

## Development Commands

### Build & Type Checking

- `bun run build` - Clean and run full build with type checking
- `bun run typecheck` - Run TypeScript type checking across all packages
- `bun run typecheck:watch` - Watch mode for type checking

### Testing

- `bun test` - Run all tests using Bun test runner
- `bun test --watch` - Watch mode for tests

### Code Quality

- `bun run lint` - Run ESLint with zero warnings tolerance
- `bun run lint:fix` - Auto-fix linting issues
- `bun run format` - Format code with Prettier
- `bun run format:check` - Check code formatting
- `bun run check` - Run typecheck, lint, and format check
- `bun run check:fix` - Run typecheck, lint:fix, and format

### Server Operations

- `bun run start` - Start MCP server with stdio transport (default)
- `bun run start:http` - Start MCP server with HTTP transport on port 3000
- `bun run config-ui` - Start configuration UI server

### Setup & Utilities

- `bun run setup` - Interactive setup script for API keys and configuration
- `bun run clean` - Clean all dist directories and build artifacts

## Architecture

### Core Components

**Monorepo Structure**:

- `packages/core/` - Knowledge graph engine, logging, and utilities
- `packages/agents/` - Actor-critic agents and base agent framework
- `packages/server/` - MCP server implementation with stdio/HTTP transports
- `packages/config/` - Configuration management and provider setup
- `packages/config-ui/` - Web-based configuration interface

**Key Classes**:

- `KnowledgeGraphManager` (`packages/core/src/engine/KnowledgeGraph.ts`) - Manages persistent DAG of thought nodes with NDJSON storage
- `ActorCriticEngine` - Orchestrates actor-critic loops
- `BaseAgent` - Unified agent framework wrapping VoltAgent with retry logic and streaming
- MCP Tools registration in `packages/server/src/tools.ts`

### Actor-Critic System

The system follows an actor-critic pattern where:

1. **Actor** generates thoughts/plans and adds nodes to knowledge graph
2. **Critic** evaluates nodes with verdicts: `approved`, `needs_revision`, or `reject`
3. **Knowledge Graph** persists all interactions with parent-child relationships
4. **MCP Server** exposes tools for agent interaction

### Data Flow

```
AI Agent → MCP Tools → ActorCriticEngine → KnowledgeGraph (NDJSON)
                                       ↗
                              Critic Agent
```

## MCP Tools

**Primary Tool**: `actor_think` - Add thought nodes, automatically triggers critic reviews
**Context Tools**: `resume`, `export`, `list_projects`
**Management**: `delete_thoughts` (soft delete with dependency checking)

## Configuration

- Main config: `codeloops.config.json` (created by setup script)
- Supports OpenAI, Anthropic, Azure OpenAI, and custom providers
- Environment variable fallbacks for all API keys
- HTTP transport options: `--http --port <num> --host <addr>`

## Key Patterns

**Project Context**: Tools require `projectContext` (full directory path) to extract project names and scope operations

**Git Integration**: Auto-captures git diffs for thought nodes to track code changes

**Logging**: Structured JSON logging with per-project context switching

**Error Handling**: Comprehensive error handling with retries and graceful fallbacks

## Development Notes

- Uses Bun as runtime and package manager
- TypeScript throughout with strict type checking
- ESLint config with zero warnings policy
- Prettier for consistent formatting
- Commitlint enforces conventional commits
- Husky for git hooks
