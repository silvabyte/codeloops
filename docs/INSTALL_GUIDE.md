# Install CodeLoops

Set up a shared history for your coding agents. This guide configures automatic
capture for Cursor and OpenCode across projects. You do not need to install
OpenCode to use Cursor. Other harnesses can [connect through MCP](USAGE.md#connect-another-coding-harness).

## Quick start

You need Rust/Cargo **1.97.1**, Git, Make, a C compiler/linker, and standard Unix
build tools. With rustup installed, the checkout selects the pinned toolchain.
On macOS, install the Xcode Command Line Tools with `xcode-select --install` if
needed. SQLite and Zstandard build from bundled sources.

```sh
git clone https://github.com/silvabyte/codeloops.git
cd codeloops
make start
```

`make start` builds and installs CodeLoops, registers capture and MCP for both
clients, then starts the history service in the foreground. The first build can
take a few minutes. When you see `CodeLoops listening at ...`, keep that terminal
open and reload your client:

- **OpenCode:** quit and restart.
- **Cursor desktop:** open a new Agent Chat. Restart Cursor if the
  `codeloops-history-preview` MCP server does not appear enabled and connected.

Send:

> Reply with exactly: codeloops first recall check

Then open a fresh chat and ask:

> Use CodeLoops history_query to find the earlier assistant message containing
> "codeloops first recall check". Return its text, session ID, and entry ID.
> Retrieve it from history, not from this chat.

Expect the earlier reply and its archive IDs. Capture starts with new events;
older client conversations are not imported.

Ctrl+C stops the service. Restart it with `make run`. Reload clients after setup
or an adapter upgrade. The service runs in this terminal; setup does not install
a login service or start a background daemon.

Node and npm are only needed for [contributor checks](DEVELOPMENT.md).

## Commands and configuration

| Command | What it does |
| --- | --- |
| `make start` | Build, install, register clients, and start the service |
| `make setup` | Build, install, and register clients without starting the service |
| `make install` | Build and install the binary and adapters without editing client configs |
| `make run` | Start the installed service with its saved settings |

Repeating `make start` after stopping the service is supported. Cargo reuses its
build cache, and setup preserves existing registrations without duplicating them.

Setup writes capture and MCP entries into both clients' user-global configuration,
even if only one client is installed. It preserves unrelated plugins, hooks, MCP
servers, and JSONC comments. Generated commands contain absolute executable/data
paths, so desktop launches do not need your shell's PATH or environment variables.

| Setting | Default |
| --- | --- |
| Install prefix | `~/.local/codeloops-history-preview` |
| Data directory | `~/.local/share/codeloops-history/preview` |
| Service address | `127.0.0.1:47823` |
| Profile / MCP server name | `preview` / `codeloops-history-preview` |
| OpenCode config | `$XDG_CONFIG_HOME/opencode/opencode.json` or `.jsonc`, normally under `~/.config` |
| Cursor config | `~/.cursor/hooks.json` and `~/.cursor/mcp.json` |

If both OpenCode config filenames exist, setup uses `opencode.jsonc` for new MCP
entries, matching OpenCode's precedence. It adds the capture plugin to the highest-priority
existing plugin list (`opencode.jsonc`, `opencode.json`, then legacy `config.json`)
so inherited plugins stay active. If no list exists, it creates one in the selected
config. Existing MCP entries keep their ownership/conflict checks in their source
file. No config-selection flag is needed for the normal quick start.

To explicitly select a different OpenCode config file:

```sh
make start SETUP_ARGS="--opencode-config '$HOME/.config/opencode/opencode.json'"
```

Use that same option when repeating setup. `--cursor-config-dir DIRECTORY` selects
a different Cursor config directory. `--opencode-version VERSION` supplies source
version metadata; otherwise setup tries `opencode --version` and records `unknown`
if unavailable.

Make prints progress and reports failures. For a detailed JSON setup report, call
the installed binary's `setup --json` command with the same profile and config
options. Service unavailability in that report is expected while the service is
stopped. If you have an earlier manual installation, follow
[migration](#migrate-a-manual-preview) first.

`codeloops mcp` is the stdio bridge started by the client. It needs the separate
history service for queries. See [client coverage](OVERVIEW.md#client-coverage)
before relying on Cursor Agent CLI hooks.

## Use the CLI and check capture

After the quick-start conversation, run in a second terminal:

```sh
export PATH="$HOME/.local/codeloops-history-preview/bin:$PATH"
codeloops health --json
codeloops history search "codeloops first recall check" --role assistant --json
```

The PATH change applies to this shell. Add it to your shell configuration for
future terminals, or call `~/.local/codeloops-history-preview/bin/codeloops` directly.
Outside the checkout, `codeloops serve` starts the installed service.

Expect the assistant entry with archive session/entry IDs. Then open a fresh chat:

> Use CodeLoops history_query to search for "codeloops first recall check" with
> role "assistant". Return the archived session and entry IDs. Do not answer from
> this chat's context.

The IDs should match the CLI result. This checks conversation capture and MCP
recall. To inspect file checkpoints, use a Git workspace and the
[file-change commands](USAGE.md#inspect-observed-file-changes).

## Profiles and data

Each prefix holds one profile. For a separate archive and service:

```sh
make start PREFIX="$HOME/.local/codeloops-review" PROFILE=review ADDRESS=127.0.0.1:47824
```

After stopping it, restart with `make run PREFIX="$HOME/.local/codeloops-review"`.

Use that prefix's binary for queries. Its saved setup selects the data directory
and address. Separate registered profiles each capture into their own archive;
remove registrations you no longer want collecting events.

`DATA_DIR=/absolute/path` overrides Make's profile-derived data location. Keep it
outside observed Git worktrees. To change an existing registration's profile,
paths, or address, uninstall it, reinstall, then set it up with the new values.
Reuse the same data directory to keep history and identities.

CLI settings resolve in this order: explicit `--data-dir` / `--address`, then
`CODELOOPS_DATA_DIR` / `CODELOOPS_ADDRESS`, then saved installation settings, then
built-in defaults. Without saved settings, the data default uses
`$XDG_DATA_HOME/codeloops-history/preview` or `~/.local/share/codeloops-history/preview`.
Make supplies its own `DATA_DIR` during setup.

Inside the data directory:

| Path | Contents |
| --- | --- |
| `archive/history.sqlite3` | Sessions, captures, projections, and search index |
| `archive/artifacts/` | Compressed, content-addressed payloads and file bytes |
| `opencode-spool.sqlite3` | Both clients' queue, health, sequences, and persistent identities |
| `credential` | Local service credential |

Keep the whole data directory when preserving an installation. Do not delete the
spool to reset health. Use [portable export](USAGE.md#export-a-session) for a
verified, offline session bundle.

## Troubleshoot capture

```sh
codeloops health --json
codeloops flush --json
```

`health` queries the running service. `flush` works locally without it and drains
all available queue batches until empty or no progress is possible. The running
service drains up to 100 deliveries every 250 ms. Both paths replay saved captures
and checkpoint links without rescanning Git.

| Symptom | Check / action |
| --- | --- |
| `service_unavailable` | Start `make run`. Confirm the querying binary, data path, and address match setup. |
| MCP disconnected | Check service health, then reload the client and its MCP connection. |
| No new entries | Check client hook/plugin logs and the configured executable. Only new observed events are captured. |
| Pending deliveries | Start the service or run `flush`. Inspect returned health for failures. |
| Rejected deliveries | Inspect the reported error. Rejected envelopes stay in the spool; retries cannot repair invalid input. |
| Enqueue failures | Check client logs/stderr, data permissions, and disk space. If the collector cannot launch or write the spool, the event was not saved. |
| Failed checkpoints | Inspect entry/capture checkpoint links for the reason and run `history checkpoint ID` for file-content coverage. Confirm the workspace is a Git repository. |
| Repeated Cursor entries | Check for overlapping project/global registrations. See [delivery identity](OVERVIEW.md#cursor-hook-identity-and-coverage). |

Health reports aggregate and per-source counters under `sources.opencode` and
`sources.cursor`; `pending` includes rejected deliveries still in the spool.
`workspaces_with_failed_checkpoint` counts workspace states
whose latest checkpoint attempt failed. A non-Git chat can capture messages while
its Git observation fails. Message delivery and file coverage are separate checks.

Collectors queue locally while the service is offline. A crash after archive
commit but before queue deletion replays the same durable delivery ID safely.
Transient archive failures remain queued. Diagnostics go to stderr and, when
writable, the spool health record; check the terminal running the service too.

## Upgrade or recover setup

Stop the service, update the checkout, then repeat `make start` with the original
prefix/profile/path/config options. Reload your client. Installation replaces
owned, unchanged assets and keeps the archive.

If installation or setup is interrupted, repeat the same command. Ownership
records under `PREFIX/share/codeloops/` let it finish partial work. Setup refuses
malformed JSONC, config-file symlinks, incompatible hook versions, conflicting MCP
names, or changed owned entries. Resolve the reported path and retry. Do not
restore an old whole-file backup over newer client settings.

### Migrate a manual preview

Setup does not claim existing manual registrations. Remove only the obsolete
CodeLoops plugin, hook, and named MCP entries from the relevant global and project
configs before running setup. Earlier entries often point directly to
`share/codeloops/adapters/opencode/history.ts`, invoke `capture-cursor`, or use the
MCP name `codeloops-history`. Preserve other integrations.

For an install prefix without ownership records, choose a fresh `PREFIX` or remove
its old installed binary/adapter assets before installing there. Pass the existing
archive path as `DATA_DIR` to preserve history, queued events, and identities.

## Uninstall

Stop the service, then run from the checkout:

```sh
make uninstall
# For a custom prefix:
make uninstall PREFIX="$HOME/.local/codeloops-review"
```

Reload clients to unload the integration. Uninstall removes exact owned config
entries and unchanged installed assets. It preserves the archive, spool,
credential, identities, and unrelated files. Modified owned config entries cause
a conflict; modified binary/adapter assets are retained and reported. Empty config
containers and lock files may remain.

Repeating uninstall is harmless. If interrupted removal deleted the binary,
`make uninstall` uses Cargo from the checkout to finish. The direct recovery form
is `cargo run --locked -p codeloops -- uninstall --prefix /absolute/install/prefix`.
