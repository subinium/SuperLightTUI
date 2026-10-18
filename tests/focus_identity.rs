//! Issue #454: focus follows the widget, not the slot number, when
//! focusables are inserted, removed or reordered between frames.

use std::cell::{Cell, RefCell};

use slt::widgets::{TextInputState, TextareaState};
use slt::{Context, EventBuilder, KeyCode, TestBackend};

/// Feed one event batch, keeping the carried focus state.
fn step(tb: &mut TestBackend, events: Vec<slt::Event>, f: impl FnOnce(&mut Context)) {
    tb.sequence().events(events, f).run();
}

#[test]
fn inserting_a_focusable_before_the_focused_input_keeps_focus_on_it() {
    let mut tb = TestBackend::new(40, 12);
    let a = RefCell::new(TextInputState::new());
    let b = RefCell::new(TextInputState::new());
    let show_banner = Cell::new(false);
    let r = |ui: &mut Context| {
        if show_banner.get() {
            let _ = ui.button("Dismiss banner");
        }
        let _ = ui.text_input(&mut a.borrow_mut());
        let _ = ui.text_input(&mut b.borrow_mut());
    };
    tb.render(r);
    tb.sequence().key(KeyCode::Tab, r).tick(r).run(); // focus -> b (slot 1)
    tb.type_string("x", r);
    assert_eq!(b.borrow().value, "x");

    show_banner.set(true); // focusable inserted before a
    tb.render(r);
    tb.type_string("y", r);

    assert_eq!(a.borrow().value, "");
    assert_eq!(b.borrow().value, "xy");
}

#[test]
fn removing_a_focusable_before_the_focused_input_keeps_focus_on_it() {
    let mut tb = TestBackend::new(40, 12);
    let a = RefCell::new(TextInputState::new());
    let b = RefCell::new(TextInputState::new());
    let show_banner = Cell::new(true);
    let r = |ui: &mut Context| {
        if show_banner.get() {
            let _ = ui.button("Dismiss banner");
        }
        let _ = ui.text_input(&mut a.borrow_mut());
        let _ = ui.text_input(&mut b.borrow_mut());
    };
    tb.render(r);
    tb.sequence()
        .key(KeyCode::Tab, r)
        .key(KeyCode::Tab, r)
        .tick(r)
        .run(); // focus -> b (slot 2)
    tb.type_string("x", r);
    assert_eq!(b.borrow().value, "x");

    show_banner.set(false);
    tb.render(r);
    tb.type_string("y", r);

    assert_eq!(a.borrow().value, "");
    assert_eq!(b.borrow().value, "xy");
}

#[test]
fn removing_the_focused_widget_falls_back_to_a_clamped_slot() {
    let mut tb = TestBackend::new(40, 14);
    let a = RefCell::new(TextInputState::new());
    let b = RefCell::new(TextInputState::new());
    let c = RefCell::new(TextInputState::new());
    let show_c = Cell::new(true);
    let r = |ui: &mut Context| {
        let _ = ui.text_input(&mut a.borrow_mut());
        let _ = ui.text_input(&mut b.borrow_mut());
        if show_c.get() {
            let _ = ui.text_input(&mut c.borrow_mut());
        }
    };
    tb.render(r);
    tb.sequence()
        .key(KeyCode::Tab, r)
        .key(KeyCode::Tab, r)
        .tick(r)
        .run(); // focus -> c (slot 2, the last one)
    tb.type_string("c", r);
    assert_eq!(c.borrow().value, "c");

    show_c.set(false); // the focused widget disappears
    tb.render(r);
    tb.render(r);
    tb.type_string("z", r);

    // Clamped to the new last slot (b), not wrapped to the first (a).
    assert_eq!(a.borrow().value, "");
    assert_eq!(b.borrow().value, "z");
}

#[test]
fn removing_every_focusable_does_not_panic() {
    let mut tb = TestBackend::new(30, 6);
    let a = RefCell::new(TextInputState::new());
    let show = Cell::new(true);
    let r = |ui: &mut Context| {
        if show.get() {
            let _ = ui.text_input(&mut a.borrow_mut());
        }
        ui.text("footer");
    };
    tb.render(r);
    tb.type_string("a", r);
    show.set(false);
    tb.render(r);
    tb.type_string("b", r);
    show.set(true);
    tb.render(r);
    tb.type_string("c", r);
    assert_eq!(a.borrow().value, "ac");
}

#[test]
fn cloned_states_receive_distinct_focus_keys() {
    let original = TextInputState::new();
    let copy = original.clone();
    assert_ne!(original.focus_key(), copy.focus_key());

    let area = TextareaState::new();
    assert_ne!(area.focus_key(), area.clone().focus_key());

    // Rendering both copies yields two independent slots: typing goes to one.
    let mut tb = TestBackend::new(40, 10);
    let first = RefCell::new(original);
    let second = RefCell::new(copy);
    let focused = Cell::new(None);
    let r = |ui: &mut Context| {
        let _ = ui.text_input(&mut first.borrow_mut());
        let _ = ui.text_input(&mut second.borrow_mut());
        focused.set(ui.focused_key());
    };
    tb.render(r);
    tb.type_string("k", r);
    assert_eq!(first.borrow().value, "k");
    assert_eq!(second.borrow().value, "");
    assert_eq!(focused.get(), Some(first.borrow().focus_key()));
}

