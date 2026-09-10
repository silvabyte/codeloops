# Session-history preview installation

## Install once, capture across projects

Prerequisites: Rust/Cargo **1.97.1**, Git, Make, a C compiler/linker and the platform's
normal native build tools. SQLite and Zstandard are bundled. Linux is verified;
macOS client evidence from earlier revisions is linked in the verification reports.
Development checks require Node **22.18+** and npm. Installation and setup do not
require a separate JavaScript package install or build.

```sh
make check
make install
make setup
make run
```

Setup registers both capture and MCP **globally for your user**. Run it once, then
use either client in any project. Repeating the same setup finishes an interrupted
registration or restores missing owned entries without accumulating registrations.

Defaults:

| Setting | Value |
| --- | --- |
| Installation | `~/.local/codeloops-history-preview` |
| Data | `~/.local/share/codeloops-history/preview` |
| Service | `127.0.0.1:47823` |
| Profile / MCP name | `preview` / `codeloops-history-preview` |
| OpenCode config | `$XDG_CONFIG_HOME/opencode/opencode.json[c]`, normally `~/.config/opencode/` |
| Cursor configs | `~/.cursor/hooks.json` and `~/.cursor/mcp.json` |

Setup reports its executable, data directory, address, configuration paths, local
capture health and service availability. Before `make run`, unavailable service
health is expected: hooks still queue captures locally. The data directory must
remain outside observed Git worktrees.

**Quit and restart OpenCode after setup or an adapter upgrade.** Open a new Cursor
Agent Chat and check that `codeloops-history-preview` is connected; restart Cursor
if needed. Setup records the installed OpenCode version when `opencode --version`
is available, or `unknown`. You can supply `--opencode-version` explicitly.

Generated adapters/hooks/MCP commands contain absolute executable and data paths,
with correct quoting for spaces and apostrophes. Desktop clients do not need to
inherit shell environment variables. The installed CLI reads its profile settings;
explicit `--data-dir` / `--address` and their `CODELOOPS_*` environment variables
override those defaults.

### Separate previews

Choose a distinct prefix, profile/data path and loopback port for a second preview:

```sh
make install PREFIX="$HOME/.local/codeloops-review"
make setup PREFIX="$HOME/.local/codeloops-review" PROFILE=review ADDRESS=127.0.0.1:47824
make run PREFIX="$HOME/.local/codeloops-review"
```

`DATA_DIR=...` overrides the Make-derived profile data location. Subsequent `make run`
uses the saved installation settings. Each prefix holds one profile; changing an
existing profile's paths requires uninstalling that registration first. History is
preserved and can be selected again. Separate registered profiles each capture to
their own archives; use only the intended preview for a given acceptance run.

For isolated client configuration, pass `SETUP_ARGS` with `--opencode-config PATH`
and `--cursor-config-dir DIRECTORY`. If both global `opencode.json` and
`opencode.jsonc` exist, select the intended file explicitly. JSONC comments and
unrelated settings are preserved. Config-file symlinks, malformed JSONC, incompatible
hook versions and conflicting named servers produce explicit errors.

### Recovery and uninstall

Ownership records live under `PREFIX/share/codeloops/`. Setup writes its intent
before changing client files, uses atomic file replacement, and serializes its own
configuration writers. On interruption, rerun the same `make setup`. If a client
file changed during the operation, setup stops and reports the path; retry after
resolving the conflict. Whole-file backups are not restored over newer user settings.

Existing manual registrations are not automatically claimed as owned. Remove the
specific obsolete CodeLoops entries before switching from an earlier manual preview
to setup, especially any project-level capture registration that would overlap the
global one. Other plugins, hooks and MCP servers remain in place.
Prefixes installed before ownership records also require a fresh `PREFIX`, or
removal of their old installed binary/adapter assets before installing again.
Reuse the existing data directory to retain queued events and archive identities.

```sh
make uninstall
# For another prefix:
make uninstall PREFIX="$HOME/.local/codeloops-review"
```

Uninstall removes exact owned entries and unchanged installed assets. Modified
owned MCP/hook options cause a conflict rather than overwriting the edit. Modified
binary/adapter assets are retained and reported. Unrelated files, the archive,
credential, spool and device/source-installation identities are preserved. Empty
configuration containers and lock files may remain. Stop the foreground service
and restart clients to unload the removed integration. Repeating `make uninstall`
after its binary is removed is harmless.
If interrupted removal left ownership records behind, `make uninstall` uses the
checkout's Cargo command to finish. The equivalent recovery command is
`codeloops uninstall --prefix /absolute/install/prefix`.

Global multi-window Cursor delivery identity is being researched separately in
`codeloops-udq`. Setup avoids adding the same registration twice; no content-only
deduplication is applied to independent hook invocations.

### Automated installed verification

```sh
make e2e
```

This builds the locked release binary and installs it into a temporary prefix with
isolated home/config/data directories. It verifies global setup, recovery, installed
hook execution, CLI/MCP/REST export agreement, exact captured bytes, repeated and
conflicting deliveries, service restart and ownership-aware uninstall. It uses
adapter fixtures; final real OpenCode/Cursor conversations remain a separate test.

## Reference: earlier manual preview configuration

