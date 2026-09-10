# Session-history architecture and contracts

For commands, read [usage](USAGE.md). For setup and recovery, read
[installation](INSTALL_GUIDE.md).

## Client coverage

MCP is the shared access layer for coding harnesses. Any harness that supports
local stdio MCP servers can [connect to the archive](USAGE.md#connect-another-coding-harness).
Automatic conversation capture uses harness-specific integrations. Cursor and
OpenCode are the first; additional integrations use the same ingestion and query
contracts.

| Surface | Capture and evidence |
| --- | --- |
| OpenCode | Native messages, part revisions/deltas, lifecycle, and exposed tools. Live Linux 1.18.30 checks cover capture, Git changes, restart, and recall. |
| Cursor desktop Agent Chat | Prompt and response hooks, lifecycle, and generic tool hooks where emitted. Final Cursor 3.18.25 acceptance on macOS covers conversation capture and fresh-chat MCP recall. |
| Cursor Agent CLI | Separate hook surface. A `2026.09.08-6caf4ff` probe delivered lifecycle/tool hooks but no prompt/response/stop hooks. Do not assume desktop conversation coverage. |

See [verification](verification-install-recovery.md) for exact revisions, earlier
cross-client tests, and evidence boundaries.

### Limits

- Local-first: clients sharing one machine's archive can recall each other's
  history. There is no cross-machine sync.
- Capture starts with newly observed events. No historical transcript import,
  complete subagent transcript, or attachment-byte capture is implemented.
- Coverage depends on the hooks the client emits. Interrupted Cursor output
  without a response hook is missing. Independent duplicate hook invocations
  remain separate observations; durable queue retries are idempotent.
- Git checkpoints are non-atomic observations, not proof of agent authorship.
  Read the timing and file-content coverage on each result.
- Per-event collector/fsync cost and large-repository latency are unbenchmarked.

The sections below define the detailed limits, identity rules, and wire formats.

## Module boundaries

`crates/session-history` owns schema migration, immutable capture records, current
message projections, full-text indexing, and compressed content-addressed artifacts.
Its public boundary includes `History::open`, `History::ingest`, `History::checkpoint`,
`History::query`, and `History::export`, with versioned models in `model`. Database
handles and artifact paths are private.

`crates/codeloops` owns process lifecycle, private data directories, credentials,
the OpenCode and Cursor collectors, shared outbox, CLI, HTTP, and MCP transports. The tiny
`adapters/opencode/history.ts` forwards native JSON without mutating prompts,
tool arguments, or outputs. Tool inputs, outputs, failures and revisions have
normalized entries as well as their original source payloads.

`codeloops capture-cursor` accepts Cursor's documented hook JSON directly on stdin.
It returns only the neutral hook response `{}`. Both collectors use the same
outbox implementation and archive ingestion API; client event translation stays
in the application rather than the storage library.

## Ingress version 1

`capture` reads one JSON envelope from stdin and submits it through HTTP. MCP's
`history_ingest` accepts the same envelope in its `capture` argument. Required fields:

| Field | Meaning |
| --- | --- |
| `schema_version` | `1` |
| `delivery_id` | Canonical UUID persisted by the producer before retry |
| `origin` | `device_id`, `installation_id`, `source`, `source_version` |
| `sequence` | Positive signed-64-bit-range observation sequence within an installation |
| `native_session_id` | Source conversation ID, scoped by device/installation/source |
| `project_id`, `workspace_id` | Canonical archive UUIDs |
| `observed_at` | Adapter observation time in Unix milliseconds |
| `occurred_at` | Source time if provided, otherwise null |
| `change` | Normalized change, described below |
| `source_payload` | Original source JSON, including unknown fields |
| `checkpoints` | Optional compact workspace checkpoint links captured before queue publication |

Changes are tagged by `type`:

- `message`: `native_id`, `role`, nullable `parent_native_id`, `removed`.
- `part`: `message_id`, `native_id`, `kind`, `text`, `removed`.
- `tool`: `native_id` (call ID), `name`, nullable `parent_native_id`, `status`,
  optional JSON `input`, `output`, and `error`. Status is pending, running,
  completed, failed, or unknown, as exposed by the client. A completed shell tool
  does not necessarily mean exit code zero. Retrieval exposes `tool.input_hash`,
  `output_hash`, and `error_hash` for bounded JSON artifact retrieval.
- `lifecycle`: nullable `state`, `title`, `parent_native_id`. A null state is a
  metadata observation; it does not reset the lifecycle state. Known states are
  active, idle, ended, interrupted, and unknown.

Roles are user, assistant, system, and unknown. An envelope or readable message
projection is limited to 4 MiB; source IDs to 1024 bytes and titles to 4096 bytes.
Oversized captures fail explicitly and the client reports the coverage loss.

## Durability and projection

The same delivery ID and envelope returns the original receipt, including stable
archive IDs and recorded time. Different content under that delivery ID conflicts.
Two equal prompts with different native message IDs remain two entries.

Message metadata and each part have independent revision sequences. Late snapshots
remain immutable capture records but do not replace newer projections. The
OpenCode collector serializes observation sequences across local processes and
persists delta accumulation alongside queue insertion in one SQLite transaction.
Independent duplicate source notifications are not promised to deduplicate; queue
retries are. A delta without an observed baseline is an explicit capture failure.

The source payload is stored once by SHA-256. The stored envelope artifact refers
to it through `source_payload_hash` in place of inline `source_payload`. Capture
listing returns both `envelope_hash` and `source_payload_hash`. Message content is
a separate artifact referenced by `content_hash`. Zstandard blobs are atomically
published and fsynced before database references commit; reads verify content hashes.
An interrupted transaction may leave an unreferenced blob but cannot advertise a
capture whose required publication failed. Entry/FTS updates commit together.

Text and reasoning parts project into one readable message in native part-ID order
(OpenCode IDs are ordered). Other kinds stay in source payloads with explicit
coverage. Removal events hide entries from search while retaining prior records.
Receipt order is distinct from source occurrence time and does not establish
causality or authorship.

Project association uses the source's project ID within the installation. OpenCode's
non-Git `global` project uses the full workspace path, not its basename. Workspaces
have separate UUIDs; sessions retain all observed workspace associations.

## Cursor hook identity and coverage

`beforeSubmitPrompt` captures `prompt`, and `afterAgentResponse` captures `text`.
Each invocation atomically queues message metadata, one text part, and a lifecycle
observation. Message IDs have a `hook-message:` UUID prefix: they are collector
identities, not native Cursor message IDs. `conversation_id` remains the native
session ID; `generation_id`, `cursor_version`, model details, attachments, and
unknown fields remain in the original payload. When a prompt from the same
conversation/generation has been captured, its ID becomes the assistant message's
`parent_native_id`. Missing generation or earlier prompt capture leaves that link
absent. Identical prompts and multiple completed messages in one generation stay
distinct. Queue retries reuse the same durable delivery IDs; independently invoking
the hook twice is two observations, even with identical input.

The two clients share a persistent device identity and have separate installation
identities. This preserves distinct sessions even when native IDs are identical.
Cursor projects use a sorted, unique workspace-root set, scoped to Cursor; they
are not automatically equated with OpenCode's native project IDs. Full root paths
map to shared device-local workspace UUIDs. All roots are retained as associations
(at most 32 input roots per hook); extra roots produce metadata observations rather
than duplicate messages. A rootless chat receives a session-specific unknown
workspace/project association. Paths remain provenance, not retrieval requirements.

`stop(completed)` means idle, while aborted/error stops mean interrupted.
`sessionEnd` records ended for completed/window-close/user-close reasons, interrupted
for aborted/error, and unknown for unfamiliar reasons. Missing sessionEnd does not
imply completion. A delayed fire-and-forget `sessionStart` cannot reset an already
observed session's state. Other late lifecycle events reflect local observation
order; the source does not supply universal ordering or event identities.

Subagent start/stop payloads are retained without changing the parent's state or
claiming a full child transcript. An explicitly distinct `conversation_id` with a
`parent_conversation_id` becomes a session relationship. A `subagent_id` alone is
not treated as a conversation ID. Optional `transcript_path` and
`agent_transcript_path` are retained as metadata, never read; absent, null, or stale
paths do not disable prompt/response capture. No historical import is performed.

The current hook set does not capture intermediate streaming fragments or thoughts.
Generic `preToolUse`, `postToolUse`, and `postToolUseFailure` capture tool records
and workspace boundaries. A missing `tool_use_id` creates an explicitly unlinked
collector ID; it cannot establish an event-local before/after pair.
Only completed assistant messages exposed by
`afterAgentResponse` are covered; interrupted output without that hook is missing.
Attachments have metadata-only coverage. Source occurrence timestamps are unknown;
collector observation and archive recording times are retained. Missing client
version is reported as `unknown`.

## Retrieval and coverage

All transports call the same library operations. `list`, `search`, `show`, `entry`,
`captures`, `artifact`, `checkpoint`, `compare`, `changes`, and `file` have matching JSON results. Pages have explicit bounds,
query-bound continuation cursors, and current projections within a receipt-order
membership boundary. Cursors are opaque navigation tokens, not public record IDs.
They do not freeze live revisions. Artifact chunks use byte offsets and base64,
so offsets can fall inside a UTF-8 sequence without losing bytes.

Errors distinguish `invalid_request`, `unknown_id`, `delivery_conflict`,
`unavailable_artifact`, and `storage_failure`. HTTP also reports `unauthorized`;
CLI/MCP report `service_unavailable` when the service cannot be reached.

## Git observations and changes

Each entry and capture exposes `checkpoints`, an array of compact workspace links.
An empty array means no checkpoint information was supplied (including old captures).
Links distinguish `fresh`, `reused`, `late`, `missing`, and `failed` boundary capture;
`baseline_status` distinguishes `pre_action`, `late`, and `missing`. `before_id`
is the observed pre-tool checkpoint, `checkpoint_id` the current observation,
and `baseline_id` the session/workspace baseline. The checkpoint itself separately
reports complete, partial, or unstable **file-content coverage**. Read both coverage
levels. `concurrent_tools` reports overlapping observed tool invocations; it does
not detect all external writers. Abandoned tool starts conservatively remain active.

The collectors synchronously invoke the private Git engine under outbox coordination.
OpenCode uses awaited `chat.message` and tool before/after hooks; error parts and
idle/error events are late-callback observations. Cursor uses `beforeSubmitPrompt`
and generic tool hooks; sessionStart never establishes the baseline. Message
fragments reuse the latest reference. Files and manifests are durable before their
checkpoint record commits and before the envelope is enqueued. Flush/retry never
scans the source filesystem. Snapshot failure preserves the conversation/tool with
failed coverage, even while the service is offline.

Read-only Git plumbing enumerates HEAD, index stages, and tracked plus non-ignored
untracked paths. File bytes and directory manifests are compressed SHA-256 artifacts.
Directory nodes reference child hashes, so unchanged directories and contents are
shared across checkpoints and layers. File access uses directory handles and
no-follow opens; symlink targets are archived as bytes. Source index/worktree files
are never staged, committed, reset, or modified by capture. Detached HEAD and unborn
repositories are supported. All retrieval survives loss of the checkout and Git
object database. Source paths are unnecessary: comparisons return base64 path bytes
plus a lossy human-readable `display_path`.

Two scans detect observable changes during capture. This is not an atomic snapshot:
changes created and undone between observations are invisible, and changes never
establish agent authorship. Conflicted indexes retain stage records; submodules,
LFS pointers, unsupported or unreadable files, and size exclusions report partial
coverage. The implementation limits individual files/manifests/patches to 8 MiB and Git
enumeration output to 32 MiB. Larger files have explicit unavailable records;
oversized patches leave before/after bytes available. Place the archive outside
the observed repository to avoid capturing its own database/artifacts. Non-Git or
unavailable roots report a failed observation, with the reason retained on the link.

`compare` takes before/after checkpoint IDs, independently chosen `head`, `index`,
or `worktree` layers (default worktree), and a page. It returns bounded changed-file
records, before/after hashes/modes/coverage, and on-demand patch artifact references.
Text patches use a valid full-file unified hunk; binary changes return byte hashes.
`file` takes checkpoint ID, base64 path, layer, byte offset and limit (1–65536).
Conflicted index files expose stage hashes for `artifact` retrieval.

`changes` takes exactly one of `session_id` or `entry_id`, a `workspace_id`, and a
page. Session changes compare the baseline to the latest observed checkpoint;
event changes require an actual boundary pair from that entry's captures. Reused
message references alone never invent an event-local difference. Missing pairs
return unavailable/not-captured status. Comparisons preserve endpoint coverage.

An idle event ends a turn, not a session. Session deletion remains an observation,
not successful completion. Attachments retain metadata-only coverage.

## Snapshot export

`History::export(session_id)` is a capability operation; `Query::Export` exposes it
through CLI, MCP and authenticated REST. It freezes committed session/entry/capture
records and checkpoint selection within one SQLite WAL read transaction. Record
pages and the final manifest are immutable content-addressed artifacts, so later
pagination/download cannot combine projections from different moments.

The version-1 `codeloops-session-export` manifest contains session provenance,
receipt-ordered record-page references and a SHA-256-to-byte-length inventory.
Entry pages expose the same projection and explicit excerpt coverage as queries;
full content hashes and every stored capture envelope preserve the original data
and revision history. Capture pages include stable delivery/capture/session/entry
IDs, original capture hashes and receipt timestamps. All checkpoint IDs appearing
in current or historical links are included, with typed traversal through every
HEAD/index/worktree manifest, nested directory and conflicted index stage.

No path reconstruction requires the source machine: manifests retain base64 path
bytes, while bundle artifact filenames are SHA-256 hashes. Existing raw payloads
retain source-reported paths as provenance. Profile settings, credentials, SQLite
files, unreferenced artifacts and other sessions are outside the export boundary.
Pending deliveries are outside the committed snapshot. Partial/late/missing capture
coverage is preserved, not upgraded to completeness by export.

The root manifest is published only after every required artifact verifies. The
CLI's `history export SESSION --output DIRECTORY` downloads and verifies raw bytes
through the public artifact endpoint, then writes the completion descriptor last.
`verify-export DIRECTORY` works offline. Record pages and the root inventory are
bounded to 8 MiB each, with explicit failure rather than truncation. The sum of
artifact bytes has no corresponding 8 MiB cap. See [export usage](USAGE.md#export-a-session)
for the bundle layout and failure/retry behavior.

## Installation ownership

Installation/configuration belongs to the application, not the history library.
The binary embeds client-loadable adapter assets; Make wraps locked Cargo builds
and installed CLI commands. A prefix holds one named profile with explicit data
and service defaults. The generated OpenCode wrapper passes per-installation
options without changing process-global environment variables. Both clients are
registered once at user-global scope.

Setup edits individual JSONC properties/array entries while preserving unrelated
configuration and comments. A write-ahead ownership record permits finishing or
removing a partially applied setup. Per-prefix and per-config-directory locks
serialize CodeLoops writers; rechecking original bytes detects external edits
before replacement. Client editors do not participate in these locks, so users
should avoid simultaneous edits to the same configuration during setup.

Uninstall removes exact owned entries and only unchanged assets, preserving data,
spool identities and unrelated files. Conflicting edits are surfaced. Cursor hook
source identity research (`codeloops-udq`) is separate: repeated setup is idempotent,
but independent native hook invocations are not deduplicated by equal text.
