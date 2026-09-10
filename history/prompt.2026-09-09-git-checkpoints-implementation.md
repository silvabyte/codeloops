# Tool history and Git checkpoints: implementation design

Extends the approved session-history design for `codeloops-vvb.3`.

## Decisions

Use read-only Git plumbing to enumerate HEAD, all index stages, and tracked plus
non-ignored untracked paths. Compared with embedding a Git library, this preserves
the installed Git's repository support without adding a second Git implementation.
A separate bare Git archive would reuse trees but complicate artifact portability;
instead CodeLoops owns compressed SHA-256 artifacts and persistent manifest trees.

Each directory manifest maps losslessly encoded path components to file records or
child manifest hashes. Rebuilding a tree publishes only changed directory nodes.
File contents are archived independently of Git object IDs. HEAD, index, and
worktree have separate roots. Unsupported entries remain visible with coverage
reasons rather than looking like deletions. Capture records retain observation
intervals, Git metadata, and detected instability. No operation writes the source
index or worktree. A checkpoint describes observation, never authorship.

The library owns snapshot publication and retrieval. The application invokes it
synchronously at capture boundaries, under the shared outbox coordination lock.
It stores per-session/workspace baseline and latest references plus per-call
pre-action references. The queued envelope contains these durable references;
flush never reads the source filesystem. Message fragments reuse references.
Missing baseline, capture failure, delayed event callbacks, and unmatched tool
completion are explicit coverage states. An acknowledged pre-action boundary
establishes only a baseline for subsequently observed activity.

Tools project by source-scoped call ID, preserving input/output/error artifacts,
status, parent message identity and immutable revisions. Generic Cursor hooks and
awaited OpenCode tool hooks supply boundaries. OpenCode error-part events supply
failure observations with delayed-callback coverage because there is no awaited
failure hook in the plugin SDK.

CLI/MCP/REST share checkpoint, comparison, file and changes queries. Comparisons
are paginated; file/patch bytes use bounded artifact chunks. Event-local changes
require an actual boundary pair. Session changes compare the recorded baseline
and latest observation for the chosen workspace. Retrieval requires only archive
IDs and artifacts, never the original repository or local absolute paths.

## Verification

Use real repositories and storage to prove dirty baseline reconstruction,
HEAD/index/worktree separation, unchanged-node reuse, binary and symlink bytes,
non-UTF-8 names, deletion/new files, detached and unborn HEAD, failed tools,
conflicted/unsupported state and publication failures. Remove source repositories
before retrieval. Exercise outbox restart/retry and bounded transport equivalence.
Run `make check`, isolated installation and real-client conversations. Desktop
Cursor acceptance requires a desktop-capable machine; record that evidence
separately from automated fixtures.
