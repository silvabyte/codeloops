# Installation, recovery and export verification

Updated: 2026-09-10. This is the acceptance record for session-history `main`.
Earlier slice reports retain their original test scope:

- [OpenCode capture and recall](verification-opencode.md)
- [Cursor and same-machine cross-client recall](verification-cursor.md)
- [Tool history and Git checkpoints](verification-git-checkpoints.md)

## Status

Accepted implementation: `36582b50ea54b0592066f61f2036606e00801674`.
The user accepted live Cursor capture and fresh-chat MCP recall on the installed
Mac build, then authorized the [main cutover](../CHANGELOG.md#session-history-main-2026-09-10).
Delivery epic `codeloops-vvb`, installation slice `codeloops-vvb.4`, and release
operation `codeloops-6u3` are closed.

| Evidence | Scope |
| --- | --- |
| [Integrated CI](https://github.com/silvabyte/codeloops/actions/runs/34498642646) | Accepted commit after PR #54 merged: `make check`, isolated install, and `make e2e` passed |
| [Main CI](https://github.com/silvabyte/codeloops/actions/runs/34503249545) | Same accepted commit after cutover: all three checks passed |
| [Final Mac acceptance](#final-integrated-acceptance) | Cursor desktop 3.18.25 conversation capture and user-confirmed MCP recall on the accepted commit |
| [Earlier live OpenCode run](#real-opencode-installed-verification) | Installation/recovery development build on Linux, including edits, offline replay, source removal, restart, MCP export/recall, and uninstall |

Earlier OpenCode, cross-client, checkpoint, and recovery tests support the release.
They were not repeated as a full manual two-client run on the final commit. The
final marker chat was not a Git workspace and did not test file changes.

## Automated verification

The following local results were recorded on `feat/history-install-recovery`,
based on `b05d410a0a459f5807e90dbf94c73e0af653ad4c`. Integrated CI is linked above.

- `make check`: 19 Rust integration tests, Clippy with warnings denied, Rust
  formatting, TypeScript typechecking, the Ultracite Biome preset and the bridge
  integration test.
- `make e2e`: three installed-interface tests using a locked release binary copied
  through the public `install` command into temporary prefixes.
- The [current CI workflow](../.github/workflows/ci.yml) runs checks, isolated
  installation, and installed-interface tests for `main` and the retained
  integration branch.

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

This run tested an uncommitted installation/recovery build, not the final
integrated commit.

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

Full findings are recorded in `codeloops-udq` comments 3 through 5. Redacted native evidence
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

Source: canonical `codeloops-vvb.4` comments 8 through 11 and `codeloops-6u3`
comment 13. No new interactive run was performed for this docs refresh.

On 2026-09-10, the exact accepted commit was built with Rust 1.97.1 and installed
from a clean Mac checkout using `make install`.

| Item | Recorded value |
| --- | --- |
| Commit | `36582b50ea54b0592066f61f2036606e00801674` |
| Installed binary SHA-256 | `5db4c941039afc18920d8e8f982027d70073a9b41d8a96682bbd5f6ffb7cae55` |
| Prefix | `~/.local/codeloops-history-preview` |
| Reused data | `~/.local/share/codeloops-history/cursor-verification` |
| Service | `127.0.0.1:47823`, started separately as a detached process |
| Cursor desktop | `3.18.25` |
| OpenCode CLI inventory | `1.17.9`, not a new live OpenCode test |

`make setup` registered user-global capture/MCP and explicitly selected
`opencode.json` because both config filenames existed. Only exactly matched older
manual CodeLoops registrations were removed. Semantic comparison confirmed
unrelated settings and the existing `herdr` hook were preserved; `opencode.jsonc`
was byte-unchanged. Private backups and the receipt remain on the Mac under
`~/.local/state/codeloops-acceptance-36582b5-wvrhekor/`.

The user's Cursor marker `mac-36582b5-cursor` produced one matching prompt and one
assistant reply in session `f934f9a0-8a08-491f-867d-d9a98479a264`:

- User entry: `7bde74cf-5611-4370-aebc-06e4a3a40e4c`
- Assistant entry: `c87ae296-3d48-4e5e-b4bf-15e65cbf54db`, linked to the prompt

The native workspace was an earlier temporary research directory. Direct
`git rev-parse` confirmed it was not a Git repository, explaining the failed
checkpoint link. Conversation delivery worked; no rejected deliveries or enqueue
failures were observed during the check.

After the fresh-chat archive lookup instruction, the user confirmed MCP recall:
"yup its working fine... what is next?" This completes the recorded installed
Cursor experience. It does not add a final-commit manual test of file changes,
cross-client recall, restart, export, or uninstall to the earlier evidence.
Independent Cursor identity research in `codeloops-udq` remains nonblocking.
