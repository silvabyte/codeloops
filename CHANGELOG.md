# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Comprehensive documentation with mdbook
- GitHub Actions workflow for docs deployment
- This CHANGELOG

## [0.1.0] - 2025-01-27

### Added
- Initial release
- Actor-critic loop orchestration for self-correcting AI coding agents
- Support for Claude Code, OpenCode, and Cursor agents
- Mixed agent configurations (different agents for actor vs critic)
- Session persistence in JSONL format
- CLI commands: run, sessions, ui, init
- Session filtering by outcome, date, project, and search text
- Session statistics and analytics
- Web UI for session browsing and analysis
- Real-time session updates via Server-Sent Events
- Syntax-highlighted diff viewer
- Interactive session picker
- Global and project-level configuration
- Onboarding UX with `init` command

[Unreleased]: https://github.com/matsilva/codeloops/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/matsilva/codeloops/releases/tag/v0.1.0
