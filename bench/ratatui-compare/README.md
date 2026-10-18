# slt-ratatui-compare

Matched-harness comparison of SuperLightTUI against ratatui (issue #482), plus a
deterministic allocation, output-byte and text-equivalence regression test.

Not published (`publish = false`). The crate is excluded from the main
workspace (`workspace.exclude` in the root `Cargo.toml`) and has its own
`Cargo.lock`, so ratatui's dependency tree (crossterm 0.29, compact_str 0.9,
lru 0.18, ...) never reaches `cargo audit`, `cargo deny check`
or `cargo hack` runs on `superlighttui`. ratatui is pinned to `=0.30.2`;
bump it deliberately and re-record the baseline below.

## Harness

Both arms render the same visible text into an in-process ANSI sink, one frame
per call, with persistent front/back buffers so steady-state diffing is
measured, not full redraws.

- **SLT**: `slt::frame_owned` driving `SltSink`, a `Backend` whose `flush`
  mirrors `Terminal::flush` (src/terminal.rs): BSU, row-hash refresh of both
  buffers plus the run-coalesced diff via
  `__bench_flush_buffer_diff_mut_with_buf` (the production
  `flush_buffer_diff` kernel), ESU, swap, then `reset_with_bg(theme.bg)`
  exactly as `run_with` configures it. Kitty, sprixel, raw-sequence and cursor
  passes are omitted; they emit nothing for these workloads. This is still a
  mirror of the flush sequence, not the production `Terminal`; see Gaps.
- **ratatui**: `Terminal<CrosstermBackend<Vec<u8>>>` with `Viewport::Fixed`. A
  frame is `Terminal::try_draw` (whose `autoresize` does nothing for a fixed
  viewport): `get_frame`, render, then the public `apply_buffer` (diff, hide
  cursor, swap, backend flush). The two halves are only called separately so the
  flush phase can be timed. All production code.
- **mem** cases render into each library's in-memory `TestBackend` with no ANSI
  flush, so the render kernel can be separated from the flush cost.

Timing: one untimed first frame, warm-up of 200 frames or 0.3 s (whichever is
longer), then samples until 1.5 s or 20 000 samples (at least 300). Reported:
median, p95, mean, mean flush time, first-frame and steady bytes per frame.

## Workloads

| | Workload | Size | Notes |
|---|---|---|---|
| a | static dashboard: header, separator, 20 rows, progress bar | 200x60 | |
| b | 10k-row `ui.list` / ratatui `List` | 100x50 | row 25 selected |
| b' | 10k rows through `ui.virtual_list` | 100x50 | SLT only, compared with ratatui b |
| c | 1000x6 `ui.table` / ratatui `Table` | 200x60 | row 25 selected |
| c' | 1000x6 `ui.table` with `page_size = 57` | 200x60 | SLT only, compared with ratatui c |
| d | 3 x 4 x 5 = 60 nested rounded boxes with labels | 240x80 | |
| e | dashboard where one cell changes per frame | 200x60 | |
| f | idle dashboard, nothing changes | 200x60 | |

### Equivalence

`show` and the `rendered_text_matches_ratatui_per_workload` test compare the
visible text (symbols per row, trailing blanks trimmed) of every SLT workload
with its ratatui arm, and also check that the timed sink harness draws the same
text as the in-memory one. Styles and colors are not compared.

The audit prototype's ratatui arm did **not** draw the same text for a, c and
d. The ratatui arm was adjusted to SLT's default output (never the other way,
so the SLT arm measures what SLT users get):

- **a/e/f progress bar**: SLT draws `█` x round(0.75 x 20) then `░`. ratatui's
  `Gauge` draws a centered label hole and `LineGauge` always leaves a gap after
  the label, so ratatui renders the same 20-glyph styled line as a `Paragraph`
  (which is also how SLT builds it).
- **c table**: SLT sizes columns to content and draws ` │ ` column rules and a
  `─┼─` header rule. ratatui gets content-sized `Length` columns computed once
  at setup, three-cell rule columns and a two-line header. This makes ratatui
  draw 11 cells per row instead of 6 with `Fill` columns: its c time went from
  ~591 µs (prototype) to ~1015 µs, so the c ratio is not comparable with the
  issue table.
