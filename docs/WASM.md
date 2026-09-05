# Browser Backend

`slt-wasm` renders SuperLightTUI widgets into a DOM cell grid. A browser
application supplies its own `cdylib`, wasm-bindgen entry point, and retained
`WasmAppHandle`. No npm package publication is required.

The v0.24 implementation is verified from source by compiled browser tests.
Source verification is not evidence of registry publication. Exact-version
registry builds, docs.rs, and release workflow verification remain release gates.

Candidate package verification uses `cargo package --workspace
--no-default-features --target wasm32-unknown-unknown --locked`; Cargo builds
the packaged root and companion through a temporary local registry. This does
not publish either package. After publication, `python3
scripts/smoke_wasm_release.py X.Y.Z --expect-commit COMMIT` resolves both exact
versions from crates.io, runs the compiled browser consumer, and waits for
their exact docs.rs pages. The release workflow blocks its combined success
announcement on this verification.

## Runnable Example

The standalone application at `crates/slt-wasm/examples/browser` has its own
manifest and workspace. It includes editable text, a counter, calendar, and a
Stop button. From the repository root:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
wasm-pack build crates/slt-wasm/examples/browser --target web --dev
python3 -m http.server 8080 --directory crates/slt-wasm/examples/browser
```

Open `http://localhost:8080`. Its HTML imports generated local JavaScript and
WASM from `pkg/`; HTTP serving is necessary, not `file://` loading. `pagehide`
disposes the retained Rust runtime. No Node toolchain is needed to run it.

For an independent consumer after the matching versions are published:

```toml
[package]
name = "my-browser-app"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
superlighttui = { version = "=0.24.0", default-features = false }
slt-wasm = "=0.24.0"
wasm-bindgen = "0.2"
web-sys = { version = "0.3", features = ["HtmlElement"] }
```

Use the example's `src/lib.rs` and HTML loading pattern. Do not enable the root
crate's native defaults through another dependency: Cargo unifies features.
The example's path dependencies are for source development; registry acceptance
must remove every path/git override and resolve both exact published versions.

## Mounting And Options

```rust,ignore
let options = slt_wasm::WasmOptions {
    width: 80,
    height: 24,
    theme: slt::Theme::light(),
    scroll_speed: 3,
    max_fps: Some(30),
    auto_fit: true,
    ..Default::default()
};
let handle = slt_wasm::run_wasm_with_options(host, options, |ui| {
    ui.text("Browser application");
})?;
```

- `run_wasm_with_handle` and `run_wasm` remain default wrappers. Keep an owned
  handle alive; dropping it stops the runtime. The compatibility fire-and-forget
  wrapper retains ownership internally until quit or failure.
- Options are mount-time: theme, widget theme, scroll speed, input ownership,
  auto-fit, and frame limit. Per-frame `ui.set_theme` and `ui.set_scroll_speed`
  remain available. `input: false` mounts a display-only grid without an
  editable sink or keyboard/pointer listeners.
- The default is a fixed grid and a 60 FPS cap. `auto_fit: true` observes the
  host element with `ResizeObserver`, including changes without window resize.
  Give auto-fit hosts an independently sized content box, not content-driven
  height. Padding and borders are excluded from fitting.
- `max_fps: None` renders each RAF. `Some(0)` and zero grid dimensions are
  rejected. Throttled frames preserve queued events in order. Background-tab
  resume renders once without replaying missed frames. `ui.tick()` counts
  rendered frames, not elapsed time; scheduler time uses a portable monotonic
  clock, and calendar time uses a separate epoch clock.
- Mount at most one active runtime per host. Multiple separate hosts are
  independent. Host contents belong to the embedding page; disposing retains
  the last grid and caller-owned children. Remounting replaces the old SLT grid.
  Removing the host from the document does not implicitly dispose its runtime.

## Rendering Contract

The owned runtime resets the live buffer before every rendered frame while
retaining the previous presented buffer for diffing. Removed text, widgets,
styles, wide characters, and modals are erased without application-side clears.
An unchanged complete redraw does not mutate grid cells. Low-level
`DomBackend::buffer_mut()` followed by `flush()` retains its incremental drawing
contract; direct callers own their clear/frame boundary.

The grid uses a 14px monospace font and 16px row pitch. Column geometry derives
from the grid's fixed CSS width, not any displayed glyph. Wide continuations
are empty, hidden metadata spans; their leading grapheme occupies two columns.
Pointer and wheel coordinates use the grid's actual bounding rectangle,
including host padding/borders and browser scale. The editing overlay is an
absolutely positioned child of that same grid, so CSS translation and
axis-aligned positive scaling apply once to both. Resizing rebuilds painted
cells while preserving the grid, editable sink node, and its focus.
Do not replace its geometry styles with proportional fonts or rotate/skew its
ancestors. Glyph fallback can
still affect appearance; this does not change cell occupancy.

## Browser Input

