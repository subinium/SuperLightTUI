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
OS scheduler. The pipeline runs in `slt::frame()` (see `src/runtime/kernel.rs`)
which is called once per tick by `run_with` / `run_inline_with` /
`run_static_with`.

### Measured baselines (reference HW)

The figures below are **actual `cargo bench` results**, not targets. Each is
the criterion median for the named bench in `benches/benchmarks.rs`. The
phase budgets in the table above are still per-phase targets.

What each bench covers matters:

- `full_render_*`, `full_render_dims/*`, `animation/*` and
  `dashboard_200x60/slt` go through `TestBackend::render`, which runs
  `Buffer::reset` + the frame kernel (closure → `build_tree` → `compute` →
  `collect_all` → `render`) into an in-memory buffer. They do **not** diff
  against a previous frame, hash rows, or emit ANSI (`src/test_utils.rs`).
- `buffer_diff_200x50` is the cell diff alone; `flush/*` is
  `flush_buffer_diff` (diff + ANSI emit) into a `Vec<u8>` sink on prepared
  buffers, with no layout or render work.

No single bench measures a full production frame. The matched-harness
numbers in [§5](#vs-ratatui-matched-harness) do.

> **Two columns, two revisions.** "pre-v0.23" is the table this guide
> shipped with (Apple M3 Pro, macOS 26.4, rustc 1.95.0, 2026-05-29). It
> matches pre-v0.23 code, not the current release. "HEAD" is `15fd63a`
> (v0.25.0), Apple M3 Pro, rustc 1.98.0, 2026-10. Both were captured on a
> busy developer machine, so treat them as order of magnitude, not a
> contract. Re-run `cargo bench --bench benchmarks` on a quiet machine
> before quoting them, and re-measure on your own hardware
> ([§3](#3-measuring-performance)).

| Bench | Terminal size | pre-v0.23 (median) | HEAD `15fd63a` (median) | Notes |
|---|---|---|---|---|
| `full_render_dims/80x24` | 80×24 | ~18 µs | ~67 µs | small dashboard, no diff/flush |
| `full_render_dims/120x40` | 120×40 | ~37 µs | ~65 µs | header + 20 rows + progress |
| `full_render_dims/300x100` | 300×100 | ~189 µs | ~247 µs | ultra-wide, no diff/flush |
| `dashboard_200x60/slt` | 200×60 | ~81 µs | ~117 µs | see §5 |
| `buffer_diff_200x50` | 200×50 | ~48 µs | not re-measured | cell diff only (no emit) |
| `flush/full_redraw_200x60` | 200×60 | ~309 µs | ~503 µs | full ANSI emit into `Vec<u8>` |
| `flush/full_redraw_300x100` | 300×100 | ~784 µs | not re-measured | ultra-wide full ANSI emit |
| `flush/static_200x60` | 200×60 | — | ~0.12 µs | clean rows only, see caveat below |
| `animation/churn_200x60` | 200×60 | ~92 µs | not re-measured | changing content + sparkline |

**Regression since v0.23.** A same-session A/B of `3eb59a9` (pre-v0.23)
against `15fd63a` on the same host measured:

| Bench | `3eb59a9` | `15fd63a` | Change |
|---|---|---|---|
| `full_render_dims/120x40` | 39.0 µs | 65.1 µs | +67% |
| `dashboard_200x60/slt` | 90.4 µs | 116.7 µs | +29% |
| `flush/full_redraw_200x60` | 358 µs | 503 µs | +41% |

This is a real slowdown, not noise. A large share of it is fixed per-frame
cost: `Buffer::reset` on a 200×60 buffer takes ~70–75 µs, against ~10 µs
for ratatui's `Buffer::reset`. That is why the 80×24 and 120×40 renders now
cost about the same. Optimization work is tracked in the performance issues
linked from the audit tracker.

**`flush/static_200x60` caveat.** This bench flushes two identical buffers
whose rows are already *clean*, so the per-row hash skip (#171) short-circuits
every row and the bench reports ~0.12 µs. Production frames never hit that
path. `Buffer::reset()` marks every row dirty (`src/buffer.rs:1257`), so every
real frame, idle ones included, recomputes all row hashes before any skip. In
the matched harness an idle 200×60 dashboard still costs ~219 µs per frame,
and ~170 µs of that is the flush step (row-hash refresh + diff + swap +
reset). Do not quote the static bench as the idle-frame cost.

What to read from these: every committed bench is well inside a 16.6 ms
frame. The **flush** path is several times heavier than the render path,
because it walks every cell and emits SGR/ANSI bytes. The
`flush/full_redraw_*` numbers are *full redraws* (every cell dirty), the
worst case for output volume; steady-state frames emit few bytes (see
`flush/sparse_change_*`). They still pay the row-hash and reset cost above.

### Idle cost

What an app costs while nobody touches it, measured in a real PTY (#414,
#490): the static 200×60 dashboard from `tests/pty_idle.rs` (rounded border,
four bordered metric panels, 40 table rows) runs for 30 s with no input under
`tests/support/idle_pty.py`, which reads every output byte and the child's
CPU time from `getrusage(RUSAGE_CHILDREN)` (process start and exit
included). Release build, `--all-features`, Apple M3 Pro, Rust 1.98.0, on a
host shared with other builds (load average 6–15), so the default rows
spread widely between runs. "Base" is `release/v0.26.0` `dce9b67` with the
same harness; base and branch runs were interleaved.

```bash
cargo test --release --all-features --test pty_idle idle_cpu_report -- --ignored --nocapture
```

| Config | CPU, share of one core (runs) | Frames in 30 s | Idle output |
|---|---|---|---|
| Base, default, 16 ms tick | 3.6%, 5.9%, 8.1% | 1,555–1,612 (~53/s) | ~850 B/s |
| Default `idle_redraw(true)`, 16 ms tick | 3.0%, 3.9%, 7.2% | 1,573–1,624 (~53/s) | ~850 B/s |
| `idle_redraw(false)`, 16 ms tick | 0.08%, 0.09%, 0.09% | 3 | 0 B/s |
| Base, default, `tick_rate(1 s)` | 0.11%, 0.15%, 0.17% | 31 | 16 B/s |
| Default `idle_redraw(true)`, `tick_rate(1 s)` | 0.08%, 0.12%, 0.15% | 31 | 16 B/s |
| `idle_redraw(false)`, `tick_rate(1 s)` | 0.02%, 0.03%, 0.04% | 3 | 0.6 B/s |
| `run_async_with`, default, 16 ms tick | 4.8% | 1,698 | ~900 B/s |
| `run_async_with`, `idle_redraw(false)`, 16 ms tick | 1.0% | 6 | 0 B/s |

"Idle output" counts bytes from 1 s after start to 0.5 s before the quit
timer. With the default config every tick runs the closure, layout, render
and diff and writes a synchronized-output pair (16 bytes) although nothing
changed: about 3–8% of a core at 16 ms on this host. The default rows match
the base within run-to-run spread, so the default path did not regress.

With `idle_redraw(false)` the three frames are the startup frame, its
follow-up and the frame that runs the quit timer. The native loops still
wake once per `tick_rate` to poll input (resume and resize detection rely on
it), which is the remaining 0.02–0.09%. The 0.6 B/s at a 1 s tick is the
follow-up frame's empty synchronized-output pair, which lands inside the
window. The async loop keeps its 4 ms input-poll slices so a message wake is
seen within 4 ms; that polling is the remaining ~1% and would need a
combined input/message wait to remove.

Wake latency with `idle_redraw(false)` (`tests/pty_idle.rs`, release, same
host, three runs): a key press reaches the terminal as frame bytes in
0.2–1.6 ms and a resize in 0.1–0.3 ms. A tooltip with a 300 ms
`tooltip_delay` appears 301–302 ms after the hover with no further input.
For 20 async messages sent 50 ms apart, the time from `send` to the frame
that drains the message is p95 13.5–15.4 ms with the default config and
0.03–3.7 ms with `idle_redraw(false)`: the default loop holds a message
until the `max_fps` cap since the last tick frame expires, and an idle
loop has no recent frame for the `max_fps` cap to wait on.

## 2. Allocation budget

### v0.24 diagnostic baseline

`benches/alloc_diagnostics.rs` is a standalone release diagnostic, not a timing gate.
It covers idle rendering, typing, paste, filtered lists, static-log enqueue,
and line charts at 80x24, 120x40 and 240x80, plus per-issue chart, animation
and syntax workloads. It does not replace terminal resize, image-update or
idle-process measurements.

```bash
cargo +1.98.1 bench -p superlighttui --no-default-features --features syntax-cpp --bench alloc_diagnostics > before.csv
# Run the same command on the changed revision, using the identical harness:
cargo +1.98.1 bench -p superlighttui --no-default-features --features syntax-cpp --bench alloc_diagnostics > after.csv
SLT_AUDIT_FILTER=issue420 cargo +1.98.1 bench -p superlighttui --no-default-features --features syntax-cpp --bench alloc_diagnostics
SLT_AUDIT_FILTER=textarea_memory cargo +1.98.1 bench -p superlighttui --no-default-features --features syntax-cpp --bench alloc_diagnostics
```

Compare matching rows without running workspace gates:

```bash
python3 - before.csv after.csv <<'PY'
import csv, sys
def read(path):
    with open(path) as stream:
        return {r['workload']: r for r in csv.DictReader(line for line in stream if not line.startswith('#'))}
before, after = map(read, sys.argv[1:])
for name in before.keys() & after.keys():
    a, b = before[name], after[name]
    ratio = float(b['p50_ns']) / float(a['p50_ns'])
    delta = float(b['requested_bytes']) - float(a['requested_bytes'])
    print(f'{name}: p50 ratio={ratio:.3f}, requested bytes delta={delta:+.2f}')
PY
```

Method: 32 warm-up operations and 101 samples; each sample averages the
batch size specified in the source (1 for full frames, up to 1000 for tiny
animation calls). p50 is sorted sample 50 and p95 is sample 95. Every
allocation/reallocation is delegated to `System` and instrumented, so wall
times include allocator-counter overhead and scheduler noise. Allocation
calls include reallocations; requested bytes are allocation sizes, not RSS.
`retained_delta_bytes` is the live requested-byte change across the sample
window, and `peak_extra_bytes` is its maximum increase above the starting
live allocation total. Neither is process peak RSS or total cache capacity.

The textarea mode uses ASCII, CJK and ZWJ input near 1 KB, 100 KB and 1 MB,
40-grapheme logical lines, an 80x24 viewport, and history caps 0/8. Each
sustained paste adds 128 content graphemes and four newlines. Fixture comments
report actual source bytes, history length and total live requested bytes
for source + document + backend + history after 32+101 pastes. This is a
bounded-count history measurement, not a byte cap or a delta-undo claim.
The document grows deterministically during sampling; these are not repeated
edits of an immutable input. Idle fixtures do not contain undo snapshots.

On 2026-09-05, Apple M3 Pro, macOS 26.5.1, rustc 1.98.1
(`48a229cea`, aarch64, LLVM 22.1.8), an archived `37933f9` baseline was
compared with only the secondary chart/animation/syntax fixes applied.
Features were exactly `--no-default-features --features syntax-cpp`.
Representative sequential #420 rerun results follow; concurrent development
on this host makes the tail timings unsuitable for release speed claims.

| Workload | Before p50 / p95 | After p50 / p95 | Requested bytes/op before -> after |
|---|---|---|---|
| Histogram, 1,000 values, 32 bins | 295 / 702 us | 294 / 1,770 us | 181,863 -> 157,238 |
| Histogram, 100,000 values, 32 bins | 1,279 / 2,326 us | 897 / 4,131 us | 3,118,451 -> 223,021 |
| Sequence, 64 segments, early sample | 52.9 / 53.8 ns | 3.3 / 3.3 ns | 0 -> 0 |
| Sequence, 1,024 segments, early sample | 576 / 781 ns | 3.3 / 3.3 ns | 0 -> 0 |
| Sequence, 1,024 segments, late sample | 1,543 / 1,594 ns | 580 / 718 ns | 0 -> 0 |

Histogram preparation now scans finite values for min/max/count, then counts
bins, without allocating/sorting a data copy. The histogram comparison also
includes corrected zero bins and edge labels; output parity of the counting
algorithm is tested independently against sorted counting. In the 100,000
case the sampled extra live allocation peak fell from 1,848,576 to 113,972
bytes, while allocation *calls* increased from 634 to 641 because corrected
labels change rendering work. This is not an allocation-count win.
Sequence stores one saturating duration sum when segments are appended;
sampling late segments still scans the segment list. The new private field
does not promise a whole-frame speedup.

Correctness fixes have separate diagnostic rows. Zero bars and unchanged
Spring targets showed no material timing change. Left CJK legend rendering
kept allocation calls at 422, with requests 59,514 -> 59,644 bytes, while
preserving graphemes and row width. Warm standalone C++ highlighting changed
from 7 allocations / 246 requested bytes to 24 / 926: the old output was
unstyled text, so the additional style segments are expected, not a regression
against equivalent output. No performance win is claimed for these fixes.

These are **in-memory render-kernel** results. They do not measure terminal
flush duration/output bytes, real input-to-display latency, idle CPU,
retained OS pages, or image transport. Static-log rows measure enqueueing,
not terminal scrollback writes. No threshold here should fail shared-runner CI.

The #414 textarea follow-up compared that same archived baseline with a
snapshot of the **integrated** v0.24 work, including the input worker's #415
changes (not secondary fixes alone). The 1 MB cases below use history cap 8:

| Input / operation | Allocation calls/op before -> after | Requested bytes/op before -> after |
|---|---|---|
| ASCII idle | 24,549 -> 54 | 3,163,544 -> 27,085 |
| CJK idle | 8,444 -> 54 | 2,010,744 -> 28,845 |
| ZWJ idle | 2,489 -> 54 | 1,315,110 -> 35,885 |
| ASCII sustained paste | 49,611 -> 49,502 | 6,334,093 -> 3,186,413 |
| CJK sustained paste | 17,390 -> 17,250 | 4,066,237 -> 2,489,165 |
| ZWJ sustained paste | 5,437 -> 5,256 | 2,809,439 -> 2,391,469 |

The 1 MB ASCII fixture with eight retained undo snapshots still occupied
15,625,750 requested live bytes after the run, versus 15,634,486 before.
With history disabled those totals were 2,906,326 and 2,915,702. These include
the source string, current document and backend as well as history. The
roughly 12.7 MB history-related difference did **not** disappear: undo remains
whole-document snapshots capped by count, not a byte-bounded or delta history.
Do not infer retained-memory improvement from the lower per-frame requests.

### Steady-state allocations: target vs measured

The goal is zero unnecessary heap allocations on the steady-state render
path. **The goal is not met.** In the release-mode harness from
[§5](#vs-ratatui-matched-harness) (counting global allocator, SLT `15fd63a`,
counted after warm-up, per frame):

| Workload | SLT calls / requested bytes | ratatui 0.30.2 calls / requested bytes |
|---|---|---|
| Dashboard 200×60 (static, idle, or one changing cell) | 54 / 27.7 KB | 74 / 6.3 KB |
| 10k-item `ui.list`, 100×50 | 20,018 / 13.8 MB | 20,056 / 4.16 MB |
| 10k items via `ui.virtual_list`, 100×50 | 160 / 56 KB | — |
| `ui.table` 1000×6, 200×60 | 9,038 / 1.15 MB | 13,082 / 2.60 MB |
| `ui.table` 1000×6, `page_size = 57` | 551 / 71 KB | — |
| 60 nested bordered boxes, 240×80 | 285 / 168 KB | 273 / 46 KB |

Calls include reallocations. Requested bytes are allocation sizes, not RSS.
SLT makes fewer allocation calls than ratatui on the dashboard and the table,
but it requests 3.3–4.4× more bytes per frame on the dashboard, the full
list, and the nested layout. Full-collection widgets scale with item count, not
with what is visible. The reuse below removes specific allocations; it does
not make the frame allocation-free.

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

**Target (not yet met, see the table above)**: no unnecessary heap
allocations on the steady-state render path. New widget contributions should justify any frame-rate-path
allocation in the PR description; reviewers should push back on
`String::from`, `format!`, `Vec::new` inside the `frame()` body unless the
allocation is one-shot or amortized.

`FrameState` owns the reusable command and collection buffers. Allocation
regression tests verify that these buffers are returned after normal, quit, and
panic-recovery frames; changes to that handoff require a measured benchmark.

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
// AppState API (see src/lib.rs)
let fps = state.fps_f64();         // exponential moving average
state.set_debug(true);             // same as pressing F12
```

When the debug overlay is active (toggled by F12 at runtime, or via
`AppState::set_debug(true)` programmatically), the `render_debug_overlay`
pass in `src/layout/render.rs` draws layout outlines on top of the
frame. The overlay layer is configurable via
`DiagnosticsState.debug_layer: DebugLayer` — `All` (default), `TopMost`,
or `BaseOnly` (issue #201; see `DiagnosticsState` in `src/runtime/frame_state.rs`).

There is no `RunConfig::show_fps()` builder method. To put an FPS readout
on screen, render `state.fps_f64()` yourself in your UI closure, or rely on
the F12 overlay during development.

### Custom instrumentation

For deeper analysis, wrap a frame call:

```rust
use std::time::Instant;
let start = Instant::now();
let _keep_going = slt::frame(&mut backend, &mut state, &config, &events, &mut f)?;
println!("frame took {:?}", start.elapsed());
```

For phase-level breakdown, splice timestamps inside `frame()` itself in
`src/lib.rs` and capture them under a feature flag. Don't
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

- `consume_activation_keys` in `src/context/runtime.rs` typically
  pushes 0–2 indices per frame → `SmallVec<[usize; 8]>` keeps the common
  case allocation-free (#135).
- `flexbox::U32Stack` in `src/layout/flexbox.rs` is a `[u32; 16]`
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

- `set_string_inner` in `src/buffer.rs` — private, single insertion
  point, dedup'd from `set_string` and `set_string_with_url` (#169).
- `set_string` in `src/buffer.rs` — no hyperlink, calls `_inner` with
  `link: None`.
- `set_string_with_url` in `src/buffer.rs` — OSC 8 hyperlink path,
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

For your own derived values, use `ui.use_memo(deps, |d| compute(d))` from
`src/context/runtime.rs` — the hook stores `(deps, value)` and
recomputes only on `PartialEq` deps change.

### Pattern 7: Token streaming diagnostics (#273, #419)

The dominant LLM-streaming loop is "append one token, re-render the
whole frame": `stream.push(delta)` then the entire closure runs again.
Every token re-walks the full pipeline (closure → `build_tree` →
flexbox `compute` → `collect_all` → `render`) — including large static
chrome (a chat transcript, a fixed sidebar, a status bar) that did not
change. The flush-stage row-hash (#171, `buffer.rs` `line_dirty`) only
short-circuits *emitting* unchanged rows to stdout; the upstream
build/layout/collect/render cost was already paid.

`ContainerBuilder::cached(version_key, f)` is an **author-controlled diagnostic**
for this. You wrap the *static surroundings* — keyed off a value
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

The bounded `issue419_chrome_2000` workload in `alloc_diagnostics` uses the same
2,000 static lines and one changing, constant-length token label for both
`.col(f)` and `.cached(1, f)`. Unlike an ever-growing streaming buffer, its
input size is stable. In the secondary-only run, p50 was 549 us without the
diagnostic and 536 us with it; both requested exactly 1,823,798 bytes in
4,017 allocations per frame, with zero sample-window retained growth and
1,087,742 extra live bytes at peak. The small timing difference is noise,
not evidence that `cached` skips work. Callback/output parity is covered by
`tests/chart_anim_edge_cases.rs`.

Decision for #419: keep diagnostics-only semantics. No command/tree replay,
new preparation cache, or promised cache speedup follows from this
whole-kernel measurement. Pure wrapping/highlighting and media preparation
need their own stage measurements and invalidation/memory budgets before
adding further reuse. Existing bounded syntax preparation reuse remains
unchanged by the C++ query fix.

## 5. Compared to other UI frameworks

| Framework | Render model | Per-frame allocations | Profiler |
|---|---|---|---|
| **SLT (TUI)** | Immediate-mode, `Buffer` diff vs prev frame | Reuses command/collection buffers, but still allocates every frame (54 calls / 27.7 KB on a 200×60 dashboard; scales with item count). Zero is a goal, not met | F12 overlay + `cargo bench` |
| **React** | Virtual DOM diff, retained components | Many (props, vnodes, fibers) | React DevTools Profiler |
| **Flutter** | Retained widget tree, RenderObject layout | Few (per-build only) | Flutter DevTools Timeline |
| **iOS UIKit** | Retained view hierarchy, Auto Layout solver | Few (constraint solver only) | Instruments |
| **ratatui** | Immediate-mode, `Buffer` diff vs prev frame | Per-widget value types (74 calls / 6.3 KB on the same dashboard; scales with item count) | manual `Instant::elapsed` |

SLT is closest to ratatui in render model. Both rebuild the UI every frame
and diff the resulting `Buffer` against the previous one. SLT recycles
`commands`, `FrameData`, flexbox scratch, and group names across frames,
but that reuse does not give it a smaller allocation footprint: it makes
fewer allocation calls than ratatui on the dashboard and table workloads,
yet requests 3.3–4.4× more bytes per frame on the dashboard, full list,
and nested workloads (see
[§2](#steady-state-allocations-target-vs-measured)). Terminal write and
emulator parse time were not measured here. At 60 FPS both frameworks use a
small fraction of the 16.6 ms budget on these workloads.

### vs ratatui (matched harness)

**Why the old figure was wrong.** Earlier versions of this guide compared
`dashboard_200x60/slt` (~81 µs) with ratatui 0.29 `Terminal::draw` on
ratatui's `TestBackend` (~183 µs) and concluded SLT was "roughly half"
ratatui's cost. That comparison was unequal. SLT's `TestBackend::render`
runs reset + the frame kernel with **no diff and no flush**
(`src/test_utils.rs`), while ratatui's `Terminal::draw` diffs the buffer and
calls the backend on every frame. The old description of the SLT bench as
"build → layout → render → diff" was also wrong. Do not quote that figure.

**Method.** A standalone release-mode harness (not committed to this repo)
drives both frameworks through their real per-frame path into a `Vec<u8>`:

- **SLT**: `slt::frame_owned` with a custom `Backend` whose `flush` mirrors
  `Terminal::flush` (synchronized-output markers, row-hash refresh,
  `flush_buffer_diff`, buffer swap, `reset`/`reset_with_bg` as `run_with`
  configures it). Kitty/sixel passes are no-ops for this content.
- **ratatui 0.30.2** (crossterm 0.29): `Terminal<CrosstermBackend<Vec<u8>>>`
  and `Terminal::draw`.
- Same logical UI, terminal size, and selected row for both. `Instant`
  loop, at least 200 warm-up frames and 0.3 s, then at least 300 samples;
  median and p95 per frame; 3 runs.
- Apple M3 Pro, macOS, rustc 1.98.0, SLT `15fd63a` (v0.25.0), 2026-10.
  The host was busy, so absolute numbers carry noise; the ratios held
  across runs.

| Workload | SLT median (p95) | ratatui median (p95) | SLT / ratatui |
|---|---|---|---|
| Dashboard, 200×60 | 215 µs (235) | 107 µs (117) | 2.0× |
| 10k-item list, 100×50, `ui.list` | 1,832 µs | 648 µs | 2.8× |
| 10k-item list, 100×50, `ui.virtual_list` | 258 µs | — | — |
| Table 1000×6, 200×60, `ui.table` | 1,032 µs | 591 µs | 1.7× |
| Table 1000×6, 200×60, `page_size = 57` | 643 µs | — | — |
| 60 nested bordered boxes, 240×80 | 1,255 µs | 358 µs | 3.5× |
| Dashboard, one cell changes per frame | 221 µs | 108 µs | 2.0× |
| Dashboard, idle (nothing changes) | 219 µs | 108 µs | 2.0× |
| Dashboard, in-memory `TestBackend` (old, unequal harness) | 115 µs | 106 µs | 1.1× |

**Result: SLT is currently about 1.7–3.5× slower per frame than ratatui on
matched workloads (2× on the dashboard).** The last row reruns the old
in-memory setup at HEAD: SLT without diff against ratatui with diff. Even
with that head start SLT is slightly slower. On the matched dashboard, most
of the gap is the flush step (~165–185 µs mean for SLT vs ~86–90 µs for
ratatui).

Output bytes per frame:

| Workload | SLT | ratatui |
|---|---|---|
| Full redraw, dashboard 200×60 | 1,008 | 1,268 |
| Full redraw, 10k list 100×50 | 2,309 | 3,873 |
| Full redraw, table 200×60 | 5,292 | 6,488 |
| Idle frame | 16 | 25 |
| One changed cell | 44 | 32 |

**Where SLT wins:**

- **Output bytes.** Full redraws emit 18–40% fewer bytes, and idle frames
  emit only the 16-byte synchronized-output pair. One-cell updates emit
  *more* (44 vs 32) because SLT writes a redundant SGR reset.
- **Dependencies.** A default-feature build resolves 26 unique normal
  dependencies against 67 for ratatui 0.30.2 + crossterm. Binary size and
  build time do **not** follow: a stripped release hello-world is 1,137 KiB
  vs 540 KiB (2.1× larger, see [Binary size](#binary-size)), and a cold
  release build is about 1.6× slower.
- **Viewport-limited widgets.** `ui.virtual_list` over 10k items (258 µs)
  is faster than ratatui's `List` built from all 10k items (648 µs). This is
  not like-for-like, since a ratatui app can slice its items to the viewport
  too. Paging the table (`page_size = 57`, 643 µs) closes most of the gap
  but is still slower than ratatui's full table (591 µs).

**Root causes (macOS `sample` profiles):**

- **Buffer reset.** `Buffer::reset` costs ~70–75 µs at 200×60 against
  ~10 µs for ratatui, and runs every frame.
- **Row hashing.** `reset()` dirties every row, so every flush recomputes
  every row hash. `recompute_line_hashes` is 42.5% of the dashboard sink
  profile, idle frames included.
- **Cell writes.** In the nested-boxes workload, `Buffer::set_char` is 78%
  of the profile and `normalize_cell_symbol` alone is 62%: each single-char
  border write goes through the grapheme path (`set_grapheme_visual`) and
  re-normalizes its symbol. `build_tree`, flexbox `compute`,
  and `collect_all` together are about 1%. Layout is not the bottleneck.
- **Full-collection widgets (fixed in 0.26, #487).** At `15fd63a`
  `ui.list` and `ui.table` did work and allocated per item, not per
  visible row (see §2). They now build only the rows that fit: on the same
  harness the 10k list takes ~255 µs (109 allocations, 56 KB per frame)
  and the unpaged 1000×6 table ~600 µs (129 allocations, 60 KB) against
  ratatui's ~645 µs and ~1,035 µs.

Optimization work is tracked in the performance issues linked from the
audit tracker. Until it lands, do not describe SLT as faster than ratatui.

- **Why ratatui is not a `[dev-dependencies]` entry**: ratatui 0.29 pulled
  the advisory `lru 0.12` (RUSTSEC-2026-0002), which would fail the release
  `cargo audit` gate. ratatui 0.30.2 resolves `lru 0.18.5`, so that reason
  may no longer apply, but no in-repo comparison bench exists yet. Only the
  SLT `dashboard_200x60/slt` arm ships, and it measures the in-memory kernel
  only: `cargo bench --bench benchmarks -- dashboard_200x60`.

### Binary size

Hello-world from #491 (`slt::run` + `ui.text` + `ui.key('q')`; ratatui:
`ratatui::run` + `render_widget(&str)` + `event::read`), each in its own crate
with the default `[profile.release]` (no LTO, `opt-level = 3`,
`panic = "unwind"`), then `strip`. Apple M3 Pro, aarch64 macOS, rustc 1.98.0.
Reproduce with `bash bench/ratatui-compare/size.sh` and re-record this table
on every release.

| Program (stripped bytes) | v0.25.0 | 0.26 base (`0a8cf43`) | 0.26 (#491) |
|---|---|---|---|
| SLT, default (`crossterm`, `bidi`) | 1,063,944 | 1,198,888 | **1,164,360** |
| SLT, `default-features = false, features = ["crossterm"]` | 996,128 | 1,147,544 | **1,096,520** |
| ratatui 0.30.2, default | 553,376 | 553,376 | 553,384 |
| SLT default / ratatui | 1.92x | 2.17x | **2.10x** |

Of the 0.25 → 0.26 growth (+134,944 B default), the accessible output mode
(#469) accounts for ~32 KiB of `.text` plus a second ~16 KiB frame-kernel
copy (removed below). The rest was not attributed commit by commit.

**Composition** (default build after #491; `cargo bloat --release --crates`
and per symbol, `size -m` for sections; cargo-bloat attribution is
approximate):

| Part | SLT | ratatui |
|---|---|---|
| `.text` total | 755 KiB | 320 KiB |
| — `slt` crate / `ratatui_core` + `crossterm` | 399 KiB | 14 + 31 KiB |
| — std backtrace symbolization (gimli, addr2line, demangle) | 142 KiB | 141 KiB |
| — drop glue (std + slt) | 55 KiB | 8 KiB |
| — `hashbrown` + SipHash, slice sort instances | 23 + 21 KiB | ~0 |
| — `unicode-bidi` / `-segmentation` / `-width` | 11 / 8 / 8 KiB | 0 / 1 / 1 KiB |
| `__eh_frame` + `__gcc_except_tab` (unwind tables) | 101 + 24 KiB | 47 + 10 KiB |
| `__const` read-only data (Unicode tables, vtables) | 93 KiB | 50 KiB |

The `crossterm`-only build drops `unicode-bidi` (code plus ~19 KiB of
tables); everything else is the same.

Largest SLT symbols (default build): `run_frame_kernel` 19.9 KiB,
`accessible::Session::run` 10.2 KiB, `event::reply::Replay::decode` 6.3 KiB,
`terminal::flush::flush_buffer_diff_rows` 6.2 KiB, `width::str_width`
6.0 KiB (plus three 3.1 KiB copies), `Context::new` 5.7 KiB,
`layout::render::render_inner` 5.0 KiB, `unicode_bidi::BidiInfo::new`
4.7 KiB. By module: `runtime::accessible` 32 KiB, `runtime::kernel` 24 KiB,
`layout::tree` 21 KiB, `layout::render` 20 KiB, `event::native` 20 KiB and
`event::reply` 18 KiB (SLT's own input parser; `crossterm` itself
contributes 6 KiB), `context::core` 17 KiB, inlined `Graphemes::next`
copies 17 KiB, `str_width` and its inlined `unicode-width` walks 31 KiB,
`layout::flexbox` 15 KiB, `buffer` 12 KiB. `serde`, `image`, tree-sitter and
the syntax tables are absent with default features.

**Why SLT stays about 2x ratatui.** ratatui links only what the program
calls: a `&str` render pulls 14 KiB of `ratatui_core`. `slt::run` reaches
the whole runtime through run-time switches (layout, input parser, focus,
overlays, notifications, tooltips, debug overlay, inspector, accessible mode,
terminal probes, kitty/sixel image passes), so a hello-world already
contains the code a full app uses. That code, its drop glue and its unwind
tables make up most of the ~611 KB difference; #491 only removed
duplication.

**Reductions in #491** (stripped bytes, same toolchain, each step measured on
top of the previous one; `codegen-units = 16` reshuffles duplicate copies
between steps, so per-step numbers carry some partition noise):

| Change | hello (issue) | `examples/hello` | `examples/demo` | `examples/widgets_tour` |
|---|---|---|---|---|
| Baseline `0a8cf43` | 1,198,888 | 1,215,608 | 2,698,200 | 1,715,256 |
| `ContainerBuilder::finish`, `push_container`, `line_wrap`, `button_colored`: generic shim + non-generic body | +80 | −16,464 | **−593,392** | **−230,752** |
| `run_frame_kernel` takes `&mut dyn FnMut` (one copy, not one per run mode) | −17,584 | −17,600 | −17,552 | −17,536 |
| Debug bar formats ms / fps with integers (no float formatting) | −17,032 | −17,032 | −16,552 | −48 |
| **Total** | **−34,536 (−2.9%)** | **−51,096 (−4.2%)** | **−627,496 (−23.3%)** | **−248,336 (−14.5%)** |

The `crossterm`-only hello-world went from 1,147,544 to 1,096,520 B
(−51,024, −4.4%).

- `finish` was monomorphized once per `.col(..)` / `.row(..)` closure, each
  copy carrying the whole ~4 KB container body: 122 copies, ~520 KB of
  `examples/demo`. Every SLT app with more than a handful of containers
  pays this, so it matters far more than the hello-world number.
- The frame kernel was instantiated for the native loop and again for the
  accessible session (and once more per test closure). Passing the frame
  closure as `dyn` costs one indirect call per frame.
- The F12 debug bar was the only `f32`/`f64` `Display` user in a default
  build; `{:.1}ms` / `{:.0}fps` are now integer formatted and digit for
  digit identical (a test compares every frame time below 2 s and ranges
  near 2^32, 2^40 and 2^48 µs against `format!`, including decimal ties).
  Apps that format floats themselves (charts, gauges) still link it.

Frame time (`bench/ratatui-compare`, `time slt`, three interleaved runs of
the base and the #491 binary, median of medians): every SLT workload is
within ±2% of the base (dashboard mem 97.1 → 97.6 µs, sink 190.7 → 189.2 µs;
nested mem 852.8 → 855.2 µs, sink 1033.5 → 1028.7 µs; list, table, one-cell
and idle likewise), inside run-to-run spread.

**Target (set from this profile): not met.** The profile attributed about
80 KB of the default hello-world to duplication (second kernel copy, float
formatting for the debug bar, per-unit copies of `str_width` and
`Graphemes::next`). The target was 1,140,000 B stripped (≥ 5% under the
0.26 base) without removing features or slowing frames; #491 reached
1,164,360 B. The rejected step:

- Un-inlining `width::str_width` saved another 16.6 KB in hello-world and
  49.7 KB in `demo`, but cost ~5% (≈5 µs) on every 200x60 in-memory
  workload (dashboard, one-cell, idle), reproducibly across interleaved runs.
  Keeping the default walk inline and moving only the ambiguous-wide walk out
  of line saved 16.5 KB and cost the same. Reverted. A size-neutral version
  needs the per-grapheme callers (`buffer`, `layout::render`) to measure
  ASCII runs in bulk instead of calling `str_width` per cell.

**Follow-ups** (measured, not landed; the call sites are in files under
concurrent change):

- `unicode_segmentation::Graphemes::next` is copied into 4 codegen units
  (~10 KiB duplicate in hello, ~24 KiB in `demo`): `buffer`, `cell`,
  `layout::{tree,render,focus}`. A crate-local wrapper with one out-of-line
  `next` would keep a single copy. `next` is already an out-of-line call at
  those sites, but measure frame time as for `str_width` above.
- Direct `UnicodeWidthStr::width` calls (`terminal::flush`,
  `terminal::effects`, tooltips) keep inlined copies of the width table walk.
- `accessible::Session<I, F>` (~16 KiB) is generic over the app closure; only
  apps that call `run` with several closure types duplicate it.

**User options** (not SLT's result): `lto = true`, `codegen-units = 1`,
`opt-level = "z"`, `panic = "abort"` and `strip = true` in the application's
`[profile.release]` remove most cross-unit duplication and landing pads;
measure them on the application rather than this hello-world.

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
  `ui.list(&mut state)` for plain rows or
  `ui.virtual_list(&mut state, visible_height, |ui, idx| {...})` from
  `src/context/widgets/collections/list.rs` for custom rows. Both only
  render rows in the visible window; a 100k-item list pays for the
  visible 50 rows, not all 100k.
- **Heavy derivation on every frame** — cache results in
  `ui.use_memo(deps, |d| ...)` from `src/context/runtime.rs`. The
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
