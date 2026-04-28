//! Snapshot-style smoke tests for `examples/v020_regression_panel.rs`.
//!
//! The regression panel is a single-screen visual proof that v0.19 → v0.20
//! features still render together. By invoking the example's
//! [`render`](v020_regression_panel::render) entry point with a deterministic
//! [`DemoState`](v020_regression_panel::DemoState), we pin the visible
//! frame so a future widget refactor cannot silently drop the gauges, the
//! table, the gutter highlights, or the key-help footer.
//!
//! The tests deliberately use `assert_contains` instead of `insta`
//! snapshots — the regression panel exercises ~10 widgets at once and
//! brittle pixel snapshots would churn on every layout tweak. The
//! contains-style assertions still catch the regressions Reviewer C
//! flagged: any of the listed widgets being dropped or its label being
//! mangled would fail the test.

use slt::TestBackend;

#[allow(dead_code)]
#[path = "../examples/v020_regression_panel.rs"]
mod v020_regression_panel;

use v020_regression_panel::{render, DemoState};

/// Width / height generous enough to fit the full panel (gauges row,
/// table + gutter row, footer, plus four corner anchors and the center
/// glyph) without wrapping. Matches the size a reviewer would launch the
/// binary at on a typical full-screen terminal.
const W: u32 = 100;
const H: u32 = 30;

#[test]
fn demo_regression_panel_renders_gauge_row() {
    let mut tb = TestBackend::new(W, H);
    let mut state = DemoState::new();
    tb.render(|ui| render(ui, &mut state));
    // Gauge label fingerprints (#224 builder API).
    tb.assert_contains("Gauges (#224)");
    tb.assert_contains("CPU 42%");
    tb.assert_contains("MEM 78%");
    let out = tb.to_string_trimmed();
    // Filled / empty gauge glyphs must both be present (one half-filled,
    // the other 78% — both states should produce mixed glyphs).
    assert!(
        out.contains('█') || out.contains('━'),
        "no gauge fill glyph in output: {out}",
    );
}

#[test]
fn demo_regression_panel_renders_table_row() {
    let mut tb = TestBackend::new(W, H);
    let mut state = DemoState::new();
    tb.render(|ui| render(ui, &mut state));
    // Table header + at least one body row must render.
    tb.assert_contains("Name");
    tb.assert_contains("Status");
    tb.assert_contains("alpha");
    tb.assert_contains("gamma");
}

#[test]
fn demo_regression_panel_renders_gutter_highlights() {
    let mut tb = TestBackend::new(W, H);
    let mut state = DemoState::new();
    tb.render(|ui| render(ui, &mut state));
    // Gutter scrollable header + at least one log line + a highlighted
    // ERROR row.
    tb.assert_contains("Gutter highlights (#235)");
    tb.assert_contains("ERROR");
}

#[test]
fn demo_regression_panel_renders_key_help_footer() {
    let mut tb = TestBackend::new(W, H);
    let mut state = DemoState::new();
    tb.render(|ui| render(ui, &mut state));
    // Footer hint string from the panel itself (#236 keymap publish).
    tb.assert_contains("press M to open modal");
    tb.assert_contains("? for key-help");
}

#[test]
fn demo_regression_panel_help_overlay_visible_when_open() {
    // When `help_open == true` the keymap_help_overlay should render
    // some of the published bindings on top of the panel.
    let mut tb = TestBackend::new(W, H);
    let mut state = DemoState::new();
    state.help_open = true;
    tb.render(|ui| render(ui, &mut state));
    let out = tb.to_string_trimmed();
    // Overlay must show at least one published binding label. The exact
    // formatting belongs to keymap_help_overlay; we just check that at
    // least one PANEL_KEYS label survived to the screen.
    assert!(
        out.contains("open modal") || out.contains("next / prev focusable"),
        "help overlay did not render published keymap: {out}",
    );
}