Each interactive mount owns a transparent editable textarea over its cell grid,
with its caret padded to the last rendered SLT cursor. Focusing or clicking the
mount activates that input sink and gives native context menus an editable target.
Plain text arrives through browser editing events; navigation and SLT shortcuts
arrive through keydown. Composition preedit is displayed as underlined
text at the last rendered caret, in a separate pointer-transparent grid overlay.
It uses the caret's foreground/background, core grapheme widths, and clipping;
replacing preedit replaces only that presentation. Composition updates and
confirming Enter are not sent as widget input, so application text stays
unchanged until the nonempty composition-end commit. Empty cancellation, focus
loss, and disposal dismiss the preview without editing application state.
Masked text inputs suppress the preview entirely using the rendered cursor's
explicit `Buffer::cursor_is_masked()` policy, never by inspecting glyphs or styles. A composition
that starts masked, or becomes masked while active, stays suppressed until that
composition ends. A new composition in an unmasked field can show preedit again.
Committed text still goes through the widget's normal masking behavior.
Preedit updates are painted on runtime frames, after rechecking the current
caret privacy, so a pending focus or masking change cannot expose text using
the previous frame's policy. The configured frame pacing also applies here.
Ordinary single-character keyboard commits retain `Event::Key`
and modifiers for SLT character shortcuts. Composed, pasted, and other text
commits use `Event::Paste`, preserving multi-codepoint characters. Ctrl/Cmd+C, X,
and V retain browser defaults; paste
uses the delivered event's plain-text payload without requesting clipboard-read
permission. Browser copy/cut do not export SLT selections.

Pointer capture plus a scoped window fallback delivers drags and outside
releases. Outside release/cancellation emits `Up` at `(u32::MAX, u32::MAX)`,
which terminates dragging without clicking an edge widget. Hover/wheel outside
the grid are ignored. Blur and pointer cancellation terminate an active drag.
Touch gesture interpretation beyond pointer events is not provided.

Automated tests include trusted desktop typing and clipboard paste, synthetic
paste delivery, composition, cancellation, dead-key-style text commits, emoji,
separate page inputs, and independent mounts. Synthetic composition is **not**
physical Korean/Japanese IME validation. OS candidate-window placement, actual
keyboard-layout dead keys, native context-menu UI paste, mobile soft-keyboard
activation, and physical IME cancellation/confirmation require manual browser
and OS testing before a broader compatibility claim. The editable sink provides
the bridge, not blanket OS/browser parity or full terminal accessibility.

## Lifecycle And Errors

`dispose()`, Rust `Drop`, and `ui.quit()` cancel future RAF work. Input forwarding,
pointer capture, and a runtime-added host tab stop are released synchronously;
an old runtime cannot remove a new runtime's tab stop when disposal and remount
happen in the same JavaScript call. Caller-provided tabindex values are kept.
Listener, observer, sink, and app-closure destruction is deferred to a microtask so a
currently executing callback is not freed on its own stack. Disposal is
idempotent, including disposal requested synchronously from a user frame.
`is_running()` becomes false immediately; `error()` is absent for normal stops.
The presentation-only preedit is cleared immediately and removed with the input
sink during teardown, including quit and fatal-frame paths. A stopped runtime
does not accept later keyboard input or events dispatched on a retained,
disconnected sink. The last painted application frame remains visible.

Returned rendering/scheduling errors stop the app and populate `error()`.
A JavaScript guard around the real Rust RAF callback also records escaping
WASM traps as fatal failures and prevents rescheduling. After a panic/trap,
**discard the WASM instance and reload it**. Rust stack destructors and arbitrary
application state cannot be assumed recovered after `panic=abort`; do not
restart or reuse that instance or claim complete resource recovery from a trap.

## Capability Boundary

The browser backend is a DOM renderer, not a terminal emulator or native runtime.
It does not promise ANSI/OSC output, OSC 52 clipboard writes, Kitty/Sixel image
protocols, terminal suspend/signals, filesystem/process APIs, a native event
poller, or native async runtime behavior. Use browser-native integration APIs for
those capabilities. Clipboard history and arbitrary clipboard reads are not
part of mounting. Recent browsers must support WebAssembly, RAF, Pointer Events,
ResizeObserver (auto-fit), and composition/input events. Chrome is the automated
browser gate; other browsers are not implied tested by compilation alone.

## Verification

Compiled WASM integration tests run in the existing browser gate:

```sh
wasm-pack test --headless --chrome crates/slt-wasm
```

The standalone example also has a test-only compiled Rust fixture for trusted
Playwright input, layout measurements, controlled RAF clocks, and isolated fatal
trap tests. Its private npm package pins Playwright for verification only;
it is not a distribution package. With Node.js 20 or newer, from the repository
root run:

```sh
wasm-pack build crates/slt-wasm/examples/browser --target web --dev -- --features browser-tests
cd crates/slt-wasm/examples/browser
npm ci
npx playwright install chromium
npm test
```

`PLAYWRIGHT_MODULE` may point to an existing Playwright module installation and
`CHROME_EXECUTABLE` to an installed Chrome binary. The runner serves the compiled
assets locally, closes its server/browser, fails on unexpected page errors, and
prints evidence. It does not project the Rust implementation into JavaScript.
When using an installed Chrome binary, the Chromium installation step can be
omitted. `SLT_BROWSER_SCREENSHOT` overrides the public example screenshot path;
its default is the operating system's temporary directory.