#[test]
fn keyed_custom_widgets_keep_focus_across_reordering() {
    let mut tb = TestBackend::new(30, 8);
    let order = RefCell::new(vec!["alpha", "beta", "gamma"]);
    let focused = RefCell::new(None::<&'static str>);
    let r = |ui: &mut Context| {
        *focused.borrow_mut() = None;
        for item in order.borrow().iter().copied() {
            if ui.register_focusable_keyed(item) {
                *focused.borrow_mut() = Some(item);
            }
            ui.text(item);
        }
    };
    tb.render(r);
    tb.sequence().key(KeyCode::Tab, r).tick(r).run(); // focus -> beta
    assert_eq!(*focused.borrow(), Some("beta"));

    order.borrow_mut().reverse(); // gamma, beta, alpha: beta keeps index 1
    tb.render(r);
    assert_eq!(*focused.borrow(), Some("beta"));

    order.borrow_mut().rotate_left(1); // beta, alpha, gamma: beta moves to 0
    tb.render(r);
    assert_eq!(*focused.borrow(), Some("beta"));

    // Tab order is still declaration order: from beta (slot 0) to alpha.
    tb.sequence().key(KeyCode::Tab, r).tick(r).run();
    assert_eq!(*focused.borrow(), Some("alpha"));
}

#[test]
fn focus_by_name_still_resolves_with_keyed_focus() {
    let mut tb = TestBackend::new(40, 12);
    let a = RefCell::new(TextInputState::new());
    let b = RefCell::new(TextInputState::new());
    let show_banner = Cell::new(false);
    let request = Cell::new(false);
    let r = |ui: &mut Context| {
        if show_banner.get() {
            let _ = ui.button("banner");
        }
        let _ = ui.register_focusable_named("first");
        let _ = ui.text_input(&mut a.borrow_mut());
        let _ = ui.register_focusable_named("second");
        let _ = ui.text_input(&mut b.borrow_mut());
        if request.replace(false) {
            ui.focus_by_name("second");
        }
    };
    tb.render(r);
    request.set(true);
    tb.render(r);
    tb.render(r);
    tb.type_string("n", r);
    assert_eq!(b.borrow().value, "n");

    // The named slot keeps focus when a focusable is inserted before it.
    show_banner.set(true);
    tb.render(r);
    tb.type_string("m", r);
    assert_eq!(a.borrow().value, "");
    assert_eq!(b.borrow().value, "nm");
}

#[test]
fn insertion_does_not_report_spurious_focus_transitions() {
    let mut tb = TestBackend::new(40, 12);
    let a = RefCell::new(TextInputState::new());
    let b = RefCell::new(TextInputState::new());
    let show_banner = Cell::new(false);
    let a_gained = Cell::new(false);
    let b_lost = Cell::new(false);
    let b_gained = Cell::new(false);
    let r = |ui: &mut Context| {
        if show_banner.get() {
            let _ = ui.button("banner");
        }
        let ra = ui.text_input(&mut a.borrow_mut());
        let rb = ui.text_input(&mut b.borrow_mut());
        a_gained.set(ra.gained_focus);
        b_lost.set(rb.lost_focus);
        b_gained.set(rb.gained_focus);
    };
    tb.render(r);
    tb.sequence().key(KeyCode::Tab, r).tick(r).run();
    show_banner.set(true);
    tb.render(r);
    assert!(!a_gained.get());
    assert!(!b_lost.get());
    assert!(!b_gained.get());
}

#[test]
fn state_rebuilt_every_frame_keeps_positional_focus() {
    // A state created inside the closure gets a fresh key each frame; focus
    // must not be lost (the pre-0.26 positional behaviour applies).
    let mut tb = TestBackend::new(30, 6);
    let focused = Cell::new(false);
    let r = |ui: &mut Context| {
        let mut scratch = TextInputState::new();
        focused.set(ui.text_input(&mut scratch).focused);
    };
    for _ in 0..4 {
        tb.render(r);
        assert!(focused.get());
    }
}

#[test]
fn click_focus_maps_through_identity() {
    let mut tb = TestBackend::new(40, 12);
    let a = RefCell::new(TextInputState::new());
    let b = RefCell::new(TextInputState::new());
    b.borrow_mut().placeholder = "second".into();
    let r = |ui: &mut Context| {
        let _ = ui.text_input(&mut a.borrow_mut());
        let _ = ui.text_input(&mut b.borrow_mut());
    };
    tb.render(r);
    tb.render(r);
    let (x, y) = tb.find_text("second").expect("placeholder rendered");
    step(&mut tb, EventBuilder::new().click(x, y).build(), r);
    tb.type_string("q", r);
    assert_eq!(a.borrow().value, "");
    assert_eq!(b.borrow().value, "q");
}
