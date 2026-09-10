# CodeLoops session history

Durable local conversation history for coding agents. Capture a conversation,
restart the service, and find it again through the CLI, MCP, or REST.

This integration branch is a **Rust preview**. It captures live OpenCode and Cursor
conversations, source-exposed tool outcomes, and self-contained Git checkpoints.
Global setup registers both clients; portable exports retain records and captured
bytes independently of the source checkout. See [client coverage](docs/OVERVIEW.md)
and [installation/recovery verification](docs/verification-install-recovery.md)
for automated evidence and final integrated real-client acceptance status.

## Try the preview

Build/install requires Rust **1.97.1**, Git, a C compiler/linker, Make, and standard
Unix installation tools. SQLite and Zstandard compile from bundled sources;
no database server or system SQLite package is required. Linux is the verified
platform. Development checks additionally require Node **22.18+** and npm.

```sh
make check
make install
make setup
make run
```

`make check` installs locked development dependencies when needed and runs Rust
formatting, Clippy, persistence/transport integration tests, and bridge checks.
`make install` builds with `Cargo.lock` and installs the binary and client-loadable
adapter assets without an npm/Bun build. `make setup` registers capture and MCP in
both clients' user-global settings once, preserving other plugins, hooks, servers,
and JSONC comments. `make run` starts the installed service in the foreground.

The default prefix is `~/.local/codeloops-history-preview`, separate from an
existing CodeLoops executable. Override it with `make install PREFIX=/your/path`.

**Quit and restart OpenCode** after setup. Open a new Cursor Agent Chat and verify
its MCP connection. Generated integrations carry explicit executable/data paths;
desktop launches do not need shell environment variables. Capture works while the
service is offline; restart it or run `codeloops flush` to deliver queued events.

`make e2e` verifies installed CLI/MCP/REST behavior in temporary state. It complements
the real conversations in both clients. `make uninstall` removes owned integration
entries and assets while preserving the archive, spool and persistent identities.
See the [installation guide](docs/INSTALL_GUIDE.md) for profiles and recovery.

## Retrieve history

Use the installed executable (or add its `bin` directory to your PATH):

```sh
codeloops history list --source opencode --json
codeloops history list --source cursor --json
codeloops history search "database migration" --role assistant --json
codeloops history show <session-uuid> --limit 20 --json
codeloops history entry <entry-uuid> --json
codeloops history captures <session-uuid> --json
codeloops history artifact <sha256> --offset 0 --limit 65536 --json
codeloops history export <session-uuid> --output /path/to/new-export-directory --json
codeloops verify-export /path/to/new-export-directory --json
codeloops health --json
```

Search is a literal FTS5 phrase, with filters for project, source, device,
session, role, kind, and observation time (`--since` inclusive / `--until`
exclusive, Unix milliseconds). Session listing supports the session-level filters.
IDs are archive UUIDs; native client IDs remain separate provenance fields.

Follow `next_cursor` with the same query/filter. Pages contain at most 100 items,
ordered by first receipt, and exclude entries created after the first page's
membership boundary. Revisions remain live: pagination is not a frozen historical
view. Text excerpts are at most 4096 Unicode characters; `truncated`, `text_bytes`,
and `content_hash` identify content to fetch separately. Artifact chunks are base64
encoded, at most 64 KiB, with `next_offset` and `total_bytes`.

## Application interfaces

`codeloops mcp` exposes `history_query` and `history_ingest` using the official
Rust MCP SDK (`rmcp` 3.2.0). `history_query` takes a `request` object:

```json
{"request":{"operation":"search","text":"database migration","filter":{"source":"opencode"},"page":{"limit":20}}}
```

REST uses the same request at `POST /v1/history/query`. Ingestion is
`POST /v1/history/ingest`; health is `POST /v1/health`. Requests require
`Authorization: Bearer <credential>` from `CODELOOPS_DATA_DIR/credential`.
The service binds only to loopback, and the application data directory is private
to its owner. CLI and MCP read the credential themselves; it is never put into
capture records. Diagnostics go to stderr.

See [architecture and wire contracts](docs/OVERVIEW.md) and
[verification evidence](docs/verification-opencode.md).

## Delivery and release

Work is tracked in beads under `codeloops-vvb`. Slice PRs target
`feat/session-memory-rust`; CI covers that target and its integrated tip. The
released `main` line is promoted only after the user's real-client acceptance of
the complete integration branch. The approved design is in
`history/prompt.2026-09-09-session-history-design.md`.
