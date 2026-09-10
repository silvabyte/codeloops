# CodeLoops

Keep your coding-agent history. Find it from the next chat.

CodeLoops captures OpenCode and Cursor conversations, tool results, and observed
Git file changes into a local archive. Search through the CLI, MCP, or REST. Read
captured file bytes after the source checkout is gone. Export a session and verify
it offline.

## Get started

[Install and connect your client](docs/INSTALL_GUIDE.md). The guide takes you from
clone to one captured conversation and a fresh-chat recall check.

Build from source with Rust **1.97.1**, Git, Make, and a C compiler/linker. No
database server or npm install is needed for installation. Linux and macOS have
[recorded verification](docs/verification-install-recovery.md).

Once installed, add the binary to your current shell's PATH and search:

```sh
export PATH="$HOME/.local/codeloops-history-preview/bin:$PATH"
codeloops history list --json
codeloops history search "database migration" --role assistant --json
```

Or ask your agent:

> Use CodeLoops history_query to find our earlier database migration discussion.
> Return the session and entry IDs with a short summary.

## Read next

| Task | Guide |
| --- | --- |
| Install, connect, upgrade, recover, or uninstall | [Installation](docs/INSTALL_GUIDE.md) |
| Search, inspect files, export, or call the API | [Usage](docs/USAGE.md) |
| Understand client coverage and limits | [Architecture and contracts](docs/OVERVIEW.md) |
| Build and test a change | [Development](docs/DEVELOPMENT.md) |
| Give an agent recall instructions | [Project instruction template](docs/AGENTS_TEMPLATE.md) |
| Check what was actually tested | [Verification and acceptance](docs/verification-install-recovery.md) |
| Find the old implementation | [Release history](CHANGELOG.md) |
