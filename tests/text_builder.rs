//! Issue #457: `ui.text(..)` returns a scoped [`slt::Text`] builder.
//!
//! The compile-time half (chaining a widget after `text()` is an error, and
//! the deprecated `Context` modifiers warn) lives in `compile_fail`
//! doc-tests on `slt::Text` and `Context::bold`.

use std::time::Duration;

use slt::{Align, Color, Event, Modifiers, Style, TestBackend};

fn cell_style(tb: &TestBackend, x: u32, y: u32) -> Style {
    tb.buffer().get(x, y).style
}

#[test]
fn builder_chain_renders_bold_cyan() {
    let mut tb = TestBackend::new(20, 2);
    tb.render(|ui| {
        ui.text("x").bold().fg(Color::Cyan);
    });
    let style = cell_style(&tb, 0, 0);
    assert_eq!(style.fg, Some(Color::Cyan));
    assert!(style.modifiers.contains(Modifiers::BOLD));
}

#[test]
fn consecutive_texts_keep_call_order() {
    let mut tb = TestBackend::new(20, 4);
    tb.render(|ui| {
        ui.text("a");
        ui.text("b").fg(Color::Red);
        ui.text("c");
    });
    tb.assert_line(0, "a");
    tb.assert_line(1, "b");
    tb.assert_line(2, "c");
    assert_eq!(cell_style(&tb, 0, 1).fg, Some(Color::Red));
    assert_ne!(cell_style(&tb, 0, 2).fg, Some(Color::Red));
}

#[test]
fn builder_layout_options_apply() {
    let mut tb = TestBackend::new(20, 4);
    tb.render(|ui| {
        ui.text("hi").w(6).align(Align::End).mt(1);
    });
    // Top margin pushes the text to row 1; right-aligned inside 6 columns.
    tb.assert_line(1, "    hi");
}

#[test]
fn show_reports_hover_like_other_display_widgets() {
    let mut tb = TestBackend::new(20, 3);
    let mut hovered = Vec::new();
    // Frame 1 lays out the hit area; frame 2 moves the pointer onto it.
    tb.render(|ui| {
        hovered.push(ui.text("hover me").show().hovered);
    });
    tb.run_with_events(vec![Event::mouse_move(2, 0)], |ui| {
        hovered.push(ui.text("hover me").show().hovered);
    });
    tb.run_with_events(vec![Event::mouse_move(15, 2)], |ui| {
        hovered.push(ui.text("hover me").show().hovered);
    });
    assert_eq!(hovered, vec![false, true, false]);
}

#[test]
fn show_keeps_styling_and_later_widgets_hit_testable() {
    let mut tb = TestBackend::new(20, 3);
    let mut badge_hovered = false;
    let view = |ui: &mut slt::Context, badge_hovered: &mut bool| {
        let _ = ui.text("title").bold().fg(Color::Green).show();
        *badge_hovered = ui.badge("tag").hovered;
    };
    tb.render(|ui| view(ui, &mut badge_hovered));
    tb.run_with_events(vec![Event::mouse_move(1, 1)], |ui| {
        view(ui, &mut badge_hovered)
    });
    assert!(
        badge_hovered,
        "the slot reserved by show() must not shift later widgets"
    );
    let style = cell_style(&tb, 0, 0);
    assert_eq!(style.fg, Some(Color::Green));
    assert!(style.modifiers.contains(Modifiers::BOLD));
}

#[test]
fn with_if_styles_the_same_element() {
    let mut tb = TestBackend::new(20, 3);
    tb.render(|ui| {
        ui.text("on")
            .with_if(true, |t| {
                t.fg(Color::Red);
            })
            .bold();
        ui.text("off").with_if(false, |t| {
            t.fg(Color::Red);
        });
    });
    let on = cell_style(&tb, 0, 0);
    assert_eq!(on.fg, Some(Color::Red));
    assert!(on.modifiers.contains(Modifiers::BOLD));
    assert_ne!(cell_style(&tb, 0, 1).fg, Some(Color::Red));
}

#[test]
fn styled_link_and_timer_return_the_builder() {
    let mut tb = TestBackend::new(30, 4);
    tb.render(|ui| {
        ui.styled("styled", Style::new()).fg(Color::Yellow);
        ui.timer_display(Duration::from_secs(61)).bold();
        let link = ui.link("docs", "https://example.com").show();
        assert!(!link.clicked);
    });
    assert_eq!(cell_style(&tb, 0, 0).fg, Some(Color::Yellow));
    tb.assert_line(1, "01:01.00");
    assert!(cell_style(&tb, 0, 1).modifiers.contains(Modifiers::BOLD));
    tb.assert_contains("docs");
}

#[test]
#[allow(deprecated)]
fn deprecated_context_modifiers_still_apply_to_last_text() {
    let mut tb = TestBackend::new(20, 3);
    tb.render(|ui| {
        ui.text("x");
        ui.bold().fg(Color::Magenta);
        ui.styled("y", Style::new());
        ui.underline();
    });
    let x = cell_style(&tb, 0, 0);
    assert_eq!(x.fg, Some(Color::Magenta));
    assert!(x.modifiers.contains(Modifiers::BOLD));
    assert!(
        cell_style(&tb, 0, 1)
            .modifiers
            .contains(Modifiers::UNDERLINE)
    );
}

#[test]
#[allow(deprecated)]
fn deprecated_modifier_after_a_container_stays_a_no_op() {
    // v0.25 behaviour, kept for the deprecation window: the modifiers only
    // ever reach text elements.
    let mut tb = TestBackend::new(20, 3);
    tb.render(|ui| {
        ui.row(|ui| {
            ui.text("inner");
        });
        ui.bold();
    });
    assert!(!cell_style(&tb, 0, 0).modifiers.contains(Modifiers::BOLD));
}
