# Performance

A performance guide for SLT — frame budget, allocation budget, optimization
patterns, and how to detect regressions. If you've used the React profiler,
the Flutter timeline, or browser DevTools' performance panel, the model here
will feel familiar: SLT is an immediate-mode renderer with a per-frame
pipeline you can measure, profile, and optimize.

## 1. Frame budget (target: 60 FPS)

At 60 FPS, each frame has a ~16.6 ms budget. SLT's per-frame pipeline,
broken down by phase:

| Phase | Target | Source |
|---|---|---|
| Closure execution (your app code) | < 2 ms | user-controlled |
| `build_tree` (commands → `LayoutNode`) | < 0.5 ms | `src/layout/tree.rs` |
| `compute` (flexbox layout) | < 1 ms | `src/layout/flexbox.rs` |
| `collect_all` (single DFS) | < 0.3 ms | `src/layout/collect.rs` |
| `render` (`LayoutNode` → `Buffer`) | < 1 ms | `src/layout/render.rs` |
| `flush_buffer_diff` (`Buffer` → ANSI bytes → stdout) | < 2 ms | `src/terminal.rs` |
| **Total framework overhead** | **< 5 ms** | |

The remaining ~11 ms is yours: terminal I/O, async work, and slack for the
OS scheduler. The pipeline runs in `slt::frame()` (`src/lib.rs:359`)
which is called once per tick by `run_with` / `run_inline_with` /
`run_static_with`.

### Measured baselines (reference HW)

The figures below are **actual `cargo bench` results**, not targets. Each is
the criterion median for the named bench in `benches/benchmarks.rs`. The
phase budgets in the table above are still per-phase targets; these are
end-to-end whole-pipeline measurements (closure → build → compute → collect
→ render → diff, and for `flush/*` the ANSI emit into an in-memory sink).