- **d nested boxes**: `Fill(1)` spreads the rounding remainder; SLT gives it to
  the trailing items. `Length` constraints reproduce SLT's split (same solver
  path, cached the same way).

Documented exceptions (rows allowed to differ), both SLT-only variants:

- b' row 49: `↓ 9951 more` overflow hint drawn by `virtual_list`.
- c' row 59: `Page 1/18` pager drawn by the paged table.

All other rows of all workloads are identical.

## Commands

Run from `bench/ratatui-compare/`.

```sh
cargo build --release
cargo run --release -- time [filter]                         # µs/frame, bytes/frame, flush share, summary
cargo run --release --features count-alloc -- alloc [filter] # allocations + requested bytes per steady frame
cargo run --release -- bytes [filter]                        # first/steady bytes, one-cell and idle frame dump
cargo run --release -- show [filter] [--dump]                # text equivalence (exit 1 on mismatch)
cargo test                                                   # deterministic gate (debug and release agree)
bash size.sh                                                 # stripped hello-world size, SLT vs ratatui (#491)
```

`size.sh` builds the #491 hello-world programs as separate crates under
`target/size` with the default release profile and prints a markdown table
for docs/PERFORMANCE.md ("Binary size"). Re-record it on every release.
`SLT_HELLO_MAX_BYTES=<n>` makes it exit 1 when the stripped default SLT
binary is larger; gate only on large deltas, since toolchain and linker
updates move the numbers by a few KB.

`filter` is a substring of the case name (`<workload>/<slt|ratatui>/<mem|sink>`),
for example `sink`, `d_nested` or `slt/sink`. Every report starts with host,
rustc, profile, superlighttui SHA and feature set.

Hygiene for this crate (it is outside the workspace, so root runs skip it):

```sh
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
typos .                 # from the repo root, typos also covers this directory
cargo audit             # audits this crate's own Cargo.lock
```

### Regression test (optional CI)

`tests/regression.rs` asserts only deterministic properties, no wall time:

- visible-text equivalence per workload (see above);
- SLT steady-state allocation calls and requested bytes per frame under fixed
  ceilings (roughly 25-50% above the baseline; counts are per-thread and
  identical in debug and release);
- SLT first-frame bytes (baseline + ~5%) and steady bytes per frame (exactly the
  baseline: 16 = BSU + ESU when idle, 44 for one changed cell);
- an idle frame emits exactly BSU + ESU.

Ceilings are upper bounds, so improvements pass. When a fix for #483-#491 lands,
lower the matching ceiling in the same PR; raising one must be an intentional,
reviewed diff. Not wired into CI by this change. To run it in CI:

```sh
cd bench/ratatui-compare && cargo test --locked
```

## Baseline (v0.25.0 + release/v0.26.0 base)

Recorded 2026-10-18 on `release/v0.26.0` at 2eb49b6 (v0.25.0 15fd63a plus a
docs-only commit; no library change). The `-dirty` suffix in the header is the
uncommitted root `workspace.exclude` line of this change.

- host: Apple M3 Pro (aarch64-macos), shared machine, load average 5.8-9.4
- rustc: rustc 1.98.0 (88d9e12ae 2026-08-18)
- profile: release (`debug = 1`)
- superlighttui: 2eb49b6710b9-dirty (v0.25.0-1-g2eb49b6) (default features)
- ratatui: 0.30.2
- bench features: none (time), count-alloc (alloc)

### Time, sink harness (median of three `time` runs, µs per frame)

