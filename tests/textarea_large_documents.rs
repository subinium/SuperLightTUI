//! Issue #415: large-document textarea behaviour — delta undo records,
//! the undo byte budget and the cross-frame soft-wrap layout cache.

#![allow(unused_must_use)]

use slt::event::Event;
use slt::widgets::{TextInputState, TextareaState};
use slt::{EventBuilder, KeyCode, KeyModifiers, TestBackend};

fn chord(ch: char) -> Event {
    EventBuilder::new()
        .key_with(KeyCode::Char(ch), KeyModifiers::CONTROL)
        .build()
        .remove(0)
}

fn paste(text: &str) -> Event {
    EventBuilder::new().paste(text).build().remove(0)
}

fn document(bytes: usize) -> String {
    let row = format!("{}\n", "a".repeat(40));
    let mut text = row.repeat(bytes / row.len());
    text.pop();
    text
}

fn frame(tb: &mut TestBackend, state: &mut TextareaState, events: Vec<Event>) {
    tb.render_with_events(events, 0, 1, |ui| {
        ui.textarea(state, 5);
    });
}

#[test]
fn undo_history_retains_edit_records_not_documents() {
    let mut tb = TestBackend::new(60, 6);
    let mut state = TextareaState::new();
    state.set_value(document(200_000));
    state.cursor_row = state.lines.len() / 2;
    state.cursor_col = 10;
    let original = state.lines.clone();

    frame(&mut tb, &mut state, vec![paste("x"); 100]);
    let edited = state.lines.clone();
    assert_eq!(state.history_len(), 100);
    // A snapshot history retained 100 copies of the 200 KB document here.
    assert!(
        state.history_bytes() < 100 * 256,
        "undo records retain {} bytes",
        state.history_bytes()
    );

    frame(&mut tb, &mut state, vec![chord('z'); 100]);
    assert_eq!(state.lines, original);
    frame(&mut tb, &mut state, vec![chord('y'); 100]);
    assert_eq!(state.lines, edited);
}

#[test]
fn byte_budget_drops_oldest_paste_steps_and_keeps_newest() {
    let mut tb = TestBackend::new(60, 6);
    let mut state = TextareaState::new().history_max_bytes(64 * 1024);
    assert_eq!(state.history_bytes_cap(), 64 * 1024);
    let payload = format!("\n{}", document(40_000));
    let mut before_each = Vec::new();
    for _ in 0..5 {
        before_each.push(state.lines.clone());
        frame(&mut tb, &mut state, vec![paste(&payload)]);
    }
    // Each step after the first holds a ~40 KB record: one fits the budget.
    assert_eq!(state.history_len(), 2);
    assert!(state.history_bytes() <= 64 * 1024);
    frame(&mut tb, &mut state, vec![chord('z')]);
    assert_eq!(state.lines, before_each[4]);
    frame(&mut tb, &mut state, vec![chord('z')]);
    assert_eq!(state.lines, before_each[3]);
    frame(&mut tb, &mut state, vec![chord('z')]);
    assert_eq!(state.lines, before_each[3], "older steps were evicted");

    // A single step above the budget is still undoable.
    let mut state = TextareaState::new().history_max_bytes(16);
    frame(&mut tb, &mut state, vec![paste("first"), paste(&payload)]);
    assert_eq!(state.history_len(), 1);
    frame(&mut tb, &mut state, vec![chord('z')]);
    assert_eq!(state.lines, vec!["first"]);
}

/// #374 with delta history: a rejected paste neither records a step nor
/// changes the retained records.
#[test]
fn rejected_paste_leaves_delta_history_untouched() {
    let mut tb = TestBackend::new(60, 6);
    let mut state = TextareaState::new().max_length(5);
    frame(&mut tb, &mut state, vec![paste("abc"), paste("de")]);
    let (len, bytes) = (state.history_len(), state.history_bytes());
    frame(&mut tb, &mut state, vec![paste("overflow")]);
    assert_eq!(state.lines, vec!["abcde"]);
    assert_eq!((state.history_len(), state.history_bytes()), (len, bytes));
    frame(&mut tb, &mut state, vec![chord('z')]);
    assert_eq!(state.lines, vec!["abc"]);
}

#[test]
fn text_input_undo_restores_large_value_exactly() {
    let mut tb = TestBackend::new(40, 3);
    let mut state = TextInputState::new();
    let original = format!("{}\u{d55c}\u{1f469}\u{200d}\u{1f4bb}", "a".repeat(200_000));
    state.set_value(original.clone());
    let backspace = EventBuilder::new()
        .key_code(KeyCode::Backspace)
        .build()
        .remove(0);
    tb.render_with_events(vec![backspace; 50], 0, 1, |ui| {
        ui.text_input(&mut state);
    });
    let edited = state.value.clone();
    assert_eq!(edited.len(), 200_000 - 48);
    tb.render_with_events(vec![chord('z'); 50], 0, 1, |ui| {
        ui.text_input(&mut state);
    });
    assert_eq!(state.value, original);
    tb.render_with_events(vec![chord('y'); 50], 0, 1, |ui| {
        ui.text_input(&mut state);
    });
    assert_eq!(state.value, edited);
}

/// The soft-wrap layout is reused across frames, but any change to the
/// public `lines` — even one that keeps every byte length — or to the wrap
/// width re-wraps.
#[test]
fn wrap_layout_follows_direct_line_changes_and_width() {
    let mut tb = TestBackend::new(12, 5);
    let mut state = TextareaState::new().word_wrap(4);
    state.set_value("aaaaaa\nbb");
    let render = |tb: &mut TestBackend, state: &mut TextareaState| {
        tb.render(|ui| {
            ui.textarea(state, 5);
        });
    };
    render(&mut tb, &mut state);
    render(&mut tb, &mut state);
    tb.assert_line_contains(1, "aa");
    tb.assert_line_contains(2, "bb");

    // Same byte length (6), different display width: one visual line now.
    state.lines[0] = "\u{d55c}\u{d55c}".to_string();
    render(&mut tb, &mut state);
    tb.assert_line_contains(0, "\u{d55c}\u{d55c}");
    tb.assert_line_contains(1, "bb");

    state.wrap_width = Some(3);
    render(&mut tb, &mut state);
    tb.assert_line_contains(1, "\u{d55c}");
    tb.assert_line_contains(2, "bb");

    state.lines.push("cc".to_string());
    render(&mut tb, &mut state);
    tb.assert_line_contains(3, "cc");
}
