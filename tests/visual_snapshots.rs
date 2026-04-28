//! Visual regression snapshots.
//!
//! Each test renders a known demo for one frame at a fixed terminal size
//! and snapshots the buffer output as plain text. Failures indicate a
//! layout, border, wrap, or theme regression.
//!
//! Snapshot files live next to this test under `tests/snapshots/visual__*.snap`
//! and are committed to source control. When the visual output changes
//! intentionally (a deliberate widget restyling, layout tweak, etc.),
//! review and accept the new baseline:
//!
//! ```bash
//! cargo insta review
//! ```
//!
//! ## What this catches
//!
//! - Layout drift (flexbox grow/shrink/gap regressions)
//! - Border rendering (wrong corners, missing edges, title overflow)
//! - Theme color shifts that flip glyph attributes
//! - CJK / wide-char width handling at the right edge
//! - Wrap and truncation at small terminal sizes
//!
//! ## What this does NOT catch
//!
//! - Interactive state transitions (focus, hover, click) — those need
//!   `EventBuilder` and assertion-based tests
//! - Animation / frame-timing — different lane (parity/property tests)
//! - Sixel / kitty image output — not represented in plain-text buffer
//!
//! ## Implementation
//!
//! Each `examples/demo_*.rs` file exposes a `pub fn render(ui: &mut Context)`
//! entry point that builds fresh state and runs one rendering pass. The
//! example's own `main` keeps using `slt::run` (or `slt::run_with`) so the
//! interactive demo still works; we just call the render fn directly with
//! a `TestBackend` here.
//!
//! demo.rs and demo_infoviz.rs are imported via `#[path]` because
//! `examples/*.rs` files are not part of the main library crate.

use slt::TestBackend;

// Each example exposes `pub fn render(ui: &mut Context)`.
// The example's own `fn main` is unused here, hence `dead_code`.
#[allow(dead_code)]
#[path = "../examples/demo.rs"]
mod demo;

#[allow(dead_code)]
#[path = "../examples/demo_dashboard.rs"]
mod demo_dashboard;

#[allow(dead_code)]
#[path = "../examples/demo_cjk.rs"]
mod demo_cjk;

#[allow(dead_code)]
#[path = "../examples/demo_infoviz.rs"]
mod demo_infoviz;

#[allow(dead_code)]
#[path = "../examples/demo_overlay_anchor.rs"]
mod demo_overlay_anchor;

/// Render one frame of `f` at `(w, h)` and snapshot under `tests/snapshots/visual__<name>.snap`.
///
/// We bind `prepend_module_to_snapshot => false` so the snapshot file is
/// just `visual__<name>.snap` instead of
/// `visual_snapshots__visual__<name>.snap`.
fn snapshot_frame(name: &str, w: u32, h: u32, f: impl FnOnce(&mut slt::Context)) {
    let mut tb = TestBackend::new(w, h);
    tb.render(f);
    let body = tb.to_string_trimmed();
    insta::with_settings!({
        snapshot_path => "snapshots",
        prepend_module_to_snapshot => false,
        omit_expression => true,
    }, {
        insta::assert_snapshot!(format!("visual__{name}"), body);
    });
}

#[test]
fn visual_demo() {
    // demo.rs uses persistent `DemoState` for the runtime loop, so the
    // snapshot path goes through `render_snapshot` which constructs a
    // fresh `DemoState` every call (deterministic frame-1 output).
    snapshot_frame("demo", 80, 24, demo::render_snapshot);
}

#[test]
fn visual_demo_dashboard() {
    snapshot_frame("demo_dashboard", 120, 40, demo_dashboard::render);
}

#[test]
fn visual_demo_cjk() {
    snapshot_frame("demo_cjk", 80, 24, demo_cjk::render);
}

#[test]
fn visual_demo_infoviz() {
    snapshot_frame("demo_infoviz", 120, 40, demo_infoviz::render);
}

#[test]
fn visual_demo_overlay_anchor() {
    snapshot_frame("demo_overlay_anchor", 80, 24, demo_overlay_anchor::render);
}
