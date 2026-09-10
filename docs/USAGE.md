# Use your history

These examples use the [installed binary](INSTALL_GUIDE.md) and a running service:

```sh
export PATH="$HOME/.local/codeloops-history-preview/bin:$PATH"
```

Replace uppercase placeholders with IDs or hashes returned by a query. Archive
session, entry, workspace, and checkpoint IDs are UUIDs. Native client IDs are
separate provenance fields. Commands return JSON; `--json` makes it compact.

## Find a conversation

```sh
codeloops history list --source cursor --json
codeloops history list --source opencode --json
codeloops history search "database migration" --role assistant --limit 20 --json
codeloops history show SESSION_ID --limit 20 --json
codeloops history entry ENTRY_ID --json
codeloops history captures SESSION_ID --json
```

Search treats text as a literal FTS5 phrase. It does not accept raw FTS operators.
Use `--project-id`, `--source`, `--device-id`, `--session-id`, `--role`, `--kind`,
`--since`, or `--until` to narrow results. Times are observation times in Unix
milliseconds: `--since` is inclusive, `--until` exclusive. Session listing accepts
session-level filters, not role or kind.

Results include current entries and coverage; `captures` exposes immutable
observations and revision payload hashes. Tool entries link input, output, and
error artifacts. A completed tool status reflects the client report, not a
guarantee that a shell command exited successfully.

### Read the next page or full content

```sh
codeloops history search "database migration" --role assistant --limit 20 --cursor 'NEXT_CURSOR' --json
codeloops history artifact CONTENT_HASH --offset 0 --limit 65536 --json
```

Follow `next_cursor` with the same operation, query, and filters. Pages default to
20 items and allow at most 100. Membership is bounded by the first page's receipt
order; later revisions remain live. This is not a frozen snapshot.

Entry excerpts stop at 4096 Unicode characters. Check `truncated`, `text_bytes`,
and `content_hash`, then fetch the artifact for full text. Artifact chunks are
base64-encoded bytes, at most 64 KiB. Follow `next_offset` until complete and use
`total_bytes` to check length. Decode and concatenate bytes before interpreting
UTF-8 text.

## Inspect observed file changes

Take the session/workspace IDs from `history list` and checkpoint links from
entries or captures:

```sh
codeloops history search "your tool command" --kind tool --json
codeloops history changes --session-id SESSION_ID --workspace-id WORKSPACE_ID --json
codeloops history changes --entry-id TOOL_ENTRY_ID --workspace-id WORKSPACE_ID --json
codeloops history checkpoint CHECKPOINT_ID --json
```

Session changes compare the baseline with the latest observed checkpoint.
Entry changes require a recorded before/after boundary pair. Use exactly one of
`--session-id` and `--entry-id`.

Compare layers to separate staged and unstaged changes, then retrieve file bytes:

```sh
codeloops history compare BASELINE_ID BASELINE_ID --before-layer head --after-layer index --json
codeloops history compare BASELINE_ID BASELINE_ID --before-layer index --after-layer worktree --json
codeloops history compare BEFORE_ID AFTER_ID --json
codeloops history file CHECKPOINT_ID BASE64_PATH --layer worktree --offset 0 --limit 65536 --json
codeloops history artifact PATCH_OR_TOOL_HASH --json
```

`BASE64_PATH` is the comparison's encoded path, not its `display_path`. File reads
default to the worktree layer. Comparisons return patch hashes when available;
binary changes return byte hashes. Conflicted index records expose stage hashes
for artifact retrieval.

Read both checkpoint-link timing and checkpoint file-content coverage. Missing,
late, failed, partial, or unstable observations do not prove complete changes or
agent authorship. See the [Git contract](OVERVIEW.md#git-observations-and-changes).

## Export a session

Flush and inspect health first if you need queued captures included:

```sh
codeloops flush --json
codeloops history export SESSION_ID --output "$HOME/codeloops-session-export" --json
codeloops verify-export "$HOME/codeloops-session-export" --json
```

The output parent must exist and the destination directory must be new. Choose a
different name for the next export. Download requires the service; verification
works offline without credentials, the source checkout, or the original archive.

The bundle contains:

| Path | Contents |
| --- | --- |
| `export.json` | Completion descriptor, written last |
| `manifest.json` | Snapshot metadata and artifact inventory |
| `artifacts/SHA256` | Raw verified artifact bytes |

Export freezes one session's committed records, all capture revisions, and reachable
artifacts, including captured HEAD/index/worktree bytes. Partial coverage stays
partial. Pending deliveries, unrelated sessions, credentials, profile settings,
and database files are excluded. Related sessions are referenced, not recursively
exported. Raw source payloads retain their original provenance.

Download verifies hashes and byte lengths. An interrupted download leaves an
incomplete directory without `export.json`; remove that incomplete destination or
choose a new one before retrying. `verify-export` detects missing or corrupt
artifacts. A bundle is an offline archive; live import/restore is not implemented.
For snapshot and size bounds, see the [export contract](OVERVIEW.md#snapshot-export).

## Query through MCP

Setup registers `codeloops mcp` for your client. It exposes `history_query` and
`history_ingest`. Query arguments wrap the operation in `request`:

```json
{
  "request": {
    "operation": "search",
    "text": "database migration",
    "filter": { "source": "opencode", "role": "assistant" },
    "page": { "limit": 20 }
  }
}
```

For a session, use `{"request":{"operation":"show","session_id":"SESSION_ID"}}`.
Both clients can query the same archive on one machine. Source filters select
which client's records to retrieve. Use the [agent instruction template](AGENTS_TEMPLATE.md)
to make this workflow discoverable in a project.

## Query through REST

All endpoints use POST and require `Authorization: Bearer TOKEN`, where TOKEN is
the content of your data directory's `credential` file. The service binds only to
loopback. CLI and MCP read the credential themselves.

This example uses the default profile and curl:

```sh
TOKEN="$(cat "$HOME/.local/share/codeloops-history/preview/credential")"
curl --fail-with-body --silent --show-error \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  --data '{"operation":"search","text":"database migration","filter":{"source":"opencode"},"page":{"limit":20}}' \
  http://127.0.0.1:47823/v1/history/query
unset TOKEN
```

| Endpoint | Body |
| --- | --- |
| `/v1/history/query` | Query object, without MCP's `request` wrapper |
| `/v1/history/ingest` | Version-1 capture envelope |
| `/v1/health` | `{}` |

CLI, MCP, and REST use the same query operations and result IDs. Wire fields use
snake_case. `export` without CLI `--output` returns a descriptor: fetch its
`manifest_hash` through `artifact`, then download the manifest's inventory,
following each `next_offset`.

For ingestion, `codeloops capture` reads one envelope from stdin and submits it to
the service; MCP uses `history_ingest` with a `capture` argument. Custom producers
must follow the [ingress and replay contract](OVERVIEW.md#ingress-version-1).
