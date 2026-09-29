# Develop CodeLoops

Start from `main`. Use the [installation guide](INSTALL_GUIDE.md) to run the app;
use this guide to change it.

## Build and check

Install Rust **1.97.1** with rustfmt and Clippy, Git, Make, native C build tools,
Node **22.18+**, npm, and Python 3 for service-manager fixtures. The toolchain is
pinned in `rust-toolchain.toml`.

```sh
make check
```

This installs locked npm dependencies when needed, then runs Rust formatting,
Clippy with warnings denied, workspace tests, TypeScript typechecking, Biome with
the Ultracite preset, and the OpenCode bridge test.

| Command | Use |
| --- | --- |
| `make start` | Build, install, register clients, and enable the background service |
| `make run` | Run the installed service in the foreground for development |
| `cargo build --locked -p codeloops` | Build the debug CLI at `target/debug/codeloops` |
| `cargo test --locked -p session-history` | Check storage/query/checkpoint/export behavior |
| `cargo test --locked -p codeloops --test transports` | Check collectors and CLI/HTTP/MCP transport behavior |
| `npm run typecheck && npm run lint && npm test` | Check the OpenCode bridge after npm dependencies are installed |
| `cargo fmt --all` | Format Rust |
| `npm exec -- biome check --write adapters/opencode` | Format and apply safe bridge lint fixes |
| `make e2e` | Build a release binary and exercise its installed interfaces in isolated state |
| `make e2e-service` | Verify terminal independence, crash recovery, stop, logs, and uninstall with the real user service manager |

`make e2e` covers setup, recovery, hook execution, export, restart, and uninstall
using temporary home/config/data directories. It uses fixtures, not live client
conversations. For recorded real-client runs, see [verification](verification-install-recovery.md).

The installation tests also execute real `make setup` and `make start` commands,
including a release build, with parallel Make and isolated runtime state. They
cover a missing installation, OpenCode config precedence, inherited plugins,
explicit config selection, Codex TOML preservation/conflicts, legacy setup upgrades,
repeated setup/start, build/setup/manager failures, service ownership, and
capture/recall after restart.
Manager commands use a fixture transport that launches real installed processes;
it does not register services in your daily user manager. These run in `make check`
too; the first run builds both debug and release binaries.

`make e2e-service` opts into the real systemd user manager or macOS GUI launchd
domain. It uses a unique service name and temporary installation/client/data paths,
checks native crash restart, and removes the service afterward. Cleanup failure
preserves the temporary directory and prints its path for recovery. Both Linux
and macOS CI run this native acceptance check.

## Code map

| Path | Responsibility |
| --- | --- |
| `crates/session-history/src/` | Storage, ingestion, projections, search, artifacts, Git checkpoints, export |
| `crates/codeloops/src/` | CLI, service, MCP, collectors, queue, installation, client config edits |
| `adapters/opencode/` | Small TypeScript bridge and its test |
| `adapters/cursor/hooks.example.json` | Embedded Cursor hook set |
| `crates/codeloops/src/codex_config.rs` | Ownership-aware Codex MCP TOML edits |
| `crates/*/tests/` | Persistence, transport, and installation integration tests |
| `docs/OVERVIEW.md` | Architecture and wire contracts |

Keep client translation in the application crate. Keep storage internals behind
the history library's API. Preserve source payloads, stable IDs, replay behavior,
and explicit coverage when changing capture. See [contracts](OVERVIEW.md).

## Test without touching your daily archive

For direct CLI development, pass a separate data directory and loopback port:

```sh
cargo run --locked -p codeloops -- \
  --data-dir /tmp/codeloops-dev-data --address 127.0.0.1:47824 serve
```

Use the same flags for queries in a second terminal. For client work, use a
[separate installation profile](INSTALL_GUIDE.md#profiles-and-data) and config
overrides. Keep history data outside observed Git worktrees. Use `make e2e` for
automated installation tests rather than registering test hooks in daily clients.

## Contribute

Track work in [beads](../.beads/README.md). Repository-specific agent rules are in
[AGENTS.md](../AGENTS.md). Include the current `.beads/issues.jsonl` snapshot with
the related change; never commit its SQLite database or local credentials.

Open feature PRs against `main`. The [Session History workflow](../.github/workflows/ci.yml)
runs `make check`, an isolated `make install`, and `make e2e` on pushes and PRs to
`main` and the retained integration branch. The separate
[Claude Code workflow](../.github/workflows/claude.yml) handles `@claude` requests.
Neither workflow publishes a versioned release.

Run checks appropriate to the change. Before publishing code, run `make check`
and review the bug scan; use `make e2e` for changes to installation, recovery, or
export. Docs-only changes need command, link, and claim checks. Keep historical
verification reports scoped to their recorded revisions rather than relabeling
older evidence as a fresh test.
