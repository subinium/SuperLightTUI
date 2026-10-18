//! Issue #456: `Response` is no longer `#[must_use]` as a type.
//!
//! This file is compiled with `unused_must_use` denied, so every bare layout
//! and display statement below must compile without `let _ =`. The negative
//! half (a bare `ui.button(..)` still warns) is a `compile_fail` doc-test on
//! [`slt::Response`].

#![deny(unused_must_use)]

use slt::{AlertLevel, TestBackend};

#[test]
fn layout_and_display_calls_need_no_let_underscore() {
    let mut tb = TestBackend::new(40, 12);
    let mut saved = false;
    tb.render(|ui| {
        ui.col(|ui| {
            ui.text("Name");
        });
        ui.row(|ui| {
            ui.text("mode:");
            ui.text("ready");
        });
        ui.separator();
        ui.badge("new");
        ui.alert("hi", AlertLevel::Info);
        ui.container().p(1).col(|ui| {
            ui.text("inside");
        });
        ui.button("Save").on_click(ui, |_| saved = true);
        ui.badge("tip").on_hover(ui, "tooltip");
    });
    tb.assert_contains("Name");
    tb.assert_contains("inside");
    assert!(!saved, "no click was delivered");
}
