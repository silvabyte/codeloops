# CodeLoops issue tracking

Use [beads](https://github.com/steveyegge/beads), command `bd`, for all repository
issues. CodeLoops itself does not require beads to build or run.

```sh
bd ready --json
bd show ISSUE_ID --json
bd update ISSUE_ID --status in_progress --json
bd create "Fix capture regression" -t bug -p 1 --json
bd close ISSUE_ID --reason "Fixed and verified" --json
```

Run `bd COMMAND --help` for flags. See [AGENTS.md](../AGENTS.md#track-work-with-beads)
for dependencies, worktree handling, and commit rules.

`.beads/issues.jsonl` is the versioned snapshot. The SQLite database is local.
Beads exports changes automatically and imports newer snapshots. Use the canonical
database when working across worktrees; do not replace current state with a stale
snapshot. Include the current JSONL with related commits.
