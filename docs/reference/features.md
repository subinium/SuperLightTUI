# Feature Flags

> **Canonical source:** `Cargo.toml [features]`. This document is a human-readable companion.

This guide is the human-readable feature matrix.
For the canonical API surface, also check docs.rs and `src/lib.rs`.

## Core model

- `unicode-width`, `unicode-segmentation`, `smallvec`, and `compact_str` are
  always part of the small core (4 direct required deps).
- `crossterm` is a **default feature**, not a hard requirement.
- Resolved tree (unique package/version pairs from `cargo tree -e normal`,
  excluding the root package): 10 crates with `default-features = false`,
  26 with default features, 63 with `--features full` — versus 69 for a
  ratatui + crossterm app. "Light" refers to this dependency footprint and the
  small public API, not stripped binary size or build speed.
- Binary size: a stripped release hello-world is ~1.15 MB with default
  features (~0.69 MB with a size-optimized profile), about 2x / 1.6x
  ratatui; a list / table / gauge dashboard is ~1.25 MB (1.6x). The size
  profile recipe, its trade-offs and the breakdown are in
  [docs/internals/performance.md](../internals/performance.md#binary-size).
  Turning off `bidi` saves another ~33-51 KB.
- The low-level core (`Backend`, `AppState`, `frame()`, widgets, events, style/layout types) works without terminal I/O.

## Main flags

| Flag | What it enables |
|------|------------------|
| `crossterm` | Built-in terminal runtime: `run()`, `run_with()`, `run_inline()`, terminal polling, clipboard query, terminal helpers |
| `bidi` | UAX #9 bidirectional text reordering; enabled by default and usable without `crossterm` |
| `async` | `run_async()` and tokio-based message-driven apps |
| `serde` | Serialize/Deserialize for style, theme, and layout-related public types |
| `theme-watch` | TOML theme hot reload; enables `serde` and `notify` |
| `image` | Image loading helpers for terminal image widgets |
| `qrcode` | `ui.qr_code(...)` |
| `syntax` | Convenience: enables all per-language `syntax-*` bundles below |
| `syntax-*` | Per-language syntax bundles such as `syntax-rust`, `syntax-python`, `syntax-typescript` |
| `kitty-compress` | zlib compression for Kitty image protocol uploads |
| `ratatui` | Render ratatui widgets inside SLT containers: `ContainerBuilder::ratatui` (closure over a ratatui `Rect` + `Buffer`) / `ratatui_widget(widget)` plus `From` conversions for `Color`, `Modifiers` and `Style`. Adds only `ratatui-core` 0.1 (ratatui 0.30), no ratatui backend; not part of `full`. Render-only — see `slt::ratatui_compat` for clipping and lossy style rules |
| `pty-test` | Development-only in-process PTY backend contract tests; not part of `full` |
| `full` | Convenience bundle: enables `crossterm`, `async`, `serde`, `theme-watch`, `image`, `qrcode`, `kitty-compress`, and `bidi` (does **not** include `syntax` — add language bundles separately) |

## What disappears without `crossterm`

When you disable default features and do not re-enable `crossterm`:

- `run()` and terminal-owned loops are unavailable
- terminal helpers such as color scheme detection are unavailable
- terminal clipboard query support is unavailable

What still remains:

- `Backend`
- `AppState`
- `frame()`
- `Context`, widgets, events, styles, layout, charts

That makes SLT usable as a rendering core for non-terminal environments.

## Terminal query safety

The built-in runtime probes capabilities only when both stdin and stdout are
TTYs and the environment identifies a direct terminal emulator. It skips
generic PTY wrappers, `TERM=dumb`, and tmux/Zellij/GNU screen sessions by
default. Reply collection is synchronous and nonblocking/pollable; after every
responsive, partial, or silent deadline, no background stdin reader remains to
consume application input.

The startup probe sends DA1/DA2 (with the Kitty keyboard query `CSI ? u` in
the same write), the Kitty graphics query, XTGETTCAP `Tc`, and DECRQM 2026.
Its results feed color depth (a `Tc` reply selects 24-bit color), the Kitty
keyboard push (pushed only after a `CSI ? u` reply), and the image ladder.
The optional width probe (`RunConfig::width_probe(true)`) adds four cursor
position reports on fullscreen direct terminals.

| Variable | Effect |
|----------|--------|
| `NO_COLOR` | Any non-empty value disables color unless `RunConfig::color_depth` is set |
| `COLORTERM` | `truecolor` / `24bit` selects 24-bit color |
| `TERM` | `dumb` disables queries, color, and title / progress / status / cursor-shape sequences; `256color` selects 256 colors; identity (`xterm-kitty`, `xterm-ghostty`, `wezterm` select 24-bit color) |
| `TERM_PROGRAM` / `TERM_PROGRAM_VERSION` | Identity: `iTerm.app`, `WezTerm`, `ghostty`, `vscode` select 24-bit color; `ghostty` and `iTerm.app` 3.6+ receive OSC 9;4 progress |
| `WT_SESSION` / `ConEmuANSI=ON` | Windows Terminal / ConEmu: OSC 9;4 progress (Windows Terminal also 24-bit color) |
| `SLT_DISABLE_TERMINAL_QUERIES` / `SLT_FORCE_TERMINAL_QUERIES` | Disable all terminal queries / opt in on an otherwise skipped host |
| `SLT_DISABLE_SYNC_OUTPUT` / `SLT_FORCE_SYNC_OUTPUT` | Synchronized output |
| `SLT_DISABLE_KITTY` / `SLT_FORCE_KITTY` | Kitty graphics |
| `SLT_DISABLE_SIXEL` / `SLT_FORCE_SIXEL` | Sixel graphics |
| `SLT_DISABLE_ITERM` / `SLT_FORCE_ITERM` | iTerm2 inline images |
| `SLT_DISABLE_KITTY_KEYBOARD` / `SLT_FORCE_KITTY_KEYBOARD` | Kitty keyboard protocol; force also pushes without a `CSI ? u` reply |
| `SLT_DISABLE_PROGRESS` / `SLT_FORCE_PROGRESS` | OSC 9;4 taskbar progress |
| `SLT_DISABLE_PROGRAM_STATUS` / `SLT_FORCE_PROGRAM_STATUS` | OSC 7501 program status |

- `SLT_*` flags accept `1`, `true`, `yes`, or `on`. Disable flags take
  precedence when both forms are set.
- Zellij enables Sixel and Kitty keyboard by default (the keyboard push is
  policy-only there because queries are skipped); other multiplexer
  protocols remain conservative unless explicitly forced.

See `docs/guide/09-runtime-modes.md` for the full direct-terminal, multiplexer, SSH, and PTY
compatibility table and the bounded real-process CI matrix.

## Recommended combos

| Goal | Suggested dependency |
|------|----------------------|
| Regular terminal app | `superlighttui = "..."` |
| Async terminal app | `superlighttui = { version = "...", features = ["async"] }` |
| QR output | `superlighttui = { version = "...", features = ["qrcode"] }` |
| Syntax-highlighted code blocks | `superlighttui = { version = "...", features = ["syntax", "syntax-rust"] }` |
| Custom backend only | `superlighttui = { version = "...", default-features = false }` |

## AI-friendly rule of thumb

When reading or generating code:

- if you see `run()` / `run_inline()` / clipboard query usage, assume `crossterm` is needed
- if you see `run_async()`, assume `async` is needed
- if you see `ui.qr_code(...)`, assume `qrcode` is needed
- if you see `ui.code_block(..).lang(...)` with tree-sitter behavior, check `syntax` flags


## Related docs

- `docs/guide/09-runtime-modes.md` - no-default-features and custom backend path
- `docs/README.md` - docs index
- `src/lib.rs` - crate-level rustdoc and cfg-gated API surface
