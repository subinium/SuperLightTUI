//! Issue #453: global shortcuts must not fire while a text-entry widget owns
//! focus, regardless of where the shortcut check sits in the frame closure.

use std::cell::{Cell, RefCell};

use slt::widgets::{CommandPaletteState, PaletteCommand, TextInputState, TextareaState};
use slt::{EventBuilder, KeyCode, KeyModifiers, TestBackend};

fn q_batch() -> Vec<slt::Event> {
    EventBuilder::new().key('q').build()
}

/// Feed one event batch without resetting the carried focus state (unlike
/// `TestBackend::run_with_events`, which forces focus back to slot 0).
fn step(tb: &mut TestBackend, events: Vec<slt::Event>, f: impl FnOnce(&mut slt::Context)) {
    tb.sequence().events(events, f).run();
}

#[test]
fn key_before_text_input_does_not_fire_while_input_is_focused() {
    let mut tb = TestBackend::new(30, 5);
    let input = RefCell::new(TextInputState::new());
    let seen = Cell::new(true);
    tb.render(|ui| {
        let _ = ui.text_input(&mut input.borrow_mut());
    });
    tb.type_string("q", |ui| {
        seen.set(ui.key('q'));
        let _ = ui.text_input(&mut input.borrow_mut());
    });
    assert!(!seen.get(), "shortcut before the input must not see 'q'");
    assert_eq!(input.borrow().value, "q");
}

#[test]
fn key_after_text_input_does_not_fire_while_input_is_focused() {
    let mut tb = TestBackend::new(30, 5);
    let input = RefCell::new(TextInputState::new());
    let seen = Cell::new(true);
    tb.render(|ui| {
        let _ = ui.text_input(&mut input.borrow_mut());
    });
    tb.type_string("q", |ui| {
        let _ = ui.text_input(&mut input.borrow_mut());
        seen.set(ui.key('q'));
    });
    assert!(!seen.get());
    assert_eq!(input.borrow().value, "q");
}

#[test]
fn key_fires_when_focus_is_on_a_button() {
    let mut tb = TestBackend::new(30, 6);
    let input = RefCell::new(TextInputState::new());
    let seen = Cell::new(false);
    let render = |ui: &mut slt::Context| {
        seen.set(ui.key('q'));
        let _ = ui.button("OK");
        let _ = ui.text_input(&mut input.borrow_mut());
    };
    tb.render(render);
    step(&mut tb, q_batch(), &render);
    assert!(seen.get(), "button focus must not withhold 'q'");
    assert_eq!(input.borrow().value, "");
}

#[test]
fn ctrl_chord_reaches_key_mod_while_text_input_is_focused() {
    let mut tb = TestBackend::new(30, 5);
    let input = RefCell::new(TextInputState::new());
    let before = Cell::new(false);
    let after = Cell::new(false);
    let render = |ui: &mut slt::Context| {
        before.set(ui.key_mod('q', KeyModifiers::CONTROL));
        let _ = ui.text_input(&mut input.borrow_mut());
        after.set(ui.key_mod('q', KeyModifiers::CONTROL));
    };
    tb.render(render);
    step(
        &mut tb,
        EventBuilder::new()
            .key_with(KeyCode::Char('q'), KeyModifiers::CONTROL)
            .build(),
        &render,
    );
    assert!(before.get() && after.get(), "#349 chord pass-through");
    assert_eq!(input.borrow().value, "");
}

#[test]
fn raw_key_sees_presses_while_text_input_is_focused() {
    let mut tb = TestBackend::new(30, 5);
    let input = RefCell::new(TextInputState::new());
    let raw = Cell::new(false);
    let render = |ui: &mut slt::Context| {
        raw.set(ui.raw_key('q'));
        let _ = ui.text_input(&mut input.borrow_mut());
    };
    tb.render(render);
    step(&mut tb, q_batch(), &render);
    assert!(raw.get());
    assert_eq!(input.borrow().value, "q", "the input still inserts");
}

#[test]
fn shortcut_is_order_independent_and_exact() {
    let mut tb = TestBackend::new(30, 5);
    let input = RefCell::new(TextInputState::new());
    let plain = Cell::new(true);
    let save = Cell::new(false);
    let wrong_mods = Cell::new(true);
    let render = |ui: &mut slt::Context| {
        plain.set(ui.shortcut(KeyCode::Char('s'), KeyModifiers::NONE));
        save.set(ui.shortcut(KeyCode::Char('s'), KeyModifiers::CONTROL));
        wrong_mods.set(ui.shortcut(KeyCode::Char('s'), KeyModifiers::ALT));
        let _ = ui.text_input(&mut input.borrow_mut());
    };
    tb.render(render);
    step(
        &mut tb,
        EventBuilder::new()
            .key('s')
            .key_with(KeyCode::Char('s'), KeyModifiers::CONTROL)
            .build(),
        &render,
    );
    assert!(!plain.get(), "typed 's' belongs to the input");
    assert!(save.get(), "Ctrl+S chord reaches the shortcut");
    assert!(!wrong_mods.get(), "modifiers must match exactly");
    assert_eq!(input.borrow().value, "s");
}

#[test]
fn consume_key_before_input_cannot_steal_typed_text() {
    let mut tb = TestBackend::new(30, 5);
    let input = RefCell::new(TextInputState::new());
    let stolen = Cell::new(true);
    let render = |ui: &mut slt::Context| {
        stolen.set(ui.consume_key('q'));
        let _ = ui.text_input(&mut input.borrow_mut());
    };
    tb.render(render);
    step(&mut tb, q_batch(), &render);
    assert!(!stolen.get());
    assert_eq!(input.borrow().value, "q");
}

