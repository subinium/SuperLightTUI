//! Regression tests for two overlay gaps left by #458 / #459:
//!
//! - a click that lands on a menu-bar title in the same frame in which hover
//!   switches the open dropdown to that title;
//! - a modal focus trap whose slots include disabled widgets.

use slt::widgets::{MenuBarState, MenuItem};
use slt::{Context, Event, EventBuilder, KeyCode, KeyModifiers, MouseKind, TestBackend};

// ── menu bar: same-frame hover switch + click ───────────────────────

fn app_bar() -> MenuBarState {
    MenuBarState::new(vec![
        ("File", vec![MenuItem::new("new", "New")]),
        ("Edit", vec![MenuItem::new("undo", "Undo")]),
        ("Help", vec![MenuItem::new("about", "About")]),
    ])
}

#[derive(Default)]
struct BarFrame {
    activated: Option<String>,
    /// Left presses still unconsumed after the bar handled the frame.
    leaked_presses: usize,
}

fn bar_scene(ui: &mut Context, bar: &mut MenuBarState, out: &mut BarFrame) {
    if let Some(id) = ui.menu_bar(bar) {
        out.activated = Some(id);
    }
    out.leaked_presses = ui
        .raw_events()
        .filter(|event| {
            matches!(event, Event::Mouse(mouse) if matches!(mouse.kind, MouseKind::Down(_)))
        })
        .count();
    ui.text("document");
}

/// Open the `File` dropdown with a click and return the backend.
fn bar_with_file_open(bar: &mut MenuBarState) -> TestBackend {
    let mut tb = TestBackend::new(40, 10);
    let mut out = BarFrame::default();
    tb.run_with_events(vec![], |ui| bar_scene(ui, bar, &mut out));
    let (fx, fy) = tb.find_text("File").expect("File title");
    tb.run_with_events(EventBuilder::new().click(fx, fy).build(), |ui| {
        bar_scene(ui, bar, &mut out)
    });
    assert!(bar.is_open());
    assert_eq!(bar.selected(), 0);
    tb.run_with_events(vec![], |ui| bar_scene(ui, bar, &mut out));
    tb.assert_contains("New");
    tb
}

#[test]
fn click_on_a_title_in_the_frame_hover_switches_to_it_is_handled() {
    let mut bar = app_bar();
    let mut tb = bar_with_file_open(&mut bar);
    let (ex, ey) = tb.find_text("Edit").expect("Edit title");
    let mut out = BarFrame::default();
    tb.run_with_events(
        EventBuilder::new().mouse_move(ex, ey).click(ex, ey).build(),
        |ui| bar_scene(ui, &mut bar, &mut out),
    );
    assert_eq!(
        out.leaked_presses, 0,
        "the bar handles the click on the title it switched to"
    );
    assert!(bar.is_open(), "the clicked menu stays open");
    assert_eq!(bar.selected(), 1);
    assert_eq!(out.activated, None);
    let mut out = BarFrame::default();
    tb.run_with_events(vec![], |ui| bar_scene(ui, &mut bar, &mut out));
    tb.assert_contains("Undo");
    tb.assert_not_contains("New");
}

#[test]
fn click_on_one_title_then_hover_onto_another_ends_on_the_hovered_menu() {
    let mut bar = app_bar();
    let mut tb = bar_with_file_open(&mut bar);
    let (hx, hy) = tb.find_text("Help").expect("Help title");
    let (ex, ey) = tb.find_text("Edit").expect("Edit title");
    let mut out = BarFrame::default();
    // Events apply in order: the click opens Help, the move then hovers Edit.
    tb.run_with_events(
        EventBuilder::new().click(hx, hy).mouse_move(ex, ey).build(),
        |ui| bar_scene(ui, &mut bar, &mut out),
    );
    assert_eq!(out.leaked_presses, 0);
    assert!(bar.is_open());
    assert_eq!(bar.selected(), 1);
}

#[test]
fn hover_alone_still_switches_the_open_menu() {
    let mut bar = app_bar();
    let mut tb = bar_with_file_open(&mut bar);
    let (hx, hy) = tb.find_text("Help").expect("Help title");
    let mut out = BarFrame::default();
    tb.run_with_events(EventBuilder::new().mouse_move(hx, hy).build(), |ui| {
        bar_scene(ui, &mut bar, &mut out)
    });
    assert!(bar.is_open());
    assert_eq!(bar.selected(), 2);
}

// ── modal focus trap with disabled slots ────────────────────────────

/// Which of the modal's buttons reported `focused`, plus the background one.
#[derive(Default, Debug, PartialEq, Eq)]
struct Focus {
    background: bool,
    modal: Vec<bool>,
}

/// A background button, then (when `open`) a modal with one button per
/// entry of `disabled`, each wrapped in `ui.disabled(flag, ..)`.
fn modal_scene(ui: &mut Context, open: bool, disabled: &[bool], out: &mut Focus) {
    out.background = ui.button("Background").focused;
    out.modal.clear();
    if open {
        let _ = ui.modal(|ui| {
            for (index, &flag) in disabled.iter().enumerate() {
                let focused = ui.disabled(flag, |ui| ui.button(format!("Item {index}")).focused);
                out.modal.push(focused);
            }
        });
    }
}

fn shift_tab() -> Vec<Event> {
    EventBuilder::new()
        .key_with(KeyCode::Tab, KeyModifiers::SHIFT)
        .build()
}

