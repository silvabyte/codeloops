# Issue tracker

Use Beads (`bd`) for all issue tracking, including work produced by
`to-issues`, `to-prd`, `to-spec`, and `triage`. External PRs are not a
triage surface. GitHub hosts the code at `silvabyte/codeloops`.

Follow the tracking and canonical database rules in [AGENTS.md](../../AGENTS.md).
Use `--json` for programmatic commands and check `bd COMMAND --help`
before using unfamiliar flags.

## Workflow

- Find unblocked work: `bd ready --json`.
- Read an issue: `bd show ISSUE_ID --json`.
- Claim it: `bd update ISSUE_ID --status in_progress --json`.
- Create work: `bd create "Title" -t task -p 2 --json`.
- Link discovered work with `--deps discovered-from:ISSUE_ID`.
- Create child issues with `--parent EPIC_ID`.
- Complete verified work: `bd close ISSUE_ID --reason "Completed and verified" --json`.

Store specs and acceptance criteria in Beads issue fields. Keep supporting
AI planning/design artifacts under `history/`.

Include the current canonical `.beads/issues.jsonl` with related commits.
Never commit the SQLite database. Commit, push, or open a PR only when requested.
