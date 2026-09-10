# Installation, recovery and portable export

Implementation decisions for `codeloops-vvb.4`, following the approved
`prompt.2026-09-09-session-history-design.md`.

## User decisions

- Global setup is the default: run it once and capture across projects.
- Cursor per-hook event identity is a separate research task, `codeloops-udq`.
  Its handoff is `/tmp/opencode/codeloops-cursor-identity-research-handoff.md`.
- Preserve legitimate repeated messages; do not introduce speculative content-only
  signatures while that research is pending.

## Installation

Make wraps native install/setup/uninstall commands. Embedding loadable adapter
assets avoids an end-user npm build and lets an installed command recover setup.
Each prefix owns one profile; explicit root/address defaults are saved adjacent to
the installation and passed into generated client registrations. This avoids
depending on a desktop launcher's environment or changing shared process variables.

Configuration edits use a JSONC concrete syntax tree rather than reparsing and
rewriting whole files. Exact array values and named MCP entries form the ownership
boundary. Full-file rollback was rejected because it can erase newer unrelated user
changes. Durable intent is written before edits; interrupted setup resumes the same
plan. Locks coordinate CodeLoops writers, with original-byte rechecks for external
changes. Uninstall removes only owned entries and hash-matching installed assets.

## Export

Use an immutable, hash-addressed snapshot manifest instead of copying SQLite or
querying live pages independently. One WAL read transaction selects records and all
current/historical checkpoint links. Typed graph traversal belongs to the private
checkpoint module, while export orchestration stays behind `History::export`.

Record pages preserve public identities, provenance and receipt order. The artifact
inventory covers original envelopes/payloads, full readable content, tool values,
Git directory nodes and file/conflict-stage bytes. The root manifest publishes only
after those artifacts verify. Existing artifact chunk retrieval provides identical
REST/MCP/CLI transport behavior without server-side destination paths.

The CLI downloads to a new private directory and publishes a completion descriptor
last. An offline verifier checks root-manifest and inventory hashes/lengths. A
single giant JSON response was rejected because it duplicates large byte payloads
in memory and does not give an immutable retrieval reference for retry. Individual
record pages and root metadata are capped at the existing 8 MiB artifact bound;
oversize errors are explicit. Related sessions are references; pending deliveries
must be flushed separately. Import is outside the approved launch scope.

## Recovery and verification

The service keeps bounded queue batches. Explicit operator flush drains successive
batches until done or until errors stop progress. Both paths preserve existing
delivery IDs, source identities and boundary-captured Git references.

Installed-interface E2E uses temporary prefixes, homes, configuration, data and Git
repositories. It exercises global merge/repeat/recovery/uninstall, quoted hook
commands, offline capture and restart, conflicting/repeated delivery, cross-surface
export, nested staged/unstaged binary reconstruction and offline verification after
removing the original data. Snapshot tests cover multi-page revision histories and
concurrent ingestion. Existing publication-failure tests remain part of `make check`.

Final integrated real-client acceptance must record the actual integration commit
and both client versions. Earlier Cursor acceptance remains user-confirmed. Research
fixtures and this session's installed-interface E2E do not constitute a new desktop
run. Delivery PR base remains `feat/session-memory-rust`.