| Workload | SLT | ratatui 0.30.2 | SLT / ratatui | SLT range over 3 runs | issue #482 SLT / ratatui |
|---|---|---|---|---|---|
| a. static dashboard 200x60 | 213.6 | 112.9 | 1.9x | 205.4-222.6 | 215 / 107 |
| b. 10k `ui.list`, 100x50 | 1834.0 | 643.3 | 2.9x | 1790.0-1848.4 | 1832 / 648 |
| b'. 10k `ui.virtual_list`, 100x50 | 262.8 | — | — | 258.1-267.7 | 258 / — |
| c. table 1000x6, 200x60 | 1037.9 | 1014.7 | 1.0x | 1011.9-1078.0 | 1032 / 591 ¹ |
| c'. table 1000x6, `page_size = 57` | 636.4 | — | — | 623.8-637.5 | 643 / — |
| d. 60 nested bordered boxes, 240x80 | 1104.8 | 352.9 | 3.1x | 1080.9-1105.6 | 1255 / 358 |
| e. one cell changes per frame, 200x60 | 218.1 | 112.6 | 1.9x | 210.8-219.3 | 221 / 108 |
| f. idle (unchanged), 200x60 | 213.2 | 112.8 | 1.9x | 205.3-213.9 | 219 / 108 |

¹ The prototype's ratatui table drew 6 `Fill` columns without rules; the
equivalent table draws 11 cells per row (see Equivalence), so only the SLT
number is comparable.

Every SLT median is within ±15% of the issue table (d is −12%). ratatui a/e/f
are ~5% slower than in the issue because the progress bar is now a styled
`Paragraph` line instead of a `Gauge`.

Per-case detail (median over the three runs; first-frame and steady bytes are
identical in every run):

| case | first-frame bytes | steady bytes/frame | median µs | range of medians | p95 µs | flush mean µs |
|---|---|---|---|---|---|---|
| a_dashboard_200x60/slt/sink | 1008 | 16.0 | 213.6 | 205.4-222.6 | 221.7 | 170.7 |
| a_dashboard_200x60/ratatui/sink | 1290 | 25.0 | 112.9 | 110.1-114.6 | 122.7 | 91.1 |
| b_list10k_100x50/slt/sink | 2309 | 16.0 | 1834.0 | 1790.0-1848.4 | 1949.8 | 71.2 |
| b_list10k_100x50/ratatui/sink | 3873 | 25.0 | 643.3 | 632.2-644.5 | 675.2 | 39.9 |
| b_list10k_100x50(virtual_list)/slt/sink | 2300 | 16.0 | 262.8 | 258.1-267.7 | 274.0 | 70.2 |
| c_table1000x6_200x60/slt/sink | 5292 | 16.0 | 1037.9 | 1011.9-1078.0 | 1070.7 | 175.2 |
| c_table1000x6_200x60/ratatui/sink | 9568 | 25.0 | 1014.7 | 996.3-1026.2 | 1080.9 | 98.3 |
| c_table1000x6_200x60(page_size=57)/slt/sink | 5238 | 16.0 | 636.4 | 623.8-637.5 | 657.2 | 174.3 |
| d_nested60_240x80/slt/sink | 26430 | 16.0 | 1104.8 | 1080.9-1105.6 | 1129.3 | 306.4 |
| d_nested60_240x80/ratatui/sink | 25879 | 25.0 | 352.9 | 350.3-355.6 | 373.8 | 200.4 |
| e_onecell_200x60/slt/sink | 1008 | 44.0 | 218.1 | 210.8-219.3 | 227.4 | 175.3 |
| e_onecell_200x60/ratatui/sink | 1290 | 32.0 | 112.6 | 112.6-112.8 | 122.1 | 90.2 |
| f_idle_200x60/slt/sink | 1008 | 16.0 | 213.2 | 205.3-213.9 | 222.1 | 170.3 |
| f_idle_200x60/ratatui/sink | 1290 | 25.0 | 112.8 | 110.2-112.9 | 122.5 | 91.2 |

Flush is 75-80% of an SLT dashboard frame (~171 of ~214 µs) even when nothing
changed and only 16 bytes go out, versus ~91 of ~113 µs for ratatui.

### Time, in-memory TestBackend (render only, median of three runs, µs)

| Workload | SLT | ratatui | SLT / ratatui |
|---|---|---|---|
| a. dashboard | 120.2 | 112.9 | 1.1x |
| b. 10k list | 1798.5 | 650.0 | 2.8x |
| b'. virtual_list | 224.8 | — | — |
| c. table | 935.3 | 1024.2 | 0.9x |
| c'. paged table | 543.9 | — | — |
| d. nested boxes | 929.0 | 356.9 | 2.6x |
| e. one cell | 120.4 | 112.6 | 1.1x |
| f. idle | 121.2 | 112.8 | 1.1x |

