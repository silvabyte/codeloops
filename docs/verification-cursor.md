# Cursor slice verification

Date: 2026-09-09. Branch: `feat/history-cursor`, based on integrated commit
`82564c06a70ce689b17e246c3e5c173d4079796a`. The base's integrated GitHub CI run
[`34386427461`](https://github.com/silvabyte/codeloops/actions/runs/34386427461)
passed. Checks below ran locally before publication of the commit containing this
report. Record `git rev-parse HEAD` when verifying the branch on another machine.
Continue with `history/handoff.2026-09-09-cursor-verification.md`.

## Automated and installed verification

- `make check`: Rust formatting, Clippy with warnings denied, real SQLite/artifact
  persistence and transport tests, OpenCode bridge typecheck/lint/test.
- `make install PREFIX=/tmp/opencode/codeloops-cursor-preview`: locked release
  binary, loadable OpenCode bridge, and Cursor hook example installed successfully.
- Transport integration test captures both sources offline, starts/restarts the
  service, and retrieves Cursor and OpenCode messages through CLI, REST, and an
  official-SDK MCP client. Search results and stable IDs match across surfaces;
  identical native conversation IDs in different clients produce distinct sessions.
- Cursor fixtures exercise repeated equal prompts, multiple assistant messages
  per generation, generation-scoped parent links, parallel hook processes, multiple
  workspace roots, absent/stale transcript paths, missing client version, source
  payload preservation, explicit parent conversations, and subagent observations.
- Lifecycle fixtures distinguish turn stop, interrupted turns, and session end;
  a delayed sessionStart and a child's stop do not reset the parent's idle state.
- Replay test simulates archive commit before queue acknowledgment, then flushes
  without duplicate capture records. Invalid input rolls back the entire enqueue,
  emits neutral `{}`, and increments Cursor health. Initialization failure also
  emits `{}` rather than blocking the client's action.
- Upgrade fixture seeds the first slice's spool schema, device/installation IDs,
  health counters, observation sequence, and text-delta baseline; the shared
  collector preserves them and resumes capture correctly.

Fixtures are derived from the [documented Cursor hook
schemas](https://cursor.com/docs/agent/hooks), consulted on 2026-09-09. They are
not presented as recorded desktop events.

## Real-client observations and blockers

| Client | Version | Result |
| --- | --- | --- |
| OpenCode | 1.18.30, `openai/gpt-6-astra` | Fresh prompt/response captured by the installed build while the service was offline, then searchable through the shared service |
| Cursor CLI | `2026.01.28-fd13201` | Agent execution fails with `Authentication required`; no successful real hook delivery or MCP recall established |
| Cursor desktop Agent Chat | Unavailable | `cursor --version` reports no IDE installation; real desktop hook timing/delivery remains unverified |

The CLI's `status` command says `Login successful` but cannot fetch user details.
Actual `--print` executions require authentication, both with a temporary profile
and the existing login configuration. This is an environment blocker, not proof
of capture-hook support or failure. The CLI is not a substitute for the desktop
acceptance target.

Latest probe artifacts: `/tmp/opencode/history-cursor-live-azu2kdau`.
Walkthrough script: `/tmp/opencode/verify-codeloops-cursor.py`. The script stops its
service on exit. Hooks and MCP entries were written only to a temporary workspace;
the user's existing Cursor hook/MCP/configuration and authentication files were
hash-checked unchanged. Temporary authentication copies were removed on exit.

The probe successfully archived an OpenCode assistant response containing
`OPENCODE_CROSS_RECALL_9B27`; it stopped when Cursor could not authenticate. It did
**not** complete real cross-client recall. Issue `codeloops-vvb.2` remains in progress
until the required real-client verification passes.

## Completing real-client acceptance

Install/open Cursor desktop Agent Chat with a working login. Follow the additive
hook/MCP configuration in [the installation guide](INSTALL_GUIDE.md), using one
shared preview profile. Record the actual desktop version and tested code commit.

Capture two identical user prompts, multiple completed assistant messages, and
an interrupted turn. Check health and original payloads to establish which hooks
were really delivered, their ordering, generation IDs, and transcript availability.
Verify a completed turn is idle rather than ended and record whether sessionEnd
is delivered on conversation/window closure. Exercise subagent relationships only
where the source actually exposes them.

Ask Cursor to retrieve a distinctive OpenCode assistant message through
`history_query`, recording its stable entry UUID. Ask OpenCode to retrieve the
new Cursor assistant message in return. Confirm CLI/REST return the same UUIDs
after service restart. Record hooks/settings preservation and any missing events.
Successful fixtures alone do not close this acceptance requirement.

## Coverage and scan review

Only the seven hooks in the installed example are enabled in this slice. Prompts
are observations before submission, so a different hook can still block the
submission afterward. Completed response hooks do not promise partial/interrupted
output, every internal model message, or complete subagent transcripts. Optional
transcript paths are never read. Attachments are metadata only; normalized tools,
Git checkpoints, complete setup/recovery, and export remain dependent slices.

UBS scanned the application crate (`/tmp/opencode/codeloops-cursor-ubs.json`).
Its four critical heuristics are the test startup panic and existing code that
generates/reads credentials, not hardcoded credentials. Warnings concern test
assertions/unwraps and SQL-construction heuristics; collector SQL uses fixed
statements with bound values. Synchronous SQLite capture is intentional at the
hook boundary, and the service runs flush/query work in blocking workers. No
confirmed new defect was identified by that scan review.