/// Index of the single focused modal button, `None` if none is focused.
fn focused_item(out: &Focus) -> Option<usize> {
    let focused: Vec<usize> = out
        .modal
        .iter()
        .enumerate()
        .filter_map(|(index, &focused)| focused.then_some(index))
        .collect();
    assert!(focused.len() <= 1, "at most one focused widget: {out:?}");
    focused.first().copied()
}

/// Render the base frame, open the modal, and settle one more frame.
fn open_modal(tb: &mut TestBackend, disabled: &[bool]) -> Focus {
    let mut out = Focus::default();
    tb.run_with_events(vec![], |ui| modal_scene(ui, false, disabled, &mut out));
    assert!(out.background, "focus starts on the background button");
    tb.run_with_events(vec![], |ui| modal_scene(ui, true, disabled, &mut out));
    tb.run_with_events(vec![], |ui| modal_scene(ui, true, disabled, &mut out));
    out
}

#[test]
fn modal_initial_focus_skips_a_disabled_first_slot() {
    let disabled = [true, false, false];
    let mut tb = TestBackend::new(40, 12);
    let out = open_modal(&mut tb, &disabled);
    assert!(!out.background);
    assert_eq!(focused_item(&out), Some(1), "first enabled slot: {out:?}");
}

#[test]
fn modal_initial_focus_skips_several_leading_disabled_slots() {
    let disabled = [true, true, false];
    let mut tb = TestBackend::new(40, 12);
    let out = open_modal(&mut tb, &disabled);
    assert_eq!(focused_item(&out), Some(2), "{out:?}");
}

#[test]
fn modal_tab_and_shift_tab_never_land_on_a_disabled_slot() {
    let disabled = [true, false, true, false];
    let mut tb = TestBackend::new(40, 12);
    let mut out = open_modal(&mut tb, &disabled);
    assert_eq!(focused_item(&out), Some(1), "{out:?}");

    let mut seen = Vec::new();
    for _ in 0..4 {
        tb.run_with_events(EventBuilder::new().key_code(KeyCode::Tab).build(), |ui| {
            modal_scene(ui, true, &disabled, &mut out)
        });
        tb.run_with_events(vec![], |ui| modal_scene(ui, true, &disabled, &mut out));
        assert!(!out.background, "Tab stays inside the modal");
        seen.push(focused_item(&out));
    }
    assert_eq!(seen, [Some(3), Some(1), Some(3), Some(1)], "Tab");

    let mut seen = Vec::new();
    for _ in 0..4 {
        tb.run_with_events(shift_tab(), |ui| modal_scene(ui, true, &disabled, &mut out));
        tb.run_with_events(vec![], |ui| modal_scene(ui, true, &disabled, &mut out));
        assert!(!out.background, "Shift+Tab stays inside the modal");
        seen.push(focused_item(&out));
    }
    assert_eq!(seen, [Some(3), Some(1), Some(3), Some(1)], "Shift+Tab");
}

#[test]
fn modal_shift_tab_from_the_open_frame_skips_a_disabled_last_slot() {
    let disabled = [false, false, true];
    let mut tb = TestBackend::new(40, 12);
    let mut out = open_modal(&mut tb, &disabled);
    assert_eq!(focused_item(&out), Some(0), "{out:?}");
    tb.run_with_events(shift_tab(), |ui| modal_scene(ui, true, &disabled, &mut out));
    tb.run_with_events(vec![], |ui| modal_scene(ui, true, &disabled, &mut out));
    assert_eq!(focused_item(&out), Some(1), "wraps past disabled Item 2");
}

/// With every modal slot disabled there is nothing to focus: no widget
/// reports focus, Tab / Shift+Tab are no-ops that stay trapped (no panic, no
/// endless loop), and focus returns to the background once the modal closes.
#[test]
fn modal_with_only_disabled_slots_focuses_nothing_and_releases_on_close() {
    let disabled = [true, true];
    let mut tb = TestBackend::new(40, 12);
    let mut out = open_modal(&mut tb, &disabled);
    assert_eq!(focused_item(&out), None);
    assert!(!out.background);
    for events in [
        EventBuilder::new().key_code(KeyCode::Tab).build(),
        shift_tab(),
        EventBuilder::new()
            .key_code(KeyCode::Tab)
            .key_code(KeyCode::Tab)
            .build(),
    ] {
        tb.run_with_events(events, |ui| modal_scene(ui, true, &disabled, &mut out));
        tb.run_with_events(vec![], |ui| modal_scene(ui, true, &disabled, &mut out));
        assert_eq!(focused_item(&out), None);
        assert!(!out.background, "focus does not escape the modal");
    }
    tb.run_with_events(vec![], |ui| modal_scene(ui, false, &disabled, &mut out));
    tb.run_with_events(vec![], |ui| modal_scene(ui, false, &disabled, &mut out));
    assert!(out.background, "focus returns to the background button");
}

/// A modal whose only enabled slot is in the middle keeps focus on it.
#[test]
fn modal_with_a_single_enabled_slot_keeps_focus_there() {
    let disabled = [true, false, true];
    let mut tb = TestBackend::new(40, 12);
    let mut out = open_modal(&mut tb, &disabled);
    assert_eq!(focused_item(&out), Some(1));
    for events in [
        EventBuilder::new().key_code(KeyCode::Tab).build(),
        shift_tab(),
    ] {
        tb.run_with_events(events, |ui| modal_scene(ui, true, &disabled, &mut out));
        tb.run_with_events(vec![], |ui| modal_scene(ui, true, &disabled, &mut out));
        assert_eq!(focused_item(&out), Some(1));
    }
}