#[test]
fn keys_the_input_never_edits_with_stay_visible() {
    let mut tb = TestBackend::new(30, 5);
    let input = RefCell::new(TextInputState::new());
    let esc = Cell::new(false);
    let f1 = Cell::new(false);
    let enter = Cell::new(false);
    let left = Cell::new(true);
    let render = |ui: &mut slt::Context| {
        esc.set(ui.key_code(KeyCode::Esc));
        f1.set(ui.key_code(KeyCode::F(1)));
        enter.set(ui.key_code(KeyCode::Enter));
        left.set(ui.key_code(KeyCode::Left));
        let _ = ui.text_input(&mut input.borrow_mut());
    };
    tb.render(render);
    step(
        &mut tb,
        EventBuilder::new()
            .key_code(KeyCode::Esc)
            .key_code(KeyCode::F(1))
            .key_code(KeyCode::Enter)
            .key_code(KeyCode::Left)
            .build(),
        &render,
    );
    assert!(esc.get() && f1.get());
    assert!(
        enter.get(),
        "single-line inputs do not own Enter for key_code"
    );
    assert!(!left.get(), "cursor keys belong to the focused input");
}

#[test]
fn key_does_not_fire_while_textarea_is_focused_in_either_order() {
    for shortcut_first in [true, false] {
        let mut tb = TestBackend::new(30, 6);
        let area = RefCell::new(TextareaState::new());
        let seen = Cell::new(true);
        let enter = Cell::new(true);
        let raw = Cell::new(false);
        let ctrl = Cell::new(false);
        let render = |ui: &mut slt::Context| {
            if shortcut_first {
                seen.set(ui.key('q'));
                enter.set(ui.key_code(KeyCode::Enter));
                raw.set(ui.raw_key('q'));
                ctrl.set(ui.key_mod('w', KeyModifiers::CONTROL));
            }
            let _ = ui.textarea(&mut area.borrow_mut(), 3);
            if !shortcut_first {
                seen.set(ui.key('q'));
                enter.set(ui.key_code(KeyCode::Enter));
                raw.set(ui.raw_key('q'));
                ctrl.set(ui.key_mod('w', KeyModifiers::CONTROL));
            }
        };
        tb.render(render);
        step(
            &mut tb,
            EventBuilder::new()
                .key('q')
                .key_code(KeyCode::Enter)
                .key_with(KeyCode::Char('w'), KeyModifiers::CONTROL)
                .build(),
            &render,
        );
        assert!(!seen.get(), "shortcut_first={shortcut_first}");
        assert!(
            !enter.get(),
            "textarea owns Enter (shortcut_first={shortcut_first})"
        );
        assert!(
            ctrl.get(),
            "Ctrl chord passes through (shortcut_first={shortcut_first})"
        );
        if shortcut_first {
            assert!(raw.get(), "raw_key ignores text-entry ownership");
        }
        assert_eq!(area.borrow().value(), "q\n");
    }
}

#[test]
fn key_does_not_fire_while_command_palette_is_open() {
    let mut tb = TestBackend::new(60, 20);
    let palette = RefCell::new(CommandPaletteState::new(vec![PaletteCommand::new(
        "Quit", "exit",
    )]));
    palette.borrow_mut().open = true;
    let seen = Cell::new(true);
    let raw = Cell::new(false);
    let ctrl = Cell::new(false);
    let render = |ui: &mut slt::Context| {
        seen.set(ui.key('q'));
        raw.set(ui.raw_key('q'));
        ctrl.set(ui.raw_key_mod('q', KeyModifiers::CONTROL));
        let _ = ui.command_palette(&mut palette.borrow_mut());
    };
    tb.render(render);
    step(
        &mut tb,
        EventBuilder::new()
            .key('q')
            .key_with(KeyCode::Char('q'), KeyModifiers::CONTROL)
            .build(),
        &render,
    );
    assert!(!seen.get());
    assert!(raw.get());
    assert!(ctrl.get());
    assert_eq!(palette.borrow().input, "q");
}

#[test]
fn click_that_moves_focus_off_the_input_releases_keys_in_the_same_batch() {
    // Pins the chosen latency rule: ownership follows the slot that holds
    // focus as the frame starts, which already reflects a click in the same
    // event batch.
    let mut tb = TestBackend::new(40, 8);
    let input = RefCell::new(TextInputState::new());
    let seen = Cell::new(false);
    let render = |ui: &mut slt::Context| {
        seen.set(ui.key('q'));
        let _ = ui.text_input(&mut input.borrow_mut());
        let _ = ui.button("Apply");
    };
    tb.render(render);
    tb.render(render);
    let (x, y) = tb.find_text("Apply").expect("button rendered");
    step(
        &mut tb,
        EventBuilder::new().click(x, y).key('q').build(),
        &render,
    );
    assert!(seen.get(), "focus left the input before the 'q' press");
    assert_eq!(input.borrow().value, "");
}

#[test]
fn tab_away_from_the_input_releases_keys_next_frame() {
    let mut tb = TestBackend::new(40, 8);
    let input = RefCell::new(TextInputState::new());
    let seen = Cell::new(false);
    let render = |ui: &mut slt::Context| {
        seen.set(ui.key('q'));
        let _ = ui.text_input(&mut input.borrow_mut());
        let _ = ui.button("Apply");
    };
    tb.render(render);
    step(
        &mut tb,
        EventBuilder::new().key_code(KeyCode::Tab).build(),
        &render,
    );
    step(&mut tb, q_batch(), &render);
    assert!(seen.get());
    assert_eq!(input.borrow().value, "");
}
