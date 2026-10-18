//! Issue #453: global shortcuts must not fire while a text-entry widget owns
//! focus, regardless of where the shortcut check sits in the frame closure.
//!
//! Issue #464: every multi-step test here drives one `FnMut` render closure
//! over plain `&mut` state through `TestBackend::step` / `type_text`, which
//! keep the carried focus state; no `RefCell` or `Cell` is needed.

use slt::widgets::{CommandPaletteState, PaletteCommand, TextInputState, TextareaState};
use slt::{EventBuilder, KeyCode, KeyModifiers, TestBackend};

fn q_batch() -> Vec<slt::Event> {
    EventBuilder::new().key('q').build()
}

#[test]
fn key_before_text_input_does_not_fire_while_input_is_focused() {
    let mut tb = TestBackend::new(30, 5);
    let mut input = TextInputState::new();
    let mut seen = true;
    let mut render = |ui: &mut slt::Context| {
        seen = ui.key('q');
        ui.text_input(&mut input);
    };
    tb.step(vec![], &mut render).type_text("q", &mut render);
    assert!(!seen, "shortcut before the input must not see 'q'");
    assert_eq!(input.value, "q");
}

#[test]
fn key_after_text_input_does_not_fire_while_input_is_focused() {
    let mut tb = TestBackend::new(30, 5);
    let mut input = TextInputState::new();
    let mut seen = true;
    let mut render = |ui: &mut slt::Context| {
        ui.text_input(&mut input);
        seen = ui.key('q');
    };
    tb.step(vec![], &mut render).type_text("q", &mut render);
    assert!(!seen);
    assert_eq!(input.value, "q");
}

#[test]
fn key_fires_when_focus_is_on_a_button() {
    let mut tb = TestBackend::new(30, 6);
    let mut input = TextInputState::new();
    let mut seen = false;
    let mut render = |ui: &mut slt::Context| {
        seen = ui.key('q');
        let _ = ui.button("OK");
        ui.text_input(&mut input);
    };
    tb.step(vec![], &mut render).step(q_batch(), &mut render);
    assert!(seen, "button focus must not withhold 'q'");
    assert_eq!(input.value, "");
}

#[test]
fn ctrl_chord_reaches_key_mod_while_text_input_is_focused() {
    let mut tb = TestBackend::new(30, 5);
    let mut input = TextInputState::new();
    let mut before = false;
    let mut after = false;
    let mut render = |ui: &mut slt::Context| {
        before = ui.key_mod('q', KeyModifiers::CONTROL);
        ui.text_input(&mut input);
        after = ui.key_mod('q', KeyModifiers::CONTROL);
    };
    tb.step(vec![], &mut render).step(
        EventBuilder::new()
            .key_with(KeyCode::Char('q'), KeyModifiers::CONTROL)
            .build(),
        &mut render,
    );
    assert!(before && after, "#349 chord pass-through");
    assert_eq!(input.value, "");
}

#[test]
fn raw_key_sees_presses_while_text_input_is_focused() {
    let mut tb = TestBackend::new(30, 5);
    let mut input = TextInputState::new();
    let mut raw = false;
    let mut render = |ui: &mut slt::Context| {
        raw = ui.raw_key('q');
        ui.text_input(&mut input);
    };
    tb.step(vec![], &mut render).step(q_batch(), &mut render);
    assert!(raw);
    assert_eq!(input.value, "q", "the input still inserts");
}

#[test]
fn shortcut_is_order_independent_and_exact() {
    let mut tb = TestBackend::new(30, 5);
    let mut input = TextInputState::new();
    let mut plain = true;
    let mut save = false;
    let mut wrong_mods = true;
    let mut render = |ui: &mut slt::Context| {
        plain = ui.shortcut(KeyCode::Char('s'), KeyModifiers::NONE);
        save = ui.shortcut(KeyCode::Char('s'), KeyModifiers::CONTROL);
        wrong_mods = ui.shortcut(KeyCode::Char('s'), KeyModifiers::ALT);
        ui.text_input(&mut input);
    };
    tb.step(vec![], &mut render).step(
        EventBuilder::new()
            .key('s')
            .key_with(KeyCode::Char('s'), KeyModifiers::CONTROL)
            .build(),
        &mut render,
    );
    assert!(!plain, "typed 's' belongs to the input");
    assert!(save, "Ctrl+S chord reaches the shortcut");
    assert!(!wrong_mods, "modifiers must match exactly");
    assert_eq!(input.value, "s");
}

#[test]
fn consume_key_before_input_cannot_steal_typed_text() {
    let mut tb = TestBackend::new(30, 5);
    let mut input = TextInputState::new();
    let mut stolen = true;
    let mut render = |ui: &mut slt::Context| {
        stolen = ui.consume_key('q');
        ui.text_input(&mut input);
    };
    tb.step(vec![], &mut render).step(q_batch(), &mut render);
    assert!(!stolen);
    assert_eq!(input.value, "q");
}