The following shapes identify entries created by earlier preview installations.
New installations should use `make setup` above.

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
ten conversation/lifecycle/tool hooks. Replace `/absolute/prefix` and `/absolute/data` with your
installation prefix and `CODELOOPS_DATA_DIR`. Append each hook definition to the
corresponding array in a trusted project's `.cursor/hooks.json`, or your existing
`~/.cursor/hooks.json`. Preserve all existing hooks and settings. Register each
CodeLoops hook once, at one level: configuring it at both levels captures twice.
An earlier global trial recorded three deliveries with three windows open;
project scope stopped that repetition, but its underlying cause was not established.
The separate Cursor identity research tracks this observation.
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
Uninstall a manually configured integration by removing only the hook definitions and named
MCP server you added. Preserve other settings and the data directory.

## Capture health and recovery

The OpenCode adapter uses a synchronous `codeloops capture-opencode` subprocess to durably
enqueue each observed event, with a 60-second process timeout. Awaited pre-action
and tool-completion hooks archive Git file state before returning. Error/turn
callbacks carry explicit late timing coverage. Keep the data directory outside
the observed Git worktree. The collector owns
the shared SQLite spool and persistent device/installation identities. Its legacy
filename `opencode-spool.sqlite3` is retained to preserve installed preview data.
Cursor invokes `capture-cursor` directly and atomically queues message metadata,
text, and lifecycle observations before returning. The service drains
up to 100 queued captures every 250 ms. There is no network dependency in the hook.

`codeloops health --json` reports pending/rejected deliveries, delivered count,
enqueue failure count, and the most recent error, both in aggregate and by source.
`workspaces_with_failed_checkpoint` counts session/workspace states whose last
attempt failed; inspect entry/capture checkpoint links for the reason. File-content
exclusions and detected instability are reported by `history checkpoint <id>`.
A rejected envelope remains in
the spool for inspection. Transient archive failures leave deliveries queued for
retry. Errors during enqueue go to OpenCode's log and stderr and are also recorded
in the spool when it remains writable. Failure to launch the collector or to write
the spool can only be reported by the client; those events were not captured.

After a service outage, start it again or run `codeloops flush`. A crash between
archive commit and queue deletion is safe: the same durable delivery ID is replayed.
Do not delete the spool to clear health; it contains capture state and identities.

An explicit `codeloops flush` drains all available batches until empty or until
delivery stops making progress. Inspect its returned health for rejected or pending
records. The background worker processes at most 100 deliveries per tick. Both
paths replay captured envelopes and stored checkpoint links; neither rescans Git.

## Retrieve observed file changes

Use session and workspace IDs from `history list`, or a tool entry ID from search:

```sh
codeloops history search "your tool command" --kind tool --json
codeloops history changes --session-id SESSION --workspace-id WORKSPACE --json
codeloops history changes --entry-id TOOL_ENTRY --workspace-id WORKSPACE --json
codeloops history checkpoint CHECKPOINT --json
codeloops history compare BASELINE BASELINE --before-layer head --after-layer index --json
codeloops history compare BASELINE BASELINE --before-layer index --after-layer worktree --json
codeloops history file CHECKPOINT BASE64_PATH --layer worktree --offset 0 --limit 65536 --json
codeloops history artifact INPUT_OUTPUT_ERROR_OR_PATCH_HASH --json
```

MCP `history_query` and `/v1/history/query` accept the corresponding `operation`
and snake_case fields. Follow `next_cursor`/`next_offset`. See
[checkpoint verification](verification-git-checkpoints.md) for tested behavior and
the historical client walkthrough. Prior Cursor acceptance is user-confirmed;
final integrated client acceptance is tracked in the installation verification.
After installing the
updated OpenCode adapter, quit and restart OpenCode to load its new hooks.

## Portable session export

```sh
codeloops history export SESSION --output /absolute/path/to/new-bundle --json
codeloops verify-export /absolute/path/to/new-bundle --json
```

Export freezes one session's committed records and reachable artifacts. Pending
spool deliveries are not included; flush and inspect health before exporting if
you need them. The service is required for download; verification is fully offline.
The output parent must exist and the output directory must be new.

A bundle contains `export.json` (completion descriptor), `manifest.json` and raw
`artifacts/SHA256` files. Record pages preserve session/entry/capture UUIDs, source
provenance, every capture revision, current projections and checkpoint coverage.
Full message text and tool data are available by content hashes; entry excerpts
retain their truncation markers. All referenced Git manifest nodes, conflict-stage
contents and HEAD/index/worktree bytes are included. Paths inside Git manifests
remain base64 path bytes, not destination filesystem paths.

The exporter checks every referenced artifact before returning a manifest reference.
Download verifies each SHA-256 and byte length, fsyncs files, then publishes
`export.json` last. An interrupted download leaves an incomplete directory without
a completion descriptor; remove that incomplete output or choose a new destination
before retrying. `verify-export` detects missing/corrupt artifacts and requires no
source repository, original archive, service or credentials.

Without `--output`, CLI returns the same descriptor as REST/MCP
`{"operation":"export","session_id":"SESSION"}`. Fetch `manifest_hash` through
`artifact` queries, then the manifest's inventory, following every `next_offset`.
The database snapshot is consistent even during concurrent ingestion; artifact
references remain immutable during later download. Export does not copy database
files, service credentials, local profile configuration or unrelated sessions.
Raw captured payloads retain their original source-reported provenance.

Record pages and the root manifest each have an 8 MiB limit. Oversized metadata
fails explicitly instead of returning a truncated bundle. Total artifact bytes may
exceed that limit. On-demand patches can be derived from the included file layers;
source-reported missing/partial coverage remains missing/partial. Related sessions
are referenced, not recursively exported. Import/restore into a live archive is a
future operation.
