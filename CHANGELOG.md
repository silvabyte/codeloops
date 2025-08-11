# Changelog

## [0.5.1] - 2025-06-06

- feat: add diff control options to resume command (#43)
- chore: release v0.5.0

## [0.5.0] - 2025-06-06

- feat: add HTTP server support with modular transport architecture (#41)
- feat: add safe soft delete functionality for knowledge graph nodes
- chore: release v0.4.0

## [0.4.0] - 2025-06-05

- test: add comprehensive unit tests for git utility functions
- refactor: extract git diff logic to dedicated utility module
- feat: enhance diff tracking with automatic git diff generation
- feat: add diff tracking to knowledge graph nodes
- Add Claude PR Assistant workflow
- feat: add brand assets and logo files to media directory
- chore: release v0.3.8

## [0.3.8] - 2025-05-28

- chore: switch display to false in template config
- chore: release v0.3.7

## [0.3.7] - 2025-05-26

- fix: json output and artifacts schema
- chore: release v0.3.6

## [0.3.6] - 2025-05-22

- chore: update eslint config
- chore: move lint to pre-commit
- chore: remove dup config
- feat: add eslint
- chore: general cleanup
- chore: release v0.3.5

## [0.3.5] - 2025-05-21

- fix: chatty summarize response
- chore: release v0.3.4

## [0.3.4] - 2025-05-19

- feat: implement initial fix and enhanced logging
- chore: release v0.3.3

## [0.3.3] - 2025-05-17

- feat: silence test logs
- chore: update readme
- chore: release v0.3.2

## [0.3.2] - 2025-05-16

- chore: fix unit test
- feat: add get node tool and remove project from getNode
- feat: iniitial branch label removal
- chore: release v0.3.1

## [0.3.1] - 2025-05-16

- refactor: simplify export functionality by removing filterTag
- chore: release v0.3.0

## [0.3.0] - 2025-05-16

- chore: remove next steps
- fix: list branches
- feat: enhance KnowledgeGraphManager with async operations
- chore: remove artifacts as individual
- chore: further remove un-needed config
- refactor: improve project context handling in actor-critic workflow
- refactor: migrate to TypeScript with strict type checking and project context
- refactor: rename export_knowledge_graph tool to export for better clarity
- refactor: rename export_plan to export_knowledge_graph and add limit option
- refactor: centralize project loading logic and add per-project logger contexts
- refactor: remove RevisionCounter and simplify critic review logic
- refactor: rename loadProject to tryLoadProject and add unit tests
- chore: remove technical overview
- refactor: rename selectedProject variable to activeProject for clarity
- refactor: migrate to unified NDJSON format and enhance logging with pino-roll
- chore: remove notes
- feat: implement knowledge graph persistence redesign with NDJSON and explicit project context
- refactor: replace console logging with logger usage across multiple files and delete todos.md file
- feat: add logger
- feat: implement project context switching to support multiple concurrent projects
- chore: remove needs more from input schema
- chore: update think descriptioon
- chore: release v0.2.1

## [0.2.1] - 2025-05-10

- chore: update prompt in readme
- feat: add detailed install guide
- chore: minor updates to next stesp
- feat: rework readme
- chore: remove cli.js
- feat: add iniital quickstart scripts
- chore: rename workflow
- feat: add basic ci action
- chore: release v0.2.0

## [0.2.0] - 2025-05-09

- chore: add link to article and bannger img
- feat: add initial rebrand
- chore: add project tool docs
- refactor: document critic_review tool as manual intervention
- refactor: improve type safety and standardize knowledge graph structures
- chore: remove summarize init
- fix: switch projects call
- refactor: improve file operations API and maintain backward compatibility
- chore: release v0.1.0

## [0.1.0] - 2025-05-07

- feat: fix import and refactor structure
- feat: add kg unit tests
- feat: refactor summarization logic out of knowledge graph
- chore: add vitest
- feat: add initial fix
- chore: release v0.0.2

## [0.0.2] - 2025-05-04

- chore: slight tweak to readme
- chore: add more quickstart refinements
- chore: adds uv installation docs link
- feat: add quickstart docs
- chore: update configs and readme quickstart draft
- chore: release v0.0.1

## [0.0.1] - 2025-05-04

- chore: update version and add tidy agent next steps
- chore: format via prettier
- feat: add release tooling
- chore: update next steps
- fix: add summary agent deps
- feat: use actor critic to create summarize agent
- feat: init uv
- fix: dirname
- feat: add exec critic python agent
- chore: add execa
- fix: nvm rc file
- chore: add ignore config files
- feat: add actor agent instructions
- feat: add blank critic agent
- chore: update readme
- chore: rename kg file
- feat: add default kg file
- feat: add basic guards
- feat: update thought description
- chore: update readme
- chore: update readme
- chore: update readme
- chore: clean up list
- chore: update title
- chore: add next steps
- chore: add readme
- feat: refactor actor critic engine
- chore: add running comment
- fix: types and format
- feat: initial commit
- Initial commit
