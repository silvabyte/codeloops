# OpenCode verification

Historical slice report. Results below apply to the recorded revision, not every
later build. See [current acceptance](verification-install-recovery.md) and
[current setup](INSTALL_GUIDE.md).

Verified on Linux on 2026-09-09 with Rust/Cargo 1.97.1, OpenCode 1.18.30,
`openai/gpt-6-astra`, and the official Rust MCP SDK 3.2.0.

Application code revision: `9d66243`. PR:
https://github.com/silvabyte/codeloops/pull/51 (base `feat/session-memory-rust`).
GitHub CI run `34382799825` passed both `make check` and the isolated release install.

## Automated checks

`make check` exercises real SQLite/artifact persistence, replay/conflict semantics,
distinct repeated messages, out-of-order snapshots, removals, Unicode truncation,
artifact publication failure/corruption, query-bound pagination, offline native
event capture, streaming deltas, service restart, and matching CLI/REST/MCP results.
MCP tests launch the actual binary and negotiate the protocol using the SDK client.
The bridge test checks that native JSON is forwarded without mutation and that
durable enqueue happens before the unawaited event hook yields.

`make install PREFIX=/tmp/opencode/codeloops-history-preview` builds locked release
dependencies and installs the executable and loadable plugin without a JS build.

## Real-client walkthrough

The live check uses isolated OpenCode configuration, a temporary workspace, an
isolated CodeLoops data directory/address, and the installed executable/plugin.
It uses the existing OpenCode provider authentication.

1. With the history service stopped, OpenCode receives a prompt requesting a
   distinctive sentinel and returns that exact text.
2. `codeloops flush` drains the persisted events. Both the user prompt and assistant
   response are searchable, with their native IDs, roles, stable archive IDs, and
   source version retained. Streaming events project to one assistant message.
3. After starting, stopping, and restarting the service, CLI and REST return equal
   search results with the same entry IDs.
4. A separate real OpenCode conversation connects to the installed stdio MCP
   server, calls `history_query`, and reports the archived assistant entry ID from
   the first conversation. That MCP call/result and the new assistant response are
   themselves captured as source events.
5. Capture health reports no pending/rejected deliveries or capture failures.

The final successful walkthrough captured 99 deliveries across two conversations,
with zero pending/rejected deliveries and zero enqueue failures. Raw local evidence
is under `/tmp/opencode/history-live-_0bjyheb` (temporary, untracked). The first
archived assistant entry was `504d5ad1-bb8b-4f91-9efb-1bc316b7c0fe`; the second
conversation returned that exact ID after calling MCP. The code revision is
recorded in beads.

## Observed coverage and timing

OpenCode 1.18.30's plugin dispatcher calls event hooks without awaiting promises.
The bridge's synchronous subprocess makes queue publication complete within the
callback. The source emits initial/full part snapshots and separate text deltas;
the collector persists accumulation state and normalized snapshots transactionally.
Message/lifecycle native events and an actual MCP tool invocation were observed.

The v1 SDK omits the runtime health/version API and some newer event types even
though the installed runtime emits them. The bridge forwards native JSON and uses
`CODELOOPS_OPENCODE_VERSION` as operator-reported provenance.

This is a CLI-hosted OpenCode verification, not a claim about every desktop host,
every plugin callback, abrupt OS termination, or complete subagent transcripts.
An event cannot be captured before OpenCode delivers its callback. The synchronous
collector adds per-event process/fsync overhead; high-volume performance remains
unbenchmarked. Attachment bytes, normalized tool retrieval, and Git snapshots are
outside this slice's reported coverage. Real Cursor/cross-client acceptance belongs
to the dependent slices and final user test of the integration commit.

Source references checked at `v1.18.30`:

- `packages/opencode/src/plugin/index.ts` (unawaited event hooks)
- `packages/opencode/src/session/processor.ts` (snapshots, deltas, cleanup)
- https://opencode.ai/docs/plugins/
