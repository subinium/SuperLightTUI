# Performance

<!-- slt-check-default: illustrative fragments require application context -->

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
  against an earlier frame, hash rows, or emit ANSI (`src/test_utils.rs`).
- `buffer_diff_200x50` is the cell diff alone; `flush/*` is
  `flush_buffer_diff` (diff + ANSI emit) into a `Vec<u8>` sink on prepared
  buffers, with no layout or render work.

No single bench measures a full production frame. The matched-harness
numbers in [§5](#vs-ratatui-matched-harness) do.

> **Four revisions.** "pre-v0.23" is the table this guide shipped with
> (Apple M3 Pro, macOS 26.4, rustc 1.95.0, 2026-05-29). "v0.25.0" is
> `15fd63a`, Apple M3 Pro, rustc 1.98.0, 2026-10. "0.26 base" is
> `release/v0.26.0` at `df6690e`, after the frame-kernel, flush and
> layout-arena work; "0.26" adds the command, text and border work on top.
> All were captured on a busy developer machine, so treat them as order of
> magnitude, not a contract. Re-run `cargo bench --bench benchmarks` on a quiet machine
> before quoting them, and re-measure on your own hardware
> ([§3](#3-measuring-performance)).

| Bench | Terminal size | pre-v0.23 | v0.25.0 `15fd63a` | 0.26 base `df6690e` | 0.26 | Notes |
|---|---|---|---|---|---|---|
| `full_render_dims/80x24` | 80×24 | ~18 µs | ~67 µs | 12.3 µs | **7.0 µs** | small dashboard, no diff/flush |
| `full_render_dims/120x40` | 120×40 | ~37 µs | ~65 µs | 13.2 µs | **7.9 µs** | header + 20 rows + progress |
| `full_render_dims/300x100` | 300×100 | ~189 µs | ~247 µs | 15.1 µs | **8.8 µs** | ultra-wide, no diff/flush |
| `dashboard_200x60/slt` | 200×60 | ~81 µs | ~117 µs | 15.1 µs | **8.8 µs** | see §5 |
| `buffer_set_string_200x50` | 200×50 | — | ~284 µs | 10.1 µs | **8.2 µs** | `set_string` hot path |
| `buffer_diff_200x50` | 200×50 | ~48 µs | not re-measured | 1.9 µs | **1.9 µs** | cell diff only (no emit) |
| `widget_table_50_rows` | 120×60 | — | — | 103.8 µs | **40.9 µs** | ` │ `-ruled rows (non-ASCII text path) |
| `widget_list_100_items` | 80×40 | — | — | 9.4 µs | **8.8 µs** | |
| `flush/full_redraw_200x60` | 200×60 | ~309 µs | ~503 µs | 172.8 µs | **171.1 µs** | full ANSI emit into `Vec<u8>` |
| `flush/full_redraw_300x100` | 300×100 | ~784 µs | not re-measured | 430.4 µs | **428.3 µs** | ultra-wide full ANSI emit |
| `flush/static_200x60` | 200×60 | — | ~0.12 µs | 51.9 µs | **51.1 µs** | identical, fully written rows, see below |
| `animation/churn_200x60` | 200×60 | ~92 µs | not re-measured | 5.5 µs | **3.3 µs** | changing content + sparkline |

The two 0.26 columns are one same-session run each (criterion
`--warm-up-time 1 --measurement-time 3`, Apple M3 Pro, rustc 1.98.0,
2026-10, load average ~5).

**Regression since v0.23.** A same-session A/B of `3eb59a9` (pre-v0.23)
against `15fd63a` on the same host measured:

| Bench | `3eb59a9` | `15fd63a` | Change |
|---|---|---|---|
| `full_render_dims/120x40` | 39.0 µs | 65.1 µs | +67% |
| `dashboard_200x60/slt` | 90.4 µs | 116.7 µs | +29% |
| `flush/full_redraw_200x60` | 358 µs | 503 µs | +41% |

This was a real slowdown, not noise; a large share of it was fixed
per-frame cost (`Buffer::reset` on a 200×60 buffer took ~70–75 µs). 0.26
removed it: reset re-blanks only written spans (#485), cell writes
normalize once (#484), the flush skips untouched rows and coalesces runs
(#486, #495), and the layout tree and widget text are recycled (#488,
v0.26). Every bench above is now below its pre-v0.23 figure.

**`flush/static_200x60`.** Two identical buffers whose every row is
written. Rows untouched in both frames are skipped in O(1) by the
written-span invariant (#486), but written rows are compared cell by cell,
so this bench is the cost of an unchanged but fully drawn screen: ~51 µs
for 12,000 cells (the #486 target was 40 µs, see §5 for why it was not
reached). An idle frame of a sparse UI is much cheaper: the matched-harness
idle dashboard costs ~10 µs per frame in total, ~1.8 µs of it in flush.

What to read from these: every committed bench is well inside a 16.6 ms
frame. The **flush** path is several times heavier than the render path,
because it walks every cell and emits SGR/ANSI bytes. The
`flush/full_redraw_*` numbers are *full redraws* (every cell dirty), the
worst case for output volume; steady-state frames emit few bytes (see
`flush/sparse_change_*`).

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

After the frame-kernel and flush work (#483–#489, `f14fd80`) merged, one
15 s re-run on the same host (load ~8) measured: default 16 ms 1.03% of a
core (832 frames), `idle_redraw(false)` 0.12% (3 frames, 0 B/s); 1 s tick
0.07% vs 0.04%; `run_async_with` 16 ms 2.66% vs 0.91%. Cheaper frames
narrow the gap, but an idle opted-in loop still does no frame work and
writes nothing.

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
(v0.26 replaced the snapshots with delta records; see the next section.)

### Textarea undo history and large documents (#415, v0.26)

`TextareaState` and `TextInputState` record each undo step as an edit
record — the changed rows (a text input: the changed bytes) before and
after, with the shared first-row prefix and last-row suffix trimmed —
against one materialized copy of the document taken at the first recorded
edit. Records are computed by diffing that copy against the live document
when a step is recorded, so direct writes to the public `lines` / `value`
are captured exactly as the snapshots captured them. Records are bounded by
`history_max` steps and `TextareaState::history_max_bytes` (default 8 MiB;
the newest step is always kept). A property test drives random edit /
undo / redo / external-write sequences through the delta history and the
pre-0.26 snapshot implementation and requires identical documents,
restored cursors and history lengths after every step.

Measured on 2026-10-18, Apple M3 Pro, rustc 1.98.0, release, shared host
(load average 7–10). "Before" is `release/v0.26.0` `b1e4f66`; "after" is
the same tree with this change. Same harness both sides.

**Retained memory** (`tests/textarea_retained_memory.rs`, counting
allocator, live bytes retained by one frame of edits; 1 MB document of
40-glyph rows):

```bash
cargo test --release --test textarea_retained_memory -- --ignored --nocapture --test-threads=1
```

| Fixture | Before | After |
|---|---|---|
| ASCII, default cap 100, 150 one-byte pastes | 156,121,362 B | 1,587,647 B |
| ASCII, cap 8, same edits | 12,495,296 B | 1,568,227 B |
| CJK, default cap 100, same edits | 119,026,962 B | 1,216,783 B |
| ASCII, history disabled, same edits | 5,172 B | 5,172 B |
| ASCII, default cap, 10 pastes of 100 KB | 24,397,869 B | 6,193,962 B |
| Undo all 150 steps (redo tip), ASCII cap 100 | +1,359,998 B | +109 B |
| `text_input`, 1 MB single line, 100 Backspaces | 100,004,622 B | 1,013,767 B |

After the change almost all retained history is the one document copy
(~1.56 MB for the 1 MB ASCII document, which is ~1.76 MB live as rows); the
100 records themselves are ~20 KB (`history_bytes()`). The 100 KB paste row
retains the copy of the grown 2 MB document plus ~1.6 MB of inserted rows.
History is no longer proportional to `history_max × document size`; the
worst case is one document copy plus `history_max_bytes` of records plus
the newest record.

**Large paste scaling** (`SLT_AUDIT_FILTER=textarea_paste cargo bench
--bench alloc_diagnostics`): one paste per sample, a full 80x24 frame, on a
fresh clone of a document whose history already holds one step; 41 samples
(21 when document + payload exceed 1 MB), p95 is the ceil(0.95 n)-th
sample. µs and allocation calls per paste:

| Alphabet / document / payload / position | p50 before -> after | p95 before -> after | Allocations before -> after |
|---|---|---|---|
| ASCII / 1 KB / 100 KB / end | 402 -> 138 | 569 -> 152 | 2,557 -> 2,537 |
| ASCII / 1 KB / 1 MB / end | 1,142 -> 1,159 | 1,194 -> 1,228 | 24,511 -> 24,491 |
| ASCII / 100 KB / 100 KB / middle | 226 -> 206 | 229 -> 221 | 7,392 -> 4,957 |
| ASCII / 100 KB / 1 MB / middle | 1,270 -> 1,276 | 1,284 -> 1,331 | 29,346 -> 26,911 |
| ASCII / 1 MB / 100 KB / end | 1,003 -> 765 | 1,054 -> 798 | 51,283 -> 26,897 |
| ASCII / 1 MB / 100 KB / middle | 1,012 -> 780 | 1,051 -> 807 | 51,294 -> 26,908 |
| ASCII / 1 MB / 1 MB / end | 2,007 -> 1,799 | 2,067 -> 1,824 | 73,234 -> 48,848 |
| ASCII / 1 MB / 1 MB / middle | 2,105 -> 1,871 | 2,156 -> 1,944 | 73,248 -> 48,862 |
| CJK / 1 MB / 100 KB / middle | 404 -> 359 | 409 -> 380 | 17,427 -> 9,167 |
| CJK / 1 MB / 1 MB / middle | 1,088 -> 1,080 | 1,104 -> 1,164 | 24,869 -> 16,609 |
| ZWJ / 1 MB / 100 KB / middle | 481 -> 475 | 492 -> 498 | 4,831 -> 2,568 |
| ZWJ / 1 MB / 1 MB / middle | 1,194 -> 1,179 | 1,348 -> 1,250 | 6,876 -> 4,613 |

The full 36-row matrix (ASCII / CJK / ZWJ × 1 KB / 100 KB / 1 MB documents
× 100 KB / 1 MB payloads × end / middle) is printed by the command above.
End and middle insertion cost the same within noise: rows are spliced, so
position does not matter. Cost scales with **both** payload and document
size. The payload part (normalizing CR/LF, building the inserted rows,
rendering) is unchanged; the document part dropped by the snapshot clone
(about 24k allocations for the 1 MB ASCII document) but remains, because
an editing frame still clones the document once for exact
`Response::changed` and compares it afterwards, and recording a step
compares the history copy with the document. The 1 KB-document rows
(p50 402 -> 138 for the first) are dominated by sampling noise in the
before run; their allocation counts are unchanged.

**Undo, redo and typing** (`cargo bench --bench textarea_history`,
criterion medians, 1 MB document, one frame per operation):

| Operation (end / middle of document) | ASCII before -> after | CJK before -> after |
|---|---|---|
| First key of a typing burst (records a step) | 895 -> 653 / 860 -> 636 µs | 344 -> 293 / 728 -> 275 µs |
| Paste 100 KB | 989 -> 729 / 959 -> 755 µs | 399 -> 334 / 485 -> 344 µs |
| Undo a 1 KB paste | 1,402 -> 592 / 1,371 -> 627 µs | 464 -> 242 / 475 -> 261 µs |
| Redo it | 1,114 -> 620 / 1,121 -> 639 µs | 612 -> 260 / 405 -> 270 µs |
| Wrapped idle, `word_wrap(60)` | 13,508 -> 71 µs | 5,820 -> 76 µs |

"Before" ran the identical bench file against an exported `b1e4f66` tree,
immediately followed by "after". Single runs on a loaded host: the CJK
"before" medians spread by up to ~1.8x between runs (an earlier CJK redo
run measured 387 µs, typing-burst middle 417 µs), so read ratios, not
absolute times. Undo and redo no longer clone the document: they compare
it with the history copy and apply one record. Typing still clones the
document once per editing frame for `Response::changed`.

**Wrapped idle** (`SLT_AUDIT_FILTER=textarea_wrapped_idle`, 1 MB document,
`word_wrap(60)`, 80x24, no input): ASCII p50 12,835 -> 73 µs (p95 13,095 ->
80), CJK p50 5,926 -> 76 µs (p95 6,068 -> 80); requested bytes per frame
1,601 KB -> 28 KB. The textarea now keeps its soft-wrap layout across frames
and re-wraps only when the rows or the width change, checked by one byte
comparison of the rows against a contiguous copy (so a wrapped textarea
retains about one more copy of its text plus the layout). An editing or
navigation frame on a wrapped document still re-wraps the whole document.

*Attribution of the v0.24 "6.654 -> 7.230 ms" wrapped CJK idle sample.* The
same out-of-tree harness (TestBackend 80x24, 1 MB CJK, `word_wrap(60)`,
101 samples after 20 warm-ups, three rounds, cleanest round shown) was built
against each revision: `37933f9` (v0.23 docs) 5.79 ms, `c05c856` (v0.24.0)
5.57–5.62 ms, `15fd63a` (v0.25.0) 5.56–5.63 ms, `b1e4f66` 5.91–5.96 ms
p50. A 1 KB wrapped document took 0.085 / 0.080 / 0.081 / 0.036 ms, so the
frame kernel is under 1.5% of the 1 MB frame. v0.24 did not regress wrapped
idle in a matched run; the earlier sample difference is within this host's
run-to-run spread (one v0.23 round measured 17.6 ms under load). The cost
was the per-frame whole-document wrap in the textarea, not the frame kernel
(#483/#484), and the wrap cache above removes it.

### Steady-state allocations: target vs measured

The goal is zero unnecessary heap allocations on the steady-state render
path. **Status: met for everything SLT allocates on its own; what remains
is text the app passes in.** Since v0.26 the framework's per-frame storage
(commands, collection buffers, the layout tree, flexbox scratch, #488) is
recycled, container commands are stored inline instead of boxed, and
widgets that build their own row text (`ui.list`, `ui.table`, separators,
progress bars, list and pager hints) format into strings recycled from the
last frame's tree. Measured with the committed harness
(`cd bench/ratatui-compare && cargo run --release --features count-alloc -- alloc`,
ANSI-sink arm, 50 warm-up frames, mean of 200; Apple M3 Pro, rustc 1.98.0),
allocation calls / requested bytes per frame:

| Workload | SLT before #488 (`f14fd80`) | SLT 0.26 | ratatui 0.30.2 |
|---|---|---|---|
| Dashboard 200×60 (static, idle, or one changing cell) | 54 / 27,664 B | **21 / 172 B** | 79 / 8,445 B |
| 10k-item `ui.list`, 100×50 | 109 / 55,958 B | **0 / 0 B** | 20,056 / 4.16 MB |
| 10k items via `ui.virtual_list`, 100×50 | 160 / 56,166 B | **98 / 2,064 B** | — |
| `ui.table` 1000×6, 200×60 | 129 / 60,222 B | **0 / 0 B** | 25,104 / 5.95 MB |
| `ui.table` 1000×6, `page_size = 57` | 131 / 60,156 B | **0 / 0 B** | — |
| 60 nested bordered boxes, 240×80 | 285 / 168,254 B | **60 / 350 B** | 273 / 45,965 B |

Calls include reallocations. Requested bytes are allocation sizes, not RSS.
Every remaining allocation is a `String` the workload itself creates:

- **Dashboard (21):** 20 `format!` rows and `ui.text("Header")`.
  `ui.text` takes `impl Into<String>`, so a `&str` becomes one owned
  `String`. With `forbid(unsafe_code)` the method cannot tell a borrowed
  `&str` from an owned `String` to skip that copy, and changing the
  signature would break every caller.
- **Virtual list (98):** the row closure's two `format!` calls per visible
  row (an allocation plus one growth each).
- **Nested boxes (60):** one `String` per `&str` label.

An app that formats into a reused `String` it owns still hands `ui.text`
a fresh `String` each frame. Once the frame's tree is recycled, that
string's buffer goes back into the pool for widget rows.

Before #488 the layout tree allocated one `children` vector per container
(regrowing 4 → 8 → 16 → 32 for the dashboard's 23-row column, 368 B per
node) and one 128 B `TextNodeData` box per text node, and flexbox spilled
its scratch to the heap for containers over 16 children. Until v0.26 every
container also boxed its command arguments (#64), 76 of the nested
workload's 136 allocations. `tests/alloc_budget.rs` asserts zero steady
allocations for `ui.list` and `ui.table`, pins the dashboard at ≤ 21 per
frame and the per-container cost at ≤ 1 (its label), and
`bench/ratatui-compare/tests/regression.rs` holds the table above as
ceilings.

What we reuse, and where:

| Per-frame allocation | Status | Issue / version |
|---|---|---|
| `commands` `Vec<Command>` | reused via `FrameState.commands_buf` | #143 / v0.19.1 |
| `FrameData` (8 collection `Vec`s) | reused via `&mut FrameData` in `collect_all` | #155 / source |
| `flexbox` row/column scratch | inline `U32Stack { [u32; 16] }`; wider containers borrow a per-thread pooled `Vec<u32>` | #67 / v0.18.2, #488 / v0.26 |
| Layout tree (`children` vectors, `TextNodeData` boxes, overlay list) | recycled via `FrameState.layout_arena` (`LayoutArena`), trimmed after sustained smaller frames | #488 / v0.26 |
| Group name strings | `Arc<str>` (atomic ref-count, no heap) | #139, #145 / v0.19.1 |
| `Style` commands | `Style` is `Copy` (no heap) | always |
| `Color`, `Rect` | `Copy` (no heap) | always |
| `Buffer` cells | pre-allocated `Vec<Cell>`, only resized on terminal resize | always |
| `consume_activation_keys` queue | `SmallVec<[usize; 8]>` inline | #135 / v0.19.1 |
| `separator()` repeat string | `OnceLock`-cached static, copied into a pooled buffer | #177 / v0.19.2, v0.26 |
| `set_string_inner` private helper | dedup'd from public variants | #169 / v0.19.1 |
| Container command arguments | stored inline in `Command` (136 bytes); only a rare `title` is boxed | v0.26 |
| Widget row text (list, table, separator, progress, hints) | `Context::pooled_string`, backed by the layout arena's `StringPool` of cleared strings from the last frame's tree (bounded: 256 per size class, 512 KiB total) | v0.26 |

`Command::BeginContainer` and `Command::BeginScrollable` were boxed from
v0.18.2 (#64) to v0.25 so the `Command` enum stayed ≤ 128 bytes. Since
v0.26 they are inline: `Command` grew from 104 to 136 bytes, and in the
matched harness the larger copy costs less than the malloc/free pair per
container it replaced.

**Target**: no unnecessary heap allocations on the steady-state render
path. Met for framework-owned storage and widget-built text (see the table
above); text the app creates is the app's allocation. New widget
contributions should format row text with `Context::pooled_string` and
justify any other frame-rate-path
allocation in the PR description; reviewers should push back on
`String::from`, `format!`, `Vec::new` inside the `frame()` body unless the
allocation is one-shot or amortized.

`FrameState` owns the reusable command and collection buffers and the
layout arena. Allocation
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

`cargo bench --bench textarea_history` (#415) covers textarea undo
recording, undo/redo, 100 KB pastes and wrapped idle frames on 1 MB
documents; `cargo bench --bench code_editor` covers code-editor keystrokes.

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
  pay zero allocations per `layout_row` / `layout_column` call; wider
  containers borrow the overflow `Vec` from a per-thread pool (#488).

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
| **SLT (TUI)** | Immediate-mode, `Buffer` diff vs prev frame | Reuses command, collection, layout-tree and widget-text storage; what remains is text the app passes in (21 calls / 172 B on a 200×60 dashboard, 0 for `ui.list` / `ui.table`) | F12 overlay + `cargo bench` |
| **React** | Virtual DOM diff, retained components | Many (props, vnodes, fibers) | React DevTools Profiler |
| **Flutter** | Retained widget tree, RenderObject layout | Few (per-build only) | Flutter DevTools Timeline |
| **iOS UIKit** | Retained view hierarchy, Auto Layout solver | Few (constraint solver only) | Instruments |
| **ratatui** | Immediate-mode, `Buffer` diff vs prev frame | Per-widget value types (79 calls / 8.4 KB on the same dashboard; scales with item count) | manual `Instant::elapsed` |

SLT is closest to ratatui in render model. Both rebuild the UI every frame
and diff the resulting `Buffer` against the previous one. SLT recycles
`commands`, `FrameData`, the layout tree, flexbox scratch, group names and
widget row strings across frames. It makes fewer allocation calls and
requests fewer bytes per frame than ratatui on every matched workload (see
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

**Method.** `bench/ratatui-compare` (outside the workspace, own
`Cargo.lock`) drives both frameworks through their real per-frame path into
a `Vec<u8>`; its README has the full method and the text-equivalence check:

- **SLT**: `slt::frame_owned` with a `Backend` whose `flush` mirrors
  `Terminal::flush` (synchronized-output markers, the written-span row skip
  and run-coalesced `flush_buffer_diff`, buffer swap, `reset_with_bg` as
  `run_with` configures it). Kitty/sixel passes are no-ops for this content.
- **ratatui 0.30.2** (crossterm 0.29): `Terminal<CrosstermBackend<Vec<u8>>>`
  and `Terminal::try_draw`, with the same visible text per workload.
- `cargo run --release -- time`: one untimed frame, at least 200 warm-up
  frames and 0.3 s, then up to 1.5 s or 20,000 samples; median per frame.
  Three interleaved runs of the v0.26 base (`df6690e`) and this release;
  the table is the median of the three medians.
- Apple M3 Pro, macOS, rustc 1.98.0, 2026-10, on a shared host (load
  average ~5). Absolute numbers carry noise; the ratios held across runs.

| Workload | SLT v0.25.0 | SLT 0.26 base `df6690e` | SLT 0.26 | ratatui 0.30.2 | SLT 0.26 / ratatui |
|---|---|---|---|---|---|
| Dashboard, 200×60 | 213.6 µs | 13.2 µs | **10.3 µs** | 110.2 µs | 0.09× |
| 10k-item list, 100×50, `ui.list` | 1,834.0 µs | 72.3 µs | **23.0 µs** | 622.2 µs | 0.04× |
| 10k-item list, 100×50, `ui.virtual_list` | 262.8 µs | 73.8 µs | **25.6 µs** | — | — |
| Table 1000×6, 200×60, `ui.table` | 1,037.9 µs | 166.4 µs | **55.6 µs** | 998.2 µs ¹ | 0.06× |
| Table 1000×6, 200×60, `page_size = 57` | 636.4 µs | 163.8 µs | **55.0 µs** | — | — |
| 60 nested bordered boxes, 240×80 | 1,104.8 µs | 175.4 µs | **159.0 µs** | 344.2 µs | 0.46× |
| Dashboard, one cell changes per frame | 218.1 µs | 13.2 µs | **10.2 µs** | 110.6 µs | 0.09× |
| Dashboard, idle (nothing changes) | 213.2 µs | 13.2 µs | **10.3 µs** | 110.3 µs | 0.09× |

¹ ratatui draws the table with SLT's ` │ ` column rules and `─┼─` header
rule (11 cells per row) so both arms show the same text; see the harness
README.

In-memory `TestBackend` (render only, no diff or flush), same runs:
dashboard 8.6 µs vs ratatui 114.2 µs, list 15.0 vs 621.0, table 39.0 vs
990.6, nested 101.5 vs 350.0.

**Result: SLT 0.26 is faster than ratatui 0.30.2 on every matched
workload**, from about 2× on the nested boxes to 10-27× on the dashboard,
list and table. v0.25.0 was 1.7-3.5× *slower* on the same workloads; the
frame kernel, buffer and flush work of 0.26 (#483-#489, #493-#495) and the
text and allocation work after it account for the change.

Output bytes per frame (first frame = full paint):

| Workload | SLT | ratatui |
|---|---|---|
| Full redraw, dashboard 200×60 | 1,000 | 1,290 |
| Full redraw, 10k list 100×50 | 2,301 | 3,873 |
| Full redraw, table 200×60 | 5,284 | 9,568 |
| Full redraw, nested boxes 240×80 | 26,422 | 25,879 |
| Idle frame | 16 | 25 |
| One changed cell | 36 | 32 |

Other comparisons:

- **Output bytes.** Full redraws emit 22-45% fewer bytes except the nested
  boxes (2% more), and idle frames emit only the 16-byte
  synchronized-output pair. A one-cell update emits 4 bytes more than
  ratatui (36 vs 32).
- **Dependencies and size.** A default-feature build resolves 26 unique
  normal dependencies against 67 for ratatui 0.30.2 + crossterm. Binary
  size and build time do **not** follow: the stripped release hello-world
  is about 2.1× larger (1.6× with the size profile; a dashboard-class app
  1.6× and 1.5×, see [Binary size](#binary-size)) and a cold release build
  is about 1.6× slower.

**Where the remaining time goes (macOS `sample`, 0.26):**

- **Nested boxes** (every row of the 240×80 screen written each frame):
  the flush compares every written cell with the last frame
  (~55% of the frame), `Buffer::reset` re-blanks every written cell (~15%)
  and border edges are most of the render. The compare runs at about
  4.5 ns per cell. Most of that is the 64-byte public `Cell` layout and the
  derived `Style` equality: in a microbenchmark one field load per cell
  costs ~1.2 ns, the symbol compare ~1.5 ns and the style compare ~2.5 ns.
  `flush/static_200x60` (the same compare over 12,000 cells) is ~51 µs
  against the #486 target of 40 µs. Skipping the compare for unchanged
  rows needs either a per-row content hash (collisions would leave stale
  cells on screen) or comparing against the last frame at write time
  (the render pass only sees the back buffer); neither is done.
- **Table and list**: cell writes and the flush compare of the rows that
  are drawn. Layout (`build_tree`, flexbox, `collect_all`) is a few
  percent.

- **Why ratatui is not a `[dev-dependencies]` entry**: ratatui 0.29 pulled
  the advisory `lru 0.12` (RUSTSEC-2026-0002), which would fail the release
  `cargo audit` gate. The comparison lives in `bench/ratatui-compare`,
  excluded from the workspace with its own `Cargo.lock`, so ratatui's
  dependency tree never reaches `cargo audit`, `cargo deny` or
  `cargo hack` runs on `superlighttui`.

### Binary size

Two programs, each written for both libraries and built as its own crate by
`bash bench/ratatui-compare/size.sh` (re-record these tables on every
release):

- **hello**: the #491 hello-world (`slt::run` + `ui.text` + `ui.key('q')`;
  ratatui: `ratatui::run` + `render_widget(&str)` + `event::read`).
- **dashboard**: header, separator, a 200-row list and a 100x3 table with
  selection, a gauge that follows the list, Tab between list and table,
  Up/Down/j/k, `q` to quit. The ratatui version handles the same keys with
  its own `ListState` / `TableState` and a focus enum; SLT's widgets do that
  themselves and also take mouse clicks.

Each is built under two profiles, applied equally to both libraries: the
default `[profile.release]` (no LTO, `opt-level = 3`, 16 codegen units,
`panic = "unwind"`) followed by `strip`, and the **size profile** below.
Apple M3 Pro, aarch64 macOS, rustc 1.98.0, ratatui 0.30.2 with default
features. Stripped bytes:

| Program | Library | release, 0.26 base `1a6569a` | release, 0.26 | size profile, base | size profile, 0.26 |
|---|---|---|---|---|---|
| hello | SLT default (`crossterm`, `bidi`) | 1,232,400 | **1,149,200** | 704,960 | **688,304** |
| hello | SLT `crossterm` only | 1,164,696 | **1,097,864** | 671,784 | **655,144** |
| hello | ratatui | 553,384 | 553,384 | 421,568 | 421,568 |
| dashboard | SLT default | 1,367,208 | **1,250,904** | 754,784 | **738,144** |
| dashboard | SLT `crossterm` only | 1,299,488 | **1,216,128** | 721,632 | **704,992** |
| dashboard | ratatui | 759,976 | 759,976 | 504,824 | 504,824 |

| SLT default / ratatui | release, base | release, 0.26 | size profile, base | size profile, 0.26 |
|---|---|---|---|---|
| hello | 2.22x | **2.07x** | 1.67x | **1.63x** |
| dashboard | 1.79x | **1.64x** | 1.49x | **1.46x** |

The hello-world measures SLT's fixed cost, the runtime that every `slt::run`
program links (see "Why SLT stays larger" below). Widgets add less in SLT
than in ratatui: going from hello to the dashboard adds ~102 KB to SLT and
~207 KB to ratatui in the release profile, so the ratio shrinks as an app
grows. On arm64 macOS `__TEXT` is padded to 16 KiB pages, so stripped sizes
move in 16 KiB steps; a change of a few KB of code can show as 0 or 16 KiB.

#### Getting a small binary

The size profile is an application setting; a library cannot set it for its
users. Put it in the application's `Cargo.toml`, either as the release
profile or as a separate one (`cargo build --profile size`):

```toml
[profile.size]
inherits = "release"
opt-level = "z"     # optimize for size
lto = "fat"         # whole-program dead-code and duplicate removal
codegen-units = 1   # one unit, so generic and inline copies are shared
panic = "abort"     # no unwind tables or landing pads
strip = true        # drop symbols
```

On the hello-world it removes 40% (1,149,200 → 688,304 B), on the dashboard
41% (1,250,904 → 738,144 B). Trade-offs: `opt-level = "z"` is slower code (measure your own frame
times; `opt-level = "s"` is a middle ground), fat LTO and one codegen unit
make release builds slower, and `panic = "abort"` turns every panic into
process exit: SLT's panic hook still restores the terminal first (it is
abort-aware), but `ui.error_boundary` can no longer recover a panicking
subtree. Disabling the `bidi` default feature
(`default-features = false, features = ["crossterm"]`) saves another
~33-51 KB, and SLT then does not reorder right-to-left text.

**Composition** (default hello after this change; `cargo bloat --release
--crates` and per symbol, `size -m` for sections; cargo-bloat attribution is
approximate):

| Part | SLT default | SLT `crossterm` only | ratatui |
|---|---|---|---|
| `.text` total | 754 KiB | 723 KiB | 320 KiB |
| — `slt` crate / `ratatui_core` + `crossterm` | 406 KiB | 384 KiB | 14 + 31 KiB |
| — `std` (backtrace symbolization ~142 KiB in both, drop glue, fmt) | 301 KiB | 301 KiB | ~241 KiB |
| — `unicode-bidi` (plus ~19 KiB of tables) | 11 KiB | — | — |
| `__eh_frame` + `__gcc_except_tab` (unwind tables) | 105 + 25 KiB | | 47 + 10 KiB |
| `__const` read-only data (Unicode tables, vtables) | 78 KiB | | 50 KiB |

Largest SLT symbols (default hello): `run_frame_kernel` 18.6 KiB,
`accessible::Session::run` 10.3 KiB, `Context::new` 8.6 KiB,
`event::reply::Replay::decode` 6.3 KiB, `width::unicode_str_width` (the one
copy of both width walks) 6.0 KiB, `terminal::flush::flush_buffer_diff_rows`
5.9 KiB, `layout::tree::build_children` 5.8 KiB, `layout::render::render_inner`
5.7 KiB. By area: layout 91 KiB (tree 29, render 22, flexbox 15, focus 12),
accessible mode 36 KiB, `context::core` 31 KiB, buffer 27 KiB, frame kernel
23 KiB, terminal replies and probes (`event::reply`, `terminal::query`,
`terminal::capabilities`) 35 KiB, SLT's own input parser (`event::native`)
20 KiB, flush 13 KiB.
`serde`, `image`, tree-sitter and the syntax tables are absent with default
features, and since this change so are the Kitty, Sixel and iTerm2 flush
passes unless the program draws an image.

**Why SLT stays larger.** ratatui links only what the program calls: a
`&str` render pulls 14 KiB of `ratatui_core`, and the app's own loop does
the rest. `slt::run` is the whole runtime, and most of it is reached through
run-time switches, not calls the app makes: input parsing for every
protocol SLT negotiates, the terminal probes and their reply parser, layout
and focus, the accessible output mode (`SLT_ACCESSIBLE`), the F12 debug
overlay and Ctrl+F12 inspector, mouse selection, Ctrl+Z suspend, the panic
hook, title / progress and clipboard delivery. A static linker keeps
everything that is reachable, so these stay in every binary; removing them
would remove default behaviour. What #491 changed is code that was
duplicated, or reachable only through an API the program may never call.

**Reductions in #491, round 2** (stripped bytes, default hello, release
profile, each step on top of the previous one; examples are built from this
repository, `cargo build --release --example ...`):

| Change | hello | dashboard | `examples/demo` | `examples/widgets_tour` |
|---|---|---|---|---|
| Base `1a6569a` | 1,232,400 | 1,367,208 | 2,354,688 | 1,585,224 |
| One out-of-line `Graphemes::next` (`crate::width::UnicodeSegmentation`) | −49,568 | | | |
| Image passes installed by the first placement | −17,184 | | | |
| One out-of-line `unicode-width` walk (`str_width`, `unicode_width`) | −16,448 | | | |
| **Total** | **−83,200 (−6.8%)** | **−116,304 (−8.5%)** | **−182,280 (−7.7%)** | **−149,336 (−9.4%)** |

- `unicode_segmentation::Graphemes::next` and the `unicode-width` string
  walk are `#[inline]` upstream, so every codegen unit that segmented or
  measured text carried its own copy (five and four in the hello-world,
  many more in apps that use widgets) plus inlined copies inside callers.
  Crate-local wrappers keep one out-of-line copy each. Calls were already
  out of line at the per-grapheme sites; `str_width` measures printable
  ASCII inline (one column per byte, as the tables give) before calling
  the walk, so the round-1 objection (ASCII cells paying for a call) no
  longer applies. Tests compare both wrappers with the upstream crates.
- `Terminal::flush` and `InlineTerminal::flush` reach the Kitty upload pass
  and the Sixel / iTerm2 damage tracker only through a table that
  `Buffer::kitty_place` / `Buffer::sprixel_place` install. Before the first
  placement no buffer holds one and each pass is a no-op on empty lists
  (a test checks this on a terminal that forces every protocol), so the
  output bytes are unchanged. Programs that draw an image link the passes
  as before.
- Tried and reverted: making `accessible::Session` non-generic over the app
  closure. It adds 108 B to single-closure apps and only helps apps that
  call several `run*` entry points.

Frame time (`bench/ratatui-compare`, `time slt`, six interleaved runs of
the base and the round-2 binary, median of medians): every workload is
between −2.9% and +0.5% of the base (dashboard sink 10.2 → 10.2 µs, list
23.0 → 22.8, table 56.4 → 55.5, nested 159.7 → 156.6, idle 10.3 → 10.1).

**Target (set from the round-2 profile): met.** The profile showed about
45 KiB of standalone duplicate or image-only functions in the default
hello-world (five `Graphemes::next` copies, four width walks, the image
passes and their hash tables), plus copies inlined into callers that
cargo-bloat does not separate. Target: at least 5% off the 0.26 base on the same toolchain
for both the hello-world (≤ 1,170,780 B) and the dashboard
(≤ 1,298,848 B) in the default release profile, with no feature or API
removed and no workload slower by more than 2%. Result: hello −6.8%,
dashboard −8.5%, frame time unchanged. The round-1 absolute number
(1,140,000 B) is still 9,200 B (one 16 KiB page) away; the base grew by
68 KB between the rounds with the v0.26 frame-path work.

**Round 1** (on `0a8cf43`, same toolchain):

| Change | hello (issue) | `examples/hello` | `examples/demo` | `examples/widgets_tour` |
|---|---|---|---|---|
| Baseline `0a8cf43` | 1,198,888 | 1,215,608 | 2,698,200 | 1,715,256 |
| `ContainerBuilder::finish`, `push_container`, `line_wrap`, `button_colored`: generic shim + non-generic body | +80 | −16,464 | **−593,392** | **−230,752** |
| `run_frame_kernel` takes `&mut dyn FnMut` (one copy, not one per run mode) | −17,584 | −17,600 | −17,552 | −17,536 |
| Debug bar formats ms / fps with integers (no float formatting) | −17,032 | −17,032 | −16,552 | −48 |
| **Total** | **−34,536 (−2.9%)** | **−51,096 (−4.2%)** | **−627,496 (−23.3%)** | **−248,336 (−14.5%)** |

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

**Follow-ups** (measured, not landed):

- Tooltips (~4-5 KiB in a hello-world) are emitted by the frame kernel from
  a list only `.tooltip()` fills; the same install-on-use table would drop
  them from apps without tooltips, but the hooks live in `context/core.rs`
  and `context/runtime.rs`, which change often.
- Title / progress / status emission (~6.5 KiB) and the width probe
  (~2 KiB) are driven by public `RunConfig` fields (`title`, `width_probe`),
  so a field assignment must keep working without a registration call;
  they stay linked.

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
- `docs/internals/architecture.md` — render pipeline overview
- `docs/reference/debugging.md` — F12 overlay usage and layout-debug walkthrough
- `docs/guide/03-patterns.md` — component patterns including `use_memo`
- `CHANGELOG.md` — issue numbers cited above (#67, #135, #143, #155, #169, …)
