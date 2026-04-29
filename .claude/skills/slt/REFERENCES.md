# SLT References — v0.20

Load this when the user asks about feature flags, version-specific APIs, or doc locations.

## Feature matrix (from `Cargo.toml`)

| Feature | Default? | Purpose | APIs / capabilities enabled |
|---|---|---|---|
| `crossterm` | yes | Terminal backend | `slt::run`, `slt::run_with`, `slt::run_inline`, `slt::run_static`, `detect_color_scheme`, `read_clipboard` |
| `async` | no | Tokio integration | `slt::run_async` (channel-based message pump) |
| `serde` | no | Serialize/Deserialize | derives on `Style`, `Color`, `Theme`, layout types |
| `image` | no | Image loading helpers | `ui.image(...)` halfblock/kitty/sixel decode paths |
| `qrcode` | no | QR widget | `ui.qr_code(...)` |
| `syntax-rust` / `syntax-python` / `syntax-javascript` / `syntax-typescript` / `syntax-go` / `syntax-bash` / `syntax-json` / `syntax-toml` / `syntax-c` / `syntax-cpp` / `syntax-java` / `syntax-ruby` / `syntax-css` / `syntax-html` / `syntax-yaml` | no | Per-language tree-sitter grammars | `ui.code_block(code, "rust")` etc. |
| `syntax` | no | All `syntax-*` combined | every language above |
| `kitty-compress` | no | zlib-compressed kitty protocol | larger images, smaller payloads |
| `full` | no | crossterm+async+serde+image+qrcode+kitty-compress | development / demos only |

## v0.20 API surface (highlights AI agents miss)

These shipped in v0.20 and are NOT in older docs. Every entry verified against `src/`.

### Builders (Rule 1)
- `ui.gauge(ratio: f64) -> Gauge<'_>` — `.label(s).width(n).color(c).show() -> GaugeResponse`. `src/context/widgets_display/gauge.rs:37`
- `ui.line_gauge(ratio: f64) -> LineGauge<'_>` — `.label(s).filled(c).empty(c).width(n).show()`. `gauge.rs:55`
- `ui.breadcrumb(&[&str]) -> Breadcrumb<'_>` — `.separator(s).color(c).show() -> BreadcrumbResponse`. `widgets_display/status.rs:200`

### Opts struct (Rule 3)
- `ui.scrollable_with_gutter(&mut ScrollState, GutterOpts<G>, body)` — `widgets_display/gutter.rs:115`
- `GutterOpts::line_numbers(total, viewport)` — 90% case smart constructor
- `GutterOpts::new(total, viewport, |i| label)` — custom labels
- `HighlightRange::line(i)` / `::span(start, len)` — replaces v0.19 `::single`

### Split panes
- `ui.split_pane(&mut SplitPaneState, left, right) -> SplitPaneResponse` — horizontal split, draggable
- `ui.vsplit_pane(&mut SplitPaneState, top, bottom) -> SplitPaneResponse` — vertical split
- `SplitPaneState::new(0.5)`, `.with_min_ratio(0.2)`

### Hooks (v0.19+ + v0.20 additions)
- `ui.use_state(\|\| init)` — order-based, `src/context/runtime.rs:632`
- `ui.use_state_named::<T>("id")` — `&'static str`, `T: Default`, `runtime.rs:690`
- `ui.use_state_named_with("id", \|\| init)` — `&'static str` + init fn, `runtime.rs:671`
- `ui.use_state_keyed("id-{i}", \|\| init)` — runtime `String`, `runtime.rs:978`
- `ui.use_state_keyed_default::<T>("id-{i}")` — runtime `String` + `T: Default`, `runtime.rs:1002`
- `ui.use_memo(&deps, \|d\| compute(d))` — cached compute, `runtime.rs:832`
- `ui.use_effect(\|d\| { ... }, &deps)` — side effect on deps change, `runtime.rs:1039`
- `ui.provide(value, \|ui\| ...)` — typed scoped injection, `runtime.rs:780`
- `ui.use_context::<T>()` — panics if missing, `runtime.rs:807`
- `ui.try_use_context::<T>()` — `Option<&T>`, `runtime.rs:818`

### Animation shorthand (v0.20)
- `ui.animate_value("id", target, duration_ticks) -> f64` — `runtime.rs:743`
- `ui.animate_bool("id", value) -> f64` — eased 0..1 toggle, `runtime.rs:716`

### Response signal fields (v0.20)
`Response { clicked, right_clicked, hovered, changed, focused, gained_focus, lost_focus, rect }`. `right_clicked`, `gained_focus`, `lost_focus` are new in v0.20. Older docs may show only the v0.19 set.
- `Response::on_hover(ui, "tooltip text")` — chained tooltip (`src/context/state.rs:221`)
- `Response::on_hover_ui(ui, |ui| { ... })` — chained tooltip with custom body

### Theme additions (v0.20)
- `Theme::compact()` / `Theme::comfortable()` / `Theme::spacious()` — density variants of dark
- `Theme::with_spacing(spacing)` — override Spacing on any theme
- `ContainerBuilder::theme(theme)` — per-subtree theme override (no ambient state)
- `ThemeBuilder` / `ThemeBuilder::builder_from(base)` — `const fn` since v0.19.2

