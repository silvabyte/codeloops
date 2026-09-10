# Tool history and Git checkpoint verification

Development branch: `feat/history-git-checkpoints`, based on integration commit
`9b5d590544f578d62859bd59c0cdbea9c045935d`. The baseline's integrated CI run
[34417404742](https://github.com/silvabyte/codeloops/actions/runs/34417404742)
completed successfully. Verification was performed on the working tree introducing
this slice; its delivery commit is recorded in the feature-targeted pull request.

## Automated checks

`make check` exercises 15 Rust integration tests and the TypeScript bridge test.
The new checks use real Git repositories, SQLite, artifact storage, subprocess
collectors, HTTP, and official-SDK MCP clients:

- Dirty baseline with separate HEAD, index and worktree versions; exact retrieval
  after deleting the checkout and its Git object database.
- New non-ignored files, ignored exclusions, tracked-but-ignored files, deletions,
  binary contents, executable modes, symlink targets, and non-UTF-8 path bytes.
- Detached and unborn HEAD; unchanged artifact/manifest reuse; unchanged source
  index bytes and Git status; paginated comparisons and query-scoped cursors.
- Conflicted index stages, LFS pointer-only coverage, submodule exclusions, and
  failed artifact publication without a published checkpoint.
- A concurrent filesystem writer produces unstable coverage; overlapping tools
  across conversations report concurrent activity without authorship claims.
- Both translators capture tool inputs/results/failures; a shell writes and fails;
  the source checkout is removed before the first queue flush. File retrieval,
  event-local comparisons and net session changes still work. CLI/MCP/REST event
  comparisons are equal; restart preserves session IDs.
- Missing pre-tool capture, late baseline, non-Git capture failure, neutral Cursor
  output, message/part revisions and durable replay remain explicit.
- The bridge synchronously forwards native events and awaited tool boundaries
  without mutating the source tool input or result.

The first commit attempt exposed a fixture-isolation bug: hook-exported Git
environment variables redirected temporary-repository initialization to the parent
checkout. Git fixture commands now strip inherited `GIT_*` variables before
applying their controlled test configuration. All 15 Rust tests also pass with
simulated hook variables (`GIT_DIR`, `GIT_COMMON_DIR`, `GIT_WORK_TREE`,
`GIT_INDEX_FILE`, and `GIT_PREFIX`) pointing to disposable decoys.

`make install PREFIX=/tmp/opencode/codeloops-git-preview` successfully installed
the locked release binary and adapters on Linux with Rust/Cargo 1.97.1.

## Live OpenCode verification

OpenCode **1.18.30**, model `openai/gpt-6-astra`, Linux, 2026-09-09 local time
(2026-09-10 UTC). Local raw evidence:
`/tmp/opencode/history-git-live-bnr3r8_w`; walkthrough harness:
`/tmp/opencode/verify-codeloops-git-live.py`.

The installed plugin recorded a real conversation in an unborn Git repository
with different staged and dirty file contents. It observed a successful shell
write, a shell write followed by exit 7, and a read of a nonexistent file. The
index remained byte-for-byte unchanged. Events were captured with the history
service offline, then drained. The source checkout was deleted, the service
restarted, and CLI file queries recovered the staged, dirty, and final contents.
A second real OpenCode conversation called MCP to retrieve the first session's
changes and then read/decode `failed.txt` from the archived final checkpoint.

- Session: `d1497bb9-14bd-4d8f-9c97-979b930a09a8`
- Workspace: `9122720f-fe4e-4638-8958-cda8151c7b69`
- Baseline: `e12de30c-b3c2-4d09-b336-47d4c755f9b6`
- Final observation: `cbb59e46-260b-41f7-8dec-4dd7558e6057`
- Health: **176 delivered**, zero pending/rejected/enqueue failures and zero
  workspaces whose latest checkpoint failed.

Observed semantics: OpenCode emits a **completed tool** for a shell that exits
nonzero. CodeLoops preserves that source status and payload, rather than inferring
the command succeeded. The missing-file read emitted an error part and a normalized
failed tool; its after-state is labeled **late**, because the SDK has no awaited
failure hook. `chat.message` established the pre-action baseline, and the successful
tool hooks produced saved before/after pairs. Turn callbacks are likewise labeled
late. Concurrent external writes and transient writes undone inside a tool remain
outside attribution/atomicity guarantees.

An initial harness attempt expected exactly three calls but OpenCode first ran
`ls` to verify the directory. That attempt captured correctly; the successful
harness asserts the required writes and failure without assuming an exact count.

## Remaining Cursor desktop acceptance

This Linux environment has no Cursor desktop IDE. The earlier desktop evidence in
`verification-cursor.md` predates generic tool/checkpoint hooks and does **not**
verify this slice. `codeloops-vvb.3` remains in progress pending this acceptance.

On a desktop-capable machine, install this implementation revision and register
the ten hooks from the installed example, preferably project-scoped. Use isolated
history data outside the test repository and preserve existing client settings.
Record the exact implementation commit, Cursor version and OpenCode version.

1. Begin with a committed file, staged changes, different unstaged changes, a
   binary and a symlink. Start a new Cursor Agent Chat with a distinctive prompt.
2. Execute successful and failing tools, including a shell that writes before
   failure, and create/delete files. Confirm generic hook call IDs link inputs to
   results and that beforeSubmitPrompt/preToolUse are awaited before mutation.
3. From OpenCode, call `history_query` to find that Cursor session, retrieve tool
   artifacts, request `changes`, and read exact before/after file bytes. Verify
   the checkpoint baseline, timing, partial/unstable coverage and index preservation.
4. In a real OpenCode conversation, make another distinctive change. From Cursor,
   retrieve the OpenCode session's changes and file bytes through MCP.
5. Remove the disposable source checkout and restart the service. Repeat both
   retrievals, following pagination/chunks and confirming stable archive IDs.
6. Include interruption/failed tools; record actual status/timing evidence and
   health. Fixtures supplement this walkthrough but cannot replace it.

Preview limits are explicit in `OVERVIEW.md`: non-atomic scans, 8 MiB individual
file/manifests/patch bounds, 32 MiB Git enumeration bounds, and partial coverage
for unsupported content. Large-repository latency is not benchmarked.
