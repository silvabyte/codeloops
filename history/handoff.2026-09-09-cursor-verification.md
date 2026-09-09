# Cursor session-history verification handoff

## At a glance

Continue `codeloops-vvb.2` on `feat/history-cursor` using a machine with Cursor
desktop Agent Chat installed and authenticated. The implementation, automated
tests, installation instructions, and this handoff are on this branch. First run
`git status --short --branch` and `git rev-parse HEAD`, then follow
`docs/INSTALL_GUIDE.md` to install an isolated preview shared by Cursor and
OpenCode. The remaining acceptance work is real Cursor hook delivery and
bidirectional MCP recall; update `docs/verification-cursor.md` with the results.

## Get the branch on the other machine

From an existing clean clone of `silvabyte/codeloops`:

```sh
git fetch origin
git switch --track origin/feat/history-cursor
git status --short --branch
git rev-parse HEAD
```

If the local branch already exists, switch to `feat/history-cursor` and
fast-forward it from `origin/feat/history-cursor`. Inspect any local changes first.
Record the resulting commit as the tested implementation revision.

## Build and configure the preview

Run from the repository root:

```sh
make check
make install PREFIX="$HOME/.local/codeloops-cursor-preview"

export CODELOOPS_BIN="$HOME/.local/codeloops-cursor-preview/bin/codeloops"
export CODELOOPS_DATA_DIR="$HOME/.local/share/codeloops-history/cursor-verification"
export CODELOOPS_ADDRESS="127.0.0.1:47823"
export CODELOOPS_OPENCODE_VERSION="$(opencode --version)"
```

- Read the prerequisites in `README.md`; Linux was the build/test platform for
  this session. Record any adjustments needed on the destination platform.
- Follow `docs/INSTALL_GUIDE.md` for both clients. The installed Cursor example is
  `share/codeloops/adapters/cursor/hooks.example.json` under the chosen prefix.
- Replace the example's absolute paths with this machine's paths. Use the same
  data directory and address for both clients, their MCP servers, and the service.
- Add the CodeLoops entries to existing configurations without replacing other
  hooks, plugins, or MCP servers. Install each capture hook at only one level.
- Start `"$CODELOOPS_BIN" serve` in a separate terminal. Desktop launchers may not
  inherit shell variables; the guide uses explicit Cursor executable/data paths.
- The Make interface currently provides `check`, `install`, and `run`; automatic
  `setup`, `e2e`, and `uninstall` belong to the final delivery slice.

## Finish the acceptance work

Use the scenarios under **Completing real-client acceptance** in
`docs/verification-cursor.md`. That document is the source of truth for completed
checks, tested versions, source coverage, and the remaining observations to make.

Start with a distinctive phrase in a new OpenCode conversation. Ask Cursor to
retrieve its assistant entry through `history_query`, then have OpenCode retrieve
the new Cursor assistant entry. Save the tool results and stable archive UUIDs.
The new machine has a fresh archive: it must create its own sample conversations.

Useful installed checks:

```sh
"$CODELOOPS_BIN" health --json
"$CODELOOPS_BIN" history list --source cursor --json
"$CODELOOPS_BIN" history list --source opencode --json
"$CODELOOPS_BIN" history search "your distinctive phrase" --json
"$CODELOOPS_BIN" history entry <entry-uuid> --json
"$CODELOOPS_BIN" history captures <session-uuid> --json
```

If a hook is missing or its payload differs, preserve a minimal redacted fixture,
fix the source translation, and rerun the relevant tests plus the live scenario.
Record the actual desktop version, hook timing, configuration scope, tested commit,
health counts, and CLI/MCP/REST agreement in `docs/verification-cursor.md`.

## State and implementation references

- Delivery branch: `feat/history-cursor`. Its integration base is
  `82564c06a70ce689b17e246c3e5c173d4079796a` on `feat/session-memory-rust`.
- The base contains merged [PR #51](https://github.com/silvabyte/codeloops/pull/51),
  the OpenCode slice. Its integrated CI passed.
- `crates/codeloops/src/cursor.rs` translates native Cursor hooks.
- `crates/codeloops/src/outbox.rs` owns the shared local outbox and health. Its
  historical `opencode-spool.sqlite3` filename preserves existing preview data.
- `crates/codeloops/tests/transports.rs` covers both collectors and retrieval
  surfaces. `docs/OVERVIEW.md` explains identity, revision, and coverage contracts.
- Product design is already approved in
  `history/prompt.2026-09-09-session-history-design.md`; a fresh design interview is
  unnecessary for this verification pass.
- The source machine lacked Cursor desktop. Cursor CLI
  `2026.01.28-fd13201` returned `Authentication required` on actual agent runs even
  though `status` reported a login. No real Cursor hook or cross-client result was
  established there. Desktop verification is the acceptance target.
- Paths under `/tmp/opencode` in the verification report identify source-machine
  evidence only. Those artifacts, credentials, and the exploratory CLI script are
  not transferred or required; use the committed desktop instructions above.
- Source-machine test services were stopped, and existing Cursor configuration
  and authentication files were checked unchanged.

## Tracker and delivery workflow

- Read `AGENTS.md` and load `bd-cli` before tracker changes. The current
  `codeloops-vvb.2` snapshot is included in `.beads/issues.jsonl` and remains
  `in_progress` until real-client acceptance succeeds.
- On a new machine, use the clone's beads setup and inspect its current issue
  state. The original session's separate canonical database/worktree routing was
  machine-specific; do not reuse absolute source-machine paths or overwrite a
  newer tracker state with an older feature snapshot.
- After acceptance, update the verification document and issue together, run
  `make check` and UBS, and commit the verification/fixes with the tracker export.
- The slice PR must target **`feat/session-memory-rust`**, not `main`. No Cursor
  slice PR existed at handoff creation. Inspect the remote before creating one.
- Merge this slice before beginning dependent `codeloops-vvb.3`; read `.3` and `.4`
  for the remaining Git/tool and installation/recovery work.
- Main cutover is the separate `codeloops-6u3` release operation after the user's
  acceptance of the exact completed integration commit.

## Suggested skills

- `bd-cli` for issue updates and completion.
- `system-design` when changing the collector/capability boundary.
- `customize-opencode` before editing OpenCode configuration or plugin integration.
- `ubs` before committing fixes or verification changes; inspect heuristic findings.
- `diagnosing-bugs` for unexpected real-client behavior. The globally named
  `systematic-debugging` skill was not available in the source environment.
- `handoff` if passing the remaining work to another session.
