# PR #50 review

## Summary
- Signal-grade findings: 2
- Items checked-and-OK: 8

## Findings (signal only)

### F1: `SpinnerState.total` is incremented every FileChange but never read
- **Severity**: Cleanup (half-finished diff)
- **Location**: `crates/codeloops-tui/src/renderer.rs:163` (field decl), `:329` (write site), `:435` (init in CriticStart)
- **Reproduction**: `grep -n state\.total crates/codeloops-tui` — one write, zero reads.
- **Why this is wrong**: The field's doc comment ("currently informational; reserved for future +N more") asserts an intention. clippy doesn't flag the unused-read because `+= 1` counts as a use. Shipping a counter that has no observable effect implies the diff is half-merged: either the streaming row should display "(+N more)" once `recent.len() == box_height`, or the field shouldn't exist. The PR clearly chose to land the truncated VecDeque view without that summary, so the field is dead state.
- **Fix**: Remove the field and its three writes. (Applied.)

### F2: `fit_path_to_width` overflows term_w by one column at the narrow cliff
- **Severity**: Correctness bug
- **Location**: `crates/codeloops-tui/src/layout.rs:51` (the `.max(1)`)
- **Reproduction**: `term_w = 12`. `fit_path_to_width("anything/at/all.rs", 12)` returns `"…"` (1 char). Total printed line = `CONTENT_COL(10) + sigil(1) + space(1) + "…"(1)` = **13 visible cols** on a 12-col terminal. The terminal wraps the row, the next tick's `MoveUp(box_height + 1)` is now off by one per wrapped line, and the streaming box drifts — exactly the failure mode the PR set out to eliminate.
- **Why this is wrong**: `saturating_sub(CONTENT_COL + 2)` is the available *path budget*. When that budget is 0, there's literally no room for path bytes; the line `(indent)(sigil)(space)` already fills the row. Forcing `.max(1)` re-introduces a phantom column. The pre-fix unit test `fit_path_narrow_terminal_degrades` (layout.rs:195) actually pinned the buggy output (`"…"`) instead of the invariant the function is supposed to uphold.
- **Fix**: Drop `.max(1)`; return an empty string when `avail == 0`. Update the narrow-terminal test to assert the new (correct) result and add a width-invariant test that exercises the cliff. The new test fails on pre-fix HEAD because at `term_w=12` the function returned `"…"`, making `total = 13 > 12`.

```diff
-    let avail = term_w.saturating_sub(CONTENT_COL + 2).max(1);
-    truncate_path(path, avail)
+    let avail = term_w.saturating_sub(CONTENT_COL + 2);
+    if avail == 0 {
+        return String::new();
+    }
+    truncate_path(path, avail)
```

(Applied, plus updated `fit_path_narrow_terminal_degrades` and added `fit_path_total_visible_width_never_exceeds_term_w`.)

## Items checked and OK