### Allocations (`alloc`, 50 warm-up frames, mean over 200 frames)

| Workload | SLT allocs/frame | SLT bytes/frame | ratatui allocs/frame | ratatui bytes/frame |
|---|---|---|---|---|
| a. dashboard | 54 | 27 664 | 79 | 8 445 |
| b. 10k list | 20 018 | 13 798 852 | 20 056 | 4 160 429 |
| b'. virtual_list | 160 | 56 166 | — | — |
| c. table | 9 038 | 1 148 024 | 25 104 | 5 948 401 |
| c'. paged table | 551 | 71 013 | — | — |
| d. nested boxes | 285 | 168 254 | 273 | 45 965 |
| e. one cell | 56 | 27 674 | 79 | 8 445 |
| f. idle | 54 | 27 664 | 79 | 8 445 |

Sink harness; the mem harness differs by at most two allocations and 10 bytes per frame.

### Output bytes (`bytes`)

Steady frames: SLT 16 bytes (BSU + ESU) when nothing changed versus ratatui 25
(SGR resets plus hide-cursor). One changed cell (e, frame 3):

- SLT, 44 bytes: `ESC[?2026h ESC[3;8H ESC[0m ESC[0m ESC[37m 3 ESC[0m ESC[0m ESC[?2026l`
- ratatui, 32 bytes: `ESC[3;8H 3 ESC[39m ESC[49m ESC[59m ESC[0m ESC[?25l`

The doubled `ESC[0m` around the run is #489.

## Frame-kernel regression evidence (#483)

The audit bisect (issue #483, criterion medians, µs) placed almost the whole
regression in 493677a, the v0.23.0 squash commit:

| Bench | 3eb59a9 (v0.22.3) | 493677a (v0.23.0) | c05c856 (v0.24.0) | 15fd63a (v0.25.0) |
|---|---|---|---|---|
| `buffer_set_string_200x50` | 80.5 | 284.0 | 285.6 | 283.8 |
| `full_render_dims/120x40` | 38.5 | 63.6 | 65.2 | 65.4 |
| `dashboard_200x60/slt` | 83.3 | 116.1 | 120.7 | 117.4 |
| `flush/full_redraw_200x60` | 324.1 | 507.9 | 524.5 | 513.2 |

Re-run on this branch (2eb49b6, same host and toolchain), from the repository
root:

```sh
cargo bench --bench benchmarks -- 'buffer_set_string_200x50|full_render_dims/120x40|flush/full_redraw_200x60|dashboard_200x60/slt'
```

| Bench | criterion estimate [low, high] | vs 3eb59a9 |
|---|---|---|
| `buffer_set_string_200x50` | 288.37 [286.43, 290.82] ¹ | 3.6x |
| `full_render_dims/120x40` | 63.90 [63.71, 64.12] | 1.7x |
| `dashboard_200x60/slt` | 116.83 [116.37, 117.35] ¹ | 1.4x |
| `flush/full_redraw_200x60` | 512.44 [511.41, 513.59] | 1.6x |

¹ Second run; the first was noisy under load (`buffer_set_string_200x50`
335.73 [301.18, 376.93], `dashboard_200x60/slt` 134.05 [123.80, 145.49]).

The regression is still fully present on the v0.26.0 base: all four numbers
match 15fd63a within 3%. The kernel hotspots are tracked in #484-#486 and #493; this crate
and its ceilings are the acceptance harness for them.

## Gaps against #482

Out of scope for this change, still open on #482:

- The SLT arm mirrors `Terminal::flush`; driving the production code needs a
  persistent `#[doc(hidden)]` sink `Terminal` in `superlighttui` (the current
  `PtyBackend` builds a new `Terminal` per render, so every frame is a full
  redraw).
- Prototype modes not ported: `live`, `sizes`, `micro`, `loop`. The
  hello-world binary-size probe is `size.sh` (#491).
- No CI job (gate on PRs, timing artifact on main) and docs/PERFORMANCE.md is
  not regenerated from this output yet.
