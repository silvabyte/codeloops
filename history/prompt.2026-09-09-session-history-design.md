# CodeLoops session history

Date: 2026-09-09

Status: Product direction and four delivery slices approved in conversation.
The technical contracts below guide implementation; delivery is tracked in beads.

Design issue: `codeloops-9s6`

Delivery epic: `codeloops-vvb` (approved slices `codeloops-vvb.1` through
`codeloops-vvb.4`). Post-acceptance release operation: `codeloops-6u3`.

Branch: `feat/session-memory-rust`

## Purpose

Give humans and agents durable, searchable access to conversations and observed
file changes across OpenCode and Cursor sessions. Session history is one capability
within a larger platform. Future capabilities consume its public operations rather
than its database tables or client-specific event formats.

The first release captures new activity on the same device. CLI, MCP, and REST
provide equivalent retrieval behavior. Historical import and cross-device storage
are future clients of the same ingestion and retrieval contracts.

The primary acceptance story is: work in OpenCode, find that session from Cursor,
read the relevant conversation and actual file changes, continue in Cursor, and
retrieve the new session from OpenCode or the CLI.

## Agreed scope

- Rust implementation with OpenCode and Cursor live-capture adapters.
- User and assistant messages, tool inputs/results/failures, session lifecycle,
  and source-exposed relationships between sessions and subagents.
- Lightweight Git context on entries with on-demand file content and diffs.
- Durable captured file versions, including uncommitted changes, surviving loss
  of the original checkout.
- SQLite full-text search and compressed, content-addressed artifacts.
- Live capture only initially; no retrospective session import at launch.
- Local operation initially; portable records suitable for later centralization.

Cursor desktop Agent Chat is the working client assumption. Coverage of Cursor
CLI, cloud agents, Tab completions, and every internal model message is not implied
by coverage of desktop Agent Chat. Adapters report what the source actually exposes.

## Capability boundary

```text
OpenCode plugin ──┐
                 ├── local capture ingress ── session-history capability
Cursor hooks ────┘                                  │
                                     SQLite + artifact storage
                                                   │
                                      CLI / MCP / REST retrieval
```

Start with a Cargo workspace containing a session-history library and a CodeLoops
application. Inside the library, keep ingest, query, Git capture, and artifact
storage private behind a small application API. Split out further crates only when
there is a concrete independent consumer. The OpenCode bridge is a small TypeScript
package because that is its supported plugin interface.

Application operations cover ingest, list sessions, search entries, read a session
page, get an entry, compare checkpoints, read a captured file version, and export
session records with their referenced artifacts. Export is useful immediately for
inspection and portability; it does not require an import command in the first release.

The application owns process lifecycle, configuration, and transports. Client
adapters translate source events and retain provenance. Git capture knows about
repositories and file state, not OpenCode or Cursor. Storage owns transactions,
schema migration, search indexing, and artifact durability.

## Identity and records

Use versioned serializable envelopes and globally unique archive IDs. A persistent
device ID and source-installation ID distinguish native IDs originating on different
machines. Local database row IDs never become public identity.

| Record | Responsibilities |
| --- | --- |
| Project | Logical grouping across sessions and eventually devices |
| Workspace | Device-local working directory or Git worktree; project association |
| Session | Source conversation identity, source version, device, workspace links, lifecycle and optional parent session |
| Entry | Stable reference to a message, tool call/result, or lifecycle item; content, role, source IDs, relationships and revisions |
| Capture record | Immutable delivery/revision envelope, source payload reference, receipt order and timestamps |
| Checkpoint | Observed repository state, capture interval, parent, manifests, Git metadata and coverage |
| Artifact | Content hash, byte length, media/encoding metadata and compressed content |

Record `occurred_at` when the source supplies it, `observed_at` when the adapter sees
it, and `recorded_at` at durable ingestion. Receipt order is distinct from source
order; clocks do not establish causality across concurrent agents or devices.

Entries reference workspace checkpoints rather than embedding manifests and diffs.
A multi-root session can reference multiple workspaces. A project is not identified
by directory basename alone. Git remote URLs are association hints, not infallible
project identity: forks, renamed repositories, and multiple worktrees exist.

