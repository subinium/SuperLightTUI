# Changelog

## [0.21.0] - Unreleased

### Changed (Breaking)

- **`feat(widgets-display)`! — `scrollbar()`, `separator()`, `separator_colored()` return real `Response`** (#241) — Resolves `Known v0.21 migration note #2` / `docs/ARCHITECTURE.md` M4 (`no () returns`). `separator()` / `separator_colored()` previously returned `&mut Self` from a legacy text-chain pattern, but the chained `.bold()` / `.fg()` mutators were a no-op (the cached separator string is already finalized), so the misleading chain is dropped in favor of a real `Response` routed through `Context::interaction()`. `scrollbar()` already returned `Response::none()` in v0.20 (#184); it is now routed through `Context::interaction()` so the track hit-test rect is populated, and its receiver changes from `&ScrollState` to `&mut ScrollState` to reserve the click-to-jump / drag-to-scroll extension point (#249) without another breaking change. **Migration:** statement-form `separator` callers (`ui.separator();`) compile unchanged; any `.separator().bold()`-style chains must drop the no-op suffix (none existed in-crate). `scrollbar` callers pass `&mut scroll` instead of `&scroll`.

### Added

- **`feat(test-utils)` — `PtyBackend` + `PtyFrame` end-to-end escape-byte harness** (#274) — Behind the dev-only `pty-test` feature (off by default), `PtyBackend` drives the *real* `Terminal` flush pipeline into an in-process `Vec<u8>` sink, so the actual escape/image-protocol bytes that ship to a terminal — SGR runs, OSC 8 hyperlinks, Sixel (`\x1bPq`), Kitty graphics (`\x1b_Ga=`), and color-depth-downsampled SGR — are asserted as whole frames. Assertions: `assert_emits`, `assert_not_emits`, `last_raw`, `frames_raw`, `with_color_depth`. This is the byte/protocol regression tier that the buffer-only `TestBackend` and the plain-text `insta` snapshots in `tests/visual_snapshots.rs` deliberately cannot reach. No real TTY required — reproducible on `ubuntu-latest`.

### CI

- **`ci(gallery)` — VHS gallery gate + tape coverage parity** (#274) — New `gallery` job renders every `.tape` in the repo through VHS and asserts each produces its declared `Output` asset at a non-trivial byte size, uploading the generated assets as a workflow artifact (no auto-commit). A `.tape` now exists for every gallery asset advertised in `README.md` (added `demo`, `demo_dashboard`, `demo_website`, `demo_game`, `demo_pretext`), and `tests/gallery_manifest.rs` (plain `cargo test`, no VHS) fails if README ↔ tape parity drifts. Replaces the manual "tmux verification for visual demos before release" release-checklist step.

## [0.20.1] - 2026-04-29

### Deprecated

- **`deprecated(layout)` — `Context::col_gap` and `Context::row_gap`** — The two-arg shorthand collides with `ContainerBuilder::col_gap` / `ContainerBuilder::row_gap`, which set the *row-finalize* / *column-finalize* main-axis gap (Tailwind `gap-x` / `gap-y` axis convention) and so mean the opposite thing despite the same name. Use `ui.container().gap(n).col(f)` / `.row(f)` instead — same output, no name collision. AI-generated code that hits the old form continues to compile with a deprecation warning until the planned v0.21+ removal.

### Docs

- **`docs(skills)` — `.claude/skills/slt/{SKILL,REFERENCES}.md` and `.claude/skills/slt-migration/SKILL.md` synced to v0.20** — Mental-model section trimmed, 5 API rules and 5 Layer model formalized, v0.20 removed-API table inlined (`gauge_w`, `gauge_colored`, `line_gauge_with`, `breadcrumb_sep`, `LineGaugeOpts`, `HighlightRange::single`, `label_owned`), Korean trigger phrases added, hook-ordering decision table for `use_state` / `use_state_named` / `use_state_keyed`, per-domain reference-example index. Migration skill version banner bumped from v0.19.2 → v0.20.0.
- **`docs(skills)` — Rule 6 (return-type pattern) added to `SKILL.md`** — `Context` methods return `&mut Self` for chainable display mutators (`text`, `link`, `styled`, style chain) or `Response` for interaction results (every stateful interactive widget plus `col` / `row` / `modal`); `line` / `line_wrap` / `screen` continue an inline-text chain so they return `&mut Self`. The split is the most common AI-generated compile error — chaining `.bold()` on `ui.button(...)` etc. — and now has a single discoverable rule.
- **`docs(skills)` — Custom widget pattern decision guide** — Function form (`fn render_card(ui, &data)`) covers the 90% case; `impl Widget` is for caller-owned state types and third-party crates exporting widgets through trait-bound APIs. Closes the "when do I pick which" gap that previously left readers to guess from the trait example alone.
- **`docs(rustdoc)` — `# Example` blocks added to the status family** — `badge`, `badge_colored`, `key_hint`, `stat`, `stat_colored`, `stat_trend`, `empty_state`, `empty_state_action` (`src/context/widgets_display/status.rs`) each carry a runnable `no_run` example. Closes the patch-safe doc-only audit gap flagged in v0.20.0.
- **`docs(rustdoc)` — `# Example` block added to `vsplit_pane`** — `src/context/widgets_display/split.rs:66` — paired with the existing `split_pane` example so both orientations are equally discoverable.
- **`docs(readme)` — Removed version-specific "v0.20 Demo Catalog"** — The 20-row table of `v020_*` demos with issue numbers belongs in `docs/EXAMPLES.md` (already linked from the same page) rather than the top-level README, which should stay timeless across minor releases. Replaced with a tighter "Demo Launcher" subsection pointing readers at the per-release catalog. Same applies to the inline `# All v0.20 demos at once` comment in the launcher snippet — generalized to "full feature-tour spread".

### Refactor

- **`refactor(examples)` / `refactor(tests)` — 25 demo and integration files migrated off `Context::col_gap` / `Context::row_gap`** to the explicit `ui.container().gap(n).col(f)` / `.row(f)` form. 22 example files (counter, demo, demo_design_system, cookbook_*, system_tour, v020_tour, v020_use_state_keyed, v020_modal_trap, v020_theme_subtree, v020_spacing_scale, anim, etc.) and 3 integration tests (`v020_interaction_regression`, `v020_theme_modal_demos`, `v020_widthspec_demo`). Output is byte-identical (same finalize path, same gap value) — the change is purely about removing the deprecated form so AI training data and downstream copy-paste land on the unambiguous shape from the start.

### Fixed

- **`fix(tests)` — `tests/v020_perf_alloc.rs` cross-test allocator contamination** — Every `#[test]` in the file now grabs the file-wide `measure_lock` mutex via the new `enter_perf_test()` helper on its first line. The pre-fix `measure_allocs` lock protected only the measurement critical section, so non-measuring sibling tests still ran concurrently and their `String::from(...)` / `Vec::new()` calls leaked into the global `ALLOC_COUNT`. Pattern manifested as flaky `framestate_reuse_steady_state` / `kitty_placement_flush` / `use_state_keyed_*` budget breaches whose noise scaled with macOS thread-cache timing. Root cause now fixed in source — the `--test-threads=1` workaround in `.github/workflows/ci.yml` (line 41-46) was removed in the same change.
- **`fix(buffer)` — `dead_code` warning under `--no-default-features`** — `Buffer::recompute_line_hashes`, `Buffer::row_clean`, `Buffer::row_hash` (`src/buffer.rs:475-540`) are gated on `#[cfg(any(feature = "crossterm", test))]`. The methods exist solely to support the per-row hash fast-path inside `flush_buffer_diff` (added in #171), which is itself behind the `crossterm` feature. Without the gate they showed as `dead_code` whenever the crate was built with `--no-default-features`, tripping `cargo check -p superlighttui --no-default-features` on a clean tree. No public-API change — methods stay `pub(crate)`.

### CI

- **`ci(test)` — Removed `--test-threads=1` from the Test job** — `.github/workflows/ci.yml` now runs `cargo test --all-features` with the default parallel runner. Justified by the `tests/v020_perf_alloc.rs` source fix above; CI run time drops by ~10 s on the typical SLT testsuite size.

## [0.20.0] - 2026-04-28

### Added

- **`feat(test-utils)` — `TestBackend::record_frames()` + `FrameRecord` history** (#229) — Opt-in frame recorder. Every `render()` call appends a `FrameRecord { snapshot, lines }` accessible via `tb.frames()`. `FrameRecord` exposes `assert_contains`, `to_string_trimmed`, and per-row text. Disabled by default → zero allocation overhead for tests that don't need history.
- **`feat(test-utils)` — `TestBackend::sequence()` builder + `type_string` helper** (#230) — Multi-step interaction sequences without manual `focus_index` / `prev_focus_count` threading. Methods: `.tick()`, `.key(KeyCode)`, `.type_string(&str)`, `.events(Vec<Event>)`, `.run()`. Backend-level `tb.type_string("hi", render)` fires one frame per character.
- **`feat(buffer)` — `Buffer::snapshot_format()`** (#231) — Stable styled-snapshot string for `insta::assert_snapshot!` compatibility. Named palette colors → short codes (`red`, `light_blue`); RGB → `#rrggbb`; indexed → `idx<N>`; canonical modifier order (`bold,dim,italic,underline,reversed,strikethrough`). Format guaranteed stable across patch and minor versions; locked by `tests/snapshot_format_stability.rs`.
- **`feat(test-utils)` — `assert_not_contains` / `assert_line_not_contains` / `assert_empty_line` / `assert_style_at`** (#232) — Negative assertion helpers on `TestBackend`. Failures show offending row indices and full row contents; `assert_style_at` reports `(x, y, expected, actual)` on style mismatch.
- **`feat(context)` — `Response::right_clicked` / `gained_focus` / `lost_focus`** (#208) — three new public bool fields on every widget `Response`. `right_clicked` mirrors the existing `clicked` logic for `MouseButton::Right`. `gained_focus` is `true` exactly on the frame focus moves to the widget; `lost_focus` is `true` exactly on the frame focus moves away. Mutually exclusive within a single Response. Hooked into `begin_widget_interaction` so all widgets that use the standard interaction path (button / table / select / radio / checkbox / toggle / tree etc.) populate the signals automatically.
- **`feat(hooks)` — `Context::use_state_keyed` / `use_state_keyed_default`** (#215) — runtime-string-keyed persistent state. Accepts `impl Into<String>`, so `format!("item-{i}")` works in dynamic loops where `use_state_named` (which requires `&'static str`) does not. Stored in a parallel `keyed_states: HashMap<String, Box<dyn Any>>` on `Context` / `FrameState`. Mirrors the namespace-collision + per-frame-allocation caveats of `use_state_named`. **See breaking section below for the `State<T>` Copy removal.**
- **`feat(hooks)` — `Context::use_effect`** (#216) — dependency-tracked side effects. `ui.use_effect(|deps| do_thing(deps), &deps)` runs the closure on the first frame and on every frame thereafter where `*deps != stored_deps`. Positional hook (same rules as `use_state` / `use_memo`). Fire-and-forget — no cleanup callback. Doc warns that `use_effect` inside `error_boundary` may re-fire on rollback.
- **`feat(focus)` — `register_focusable_named` + `focus_by_name` + `focused_name`** (#217) — Ink-style named focus manager. `register_focusable_named(name)` is a drop-in replacement for `register_focusable()` that records `name → focus_index`. `focus_by_name(name)` requests focus on the named widget; resolution happens against the previous frame's map (deferred-command pattern). `focused_name()` returns the name of the currently focused widget, if any. Compatible with the existing positional Tab/Shift+Tab cycling.
- **`feat(context)` — `Context::key_presses_when` + `Context::consume_event`** (#218) — public focus-gated key-press iterator and per-event consume helper. `key_presses_when(active)` returns an empty iterator when `active=false` and the same items as the internal `available_key_presses` when `active=true`. `consume_event(idx)` is the public counterpart of the crate-internal `consume_indices`, enabling user-land `Widget` impls to mark events handled. Out-of-range indices silently no-op.
- **`feat(context)` — `Response::on_hover` / `on_hover_ui` chaining** (#209) — Attach a tooltip (or run an arbitrary tooltip-rendering closure) directly on a widget's `Response` without the order-sensitive `ui.tooltip(...)` post-call. Composes cleanly: `if ui.button("Save").on_hover(ui, "Saves the file").clicked { ... }`. Skips alloc when `hovered == false` or `text` is empty.
- **`feat(anim)` — `Context::animate_bool` / `animate_value` shorthand** (#210) — Zero-boilerplate implicit animation keyed by `&'static str`, stored in `named_states`. `animate_bool(id, value) -> f64` returns 0.0..=1.0 over `DEFAULT_ANIMATE_TICKS` (12 ticks ≈ 200 ms @ 60 Hz). `animate_value(id, target, duration) -> f64` retargets smoothly from the current interpolated value; `duration_ticks == 0` snaps. First call snaps to target with no visible pop.
- **`feat(container)` — `ContainerBuilder::fill()` shorthand** (#220) — Self-documenting alias for `.grow(1)` (CSS `flex: 1`, ratatui `Constraint::Fill(1)`). One-liner that improves readability of the most common flex case without changing semantics.
- **`feat(rect)` — `Rect::center_in` / `center_horizontally_in` / `center_vertically_in`** (#221) — Position a sized rect centered inside a parent (the inverse of `Rect::centered`). Matches ratatui v0.30. Clamps to parent extent on oversize. `const fn` — usable in static contexts.
- **`feat(modal)` — `ModalOptions` + `Context::modal_with`** (#225) — opt-in WCAG 2.1 SC 2.4.3 (Focus Order) compliance. `ModalOptions { tab_trap: true }` prevents focus escape when programmatic `set_focus_index` or a stray click lands focus outside the modal range. `ModalOptions::default()` enables `tab_trap`. Plain `Context::modal(...)` keeps the legacy non-trapping behavior unchanged for backward compatibility — i.e. `ui.modal(f)` and `ui.modal_with(ModalOptions::default(), f)` deliberately produce different focus semantics. Use `modal_with(ModalOptions::default(), f)` (or `ModalOptions { tab_trap: true, ..Default::default() }`) when you want the WCAG-aligned trap; keep `modal(f)` for the v0.19 escape-friendly behavior.
- **`feat(theme)` — `ContainerBuilder::theme(theme)`** (#226) — per-subtree theme override. Swaps `ctx.theme` (and `dark_mode` flag) for the duration of the closure body, restoring on exit — including on panic. Nested `.theme(...)` calls compose correctly: outer theme resumes once the inner scope closes. Independent of `provide` / `use_context` (general-purpose context injection); this method directly mutates the active theme so every built-in widget (which reads `self.theme`) picks up the change without opt-in.
- **`feat(theme)` — `Theme::compact()` / `Theme::comfortable()` / `Theme::spacious()` density presets** (#227) — base spacings 1 / 2 / 3 respectively. Matches the existing `Spacing` scale (`xs` / `sm` / `md` / `lg` / `xl` / `xxl`). `compact()` is bit-identical to existing presets, preserving v0.19 visuals when adopted explicitly.
- **`feat(theme)` — `Theme::with_spacing(spacing)`** (#227) — mutate spacing on any preset (Nord, Dracula, custom) without touching colors.
- **`feat(widgets-display)` — `split_pane` / `vsplit_pane`** (#223) — resizable horizontal/vertical split containers driven by `SplitPaneState`. Drag the 1-cell handle (`│` / `─`) with the mouse, or focus it (Tab) and use arrow keys to grow/shrink the first pane by 5% per press. `SplitPaneResponse` exposes `ratio` and `drag_active`.
- **`feat(widgets-display)` — `gauge` (chainable builder)** (#224, builder finalized in v0.20.0 API consistency pass) — block-fill progress bar with optional centered inline label (e.g. `█████████ 60% ░░░░░░`). Chainable: `ui.gauge(ratio).label(s).width(n).color(c)`. Color-tiered by default (`success` < 50%, `warning` 50–80%, `error` ≥ 80%); `.color(c)` disables tiering. Auto-renders on `Drop`; call `.show()` for a `GaugeResponse` (derefs to `Response`). `ratio` is `f64` for parity with `animate_value`, chart APIs, and `progress_bar`.
- **`feat(widgets-display)` — `line_gauge` (chainable builder)** (#224, builder finalized in v0.20.0 API consistency pass) — single-line gauge with configurable fill/empty characters and an optional trailing label. Chainable: `ui.line_gauge(ratio).label(s).width(n).filled(c).empty(c)`. Auto-renders on `Drop`; call `.show()` for a `GaugeResponse`.
- **`feat(widgets-display)` — `scrollable_with_gutter` + `GutterOpts<G>`** (#235, signature finalized in v0.20.0 API consistency pass) — scrollable container variant rendering a per-line left gutter (line numbers, breakpoint markers, etc.) plus search-result highlight bands. The bookkeeping arguments collapse into `GutterOpts<G>`; use `GutterOpts::line_numbers(total, viewport)` for the 90% case or `GutterOpts::new(total, viewport, |i| ...)` for custom labels. Companion API on `ScrollState`: `set_highlights`, `highlight_next`, `highlight_previous`, `clear_highlights`, `current_highlight`. `HighlightRange` is re-exported at the crate root; use `HighlightRange::line(i)` for single-line and `HighlightRange::span(start, count)` for multi-line ranges. Returns `GutterResponse` carrying the current highlight index and total count.
- **`feat(lib)` — `Context::static_log(line)`** (#233) — append-only scrollback widget API. Inside the frame closure, queue lines that get committed to the terminal's history above the inline dynamic area. Drains automatically through `slt::run_static` / `slt::run_static_with`; no-op (with a `cfg(debug_assertions)` warning) on full-screen and inline runtimes that have no scrollback channel. Inspired by Ink's `<Static>`. Companion accessor `Context::take_static_log()` exposes the same buffer to custom backends and `TestBackend` callers.
- **`feat(keymap)` — `WidgetKeyHelp` trait + `Context::publish_keymap` / `published_keymaps` / `keymap_help_overlay`** (#236) — opt-in trait for widgets to publish their `&'static [(key, description)]` shortcut list. The framework aggregates every keymap registered this frame (cleared at frame start by `run_frame_kernel`) and `keymap_help_overlay(open)` renders an automatic modal listing all bindings — wire it to `?` / `F1` for instant discoverability. Standalone `PublishedKeymap` struct exposed for downstream widgets / palettes.
- **`feat(lib)` — `RunConfig::handle_ctrl_c(bool)`** (#238) — opt-out for the auto-Ctrl+C-quits behavior. Defaults to `true` (preserves v0.19 contract). Setting `false` delivers Ctrl+C to the frame closure as a normal `Event::Key` with `KeyModifiers::CONTROL` — matches RataTUI's raw-mode semantics so users migrating code that already handles Ctrl+C explicitly do not need to fork SLT. Threaded through `run_with`, `run_inline_with`, `run_static_with`, and `run_async_loop`. `run()` rustdoc updated to note that wrapping with `crossterm::terminal::enable_raw_mode()` / `disable_raw_mode()` is redundant — SLT enters raw mode automatically.

### Changed

- **`change(theme)` Built-in widgets now derive padding/gap from `theme.spacing`** (#227) — code_block, code_block_numbered, accordion, tooltip, help, help_colored, tabs, checkbox, toggle, select trigger, calendar header, text_input, suggestion box, command palette, markdown code blocks. Default theme spacing is unchanged (`Spacing::new(1)`), so every preset produces v0.19-identical output by default. To get larger paddings, use `Theme::comfortable()`/`Theme::spacious()` or set `theme.spacing` explicitly via `ThemeBuilder::spacing(...)`.
- **`change(examples)` Cargo example list compacted from 53 → 16** — `Cargo.toml` sets `autoexamples = false` and lists 16 explicit `[[example]]` entries: 6 tour binaries (`v020_tour`, `cookbook_tour`, `showcase_tour`, `canvas_tour`, `text_tour`, `system_tour`), 5 standalone demos (`hello`, `counter`, `demo`, plus 2 perf tools), 3 dev tools, and 2 v0.20 reports. Source files for the demos that compose into tours stay in `examples/` and are reached via `#[path = ...] mod` includes from the tour binaries — see `docs/DEMO_GUIDE.md` for the archetype rules.

### Fixed

- **`fix(focus)` — `register_focusable_named` now allocates a slot eagerly and reserves it for the next `register_focusable()` call** (#217 follow-up) — In v0.20.0-preview the call queued a name and waited for a following widget to drain it, which made `register_focusable_named("x")` a silent no-op when called standalone (the `name → slot` map never picked up the binding). The new behaviour allocates the slot on the named call itself and stores the slot id as a one-shot reservation: the very next `register_focusable()` reuses it instead of allocating a fresh slot, so widgets like `text_input`, `button`, and `tabs` placed immediately after still inherit the name. Both shapes work — "name + widget" (the common idiom) and "name alone" (custom focusable regions, unit tests). Verified by 24 tests in `tests/v020_hooks_focus.rs`.
- **`fix(chart)` — Legend names and treemap labels are clipped with an ellipsis (`…`) instead of bare-truncated** — `crate::chart::truncate_label(text, max_cols)` is the new shared helper. Returns the original text when it fits, an ellipsis-suffixed prefix when it does not, and an empty string when `max_cols < 3` (drops the label entirely rather than emit a 1- or 2-cell garbled prefix). Used by `chart::render` for legend names (after the legend column budget is computed against y-axis width and a `MIN_PLOT_COLS = 4` reservation) and by `treemap` for cell labels. Pre-fix output showed `Memor` / `TypeS`; post-fix shows `Memo…` / `Type…` or drops the label.
- **`fix(examples)` — Tab clicks in `cargo run --example demo` now persist** — `examples/demo.rs` lifted all per-frame `let mut state = ...` into a `pub struct DemoState` owned by `main()`. Previously every render re-created `TabsState`, `TextInputState`, etc., which made tab clicks visibly flash for ≈ 0.1 s before snapping back to the first tab. The same `pub fn render(ui, &mut state)` + `pub fn render_snapshot(ui)` split applied to `v020_keymap_help`, `v020_gutter_highlights`, `v020_ctrl_c_passthrough`, and `v020_dx_shortcuts` so they keep state across frames when embedded in `v020_tour` (per `docs/DEMO_GUIDE.md` §2).
- **`fix(widgets-interactive)` — `virtual_list` keeps cursor mid-viewport instead of always anchored to the bottom** (#192) — Added `pub(crate) viewport_offset: usize` to `ListState` (`Default = 0`, additive). The `virtual_list` viewport now sticks to the cursor on entry/exit only, mirroring every other TUI library. Pre-fix moving up dragged the entire viewport because `start = selected - vh + 1`. Test: `virtual_list_cursor_not_anchored_to_viewport_bottom`.
- **`fix(widgets-interactive)` — `calendar` `h`/`l` now move ±1 day; `[` / `]` move ±1 month** (#193) — Vim convention restored. Pre-fix `h`/`l` were aliased to `prev_month` / `next_month`, which contradicted the universal vim "single-cell move" mental model. `Left`/`Right` arrows still move ±1 day too. Calendar rustdoc carries a keybinding table; `WidgetKeyHelp` updated so the `?` overlay reflects the new bindings. Test: `calendar_h_l_move_by_day`.
- **`refactor(widgets-input)` — `FilePickerState::selected_file()` disambiguates from `selected: usize`** (#98) — Added `pub fn selected_file(&self) -> Option<&PathBuf>`; the existing `selected()` method is `#[deprecated(since = "0.20.0")]` and delegates to `selected_file()`. The duplicate identifier (`pub selected: usize` field vs `pub fn selected() -> Option<&PathBuf>` method) was confusing readers. Migration: replace `state.selected()` with `state.selected_file()` to clear the deprecation warning.
- **`feat(widgets-input)` — Textarea undo/redo (`Ctrl+Z` / `Ctrl+Y`)** (#102) — Pure additive. `TextareaState` now holds a `history: Vec<TextareaSnapshot>` (default cap 100, configurable via `history_max(cap)` builder) plus `history_index: usize`. Snapshots are pushed before destructive mutations (insert, Enter, Backspace, Delete, paste). Rapid character typing coalesces into a single undoable batch (one snapshot per "edit burst" rather than one per keystroke). 5 new tests in `tests/textarea_undo.rs`. Programmatic `set_value()` clears history (replacement is not undoable).
- **`refactor(container)` — `ContainerBuilder::scroll_offset` hidden from rustdoc** (#149) — Added `#[doc(hidden)]` (Option B from the issue) so the implementation-detail builder method stops appearing in the public docs. `cargo-semver-checks` still tracks the symbol so promote to `pub(crate)` happens at v1.0. No behavior change.
- **`feat(widgets-display)` — `scrollbar()` returns `Response`** (#184) — Reserves a future-compatible extension point for click-to-jump and drag handling without another breaking change. `Response::none()` for now. All in-crate call sites continue to compile (statement form, response ignored).
- **`perf(layout)` — drain `commands` Vec in `build_tree` to reuse capacity across frames** (#150) — `build_tree(commands: Vec<Command>)` → `build_tree(&mut Vec<Command>)` using `drain(..)`. `run_frame_kernel` reclaims via `mem::take`. Steady-state allocation count for the per-frame command buffer drops -15% (1300 → 1100 over 100 frames in serial mode).
- **`perf(layout)` — reuse `line_segs` scratch in `wrap_segments`** (#157) — Hoisted the per-line `Vec<Segment>` out of the outer loop; `mem::replace` returns an empty buffer with the previous capacity. Output byte-identical (existing `wrap_segments_*` tests + alloc-count budget hold). Internal-only, no public API change.
- **`feat(flexbox)` — Opt-in proportional `.shrink()` flag for overflow handling** (#161) — `ContainerBuilder::shrink(self) -> Self` marks a child eligible for CSS-style proportional shrink when `fixed_width > available`. Default behaviour (no shrink, overflow-by-design) is preserved. Implementation uses a `Command::ShrinkMarker` indirection (mirrors `FocusMarker` / `InteractionMarker`) so `Constraints` size invariant (`_ASSERT_CONSTRAINTS_SIZE == 24`) is preserved. 3 new layout tests cover (a) default-off, (b) all-shrink, (c) mixed.
- **`perf(terminal)` — Per-row hash skip in `flush_buffer_diff`** (#171) — Added `Buffer::line_hashes` + `line_dirty` (`pub(crate)`); rows with unchanged hash skip cell iteration. Bench `bench_flush_static_200x60`: **113–138 µs → 127 ns (~1000× speedup)** on fully-static screens. Sparse and full-redraw paths unchanged. Uses `std::collections::hash_map::DefaultHasher` — no new dep.
- **`feat(layout)` — `DebugLayer` enum for F12 overlay opt-in** (#201) — Added `Shift+F12` keybinding that cycles `All → TopMost → BaseOnly → All`. Plain `F12` still toggles the overlay on/off. Per-variant rustdoc + `# Example` blocks on `DebugLayer`, `Context::debug_layer`, `Context::set_debug_layer`. The enum + getter/setter base shipped earlier in v0.20; this adds the missing keybinding and rustdoc.

### Closed (verified already applied in v0.19.3)

The following v0.19.x-milestoned issues had their fix land in PR #202 (v0.19.3 `0a56880`) ahead of this release. The fix is part of v0.20.0 transitively; no separate v0.20 commit was needed.

- **#134** `screen_hook_map` cache-hit `String` alloc removed (verified at `src/context/widgets_display/layout.rs:226-232`).
- **#146** `filled_circle` integer Newton's-method `isqrt`. (`u64::isqrt` would be cleaner but is gated behind MSRV 1.84+; SLT MSRV is 1.81. Tracked as a `TODO(msrv)` for v0.21+.)
- **#147** Breakpoint variants for `min_h` / `max_h`.
- **#148** `#[deprecated]` aliases for `pad` / `min_width` / `max_width` / `min_height` / `max_height` (canonical short forms `p` / `min_w` / `max_w` / `min_h` / `max_h`).
- **#152** `LayoutNode::group_name` widened to `Option<Arc<str>>` (collect-time pointer-bump, not heap alloc).
- **#153** `LayoutNode` text-only fields hoisted into `Box<TextNodeData>` — measured `size_of::<LayoutNode>()` 432 → 304 bytes (-29.6 %). `_ASSERT_LAYOUT_NODE_SIZE` const-asserts the upper bound.
- **#155** `FrameData` re-use via `&mut` parameter (`collect_all(node, data)` + `mem::take` reclaim in `lib::run_frame_kernel`).
- **#162** Viewport bound check before bottom-border corner render in `render_container_border`.

### Known v0.21 migration notes

A design-discipline audit of v0.20.0 surfaced four critical drifts that require a *breaking* change to fully resolve. They are intentionally deferred to v0.21.0 so v0.20.0 stays a single-bump migration:

1. **Hook family naming asymmetry** — `use_state_named` (no init closure, requires `Default`) vs `use_state_named_with` (init closure) flips the suffix relative to `use_state_keyed` (init closure) vs `use_state_keyed_default` (no init, requires `Default`). v0.21 will pick the "no-suffix = init closure" convention (matches `use_state(init)`) and add `#[deprecated]` aliases.
2. **`scrollbar()` and `separator()` return shape** — `scrollbar` already returns `Response::none()` in v0.20 (#184); `separator` / `separator_colored` still return `&mut Self` from a legacy text-chain pattern that doesn't carry chainable methods worth chaining. v0.21 promotes them to `Response` so the `M4 — no () returns` rule from `docs/ARCHITECTURE.md` is met everywhere.
3. **`status` family fake `Response::none()` returns** — `badge` / `key_hint` / `stat` / `stat_colored` / `stat_trend` / `empty_state` return `Response::none()` for shape compatibility but never populate interaction fields. v0.21 will route them through `self.interaction()` so `.on_hover` / `.hovered` / `.gained_focus` work.
4. **`ScrollState::progress() -> f32`** — outlier in the v0.20 ratio surface (every other ratio is `f64`: gauge, line_gauge, split_pane, animate_value, progress_bar). v0.21 adds `progress_ratio() -> f64` and `#[deprecated]`s the `f32` form.

The audit also identified several *patch-safe* doc-only gaps (missing `# Example` / `# Panics` sections on hook methods, status family widgets, `vsplit_pane`) that will land as a follow-up in `0.20.x` rather than block this release.

### Performance

- **`perf(context)` reuse 6 per-frame `Vec`/`HashSet` allocations via `FrameState`** (#204) — `context_stack`, `deferred_draws`, `rollback.group_stack`, `rollback.text_color_stack`, `pending_tooltips`, `hovered_groups` now follow the `mem::take` pattern established by #150 / #155. Steady-state allocation count for a 100-frame loop drops from a baseline that scaled with these six fields to a tight per-frame budget (verified by `tests/v020_perf_alloc.rs::framestate_reuse_steady_state_alloc_count_low`).
- **`perf(layout)` pre-size `wrap_segments` per-style-run `String` with `with_capacity`** (#205) — eliminates the realloc on the first character pushed at every style boundary in `wrap_segments`. Capacity is computed from the remaining bytes in the source segment, capped at `max_width * 4` to bound over-allocation. Output is byte-identical to the prior implementation (`tests/v020_perf_demo.rs::wrap_segments_with_capacity_preserves_byte_output`).
- **`perf(terminal)` avoid `Vec<KittyPlacement>` clone in `InlineTerminal::flush`** (#206) — `KittyImageManager::flush` now accepts a `row_offset: u32` and applies it arithmetically at point of use. The `prev_placements` diff stores post-offset y values so resize-driven offset changes still re-emit. Eliminates one `Vec` allocation + N `Arc::clone`/`Arc::drop` round-trips per inline-mode frame with images (`tests/v020_perf_alloc.rs::kitty_placement_flush_alloc_count_low` confirms 1 alloc across 100 stable flushes).
- **`perf(render)` modal-aware `dim_buffer_around` replaces full-buffer scan** (#228) — for the common modal-with-margin case, `render` now calls `dim_buffer_around(modal_rect)` which walks only the four strips outside the modal instead of the full O(W×H) buffer. The legacy `dim_entire_buffer` path is retained for the zero-size-modal fallback. Visible output is unchanged (`tests/v020_perf_demo.rs::modal_dim_path_preserves_modal_content`).

### Breaking

- **API consistency pass on new widgets** — `gauge` / `line_gauge` / `breadcrumb` and `scrollable_with_gutter` were unified onto a single design language so v0.20 does not ship five new widgets with five argument styles. The five mismatched signatures became three Egui-style chainable builders + one options struct. The new builders are the only public form; no deprecated shim wrappers ship in v0.20. v0.19 preview users who manually constructed the unstable `gauge_w` / `gauge_colored` / `line_gauge_with(LineGaugeOpts)` / `breadcrumb_sep` signatures should migrate to the builders:

  ```rust
  // before (v0.19.x preview)
  ui.gauge(0.42, "CPU");                                       // 2 positional
  ui.gauge_w(0.42, "CPU", 48);                                 // 3 positional
  ui.gauge_colored(0.42, "CPU", 48, Color::Red);               // 4 positional
  ui.line_gauge(0.42, LineGaugeOpts::default().width(48).label("CPU"));
  ui.breadcrumb_sep(&["Home", "src"], " > ");
  ui.scrollable_with_gutter(
      &mut scroll, total, viewport, |i| line_label(i), |ui, i| { ... }
  );

  // after (v0.20.0)
  ui.gauge(0.42).label("CPU");
  ui.gauge(0.42).label("CPU").width(48);
  ui.gauge(0.42).label("CPU").width(48).color(Color::Red);
  ui.line_gauge(0.42).label("CPU").width(48);
  ui.breadcrumb(&["Home", "src"]).separator(" > ");
  ui.scrollable_with_gutter(
      &mut scroll,
      GutterOpts::line_numbers(total, viewport),    // 90% case shortcut
      |ui, i| { ... },
  );
  ```

  The new builders auto-render on `Drop` so a bare `ui.gauge(0.5).label("CPU");` is the idiomatic form when the response isn't needed; call `.show()` to capture a `GaugeResponse` / `BreadcrumbResponse`.

- **`f32 → f64` unification on ratio APIs (`gauge` / `line_gauge` / `split_pane`)** — every public `f32` ratio in v0.20 is widened to `f64` to align with `Context::animate_value` (`f64`), chart APIs (`f64`), and `progress_bar` (`f64`). Touched APIs: `Context::gauge` / `line_gauge` argument, `GaugeResponse.ratio`, `SplitPaneState::{ratio, min_ratio, new, with_min_ratio, set_ratio}`, `SplitPaneResponse.ratio`, `DEFAULT_SPLIT_MIN_RATIO`. Most callers need no change because Rust auto-coerces float literals (`SplitPaneState::new(0.5)` works either way). Explicit `f32` casts (`ratio as f32`) at the call site must be removed; use `f64` arithmetic throughout, or cast `as f64` once at the boundary. The `f32` ratio inside `style/color.rs` blending math is intentional graphics-internal precision per `docs/API_DESIGN.md` Rule 2 exception.

- **`refactor(widgets-display)` — `scrollable_with_gutter` now takes `GutterOpts<G>`** (#235 follow-up) — the four-positional signature `(state, total_lines, viewport_height, gutter_fn, body_fn)` was hard to read and easy to misorder. v0.20 collapses the bookkeeping arguments into a `GutterOpts<G>` struct: `pub fn scrollable_with_gutter(&mut self, state, opts: GutterOpts<G>, body_fn)`. The 90% case (1-based line numbers) gets a `GutterOpts::line_numbers(total, viewport)` shortcut so most callers never write the labeling closure. Use `GutterOpts::new(total, viewport, gutter_fn)` for custom labels (breakpoints, Git diff markers, fold indicators, etc.).

- **`refactor(style)` — `Constraints` redesign with `WidthSpec`/`HeightSpec`** (#237 — closes #207, #219). The `Constraints` struct now holds two enum-typed fields, `width: WidthSpec` and `height: HeightSpec`, instead of the v0.19 trio of `Option<u32>` / `Option<u8>` fields per axis. Builder methods (`min_w`, `max_w`, `min_h`, `max_h`, `w`, `h`, `w_pct`, `h_pct`) are preserved as ergonomic shortcuts that set the appropriate variant. New: `w_ratio(num, den)` / `h_ratio(num, den)` for exact integer-fraction sizing — `Ratio(1, 3)` produces `area / 3` (floor division; for `area = 80, num = 1, den = 3` → `26`). `size_of::<Constraints>()` drops from 36 → 24 bytes (33 % reduction); `WidthSpec` and `HeightSpec` are 12 bytes each. The `MinMax` variant uses sentinel encoding (`min = 0` means "no minimum", `max = u32::MAX` means "no maximum") so the variant fits in 12 bytes.

  Migration:

  ```rust
  // before (v0.19)
  Constraints {
      min_width: Some(10),
      max_width: Some(40),
      ..Default::default()
  }
  // after (v0.20)
  Constraints::default().w_minmax(10, 40)
  // or piecewise:
  Constraints::default().min_w(10).max_w(40)
  ```

  Direct field reads (`c.min_width`, `c.max_width`, `c.width_pct`, …) become accessor calls (`c.min_width()`, `c.max_width()`, `c.width_pct()`, …) — they still return `Option<u32>` / `Option<u8>` so the receiving code typically only needs to add parentheses. For imperative mutation (rare; previously `c.min_width = Some(v)`), use the new setter methods (`set_min_width`, `set_max_width`, `set_width_pct`, …).

  `serde` wire format changes: persisted `Constraints` JSON from v0.19 will not deserialize into v0.20 because the field shape is different. Re-export persisted data after upgrading.
- **`State<T>` is no longer `Copy`** (#215) — required to support the `Keyed(String)` variant of the internal `StateKey` enum. `Clone` is still derived (cheap for `Indexed` / `Named`, allocates one `String` for `Keyed`). **Migration**: if you previously relied on implicit copy semantics — for example `let s = ui.use_state(...); use_a(s); use_b(s);` — call `s.clone()` explicitly: `use_a(s.clone()); use_b(s);`. An audit of every `State<T>` use site in `src/`, `tests/`, `examples/` showed **zero** sites depended on `Copy`; existing call sites borrow or move the handle and continue to compile unchanged.
- **`break(theme)` Spacing scale activation may shift visuals** (#227) — if you customized themes with non-default spacing (e.g., `Spacing::new(2)`), affected widgets now respect that scale. Migration: set `Theme::spacing` explicitly via `ThemeBuilder::spacing(...)` or use `Theme::with_spacing(...)` to lock down the visual you depend on. The 10 stock presets still ship `Spacing::new(1)`, so upgraders who never touched the spacing field see no change.
- **`refactor(widgets-display)` — `breadcrumb` is now a chainable builder (#213, builder finalized in v0.20.0 API consistency pass)** — replaced the four-variant API (`breadcrumb`, `breadcrumb_with`, `breadcrumb_response`, `breadcrumb_response_with`) with a chainable builder. `ui.breadcrumb(segments)` returns a `Breadcrumb<'_>` that auto-renders on `Drop`; chain `.separator(s)` or `.color(c)` and call `.show()` to capture a `BreadcrumbResponse` (derefs to `Response`, so `.hovered`, `.rect`, `.focused` work on `r`).

  Migration:
  ```rust
  // before (v0.19):
  let clicked = ui.breadcrumb(&segments);            // Option<usize>
  let (resp, clicked) = ui.breadcrumb_response(&segments);
  let clicked = ui.breadcrumb_with(&segments, " > ");

  // after (v0.20):
  ui.breadcrumb(&segments);                                 // simple
  let r = ui.breadcrumb(&segments).show();                  // capture response
  let clicked = r.clicked_segment;
  let r = ui.breadcrumb(&segments).separator(" > ").show(); // custom separator
  ```

- **`refactor(widgets-input)` — `spinner` / `progress` / `progress_bar` / `progress_bar_colored` now return `Response` (#212)** — these widgets previously returned `&mut Self` (a builder-chain shim). They now return `Response` so callers can detect hover, attach tooltips, or implement click-to-set scrubbers. `toast` continues to return `&mut Self` (no meaningful single rect — purely visual overlay).

  Migration: code that ignored the return value still compiles; the `#[must_use]` attribute on `Response` will warn at the call site. The recommended fix is `let _ = ui.progress(0.5);`. Code that chained builder-style methods (e.g. `ui.spinner(&s).fg(theme.primary)`) must split into two statements:
  ```rust
  // before:
  ui.spinner(&s).fg(theme.primary);
  // after:
  let _ = ui.spinner(&s); // color is already theme.primary; if you need a different color, render manually.
  ```

## [0.19.3] — 2026-04-27

Patch release covering 11 v0.19.x patch-safe issues plus 6 cross-cutting
extensions framing SLT in terms of broader UI library patterns (CSS / Flutter /
React Native positioning, performance budget, migration guidance, visual
snapshot regression infrastructure).

### Added

- **`feat(layout)` — `Anchor` enum + `overlay_at` / `modal_at`** (#200) — 9-cell positioning (`TopLeft`, `TopCenter`, …, `BottomRight`). Maps to CSS `place-self`, Flutter `Align(alignment:)`, React Native `position: absolute`.
- **`feat(layout)` — `overlay_at_offset(anchor, dx, dy, …)` / `modal_at_offset(…)`** — CSS `inset`-style offset on top of 9-cell anchor. Sign convention: positive `(dx, dy)` always inset toward viewport center. Mapping documented in `docs/POSITIONING.md`.
- **`feat(layout)` — `DebugLayer::{All, TopMost, BaseOnly}`** (#201) + `Context::set_debug_layer` / `debug_layer()` — F12 overlay scoped to a single layer. `All` is the default → no API call needed for the reported case.
- **`feat(api)` — `min_h` / `max_h` breakpoint variants** (#147) — `xs_min_h` / `sm_min_h` / `md_min_h` / `lg_min_h` / `xl_min_h` / `min_h_at` and same for `max_h`. Symmetric with `min_w` / `max_w` breakpoint coverage.
- **`feat(skill)` — `.claude/skills/slt-migration/SKILL.md`** (398 lines) — Migration skill mapping `ratatui` / `cursive` / `textual` → SLT with grep-verified API references.
- **`feat(test)` — `tests/visual_snapshots.rs` + 5 baselines** — Visual snapshot regression infrastructure using `insta`. Catches layout drift, border render bugs, theme color shifts, CJK width issues. Baselines: `demo`, `demo_dashboard`, `demo_cjk`, `demo_infoviz`, `demo_overlay_anchor`.
- **`docs(positioning)` — `docs/POSITIONING.md`** (286 lines) — CSS `place-self` / Flutter `Align`+`Positioned` / React Native `position: absolute` ↔ SLT `Anchor` mapping with migration recipes.
- **`docs(performance)` — `docs/PERFORMANCE.md`** (336 lines) — 60 fps frame budget, allocation budget, 6 optimization patterns, comparison vs React / Flutter / UIKit / ratatui, regression detection workflow.
- **`docs(migration)` — `docs/MIGRATION.md`** (234 lines) — v0.19 → v0.20 migration guide with deprecation table + sed-based codemod + comparison vs React / Vue / Angular / Flutter migration tooling.
- **`example` — `examples/demo_overlay_anchor.rs`** — 9 anchor positions + 4 inset corners using `overlay_at_offset`.

### Fixes

- **`fix(layout)` overlay `align(End)/justify(End)` rendered at center** (#200 part 2) — root cause in `src/layout/flexbox.rs`: overlay sizing block hard-coded shrink-and-center, starving any inner `grow`. New `any_grow` heuristic expands the wrapper to full area when a child has `grow > 0`; legacy behavior preserved otherwise.
- **`fix(layout)` `container.grow(1).draw(|buf, rect|)` inside overlay didn't render** (#200 part 3) — same root cause: 0×0 wrapper rect was being skipped. Same fix resolves both bugs together.
- **`fix(layout)` F12 debug overlay skipped `node.overlays`** (#201 part A) — `render_debug_overlay` now walks both `node.children` and `node.overlays`, matching `count_leaf_widgets`. Default-on; no API call needed.

### Perf

- **`perf(container)` integer `isqrt` for `filled_circle`** (#146) — Newton's method replaces `f64::sqrt()` round-trip. MSRV 1.81 blocks `u64::isqrt` (1.84+); migrate when MSRV bumps.
- **`perf(layout)` `LayoutNode` size 432 → 320 bytes (~26 % reduction)** (#153) — 6 text-only fields extracted into `Box<TextNodeData>`. Spacer / Container / RawDraw nodes (the majority) now pay 8 bytes (`Option<Box<…>>`) instead of ~120 bytes of always-`None` fields. `const _ASSERT_LAYOUT_NODE_SIZE` regression guard at `tree.rs:3-15`.
- **`perf(layout)` `commands` Vec capacity reused via `FrameState.commands_buf`** (#150) — eliminates per-frame Vec allocation in `Context::new`.
- **`perf(layout)` `FrameData` Vec capacity reused via `&mut FrameData`** (#155) — `collect_all` signature changed to `(&LayoutNode, &mut FrameData)`. 8 Vec allocations per frame eliminated.
- **`perf(layout)` `wrap_segments` `line_segs` capacity hint** (#157) — `Vec::with_capacity(segments.len().min(16))` reduces early growth churn on text wrap path.
- **`perf(layout)` viewport bound check before bottom border corner** (#162) — gates `set_char` on `bottom_i < viewport_bottom`. No functional change (OOB writes were silently skipped); saves up to 2 `set_char` per scrolled border frame.

### Refactor

- **`refactor(api)` deprecate long-form aliases** (#148) — `pad()` / `min_width()` / `max_width()` / `min_height()` / `max_height()` are now `#[deprecated(since = "0.20.0")]`. Use short forms: `p()` / `min_w()` / `max_w()` / `min_h()` / `max_h()`. Internal callers updated. Migration guide in `docs/MIGRATION.md`.
- **`refactor(debug)` F12 per-layer color tagging** — Base = green family, Overlay = red, Modal = blue. Status bar adds breakdown: `14 widgets (8 base, 5 overlay, 1 modal)`. Inspired by Chrome DevTools / React DevTools / Flutter Inspector layer color conventions.
- **`refactor(layout)` `group_name: Option<Arc<str>>` confirmed in `LayoutNode`** (#152) — already shipped earlier in v0.19.x; collect-side conversion is now a pointer bump (atomic increment) rather than heap alloc.

### Docs

- **`docs(skill)` SLT skill (`.claude/skills/slt/`) v0.19.x sync** — v0.19.0 component DX (`provide` / `use_context` / `use_state_named` / `with_if`), `RichLogState` bounded default, `ThemeBuilder` `const fn`, `EventBuilder` v0.19.1 chain wrappers.
- **`docs(audit)` 17 docs files audited and synced to v0.19.x** — BLOCKING: `AppCtx` `'static` lifetime fix in AI_GUIDE / COOKBOOK examples (owned `Theme` via `*ui.theme()` Copy pattern, mirroring `examples/demo_website.rs`). HIGH: 16 leaked GitHub issue refs scrubbed from prose. MEDIUM: `COMPLETE_REFERENCE` version banner update, `WIDGETS` separator path corrected, `EXAMPLES` `demo_cjk` row added with audit prose cleanup, `llms.txt` `try_get` signature corrected and `demo_website` / `demo_cjk` added to examples list.
- **`docs(testing)` visual snapshot regression workflow** — section in `docs/TESTING.md` covering `cargo test --test visual_snapshots`, `cargo insta review` flow, scope of detection.
- **`docs(debugging)` F12 layer-color reading guide** — section in `docs/DEBUGGING.md` describing color tagging and status-line breakdown.

### Tests

- **7 new regression tests** for #200 (overlay anchor + 2 bug fixes) and #201 (F12 walks overlays, layer color distinction, count breakdown).
- **5 visual snapshot baselines** committed in `tests/snapshots/visual__*.snap`.

### Asset cleanup

- Removed orphan assets (~6.1 MB): `assets/tui-builders-demo.gif`, `assets/demo_tetris.png`, `examples/demo_wiki.rs` (depended on private `assets/blackpink/`, broke external builds).

### Notes

This release closes the v0.19.x perf / refactor backlog (11 patch-safe issues) plus pairs with the external reporter promise on #200 / #201. Breaking-change issues (#98, #134, #149, #161, #184, #192, #193) remain deferred to v0.20.0. #102 (textarea undo / redo) remains blocked on `cargo-semver-checks` private-field rule; #171 (line-hash flush skip) remains blocked on bench gate. 3-pass review (5 author agents → 10 independent reviewers → 5 post-fix reviewers) per group; full Core + Extended Gate green at tag.

## [0.19.2] — 2026-04-27

Patch release covering 34 v0.19.x issues plus two long-standing visual regressions
caught during demo validation.

### Fixes (Visual regressions)

- **Bordered container title overdraws the top horizontal bar with spaces** — regression introduced in v0.19.1 (#160) when the CJK title clamp added a blank-pad pass that overwrote `─` cells with `' '`. ASCII titles like `cargo-slt` lost the surrounding `──` and rendered as `╭─cargo-slt           ╮` instead of `╭─cargo-slt──────╮`. The clamp now relies on `chars.h` (the original horizontal bar character) being already in place; only the clamped-title char is overwritten. CJK truncation correctness (the original purpose of #160) is preserved by the `UnicodeWidthChar` loop.
- **`text_input()` claims all available vertical space in its parent column (regression dating to v0.17)** — `text_input_colored` always called `.grow(1)` on its bordered container, so a single-line input inside `bordered("Packages").p(1).grow(1).col(...)` would expand to fill the column and push siblings off-screen. Removed the implicit grow. Cross-axis stretch via the parent column still gives the input the full row width as before.

### Added

- **`examples/demo_cjk.rs`** — multi-language demo covering CJK title truncation, mixed Korean / Chinese / Japanese body wrap, narrow-clamp title boxes (12-cell width), and CJK form fields. Pins both regressions above and serves as a manual visual gate for future title-clamp / wrap changes.
- **`feat(style)` — `ContainerStyle::mx(value)` / `my(value)`** (#108) — Tailwind-style horizontal/vertical margin shorthands.
- **`feat(style)` — `Modifiers::remove(other)`** (#111) — opposite of `insert()`.
- **`feat(theme)` — `Theme::builder_from(base)` and `Theme::light_builder()`** (#110) — start a `ThemeBuilder` from any `Theme` or from the `light` preset, instead of having to copy/reapply every field.
- **`feat(theme)` — `ThemeBuilder` methods are `const fn`** (#109) — themes can now be defined in a `const` context for compile-time evaluation.
- **`feat(widgets-display)` — `breadcrumb_response()` / `breadcrumb_response_with()`** (#182) — returns `(Response, Option<usize>)` so callers can read both the click index and the underlying interaction state.
- **`feat(widgets-input)` — textarea kill-line (Ctrl+K) and word jump (Ctrl/Alt + ←/→)** (#103) — standard Emacs-style line and word movement on the multi-line input.

### Fixes

- **`fix(viz)` sixel auto-detection false positive on `xterm-256color`** (#116) — substring `"xterm"` previously fired on macOS Terminal, VS Code, and most SSH clients, none of which speak sixel. Replaced with an exact-match list (`mlterm`, `foot`, `yaft`, `xterm-256color-sixel`) plus the `"sixel"` substring catch-all and `SLT_FORCE_SIXEL=1` opt-in for patched builds.
- **`fix(widgets-display)` `separator()` 200-char hardcoded string causes per-frame allocation and overflow** (#177) — `OnceLock`-cached `SEP_LINE` initialized once per process; `set_string` clip truncates the trailing chars on narrow terminals. Was already in place via #160 prep; this release closes the tracking issue.
- **`fix(widgets-feedback)` `RichLogState::new()` defaults to unbounded entry accumulation** (#191) — bounded default of 10000 entries; `RichLogState::unbounded()` is the new opt-in for explicit unbounded callers.
- **`fix(widgets-interactive)` `tabs` mouse `x - rect.x` underflow panic in debug builds** (#196) — `saturating_sub` on the offset.

### Perf

- **`perf(chart)` flat `Vec<T>` for plot buffers** (#117) — `plot_chars` and `plot_styles` are now contiguous `Vec<T>` instead of `Vec<Vec<T>>`. Removes per-row indirection and shrinks the per-frame allocation count.
- **`perf(viz)` `squarify_recursive` no per-iter `Vec::clone`** (#115) — incremental sum / pos-min / pos-max / pos-count tracking via a closed-form `worst_ratio_incremental`. Inner loop is now alloc-free.
- **`perf(viz)` `treemap` sort uses `total_cmp`** (#121) — replaces the `partial_cmp + NaN fallback` pattern, removing one branch per comparison.
- **`perf(viz)` candlestick body renders at half-cell precision** (#123, closes #120) — body cells now emit `█` / `▀` / `▄` based on price-edge half-cell membership instead of full-cell snapping. Doc updated.
- **`perf(viz)` `blend_color` lifted to a module-level helper** (#119) — eliminated three copies of the same closure.
- **`perf(widgets-input)` `SpinnerState` static frame data** (#99) — `frames: Vec<char>` → `&'static [char]`. Removes the per-state heap allocation; constructor is now `const fn`-friendly.
- **`perf(widgets-interactive)` `CommandPaletteState::filtered_indices` cache** (#101) — `fuzzy_score` no longer runs twice per render. Cache is invalidated on `toggle()`.
- **`perf(widgets-interactive)` `parse_inline_segments` byte-index scan** (#176) — markdown inline tokens (`**`, `*`, `` ` ``) are scanned via byte indices into `text.as_bytes()` instead of `chars[i+N..].iter().collect::<String>()`. Multi-byte characters in `inner` are never split.
- **`perf(widgets-interactive)` `recompute_widths` dirty-flag short-circuit** (#195) — table column widths are only recomputed when the items vector or filter actually changed, instead of once per frame.
- **`perf(widgets-interactive)` `collect_grid_elements` extracted** (#194) — 41-line duplicate between `grid()` and `grid_with()` lifted into a shared free function.
- **`perf(hooks)` single `downcast_mut` in `use_memo` cache-hit path** (#133) — eliminates the redundant second `downcast_ref` after key comparison; type mismatch panics with the hook index instead of silently overwriting.
- **`perf(events)` `SmallVec<[usize; 8]>` in `consume_activation_keys`** (#135) — typical button activation queues 0-2 indices; the 8-slot stack buffer keeps these allocation-free.
- **`perf(table)` extract `let visible = state.visible_indices()` local binding in `table_visible_len`** (#140) — avoids re-traversing the filter chain inside the loop.
- **`perf(widgets-display)` `streaming_text` no per-frame content clone** (#178) — caller buffer borrowed via `&str` instead of cloned into `Command::Text`.
- **`perf(widgets-display)` `scrollbar` static cell glyphs** (#179) — `'█'.to_string()` / `'│'.to_string()` per-cell allocations replaced with `const THUMB: &str / const TRACK: &str`.
- **`perf(widgets-display)` `code_block_numbered` gutter via `ilog10`** (#180) — gutter width derived from `lines.len().ilog10()` instead of `format!("{}", lines.len()).len()`.
- **`perf(widgets-display)` `definition_list` manual padding** (#181) — eliminates the per-row `format!()` for right-aligned key padding.
- **`perf(terminal)` `extract_selection_text` `truncate(trim_end().len())`** (#173) — drops the trailing `to_string()` reallocation.

### Refactor

- **`refactor(widgets-display)` `separator()` moved from `widgets_interactive/events.rs` to `widgets_display/layout.rs`** (#183) — same `impl Context` so the public call site is unchanged. Restores topical placement (separator is a display widget, not an event helper).
- **`refactor(viz)` `blend_color` helper extracted** (#119) — see Perf above; flagged as refactor in the issue tracker.

### Docs

- **`docs(buffer)` `Buffer::diff()` documents per-call `Vec` allocation cost and hot-path warning** (#170).
- **`docs(style)` `Align::Start` clarifies CSS-stretch semantics** (#165) — documents the `flex-start` vs CSS-stretch divergence.

### Tests

- **`test(cell)` compile-time `Cell` size assertion** (#167) — `const _: () = assert!(size_of::<Cell>() <= N);` catches accidental field-bloat regressions at build time.

### Closed (already implemented)

- #132, #125, #136, #138, #118, #177, #179 — verified that the working code already matches the proposed solution; closed without further changes.

### Deferred to v0.19.3

- **#102** (textarea undo / redo Ctrl+Z / Ctrl+Y) — landed during the wave but reverted: the implementation requires three new `pub(crate)` fields on `TextareaState`, and `cargo-semver-checks` reports `constructible_struct_adds_private_field` as a major-version break. v0.19.1 followed the same constraint for #94 (deferred to v0.20.0) — this is the same constraint applied here. Will re-apply alongside #94 in the v0.20.0 textarea state revision (or, if patch-safe, by gating history behind a separate state type that the textarea looks up via `use_state_named`).
- **#171** (`terminal` per-row hash skip in `flush_buffer_diff`) — landed during the wave but reverted: the issue body's own bench gate (`< 50µs → NO-GO`) was not met, and a regression test caught a row-skip case where the dirty-flag interaction lost a changed row. Will re-evaluate once a dedicated bench rig lands.
- **#150 / #152 / #153 / #155 / #157 / #162** (layout perf — commands buffer reuse, `group_name → Arc<str>`, `LayoutNode::TextData` boxing, `FrameData` reuse, `wrap_segments` scratch, viewport bound check) — landed but reverted while triaging the unrelated v0.19.1 title-clamp regression. Will re-apply on top of the title fix in v0.19.3 with isolated visual gates.
- **#146 / #147 / #148** (container — `filled_circle` `isqrt`, `min_h` / `max_h` breakpoint variants, deprecation of long-form aliases) — same triage cycle as the layout perf set; re-apply in v0.19.3.

## [0.19.1] — 2026-04-27

### Fixes (Blocker)

- **`rgb_to_ansi256` u8 overflow at `r=g=b=248`** (#104) — `232u8 + 24u8` wrapped, panicking in debug builds and silently mapping near-white grayscale to `Color::Indexed(0)` (Black) in release. Inclusive boundary `r >= 248` closes the gap. Adds 256³ exhaustive panic-free regression test.
- **`treemap` label byte-slicing panic on multibyte input** (#112) — `&item.label[..max_label_w]` cut into the middle of CJK and emoji characters, raising `byte index N is not a char boundary`. Replaced with `char_indices` + `UnicodeWidthChar` truncation; label centering now uses display width via `UnicodeWidthStr`.

### Fixes (Critical)

- **`textarea` paste with `max_length` rescans every line per character** (#91) — previously `O(n²)` over paste length × line count. Hoisted `total_chars` once per paste, and the newline branch now also respects `max_length` (secondary bug).
- **WCAG luminance must apply sRGB gamma linearization** (#105) — `Color::luminance()` now linearizes via the sRGB inverse transfer function before applying BT.709 weights. `contrast_fg` threshold corrected from `0.5` to the WCAG `0.179`. Dracula purple, Solarized base1, and similar mid-tones now route to the correct contrasting foreground.
- **`syntax::highlight_code` allocates `Highlighter` per call** (#113) — switched to a `thread_local! RefCell<Highlighter>` so the parser is reused across frames in the single-threaded event loop.
- **`Spring` damping > 1.0 diverges, damping == 1.0 oscillates forever** (#124) — added `debug_assert!` validating `damping ∈ (0, 1)` in `Spring::new`. Doc clarifies that the parameter is the per-tick velocity multiplier, not the standard ODE damping ratio ζ.
- **`group("name").scrollable(...)` silently drops hover/focus registration** (#141) — `BeginScrollableArgs` now carries `group_name`; `tree.rs` propagates it onto the layout node so `is_group_hovered`/`is_group_focused` work for scrollable containers.
- **`BeginScrollableArgs` ignored 5 container fields** (#142) — `bg_color`, `align`, `align_self`, `justify`, `gap` are now passed through and applied; previously hard-coded defaults silently overrode the builder chain.
- **`flexbox` grow children with `max_width`/`max_height` left a gap** (#159) — sibling position now advances by post-clamp `child.size.0`/`child.size.1` plus margin, not the pre-clamp share.
- **Border title truncation breaks on CJK** (#160) — `chars().take(n)` (code-point count) replaced with `UnicodeWidthChar`-based display-width loop matching `truncate_with_ellipsis`. Title clamp also accounts for the right corner cell.
- **`Rect::area()` u32 multiplication can wrap to 0** (#166) — `saturating_mul` prevents `Buffer::empty(rect)` from allocating a zero-sized buffer for hostile/test dimensions on WASM and 32-bit targets.
- **`stdout` not buffered → one write syscall per ANSI command** (#172) — `Terminal` and `InlineTerminal` now wrap `stdout` in `BufWriter::with_capacity(65536, _)`; `TerminalSessionGuard` accepts `&mut impl Write`. A single `flush()` per frame replaces dozens of `write_all` calls.
- **`image()` emits 841 commands/frame for a 40×20 image** (#174) — replaced the per-pixel `Command::Text` row-container nesting with a single `container().draw(move |buf, rect| { ... })` (matches `kitty_image`/`big_text`). 841 commands → 1 RawDraw, 800 String allocations/frame eliminated.
- **`confirm()` `[No]` hit region was unbounded to the right** (#175) — added `no_end = no_start + 4` and the `mx < no_end` check; click-driven hit-test no longer registers as `[No]` for clicks anywhere right of the prompt.

### Fixes (Warning, batched)

- **`use_state_named_with` double `HashMap` lookup** (#137) — switched to the `entry` API.
- **`use_memo` redundant `downcast_ref`** (#138) — single downcast on cache hit; type mismatch on existing slot now panics with hook index and expected type, instead of silently overwriting.
- **`ContextCheckpoint` no longer deep-clones `pending_tooltips`** (#138) — the queue is moved out and restored, with a per-checkpoint `pending_tooltips_len` so panic recovery truncates rather than clears.
- **`group_stack` and group rect/focus paths now use `Arc<str>`** (#139) — name materialized once per `group()` call; descendants `Arc::clone` instead of allocating per node.
- **`screen()` HashMap key switched to `&'static str`** (#136) — fewer allocations for screen lifecycle hooks. (Non-breaking variant; the `String` overload remains available.)
- **`consume_activation_keys` uses inline scratch** (#136) — typical 1–2 active keys no longer trigger heap allocation.
- **`render_notifications` reuses scratch `Vec`** (#136) — frame-rate path no longer allocates when notifications are empty.
- **`is_group_hovered`/`is_group_focused` cache** (#136) — switched to a `HashSet<Arc<str>>` lookup parallel to the rect list.
- **`TextInputState::clone()` documents validator drop** (#92) — explicit doc note; clone returns a no-validator copy by design (validators are `Box<dyn Fn>` and cannot be cloned).
- **`text_input` `matched_suggestions` invalidation** (#93) — `suggestions_dirty` flag is set on Char/Backspace/Delete/paste; suggestions recompute exactly when `state.value` changes within a key burst.
- **`textarea` change detection via lines comparison** (#94) — `response.changed` now reflects whether `state.lines` differs from the pre-frame snapshot. The faster dirty-flag path (#95) is deferred to v0.20.0 because adding the field is not patch-safe under cargo-semver-checks (struct literal compatibility).
- **`textarea` `visual_lines` reused on idle frames** — when `state.lines == pre_lines`, the pre-event `pre_vlines` is reused; mutation frames rebuild. No new state field required.
- **`ListState::set_filter` uses cached lowercase items** (#96) — eliminates per-keystroke `to_lowercase()` over the whole item set.
- **`slider_with_step(label, value, range, step)`** (#97) — additive method that takes an explicit step; the existing `slider()` keeps the `span/20` default.
- **`rgb_to_ansi16` saturated colors map to standard ANSI** (#107) — pure primaries (`min == 0`) route to `Red/Green/Blue/...`; only desaturated and lifted tones (e.g. `255, 85, 85`) become `Light*` variants. Bright/standard split now uses `max >= 200 && min >= 64` instead of WCAG luminance.
- **Nord, Solarized Dark, and Solarized Light theme `text_dim` no longer collides with `border`** (#106) — distinct values restore the visual hierarchy in default themes.
- **`Stagger::is_all_done()` and `is_done()` semantics** (#127) — new method reports completion across all items rather than the last-sampled one.
- **`bind_code_mod(KeyCode, KeyModifiers, &str)`** (#128) — additive `KeyMap` builder for non-`char` keys with modifiers (`Ctrl+Enter`, `Alt+Up`, etc.).
- **Braille dot bit constants deduplicated** (#114) — `widgets_viz.rs` line chart now imports `BRAILLE_LEFT_BITS`/`BRAILLE_RIGHT_BITS` from `chart::braille` instead of redeclaring them; eliminates the silent-divergence risk on future edits.
- **`Keyframes::segment_easing` debug-asserts on out-of-range index** (#130) — release builds still ignore silently (preserving the panic-free guarantee), but builder-order mistakes now surface immediately in development.
- **`treemap` byte-slice fix** (#112) — see Blocker section.
- **`commands` Vec reused across frames via `FrameState.commands_buf`** (#143) — eliminates the per-`Context::new()` allocation.
- **`is_scrollable` branch merge in `collect.rs`** (#144) — single conditional path keeps `scroll_infos`/`scroll_rects` pairing invariant explicit.
- **`group_name` switched to `Option<Arc<str>>` in build path** (#145) — completes the `Arc<str>` migration started in v0.18.1.
- **Buffer/`set_string_inner` dedup** (#169) — the OSC 8 and non-linked paths share an inner helper; the duplicated branches diverged on minor edge cases (URL validation gating).
- **`is_valid_osc8_url(&str) -> bool`** (#168) — separate from `sanitize_osc8_url(&str) -> Option<CompactString>`; URL validation no longer allocates a `String` only to drop it.
- **`Rect::area()` saturating fix** (#166) — see Critical section.
- **`separator()` uses `area_width` and does not stretch in column layouts** (#163) — restored `grow: 0` (column layouts were stretching the separator vertically); buffer clipping handles the cached 200-cell fill on narrow terminals.
- **`scrollbar` thumb/track use `&'static str`** (#164) — eliminates `char.to_string()` per visible cell.
- **`divider_text` symmetric label centering** (#186) — replaced `left_len = 4` with `total / 2` so the label sits centered; on odd widths the right separator is one cell longer.
- **`tree()` and `directory_tree()` redundant `flatten()` per keypress** (#190) — hoisted `entries` out of the key-event loop; up/down/left/right arms now reuse the outer flatten snapshot. Single O(n) DFS per keypress instead of two.
- **`streaming_markdown` skips `code_block_lang` writes when unchanged** (#187) — guards the unconditional `state.code_block_lang = ...` assignment, eliminating the String drop+clone on idle frames between code-block transitions.
- **`form_field` no longer clones `field.label` per frame** (#189) — `as_str()` borrow + `&str` for the validation error path; eliminates two String allocations per form field render.
- **`definition_list` and `divider_text` use short `UnicodeWidthStr::width(...)`** (#185) — `unicode_width::UnicodeWidthStr::*` fully-qualified path replaced with the in-scope import.
- **Doc duplicate paragraphs on `list()`, `table()`, `button()`, `tabs()`** (#197) — merged duplicate `///` blocks; rustdoc no longer renders two near-identical descriptions on docs.rs.
- **Modal doc example used `if ui.button(...)` as `bool`** (#188) — switched to `.clicked` and a runnable `no_run` block driven by `slt::run`.
- **Layout tree depth guard** (#154) — `collect_all_inner`, `compute_inner`, and `render_inner` now thread an explicit depth counter and panic at `512` to prevent stack overflow on adversarial inputs. Build path already had a guard.
- **`LayoutNode::raw_draw(...)` constructor** (#156) — replaces the inline 34-field literal in `build_children` for `Command::RawDraw`; the test-only `collect_raw_draw_rects` walker (#158) is removed and its check folded into `collect_all_clips_raw_draw_to_scroll_viewport`.
- **`is_scrollable` branch merge in `collect.rs`** (#151) — single conditional path keeps the `scroll_infos`/`scroll_rects` pairing invariant explicit.
- **`run_async_loop` messages `Vec` hoisted** (#83 / a1-003) — cleared and reused across iterations rather than reallocated.
- **`InlineTerminal::new` honors `RunConfig::kitty_keyboard`** (#84 / a1-004) — flag was previously hardcoded `false`, silently ignoring the user setting in inline/static modes.
- **F12, mouse, and resize event passes consolidated** (#86 / a1-006) — `events: &[Event]` is now scanned once instead of three times per frame.
- **`update_last_mouse_pos` runs after `clear_frame_layout_cache`** (#90 / a1-010) — fixes a frame ordering edge case on simultaneous resize + mouse events.

### Fixes (Nit, batched)

- `frame_owned(events: Vec<Event>)` exposes a zero-copy public path for custom backends; `frame(&[Event])` retained (#81).
- FPS cap math accounts for `poll_events` blocking time so `tick_rate=16ms` + `max_fps=60` no longer caps below the configured rate (#82).
- `RunConfig::scroll_alignment_assert` downgraded to `debug_assert_eq!` (#85).
- `RunConfig::no_fps_cap()` builder method (#87).
- `set_terminal_title` flushes stdout (#88).
- 7 undocumented `pub mod` declarations gained doc comments (#89).
- `progress_bar_colored` uses `String::with_capacity` (#100).
- Theme `bright` palette restored on the `rgb_to_ansi16` path (#107).
- `MouseEvent::pixel_x` / `pixel_y` doc clarifies they are always `None` with the crossterm backend; reserved for future Kitty/WASM sub-cell precision (#129).
- `EventBuilder::mouse_up` / `drag` / `key_release` / `focus_gained` / `focus_lost` chain wrappers in `test_utils` (#131).

### Tests

- All fixes ship with regression tests; `cargo test --all-features` passes 13 binaries / 530+ test cases on `release/v0.19.1`.

### Notes

This release is a non-breaking patch wave. Public API surface is unchanged; the `Context::pending_tooltips` field migration is `pub(crate)` only.

## [0.19.0] — 2026-04-21

### Features

- **`ui.provide(value, |ui| ...)` + `ui.use_context::<T>()` + `ui.try_use_context::<T>()`** — scoped context injection for values that cross 3+ scope levels (theme, user, feature flags). No more threading `&Theme`, `&mut ToastState`, `tick` through every `render_*` parameter. Nested `provide` of the same type shadows outer (LIFO). Panic-safe pop via `std::panic::catch_unwind` + `AssertUnwindSafe`; `ContextCheckpoint` also tracks `context_stack_len` so `error_boundary` correctly unwinds partially-pushed context values. Closes #66.
- **`ui.use_state_named(id)` + `ui.use_state_named_with(id, init)`** — component-local state keyed by a stable `&'static str` id. Safe inside conditional rendering (unlike order-based `use_state`). Reusable component functions can now own internal state (expand/collapse, pagination cursor, filter mode) without requiring the caller to allocate a state struct. State handle is the existing `State<T>` type via an internal `StateKey::Named(&'static str)` discriminant — no new public type. Closes #71.
- **`.with_if(cond, modifier)` + `.with(modifier)`** — fluent conditional styling on text and `ContainerBuilder`. Compresses the 8-line `if cond { t.bold(); t.fg(Color::Red); }` pattern into one chained call. Two signatures: text uses `FnOnce(&mut Self)` (mutable-handle style to match existing text modifiers), container uses `FnOnce(Self) -> Self` (by-value to match existing builder idiom). Closes #68.

### Documentation

- **`docs/PATTERNS.md`** — new `## Components` section (~250 lines) covering "Components as Functions" (canonical pattern), "Component-local State with `use_state_named`", "Context Injection with `ui.provide` / `ui.use_context`", "Conditional Styling with `.with_if`", plus a `When to use which` comparison table and an `Anti-patterns` closer. Cross-links to COOKBOOK.md, STATE_APIS.md, COMPLETE_REFERENCE.md. Closes #72.
- **`examples/demo_website.rs`** — refactored to showcase the new APIs. Root closure calls `ui.provide(AppState { theme, tick }, |ui| ...)`; `render_home` / `render_docs` / etc. read theme and tick via `ui.use_context::<AppState>()` instead of receiving them as parameters. Mutation-heavy sections retain explicit `&mut` params for clarity (context for reads, explicit params for writes). Closes #75.

### Tests

- `tests/context_provider.rs` (8 tests) — round-trip, nested same-type shadowing, two different types coexisting, `try_use_context` None/Some, `use_context` panics when missing, stack pops after closure returns, `provide` returns body's value.
- `tests/use_state_named.rs` (6 tests) — persistence across frames, independent state for different ids, same-id sharing semantics, `Default::default()` init path, type-mismatch panic, safe inside conditional rendering.
- `tests/with_if.rs` (8 tests) — true/false branches on text and container, chained composition, `.with` unconditional variant.
- `tests/v0_19_api_integration.rs` (10 tests) — end-to-end combinations of all three APIs together.

### Semver

Additive only; no breaking changes. `cargo-semver-checks` verifies compatibility.

## [0.18.2] — 2026-04-21

### Performance

- **`flush_buffer_diff` run-length coalescing** — consecutive changed cells in the same row that share style, hyperlink, and column adjacency are now emitted as a single `Print(run)` instead of per-cell `Print`. The cursor-move + style-delta happens once per run. Estimated 200×60 full-redraw reduces `queue!` calls from ~12000 to ~2000 — 3-5x flush-path speedup expected on redraw-heavy frames (e.g., `demo_fire`). Closes #62.
- **`Command` enum size reduction** — `Command::BeginContainer` and `Command::BeginScrollable` fat variants now wrap `Box<BeginContainerArgs>` / `Box<BeginScrollableArgs>`. Enum size drops from ~200 bytes to ≤ 128 (new `size_of::<Command>()` regression test asserts this). 18 call sites updated; external public API unchanged. Closes #64.
- **`flexbox::layout_row` / `layout_column` inline scratch** — introduced `U32Stack` (inline `[u32; 16]` + heap overflow) to avoid allocating `Vec<u32>` x4 per call. Eliminates ~4 allocations per flexbox call for child counts ≤ 16 (typical case); deep nested dashboards see ~15-30% layout-step speedup. Closes #67.

### Benchmarks

- **`bench_flush_full_redraw_200x60` + `bench_flush_sparse_change_200x60`** — new criterion benches measuring actual stdout-flush cost into a `Vec<u8>` sink. Adds one `#[doc(hidden)] pub fn __bench_flush_buffer_diff` helper that wraps the private `flush_buffer_diff` for hermetic measurement. Closes #70.

### Documentation

- **`docs/ARCHITECTURE.md`** — pipeline redescribed accurately as **four top-level DFS passes** (build_tree → flexbox → collect_all → render), not "single DFS". Added `The collect_all consolidation` subsection explaining the real improvement: the collect phase went from 7 sub-walks to 1, reducing the total frame work from 10 traversals to 4.
- **`README.md`** + **localized READMEs (zh-CN, es, ja, ko)** + **`docs/COMPLETE_REFERENCE.md`** — matching language updates to describe the four-stage pipeline. Closes #73.

## [0.18.1] — 2026-04-21

### Documentation

- **`docs/COOKBOOK.md`** — five copy-paste app recipes (login form with validation, data table with search + sort, modal + toast confirmation, real-time dashboard with charts, file picker with preview). Each recipe has a matching runnable example at `examples/cookbook_<name>.rs`. Closes #59.
- **`docs/PREVIOUS_FRAME_GUIDE.md`** — Frame N vs N+1 timeline, when `Response.rect` is valid, `if ui.tick() > 0` idiom, common pitfalls (flicker, focus, animation target). Closes #60.
- **`docs/STATE_APIS.md`** — every public `*State` type with full field + method reference (1028 lines). Previously WIDGETS.md only listed fields; AI agents couldn't discover methods like `.validate()`, `.toggle_sort()`, `.set_filter()`. Closes #61.
- **`docs/llms.txt`** + **`docs/COMPLETE_REFERENCE.md`** — llms.txt manifest and single-file ~1500-line condensed reference optimized for LLM context windows. Closes #63.
- **`.claude/skills/slt/`** — Claude Code skill embedded in the repo. Provides `/slt` authoring guidance when Claude Code runs inside this project. Closes #65.

### Safety

- **`Buffer::kitty_clip_info_stack`** — replaced `Option<(u32, u32)>` with a push/pop `Vec<KittyClipInfo>`. Nested raw-draw callbacks no longer silently clobber outer clip state. Callback invocation wraps a `KittyClipGuard` (RAII `Drop`) so the stack pops even on panic. A `debug_assert!` enforces empty-stack-at-end-of-frame. Closes #69.

### Performance

- **`wrap_lines` / `wrap_segments`** — rewritten as single-pass scans over byte-index ranges. Eliminates per-word `String` reallocation via `mem::take`, eliminates the intermediate `Vec<(char, Style)>` in `wrap_segments`. Allocation count drops from roughly O(words) to O(lines). No signature change; 14 new regression tests added. Closes #74.
- **`collect_all` group names** — `FrameData.group_rects` and `FrameData.focus_groups` switched from `String` to `Arc<str>`. Group names are materialized once per group container; descendant focus registrations inherit via pointer-bump `Arc::clone` instead of per-hit `String` allocation. `Context`/`LayoutFeedbackState` `prev_*` mirrors bumped to match. Closes #76.

### Infrastructure

- **`CLAUDE.md`** — prominent top-level release-workflow checklist (8 steps: local PRE-CI → branch → PR → wait CI → merge → tag → verify release → announce). Codifies the hard-won lesson that every step is mandatory, no "probably fine" shortcuts.

### Not planned

- **`render_inner` unicode-width cache** — investigation (#77) found the proposed optimization invalid. `LayoutNode.size.0` is the allocated box width from flexbox, not the text's unicode width; replacing `UnicodeWidthStr::width(text)` with `node.size.0` would break alignment (offset always 0) and truncation (`size.0 > size.0` always false). Closed as wontfix. A separate `text_pixel_width` field on `LayoutNode` would be the correct fix if CJK render cost becomes measurable.

## [0.18.0] — 2026-04-17

### Security

- **Terminal escape injection prevention** — `Buffer::set_string` / `set_string_linked` now replace C0 (`0x00–0x1F`), DEL (`0x7F`), and C1 (`0x80–0x9F`) control bytes with `U+FFFD` before writing into cells. Previously a zero-width control character pushed into `cell.symbol` was emitted verbatim at flush time, letting attacker-controlled strings (chat logs, file names, paste buffers) break out of their cells and execute arbitrary terminal commands — cursor moves, OSC 52 clipboard hijack, title spoof, OSC 8 link spoof. CVE-2003-0063-class bug closed.
- **OSC 8 hyperlink URL sanitization** — `sanitize_osc8_url()` rejects URLs containing control bytes, BEL, ESC, or exceeding 2 KiB. Applied at write time (`set_string_linked`) and at flush time (defense in depth for direct `Cell::hyperlink` writes). Prevents URL-borne OSC injection (`https://example.com\x07` trailer attacks).
- **Bracketed-paste DoS hardening** — paste payloads are capped at 1 MiB at `Event::Paste` construction; over-long payloads are truncated on a char boundary with an ellipsis. `TextInputState` insert loop is now O(n) (cached `char_count`) instead of O(n²) (`chars().count()` per inserted character) — a 1 GB paste previously hung for minutes. Pastes into text inputs also strip control bytes to preserve the no-newline invariant.
- **Image dimension bounds** — `HalfBlockImage::{from_dynamic, from_rgb}`, `normalize_rgba`, and `encode_sixel` reject inputs whose `width × height` exceeds `MAX_IMAGE_PIXELS` (16_777_216, ≈ 4096×4096). Prevents multi-GiB allocations from hostile inputs and fixes 32-bit overflow on WASM targets.
- **Cell symbol byte cap** — zero-width combining append into a cell's `symbol` now caps at `MAX_CELL_SYMBOL_BYTES` (32). Blocks the "a million combining marks balloon one cell" pattern.

### Features

- **`ColorDepth::NoColor`** — new variant respecting [`NO_COLOR`](https://no-color.org). `ColorDepth::detect()` returns `NoColor` when the `NO_COLOR` env var is set to any non-empty value; every color downsamples to `Color::Reset` and no SGR color codes are emitted.
- **Focus-change events independent of mouse** — `FocusGained` / `FocusLost` are enabled on every terminal session, not only when `RunConfig::mouse(true)`. Keyboard-only apps that pause animations or clear hover state on window blur get those events for free. Matches modern convention (zellij, helix, yazi).
- **`Buffer::try_get` / `try_get_mut`** — non-panicking counterparts to `get` / `get_mut`, returning `Option<&Cell>`. Use inside `draw()` closures where coordinates may come from mouse input or scroll offsets.
- **`ModeState::try_switch_mode`** — returns `bool` instead of panicking when the mode has not been registered. Panicking `switch_mode` stays for ergonomic use cases.
- **`ui.scroll_col` / `ui.scroll_row`** — shortcut for the `scrollable(state).grow(1).col(f)` pattern that appeared verbatim in 6+ examples.
- **`ContainerBuilder::draw_with<D: 'static>`** — passes owned per-frame data through to the deferred raw-draw closure as a borrow. Cleaner than manually `move`ing a snapshot binding into `draw()`.
- **`form_submit` rendered as `ButtonVariant::Primary`** — distinguishes the submit affordance from incidental buttons in the same form.
- **Snapshot testing docs** — `docs/TESTING.md` now has a dedicated "Snapshot testing with `insta`" section pointing at `tests/snapshots.rs` for ~10 live examples.

### Performance

- `TextInputState` paste path is O(n) instead of O(n²). At 1 MiB paste length this is the difference between an instant update and a multi-second hang.
- Paste memory cap (1 MiB) bounds the event queue under hostile paste floods.

### Breaking Changes

- **`Context::text_wrap()` removed** — deprecated since 0.15.4. Use `ui.text(s).wrap()`.
- **`Context::bar_chart_styled()` removed** — deprecated since 0.16.1. Use `ui.bar_chart_with(...)`.
- **Thinner crate-root re-exports** — the following are no longer at `slt::*`; import from `slt::anim::*` / `slt::chart::*` instead:
  - Easing: `ease_in_cubic`, `ease_in_out_cubic`, `ease_in_out_quad`, `ease_in_quad`, `ease_linear`, `ease_out_bounce`, `ease_out_cubic`, `ease_out_elastic`, `ease_out_quad`, `lerp`
  - Chart internals: `ChartRenderer`, `RenderedLine`, `ColorSpan`, `DatasetEntry`, `HistogramBuilder`, `GraphType`, `Axis`
  - All in-tree examples and docs already used the namespaced paths.
- `ColorDepth` gained a new variant (`NoColor`). Because the enum is `#[non_exhaustive]`, external pattern-matches already required a wildcard arm and should not break.

### Cleanup

- Removed dead debug markers `draw_debug_padding_markers` / `draw_debug_margin_markers` (`#[allow(dead_code)]` with zero call sites).
- Removed stale `bar_chart_styled` doc references in `docs/WIDGETS.md` and `widgets_viz.rs`.

### Tests

- Added 8 targeted security and API tests: control-char replacement in `set_string`, zero-width BEL rejection, OSC 8 URL validation, combining-char byte cap, `Buffer::try_get` bounds, `ModeState::try_switch_mode` behavior, image dimension rejection in `HalfBlockImage::from_rgb` and `encode_sixel`, `NoColor` downsampling. 232 lib tests + 252 snapshot tests + doctests all green.

## [0.17.1] — 2026-04-05

### Features

- **Raw event access** (#56)
  - `events()` — iterate unconsumed events with modal guard
  - `raw_events()` — iterate unconsumed events bypassing modal guard

- **Mouse drag/up convenience methods** (#54)
  - `mouse_drag()`, `mouse_up()` — left-button drag and release position
  - `mouse_down_button()`, `mouse_drag_button()`, `mouse_up_button()` — any button variant
  - `Event::mouse_drag()`, `Event::mouse_up()` — test event constructors

- **Per-column grid widths** (#55)
  - `GridColumn` enum — `Auto`, `Fixed(u32)`, `Grow(u16)`, `Percent(u8)`
  - `grid_with(&[GridColumn], f)` — grid layout with per-column width control

- **Clickable custom-drawn regions** (#52)
  - `draw_interactive()` — like `draw()` but returns `Response` with `clicked`/`hovered`
  - RawDraw nodes now correctly consume pending interaction IDs in the layout tree

### Bug Fixes

- **grid() InteractionMarker preservation** — interactive widgets (buttons, links) inside `grid()` and `grid_with()` now correctly retain their interaction markers instead of detaching them into separate cells

### Tests

- Added 9 new tests (222 total): mouse_drag, mouse_up, right-click detection, consumed flag filtering, events modal guard, draw_interactive hit-testing, draw backward compat, grid_with constraints

## [0.17.0] — 2026-04-04

### Breaking Changes

- **`Theme` is now `#[non_exhaustive]`** — use `Theme::builder()` or preset constructors instead of struct literal syntax (`Theme { ... ..Theme::dark() }`)
- **`screen()` takes `&mut ScreenState`** — the old `&ScreenState` signature is removed. Each screen now gets isolated hook and focus state.

### Features

- **Design Token System**
  - `Spacing` struct — consistent spacing scale (`xs/sm/md/lg/xl/xxl`) accessible via `ui.spacing()`
  - `ThemeColor` enum — semantic color tokens resolved via `theme.resolve(ThemeColor::Primary)` or `ui.color(ThemeColor::Surface)`
  - `Theme::contrast_text_on(bg)` — auto-select readable text color for any background
  - `Color::contrast_ratio(a, b)` — WCAG 2.1 contrast ratio computation
  - `Color::meets_contrast_aa(fg, bg)` — WCAG AA compliance check (ratio >= 4.5)
  - `Theme::overlay(color, alpha)` — blend color against theme background

- **Screen/Mode System**
  - Hook segment isolation — each `screen()` reserves an independent hook range, preventing `use_state` index collisions across screens
  - Per-screen focus preservation — focus index is saved/restored when switching screens
  - `ModeState` — named modes with independent screen stacks (`switch_mode()`, `screens()`, `screens_mut()`)

- **StyleSheet Evolution**
  - `ContainerStyle::extending(&BASE)` — inherit fields from a base style, override only what you need
  - `theme_bg(ThemeColor)`, `theme_text_color(ThemeColor)`, `theme_border_fg(ThemeColor)` on `ContainerStyle` — theme-aware colors resolved at `apply()` time
  - `WidgetTheme` — global default colors per widget type, set via `RunConfig::widget_theme()`
  - `theme_fg/theme_bg/theme_border/theme_accent` on `WidgetColors` — semantic color overrides with `resolve_*()` helpers

- **New theme presets**: `Theme::gruvbox_dark()`, `Theme::one_dark()`, `Theme::solarized_light()`

### Tests

- Added 11 new tests (213 → 224 target): Spacing scale, ThemeColor resolve, contrast ratio, new presets, ThemeBuilder spacing, contrast helpers.

## [0.16.1] — 2026-03-24

### Features

- **Treemap widget** — `ui.treemap(&items)` renders a squarified treemap from `&[TreemapItem]`. Auto-filters items too small to display. Labels and values are centered with contrast-aware text color.
- **Half-block heatmap** — `ui.heatmap_halfblock()` uses `▀` with fg/bg color to pack two data rows per terminal row, doubling vertical resolution over `heatmap()`.
- **HD candlestick** — `ui.candlestick_hd()` uses heavy box-drawing `┃` for wicks with proper center alignment, improving readability over the standard `candlestick()`.
- **Stacked bar chart** — `ui.bar_chart_stacked()` / `ui.bar_chart_stacked_with()` stacks bars vertically within each group. Accepts `BarChartConfig` for bar_width, gap, and max_value.
- **`run_static_with`** — static-output mode now accepts `RunConfig` for theme, mouse, and tick rate customization.
- **Expanded demo_infoviz** — 8 tabs (was 4): Overview, Lines, Scatter, Bars, Heatmap, Financial, Treemap, Canvas.

### Improvements

- **`ColorSpan` / `RenderedLine` re-exported** — chart renderer types now accessible from crate root.
- **`virtual_list` visible_height** — changed from `usize` to `u32` for consistency with other size parameters.
- **`sixel_image` parameter names** — unified to `pixel_width`/`pixel_height` (was `pixel_w`/`pixel_h`).
- **Doc examples** — standardized to `no_run` (was mixed `ignore`/`no_run`).
- **Widget count** — crate-level doc updated from "30+" to "50+" reflecting actual widget count.

### Deprecations

- **`bar_chart_styled`** — use `bar_chart_with` instead.

### Safety

- **Scroll feedback assert** — promoted `debug_assert` to `assert` for scroll vector alignment check; prevents silent corruption in release builds.
- **Animation div-by-zero guard** — `Tween`, `Keyframes`, and `Stagger` now return target value immediately when `duration_ticks == 0`.

### Tests

- Added 18 new tests (234 → 252): treemap (5), heatmap_halfblock (3), candlestick_hd (3), bar_chart_stacked (3), bar_chart (1), sparkline (1), heatmap (1), candlestick (1).

## [0.16.0] — 2026-03-23

This release is about consolidation, not API sprawl.
The public grammar stays familiar while the runtime, backend path, and verification story become more disciplined.

### What gets better for users

- **Same easy mental model, stronger core** — the closure-oriented API stays the same, but production run loops, `frame()`, and `TestBackend` now share a single internal frame kernel.
- **Large apps are easier to trust** — interaction allocation, rollback handling, layout collection, and frame/session bookkeeping are more explicit internally instead of relying on scattered implicit coupling.
- **Backend path is more credible** — the `Backend` / `AppState` / `frame()` route now has clearer guarantees, stronger docs, and dedicated contract coverage.

### Backend and runtime hardening

- **Single frame kernel** — production and test rendering now share the same internal frame kernel, reducing lifecycle drift between `frame()`, run loops, and `TestBackend`.
- **Session state split** — internal frame/session data is now grouped into explicit focus, layout-feedback, and diagnostics state instead of one broad bucket.
- **Structured rollback** — `Context` rollback state is restored through a dedicated structured checkpoint instead of manual snapshot field syncing.
- **Real `context` modules** — the old `include!`-based aggregation was replaced with actual Rust modules and narrower `pub(crate)` boundaries.
- **Layout kernel split** — the former `layout.rs` monolith is now separated into command, tree, and collect kernels with `collect_all()` as the sole runtime collector.
- **Interaction allocator unification** — widget interaction slot allocation now flows through shared helpers instead of scattered direct counter mutation.
- **Terminal session hardening** — terminal session setup and teardown are now centralized instead of being split across duplicated fullscreen and inline paths.
- **Terminal flush dedup** — fullscreen and inline terminals share the same internal diff, raw-sequence, and cursor writer path.
- **Interactive widget helpers** — core widgets now prefer shared input and hit-test helpers such as `begin_widget_interaction()`, `available_key_presses()`, and `consume_indices()`.

### Testing and verification

- **Backend contract coverage** — added direct `frame()` / `Backend` contract tests for flush-error propagation, quit semantics, `AppState` persistence, and resize handling.
- **Kernel parity coverage** — added dedicated parity tests that compare `TestBackend` with a custom `frame()` backend across stateful hook frames and previous-frame hit testing.
- **Kernel proptest coverage** — added event-sequence property tests to lock parity between the shared frame kernel and headless test rendering.
- **Persistent `TestBackend` session state** — headless tests now retain full internal frame state across renders, allowing multi-frame focus and hit-map regression coverage instead of hook-only persistence.

### Documentation

- **README sharpened** — the landing page now explains the small grammar, where SLT fits well, where it does not, and why the runtime can be trusted.
- **Backend and testing guides expanded** — low-level guarantees, contract testing, and verification strategy are now documented directly instead of being left implicit in the codebase.
- **Patterns guide upgraded** — large-app organization, helper extraction, and readability practices are now first-class documentation topics.

### Migration notes

- **No planned public breaking change for typical apps** — most users should not need to rewrite application code.
- **Custom backend users get stronger guarantees, not a new abstraction** — the `Backend` trait remains intentionally small.
- **Performance note** — the current 0.16 baseline shows a few small micro-benchmark regressions in core buffer/layout paths and a small win in list rendering; correctness and runtime stability were prioritized first.
- **Pixel mouse / smooth scrolling status** — terminal-side SGR pixel mouse mode and smooth sub-cell scrolling are still deferred beyond 0.16; browser-side pixel mouse remains available in the WASM path.

## [0.15.8] — 2026-03-22

### Documentation

- **Complete docs overhaul** — added 7 new guides: `ANIMATION.md`, `THEMING.md`, `BACKENDS.md`, `TESTING.md`, `DEBUGGING.md`, `FEATURES.md`, `AI_GUIDE.md`.
- **WIDGETS.md rewritten** — now a complete API catalog of every widget, state type, ContainerBuilder method, and CanvasContext primitive.
- **PATTERNS.md expanded** — added animation patterns, responsive layout patterns; removed single-API-usage sections that belong in WIDGETS.md.
- **Translation sync** — Korean, Chinese, Japanese, and Spanish READMEs aligned with the current English structure including all new guide links.
- **Cross-reference cleanup** — README.md, docs/README.md, CONTRIBUTING.md, ARCHITECTURE.md, and DESIGN_PRINCIPLES.md now link to all guides consistently.
- **Stale data fixed** — SECURITY.md version updated to 0.15.x, DESIGN_PRINCIPLES.md version range updated, EXAMPLES.md now shows feature flag requirements.

### Fixes

- **Buffer bounds safety** — `Buffer::get()` and `get_mut()` upgraded from `debug_assert!` to `assert!`, enforcing bounds checks in release builds. Selection overlay now guards against out-of-bounds widget rects.
- **F12 debug toggle missing in async** — `run_async_loop` was the only run loop without F12 support; now fixed via shared `poll_events()`.
- **Syntax highlighting graceful degradation** — tree-sitter config failures now return `None` instead of panicking, allowing fallback to keyword highlighting.

### Performance

- **Events clone eliminated** — internal run loops now transfer event ownership via `std::mem::take` instead of cloning the events vector every frame.

### Internal

- **Run loop dedup** — extracted `poll_events()` shared function, reducing ~160 lines of duplicated event polling across 4 run loops to a single implementation.
- **Saturating u16 casts** — all `u32` to `u16` coordinate casts in `terminal.rs` now use a saturating helper instead of raw `as u16` truncation.
- **CLAUDE.md architecture tree** — updated to reflect current module split with accurate line counts.

## [0.15.7] — 2026-03-22

### Improvements

- **Documentation layering** — root `README.md` is now a landing page focused on the product hook, quick start, example highlights, and the next docs to read instead of acting as the full widget catalog.
- **Guided docs** — added `docs/QUICK_START.md`, `docs/WIDGETS.md`, `docs/PATTERNS.md`, and `docs/EXAMPLES.md` to separate onboarding, API discovery, composition patterns, and runnable example navigation.
- **Contributor navigation** — `CONTRIBUTING.md`, `docs/ARCHITECTURE.md`, and `docs/DESIGN_PRINCIPLES.md` now cross-link the new guides so contributors can move between product-facing and codebase-facing documentation without guessing.
- **Docs consistency** — translated README variants now avoid stale widget counts, align dependency wording with the feature-flagged architecture, and replace deprecated `ui.text_wrap(...)` examples with `ui.text(...).wrap()`.

### Cleanup

- **Duplicate local copies removed** — orphaned `* 2.rs` files under `src/layout/` and `examples/` are removed from the release prep branch to reduce contributor confusion.

## [0.15.6] — 2026-03-21

### Improvements

- **Hot-path render cleanup** — text input and textarea cursor placement now tracks the cursor during rendering instead of scanning the full terminal buffer during flush.
- **Inline flush path** — `InlineTerminal` now emits diffs with direct buffer iteration, removing the per-frame `Vec` allocation previously used by `Buffer::diff()`.
- **Layout build path** — `build_tree()` now consumes the command list and moves owned strings/segments into `LayoutNode`s instead of cloning them through the hot path.
- **Wrapped text reuse** — wrapped text measurements now cache by width so repeated layout sizing and render passes can reuse the same wrapped output within a frame.
- **`perf_regression` example** — new headless perf sanity example covering input cursor rendering, wrapped text, and textarea cursor behavior.

## [0.15.5] — 2026-03-21

### Features

- **Kitty image ID management** — images are uploaded once with `a=t,i=ID` and placed with `a=p`. Identical images (by content hash) are never re-uploaded. Unused images are automatically cleaned up from terminal memory.
- **Kitty zlib compression** — new `kitty-compress` feature flag (included in `full`). Image data is compressed with zlib (`o=z`) before base64 encoding, reducing upload size 2–5×.
- **Kitty scroll crop** — images inside `scrollable()` containers are cropped to the visible viewport using Kitty's `y=` and `h=` source rect parameters. Partially visible images render correctly instead of overlapping.
- **Cell pixel size detection** — `kitty_image_fit()` now queries the terminal's actual cell dimensions via CSI 16 t for accurate aspect ratio calculation. Falls back to 8×16 if detection fails.
- **`demo_kitty_image`** — new example: scrollable gallery of 10 generated images demonstrating viewport culling, scroll crop, and image ID reuse.

### Fixes

- **Viewport culling for images** — `collect_raw_draw_rects` uses signed math for Y calculation and tracks scrollable viewport bounds. Images fully outside the viewport are culled entirely (zero I/O cost).
- **`raw_sequence()` respects clip stack** — sixel and other passthrough sequences are now skipped when outside the current clip region, fixing sixel images inside scrollable containers.
- **Kitty image cleanup on exit** — `Terminal::drop` sends `a=d,d=A` to delete all images before leaving the alternate screen.
- **Individual image deletion** — replaced `d=A` (delete all) with `d=i` (delete by ID) for targeted cleanup. Only changed images are re-uploaded.

### Dependencies

- **`flate2`** — optional dependency for zlib compression (behind `kitty-compress` feature).

## [0.15.4] — 2026-03-20

### Features

- **Table cell inline formatting** — `**bold**`, `*italic*`, `` `code` ``, and `[links](url)` in table cells are now rendered with proper styling instead of plain text.
- **List item link/image support** — `- [text](url)` in markdown now renders clickable links, not raw syntax.
- **`line_wrap()` preserves links** — `Command::Link` is no longer dropped; lines with inline links wrap correctly at container boundaries.

### Fixes

- **Image display consistency** — `![alt](url)` renders as alt text only (code styled), matching `md_strip()` output for correct table column alignment.

### Deprecations

- **`text_wrap()`** — use `ui.text("...").wrap()` chaining instead. `text_wrap()` is still available but marked `#[deprecated]`.

### Demo

- **Complex markdown cases** section: wrapping + links, blockquotes, tables with formatting, mixed content.

## [0.15.3] — 2026-03-20

### Fixes

- **Markdown text wrapping** — paragraph text in `ui.markdown()` now auto-wraps to container width using `text_wrap()`/`line_wrap()` instead of overflowing.
- **Markdown links in wrapped text** — `line_wrap()` was silently dropping `Command::Link`; mixed content with links now uses `line()` to preserve clickable links.
- **Table cell width with markdown** — cells containing `**bold**`, `*italic*`, `[links](url)` now calculate column width from display text, not raw markdown source. Prevents column blowup from long URLs.
- **CI commit style check removed** — redundant with squash merge workflow.

### Features

- **Blockquote rendering** — `> text` in `ui.markdown()` renders with `│ ` left bar and italic dim styling.

## [0.15.2] — 2026-03-20

### Features

- **Programmatic focus control** — `ui.focus_index()`, `ui.set_focus_index(n)`, `ui.focus_count()` for keyboard focus management in complex UIs with multiple focusable widgets.
- **Markdown pipe table rendering** — `ui.markdown()` now renders GFM-style pipe tables (`| A | B |`) with box-drawing borders and bold headers.
- **Markdown link support** — `[text](url)` in `ui.markdown()` renders as clickable OSC 8 links via `ui.link()`. `![alt](url)` renders as `[Image: alt]` placeholder.
- **text_input auto-fill** — `text_input()` now uses `grow(1)` internally, filling available width in row layouts without manual container wrapping.
- **Sixel image docs** — `sixel_image()` docstring expanded with usage example and `SLT_FORCE_SIXEL` documentation (API was already public since v0.14.0).

### Performance

- **Image flush optimization** — `raw_sequences` (Kitty/Sixel image data) are now diff-compared between frames. Static images skip the delete + re-upload cycle entirely, reducing per-frame cost to zero for unchanged images.

### Demo

- **`v0.15.2` tab** in the demo showcasing markdown tables, links, focus control, and text_input grow.

## [0.15.1] — 2026-03-20

### Fixes

- **Tab/BackTab/Esc/F-keys now reachable via `key_code()` / `key_mod()`** — `process_focus_keys()` moved after user closure so user code sees events before the focus system consumes them. Focus cycling still works identically for apps that don't intercept Tab.
- **`process_focus_keys()` respects consumed events** — if user calls `consume_key_code(KeyCode::Tab)`, the focus system no longer cycles on that event.

### Features

- **`raw_key_code(code)` / `raw_key_mod(c, mods)`** — global shortcut helpers that bypass the modal/overlay guard. Use for Esc-to-close, Ctrl+Q-to-quit, and other shortcuts that must work regardless of overlay state.

### Demo

- **`demo_key_test`** — interactive key event tester with mode switching, kitty keyboard toggle, and event log.

## [0.15.0] — 2026-03-19

### Breaking Changes

- **`#[non_exhaustive]` on all extensible enums and structs** — 22 enums (`Event`, `KeyCode`, `KeyEventKind`, `MouseKind`, `MouseButton`, `Color`, `ColorDepth`, `Border`, `Breakpoint`, `Align`, `Justify`, `Direction`, `BarDirection`, `AlertLevel`, `Trend`, `ButtonVariant`, `ApprovalAction`, `LoopMode`, `Marker`, `GraphType`, `LegendPosition`, `ColorScheme`) + 3 structs (`RunConfig`, `KeyEvent`, `MouseEvent`). Existing exhaustive `match` statements must add `_ =>` arm. Struct literal construction from external crates must use builder/constructor.
- **`RunConfig` is now `#[non_exhaustive]`** — use `RunConfig::default().mouse(true).theme(Theme::dark())` builder pattern instead of struct literal

### Features

- **9 new `KeyCode` variants** — `Insert`, `Null`, `CapsLock`, `ScrollLock`, `NumLock`, `PrintScreen`, `Pause`, `Menu`, `KeypadBegin`. Previously silently dropped by the crossterm conversion.
- **3 new `KeyModifiers`** — `SUPER` (Cmd/Win), `HYPER`, `META`. Enables capturing Cmd+S, Win+key combos with Kitty keyboard protocol.
- **`MouseKind::ScrollLeft` / `ScrollRight`** — horizontal scroll events
- **`MouseEvent::pixel_x` / `pixel_y`** — optional pixel-level coordinates. WASM populates from browser; `None` for crossterm
- **`MouseEvent::new()`** — constructor for `#[non_exhaustive]` struct
- **`MouseEvent::is_scroll()`** — check if event is any scroll variant
- **`RunConfig` builder methods** — `.mouse()`, `.kitty_keyboard()`, `.theme()`, `.tick_rate()`, `.color_depth()`, `.max_fps()`, `.scroll_speed()`, `.title()`
- **`RunConfig::scroll_speed`** — configure scroll lines per event at startup
- **`RunConfig::title`** — set terminal window title via OSC 2
- **`Context::set_scroll_speed(n)` / `scroll_speed()`** — runtime scroll speed
- **`Context::scroll_left()` / `scroll_right()`** — horizontal scroll query methods
- **`Event::scroll_up(x, y)` / `scroll_down(x, y)` / `key_release(c)`** — new constructors
- **`Event::is_key()` / `is_mouse()`** — type check helpers
- **`KeyEvent::is_char(c)` / `is_ctrl_char(c)` / `is_code(code)`** — pattern matching helpers
- **`KeyEvent` re-exported** from `slt::KeyEvent` (was missing)

### Notes

- This is a **semver-breaking** release (0.14 → 0.15) due to `#[non_exhaustive]` additions
- Compositing/z-order (overlay, modal) was already fully implemented — no changes needed
- Pixel mouse SGR pixel mode and smooth sub-pixel scrolling deferred to v0.16 pending crossterm upstream support and layout engine refactor
- WASM backend now populates `pixel_x`/`pixel_y` from `event.offset_x()`/`event.offset_y()` — first TUI framework with pixel mouse in browser

## [0.14.2] — 2026-03-19

### Improvements

- **100% doc coverage** — all 101 previously undocumented pub items now have `///` doc comments
- **`#![warn(missing_docs)]`** enabled crate-wide — future pub items without docs produce compiler warnings
- **Event safe accessors** — `Event::as_key()`, `Event::as_mouse()`, `Event::as_resize()`, `Event::as_paste()` return `Option` instead of panicking

### Fixes

- **cfg-gate cleanup** — 9 dead-code warnings when building with `--no-default-features` eliminated. `PANIC_HOOK_ONCE`, `update_last_mouse_pos`, `clear_frame_layout_cache`, `sleep_for_fps_cap` now properly gated behind `crossterm` feature. `sixel_image()` split into crossterm/non-crossterm variants.

## [0.14.1] — 2026-03-19

### Features

- **Tree-sitter syntax highlighting** — `syntax` feature enables AST-accurate code highlighting via `tree-sitter-highlight`. Supports 15 languages: Rust, Python, JavaScript, TypeScript/TSX, Go, Bash, JSON, TOML, C, C++, Java, Ruby, CSS, HTML, YAML. Per-language features available (`syntax-rust`, `syntax-python`, etc.)
- **`code_block_lang(code, lang)`** — new API renders code blocks with language-aware tree-sitter highlighting (falls back to keyword highlighter when `syntax` feature is off or language is unknown)
- **`code_block_numbered_lang(code, lang)`** — numbered variant with same tree-sitter integration
- **`highlight_code(code, lang, theme)`** — public API in `slt::syntax` returns styled segments for custom rendering
- **`is_language_supported(lang)`** — query whether tree-sitter highlighting is available for a language

### Improvements

- **`streaming_markdown()`** code blocks now use per-token keyword highlighting instead of single-color rendering
- **`markdown()`** fenced code blocks now properly track open/close state and render with syntax highlighting (tree-sitter when available, keyword fallback otherwise). Previously, code blocks were rendered as a single `┌─code─` border with no content handling.

### New Dependencies (all optional)

- `tree-sitter-highlight` 0.26 (behind `syntax-*` features)
- 15 grammar crates: `tree-sitter-rust`, `tree-sitter-python`, `tree-sitter-javascript`, `tree-sitter-typescript`, `tree-sitter-go`, `tree-sitter-bash`, `tree-sitter-json`, `tree-sitter-toml-ng`, `tree-sitter-c`, `tree-sitter-cpp`, `tree-sitter-java`, `tree-sitter-ruby`, `tree-sitter-css`, `tree-sitter-html`, `tree-sitter-yaml` (each behind their `syntax-*` feature)

### Notes

- `syntax` feature requires Rust 1.84+ (tree-sitter MSRV). Base MSRV unchanged at 1.81.
- `syntax` is NOT included in `full` to avoid C build dependency surprises. Opt in explicitly with `features = ["syntax"]`.
- Existing `code_block()` and `code_block_numbered()` APIs unchanged — no breaking changes.

## [0.14.0] — 2026-03-19

### Breaking Changes

- **crossterm is now optional** — `crossterm` is a default feature. Users with `default-features = false` must add `features = ["crossterm"]` to retain `run()`, `run_with()`, and other terminal I/O functions. `Backend`, `AppState`, `frame()`, all widgets, and Event types remain always available.
- **Workspace structure** — project is now a Cargo workspace with `slt-wasm` companion crate.

### Features

- **Gradient text** — `ui.text("hello").gradient(Color::Red, Color::Blue)` interpolates foreground color per character
- **BigText (ASCII art)** — `ui.big_text("SLT")` renders 8×8 bitmap font as half-block characters (4 terminal rows tall)
- **Timer display** — `ui.timer_display(elapsed)` formats `Duration` as `MM:SS.CC` or `HH:MM:SS.CC`, stateless display-only
- **QR code** — `ui.qr_code("url")` renders QR codes using half-blocks (requires `features = ["qrcode"]`)
- **RichLog** — `ui.rich_log(&mut state)` scrollable log viewer with styled entries, auto-scroll, max_entries trimming
- **DirectoryTree** — `ui.directory_tree(&mut state)` tree widget with folder/file icons, `from_paths()` builder
- **Event constructors** — `Event::key_char('q')`, `Event::key(KeyCode::Enter)`, `Event::resize(80, 24)`, `Event::mouse_click(x, y)`, etc. — create events without crossterm dependency
- **OSC 11 background color query** — `detect_color_scheme()` returns `ColorScheme::Dark`/`Light`/`Unknown` (crossterm-only)
- **OSC 52 clipboard read** — `read_clipboard()` returns clipboard contents via terminal query (crossterm-only)
- **WASM backend** — `slt-wasm` companion crate provides `DomBackend` for browser rendering via `<span>` elements with `requestAnimationFrame` loop

### Architecture

- **crossterm decoupled** — crossterm is now `optional = true` with `default = ["crossterm"]`. Core API (`Backend`, `AppState`, `frame()`, all widgets, Event types) compiles without crossterm. Terminal I/O (`run()` family, `Terminal`, `InlineTerminal`) is `#[cfg(feature = "crossterm")]`
- **Feature flag structure** — `default = ["crossterm"]`, `async = ["dep:tokio", "crossterm"]`, `qrcode = ["dep:qrcode"]`, `full = ["crossterm", "async", "serde", "image", "qrcode"]`

### New Types

- `RichLogState`, `RichLogEntry` — log viewer state
- `DirectoryTreeState` — directory tree state (wraps `TreeState`)
- `ColorScheme` — `Dark`, `Light`, `Unknown` (crossterm-only)

### New Dependencies

- `qrcode` 0.14 (optional, behind `qrcode` feature)
- `wasm-bindgen`, `web-sys`, `js-sys` (slt-wasm crate only)

### Demo

- New "v0.14.0" tab showcasing gradient text, BigText, timer, QR code, RichLog, and DirectoryTree

## [0.13.2] — 2026-03-19

### Features

- **Tooltip** — `ui.tooltip("text")` renders a hover popup for any widget (deferred overlay rendering)
- **Table zebra striping** — `state.zebra = true` for alternating row backgrounds
- **Fuzzy matching** — `command_palette` now scores by character order match, not just substring
- **Calendar widget** — `ui.calendar(&mut CalendarState)` date picker with month navigation and day selection
- **Screens/routing** — `ScreenState` push/pop navigation stack with `ui.screen()` helper
- **Static output** — `slt::run_static()` for CLI tools with scrolling logs above + live TUI below
- **Sixel image** — `ui.sixel_image()` renders images on Sixel-capable terminals (xterm, foot, mlterm)

### Fixes

- **Hit detection architecture** — `hit_areas` now indexed by `interaction_id` (sparse vector) instead of DFS traversal order; fixes mouse hover/click targeting wrong widgets when overlays exist
- **Confirm widget** — add mouse click support for [Yes]/[No] (was keyboard-only)
- **Tooltip rendering** — deferred emit pattern prevents `interaction_count` shifts between main content widgets

### Testing & Infrastructure

- 8 new criterion benchmarks (tabs, checkbox, select, progress, tree, sparkline, grid, calendar)
- 8 new insta snapshot tests for key widget renders
- Demo tab `v0.13.2` added (tab index 13) showcasing all new features

## [0.13.1] — 2026-03-19

### Fixes

- Fix VS16 emoji rendering — explicitly clear trailing cell for characters containing U+FE0F variation selector
- Fix horizontal resize artifacts — clear screen when terminal width shrinks between frames
- Fix modal focus trap — Tab/Shift+Tab now cycles only within modal focusables, preventing focus escape

### Testing

- Add `proptest` property-based testing for layout engine (5 test cases, 500 iterations each)
  - Arbitrary dimensions, extreme grow values, deep nesting, grid layouts, percentage sizing

## [0.13.0] — 2026-03-18

### Breaking Changes

- **`modal()`** now returns `Response` instead of `()` — enables backdrop click detection
- **`overlay()`** now returns `Response` instead of `()` — consistent with other containers
- **`virtual_list()`** now returns `Response` instead of `&mut Self` — consistent with `list()`
- **`command_palette()`** now returns `Response` instead of `Option<usize>` — use `state.last_selected` for the selected index
- **`Response`** now has `#[must_use]` — unused Response values produce compiler warnings

### Features

- Add `Debug` and `Clone` derives to all widget state types (17 types)
  - `FormState` gets `Debug` only (closures prevent `Clone`)
  - `TextInputState` gets manual `Debug`/`Clone` impl (validator closures excluded from clone)
- Add `Debug`/`Clone` to helper types: `ToastMessage`, `ToastLevel`, `TreeNode`, `PaletteCommand`, `FormField`
- Add `CommandPaletteState::last_selected` field for retrieving selected command index

### Performance

- Reduce per-frame string allocations in hot render paths: `format!()` calls 78 → 19 across widget rendering code

### Documentation

- Add doc comments to 28 undocumented public APIs in `widgets.rs` (AlertLevel, FilePickerState, SelectState, etc.)
- Add doc comments to 7 items in `style.rs` (WidgetColors, BorderSides methods)

### Fixes

- Add bounds checking to `Buffer::get()`/`get_mut()` via `debug_assert!` — prevents silent u32 underflow panics
- Add empty cells guard in `streaming_text()` — prevents index-out-of-bounds panic on empty input
- Log errors instead of silently ignoring them in `open_url()` and `copy_to_clipboard()`

### CI/Governance

- Add `typos` job for automatic spell checking
- Add `cargo-hack` job for feature combination verification
- Add `cargo-deny` for license and supply chain security (`deny.toml`)
- Add `committed` for Conventional Commit enforcement on PRs

## [0.12.13] — 2026-03-18

### Fixes

- Fix docs.rs build failure: replace removed `doc_auto_cfg` feature with `doc_cfg` (removed in rustc 1.92.0, merged into `doc_cfg`)

## [0.12.12] — 2026-03-18

### Improvements

- Add `Default` impl for 8 widget state types: `FormField`, `ToastMessage`, `ListState`, `FileEntry`, `TabsState`, `TableState`, `SelectState`, `RadioState`
- Replace ~35 duplicated breakpoint methods on `ContainerBuilder` with `define_breakpoint_methods!` macro
- Split long widget functions into focused helpers: `table()`, `select()`, `bar_chart_styled()`
- Improve `use_memo` panic message with type information and guidance
- Add `PartialEq` derive to `WidgetColors`

### Documentation

- Add doc comments to `ThemeBuilder` (17 methods), `Palette` (16 constants), `ContainerStyle` fields, `WidgetColors` methods, `Modifiers` constants
- Reduce `missing_docs` warnings from 229 to 68
- Update `CLAUDE.md` architecture section (add `widgets_interactive.rs`, update line counts)

### Tests

- Add 49 new unit tests for `style.rs` (24), `style/theme.rs` (11), `widgets.rs` (14)
- Test suite: 52 → 393 total tests

## [0.12.11] — 2026-03-18

### Documentation

- Add `DESIGN_PRINCIPLES.md` — core design philosophy, widget contract, error handling guide, API stability policy
- Add `ARCHITECTURE.md` — module map, frame lifecycle, data flow, visibility rules
- Add `SECURITY.md` — vulnerability reporting policy
- Add PR template with quality checklist
- Add issue templates (bug report, feature request)
- Add `CODEOWNERS`
- Enhance `CONTRIBUTING.md` with widget creation checklist and design principles reference

### Internal

- Add crate-level lints: `forbid(unsafe_code)`, `deny(clippy::unwrap_in_result)`, `warn(clippy::unwrap_used)`, `warn(clippy::dbg_macro)`, `warn(clippy::print_stdout)`, `warn(clippy::print_stderr)`, rustdoc link lints
- Add doc coverage CI check (non-blocking) for `missing_docs` tracking
- Add `doc_auto_cfg` — feature-gated items now display their required feature on docs.rs
- Add `cargo-semver-checks` to CI (informational, non-blocking)
- Reduce crates.io package size (exclude `AUDIT-REPORT.md`, `CLAUDE.md`)

## [0.12.10] — 2026-03-17

### Features

- **`flex_center()`**: ContainerBuilder shorthand for `.justify(Center).align(Center)` — center children on both axes in one call
- **`border_x()` / `border_y()`**: ContainerBuilder shorthands for showing only horizontal (left+right) or vertical (top+bottom) borders
- **`text_center()` / `text_right()`**: Text chain shorthands for `.align(Align::Center)` / `.align(Align::End)` — horizontal text alignment
- **`text_color(Color)`**: ContainerBuilder style inheritance — set a default text color that propagates to all child text elements. Individual `.fg()` calls override.
- **`row_gap()` / `col_gap()`**: ContainerBuilder axis-specific gap control. `row_gap(v)` applies to `.col()` containers, `col_gap(v)` applies to `.row()` containers. `.gap()` still sets both.
- **`align_self(Align)`**: ContainerBuilder per-child cross-axis alignment override, like CSS `align-self`. Each child can independently override the parent's `align()`.
- **`truncate()`**: Text chain method for overflow with ellipsis (`…`). Truncates text to its allocated width when it exceeds the container or `.w()` constraint.
- **`ContainerStyle`**: All 7 new properties available as const methods: `text_color()`, `row_gap()`, `col_gap()`, `align_self()` + composable via `.apply()`
- **`demo.rs`**: New "v0.12.10" tab showcasing all 7 features with interactive examples

## [0.12.9] — 2026-03-17

### Features

- **`border_fg(Color)`**: ContainerBuilder shorthand for border foreground color
- **`separator_colored(Color)`**: Separator with custom color
- **`separator()` chaining**: Now returns `&mut Self` — `.fg()`, `.dim()` etc. chainable
- **`help_colored(bindings, key_color, text_color)`**: Help bar with custom key/text colors

## [0.12.8] — 2026-03-17

### Features

- **`kitty_image_fit(rgba, w, h, cols)`**: Aspect-ratio-preserving image display. Height auto-calculated from image ratio. Terminal handles scaling via Kitty protocol `c`/`r` params — no software resize.
- **`normalize_rgba()`**: RGBA data resilience. Short data is zero-padded, long data truncated. Images never fail silently.
- **`kitty_image()` / `kitty_image_fit()`**: Now return `Response` (was `()`) for API consistency.

### Bug Fixes

- **Jennie image not rendering**: `jpeg_decoder` hardcoded 237px height — now preserves original dimensions per image.
- **Kitty image ghost on tab switch**: Delete all previous images before rendering new frame.

## [0.12.7] — 2026-03-17

### Features

- **`kitty_image_fit()`**: Auto-resize + center-crop images to fill container. Nearest-neighbor scaling, no external dependencies.
- **Kitty image cleanup**: Previous-frame images are deleted before rendering new ones (fixes tab-switch ghost images)
- **demo_wiki**: BLACKPINK wiki-style demo with real namu wiki photos via Kitty protocol

## [0.12.6] — 2026-03-17

### Features

- **Kitty graphics protocol**: `kitty_image()` renders pixel-perfect images via Kitty protocol (Ghostty, Kitty, WezTerm)
- **demo_wiki**: BLACKPINK wiki-style demo with Kitty images and tabbed member profiles

## [0.12.5] — 2026-03-17

### Bug Fixes

- **`candlestick()` container sizing fix**: Switched rendering to `ContainerBuilder::draw()` with layout-provided `Rect`, so chart width/height now match the allocated container area instead of caller-provided dimensions.
- **`candlestick()` API update**: Removed explicit `width`/`height` parameters. New signature is `candlestick(candles, up_color, down_color)`.
- **`button_colored()` layout stability**: Unified label format to `[ label ]` and removed custom-bg-dependent text width/style branching that caused focus/hover layout shifts.
- **`demo_trading` stability**: Migrated to the new `candlestick()` API, added `Esc` quit handling, fixed right column/bottom panel heights, and set `page_size = 5` on all synced tables.

## [0.12.4] — 2026-03-16

### Features

- **`WidgetColors`**: New per-widget color override system. Pass `&WidgetColors` to any `_colored()` variant to override theme colors on individual widgets. Theme remains the default fallback.
  ```rust
  let red = WidgetColors::new().fg(Color::White).bg(Color::Red).accent(Color::LightRed);
  ui.button_colored("Delete", &red);
  ```

- **Per-widget `_colored()` methods**: 10 widgets now support individual color customization:
  - `button_colored()`, `text_input_colored()`, `checkbox_colored()`, `toggle_colored()`
  - `progress_bar_colored()`, `tabs_colored()`, `select_colored()`, `radio_colored()`
  - `list_colored()`, `table_colored()`

- **Text size/margin setters**: Text and link elements now support size constraints and margin via style chaining:
  ```rust
  ui.text("hello").w(20).m(1);
  ui.text("padded").mx(2).min_w(10);
  ```
  New methods: `w()`, `h()`, `min_w()`, `max_w()`, `min_h()`, `max_h()`, `m()`, `mx()`, `my()`, `mt()`, `mr()`, `mb()`, `ml()`

- **Color light variants**: 8 new ANSI bright color variants:
  `DarkGray`, `LightRed`, `LightGreen`, `LightYellow`, `LightBlue`, `LightMagenta`, `LightCyan`, `LightWhite`

## [0.12.3] — 2026-03-16

### Chart Rendering Engine Overhaul

Design principle: **"Chart = content, Container = chrome"** — charts render plot area + data decorations; containers handle borders, padding, and titles. Eliminates the Tailwind-style API conflict where chart frames duplicated container borders.

#### Breaking Default Changes
- **`frame_visible` defaults to `false`**: Charts no longer draw their own `┌─┐`/`└─┘` border frame. Use container `.bordered()` for borders. Opt back in with `c.frame(true)`.
- **Histogram title removed**: `histogram()` no longer renders a hardcoded "Histogram" title row. Use container `.title("Histogram")` instead.

#### Rendering Quality
- **X-axis integrated rendering**: Axis tick line and labels merged into a single row — saves 1 row of overhead per chart. ~33% more plot area for small charts.
- **Smarter tick algorithm for small charts**: Plot heights < 4 rows gracefully degrade to min/max boundary ticks instead of producing broken or missing labels. Heights 4–14 allow denser tick spacing (1 row per interval vs 2).
- **Subtler grid**: Default grid color changed from dim white to `Color::Indexed(238)` — grid dots no longer compete visually with data points.
- **Y-label truncation fix**: Vertical y-axis labels (ylabel) hidden when plot area is too short to render them meaningfully, preventing garbled single-character display.

#### Bar Chart Overhaul (ratatui-inspired)
- **Horizontal sub-cell precision**: Bars now use `▏▎▍▌▋▊▉█` for 8x resolution instead of full `█` blocks. Applies to `bar_chart()`, `bar_chart_styled()`, and `bar_chart_grouped()`.
- **`Bar::text_value()`**: Custom display text per bar (e.g., `Bar::new("Q1", 72.0).text_value("72%")`). Falls back to `format_compact_number()` when unset.
- **`Bar::value_style()`**: Override value label styling per bar.
- **`BarChartConfig` builder**: New `bar_chart_with()` and `bar_chart_grouped_with()` APIs with `bar_width`, `bar_gap`, `group_gap`, `max_value`, and `direction` controls.
- **Wide vertical bars**: `bar_width > 1` renders multi-cell bars. `bar_width >= 3` embeds value text inside the bar with inverted colors (ratatui pattern).

#### Bug Fixes
- **Tab click off-by-one**: `tabs()` widget had `interaction_count` incremented 68 lines after capture (all other widgets increment immediately). Caused `prev_hit_map` to reference the wrong rect, making some tabs unclickable.

#### Demo
- **demo_infoviz tabbed layout**: 4-tab navigation (Overview / Lines / Bars / Advanced). Overview shows all chart types at a glance. Detail tabs give each chart full height (~16 rows plot area vs previous 2-3 rows). Bars tab showcases `bar_chart_with(bar_width=3)`, `bar_chart_grouped_with(group_gap=2)`, and `Bar::text_value()`.

## [0.12.2] — 2026-03-16

### Refactor
- **chart.rs split into 6 modules**: `chart.rs` (533 lines) + `chart/render.rs` (485), `chart/grid.rs` (228), `chart/braille.rs` (184), `chart/bar.rs` (181), `chart/axis.rs` (161). No API or logic changes — pure file reorganization following Rust 2018 module pattern.

## [0.12.1] — 2026-03-16

### Chart System Overhaul — matplotlib-level customization

#### New Chart Types
- **`GraphType::Area`**: Area fill rendering — fills below the line to baseline with braille dots. Use via `c.area(&data)` or `area_chart()` / `area_chart_colored()`.
- **`candlestick(candles, w, h, up_color, down_color)`**: OHLC candlestick chart using `│` (wick) and `█` (body) block characters with automatic Y-axis scaling.
- **`heatmap(data, w, h, low_color, high_color)`**: 2D data grid rendered as colored `█` blocks with RGB color blending.

#### New Chart Customization
- **Manual ticks**: `c.xticks(&[0.0, 5.0, 10.0])`, `c.yticks(&[...])` — override auto-computed tick positions.
- **Tick labels**: `c.xtick_labels(&[0.0, 6.0, 11.0], &["Jan", "Jul", "Dec"])` — custom text labels at tick positions.
- **Reference lines**: `c.axhline(50.0, style)`, `c.axvline(5.0, style)` — horizontal/vertical reference lines with custom styling.
- **Direction coloring**: `c.line(&data).color_by_direction(green, red)` — per-segment up/down coloring for price charts.
- **Style overrides**: `c.title_style(style)`, `c.grid_style(style)`, `c.x_axis_style(style)`, `c.y_axis_style(style)`.
- **Visibility toggles**: `c.frame(false)`, `c.x_axis_visible(false)`, `c.y_axis_visible(false)` — hide frame, axes independently.

#### New Convenience Methods
- **`line_chart_colored(data, w, h, color)`**: Line chart with custom color (vs theme.primary).
- **`area_chart(data, w, h)`**: Filled area chart with theme color.
- **`area_chart_colored(data, w, h, color)`**: Filled area chart with custom color.

### New Types
- **`Candle`**: `{ open, high, low, close }` for candlestick data.

## [0.12.0] — 2026-03-16

### Features
- **Custom Backend API**: `pub trait Backend { size, buffer_mut, flush }` — implement custom rendering targets (WebGL, egui, SSH, test harnesses). Pair with `AppState` and `slt::frame()` to drive the render loop from external event loops.
- **`streaming_markdown()`**: New widget combining streaming text with markdown rendering — headings, bold, italic, inline code, bullet lists, code blocks with blinking cursor during streaming.

### Bug Fixes
- **`confirm()` hook panic on tab switch**: Removed internal `use_state()` from `confirm()` widget — was the only widget using internal hooks, causing panic when conditionally rendered across tab switches. Now uses the `result: &mut bool` parameter directly for selection state.

### Improvements
- **`parse_inline_segments()` visibility**: Changed from private to `pub(crate)` — enables inline markdown formatting reuse across widget modules.
- **README architecture section**: Added Custom Backends guide with code example and AI-Native Widgets table.

## [0.11.0] — 2026-03-16

### BREAKING: Response Pattern
- **All widgets return `Response`**: `button()`, `checkbox()`, `toggle()`, `list()`, `table()`, `tabs()`, `select()`, `radio()`, `multi_select()`, `text_input()`, `textarea()`, `accordion()`, `alert()`, `tool_approval()`, and all display/viz widgets now return `Response { clicked, hovered, changed, focused, rect }` instead of `bool`, `&mut Self`, or `()`.
- **Migration**: `if ui.button("x") {` → `if ui.button("x").clicked {`; `ui.checkbox("x", &mut v);` → check `.changed` field.
- `text()`, `styled()`, `link()` are unchanged — they still return the text builder for `.bold().fg()` chaining.
- `command_palette()` is unchanged — still returns `Option<usize>`.

### New Widgets
- **`slider("label", &mut value, range)`**: Horizontal slider for numeric input. Left/Right/h/l to adjust, returns `Response` with `.changed`.
- **`confirm("question?", &mut bool)`**: Yes/No button pair. y/n shortcuts, Tab to switch focus. Returns `Response` with `.clicked` when answered.
- **`file_picker(&mut FilePickerState)`**: Directory browser with Enter to navigate, Backspace to go up, extension filter, hidden file toggle.
- **`notify("message", ToastLevel)`**: App-level toast notification — no external `ToastState` needed. Auto-dismisses after ~3 seconds.
- **`help_from_keymap(&KeyMap)`**: Renders help bar automatically from a `KeyMap` struct.

### New Types
- **`KeyMap`** + **`Binding`**: Declarative key binding management with builder pattern. `.bind('q', "quit")`, `.bind_code(KeyCode::Up, "up")`, `.bind_mod('s', KeyModifiers::CONTROL, "save")`, `.bind_hidden(...)`.
- **`FilePickerState`** + **`FileEntry`**: State for the file picker widget.
- **`Palette`**: Color palette struct with 11 shades (c50–c950).
- **`palette::tailwind`**: 22 Tailwind CSS color palettes (slate through rose) as `const` values. Usage: `slt::palette::tailwind::BLUE.c500`.

### New Features
- **`TextInputState::set_suggestions()`**: Autocomplete dropdown with prefix matching. Tab accepts, Up/Down navigates, Esc closes.
- **`TextInputState::add_validator()`**: Multiple validators with multi-error collection. `.errors()` returns all validation errors.
- **`Context::light_dark(light, dark)`**: Returns the appropriate color based on current theme's dark/light mode.
- **`ListState::set_items()`**: Safe item replacement with automatic view index rebuild.
- **`Rect` helpers**: `.centered(w,h)`, `.union()`, `.intersection()`, `.contains(x,y)`, `.rows()`, `.positions()`.

### Bug Fixes
- **`use_memo` panic messages**: Now include hook index and expected type name (matching `use_state` quality).
- **InlineTerminal background**: `flush()` now respects `theme_bg` via `reset_with_bg()`.
- **`Color::blend()` rounding**: Changed truncation (`as u8`) to rounding (`.round() as u8`). `blend(White, Black, 0.5)` now correctly returns `(128,128,128)`.
- **README signature fixes**: `stat()`, `key_hint()`, `code_block()`, `accordion()` examples corrected.
- **ListState direct mutation crash**: `pkg_list.items = items` without rebuild caused stale view indices. Fixed with `set_items()`.

### Improvements
- **Re-exports**: Easing functions (`ease_in_quad`, `ease_out_bounce`, etc.), `ContainerBuilder`, `Cell`, `Direction`, `Palette` now exported from crate root.
- **Default impls**: `ListState`, `TabsState`, `TableState`, `SelectState`, `RadioState`, `MultiSelectState`, `TreeState`, `CommandPaletteState`, `ToolApprovalState` all implement `Default`.
- **Refactoring**: `table()` (229→3 helpers), `select()` (138→2 helpers), `bar_chart_styled()` (228→2 helpers) split into smaller functions. Vertical nav pattern extracted into shared `handle_vertical_nav()` from 7 widgets.

### Demo
- Consolidated 19 → 14 examples. Removed debug tools (`test_mouse`, `debug_selection`). Absorbed `demo_table` and `demo_ime` into main demo.
- New "v0.11.0" tab in `demo.rs` showcasing all new features.
- All help bars now correctly show `Ctrl+Q` / `Ctrl+T` modifiers.

## [0.10.1] — 2026-03-16

### Performance
- **Cell.symbol**: `String` → `CompactString` — eliminates heap allocation for ≤24-byte symbols (99%+ of terminal cells). Same approach as ratatui.
- **Cell.hyperlink**: `Option<String>` → `Option<CompactString>` — reduces per-cell overhead for hyperlinks.
- **diff+flush inline**: Removed intermediate `Vec<(u32, u32, &Cell)>` allocation in `Terminal::flush()`. Now diffs and writes to stdout in a single pass.
- **reset_with_bg()**: Theme background applied during buffer reset instead of a separate O(w×h) loop per frame.

### Changes
- **MSRV**: 1.74 → 1.81 (required by `compact_str` 0.9)
- **New dependency**: `compact_str` 0.9 (no-default-features) — adds 4 small transitive deps (castaway, ryu, static_assertions, rustversion)

## [0.10.0] — 2026-03-15

### Bug Fixes
- **error_boundary terminal recovery**: panic hook fires before `catch_unwind`, destroying terminal state. Now re-enters raw mode + alternate screen after catching the panic.
- **error_boundary rollback scope**: previously only restored 2 fields (`commands`, `last_text_idx`). Now captures and restores all 13 mutable per-frame fields via `ContextSnapshot` — prevents focus/hook/modal/group state corruption after caught panics.
- **`Theme::light()` dark_mode**: `dark_mode` was hardcoded to `true` regardless of theme. Now reads `theme.is_dark`.

### New API
- **`consume_key(c)` / `consume_key_code(code)`**: explicitly consume a key event, preventing widgets from handling it. Unlike `key()`/`key_code()` which peek without consuming.

### Theme
- **`Theme.is_dark`**: new `pub is_dark: bool` field on `Theme`. All 7 built-in presets set it correctly. `ThemeBuilder` supports `.is_dark(bool)`.
- **`Theme::light()` redesign**: Tailwind slate-based high-contrast palette — `Rgb(15,23,42)` text on `Rgb(248,250,252)` background, blue-600 primary, proper contrast for success/warning/error.
- **Default text color**: `ui.text()` now defaults fg to `theme.text` instead of terminal default. Ensures readability in light mode.
- **Root background fill**: screen background filled with `theme.bg` when not `Color::Reset`.

### DX
- **`#[must_use]` message**: `ContainerBuilder` warning now says "does nothing until .col(), .row(), .line(), or .draw() is called"
- **Documentation fixes**: RunConfig docs corrected from 100ms to 16ms (60fps), README `docs.rs/slt` → `docs.rs/superlighttui`, border style count 4 → 6, removed dead `demo_v050` reference.
- **Clippy clean**: `cargo clippy --all-targets --all-features -- -D warnings` now passes (fixed `collapsible_if`, `field_reassign_with_default`, `saturating_sub`, `if_same_then_else`, `too_many_arguments`, `len_zero`).

### Demo
- Theme-aware colors: hardcoded `Color::Green`/`Color::Red`/`Color::Cyan` replaced with `theme.success`/`theme.error`/`theme.primary` for proper light/dark mode rendering.

## [0.9.5] — 2026-03-15

### Tests
- 15 new widget tests: divider_text, alert (render + dismiss), breadcrumb, accordion (open/closed), badge (render + colored bg), key_hint (reversed), stat (render + trend arrow), definition_list, empty_state, code_block (render + numbered)

### Improvements
- **code_block theme auto-switch**: syntax highlighting adapts to dark/light theme — dark uses One Dark palette, light uses One Light
- **Syntax highlighting multi-language**: keywords for Python, JavaScript, Go added alongside Rust
- **breadcrumb Outline style**: segments use `ButtonVariant::Outline` for cleaner navigation look
- **widgets_viz.rs split**: 3012 → 884 lines. Interactive widgets (list, table, tabs, button, etc.) extracted to `widgets_interactive.rs` (2132 lines)
- **demo_dashboard upgraded**: uses `divider_text`, `stat_trend`, `stat_colored`, `badge_colored`

### Documentation
- README.md updated with v0.9.0-v0.9.4 features
- SLT skill updated with new widget API docs

## [0.9.4] — 2026-03-15

### Features — 10 New Widgets

**Tier 1 (not composable from primitives):**
- **`divider_text(label)`**: horizontal rule with centered text label — `──── Settings ────`
- **`alert(message, AlertLevel)`**: persistent inline notification with icon + dismiss — returns `true` when dismissed
- **`breadcrumb(&["Home", "Settings"])`**: clickable path navigation — returns `Some(idx)` on segment click
- **`accordion(title, &mut open, |ui| { ... })`**: collapsible content section with ▾/▸ toggle

**Tier 2 (convenience widgets):**
- **`badge(label)` / `badge_colored(label, color)`**: inline colored tag with auto-contrast foreground
- **`key_hint(key)`**: inline keyboard shortcut display — `[Ctrl+S]` reversed style
- **`stat(label, value)` / `stat_colored` / `stat_trend`**: dashboard metric with optional trend arrow ↑↓
- **`definition_list(&[("key", "value")])`**: auto-aligned key-value pairs
- **`empty_state(title, desc)` / `empty_state_action`**: centered placeholder for empty lists
- **`code_block(code)` / `code_block_numbered`**: bordered code display with optional line numbers

### New Types
- `AlertLevel` enum: `Info`, `Success`, `Warning`, `Error`
- `Trend` enum: `Up`, `Down`

## [0.9.3] — 2026-03-15

### Refactoring
- **Run loop deduplication**: extracted `run_frame()` generic over `TerminalBackend` trait — 3 near-identical ~300-line loops replaced with 1 shared frame function + 3 thin wrappers. `lib.rs` reduced from 940 to 732 lines
- **FrameState struct**: bundled 15+ per-frame local variables into `FrameState`, eliminating `Context::new()` 17-parameter constructor and removing `#[allow(clippy::too_many_arguments)]`
- **TerminalBackend trait**: `Terminal` and `InlineTerminal` now implement a shared trait with `size()`, `buffer_mut()`, `flush()`, `handle_resize()`
- **style.rs split**: extracted `style/color.rs` (Color enum + ColorDepth, 316 lines) and `style/theme.rs` (Theme + ThemeBuilder, 353 lines). `style.rs` reduced from 1429 to 765 lines
- **ContainerBuilder field unification**: renamed `bg_color` → `bg`, `dark_bg_color` → `dark_bg` to match `ContainerStyle` field names

## [0.9.2] — 2026-03-15

### Features
- **`gap_at(bp, value)`**: unified breakpoint API — `ui.container().gap_at(Md, 2)` replaces `ui.container().md_gap(2)`. Added 7 `_at` methods: `gap_at`, `w_at`, `h_at`, `min_w_at`, `max_w_at`, `grow_at`, `p_at`. Existing methods kept for backward compatibility

### Performance
- **String clone elimination**: `ContainerBuilder::finish()` changed to `mut self`, replacing `group_name.clone()` with `group_name.take()` — eliminates one heap allocation per container per frame

### Refactoring
- **context.rs split** (6527 → 2163 lines): widget methods extracted to `context/widgets_display.rs` (896), `context/widgets_input.rs` (540), `context/widgets_viz.rs` (3012)
- **layout.rs split** (2294 → 1411 lines): flexbox algorithm extracted to `layout/flexbox.rs` (343), rendering to `layout/render.rs` (548)
- **terminal.rs split** (1044 → 880 lines): selection logic extracted to `terminal/selection.rs` (175)

## [0.9.1] — 2026-03-15

### Bug Fixes
- **draw_raw focus_id**: `pending_focus_id.take()` was called twice in `RawDraw` node creation — second call clobbered the first with `None`, breaking `FocusMarker` on draw_raw regions

### Improvements
- **Hook panic messages**: `use_state` type mismatch now reports hook index and expected type name (`use_state type mismatch at hook index 3 — expected i32`) instead of bare `"use_state type mismatch"`
- **draw_raw docs**: added `'static` bound explanation with workaround code example to `ContainerBuilder::draw()` rustdoc

### Tests
- 7 new draw_raw tests: `draw_raw_with_grow_fills_available_width`, `draw_raw_alongside_normal_widgets`, `draw_raw_with_fixed_size`, `draw_raw_styled_content`, `draw_raw_multiple_regions`, `collect_all_focus_rects_match_tab_navigation`, `collect_all_scroll_works_after_merge`

## [0.9.0] — 2026-03-15

### Features
- **`draw_raw()`**: direct buffer access via `ContainerBuilder::draw()` — write to `&mut Buffer` with computed `Rect` after layout. Clip protection prevents writes outside allocated area. Enables custom renderers, game-like effects, and protocol visualizers without the Command pipeline overhead
- **`Buffer` and `Rect` re-exported**: `slt::Buffer` and `slt::Rect` now available at crate root for `draw_raw()` users

### Performance
- **7× fewer tree traversals per frame**: merged 7 independent `collect_*` functions into a single `collect_all()` DFS pass returning a `FrameData` struct — 1000-node trees go from 7000 to 1000 node visits per frame
- **Keyframes: zero allocations per frame**: `Keyframes::value()` no longer clones+sorts the stop list every frame — stops are sorted once at construction time via `stop()` builder
- **Delta-based style flushing**: `terminal::flush()` now emits only changed attributes (fg/bg/modifiers) instead of full `ResetColor + SetAttribute(Reset) + apply_style()` on every style transition — reduces escape sequences by ~50% for typical UIs

### Internal
- Removed 204 lines of dead `collect_*` code after merge
- Added `FrameData` struct and `collect_all()` to layout.rs
- Added `RawDrawCallback` type alias for deferred draw closures
- 3 new tests: `draw_raw_renders_to_buffer`, `draw_raw_respects_constraints`, `draw_raw_clips_outside_rect`
- New example: `demo_raw_draw` showcasing gradient, plasma, and box drawing effects

## [0.8.4] — 2026-03-15

### Bug Fixes
- **Tabs empty labels crash**: guard modulo-by-zero when `TabsState::new(vec![])` — no longer panics
- **Sparkline div-by-zero**: already guarded (verified, no change needed)

### Improvements
- **`State<T>`**: now `Copy + Clone + Debug + PartialEq + Eq` — pass by value, no `&` needed
- **`ContainerStyle`**: now `Copy` — eliminates unnecessary `.clone()` calls
- **`ContainerStyle`**: added `min_h()`, `max_h()`, `w_pct()`, `h_pct()` builder methods
- **`full` feature flag**: `features = ["full"]` enables async + serde + image
- **docs.rs metadata**: `all-features = true` — async/serde/image APIs now visible on docs.rs

## [0.8.3] — 2026-03-15

### Features
- **ContainerStyle**: reusable composable style recipes — `const CARD: ContainerStyle = ContainerStyle::new().border(Border::Rounded).p(1)` + `ui.container().apply(&CARD)`
- **Rustdoc examples**: added `/// # Example` sections to `modal`, `group`, `use_state`, `use_memo`, `apply`

### Bug Fixes
- **Markdown Korean panic**: `parse_inline_segments` used byte indices on char-indexed positions — panicked on multi-byte CJK text (`**bold**` with Korean). Now uses char-based string operations
- **Example warnings**: removed unused variables and dead code in demo, demo_cli

## [0.8.2] — 2026-03-15

### Features
- **IME cursor always visible**: text_input/textarea cursor no longer blinks — always shown when focused, enabling OS IME popup to anchor correctly for Korean/CJK input
- **text_input horizontal scroll**: long text scrolls within container bounds instead of overflowing — CJK double-width aware via unicode-width

### Added
- `demo_ime.rs` example for Korean/CJK input testing

## [0.8.1] — 2026-03-15

### Bug Fixes
- **ListState filter rendering**: `list()` now renders only filtered items via `view_indices` — previously `set_filter()` updated indices but rendering ignored them
- **ThemeBuilder export**: `ThemeBuilder` now exported from `slt::ThemeBuilder` — was inaccessible in v0.8.0

### Removed
- **Pie chart**: `pie_chart()` removed — not practical for terminal display
- **Area chart**: `GraphType::Area` and `ChartBuilder::area()` removed

### Improvements
- Add rustdoc to group hover/focus public API methods
- Demo: add group hover and use_memo sections to v0.8.0 tab
- Demo: interactive theme builder with Coral/Ocean/Forest presets
- Demo: all keyboard shortcuts changed to Ctrl+key to prevent input conflicts
- Other demos (spreadsheet, dashboard, cli): same Ctrl+key migration

## [0.8.0] — 2026-03-14

### Features
- **Hooks**: `use_state()` / `use_memo()` — React-style persistent state with `State<T>` handle pattern
- **Dark mode prefix**: `dark_bg()`, `dark_border_style()` — conditional container styles for dark/light modes
- **Responsive variants**: `xs_w()` through `xl_w()`, `_h`, `_min_w`, `_max_w`, `_gap`, `_p`, `_grow` (35 methods) — breakpoint-conditional layout
- **Group hover/focus**: `ui.group("card").col(...)` with `group_hover_bg()` — parent hover state affects children
- **Theme builder**: `Theme::builder().primary(Color::Red).build()` — 15-field builder with dark defaults
- **ListState filter**: `list.set_filter("rust")` — multi-token AND matching (same as TableState)
- **Animation callbacks**: `.on_complete()` for Tween/Keyframes/Sequence/Stagger, `.on_settle()` for Spring
- **Scatter plot**: `ui.scatter(&data, w, h)` — standalone braille scatter chart

### Changed
- Demo example: added "v0.8.0" tab showcasing all new features

## [0.7.2] — 2026-03-14

### Changed
- **Multi-token command palette filter**: `CommandPaletteState` search now uses the same multi-token AND logic as `TableState` — e.g. `"save buffer"` matches commands where label contains "save" and description contains "buffer"

### Fixed
- Register `demo_fire` and `demo_game` examples in `Cargo.toml`

### Added
- VHS tape file for DOOM fire demo recording (`demo_fire.tape`)

## [0.7.1] — 2026-03-14

### Changed
- **Multi-token table filter**: `TableState::set_filter` now splits input by whitespace and matches all tokens (AND logic) across any cell in a row — e.g. `"ERROR deploy"` matches rows where one cell contains "error" and another contains "deploy"

## [0.7.0] — 2026-03-14

### Features
- **Dashed borders**: `Border::Dashed` and `Border::DashedThick` variants for dashed/heavy-dashed box drawing
- **Kitty keyboard protocol**: `RunConfig { kitty_keyboard: true }` enables key release/repeat events via `KeyEventKind` — silently ignored on unsupported terminals
- **Color auto-downsampling**: `ColorDepth` enum with auto-detection from `$COLORTERM`/`$TERM`; `Color::downsampled()` converts RGB to 256/16-color; `RunConfig { color_depth }` for override
- **Scrollbar widget**: `ui.scrollbar(&scroll)` renders proportional thumb alongside `scrollable()` containers
- **Responsive breakpoints**: `Breakpoint` enum (`Xs`/`Sm`/`Md`/`Lg`/`Xl`) with `ui.breakpoint()` for terminal-width-adaptive layouts
- **OSC 52 clipboard API**: `ui.copy_to_clipboard(text)` writes to system clipboard via OSC 52 (works over SSH)
- **Enhanced DevTools overlay**: F12 now shows widget count, frame time, FPS, and terminal dimensions
- **Half-block image widget**: `HalfBlockImage` renders images at 2× vertical resolution using `▀` characters; `from_rgb()` always available, `from_dynamic()` behind `image` feature flag
- **AI native widgets**: `streaming_text()` with blinking cursor, `tool_approval()` with approve/reject buttons, `context_bar()` with token counts

### New Types
- `KeyEventKind` — `Press`, `Release`, `Repeat`
- `ColorDepth` — `TrueColor`, `EightBit`, `Basic`
- `Breakpoint` — `Xs`, `Sm`, `Md`, `Lg`, `Xl`
- `HalfBlockImage` — terminal-renderable image grid
- `StreamingTextState` — streaming text accumulator
- `ToolApprovalState` / `ApprovalAction` — tool call approval
- `ContextItem` — context bar entry with token count

### New Methods
- `Color::downsampled(ColorDepth)` — downsample to target depth
- `ColorDepth::detect()` — auto-detect from environment
- `ScrollState::content_height()`, `viewport_height()`, `progress()`
- `Context::scrollbar(&ScrollState)` — vertical scrollbar
- `Context::breakpoint()` — responsive width class
- `Context::copy_to_clipboard(text)` — OSC 52 clipboard
- `Context::image(&HalfBlockImage)` — half-block image render
- `Context::streaming_text(&mut StreamingTextState)` — streaming text
- `Context::tool_approval(&mut ToolApprovalState)` — tool approval widget
- `Context::context_bar(&[ContextItem])` — context bar
- `Context::key_release(char)`, `key_code_release(KeyCode)` — key release detection

### Feature Flags
- `image` — enables `HalfBlockImage::from_dynamic()` (adds `image` crate dependency)

## [0.6.1] — 2026-03-14

### Features
- **Table sorting**: click column header to sort ASC/DESC with ▲/▼ indicator — numeric sort when both values parse as numbers, lexicographic otherwise
- **Table filtering**: `set_filter()` applies case-insensitive substring match across all cells
- **Table pagination**: `page_size` field enables paged display with PageUp/PageDown navigation and "Page X/Y" footer
- **Rich text `line()`**: inline row with gap-0 for composing styled text segments
- **Rich text `line_wrap()`**: segment-aware word wrapping that preserves style boundaries
- **Markdown inline styles**: `markdown()` now renders **bold**, *italic*, and `code` with actual terminal styles

### New Methods on `TableState`
- `toggle_sort(column)` — sort by column, click again to reverse
- `sort_by(column)` — sort ascending by column
- `clear_sort()` — remove sorting
- `set_filter(text)` — filter visible rows
- `next_page()` / `prev_page()` — page navigation
- `total_pages()` — total page count
- `visible_indices()` — filtered + sorted row indices

### New Example
- `demo_table` — interactive showcase for table sorting, filtering, and pagination with 20-row dataset

## [0.6.0] — 2026-03-14

### Features
- **Select/Dropdown widget**: `select()` with `SelectState` — collapsible dropdown with keyboard and mouse support
- **Radio buttons**: `radio()` with `RadioState` — mutually exclusive option group with ●/○ markers
- **Multi-select**: `multi_select()` with `MultiSelectState` — checkbox-style [x]/[ ] selection with Space toggle
- **Tree view**: `tree()` with `TreeNode`/`TreeState` — hierarchical expandable tree with ▾/▸ icons
- **Virtual list**: `virtual_list()` — renders only visible items for large datasets with ↑/↓ indicators
- **Command palette**: `command_palette()` with `CommandPaletteState` — modal search overlay with fuzzy filtering
- **Markdown rendering**: `markdown()` — renders headings (#/##/###), bold, italic, lists, code, and horizontal rules
- **Key sequences**: `key_seq("gg")` — matches multi-character key sequences within a single frame
- **Password masking**: `TextInputState.masked` — displays input as `•` characters
- **Percentage-based sizing**: `w_pct()` / `h_pct()` — set container width/height as percentage of parent
- **Per-side borders**: `border_top()`, `border_right()`, `border_bottom()`, `border_left()`, `border_sides()` — show/hide individual border sides with `BorderSides` type

### Improvements
- 30+ widgets total (up from 20+)
- New state types exported: `SelectState`, `RadioState`, `MultiSelectState`, `TreeNode`, `TreeState`, `CommandPaletteState`, `PaletteCommand`

## [0.5.1] — 2026-03-14

### Documentation
- Added module-level rustdoc (`//!`) to all 10 public modules
- Documented `EventBuilder` and `TestBackend` public API in `test_utils`
- Documented `Direction` enum variants in `layout`
- Documented `max_length` fields and methods in `TextInputState` / `TextareaState`
- Removed hardcoded line count from README Architecture section

### Bug Fixes
- Fixed RNG infinite loop in demo_game — replaced LCG with xorshift64 for all 3 games
- Fixed MSRV clippy error — replaced `is_multiple_of(2)` with `% 2 == 0` (requires Rust 1.74+)
- Fixed game layout — nav pinned to top, game content centered vertically

## [0.5.0] — 2026-03-14

### Features
- **Design system overhaul**: container `bg()` now propagates to child text, borders, titles, and scroll indicators — no more split background/text rendering
- **Theme expansion**: added `surface_text`, `surface_hover` fields to Theme struct for readable text on elevated surfaces
- **5 new themes**: Dracula, Catppuccin, Nord, Solarized Dark, Tokyo Night (total 7 built-in themes)
- **Color utilities**: `Color::luminance()`, `Color::contrast_fg()`, `Color::blend()`, `Color::lighten()`, `Color::darken()`
- **Focus events**: `Event::FocusGained` / `Event::FocusLost` for terminal focus tracking; hover clears on focus loss
- **New widgets**: `button_with()` variants (Primary, Secondary, Danger, Ghost, Outline), `form_field()`, `form_submit()`, `bar_chart_grouped()`, `histogram()`, `line_chart()`, `bar_chart_styled()`
- **Justify modes**: `SpaceBetween`, `SpaceAround`, `SpaceEvenly` for flexbox-style distribution
- **Links**: `ui.link()` renders OSC 8 clickable hyperlinks
- **Canvas**: braille-based vector drawing with `line()`, `circle()`, `rect()`, `point()`
- **Animation**: `Sequence` chaining, `Stagger` for list animations, `LoopMode::PingPong`
- **Snapshot testing**: `TestBackend::to_string_trimmed()` for insta-based UI regression tests

### Bug Fixes
- Container background now correctly inherits to border characters, title text, and scroll indicators
- Modal centering respects `min_width` / `max_width` constraints
- Hover state properly clears when terminal loses focus (via `EnableFocusChange`)

### Improvements
- Demo example fully redesigned: tabbed navigation, theme-aware cards, all widgets showcased
- Demo website example: surface_text applied for readable text on colored backgrounds
- 4 new regression tests for background color inheritance
- 162 total tests passing

## [0.4.1] — 2025-12-26

### Features
- IME/Korean input support for text_input and textarea
- Text selection with mouse drag (border cell exclusion)
- Click-to-focus for interactive widgets

## [0.3.0] — 2025-12-21

### Features
- Data visualization: chart, histogram, bar_chart, sparkline
- Grid layout
- Error boundary with panic recovery
- Serde support (optional feature)
- Viewport culling for off-screen widgets
- FPS cap via `RunConfig::max_fps`

## [0.2.2] — 2025-12-18

### Features
- TestBackend for headless rendering
- Synchronized output (DECSET 2026)
- State safety improvements

## [0.2.0] — 2025-12-15

### Features
- Initial public release
- Immediate-mode API with row/col layout
- 15+ built-in widgets
- Double-buffer diff rendering
- Dark and light themes