#[test]
fn keys_the_input_never_edits_with_stay_visible() {
    let mut tb = TestBackend::new(30, 5);
    let mut input = TextInputState::new();
    let mut esc = false;
    let mut f1 = false;
    let mut enter = false;
    let mut left = true;
    let mut render = |ui: &mut slt::Context| {
        esc = ui.key_code(KeyCode::Esc);
        f1 = ui.key_code(KeyCode::F(1));
        enter = ui.key_code(KeyCode::Enter);
        left = ui.key_code(KeyCode::Left);
        ui.text_input(&mut input);
    };
    tb.step(vec![], &mut render).step(
        EventBuilder::new()
            .key_code(KeyCode::Esc)
            .key_code(KeyCode::F(1))
            .key_code(KeyCode::Enter)
            .key_code(KeyCode::Left)
            .build(),
        &mut render,
    );
    assert!(esc && f1);
    assert!(enter, "single-line inputs do not own Enter for key_code");
    assert!(!left, "cursor keys belong to the focused input");
}

#[test]
fn key_does_not_fire_while_textarea_is_focused_in_either_order() {
    for shortcut_first in [true, false] {
        let mut tb = TestBackend::new(30, 6);
        let mut area = TextareaState::new();
        let mut seen = true;
        let mut enter = true;
        let mut raw = false;
        let mut ctrl = false;
        let mut render = |ui: &mut slt::Context| {
            if shortcut_first {
                seen = ui.key('q');
                enter = ui.key_code(KeyCode::Enter);
                raw = ui.raw_key('q');
                ctrl = ui.key_mod('w', KeyModifiers::CONTROL);
            }
            ui.textarea(&mut area, 3);
            if !shortcut_first {
                seen = ui.key('q');
                enter = ui.key_code(KeyCode::Enter);
                raw = ui.raw_key('q');
                ctrl = ui.key_mod('w', KeyModifiers::CONTROL);
            }
        };
        tb.step(vec![], &mut render).step(
            EventBuilder::new()
                .key('q')
                .key_code(KeyCode::Enter)
                .key_with(KeyCode::Char('w'), KeyModifiers::CONTROL)
                .build(),
            &mut render,
        );
        assert!(!seen, "shortcut_first={shortcut_first}");
        assert!(
            !enter,
            "textarea owns Enter (shortcut_first={shortcut_first})"
        );
        assert!(
            ctrl,
            "Ctrl chord passes through (shortcut_first={shortcut_first})"
        );
        if shortcut_first {
            assert!(raw, "raw_key ignores text-entry ownership");
        }
        assert_eq!(area.value(), "q\n");
    }
}

#[test]
fn key_does_not_fire_while_command_palette_is_open() {
    let mut tb = TestBackend::new(60, 20);
    let mut palette = CommandPaletteState::new(vec![PaletteCommand::new("Quit", "exit")]);
    palette.open = true;
    let mut seen = true;
    let mut raw = false;
    let mut ctrl = false;
    let mut render = |ui: &mut slt::Context| {
        seen = ui.key('q');
        raw = ui.raw_key('q');
        ctrl = ui.raw_key_mod('q', KeyModifiers::CONTROL);
        let _ = ui.command_palette(&mut palette);
    };
    tb.step(vec![], &mut render).step(
        EventBuilder::new()
            .key('q')
            .key_with(KeyCode::Char('q'), KeyModifiers::CONTROL)
            .build(),
        &mut render,
    );
    assert!(!seen);
    assert!(raw);
    assert!(ctrl);
    assert_eq!(palette.input, "q");
}

#[test]
fn click_that_moves_focus_off_the_input_releases_keys_in_the_same_batch() {
    // Pins the chosen latency rule: ownership follows the slot that holds
    // focus as the frame starts, which already reflects a click in the same
    // event batch.
    let mut tb = TestBackend::new(40, 8);
    let mut input = TextInputState::new();
    let mut seen = false;
    let mut render = |ui: &mut slt::Context| {
        seen = ui.key('q');
        ui.text_input(&mut input);
        let _ = ui.button("Apply");
    };
    tb.step(vec![], &mut render).step(vec![], &mut render);
    let (x, y) = tb.find_text("Apply").expect("button rendered");
    tb.step(
        EventBuilder::new().click(x, y).key('q').build(),
        &mut render,
    );
    assert!(seen, "focus left the input before the 'q' press");
    assert_eq!(input.value, "");
}

#[test]
fn tab_away_from_the_input_releases_keys_next_frame() {
    let mut tb = TestBackend::new(40, 8);
    let mut input = TextInputState::new();
    let mut seen = false;
    let mut render = |ui: &mut slt::Context| {
        seen = ui.key('q');
        ui.text_input(&mut input);
        let _ = ui.button("Apply");
    };
    tb.step(vec![], &mut render)
        .press(KeyCode::Tab, &mut render)
        .step(q_batch(), &mut render);
    assert!(seen);
    assert_eq!(input.value, "");
}
