# SLT References

Targets SLT v0.25.0.

Load this when the user asks about feature flags, version-specific APIs, or doc locations.

## Feature matrix (from `Cargo.toml`)

| Feature | Default? | Purpose | APIs / capabilities enabled |
|---|---|---|---|
| `crossterm` | yes | Terminal backend | `slt::run`, `slt::run_with`, `slt::run_inline`, `slt::run_static`, `detect_color_scheme`, `read_clipboard`. Mouse capture is off unless `RunConfig::default().mouse(true)` |
| `bidi` | yes | UAX #9 text reordering | correct RTL and mixed-direction buffer rendering |
| `async` | no | Tokio integration | `slt::run_async` (channel-based message pump) |
| `serde` | no | Serialize/Deserialize | derives on `Style`, `Color`, `Theme`, layout types |
| `theme-watch` | no | TOML hot reload | `ThemeWatcher`; enables `serde` and `notify` |
| `image` | no | Image loading helpers | `ui.image(...)` halfblock/kitty/sixel decode paths |
| `qrcode` | no | QR widget | `ui.qr_code(...)` |
| `syntax-rust` / `syntax-python` / `syntax-javascript` / `syntax-typescript` / `syntax-go` / `syntax-bash` / `syntax-json` / `syntax-toml` / `syntax-c` / `syntax-cpp` / `syntax-java` / `syntax-ruby` / `syntax-css` / `syntax-html` / `syntax-yaml` | no | Per-language tree-sitter grammars | `ui.code_block(code, "rust")` etc. |
| `syntax` | no | All `syntax-*` combined | every language above |
| `kitty-compress` | no | zlib-compressed kitty protocol | larger images, smaller payloads |
| `ratatui` | no | ratatui widget bridge (`ratatui-core` 0.1, no backend) | `ContainerBuilder::ratatui` / `ratatui_widget`, `From` conversions for `Color` / `Modifiers` / `Style`; not in `full` |
| `full` | no | crossterm+async+serde+theme-watch+image+qrcode+kitty-compress+bidi | development / demos only; excludes syntax grammars |

## Runtime defaults that change behavior

- **Mouse is opt-in.** `RunConfig::default()` has `mouse: false`. Without `slt::run_with(RunConfig::default().mouse(true), ...)`, `clicked` / `hovered` / `right_clicked` / `double_clicked` / `scroll_delta`, scrollbar drag and split-pane drag never fire in a real terminal.
- **Focused inputs own their keys.** `ui.key('q')` / `ui.shortcut(..)` skip the keys a focused `text_input` / `textarea` edits with, in any call order; use `ui.raw_key(c)` to see every press.
- **`ui.modal` does not trap focus.** It keeps `tab_trap: false`; `modal_with(ModalOptions::default(), ..)` traps.
- **`RunConfig::handle_ctrl_c(true)`** is the default; pass `false` to receive Ctrl-C yourself.

## v0.24 / v0.25 additions

- `ScrollState::follow_focus: bool` (public field, default `true`) — scroll containers reveal the focused child, including nested axes and modal forms. Set `false` for app-controlled scrolling. `src/widgets/collections.rs`.
- `Context::form_field_response(&mut FormField) -> FormFieldResponse` — `{ response, input, layout_rect: Option<Rect> }`, `Deref<Target = Response>`. `form_field` stays chainable. `src/context/widgets/text_input.rs`, `src/widgets/text_input.rs`.
- `Context::focused_layout_rect() -> Option<Rect>` and `Context::measured_layout_rect(name) -> Option<Rect>` — previous-frame full border box before scroll/clip. `Response.rect` stays the clipped hit rect. `src/context/helpers.rs`.
- `TestBackend::has_pending_input()` / `AppState::has_pending_input()` — input queued behind a Tab group for the next frame.
- `TestBackend::advance_time(Duration)` — deterministic headless clock (ticks now advance per frame).
- `Context::try_canvas(w, h, draw) -> Result<Response, CanvasError>`, `Context::canvas_with(&mut CanvasContext, draw)` — bounded / reusable canvas storage.
- Browser runtime: `crates/slt-wasm` with `WasmOptions`; see `docs/WASM.md`.

## v0.20 API surface (highlights AI agents miss)

These shipped in v0.20 and are NOT in older docs. Every entry verified against `src/`.

### Builders (Rule 1)
- `ui.gauge(ratio: f64) -> Gauge<'_>` — `.label(s).width(n).color(c).show() -> GaugeResponse`. See `src/context/widgets/feedback/gauge.rs`.
- `ui.line_gauge(ratio: f64) -> LineGauge<'_>` — `.label(s).filled(c).empty(c).width(n).show()`. See `src/context/widgets/feedback/gauge.rs`.
- `ui.breadcrumb(&[&str]) -> Breadcrumb<'_>` — `.separator(s).color(c).show() -> BreadcrumbResponse`. See `src/context/widgets/display/status.rs`.

