# Work on CodeLoops

CodeLoops provides unified conversation history across coding agents. MCP is the
shared access layer; Cursor and OpenCode are today's automatic capture integrations. Read
[development](docs/DEVELOPMENT.md) for commands and the code map, and
[contracts](docs/OVERVIEW.md) before changing capture, storage, or retrieval.

## Track work with beads

Use `bd` for all issue tracking. Do not create markdown task lists or use another
tracker. Use `--json` for programmatic commands and `bd COMMAND --help` to check
available flags.

```sh
bd ready --json
bd show ISSUE_ID --json
bd update ISSUE_ID --status in_progress --json
bd create "Found bug" -t bug -p 1 --deps discovered-from:ISSUE_ID --json
bd close ISSUE_ID --reason "Completed and verified" --json
```

Claim ready work before changing it. Link discovered work to its source issue.
Priorities run from 0 (critical) to 4 (backlog); 2 is the default.

Beads auto-exports changes to `.beads/issues.jsonl` and imports newer snapshots.
In multiple worktrees, use the designated canonical database. If a handoff warns
that worktree snapshots are stale, run `bd --no-auto-import ... --json` in the
canonical checkout. Do not import an older snapshot over current issue state.
Include the current canonical JSONL with the related commit; never commit the
SQLite database. Commit, push, or open a PR only when the user requests it.

## Code standards

- Keep functions focused and interfaces small. Follow nearby structure and naming.
- Rust: use explicit error propagation, preserve typed boundaries, and keep
  database handles and artifact paths inside the storage library.
- TypeScript: keep the bridge thin and type-safe. Prefer `unknown` to `any`,
  `const`, explicit narrowing, `for...of`, and specific imports. Avoid barrel files.
- Await promises and handle failures where they can be reported meaningfully.
  Do not swallow capture errors or mutate client prompts/tool results.
- Preserve original source payloads, durable delivery IDs, and partial coverage.
  Do not infer source identity from equal text or file authorship from a diff.
- Keep stdout protocol-clean for MCP and hooks. Send diagnostics to stderr.
- Validate input and bound queries. Keep credentials and private test evidence
  out of source, capture records, and reports.
- Prefer integration tests at real storage/transport boundaries. Use isolated
  config/data directories and preserve the user's worktree, index, and clients.

## Checks and documentation

`make check` is the full development gate. Rust uses rustfmt and Clippy;
TypeScript uses Biome with the Ultracite preset through `npm run lint`. Use the
commands in [development](docs/DEVELOPMENT.md#build-and-check), not the Ultracite
wrapper as a CI gate. Run `make e2e` for installation/recovery/export changes.
Review the required UBS scan before committing.

Write terse, direct docs with concrete commands. No em dashes. Keep current user
instructions separate from revision-specific verification evidence. Record what
was tested without upgrading older results to new acceptance.

Product positioning: "Unified conversation history across all your coding agents."
Lead with continuity across harnesses. Keep current capture integrations in the
support details; they do not define the product's scope.

Store new AI planning/design artifacts under `history/`, not the repository root.
Only read existing `history/` content when asked to review past planning. Preserve
unfamiliar changes until you understand who owns them.
