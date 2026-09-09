# OpenCode preview installation

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

## Capture health and recovery

The adapter uses a synchronous `codeloops capture-opencode` subprocess to durably
enqueue each observed event, with a 15-second process timeout. The collector owns
the SQLite spool and persistent device/installation identities. The service drains
up to 100 queued captures every 250 ms. There is no network dependency in the hook.

`codeloops health --json` reports pending/rejected deliveries, delivered count,
enqueue failure count, and the most recent error. A rejected envelope remains in
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