### Opts struct (Rule 3)
- `ui.scrollable_with_gutter(&mut ScrollState, GutterOpts<G>, body)` — `context/widgets/layout/gutter.rs:115`
- `GutterOpts::line_numbers(total, viewport)` — 90% case smart constructor
- `GutterOpts::new(total, viewport, |i| label)` — custom labels
- `HighlightRange::line(i)` / `::span(start, len)` — replaces v0.19 `::single`

### Split panes
- `ui.split_pane(&mut SplitPaneState, left, right) -> SplitPaneResponse` — horizontal split, draggable
- `ui.vsplit_pane(&mut SplitPaneState, top, bottom) -> SplitPaneResponse` — vertical split
- `SplitPaneState::new(0.5)`, `.with_min_ratio(0.2)`

### Hooks
- `ui.use_state(\|\| init)` — order-based, see `src/context/runtime.rs`
- `ui.use_state_named("id", \|\| init)` — `&'static str` + init fn (two args)
- `ui.use_state_named_default::<T>("id")` — `&'static str`, `T: Default`
- `ui.use_state_keyed("id-{i}", \|\| init)` — runtime `String`
- `ui.use_state_keyed_default::<T>("id-{i}")` — runtime `String` + `T: Default`
- `ui.use_memo(&deps, \|d\| compute(d))` — cached compute
- `ui.use_effect(\|d\| { ... }, &deps)` — side effect on deps change
- `ui.provide(value, \|ui\| ...)` — typed scoped injection
- `ui.use_context::<T>()` — panics if missing
- `ui.try_use_context::<T>()` — `Option<&T>`

### Animation shorthand (v0.20)
- `ui.animate_value("id", target, duration_ticks) -> f64` — see `src/context/runtime.rs`
- `ui.animate_bool("id", value) -> f64` — eased 0..1 toggle

### Response signal fields (v0.20)
`Response { clicked, right_clicked, hovered, changed, focused, gained_focus, lost_focus, double_clicked, submitted, scroll_delta, rect }`.
- `Response::on_hover(ui, "tooltip text")` — chained tooltip; see `src/context/state.rs`
- `Response::on_hover_ui(ui, |ui| { ... })` — chained tooltip with custom body

### Theme additions (v0.20)
- `Theme::compact()` / `Theme::comfortable()` / `Theme::spacious()` — density variants of dark
- `Theme::with_spacing(spacing)` — override Spacing on any theme
- `ContainerBuilder::theme(theme)` — per-subtree theme override (no ambient state)
- `Theme::builder()` / `Theme::builder_from(base) -> ThemeBuilder` — `const fn` since v0.19.2

### Layout / DX (v0.20)
- `ContainerBuilder::fill()` — equivalent to `.grow(1)`, more readable
- `Rect::center_in(outer)`, `center_horizontally_in`, `center_vertically_in` — `src/rect.rs`
- `Context::modal_with(slt::context::ModalOptions { tab_trap: true }, \|ui\| ...)` — focus-trapped modal (plain `modal` does not trap)
- `Context::static_log(...)` / `take_static_log()` — for `slt::run_static` scrollback

### Keymap (v0.20)
- `WidgetKeyHelp` trait — implement on a state type to publish keybindings
- `Context::publish_keymap(...)` / `published_keymaps()` — collect + render help
- `Context::keymap_help_overlay(open: bool)` — auto-rendered overlay
- `RunConfig::handle_ctrl_c(bool)` — opt-out of auto Ctrl-C quit

### Accessibility (v0.26)
- `RunConfig::accessible(bool)` / `SLT_ACCESSIBLE=1` — linear prompt mode for screen readers (no screen takeover; widgets become sequential prompts). Explicit call wins over the env var
- `ui.a11y_label("Email")` — accessible name of the next widget; `ui.is_accessible()` — hide visual-only hints
- Details: `docs/TERMINAL_COMPATIBILITY.md#accessibility`

### Testing (v0.20)
- `TestBackend::new(w, h).record_frames()` + `tb.frames()[i]` — frame history
- `tb.sequence().tick(\|ui\| ..).key(KeyCode::Esc, \|ui\| ..).type_string("hi", \|ui\| ..).run()` — every step takes its render closure
- `Buffer::snapshot_format()` — readable snapshot dump
- `tb.assert_not_contains(s)`, `assert_line_not_contains`, `assert_empty_line(y)`, `assert_style_at(x, y, style)`