1. **Cursor math under terminal scrolling**: Steady-state math holds. Walked it on a 24-row terminal with `box_height = 8`: first tick prints box+spinner (9 rows of `writeln!`), cursor lands at row R+9, next tick `MoveUp(9)` returns to row R. When the layout straddles the bottom of the viewport, exactly one scroll happens and the cursor + box both shift up by one row in lockstep, so subsequent ticks redraw cleanly. The clamp `[3, 15]` for `box_height` keeps `box_height + 1 ≤ 16`, well below any realistic terminal viewport. The pathological `term_h ≤ 4` case does still scroll-on-every-tick, but the clamp lower-bound of 3 is by design (the box would be useless smaller than 3 rows) and that terminal size is exotic.
2. **Resize behavior**: `tick()` recomputes `term_w` (so width does adapt) but **not** `term_h`/`box_height` (captured once at `ActorStart`). This means the PR description's "next tick should pick up new width/height" claim is half-true: width yes, height no. Filed under Out-of-scope below since it is a pre-existing limitation in this code path (the prior `file_lines_below`-based math also did not react to resize), the corresponding PR-description checkbox is explicitly unchecked, and a clean fix needs a separate `last_rendered_rows` field to anchor the `MoveUp` math during a height transition (with leftover-row clearing for shrinks and a hard reset for grows).
3. **`clear_spinner_area`**: For actor (`box_height ≥ 3`) — `MoveUp(box_height + 1)` lands at the top of the box; the `Clear + MoveDown` loop wipes every box row plus the spinner row; the trailing `MoveUp(box_height + 1)` returns the cursor to row R, exactly where `RenderEvent::ActorDone` then writes its first `writeln!` (replaces the spinner row in place). For critic (`box_height = 0`) — `total = 1`, MoveUp 1 / clear / MoveDown / MoveUp 1; cursor ends at the spinner's row, and `CriticDone`'s first `writeln!` overwrites it. Same shape as the prior code's critic path, just with `box_height + 1` in place of the special-cased `1`.
4. **`RenderEvent::ActorDone` data source**: Only one production emit site exists: `crates/codeloops-tui/src/lib.rs:169` inside the `LogEvent::GitDiffCaptured` arm. `file_events: self.file_events.clone()` is sourced from the same `Vec<FileEvent>` that gets pushed inside `LogEvent::FileChanged` (lib.rs:148). So the canonical accumulator lives in `SessionRenderer`, not in the renderer; dropping the `current_file_events` fallback in `TuiRenderer` is safe — both sources were always populated in lockstep, the renderer copy was redundant.
5. **Tick / FileChange concurrency**: Both paths funnel through `tui_renderer: Arc<Mutex<SessionRenderer>>`. The 100ms ticker (main.rs:699) calls `r.tick()` inside `tui_for_tick.lock()`. The logger callback (main.rs:507) calls `r.on_log_event(event)` inside `renderer_for_callback.lock()` — and `on_log_event` is what dispatches `RenderEvent::FileChange`. Same Mutex, no race on `state.recent`.
6. **`total` field**: Fixed (F1).
7. **Path truncation correctness**: `truncate_path` uses `chars().count()` / `chars().skip()`, which is correct at the codepoint level (no byte-slice panic). Display-width vs char-count is a known imperfection for CJK / combining-character paths but it is pre-existing and not introduced or aggravated by this PR. ANSI escapes don't enter `path` because the sigil is appended after truncation. The off-by-one cliff is fixed (F2).
8. **`\r` + `writeln!` pattern**: After `Clear(CurrentLine)` the cursor stays at its prior column, so the leading `\r` is load-bearing only if we entered `tick()` with the cursor at non-zero column. In the established invariant the cursor is always at column 0 (each prior `writeln!`'s `\n` lands there, and `MoveUp` does not move the column), so `\r` is defensive but not needed. Harmless on Windows: cooked-mode `\n` already gets `\r` translation, and an explicit leading `\r` just re-zeros the column we're already at.
9. **Drop / Ctrl+C cleanup**: `cleanup()` is unchanged — only restores cursor visibility. The Ctrl+C handler in main.rs:680 calls `cleanup()` (not `clear_spinner_area`), so the box+spinner remain in scrollback after interrupt. Same behavior as pre-PR; not a regression.
10. **`MAX_FILE_EVENTS` overloading**: The constant is now used in exactly one place (final-list cap in `RenderEvent::ActorDone`). The doc comment "Max file events shown before collapsing" still describes that use case (collapsed via `… and N more files`). `COLLAPSED_SHOW = 12` is now unused at runtime but is `pub` so removing it is a public-API change — out of scope.

## Out of scope / explicitly rejected as noise

- **Vertical resize mid-actor not handled** (`renderer.rs:217`, `:313` capture-once). Pre-existing limitation; PR description's checkbox for this case is unchecked, and a correct fix requires `last_rendered_rows` tracking with shrink-clear and grow-reset logic. Recommend a follow-up: store `last_rendered_rows: usize` on `SpinnerState`, recompute `stream_box_height()` at the top of `tick()`, MoveUp by `last_rendered_rows + 1`, and on a height change clear the old footprint via the `clear_spinner_area`-style dance and reset `printed = false` so the next render paints fresh.
- **`COLLAPSED_SHOW` is dead at runtime** (`layout.rs:25`). `pub` const; removing is an API break. Out of scope.
- **`term_h ≤ 4` causes per-tick scroll cascade** because the `[3, 15]` clamp's lower bound is 3 (so layout = 4 rows). Not realistic; user with a 3-row terminal has bigger problems.
- **`truncate_path` uses codepoint count, not display width**, so wide CJK or combining characters can still over-/under-fill by 1-2 cols. Pre-existing; unchanged by this PR.
- All formatting / rustfmt sweep changes in `fallback.rs`, `lib.rs`, `watcher.rs` — cosmetic.

## Reproducer for F2 (terminal cursor math is hard to unit-test directly)

```bash
# Open a terminal sized to exactly 12 columns wide:
printf '\e[8;30;12t'
# Run codeloops with a TTY-attached pretty renderer and any prompt
# that triggers an actor file write. The streaming box's first event
# row prints `          + …` (13 cols) on a 12-col terminal, wraps,
# and from then on the spinner+box stutter as MoveUp drifts.
```

After the fix, the line becomes `          + ` (12 cols, sigil + trailing space, empty path) — the row fits and redraws stay anchored.
