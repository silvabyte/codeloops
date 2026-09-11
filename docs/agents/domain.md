# Domain docs

## Layout

CodeLoops uses one shared domain context across its Rust crates and adapters:

- `CONTEXT.md` at the repository root: domain vocabulary.
- `docs/adr/`: durable architectural decisions.

## Read before exploring

Read root `CONTEXT.md` and ADRs relevant to the work when they exist.
If absent, proceed silently. Do not suggest creating placeholders.
`domain-modeling` creates them as terminology and decisions are resolved.

Read `docs/DEVELOPMENT.md` for commands and the code map.
Read `docs/OVERVIEW.md` before changing capture, storage, or retrieval.
All paths above are relative to the repository root.

## Consumer rules

Use glossary terms consistently in issues, proposals, hypotheses, and tests.
If a needed term is missing, reconsider the wording or note the gap for
`domain-modeling`.

Surface conflicts with existing ADRs explicitly, naming the decision and
why it may need reconsideration.

Keep AI planning/design artifacts under `history/`. Only read existing
`history/` content when asked to review past planning.