### Layout / DX (v0.20)
- `ContainerBuilder::fill()` — equivalent to `.grow(1)`, more readable
- `Rect::center_in(outer)`, `center_horizontally_in`, `center_vertically_in` — `src/rect.rs`
- `Context::modal_with(ModalOptions { tab_trap: true }, \|ui\| ...)` — focus-trapped modal
- `Context::static_log(...)` / `take_static_log()` — for `slt::run_static` scrollback

### Keymap (v0.20)
- `WidgetKeyHelp` trait — implement on a state type to publish keybindings
- `Context::publish_keymap(...)` / `published_keymaps()` — collect + render help
- `Context::keymap_help_overlay()` — auto-rendered overlay
- `RunConfig::handle_ctrl_c(bool)` — opt-out of auto Ctrl-C quit

### Testing (v0.20)
- `TestBackend::record_frames()` + `tb.frames()[i]` — frame history
- `tb.sequence().tick(n).key(...).type_string(s, &mut value).run()`
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

These docs exist but several lag behind v0.20. When in doubt, grep source.

- `docs/COMPLETE_REFERENCE.md` — **partially stale** (header still says v0.19.2; missing gauge/line_gauge/scrollable_with_gutter/split_pane/Response signal fields/v0.19 hooks). Treat as supplementary, not authoritative.
- `docs/WIDGETS.md` — **partially stale** (breadcrumb 4-variant API, missing v0.20 builders).
- `docs/STATE_APIS.md` — current for state methods. Note `ScrollState::progress() -> f32` is a documented Rule 2 violation pending fix.
- `docs/PREVIOUS_FRAME_GUIDE.md` — current. Read for `Response.rect` timing.
- `docs/PATTERNS.md` — current. `provide`/`use_context`/`use_state_named`/`with_if` covered.
- `docs/QUICK_START.md` — current.
- `docs/COOKBOOK.md` — current. (Cargo dep line still says `superlighttui = "0.19"` — bump to `0.20` mentally.)
- `docs/THEMING.md` — current. `ThemeBuilder` `const fn` since v0.19.2.
- `docs/TESTING.md` — covers v0.19.1 EventBuilder additions. Read alongside `tests/v020_test_utils_demo.rs` for v0.20 helpers.
- `docs/ANIMATION.md` — current.
- `docs/AI_GUIDE.md` — concise overview, current.
- `docs/BACKENDS.md` — `Backend`, `AppState`, `frame()` low-level paths.
- `docs/ARCHITECTURE.md` — render pipeline.
- `docs/DEBUGGING.md` — F12 layout overlay, debug flags.
- `docs/MIGRATION.md` — **partially stale** for v0.19 → v0.20 (frames v0.20 as "next minor", but it shipped). Use this skill's "v0.20 removed APIs" table.
- `docs/llms.txt` — **partially stale** (v0.19.2 highlight list).
- `docs/RUSTDOC_GUIDE.md` — current. Doctest standards.
- `docs/NAMING.md` — current. The micro-tier rubric this skill encodes.
- `docs/API_DESIGN.md` — current. The 5 rules this skill encodes.
- `docs/DESIGN_PRINCIPLES.md` — current. North-star + audit matrix.

**Authoritative when docs disagree**: source code (`src/lib.rs` re-exports, `src/context/widgets_*.rs` signatures), `examples/v020_*.rs` runnable demos, `tests/v020_*.rs` regression tests.

## Examples (`examples/` — 59 files, 16 Cargo-listed)

The skill's `SKILL.md` lists the canonical reference example for each domain. Highlights:
- **v0.20 surface**: `v020_showcase.rs` (kitchen sink), `v020_tour.rs` (tabbed), `v020_dx_shortcuts.rs` (on_hover/animate_bool/fill)
- **Cookbook recipes**: `cookbook_login.rs`, `cookbook_table.rs`, `cookbook_modal_toast.rs`, `cookbook_dashboard.rs`, `cookbook_file_picker.rs`, `cookbook_tour.rs`
- **Composition**: `demo_website.rs` (provide/use_context), `tests/context_provider.rs` (8 unit tests)
- **CJK / wide chars**: `demo_cjk.rs`
- **All charts**: `demo_infoviz.rs`

## Release / deployment

`CLAUDE.md` at project root has the full 8-step checklist. Short:

Local PRE-CI → branch `release/vX.Y.Z` → atomic commit → push → PR → wait CI → merge (squash) → pull main → tag → push tag → wait `release.yml` → verify `gh release view` + crates.io + docs.rs → announce.

## MSRV

Rust 1.81. Verify with `cargo check --features async,serde` on a 1.81 toolchain (CI's MSRV job runs this exact command).

## Supported targets

- Native: macOS, Linux, Windows (terminal backend via `crossterm`)
- WASM: `wasm32-unknown-unknown` via `crates/slt-wasm` (no `crossterm`)