## v0.20 removed APIs (do NOT use)

These were either v0.19 preview or pre-1.0 mistakes consolidated by the v0.20 API consistency pass. AI training data may suggest them — don't.

| Removed | Replacement |
|---|---|
| `gauge_w(r, w)` | `gauge(r).width(w)` |
| `gauge_colored(r, c)` | `gauge(r).color(c)` |
| `line_gauge_with(r, opts)` | `line_gauge(r).<chain>` |
| `breadcrumb_sep(b, s)` | `breadcrumb(b).separator(s)` |
| `breadcrumb_response(b)` / `breadcrumb_response_with(b, s)` | `breadcrumb(b).show()` (returns `BreadcrumbResponse`) |
| `LineGaugeOpts` | `LineGauge<'_>` builder |
| `HighlightRange::single(i)` | `HighlightRange::line(i)` |
| `label_owned(s)` | `label(s)` (accepts `impl Into<String>`) |

## Doc pointers (cross-check before relying on)

These docs are maintained for the current release, but source and compile-tested examples remain authoritative.

- `docs/COMPLETE_REFERENCE.md` — broad single-file reference.
- `docs/WIDGETS.md` — categorized widget and state reference.
- `docs/STATE_APIS.md` — method-level state reference.
- `docs/PREVIOUS_FRAME_GUIDE.md` — current. Read for `Response.rect` timing.
- `docs/PATTERNS.md` — current. `provide`/`use_context`/`use_state_named`/`with_if` covered.
- `docs/QUICK_START.md` — current.
- `docs/COOKBOOK.md` — composition recipes; run through `cookbook_tour`.
- `docs/THEMING.md` — current. `ThemeBuilder` `const fn` since v0.19.2.
- `docs/TESTING.md` — covers v0.19.1 EventBuilder additions. Read alongside `tests/example_snapshots/testing_report.rs` for the `record_frames` / `sequence` helpers.
- `docs/ANIMATION.md` — current.
- `docs/AI_GUIDE.md` — concise overview, current.
- `docs/BACKENDS.md` — `Backend`, `AppState`, `frame()` low-level paths.
- `docs/ARCHITECTURE.md` — render pipeline.
- `docs/DEBUGGING.md` — F12 layout overlay, debug flags.
- `docs/MIGRATION.md` — version-by-version public API migration guide.
- `docs/llms.txt` — short coding-agent index.
- `docs/RUSTDOC_GUIDE.md` — current. Doctest standards.
- `docs/NAMING.md` — current. The micro-tier rubric this skill encodes.
- `docs/API_DESIGN.md` — current. The 5 rules this skill encodes.
- `docs/DESIGN_PRINCIPLES.md` — current. North-star + audit matrix.

**Authoritative when docs disagree**: source code (`src/lib.rs` re-exports, `src/context/widgets/` signatures), compile-checked `examples/*.rs` demos, `tests/*.rs` regression tests.

## Examples (`examples/` — 66 Rust files, 32 Cargo-listed targets)

The skill's `SKILL.md` lists the canonical reference example for each domain. Highlights:
- **Feature demos**: `widgets_tour.rs` (target `widgets_tour`, tabbed over every single-feature demo such as `gauge.rs`, `modal_trap.rs`, `named_focus.rs`), `dx_shortcuts.rs` (on_hover/animate_bool/fill), `feature_showcase.rs` (target `feature_showcase`, the same features on one screen).
- **Cookbook recipes**: `cookbook_login.rs`, `cookbook_table.rs`, `cookbook_modal_toast.rs`, `cookbook_dashboard.rs`, `cookbook_file_picker.rs`, `cookbook_tour.rs`
- **Composition**: `demo_website.rs` (provide/use_context), `tests/context_provider.rs` (8 unit tests)
- **CJK / wide chars**: `demo_cjk.rs`
- **All charts**: `demo_infoviz.rs`

## Release / deployment

`AGENTS.md` at project root has the full 8-step checklist. Short:

Local PRE-CI → branch `release/vX.Y.Z` → atomic commit → push → PR → wait CI → merge (squash) → pull main → tag → push tag → wait `release.yml` → verify `gh release view` + crates.io + docs.rs → announce.

## MSRV

Rust 1.88. Verify with `cargo +1.88 check -p superlighttui --features async,serde` and the matching all-features Clippy gate.

## Supported targets

- Native: macOS, Linux, Windows (terminal backend via `crossterm`)
- WASM: `wasm32-unknown-unknown` via `crates/slt-wasm` (no `crossterm`)
