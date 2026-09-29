# CodeLoops

Ever wish you could just say:

> Hey Claude, refer to that session I just had in Cursor for context.

You already explained the problem. Worked through the tradeoffs. Made the
decisions. Then you switch agents and have to explain it all again.

CodeLoops gives you **unified conversation history across all your coding agents**.
Your agent can find and read those earlier sessions through MCP.

**[Get CodeLoops](#quick-start). It's free, you bum.**

## Quick start

You'll need Rust/Cargo **1.97.1**, Git, Make, and a C compiler/linker on Linux
(with systemd) or macOS.

```sh
git clone https://github.com/silvabyte/codeloops.git
cd codeloops
make start
```

That builds CodeLoops, installs it, connects capture and MCP to your clients, and
starts the history service in the background. The first build can take a few
minutes. When it says `CodeLoops is running in the background`, you're up and
running. **You can close the terminal.** The service starts at login and restarts
if it crashes.

Restart OpenCode and Codex, or open a new Cursor Agent Chat. In **Cursor or
OpenCode**, send:

> Reply with exactly: codeloops first recall check

Open a **fresh chat in any connected client, including Codex**, and ask:

> Use CodeLoops history_query to find the earlier assistant message containing
> "codeloops first recall check". Return its text, session ID, and entry ID.
> Retrieve it from history, not from this chat.

You should get the earlier reply back with its archive IDs. That's your first
conversation recalled across chats. Now try it from your other agent.

Automatic capture supports **Cursor desktop and OpenCode** today. Codex Desktop,
CLI, and the IDE extension are connected automatically through their shared MCP
configuration for recall. Your history stays local, and capture starts with new
messages. Other harnesses can [connect through MCP](docs/USAGE.md#connect-another-coding-harness)
to read it.

Use `make status` to check it, `make logs` to see recent logs, and `make stop` to
stop it and disable login startup. `make start` enables it again.
For custom paths, configuration conflicts, or upgrades, see the
[installation guide](docs/INSTALL_GUIDE.md).

## Why this exists

I started CodeLoops about two years ago, then dropped it because I figured this
would be a solved problem by now.

How is it still this hard to use a conversation from one coding agent in another?

That's why I'm working on CodeLoops again.

## Pick your way in

- Using coding agents? [Use MCP](docs/USAGE.md#query-through-mcp).
- Don't like MCP? [Use the CLI](docs/USAGE.md#find-a-conversation).
- Can't run shell commands? [Use the REST API](docs/USAGE.md#query-through-rest).

**NO MORE EXCUSES LIL BRO!!!!**

### Prefer the terminal?

In a second terminal, add the installed binary to your shell's PATH and search:

```sh
export PATH="$HOME/.local/codeloops-history-preview/bin:$PATH"
codeloops history list --json
codeloops history search "database migration" --role assistant --json
```

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
