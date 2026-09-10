# Cursor slice verification

Historical slice report. Hook counts, identity behavior, and manual configuration
below describe this revision. See [current acceptance](verification-install-recovery.md),
[global setup](INSTALL_GUIDE.md), and the [current identity contract](OVERVIEW.md#cursor-hook-identity-and-coverage).

Date: 2026-09-09. Branch: `feat/history-cursor`, based on integrated commit
`82564c06a70ce689b17e246c3e5c173d4079796a`. The base's integrated GitHub CI run
[`34386427461`](https://github.com/silvabyte/codeloops/actions/runs/34386427461)
passed. Checks below ran locally before publication of the commit containing this
report. Desktop verification ran commit
`9539259f5f1c4e5b302df5486dedbf0581b88c9e` on Darwin 25.5.0.

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

## Real-client observations

| Client | Version | Result |
| --- | --- | --- |
| OpenCode (source machine) | 1.18.30, `openai/gpt-6-astra` | Fresh prompt/response captured by the installed build while the service was offline, then searchable through the shared service |
| OpenCode (desktop machine) | 1.17.9, `cursor-acp/auto` | Fresh messages captured and Cursor history retrieved through MCP |
| Cursor CLI | `2026.01.28-fd13201` | Agent execution fails with `Authentication required`; no successful real hook delivery or MCP recall established |
| Cursor desktop Agent Chat | app 3.18.25; hook payload `2026.09.08-6caf4ff` | Native prompt/response/lifecycle hooks delivered and OpenCode history retrieved through MCP |

The desktop machine passed `make check` and installed the locked release build at
`~/.local/codeloops-cursor-preview`. Existing Cursor and OpenCode hooks, plugins,
and MCP servers were preserved during additive configuration. Both clients used
`~/.local/share/codeloops-history/cursor-verification` and `127.0.0.1:47823`.

Cross-client recall succeeded in both directions:

- Cursor `history_query` retrieved OpenCode assistant entry
  `70413a02-fd5b-4c05-b1db-4b60f60d5890`.
- OpenCode `history_query` retrieved Cursor assistant entry
  `6911fb99-ed9d-4fc4-ae7a-c479b8e59120`.
- After a service restart, source-filtered CLI and authenticated REST searches
  returned the same Cursor user/assistant UUIDs:
  `3f61c10d-d98b-4467-a72b-ec68e45beb25` and
  `6911fb99-ed9d-4fc4-ae7a-c479b8e59120`.

Completed turns delivered `beforeSubmitPrompt`, `afterAgentResponse`, then `stop`
with one generation ID and projected the conversation as idle, not ended.
`sessionStart` had no transcript path; prompt/response/stop payloads later supplied
one, and CodeLoops did not read it. A manually stopped long response delivered
`stop` with source status `completed` before `afterAgentResponse`; the latter
contained 10,958 characters of partial output. The archive therefore preserved
the partial response but had no source signal distinguishing the interruption.
Closing the test chat did not deliver `sessionEnd`; its session remained idle.
Desktop subagent relationships were not exposed during this run.

Global hook configuration produced three deliveries per native event while three
Cursor windows were open. The payloads had identical conversation and generation
IDs, so the archive projected one message. Moving the definitions to the trusted
project restored one hook delivery per event. The installation guide at that time
recommended project scope for trials. Existing global and project hooks were restored after the
trial; the additive MCP/plugin entries remain configured.

Final health after restart was 484 delivered, zero pending/rejected/enqueue
failures: Cursor 84 and OpenCode 400. OpenCode's `cursor-acp/auto` model repeatedly
called its unavailable `getMcpTools` helper after the successful reverse lookup,
so that model run was stopped manually; the CodeLoops MCP result itself was correct.

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
**not** complete real cross-client recall on that machine. The desktop run above
subsequently completed cross-client recall.

## Repeated user-text acceptance

Cursor desktop submitted
`pickup @history/handoff.2026-09-09-cursor-verification.md ` twice in one Agent
Chat. The archive retained both user entries with the same content hash
`c0de3fa8ee483e2c121f00d26a05fb6c3222efb674feb493680be108fad94ca7`
and distinct generation-scoped native IDs:

- Entry `a139f5cf-f790-41d7-b1fd-173dae94f3e5`, native ID
  `hook-message:bd8705f8-2537-443e-9831-2dd18363a740`.
- Entry `d89d77d5-7a2e-4433-819e-9d4d96cc6fb9`, native ID
  `hook-message:da73f8c5-e8b2-4911-b159-6df6f215d02c`.

Both entries belong to archive session
`1eebe8ce-9ed4-4220-86c1-b1ee71a0b4e4`. Health after the second submission was
529 delivered, zero pending/rejected/enqueue failures: Cursor 129 and OpenCode
400. This completes the remaining real-client acceptance for
`codeloops-vvb.2`.

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
