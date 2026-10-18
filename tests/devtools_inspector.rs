//! Issue #268: devtools inspector (Ctrl+F12) end-to-end tests.
//!
//! These drive the full frame kernel through `TestBackend`, so they exercise
//! the focus snapshot threading + the `render_inspector` overlay together with
//! the public `set_inspector` / `inspector` API. The inspector reads
//! `prev_focus_count`, which is only populated on the frame *after* a
//! focusable registers, so each test renders at least two frames (TestBackend
//! reuses `FrameState` across `render()` calls).

use slt::TestBackend;

#[test]
fn inspector_off_renders_nothing() {
    let mut tb = TestBackend::new(60, 12);
    // Frame 0: register a focusable button so a later frame would have a chain.
    tb.render(|ui| {
        let _ = ui.button("OK");
    });
    // Frame 1: inspector stays off — no panel text should appear.
    tb.render(|ui| {
        let _ = ui.button("OK");
    });
    tb.assert_not_contains("SLT Inspector");
    tb.assert_not_contains("focus chain");
}

#[test]
fn inspector_shows_focused_widget_and_chain() {
    let mut tb = TestBackend::new(70, 14);
    // Frame 0: register the focusables so frame 1 has a settled focus chain.
    tb.render(|ui| {
        ui.set_inspector(true);
        let _ = ui.button("Alpha");
        let _ = ui.button("Beta");
    });
    // Frame 1: inspector is on (persisted) and the chain is now populated.
    tb.render(|ui| {
        assert!(ui.inspector(), "set_inspector must persist across frames");
        let _ = ui.button("Alpha");
        let _ = ui.button("Beta");
    });

    tb.assert_contains("SLT Inspector");
    tb.assert_contains("focused widget");
    tb.assert_contains("index: 0");
    tb.assert_contains("padding:");
    tb.assert_contains("constraints:");
    tb.assert_contains("focus chain (2)");
}

#[test]
fn inspector_named_focus_shows_name() {
    // Each `register_focusable_named` is followed by a widget so the focus
    // marker attaches to a real layout node (the style panel resolves the
    // focused widget's node via that `focus_id`). Wide buffer keeps the left
    // style panel clear of the right-aligned chain panel.
    let render_ui = |ui: &mut slt::Context| {
        let _ = ui.register_focusable_named("search");
        ui.text("search box");
        let _ = ui.register_focusable_named("submit");
        ui.text("submit button");
    };

    let mut tb = TestBackend::new(120, 14);
    tb.render(|ui| {
        ui.set_inspector(true);
        render_ui(ui);
    });
    tb.render(render_ui);

    // The currently focused (index 0) named widget appears in the style panel,
    // and both names appear in the chain panel.
    tb.assert_contains("name: search");
    tb.assert_contains("search");
    tb.assert_contains("submit");
}

#[test]
fn inspector_no_focusables_shows_notice() {
    let mut tb = TestBackend::new(60, 8);
    tb.render(|ui| {
        ui.set_inspector(true);
        ui.text("just static text, nothing focusable");
    });
    tb.render(|ui| {
        ui.text("just static text, nothing focusable");
    });
    tb.assert_contains("no focusable widgets");
}

#[test]
fn set_inspector_false_hides_panel() {
    let mut tb = TestBackend::new(60, 10);
    tb.render(|ui| {
        ui.set_inspector(true);
        let _ = ui.button("OK");
    });
    tb.render(|ui| {
        assert!(ui.inspector());
        ui.set_inspector(false);
        let _ = ui.button("OK");
    });
    // Frame 2: panel must be gone.
    tb.render(|ui| {
        assert!(!ui.inspector(), "set_inspector(false) must persist");
        let _ = ui.button("OK");
    });
    tb.assert_not_contains("SLT Inspector");
}
