# PR review — `tui/fixed-streaming-box`

Three commits, ~1.4k lines net add / 1.6k lines net delete. Hard cut from a hand-rolled crossterm renderer to a ratatui inline viewport with an mpsc-driven render task. Watcher relocated from `codeloops-tui` to `codeloops-core`. Drive-by `cargo fmt` sweep across unrelated files (called out in the commit message).

Build, all 35 workspace tests, and `clippy --all-targets -D warnings` are green.

## Verdict

Clear improvement. Architecture is cleaner (one owner of the terminal, pure `AppState`, true layering), tests went from "none on the renderer" to a smoke test against `TestBackend`, and the 12-column regression that motivated the work is now an explicit test (`renders_at_12_columns_without_panic`). A handful of small bugs and one product-visible regression below.

---

## Signal

### 1. UX regression: confidence is no longer rendered in Pretty mode (success case)

Old `RenderEvent::FinalSuccess` carried `confidence: Option<f64>` and the renderer printed `confidence high|medium|low` (`crates/codeloops-tui/src/renderer.rs:564-595` on `main`).

The new `RenderEvent::FinalSuccess` (`crates/codeloops-tui/src/app.rs:125-129`) drops `confidence`, and `main.rs` removed the field from the variant it constructs (`crates/codeloops/src/main.rs:797-810`). The `ScrollbackLine::Final` rendering in `crates/codeloops-tui/src/lib.rs:391-443` does not surface it either.

The legacy non-Pretty branch in `print_outcome` still prints `Confidence: 95%` (`crates/codeloops/src/main.rs:821-826`), so this is a Pretty-only regression. The data still exists on `LoopOutcome::Success` and is persisted to the DB. If this is intentional pruning, fine — flag it in the commit message; if not, plumb `confidence` back through `FinalSuccess` and add a span to `ScrollbackLine::Final`.

### 2. Spinner keeps showing "actor working · Ns" after the actor finished

`RenderEvent::ActorCompleted` only stashes `(exit_code, duration_secs)` into `pending_actor_done` (`app.rs:245-250`). The follow-up `RenderEvent::GitDiff` emits the `IterationDone` scrollback line but never resets `self.phase` or `self.phase_started_at` (`app.rs:252-277`). `phase` only flips when `RenderEvent::CriticStart` arrives.

Effect: between `ActorCompleted` and the orchestrator emitting `CriticStarted`, the live status line keeps rendering `actor working · X` with `X` still growing. It's usually a sub-second gap, but if anything between the two takes a beat (e.g. git diff capture on a large repo) it shows. Easy fix in `app.rs:252-277`: set `self.phase = Phase::Idle; self.phase_started_at = None;` after pushing `IterationDone`.

### 3. Long file paths get hard-clipped instead of ellipsized

The old code had `fit_path_to_width` (the bug fixed in `1cc7a53`) to truncate paths to fit. The new `render::draw` just emits `Span::raw(fe.path.clone())` (`render.rs:111`) and lets ratatui clip the line at the viewport edge.

That's fine for short paths and 80+ col terminals. For a deeply nested path on a narrow split, the user sees `crates/codeloops-tui/src/r` with no signal that it's truncated — vs. the old `…/codeloops-tui/src/render.rs`. Minor UX regression; worth restoring an ellipsizing helper (or using `ratatui::widgets::List` with `HighlightSpacing`-style truncation hooks) if you care about narrow terminals.

### 4. Dead state in `FallbackRenderer`

`FallbackRenderer.iter_files` is appended to in `RenderEvent::FileChange` and cleared in `IterationStart`, but nothing reads it (`fallback.rs:61, 78, 109, 125`). Looks like leftover scaffolding from when the fallback summarized files at iteration end. Drop the field, the `Vec::new()` init, the `clear()`, and the `push(fe.clone())` — the `clone` even removes a small allocation per file event.

### 5. `WatcherEvent.path` is unused

`crates/codeloops-core/src/watcher.rs:19` defines `path: PathBuf` and `relative_path: String`. The only consumer (`loop_runner.rs:307`) reads `event.relative_path`. The `path` field can be removed (or `path: PathBuf::from(&relative_path)` reconstructed at the call site if a future caller needs it). Trivial.