Streaming updates revise an existing message/part; they do not appear as repeated
conversation messages. Preserve revision provenance, deduplicate identical payload
blobs, and index the current readable projection. Tool-call IDs connect inputs,
outputs, errors, and parent messages. Capture removals as source events without
silently erasing the historical record.

Preserve source-exposed content and original payloads, including unknown fields.
Distinguish text, tool data, attachments, and metadata. A source attachment path is
not evidence that attachment bytes were archived; report capture coverage explicitly.

## Ingestion and reliability

The ingestion boundary accepts a versioned envelope with a durable delivery ID and
source-scoped identity. Replaying the same delivery returns the existing result;
using its ID with different content is an explicit conflict. Content equality alone
must not collapse two legitimate identical prompts or tool calls.

Adapters assign and persist delivery IDs before retrying. Where source event IDs
are absent, do not promise deduplication of arbitrary independent source redelivery.
Track message/part revisions separately from delivery retries.

Persist artifacts atomically before committing database references. Publish an
entry or checkpoint as complete only when all required artifacts are durable.
Commit entry records and their search-index updates in one database transaction.
A durable adapter spool allows event delivery to resume after service restart.

Git state must be captured while it is observable. At action boundaries, await the
snapshot operation before acknowledging capture where the client supports this.
Retrying a queued event later cannot reconstruct its earlier working tree. Keep its
conversation record and mark its checkpoint missing or late rather than associating
a later snapshot as though it were contemporaneous.

Recording failures should not rewrite prompts, tool results, or agent decisions.
Expose capture health and spool failures. Distinguish active, idle, ended, interrupted,
and unknown lifecycle states: a turn ending is not a conversation ending, and a
missing session-end hook is not proof of successful completion.

## Git snapshots and actual file changes

Capture eagerly; compute and retrieve differences on demand.

Create a baseline before the first observed action, then checkpoints after completed
tool calls (including failures that may have modified files) and at turn boundaries.
Message fragments use the latest known checkpoint without triggering full scans.
Checkpoints indicate when their state was observed; attaching one does not imply a
fresh scan at the exact timestamp of every message.

A checkpoint records repository/worktree identity, branch or detached state, HEAD
when present, index state, working-tree state, and non-ignored untracked files.
Handle repositories with no commits and sessions outside Git explicitly.

Preserve baseline contents and subsequent file versions in a content-addressed
archive. The baseline includes the content required for HEAD/index/worktree
comparisons independently of the original Git object database. This has a one-time
storage cost; subsequent checkpoints reuse existing content and structurally shared
manifests. Never copy the whole repository anew for each entry.

Manifest entries identify relative paths, content hashes, file kinds, and relevant
modes. Preserve staged and unstaged states separately. Represent deletion as absence
from the later state; preserve the earlier content. Archive binary bytes and symlink
targets without following symlinks. Ignored untracked files are excluded by default;
tracked files remain tracked even if an ignore pattern matches them. Preserve path
bytes losslessly when paths are not valid UTF-8.

Git tools can enumerate state and calculate diffs, but Git hashes alone are not the
archive. Capture must not stage, commit, reset, or otherwise alter the user's index
or working tree. Submodules, LFS materialization, conflicted indexes, unreadable
files, and exclusions require explicit coverage status; a stored gitlink or pointer
is not a claim that the referenced file contents were captured.

Record the observation interval and detect concurrent changes where possible.
Filesystem scanning is not an atomic snapshot of concurrent writers. Mark detected
inconsistency as partial/unstable, and avoid assigning authorship: checkpoints show
what changed in a worktree, not necessarily which agent caused it.

The API supports event-to-event differences, arbitrary checkpoint comparisons,
before/after file content, and net session change from the baseline. An entry without
a new checkpoint reports that fact rather than inventing an event-local diff.
Intermediate writes created and undone within one tool call are outside checkpoint
coverage.

## OpenCode integration

Use a minimal TypeScript plugin to forward native session, message/part, and tool
events to the Rust capture ingress. Keep native session IDs, message IDs, part IDs,
and tool-call IDs. Normalize repeated message-part snapshots into revisions.

Use awaited tool hooks for baseline and action-boundary capture where supported.
Treat idle as a turn boundary and handle session errors and compaction explicitly.
Verify event ordering, asynchronous plugin callback behavior, and interrupted output
against the supported installed OpenCode version; documentation alone is not proof
of complete capture.