> Measured on: Apple M3 Pro, macOS 26.4, rustc 1.95.0, `--release`
> (criterion default profile), 2026-05-29.
>
> **Indicative only.** These numbers were captured on a developer machine
> while other compiles were running concurrently, so they carry scheduler
> noise (note the wide animation interval). Treat them as a
> sanity-check order of magnitude, not a contract. Re-run
> `cargo bench --bench benchmarks` on a *quiet* machine before quoting them
> in a release announcement, and re-measure on your own hardware (the
> commands are in [§3](#3-measuring-performance)).

| Bench | Terminal size | Measured (median) | Notes |
|---|---|---|---|
| `full_render_dims/80x24` | 80×24 | ~18 µs | small dashboard baseline |
| `full_render_120x40` | 120×40 | ~37 µs | header + 20 rows + progress |
| `full_render_dims/300x100` | 300×100 | ~189 µs | ultra-wide full render |
| `buffer_diff_200x50` | 200×50 | ~48 µs | cell-diff only (no emit) |
| `flush/full_redraw_200x60` | 200×60 | ~309 µs | full ANSI emit into `Vec<u8>` |
| `flush/full_redraw_300x100` | 300×100 | ~784 µs | ultra-wide full ANSI emit |
| `animation/churn_200x60` | 200×60 | ~92 µs | per-frame changing content + sparkline |

What to read from these: at 120×40 the framework build→diff cost is well
under 0.1 ms, leaving the 16.6 ms budget effectively untouched. Even the
ultra-wide 300×100 *render* path stays in the low-hundreds-of-µs range. The
**flush** path is an order of magnitude heavier than the render path —
`full_redraw_300x100` is the single largest committed cost here — because it
walks every changed cell and emits SGR/ANSI bytes. That is exactly the path
the sibling flush-allocation issue targets, and these committed figures are
its before/after baseline. The `flush/full_redraw_*` numbers are *full
redraws* (every cell dirty), which is the pathological case; steady-state
frames touch a small fraction of cells (see `flush/sparse_change_*`).

## 2. Allocation budget

The steady-state render path targets zero unnecessary heap allocations.
What we reuse, and where:

| Per-frame allocation | Status | Issue / version |
|---|---|---|
| `commands` `Vec<Command>` | reused via `FrameState.commands_buf` | #143 / v0.19.1 |
| `FrameData` (8 collection `Vec`s) | reused via `&mut FrameData` in `collect_all` | #155 / source |
| `flexbox` row/column scratch | inline `U32Stack { [u32; 16] }` | #67 / v0.18.2 |
| Group name strings | `Arc<str>` (atomic ref-count, no heap) | #139, #145 / v0.19.1 |
| `Style` commands | `Style` is `Copy` (no heap) | always |
| `Color`, `Rect` | `Copy` (no heap) | always |
| `Buffer` cells | pre-allocated `Vec<Cell>`, only resized on terminal resize | always |
| `consume_activation_keys` queue | `SmallVec<[usize; 8]>` inline | #135 / v0.19.1 |
| `separator()` repeat string | `OnceLock`-cached static | #177 / v0.19.2 |
| `set_string_inner` private helper | dedup'd from public variants | #169 / v0.19.1 |

`Command::BeginContainer` and `Command::BeginScrollable` were boxed in
v0.18.2 (#64) so the `Command` enum stays ≤ 128 bytes — small `Command`s
(text, style change) don't pay for the fat container variants on every
push.

**Target**: no unnecessary heap allocations on the steady-state render
path. New widget contributions should justify any frame-rate-path
allocation in the PR description; reviewers should push back on
`String::from`, `format!`, `Vec::new` inside the `frame()` body unless the
allocation is one-shot or amortized.

> **Working tree note**: `FrameState.commands_buf` and `FrameState.frame_data`
> exist in the v0.19.2 source tree (`src/lib.rs:600` / `:603`) and are wired
> into `frame()` at `:1187` and `:1195`. The CHANGELOG records #155 and #157
> as "Deferred to v0.19.3" because they were reverted during release triage
> and are scheduled to re-land. Treat the deferred-list items as in-flight
> until v0.19.3 ships.

## 3. Measuring performance

### `cargo bench`

```bash
cargo bench --bench benchmarks
```

The benchmark suite is defined in `benches/benchmarks.rs` and uses
`criterion`. Current benches:

- `buffer_set_string_200x50` — hot path of the render phase
- `buffer_diff_200x50` — flush-phase input
- `layout_col_10_texts` — minimal column layout
- `layout_nested_rows_cols` — 5×4 nested rows-in-column
- `full_render_120x40` — small dashboard with header + progress
- `full_render_dims/{80x24,120x40,300x100}` — same dashboard across
  terminal sizes, including the ultra-wide 300×100 stress case
- `animation/churn_200x60` — per-frame changing content + progress +
  sparkline, forcing a non-empty diff every frame
- `dashboard_200x60/slt` — SLT rendering a representative
  dashboard into an in-memory test backend (see [§5](#5-compared-to-other-ui-frameworks))
- `flush/{full_redraw,sparse_change,static}_200x60` and
  `flush/{full_redraw,sparse_change}_300x100` — ANSI emit cost into a
  hermetic `Vec<u8>` sink (gated on the `crossterm` feature)
- `widget_list_100_items`, `widget_list_sizes`, `widget_table_50_rows`,
  `widget_tabs_5`, `widget_checkbox_10`, `widget_select_10_items`,
  `widget_progress_10`

Compare results before and after a change with criterion's built-in
baseline:

```bash
cargo bench --bench benchmarks -- --save-baseline before
# ... make a change ...
cargo bench --bench benchmarks -- --baseline before
```

### Frame timing in your app

`AppState` exposes the smoothed FPS estimate and a debug toggle:

```rust
// AppState API (src/lib.rs:251, :256)
let fps = state.fps();             // exponential moving average
state.set_debug(true);             // same as pressing F12
```

When the debug overlay is active (toggled by F12 at runtime, or via
`AppState::set_debug(true)` programmatically), the `render_debug_overlay`
pass (`src/layout/render.rs:24`) draws layout outlines on top of the
frame. The overlay layer is configurable via
`DiagnosticsState.debug_layer: DebugLayer` — `All` (default), `TopMost`,
or `BaseOnly` (issue #201 in `src/lib.rs:571–587`).

There is no `RunConfig::show_fps()` builder method. To put an FPS readout
on screen, render `state.fps()` yourself in your UI closure, or rely on
the F12 overlay during development.

### Custom instrumentation

For deeper analysis, wrap a frame call:

```rust
use std::time::Instant;
let start = Instant::now();
let _keep_going = slt::frame(&mut backend, &mut state, &config, &events, &mut f)?;
println!("frame took {:?}", start.elapsed());
```

For phase-level breakdown, splice timestamps inside `frame()` itself
(`src/lib.rs:359`) and capture them under a feature flag. Don't
ship phase timers in release binaries — they show up in the steady-state
budget.

## 4. Optimization patterns (lessons from v0.18.x–v0.19.2)

### Pattern 1: Reuse allocations across frames

Bad — every frame allocates:

```rust
let mut buf = Vec::new();
collect_into(&mut buf);
```

Good — long-lived state, take/clear/refill:

```rust
struct FrameState { commands_buf: Vec<Command> }

// per frame, in the renderer:
let mut buf = std::mem::take(&mut state.commands_buf);
buf.clear();
collect_into(&mut buf);
state.commands_buf = buf; // capacity preserved for next frame
```

This is the pattern used for `commands_buf` (#143), `FrameData` (#155),
and `RichLogState` history. `mem::take` + `clear` keeps the
`Vec`'s capacity from the previous high-water mark, so steady-state
frames don't reallocate.

### Pattern 2: Inline small collections

For collections that are almost always ≤ N items, use
`SmallVec<[T; N]>` or fixed-size arrays. SLT examples:

- `consume_activation_keys` (`src/context/runtime.rs:440`) typically
  pushes 0–2 indices per frame → `SmallVec<[usize; 8]>` keeps the common
  case allocation-free (#135).
- `flexbox::U32Stack` (`src/layout/flexbox.rs:23`) is a `[u32; 16]`
  inline buffer with a heap-`Vec` overflow path (#67). Child-counts ≤ 16
  pay zero allocations per `layout_row` / `layout_column` call.

### Pattern 3: Flatten heap structures

Bad — pointer chasing, double indirection:

```rust
let plot: Vec<Vec<char>> = vec![vec![' '; w]; h];
```

Good — flat `Vec<T>` with stride math:

```rust
let plot: Vec<char> = vec![' '; w * h];
let cell = plot[y * w + x];
```

Used in chart plot buffers (`#117` / v0.19.2) and command buffers. Flat
storage is also more cache-friendly: a 200×60 chart fits in a single
allocation instead of 60 row pointers + 60 row buffers.

### Pattern 4: `Copy` types over `Clone`

`Style`, `Color`, `Rect`, `Modifiers`, `Border`, `Padding`, `Margin`, and
`Theme` are all `Copy`. Avoid `.clone()` on a `Copy` type — it compiles
but signals confusion about the cost model. Reviewers should call this
out.

```rust
let s = Style::new().bold().fg(Color::Cyan); // Copy
let s2 = s;                                   // free (memcpy of 16 bytes)
```

### Pattern 5: Buffer cell hot path

`Buffer::set_string` is the most-called write API on the render path.
Variants:

- `set_string_inner` (`src/buffer.rs:335`) — private, single insertion
  point, dedup'd from `set_string` and `set_string_with_url` (#169).
- `set_string` (`src/buffer.rs:316`) — no hyperlink, calls `_inner` with
  `link: None`.
- `set_string_with_url` (`src/buffer.rs:325`) — OSC 8 hyperlink path,
  calls `_inner` with `link: Some(&url)`. URL validation goes through
  `is_valid_osc8_url` (#168), which doesn't allocate when validation
  fails.

Image rendering went through the same flatten in v0.19.1: `image()`
emitted 841 commands per frame for a 40×20 image (`#174`); the fix
collapses the per-pixel `Command::Text` rows into a single
`container().draw(...)` raw-draw region, dropping it to one command and
saving 800 `String` allocations per frame.

### Pattern 6: Cache derivation results across frames

When a derived value depends on stable inputs, store it on the state
type and invalidate on mutation rather than recomputing per frame:

- `CommandPaletteState::filtered_indices` (#101) — fuzzy-match score is
  computed once per query change, not twice per render.
- `TableState` column widths (#195) — `recompute_widths` short-circuits
  when neither items nor filter changed.
- `ListState` lowercase-cache (#96) — set by `set_filter`; avoids
  per-keystroke `to_lowercase()` over the whole item set.

For your own derived values, use `ui.use_memo(deps, |d| compute(d))`
(`src/context/runtime.rs:651`) — the hook stores `(deps, value)` and
recomputes only on `PartialEq` deps change.

### Pattern 7: Token streaming — cache the chrome, not the stream (#273)

The dominant LLM-streaming loop is "append one token, re-render the
whole frame": `stream.push(delta)` then the entire closure runs again.
Every token re-walks the full pipeline (closure → `build_tree` →
flexbox `compute` → `collect_all` → `render`) — including large static
chrome (a chat transcript, a fixed sidebar, a status bar) that did not
change. The flush-stage row-hash (#171, `buffer.rs` `line_dirty`) only
short-circuits *emitting* unchanged rows to stdout; the upstream
build/layout/collect/render cost was already paid.

`ContainerBuilder::cached(version_key, f)` is the **author-controlled**
gate for this. You wrap the *static surroundings* — keyed off a value
you already own (a hash of the non-streaming inputs, or the
`StreamingTextState::version()` of the *other* panes) — and leave the
stream itself uncached:

```rust
# slt::run(|ui: &mut slt::Context| {
# let history_version = 3u64;
# let mut stream = slt::StreamingTextState::new();
ui.container().cached(history_version, |ui| {
    ui.text("…long chat transcript…"); // unchanged this token
});
ui.streaming_text(&mut stream);          // changes every token
# });
```

**Important — current semantics are honest, not magic.** `cached`
preserves the immediate-mode invariant exactly: `f` runs *every frame*,
so output is byte-for-byte identical to `.col(f)` and there is zero
behavior change when unused. What it adds today is a *measured,
principle-preserving stability signal*: it records the `version_key`
per call site, classifies each region as a hit (key unchanged) or miss,
and exposes the tally via `Context::region_cache_hits()` /
`region_cache_misses()`. It does **not** yet skip `f` on a hit —
eliding the body would require splicing recorded commands and replaying
focus / hit-map / scroll / raw-draw feedback, which risks reintroducing
a retained tree (rejected by Design Principle R2, "Your Closure IS the
App"). That replay is a tracked follow-up; the gate lands first so the
win is *measured, not assumed*.

The Phase-0 baseline lives in `benches/benchmarks.rs` as
`bench_streaming_append_chat` (`chrome_uncached` vs `chrome_cached`,
~2000 lines of static chrome above a streaming line): it quantifies the
per-token full-frame cost the gate is designed to eventually elide.

## 5. Compared to other UI frameworks

| Framework | Render model | Per-frame allocations | Profiler |
|---|---|---|---|
| **SLT (TUI)** | Immediate-mode, `Buffer` diff vs prev frame | Target 0 (steady state) | F12 overlay + `cargo bench` |
| **React** | Virtual DOM diff, retained components | Many (props, vnodes, fibers) | React DevTools Profiler |
| **Flutter** | Retained widget tree, RenderObject layout | Few (per-build only) | Flutter DevTools Timeline |
| **iOS UIKit** | Retained view hierarchy, Auto Layout solver | Few (constraint solver only) | Instruments |
| **ratatui** | Immediate-mode, full re-render every frame | Many (widget value types) | manual `Instant::elapsed` |

SLT is closest to ratatui in render model — both rebuild the widget
tree every frame and diff the resulting `Buffer` against the previous
one. The difference is alloc-reuse: SLT recycles `commands`,
`FrameData`, flexbox scratch, and group names across frames, where most
ratatui apps allocate fresh widget value types each `Frame::render`.
For typical TUIs, both are limited by terminal flush bandwidth (one
syscall per ANSI command was ~10× the framework cost until #172
introduced 64 KiB `BufWriter`).

### vs ratatui (reference)

`benches/benchmarks.rs` ships a `dashboard_200x60/slt` bench that renders a
representative dashboard — a bold header, 20 text rows, and a progress/gauge
— at 200×60 into the **in-memory `TestBackend`**, so the sample is SLT's
build → layout → render → diff cost only (the OS-level flush syscall is
excluded by construction).

A one-off head-to-head against ratatui 0.29 rendering the *same* logical
dashboard via `ratatui::backend::TestBackend` + `Terminal::draw` measured:

| Framework | Per-frame (median) |
|---|---|
| **SLT** (`dashboard_200x60/slt`) | ~81 µs |
| **ratatui 0.29** (reference, not bench-linked) | ~183 µs |

- **Why ratatui is not a `[dev-dependencies]` entry**: ratatui 0.29 pulls a
  transitively advisory `lru 0.12` (RUSTSEC-2026-0002) that would fail the
  release `cargo audit` gate for a benchmark-only comparison. The ratatui
  figure above is therefore a **recorded reference measurement** (ratatui
  0.29, same HW and method) rather than a CI-regenerated number; only the SLT
  arm is shipped as a live bench.
- **Methodology**: identical widget count and terminal size; both rendered
  into the framework's own in-memory test backend; criterion median,
  reference HW above; same caveats apply (indicative, re-measure on a quiet
  machine).
- **What it shows, and what it doesn't**: on this small static dashboard,
  SLT's per-frame cost is roughly half of ratatui's. This is a single
  workload, not a sweep — the two render models are close enough (both
  immediate-mode, both diff a `Buffer`) that the result will shift with
  widget mix, terminal size, and the exact ratatui widgets chosen. It is a
  starting data point, not a definitive ranking. To reproduce:
  `cargo bench --bench benchmarks -- dashboard_200x60`.

## 6. Detecting regressions

### `cargo bench` snapshot

Run before and after each PR that touches `src/layout/`, `src/buffer.rs`,
`src/terminal.rs`, or any high-traffic widget. Threshold: > 5%
regression on `full_render_120x40` or `buffer_diff_200x50` requires a
PR-description justification and a reviewer ack.

### Visual snapshot regression

`TestBackend` produces deterministic 1-frame outputs. The repo uses
`insta` for committed snapshot baselines — see `tests/snapshots.rs` and
the `tests/snapshots/` directory (10 widgets covered as of v0.19.2:
list, table, tabs, calendar, button, progress, separator, bordered_col,
row_layout, table_zebra). Add a new `insta::assert_snapshot!` for any
widget whose visual output you change; review the `.snap` diff in the PR.

### Allocation tracking (manual)

Wrap a benchmark with `dhat-rs` or run under `heaptrack` for actual
heap-profiling. Not in CI yet — case-by-case for performance-critical
PRs.

```rust
// Cargo.toml dev-dependency: dhat = "0.3"
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() {
    let _profiler = dhat::Profiler::new_heap();
    // run a render loop
}
```

The `dhat-heap.json` output opens in
[dh_view](https://nnethercote.github.io/dh_view/dh_view.html).

## 7. Anti-patterns to avoid

- **Calling widgets inside a `for` loop with thousands of items** — use
  `ui.virtual_list(&mut state, visible_height, |ui, idx| {...})`
  (`src/context/widgets_interactive/rich_markdown.rs:151`) instead of
  `ui.list(&mut state)`. `virtual_list` only renders rows in the visible
  window; a 100k-item list pays for the visible 50 rows, not all 100k.
- **Heavy derivation on every frame** — cache results in
  `ui.use_memo(deps, |d| ...)` (`src/context/runtime.rs:651`). The
  closure runs only when `deps` changes by `PartialEq`.
- **`.clone()` on `Style`** — `Style` is `Copy`. Drop the `.clone()`.
  Same for `Color`, `Rect`, `Border`, `Padding`, `Margin`, `Theme`.
- **String concatenation in hot paths** — `format!()` in a per-frame
  callback allocates every frame. Prefer `&str` and `Style::with_*`
  chains; only allocate when you must, and prefer a one-shot allocation
  cached in `use_memo` or on your state type.
- **`Vec::new()` inside the frame closure** — same problem. Move the
  buffer to long-lived state, take/clear/refill (Pattern 1).
- **Per-cell glyph allocations** — never `'│'.to_string()` per cell.
  Use `const TRACK: &str = "│"` and `set_string` (#164, #179).
- **Forgotten `#[inline]` on tiny helpers in flexbox** — Rust usually
  inlines correctly, but if you're adding a function called millions
  of times per frame and profiling shows a cost, try `#[inline]` and
  re-bench. Don't preemptively annotate everything.
- **Ignoring `cargo bench` regressions** — a 5–10% slowdown per PR
  compounds across a release. The `criterion` baseline workflow exists;
  use it.

## 8. Cross-references

- `benches/benchmarks.rs` — criterion baselines
- `tests/snapshots.rs` and `tests/snapshots/` — `insta` visual baselines
- `docs/ARCHITECTURE.md` — render pipeline overview
- `docs/DEBUGGING.md` — F12 overlay usage and layout-debug walkthrough
- `docs/PATTERNS.md` — component patterns including `use_memo`
- `CHANGELOG.md` — issue numbers cited above (#67, #135, #143, #155, #169, …)
