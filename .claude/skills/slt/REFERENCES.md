# SLT References

Load this only when the user asks about specific feature flags, or when building something that requires a non-default feature.

## Feature matrix (from `Cargo.toml`)

| Feature | Default? | Purpose | APIs / capabilities enabled |
|---|---|---|---|
| `crossterm` | yes | Terminal backend | `slt::run`, `slt::run_inline`, `detect_color_scheme`, `read_clipboard` |
| `async` | no | Tokio channel-based messaging | `slt::run_async` |
| `serde` | no | Serialize/Deserialize | derives on `Style`, `Color`, `Theme`, layout types |
| `image` | no | Image loading helpers | `ui.image(...)` halfblock/kitty/sixel |
| `qrcode` | no | QR code widget | `ui.qr_code(...)` |
| `syntax-rust` | no | Rust syntax highlighting | `ui.code_block(code, "rust")` |
| `syntax-python` | no | Python syntax highlighting | `ui.code_block(code, "python")` |
| `syntax-javascript` / `syntax-typescript` | no | JS/TS highlighting | same |
| `syntax-go` / `syntax-bash` / `syntax-json` / `syntax-toml` / `syntax-c` / `syntax-cpp` / `syntax-java` / `syntax-ruby` / `syntax-css` / `syntax-html` / `syntax-yaml` | no | Other languages | same |
| `syntax` | no | All `syntax-*` combined | all languages |
| `kitty-compress` | no | zlib-compressed kitty protocol | larger images with smaller payloads |
| `full` | no | Everything: crossterm+async+serde+image+qrcode+kitty-compress | use for development / demos only |

## Doc pointers

- `docs/COMPLETE_REFERENCE.md` — full API, single-file, ~1500 lines (LLM-optimized)
- `docs/COOKBOOK.md` — 5 copy-paste app recipes
- `docs/STATE_APIS.md` — every public `*State` struct with methods
- `docs/PREVIOUS_FRAME_GUIDE.md` — frame timing, when `Response.rect` is valid
- `docs/PATTERNS.md` — reusable patterns
- `docs/ARCHITECTURE.md` — render pipeline (commands → build_tree → flexbox → collect → render → flush)
- `docs/THEMING.md` — `Theme` presets, `ThemeColor` semantic tokens, contrast helpers
- `docs/TESTING.md` — `TestBackend`, `EventBuilder`, snapshot patterns
- `docs/AI_GUIDE.md` — concise AI-oriented overview
- `docs/BACKENDS.md` — `Backend`, `AppState`, `frame()` low-level paths
- `docs/DEBUGGING.md` — F12 layout overlay, common debug flags
- `src/lib.rs` — authoritative public re-exports
- `examples/` — 25+ runnable examples

## Release / deployment reference

See `CLAUDE.md` at project root for the full 8-step release checklist. One-line summary:

Local PRE-CI → branch `release/vX.Y.Z` → commit → push → PR → wait CI → merge (squash) → pull main → tag → push tag → wait release workflow → verify `gh release view` + crates.io + docs.rs → announce.

## MSRV

Rust 1.81. Verify MSRV check with `cargo check --features async,serde` on a 1.81 toolchain.

## Supported targets

- Native: macOS, Linux, Windows (terminal backend via `crossterm`)
- WASM: `wasm32-unknown-unknown` via `crates/slt-wasm` (no `crossterm`)