## Cursor integration

Use command hooks that pass JSON to the Rust capture command. Prefer generic tool
hooks to avoid recording the same execution through both generic and shell/MCP hooks.

| Hook | Intended use |
| --- | --- |
| beforeSubmitPrompt | User message and ensure baseline before activity |
| afterAgentResponse | Completed assistant message |
| preToolUse | Tool input, call identity and pre-action baseline assurance |
| postToolUse / postToolUseFailure | Result or failure and post-action checkpoint |
| stop | Turn outcome and checkpoint |
| sessionStart / sessionEnd | Conversation lifecycle observations |
| subagentStart / subagentStop | Source-exposed child relationships and outcomes |

Cursor lifecycle hooks are fire-and-forget, so sessionStart alone cannot establish
a guaranteed pre-edit baseline. Use the earliest awaited pre-action hook to ensure
it exists. Mark a late start explicitly if earlier work has already occurred.

The documented common envelope includes conversation_id, generation_id, client
version, workspace roots, and an optional transcript_path. Transcript availability
is optional; it cannot be the only capture mechanism. Any reconciliation of newly
captured session output must respect the live-capture start boundary rather than
silently importing older sessions. Source-exposed subagent summaries do not imply
complete subagent transcripts.

## Retrieval surfaces

Proposed CLI command group: `codeloops history` with list, search, show, entry,
changes, file, and export operations. Machine-readable output is available through
`--json`; diagnostics stay on stderr. `codeloops serve` runs the local service and
`codeloops mcp` exposes stdio MCP access to it. Adapter capture commands accept
envelopes through stdin rather than shell-interpolated event contents.

MCP tools and versioned REST routes expose the same application operations, including
ingestion for adapters. Use the supported Rust MCP SDK rather than inventing protocol
handling. Keep HTTP handlers and MCP tools thin; neither contains storage or Git logic.

Search by text and filter by project, source, device, session, role, kind, and time.
Return excerpts with stable session/entry references and checkpoint availability.
Read surrounding conversation with cursor pagination and explicit page bounds.
Large tool outputs, file contents, and patches are fetched separately. Return explicit
truncation/continuation metadata rather than silently cutting results.

Distinguish an unknown ID, invalid request, unavailable artifact, partial checkpoint,
and unavailable source integration. A Git capture failure does not prevent retrieval
of the conversation. Local API exposure is loopback by default, with an application
credential shared by local adapters; credentials are not included in archived events.

## Centralization and future import

Preserve the local capture path when adding a central service. Background replication
can transfer records and missing content-addressed blobs, with durable acknowledgments
and retries. A central checkpoint becomes complete only after its required artifacts
arrive. Public contracts must not require a source device's absolute paths or database
handles to resolve captured data.

Keep source provenance distinct from replication delivery identity. Preserve original
IDs and timestamps while adding destination receipt information. Explicit session
relationships allow later cross-device continuation without guessing from timestamps.
Remote authentication, tenancy, synchronization cursors, conflict policy, and retention
are implementation work for that future capability, not a launch dependency.

A future historical importer constructs the same source envelopes and uses the same
validation/idempotency rules. Historical conversation import cannot manufacture past
working-tree snapshots; missing Git history must remain explicitly unavailable.

## Verification

Use temporary repositories and real SQLite/artifact storage for integration tests.
Verify exact-byte reconstruction after renaming/removing the source checkout, including
pre-existing dirty work, staged versus unstaged differences, deletion, new files,
binary content, symlinks, detached HEAD, and no-commit repositories. Verify reuse of
unchanged content without duplicating complete trees.

Exercise replay, conflicting delivery IDs, legitimate repeated messages, out-of-order
updates, interrupted tools, restart/spool recovery, and failed artifact publication.
Assert equivalent pagination, stable references, and errors across CLI/MCP/REST.

Capture one real conversation in each supported client. Include successful and failed
tools, shell-based edits, multiple assistant messages, and a session with existing dirty
files. Retrieve each conversation and its changes from the other client through MCP.
Record tested client versions and observed coverage; fixtures supplement this test,
not replace it.

## Repository transition

