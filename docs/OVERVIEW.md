# Session-history architecture and contracts

`crates/session-history` owns schema migration, immutable capture records, current
message projections, full-text indexing, and compressed content-addressed artifacts.
Its public boundary is `History::open`, `History::ingest`, and `History::query`, with
versioned models in `model`. Database handles and artifact paths are private.

`crates/codeloops` owns process lifecycle, private data directories, credentials,
the OpenCode collector/outbox, CLI, HTTP, and MCP transports. The tiny
`adapters/opencode/history.ts` forwards native JSON without mutating prompts,
tool arguments, or outputs. Tools remain in original payloads in this slice.

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

Changes are tagged by `type`:

- `message`: `native_id`, `role`, nullable `parent_native_id`, `removed`.
- `part`: `message_id`, `native_id`, `kind`, `text`, `removed`.
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

## Retrieval and coverage

All transports call the same library operations. `list`, `search`, `show`, `entry`,
`captures`, and `artifact` have matching JSON results. Pages have explicit bounds,
query-bound continuation cursors, and current projections within a receipt-order
membership boundary. Cursors are opaque navigation tokens, not public record IDs.
They do not freeze live revisions. Artifact chunks use byte offsets and base64,
so offsets can fall inside a UTF-8 sequence without losing bytes.

Errors distinguish `invalid_request`, `unknown_id`, `delivery_conflict`,
`unavailable_artifact`, and `storage_failure`. HTTP also reports `unauthorized`;
CLI/MCP report `service_unavailable` when the service cannot be reached.

This slice returns `checkpoint.status = not_captured`, attachment coverage
`metadata_only`, and tool coverage `source_payload_only`. An idle event ends a turn,
not a session. Session deletion is retained as an observation, not successful
completion. Later Git capture must preserve this distinction and never assign a
late snapshot to an earlier queued event as if it were contemporaneous.