### 6. Drop-on-panic can leave the terminal in raw mode

`SessionRenderer::Drop` only sends `Msg::Shutdown` (`lib.rs:188-193`); it doesn't await the render task. The render task is what calls `disable_raw_mode` and `cursor::Show`. On the happy path, `main.rs` calls `cleanup().await` which awaits the join handle and is fine.

If a panic unwinds past `cleanup().await` (e.g. between `runner.run().await?` returning Ok and the cleanup line), `Drop` queues Shutdown but the process exits before the render task's epilogue runs — terminal stays in raw mode. Mitigation: use a panic hook that disables raw mode + shows the cursor, or scope-guard the raw-mode bracket. Low likelihood, but the failure mode is "user's shell is unusable until they `reset`."

### 7. `run_tty` is untested

`render_smoke.rs` exercises `render::draw` against `TestBackend` (good, this is the bit most likely to break). The `tokio::select!` loop in `run_tty` — channel draining, `insert_before` of scrollback lines, the `Phase::Done → break` shortcut, the `tick.tick()` path — has no coverage. A test that drives a `Terminal<TestBackend>` through a sequence of `RenderEvent`s would catch ordering regressions (e.g. forgetting to flush a scrollback line before the final draw).

### 8. Silent `_ =` on `terminal.draw` and `terminal.insert_before`

`lib.rs:237, 243, 254, 260` all swallow the `io::Result`. If the backend disconnects mid-loop, the render task keeps spinning, dropping every frame on the floor. Not a correctness issue, but a `tracing::debug!` on first-error would help diagnose "TUI froze" reports.

### 9. Prompt area is hard-capped at two visual rows

`render.rs:28` reserves `Constraint::Length(2)` for the wrapped prompt. With `Wrap { trim: true }` over `(width - 2)` columns, that's enough for ~150 chars at 80 cols, but a long natural-language prompt will get clipped without an indicator. If you want to keep it bounded, append a `…` marker when the wrapped output would exceed two rows. If not, `Constraint::Min(1)` with a sensible upper bound would auto-grow.

### 10. Architectural win (worth calling out)

The watcher relocation reverses a layering inversion: `codeloops-core` no longer depends on `codeloops-tui`. The dependency graph now flows `tui → logging` and `core → logging`, with `tui` as a leaf. Worth keeping that invariant; consider adding a `cargo deny` rule or a comment in `codeloops-core/Cargo.toml` to prevent future regressions.

---

## Noise (filtered out)

These looked suspicious on first read but turned out to be fine:

- The `Mutex<Option<JoinHandle<()>>>` on `SessionRenderer.handle` — `cleanup` is `&self` and idempotent; std mutex is held only across a `take()`, the await happens after the lock is dropped. Correct.
- `state.tick()` is only called when `state.is_active()` — intentional, the spinner shouldn't animate when there's no spinner shown.
- `initial_viewport_height` clamps at min 8 even on tiny terminals — ratatui's `autoresize` handles real-time resize, and the 8-row minimum is a "won't fit cleanly" edge case rather than a bug.
- The reformatting in `crates/codeloops-db/src/projects.rs`, `crates/codeloops/src/api/{extractors,mod,prompt}.rs`, `crates/codeloops/src/{projects,ui}.rs`, `crates/codeloops-logging/src/{events,lib}.rs`, and `crates/codeloops/src/main.rs` non-TUI sections — all `cargo fmt --all` output, called out in the commit message.
- `Paragraph::clone()` at `render.rs:63` — micro; the original isn't reused, but it's a single Paragraph clone per frame, not worth a diff.
- Order between `SetAgentNames` and `Header` in the FIFO channel — verified that `main.rs` calls `set_agent_names` before `runner.run()` emits `LoopStarted`, so by the time the render task reads `Header`, agent names are already in `AppState`.
- `LogEvent::ActorOutput` / `AgentStreamLine` falling through to `{}` in `on_log_event` — those streams are persisted by the logger; the TUI explicitly doesn't display them.
