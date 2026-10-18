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
  mirrors `Terminal::flush` (src/terminal.rs): BSU, the written-span
  row skip plus the run-coalesced diff via
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
bash size.sh                                                 # stripped hello / dashboard sizes, SLT vs ratatui (#491)
```

`size.sh` builds two programs for each library as separate crates under
`target/size` and prints markdown tables for docs/internals/performance.md
("Binary size"). Re-record them on every release.

- **hello**: the #491 hello-world (`slt::run` + `ui.text` + `ui.key('q')`;
  ratatui: `ratatui::run` + `render_widget(&str)` + `event::read`).
- **dashboard**: header, separator, a 200-row list and a 100x3 table, both
  with selection, a gauge that follows the list, Tab to move focus,
  Up/Down/j/k to move the selection and `q` to quit. The ratatui version
  handles the same keys itself with `ListState`, `TableState` and a focus
  enum (SLT's widgets do it on their own, and also take mouse clicks).

Each program is built for SLT with default features, SLT with only
`crossterm`, and ratatui 0.30.2, under two profiles applied equally: the
default `[profile.release]` (then `strip`) and a `size` profile
(`opt-level = "z"`, `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`,
`strip = true`). `SLT_HELLO_MAX_BYTES=<n>` makes it exit 1 when the stripped
default SLT hello (release profile) is larger; gate only on large deltas,
since toolchain and linker updates move the numbers by a few KB and macOS
arm64 pads `__TEXT` to 16 KiB pages, so a few KB of code can move the file
by 0 or 16 KiB.

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
  ceilings (zero for `ui.list` and both tables, ~10% above the baseline for
  the workloads whose remaining allocations are their own `format!` / `&str`
  text; counts are per-thread and identical in debug and release);
- SLT first-frame bytes (baseline + ~5%) and steady bytes per frame (exactly the
  baseline: 16 = BSU + ESU when idle, 36 for one changed cell);
- an idle frame emits exactly BSU + ESU.

Ceilings are upper bounds, so improvements pass. When a fix for #483-#491 lands,
lower the matching ceiling in the same PR; raising one must be an intentional,
reviewed diff. Not wired into CI by this change. To run it in CI:

```sh
cd bench/ratatui-compare && cargo test --locked
```

## Baseline (v0.26)

Recorded 2026-10-18 on `wp/perf-final` (release/v0.26.0 plus the command,
string-pool, text-path and border-run commits), against the v0.26 base
`df6690e` and the v0.25.0 baseline recorded earlier with this crate.

- host: Apple M3 Pro (aarch64-macos), shared machine, load average ~5
- rustc: rustc 1.98.0 (88d9e12ae 2026-08-18)
- profile: release (`debug = 1`)
- superlighttui: default features
- ratatui: 0.30.2
- bench features: none (time), count-alloc (alloc)

### Time, sink harness (median of three interleaved `time` runs, µs per frame)

| Workload | SLT v0.25.0 | SLT base `df6690e` | SLT v0.26 | ratatui 0.30.2 | SLT / ratatui |
|---|---|---|---|---|---|
| a. static dashboard 200x60 | 213.6 | 13.2 | **10.3** | 110.2 | 0.09x |
| b. 10k `ui.list`, 100x50 | 1834.0 | 72.3 | **23.0** | 622.2 | 0.04x |
| b'. 10k `ui.virtual_list`, 100x50 | 262.8 | 73.8 | **25.6** | — | — |
| c. table 1000x6, 200x60 | 1037.9 | 166.4 | **55.6** | 998.2 ¹ | 0.06x |
| c'. table 1000x6, `page_size = 57` | 636.4 | 163.8 | **55.0** | — | — |
| d. 60 nested bordered boxes, 240x80 | 1104.8 | 175.4 | **159.0** | 344.2 | 0.46x |
| e. one cell changes per frame, 200x60 | 218.1 | 13.2 | **10.2** | 110.6 | 0.09x |
| f. idle (unchanged), 200x60 | 213.2 | 13.2 | **10.3** | 110.3 | 0.09x |

¹ ratatui draws the table with the same rules as SLT (11 cells per row, see
Equivalence), so it is slower than the prototype's 6 `Fill` columns.

SLT is faster than ratatui on every workload: ~2x on the nested boxes, where
every row of the screen is written each frame, and 10-27x on the others.

Per-case detail (v0.26; first-frame and steady bytes are identical in every
run):

| case | first-frame bytes | steady bytes/frame | median µs | flush mean µs |
|---|---|---|---|---|
| a_dashboard_200x60/slt/sink | 1000 | 16.0 | 10.3 | 1.8 |
| a_dashboard_200x60/ratatui/sink | 1290 | 25.0 | 110.2 | 88.4 |
| b_list10k_100x50/slt/sink | 2301 | 16.0 | 23.0 | 10.0 |
| b_list10k_100x50/ratatui/sink | 3873 | 25.0 | 622.2 | 38.6 |
| b_list10k_100x50(virtual_list)/slt/sink | 2292 | 16.0 | 25.6 | 9.9 |
| c_table1000x6_200x60/slt/sink | 5284 | 16.0 | 55.6 | 21.7 |
| c_table1000x6_200x60/ratatui/sink | 9568 | 25.0 | 998.2 | 95.6 |
| c_table1000x6_200x60(page_size=57)/slt/sink | 5230 | 16.0 | 55.0 | 21.5 |
| d_nested60_240x80/slt/sink | 26422 | 16.0 | 159.0 | 86.9 |
| d_nested60_240x80/ratatui/sink | 25879 | 25.0 | 344.2 | 194.2 |
| e_onecell_200x60/slt/sink | 1000 | 36.0 | 10.2 | 1.8 |
| e_onecell_200x60/ratatui/sink | 1290 | 32.0 | 110.6 | 88.5 |
| f_idle_200x60/slt/sink | 1000 | 16.0 | 10.3 | 1.8 |
| f_idle_200x60/ratatui/sink | 1290 | 25.0 | 110.3 | 88.9 |

### Time, in-memory TestBackend (render only, median of three runs, µs)

| Workload | SLT base `df6690e` | SLT v0.26 | ratatui | SLT / ratatui |
|---|---|---|---|---|
| a. dashboard | 11.5 | 8.6 | 114.2 | 0.08x |
| b. 10k list | 64.8 | 15.0 | 621.0 | 0.02x |
| b'. virtual_list | 66.5 | 17.8 | — | — |
| c. table | 149.5 | 39.0 | 990.6 | 0.04x |
| c'. paged table | 147.2 | 38.4 | — | — |
| d. nested boxes | 108.9 | 101.5 | 350.0 | 0.29x |
| e. one cell | 11.6 | 8.7 | 114.0 | 0.08x |
| f. idle | 11.6 | 8.7 | 114.2 | 0.08x |

### Allocations (`alloc`, 50 warm-up frames, mean over 200 frames)

| Workload | SLT v0.25.0 | SLT base `df6690e` | SLT v0.26 allocs / bytes | ratatui allocs / bytes |
|---|---|---|---|---|
| a. dashboard | 54 / 27 664 | 24 / 984 | **21 / 172** | 79 / 8 445 |
| b. 10k list | 20 018 / 13 798 852 | 51 / 2 054 | **0 / 0** | 20 056 / 4 160 429 |
| b'. virtual_list | 160 / 56 166 | 102 / 2 262 | **98 / 2 064** | — |
| c. table | 9 038 / 1 148 024 | 61 / 4 958 | **0 / 0** | 25 104 / 5 948 401 |
| c'. paged table | 551 / 71 013 | 63 / 4 892 | **0 / 0** | — |
| d. nested boxes | 285 / 168 254 | 136 / 11 902 | **60 / 350** | 273 / 45 965 |
| e. one cell | 56 / 27 674 | 24 / 984 | **21 / 172** | 79 / 8 445 |
| f. idle | 54 / 27 664 | 24 / 984 | **21 / 172** | 79 / 8 445 |

Sink harness; the SLT counts are identical in the mem harness (ratatui's mem
arm makes one allocation and 5 bytes less per frame). What SLT still allocates is the workloads' own text: the
dashboard's 20 `format!` rows plus its `"Header"` `&str` (which `ui.text`
turns into a `String`), two `format!` growths per visible row in the
virtual-list closure, and one `String` per nested-box label.

### Output bytes (`bytes`)

Steady frames: SLT 16 bytes (BSU + ESU) when nothing changed versus ratatui 25
(SGR resets plus hide-cursor). One changed cell (e, frame 3):

- SLT, 36 bytes: `ESC[?2026h ESC[3;8H ESC[0m ESC[37m 3 ESC[0m ESC[?2026l`
- ratatui, 32 bytes: `ESC[3;8H 3 ESC[39m ESC[49m ESC[59m ESC[0m ESC[?25l`

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

At 2eb49b6 the regression was still fully present: all four numbers matched
15fd63a within 3%. It is gone in v0.26 (same host and toolchain, criterion
`--warm-up-time 1 --measurement-time 3`):

| Bench | 0.26 base `df6690e` | v0.26 | vs 3eb59a9 |
|---|---|---|---|
| `buffer_set_string_200x50` | 10.13 | 8.22 | 0.10x |
| `full_render_dims/120x40` | 13.16 | 7.88 | 0.20x |
| `dashboard_200x60/slt` | 15.13 | 8.78 | 0.11x |
| `flush/full_redraw_200x60` | 172.84 | 171.12 | 0.53x |

## Gaps against #482

Out of scope for this change, still open on #482:

- The SLT arm mirrors `Terminal::flush`; driving the production code needs a
  persistent `#[doc(hidden)]` sink `Terminal` in `superlighttui` (the current
  `PtyBackend` builds a new `Terminal` per render, so every frame is a full
  redraw).
- Prototype modes not ported: `live`, `sizes`, `micro`, `loop`. The
  binary-size probe (hello and dashboard, two profiles) is `size.sh` (#491).
- No CI job (gate on PRs, timing artifact on main), and
  docs/internals/performance.md is updated by hand from this output, not
  generated.
