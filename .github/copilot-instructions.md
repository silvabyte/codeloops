# CodeLoops contributor instructions

This is CodeLoops, a Rust session-history service with a small TypeScript OpenCode
bridge. Follow [AGENTS.md](../AGENTS.md) for issue tracking, code standards, and
verification. Use [development](../docs/DEVELOPMENT.md) for the code map and checks.

Use `bd` for issues and `--json` for programmatic commands. In worktrees, respect
the designated canonical tracker. Include its current `.beads/issues.jsonl` with
related changes. Publish commits or PRs only when requested.

Run `make check` for code changes and `make e2e` when changing installation,
recovery, or export. Keep test archives and client configuration isolated.