The branch has an isolated worktree at
`/home/matsilva/code/silvabyte/codeloops-session-memory-rust`.
Its base is `7e1e88d`, retaining history for reference. The original checkout and its
uncommitted plugin edit stay in their existing worktree.

The first implementation slice replaces the inherited application/build layout with
the fresh Cargo workspace and thin integration packages, preserving repository
instructions, license, and beads tracking. No compatibility with the legacy actor-critic
runtime is required. This design document does not itself remove legacy code.

### Integration branch and pull requests

Use `feat/session-memory-rust` as the main feature/integration branch. Each delivery
slice starts on its own branch from the latest integrated feature tip, with a PR
whose explicit base is `feat/session-memory-rust`, not `main`. Merge prerequisites
before starting their dependent slices. Keep issue state and implementation changes
together in each PR. CI must run for PRs targeting this integration branch and for
the integrated result; do not restrict the new checks to main-only events.

Suggested slice branch names are `feat/history-opencode`, `feat/history-cursor`,
`feat/history-git-checkpoints`, and `feat/history-install-recovery`. They describe
delivery branches, not additional permanent platform branches.

After all slices merge, the user installs and tests the integration branch itself.
Record the exact tested commit and supported client versions. Fixes go through the
same feature-branch PR flow; substantive changes after testing require another test
of the resulting integration tip.

### Easy local installation and testing

Provide a small Make interface backed by ordinary Cargo/application commands:

| Command | Contract |
| --- | --- |
| `make check` | Formatting, linting, automated tests, and integration smoke checks |
| `make install` | Build locked release dependencies and install the binary plus adapter assets for the current user |
| `make setup` | Register OpenCode/Cursor capture and MCP integrations, then report configuration and health |
| `make run` | Run the local service in the foreground for real-client testing |
| `make e2e` | Exercise installed CLI/MCP/REST capture and retrieval in an isolated test directory |
| `make uninstall` | Remove this installation and its owned integration entries while preserving captured data |

The expected user path is `make check`, `make install`, `make setup`, and `make run`,
then real conversations in both clients. Document `make e2e` separately as automated
verification; passing fixtures is not a substitute for the user's real-client test.

Assume stable Rust/Cargo, Git, Make, and the platform's normal compiler/linker tools.
Use bundled SQLite to avoid a separately installed database. Ship adapter assets
that the clients can load without a separate npm/Bun build step by the user; OpenCode
already provides its plugin runtime. Pin the supported Rust version and document
actual native build prerequisites discovered during implementation.

Support a configurable install prefix and an isolated preview profile for testing
alongside an existing CodeLoops installation. Record installed binary paths and
profile-specific data locations. Setup is repeatable, merges owned hook/MCP entries,
and preserves other plugins, hooks, formatting commands, and client settings. Failed
setup must be recoverable. Uninstall removes only owned entries and assets. Report
any required client restart/reload, selected service address, and executable path.

### Archive and promote after user acceptance

The existing main branch remains the released line during feature development.
After the user confirms the exact integration commit works end-to-end, refresh the
remote state and archive the then-current remote main tip. Do not use the local main
tip as a substitute: the current local main is divergent from origin/main.

Use a descriptive version archive name such as
`archive/actor-critic-v<release>-<YYYY-MM-DD>`. Determine the real release at cutover;
if no reliable version exists, use `archive/actor-critic-<YYYY-MM-DD>-<short-sha>`.
The archive branch must point to the exact old main commit, and its remote existence
must be verified before changing main.

Then promote the accepted feature line to become main. Choose the concrete GitHub
branch-renaming/default-branch sequence after inspecting protections, open PRs,
worktrees, and automation at cutover. Preserve the approved implementation tree and
the old main archive; do not accidentally merge the legacy application back into
the rewrite. Verify the new main, repository default branch, CI/release targets,
and local tracking refs afterward. No force push or branch deletion is implicit in
this plan. Cutover is a separate tracked release operation after user acceptance,
not an automatic consequence of merging the last feature PR.

## References

- Original NDJSON persistence redesign: `7316177`.
- Simplified memory store: `2511528`.
- Current conversation buffering: `plugin/index.ts` at `7e1e88d`.
- OpenCode plugin documentation: https://opencode.ai/docs/plugins/
- Cursor hook documentation: https://cursor.com/docs/agent/hooks
- Integration documentation consulted on 2026-09-09.
