# Install CodeLoops

Set up a shared history for your coding agents. This guide configures automatic
capture for Cursor and OpenCode across projects, plus MCP recall for Codex Desktop,
CLI, and the IDE extension. You do not need to install every client. Other harnesses
can [connect through MCP](USAGE.md#connect-another-coding-harness).

## Quick start

You need Rust/Cargo **1.97.1**, Git, Make, a C compiler/linker, and standard Unix
build tools. Linux uses a systemd user session; macOS uses launchd. With rustup
installed, the checkout selects the pinned toolchain.
On macOS, install the Xcode Command Line Tools with `xcode-select --install` if
needed. SQLite and Zstandard build from bundled sources.

```sh
git clone https://github.com/silvabyte/codeloops.git
cd codeloops
make start
```

`make start` builds and installs CodeLoops, registers capture for Cursor and
OpenCode, registers MCP for Cursor, OpenCode, and Codex, then enables a background
user service. The first build can take a few
minutes. The command returns after the service passes its health check. When you
see `CodeLoops is running in the background`, you can close the terminal and
reload your client:

- **OpenCode:** quit and restart.
- **Codex Desktop, CLI, or IDE extension:** restart the client, then use `/mcp`
  to confirm `codeloops-history-preview` is connected.
- **Cursor desktop:** open a new Agent Chat. Restart Cursor if the
  `codeloops-history-preview` MCP server does not appear enabled and connected.

In Cursor or OpenCode, send:

> Reply with exactly: codeloops first recall check

Then open a fresh chat in any connected client, including Codex, and ask:

> Use CodeLoops history_query to find the earlier assistant message containing
> "codeloops first recall check". Return its text, session ID, and entry ID.
> Retrieve it from history, not from this chat.

Expect the earlier reply and its archive IDs. Capture starts with new events;
older client conversations are not imported.

The service starts at login and restarts after a crash. Reload clients after setup
or an adapter upgrade.

Node and npm are only needed for [contributor checks](DEVELOPMENT.md).

## Commands and configuration

| Command | What it does |
| --- | --- |
| `make start` | Build, install, register clients, and enable/restart the background service |
| `make setup` | Build, install, and register clients without starting the service |
| `make install` | Build and install the binary and adapters without editing client configs |
| `make stop` | Stop the background service and disable login startup |
| `make status` | Show managed process state and service health |
| `make logs` | Show recent service logs |
| `make run` | Run in the foreground for development; Ctrl+C stops it |

Repeating `make start` updates and restarts the managed service. Cargo reuses its
build cache, and setup preserves existing registrations without duplicating them.
Run `make stop` before using foreground `make run` on the same address.

Linux installs a systemd user unit; macOS installs a LaunchAgent. Each installation
has a profile/prefix-derived service name to keep installations separate. Service
definitions and ownership records are managed by CodeLoops. Linux logs go to the
user journal; macOS logs go to `PREFIX/share/codeloops/service.log`.

User services run while the user session is active and start at the next login.
For an SSH-only Linux account that must keep running after logout, enable lingering
with `loginctl enable-linger "$USER"`.

Setup writes capture entries for Cursor and OpenCode and MCP entries for Cursor,
OpenCode, and Codex at user-global scope, even if a client is not installed. It
preserves unrelated plugins, hooks, MCP servers, and JSONC/TOML comments. Generated
commands contain absolute executable/data paths, so desktop launches do not need
your shell's PATH or environment variables. Codex can recall captured Cursor and
OpenCode history, but CodeLoops does not automatically capture Codex conversations yet.

| Setting | Default |
| --- | --- |
| Install prefix | `~/.local/codeloops-history-preview` |
| Data directory | `~/.local/share/codeloops-history/preview` |
| Service address | `127.0.0.1:47823` |
| Profile / MCP server name | `preview` / `codeloops-history-preview` |
| OpenCode config | `$XDG_CONFIG_HOME/opencode/opencode.json` or `.jsonc`, normally under `~/.config` |
| Cursor config | `~/.cursor/hooks.json` and `~/.cursor/mcp.json` |
| Codex config | `~/.codex/config.toml` |

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
a different Cursor config directory. `--codex-config FILE` selects a different
Codex TOML config. `--opencode-version VERSION` supplies source version metadata;
otherwise setup tries `opencode --version` and records `unknown` if unavailable.

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
Outside the checkout, use `codeloops service start`, `stop`, `status`, or `logs`.
`codeloops serve` remains the direct foreground command.

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

Use `make stop PREFIX="$HOME/.local/codeloops-review"` to stop that installation.
Restart with the same `make start` options, or its installed `service start` command.

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
Make supplies its own `DATA_DIR` during setup. Managed service commands always use
the saved installation settings; query overrides do not reconfigure the service.

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
| `service_unavailable` | Run `make status`, `make logs`, and `make start`. Confirm the querying binary, data path, and address match setup. |
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
writable, the spool health record; check `make logs` too.

## Upgrade or recover setup

Update the checkout, then repeat `make start` with the original prefix/profile/path/config
options. It restarts the managed service with the updated binary. Reload your
client. Installation replaces owned, unchanged assets and keeps the archive.

If upgrading from the older foreground workflow, press Ctrl+C in that service's
terminal once before `make start`. CodeLoops reports an occupied address rather
than killing an unidentified listener.

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

Run from the checkout:

```sh
make uninstall
# For a custom prefix:
make uninstall PREFIX="$HOME/.local/codeloops-review"
```

Uninstall stops the managed service, disables login startup, and removes its owned
definition before removing the executable. Reload clients to unload the integration.
Uninstall removes exact owned config entries and unchanged installed assets. It preserves the archive, spool,
credential, identities, and unrelated files. Modified owned config entries cause
a conflict; modified binary/adapter assets are retained and reported. Empty config
containers and lock files may remain.

Repeating uninstall is harmless. If interrupted removal deleted the binary,
`make uninstall` uses Cargo from the checkout to finish. The direct recovery form
is `cargo run --locked -p codeloops -- uninstall --prefix /absolute/install/prefix`.
