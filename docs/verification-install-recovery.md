# Installation, recovery and export verification

Date: 2026-09-10. Slice: `codeloops-vvb.4`. Branch:
`feat/history-install-recovery`, based on integrated commit
`b05d410a0a459f5807e90dbf94c73e0af653ad4c`.

## Status

Implementation and local automated/installed-interface verification are available.
The real OpenCode run below tested an uncommitted installation/recovery build.
The completed integration commit and its final two-client acceptance must be
recorded after integration. Earlier Cursor acceptance is user-confirmed and remains
recorded in `verification-cursor.md`; it is not a new desktop run of this slice.

## Automated verification

- `make check`: 19 Rust integration tests, Clippy with warnings denied, Rust
  formatting, TypeScript typechecking, the Ultracite Biome preset and the bridge
  integration test.
- `make e2e`: three installed-interface tests using a locked release binary copied
  through the public `install` command into temporary prefixes.
- CI now runs `make e2e` after the existing check/install jobs on integration-branch
  PRs and pushes.

The installed tests use isolated home/config/data directories and cover:

- Global registration, repeat setup, JSONC comments and unrelated client settings.
- Multiple profiles sharing client configs while retaining distinct executables,
  data, credentials and MCP names; uninstalling one preserves the other.
- Preflight failure on malformed/conflicting settings and recovery of a partially
  applied registration from its durable ownership record.
- Actual installed Cursor hook command execution with spaces and apostrophes in
  executable/data paths, without relying on launcher environment variables.
- Offline capture, more than one queue batch, explicit full drain and service
  restart with stable query results.
- Idempotent explicit deliveries, conflicts on changed delivery content and
  legitimate repeated equal-text messages.
- CLI/REST/official-SDK MCP export descriptor agreement.
- Nested HEAD/index/worktree manifests and exact failed-tool binary bytes after
  deleting the source checkout before delivery.
- Offline bundle verification after deleting the original archive; corruption
  detection and preservation of all captured data during uninstall.
- Repeated uninstall and preservation of an unowned file at a formerly owned path.

Library coverage additionally freezes more than 100 records, long text and all
capture revisions while another connection ingests updates. Each exported current
projection agrees with the revisions in its own snapshot. Missing required capture
artifacts cause an explicit export failure. Existing checkpoint tests cover failed
artifact publication, conflicted index stages, symlinks, raw path bytes, unstable
observations, and unsupported/excluded file coverage.

The bridge test verifies that generated installation options reach the collector
as explicit arguments and preserve source events. `npm run lint` now calls Biome
directly with the existing Ultracite preset: the installed Ultracite wrapper printed
a lint error while returning success during development, so it could not reliably
gate `make check`.

## Real OpenCode installed verification

Client: **OpenCode 1.18.30**, model `openai/gpt-6-astra`, Linux.

Evidence: `/tmp/opencode/history-install-live-pk3v466v`.
Harness: `/tmp/opencode/verify-codeloops-install-live.py`.
Tested binary SHA-256:

```text
a41ca1a41b5aa9de481561aa48b8817d42a5ccd9af3cdf3e9ea9c544df02d461
```

The harness used `make install`, repeated `make setup`, and the generated global
OpenCode plugin/MCP entries in an isolated config directory. No plugin or MCP entry
was injected into the individual projects. It captured a live conversation with a
dirty/staged baseline, a successful shell edit, a nonzero-exit shell edit and a
failed read while the service was offline. The user's original index bytes were
unchanged by capture.

After flushing, it deleted the source project, restarted the service and launched
OpenCode in a second project. Through the generated MCP registration, the agent
exported the earlier session and retrieved its captured file bytes. The CLI also
verified staged/unstaged versions and downloaded a complete portable bundle. Offline
verification passed after stopping the service. `make uninstall` removed installed
assets and retained the archive; repeating it succeeded.

Final health: **206 delivered**, zero pending, rejected or enqueue failures, and
zero workspaces with failed checkpoint capture. The evidence directory retains
local test credentials; do not copy those into reports or commits. Subsequent
source formatting and uninstall-ownership edge-case changes are covered by the
automated tests, not presented as a repeat of this exact live binary run.

## Cursor native-hook research follow-up

On 2026-09-10, a separate native-hook logger exercised Cursor on Darwin 25.5.0
without changing this slice's installation or capture implementation:

| Surface | Version | Observation |
| --- | --- | --- |
| Desktop Agent Chat | About and payload `3.18.25` | User-confirmed one-window and three-window turns each delivered one prompt, one response and one stop hook in the test workspace |
| Agent CLI | `2026.09.08-6caf4ff` | Project-only and global-only tool runs each delivered one session start/end, four pre-tool hooks, two successes and two failures |

The desktop turns shared a conversation and had distinct generations. The second
turn reused the first marker and was identified by order, generation and the user's
confirmation. Neither assistant payload exposed a native message/event/revision ID.
In the CLI project run, identical shell commands had distinct tool-use IDs; each
success or failure retained its pre-hook ID. CLI prompt/response/stop hooks were
not observed in these runs despite completed conversations.

This focused test did not reproduce the earlier global repetition. The logger
filtered workspace roots/cwd to the disposable workspace, and the full effective
plugin/team configuration was not inspected, so cross-root dispatch and overlapping
registration remain unresolved. Global setup remains the default; no content-only
deduplication was added. Original client configuration was restored and hash-verified.

Full findings are recorded in `codeloops-udq` comments 3–5. Redacted native evidence
is at `/tmp/opencode/codeloops-cursor-identity-desktop-redacted.json`; the desktop
report is `/tmp/opencode/codeloops-cursor-identity-desktop-findings.md`. This evidence
concerns source-hook behavior, not exact-commit installed acceptance of this slice.

## Static scan review

UBS was run on tracked diffs and separately on all 22 Rust source files so the new,
untracked modules were included. The full Rust scan's critical heuristics were
three deliberate test panics and four credential-generation/read sites, not
embedded credentials. Warnings primarily covered test assertions/parsing, fixed
invariants, synchronous collector/file work, allocation inventory and bound SQL.
The JavaScript scan flagged fixture parsing and synchronous fixture I/O. TypeScript,
Clippy and actual integration tests provide the corresponding compiler/runtime
checks. Findings were reviewed rather than treating scanner exit status as proof
of correctness.

Before PR publication, `make check` and `make e2e` passed again. The staged UBS
scan covered all 13 changed Rust sources and both bridge files, including the new
modules. Its sole critical heuristic was the installation test's deliberate
startup-failure panic. Warnings were reviewed: test assertions/JSON parsing,
fixed-address parsing and unreachable dispatch invariants, synchronous local I/O,
and SQL-construction heuristics over fixed statements or bound query fragments.
No confirmed new defect was identified. Scan summary:
`/tmp/opencode/codeloops-install-pr-ubs.json`.

## Final integrated acceptance

After this slice is integrated, record the exact `git rev-parse HEAD`, OpenCode
version and Cursor desktop About version with the user's run. Install/setup/run
that integration revision, then exercise the approved cross-client story: capture
conversation and successful/failed edits in each client, retrieve the other
client's messages and file changes through MCP, export and verify a session, and
confirm history survives service restart and uninstall.

The local Linux machine has no Cursor desktop; the user's Mac is reachable through
`ssh-mbpdmi`, and the native-hook follow-up above used that machine. Adapter fixtures
and those hook probes do not replace the final integrated installed-client run.
Remaining per-hook identity and repeated-delivery research in `codeloops-udq` does
not block this slice or reopen prior acceptance.
Promotion to `main` remains the separately authorized `codeloops-6u3` operation.
