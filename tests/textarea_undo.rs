//! Regression tests for the [`TextareaState`] undo/redo history stack
//! introduced in v0.20.0 (issue #102).
//!
//! Ctrl+Z walks the history backward; Ctrl+Y walks it forward. Rapid char
//! typing coalesces into one undoable batch — only the first char of a
//! burst pushes a snapshot. The history is capped at `history_max`
//! (default 100) by evicting the oldest snapshot.

#![allow(unused_must_use)]

use slt::widgets::TextareaState;
use slt::{EventBuilder, KeyCode, KeyModifiers, TestBackend};

fn ctrl_z() -> Vec<slt::event::Event> {
    EventBuilder::new()
        .key_with(KeyCode::Char('z'), KeyModifiers::CONTROL)
        .build()
}

fn ctrl_y() -> Vec<slt::event::Event> {
    EventBuilder::new()
        .key_with(KeyCode::Char('y'), KeyModifiers::CONTROL)
        .build()
}

#[test]
fn textarea_undo_redo_roundtrip() {
    // Type "hi", undo → empty, redo → "hi" restored.
    let mut tb = TestBackend::new(40, 10);
    let mut state = TextareaState::new();

    let typing = EventBuilder::new().key('h').key('i').build();
    tb.render_with_events(typing, 0, 1, |ui| {
        ui.textarea(&mut state, 5);
    });
    assert_eq!(state.lines, vec!["hi"]);

    tb.render_with_events(ctrl_z(), 0, 1, |ui| {
        ui.textarea(&mut state, 5);
    });
    assert_eq!(
        state.lines,
        vec![""],
        "after Ctrl+Z the typing burst should fully undo"
    );

    tb.render_with_events(ctrl_y(), 0, 1, |ui| {
        ui.textarea(&mut state, 5);
    });
    assert_eq!(
        state.lines,
        vec!["hi"],
        "Ctrl+Y should redo the undone burst"
    );
}

#[test]
fn textarea_rapid_typing_coalesces_into_one_undo() {
    // Five chars typed in a single burst should undo as one unit, not five.
    let mut tb = TestBackend::new(40, 10);
    let mut state = TextareaState::new();

    let typing = EventBuilder::new()
        .key('h')
        .key('e')
        .key('l')
        .key('l')
        .key('o')
        .build();
    tb.render_with_events(typing, 0, 1, |ui| {
        ui.textarea(&mut state, 5);
    });
    assert_eq!(state.lines, vec!["hello"]);

    tb.render_with_events(ctrl_z(), 0, 1, |ui| {
        ui.textarea(&mut state, 5);
    });
    assert_eq!(
        state.lines,
        vec![""],
        "one Ctrl+Z should clear the entire typing burst"
    );
}

#[test]
fn textarea_undo_past_beginning_is_noop() {
    // Pressing Ctrl+Z on a state that was never edited must not panic and
    // must leave content unchanged.
    let mut tb = TestBackend::new(40, 10);
    let mut state = TextareaState::new();

    let events = EventBuilder::new()
        .key_with(KeyCode::Char('z'), KeyModifiers::CONTROL)
        .key_with(KeyCode::Char('z'), KeyModifiers::CONTROL)
        .key_with(KeyCode::Char('z'), KeyModifiers::CONTROL)
        .build();
    tb.render_with_events(events, 0, 1, |ui| {
        ui.textarea(&mut state, 5);
    });
    assert_eq!(state.lines, vec![""]);
    assert_eq!(state.cursor_row, 0);
    assert_eq!(state.cursor_col, 0);
}

#[test]
fn textarea_history_capped_at_max() {
    // Each Backspace + Char pair pushes two snapshots. With history_max = 4
    // and many edits, the stack must stay bounded.
    let mut tb = TestBackend::new(40, 10);
    let mut state = TextareaState::new().history_max(4);

    // 20 distinct edits: alternate Char insert, Enter, Backspace.
    let mut builder = EventBuilder::new();
    for ch in ['a', 'b', 'c', 'd', 'e'] {
        builder = builder
            .key(ch)
            .key_code(KeyCode::Enter)
            .key_code(KeyCode::Backspace)
            .key_code(KeyCode::Backspace);
    }
    let events = builder.build();
    tb.render_with_events(events, 0, 1, |ui| {
        ui.textarea(&mut state, 5);
    });

    assert!(
        state.history_len() <= state.history_cap(),
        "history_len()={} exceeded history_cap()={}",
        state.history_len(),
        state.history_cap()
    );
    assert_eq!(state.history_cap(), 4);

    tb.render_with_events(ctrl_z(), 0, 1, |ui| {
        ui.textarea(&mut state, 5);
    });
    assert!(state.history_len() <= state.history_cap());
    tb.render_with_events(ctrl_y(), 0, 1, |ui| {
        ui.textarea(&mut state, 5);
    });
    assert!(state.history_len() <= state.history_cap());
}

#[test]
fn textarea_history_cap_zero_disables_undo() {
    let mut tb = TestBackend::new(40, 10);
    let mut state = TextareaState::new().history_max(0);
    tb.render_with_events(EventBuilder::new().key('x').build(), 0, 1, |ui| {
        ui.textarea(&mut state, 5);
    });
    assert_eq!(state.history_len(), 0);

    tb.render_with_events(ctrl_z(), 0, 1, |ui| {
        ui.textarea(&mut state, 5);
    });
    assert_eq!(state.lines, vec!["x"]);
    assert_eq!(state.history_len(), 0);
}

#[test]
fn textarea_history_cap_one_preserves_live_redo_tip() {
    let mut tb = TestBackend::new(40, 10);
    let mut state = TextareaState::new().history_max(1);
    tb.render_with_events(EventBuilder::new().key('a').key('b').build(), 0, 1, |ui| {
        ui.textarea(&mut state, 5);
    });
    assert_eq!(state.lines, vec!["ab"]);
    assert_eq!(state.history_len(), 1);

    tb.render_with_events(ctrl_z(), 0, 1, |ui| {
        ui.textarea(&mut state, 5);
    });
    assert_eq!(state.lines, vec![""]);
    assert_eq!(state.history_len(), 1);

    tb.render_with_events(ctrl_y(), 0, 1, |ui| {
        ui.textarea(&mut state, 5);
    });
    assert_eq!(state.lines, vec!["ab"]);
    assert_eq!(state.history_len(), 1);
}

#[test]
fn textarea_redo_invalidated_by_new_edit() {
    // After undoing, a fresh edit must drop the redo tail — a subsequent
    // Ctrl+Y is a no-op.
    let mut tb = TestBackend::new(40, 10);
    let mut state = TextareaState::new();

    let typing = EventBuilder::new().key('a').build();
    tb.render_with_events(typing, 0, 1, |ui| {
        ui.textarea(&mut state, 5);
    });
    assert_eq!(state.lines, vec!["a"]);

    tb.render_with_events(ctrl_z(), 0, 1, |ui| {
        ui.textarea(&mut state, 5);
    });
    assert_eq!(state.lines, vec![""]);

    // New edit replaces the redo branch.
    let new_typing = EventBuilder::new().key('b').build();
    tb.render_with_events(new_typing, 0, 1, |ui| {
        ui.textarea(&mut state, 5);
    });
    assert_eq!(state.lines, vec!["b"]);

    // Ctrl+Y should not bring back "a".
    tb.render_with_events(ctrl_y(), 0, 1, |ui| {
        ui.textarea(&mut state, 5);
    });
    assert_eq!(
        state.lines,
        vec!["b"],
        "redo should not resurrect a branch invalidated by a new edit"
    );
}
