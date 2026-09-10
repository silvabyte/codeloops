# CodeLoops

Ever wish you could just say:

> Hey Claude, refer to that session I just had in Cursor for context.

You already explained the problem. Worked through the tradeoffs. Made the
decisions. Then you switch agents and have to explain it all again.

CodeLoops gives you **unified conversation history across all your coding agents**.
Your agent can find and read those earlier sessions through MCP.

**[Get CodeLoops](docs/INSTALL_GUIDE.md). It's free, you bum.**

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

## Get started

Your history stays local. Automatic capture supports Cursor and OpenCode today;
CodeLoops is built for every AI coding harness through its shared MCP layer.

[Install and connect your client](docs/INSTALL_GUIDE.md). The guide takes you from
clone to one captured conversation and a fresh-chat recall check.

Using another harness? [Connect it through MCP](docs/USAGE.md#connect-another-coding-harness)
to give it access to your history.

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
