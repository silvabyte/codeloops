# Cursor session-history verification handoff

## At a glance

Real-client acceptance for `codeloops-vvb.2` passed on `feat/history-cursor` at
commit `9539259f5f1c4e5b302df5486dedbf0581b88c9e`. Cursor desktop hook delivery,
bidirectional MCP recall, service restart stability, and two distinct native
submissions with identical user text are recorded in
`docs/verification-cursor.md`. Close the issue, run final checks, and prepare the
slice PR against `feat/session-memory-rust`.

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

## Acceptance result

The same prompt was submitted twice in archive session
`1eebe8ce-9ed4-4220-86c1-b1ee71a0b4e4`. Entries
`a139f5cf-f790-41d7-b1fd-173dae94f3e5` and
`d89d77d5-7a2e-4433-819e-9d4d96cc6fb9` have the same content hash and distinct
generation-scoped native IDs. Archive health remained clean.

Cross-client recall is already established. Cursor retrieved OpenCode assistant
entry `70413a02-fd5b-4c05-b1db-4b60f60d5890`; OpenCode retrieved Cursor assistant
entry `6911fb99-ed9d-4fc4-ae7a-c479b8e59120`. CLI and REST returned the same IDs
after service restart.

Useful installed checks:

```sh
"$CODELOOPS_BIN" health --json
"$CODELOOPS_BIN" history list --source cursor --json
"$CODELOOPS_BIN" history list --source opencode --json
"$CODELOOPS_BIN" history search "your distinctive phrase" --json
"$CODELOOPS_BIN" history entry <entry-uuid> --json
"$CODELOOPS_BIN" history captures <session-uuid> --json
```

The isolated preview remains installed under
`~/.local/codeloops-cursor-preview`, and the shared archive remains under
`~/.local/share/codeloops-history/cursor-verification`. Additive MCP/plugin entries
remain in the user configs. The temporary project-scoped CodeLoops capture hooks
were removed after acceptance without changing the existing project hooks.

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
