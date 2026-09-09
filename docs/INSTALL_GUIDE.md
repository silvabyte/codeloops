# Session-history preview installation

Run `make install`, or choose a separate prefix with `make install PREFIX=...`.
The shipped `history.ts` has only a type-only OpenCode dependency and uses Node
built-ins supported by OpenCode's runtime. No runtime npm install is necessary.

Set these variables in the environment that launches both the service and OpenCode:

```sh
export CODELOOPS_BIN="$HOME/.local/codeloops-history-preview/bin/codeloops"
export CODELOOPS_DATA_DIR="$HOME/.local/share/codeloops-history/preview"
export CODELOOPS_ADDRESS="127.0.0.1:47823"
export CODELOOPS_OPENCODE_VERSION="$(opencode --version)"
```

For a project-scoped trial, merge the following entries into that project's
`opencode.json`. Replace `/absolute/prefix` with your actual installation prefix.
Preserve existing plugin and MCP entries.

```json
{
  "$schema": "https://opencode.ai/config.json",
  "plugin": ["file:///absolute/prefix/share/codeloops/adapters/opencode/history.ts"],
  "mcp": {
    "codeloops-history": {
      "type": "local",
      "command": ["/absolute/prefix/bin/codeloops", "mcp"],
      "enabled": true
    }
  }
}
```

Start `"$CODELOOPS_BIN" serve` in another terminal, then quit and restart OpenCode.
Ask it to search a distinctive phrase through `history_query`. The plugin records
new native events only; it does not enumerate or import prior conversations.

If a desktop launch does not inherit shell variables, configure those variables
in its launcher environment. `CODELOOPS_OPENCODE_VERSION` is explicit because
OpenCode's v1 plugin SDK lacks a runtime-version API; without it the archive says
`unknown`. A version supplied here is operator-reported, not automatically detected.

## Cursor command hooks and cross-client recall

Use the **same data directory and service address** as OpenCode. Cursor capture is
built into the Rust binary; it needs no JavaScript runtime or transcript files.

The installed `share/codeloops/adapters/cursor/hooks.example.json` lists this
slice's seven hooks. Replace `/absolute/prefix` and `/absolute/data` with your
installation prefix and `CODELOOPS_DATA_DIR`. Append each hook definition to the
corresponding array in a trusted project's `.cursor/hooks.json`, or your existing
`~/.cursor/hooks.json`. Preserve all existing hooks and settings. Register each
CodeLoops hook once, at one level: configuring it at both levels captures twice.
Prefer project scope for a trial. Cursor 3.18.25 instantiated global hooks once
per open window during verification, so three open windows delivered each event
three times; moving the same definitions to project scope restored one delivery.
The quoted absolute executable/data paths also work with spaces and desktop
launchers that do not inherit shell environment variables.

Merge this named server into `.cursor/mcp.json` (or `~/.cursor/mcp.json`), preserving
existing servers. Use the actual data path and service address:

```json
{
  "mcpServers": {
    "codeloops-history": {
      "command": "/absolute/prefix/bin/codeloops",
      "args": ["--data-dir", "/absolute/data", "--address", "127.0.0.1:47823", "mcp"]
    }
  }
}
```

Cursor documents automatic hook configuration reload. Open a new Agent Chat and
check that the MCP server is enabled and connected; restart Cursor if needed.
With `codeloops serve` running, ask Cursor to call `history_query` with:

```json
{"request":{"operation":"search","text":"your distinctive OpenCode phrase","filter":{"source":"opencode"}}}
```

Then ask OpenCode to find a distinctive phrase from the Cursor conversation with
`filter.source = "cursor"`. Both return the same archive entry/session UUIDs as
CLI and REST. Use `history entry <uuid>` to inspect a result. Check `health` after
each client run: `sources.cursor` and `sources.opencode` report individual delivery
and failure counters. Cursor emits only `{}` on stdout, including capture failures;
errors go to stderr and the writable spool's health record. It never adds context,
rewrites prompts/results, blocks a turn, or requests an automatic follow-up.

Desktop Agent Chat is the intended client; actual version coverage and remaining
verification requirements are in [Cursor verification](verification-cursor.md).
Uninstall this preview integration by removing only the hook definitions and named
MCP server you added. Preserve other settings and the data directory.

## Capture health and recovery

The OpenCode adapter uses a synchronous `codeloops capture-opencode` subprocess to durably
enqueue each observed event, with a 15-second process timeout. The collector owns
the shared SQLite spool and persistent device/installation identities. Its legacy
filename `opencode-spool.sqlite3` is retained to preserve installed preview data.
Cursor invokes `capture-cursor` directly and atomically queues message metadata,
text, and lifecycle observations before returning. The service drains
up to 100 queued captures every 250 ms. There is no network dependency in the hook.

`codeloops health --json` reports pending/rejected deliveries, delivered count,
enqueue failure count, and the most recent error, both in aggregate and by source.
A rejected envelope remains in
the spool for inspection. Transient archive failures leave deliveries queued for
retry. Errors during enqueue go to OpenCode's log and stderr and are also recorded
in the spool when it remains writable. Failure to launch the collector or to write
the spool can only be reported by the client; those events were not captured.

After a service outage, start it again or run `codeloops flush`. A crash between
archive commit and queue deletion is safe: the same durable delivery ID is replayed.
Do not delete the spool to clear health; it contains capture state and identities.

Automatic `make setup`, `make e2e`, and `make uninstall` arrive with the final
installation/recovery slice. For this preview, remove the two entries you added
from OpenCode's configuration and restart it to stop integration. Remove the
isolated install prefix if desired. Preserve `CODELOOPS_DATA_DIR` to retain history.
