# Installation

This guide covers how to install codeloops and its prerequisites.

## Prerequisites

Before installing codeloops, ensure you have:

### Rust Toolchain

Codeloops is written in Rust. Install the Rust toolchain via [rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### At Least One Supported Agent

You need at least one AI coding agent CLI installed:

| Agent | Installation |
|-------|--------------|
| Claude Code | [claude.ai/code](https://claude.ai/code) |
| OpenCode | [opencode.ai/docs](https://opencode.ai/docs/#install) |
| Cursor | [cursor.com/cli](https://cursor.com/cli) |

The agent binary must be in your PATH. Verify with:

```bash
# Check which agents are available
which claude
which opencode
which cursor-agent  # or 'agent'
```

### Git

Git is required for capturing diffs between iterations:

```bash
git --version
```

## Building from Source

Clone the repository and build:

```bash
git clone https://github.com/silvabyte/codeloops
cd codeloops
cargo build --release
```

The binary will be at `./target/release/codeloops`.

## Adding to PATH

Option 1: Create a symlink:

```bash
sudo ln -s $(pwd)/target/release/codeloops /usr/local/bin/codeloops
```

Option 2: Copy the binary:

```bash
sudo cp ./target/release/codeloops /usr/local/bin/
```

Option 3: Add the target directory to your PATH in `~/.bashrc` or `~/.zshrc`:

```bash
export PATH="$PATH:/path/to/codeloops/target/release"
```

## Verifying Installation

Check that codeloops is installed correctly:

```bash
codeloops --version
```

You should see output like:

```
codeloops 0.1.0
```

## First-Time Setup

Run the interactive setup to configure your default agent:

```bash
codeloops init
```

This creates a global configuration file at `~/.config/codeloops/config.toml` with your preferred defaults.

## Installing the Web UI (Optional)

The web UI is included when you build from source. To install it for standalone use:

```bash
cd ui
bun install
bun run build
```

The built UI will be in `ui/dist/`. See [Web UI Overview](../web-ui/overview.md) for usage.

## Next Steps

With codeloops installed, proceed to the [Quickstart](./quickstart.md) guide to run your first session.
