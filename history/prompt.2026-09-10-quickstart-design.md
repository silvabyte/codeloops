# One-command quick start

Issue: codeloops-rs8

## Approved direction

Put a working quick start immediately after the README pitch. A fresh checkout
should reach a running service with `make start`, then the reader restarts their
client and proves capture and recall with two short prompts.

## Design

Use the existing Make orchestration and installed CLI. `start` depends on `setup`,
and `setup` depends on `install`. Start the service only after setup succeeds,
including with parallel Make. Cargo handles incremental builds; installation and
setup retain their existing ownership checks. `run` stays the fast restart path.

Print readable stage messages instead of echoed shell commands and setup JSON.
Keep errors visible. The service announces readiness after binding its listener,
with client reload instructions and foreground lifetime guidance.

The alternatives were a documentation-only command chain, which leaves standalone
setup broken, or a new Rust orchestration command, which duplicates Make's build
and installation responsibilities. Reuse the current boundary.

## Implementation sequence

1. Exercise real Make commands against temporary installation, home, config, and
   data paths. Cover cold setup, start, repeat invocation, custom paths/profile,
   failure short-circuiting, and capture/query through the generated hook.
2. Add ordered prerequisites, progress output, and service startup guidance.
3. Rewrite README and installation instructions around the working first run.
4. Run `make check`, `make e2e`, review UBS, and publish a PR against `main`.

Tests use fixtures rather than daily client configuration or private history.
Existing client verification reports retain their original revision scope.
