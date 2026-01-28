# Development Setup

This guide covers how to set up a development environment for contributing to codeloops.

## Prerequisites

- Rust toolchain (stable): [rustup.rs](https://rustup.rs/)
- Bun (for UI development): [bun.sh](https://bun.sh/)
- Git

## Cloning the Repository

```bash
git clone https://github.com/matsilva/codeloops
cd codeloops
```

## Building

### Debug Build

```bash
cargo build
```

Binary location: `./target/debug/codeloops`

### Release Build

```bash
cargo build --release
```

Binary location: `./target/release/codeloops`

## Running Tests

### All Tests

```bash
cargo test --workspace
```

### Specific Crate

```bash
cargo test -p codeloops-core
cargo test -p codeloops-agent
cargo test -p codeloops-sessions
```

### With Output

```bash
cargo test --workspace -- --nocapture
```

## Running the CLI Locally

```bash
# Show help
cargo run -- --help

# Run with a prompt
cargo run -- --prompt "Fix the typo"

# List sessions
cargo run -- sessions list

# Start the UI
cargo run -- ui --dev
```

## Frontend Development

### Install Dependencies

```bash
cd ui
bun install
```

### Development Server

```bash
# From project root
cargo run -- ui --dev

# Or from ui directory
cd ui && bun dev
```

### Build

```bash
cd ui
bun run build
```

### Type Checking

```bash
cd ui
bun run typecheck
```

## Code Style

### Rust

The project uses standard Rust formatting:

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt --check

# Run clippy
cargo clippy --workspace
```

### TypeScript

```bash
cd ui
bun run lint
bun run format
```

## Project Structure

```
codeloops/
├── crates/
│   ├── codeloops/          # CLI binary
│   ├── codeloops-core/     # Loop orchestration
│   ├── codeloops-agent/    # Agent abstraction
│   ├── codeloops-critic/   # Critic evaluation
│   ├── codeloops-git/      # Git operations
│   ├── codeloops-logging/  # Logging and session writing
│   └── codeloops-sessions/ # Session reading
├── ui/                     # Web UI (React)
├── docs/                   # Documentation (mdbook)
├── Cargo.toml              # Workspace manifest
└── README.md
```

## Issue Tracking

This project uses [beads](https://github.com/matsilva/beads) for issue tracking. Issues are stored in `.beads/` directory.

### Common Commands

```bash
# List ready issues (no blockers)
bd ready

# Show all open issues
bd list --status=open

# Create a new issue
bd create --title="Description" --type=task

# Start working on an issue
bd update <id> --status=in_progress

# Close an issue
bd close <id>

# Sync with remote
bd sync
```

See `AGENTS.md` for detailed workflow.

## Making Changes

### 1. Find or Create an Issue

```bash
bd ready              # See available work
bd show <id>          # Review details
```

Or create a new issue:

```bash
bd create --title="Add feature X" --type=feature
```

### 2. Create a Branch

```bash
git checkout -b feature/my-feature
```

### 3. Make Changes

Edit files, add tests, update documentation.

### 4. Test

```bash
cargo test --workspace
cargo fmt --check
cargo clippy --workspace
```

### 5. Commit

```bash
git add <files>
git commit -m "feat: add feature X"
```

Follow [Conventional Commits](https://www.conventionalcommits.org/):
- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation
- `refactor:` - Code refactoring
- `test:` - Tests
- `chore:` - Maintenance

### 6. Push and Create PR

```bash
git push -u origin feature/my-feature
gh pr create
```

## Documentation

### Building Docs

```bash
# Install mdbook
cargo install mdbook

# Build
mdbook build docs

# Serve locally
mdbook serve docs --open
```

### Rustdoc

```bash
# Build API documentation
cargo doc --no-deps --open
```

## Debugging

### Verbose Logging

```bash
RUST_LOG=debug cargo run -- --prompt "test"
```

### Session Files

Sessions are stored in `~/.local/share/codeloops/sessions/`. Inspect them with:

```bash
cat ~/.local/share/codeloops/sessions/*.jsonl | jq
```

### Testing Without Agents

For testing changes that don't require actual agent execution, you can mock the agent:

```rust
#[cfg(test)]
mod tests {
    use crate::Agent;

    struct MockAgent;

    impl Agent for MockAgent {
        // ... implement with test data
    }
}
```

## Common Development Tasks

### Adding a New CLI Option

1. Edit `crates/codeloops/src/main.rs`
2. Add the option to the `Cli` struct with clap attributes
3. Handle the option in the command logic
4. Update CLI reference documentation

### Adding a Configuration Option

1. Edit `crates/codeloops/src/config.rs`
2. Add field to appropriate struct (GlobalConfig or ProjectConfig)
3. Update resolution logic
4. Update configuration documentation

### Adding a Session Field

1. Edit `crates/codeloops-logging/src/session.rs` (SessionLine enum)
2. Edit `crates/codeloops-sessions/src/types.rs` (Session structs)
3. Update parser in `crates/codeloops-sessions/src/parser.rs`
4. Update session format documentation

### Adding an API Endpoint

1. Create handler in `crates/codeloops/src/api/`
2. Add route in `crates/codeloops/src/api/mod.rs`
3. Add TypeScript types in `ui/src/api/types.ts`
4. Add client function in `ui/src/api/client.ts`
5. Update API documentation

## Getting Help

- Open an issue on GitHub
- Check existing issues for similar problems
- Review the documentation

## Contributor License

By contributing, you agree that your contributions will be licensed under the same license as the project.
