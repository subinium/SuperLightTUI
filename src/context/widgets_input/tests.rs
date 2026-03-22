use super::*;
use crate::{EventBuilder, KeyCode, TestBackend};

#[test]
fn text_input_shows_matched_suggestions_for_prefix() {
    let mut backend = TestBackend::new(40, 10);
    let mut input = TextInputState::new();
    input.set_suggestions(vec!["hello".into(), "help".into(), "world".into()]);

    let events = EventBuilder::new().key('h').key('e').key('l').build();
    backend.run_with_events(events, |ui| {
        let _ = ui.text_input(&mut input);
    });

    backend.assert_contains("hello");
    backend.assert_contains("help");
    assert!(!backend.to_string_trimmed().contains("world"));
    assert_eq!(input.matched_suggestions().len(), 2);
}

#[test]
fn text_input_tab_accepts_top_suggestion() {
    let mut backend = TestBackend::new(40, 10);
    let mut input = TextInputState::new();
    input.set_suggestions(vec!["hello".into(), "help".into(), "world".into()]);

    let events = EventBuilder::new()
        .key('h')
        .key('e')
        .key('l')
        .key_code(KeyCode::Tab)
        .build();
    backend.run_with_events(events, |ui| {
        let _ = ui.text_input(&mut input);
    });

    assert_eq!(input.value, "hello");
    assert!(!input.show_suggestions);
}

#[test]
fn text_input_empty_value_shows_no_suggestions() {
    let mut backend = TestBackend::new(40, 10);
    let mut input = TextInputState::new();
    input.set_suggestions(vec!["hello".into(), "help".into(), "world".into()]);

    backend.render(|ui| {
        let _ = ui.text_input(&mut input);
    });

    let rendered = backend.to_string_trimmed();
    assert!(!rendered.contains("hello"));
    assert!(!rendered.contains("help"));
    assert!(!rendered.contains("world"));
    assert!(input.matched_suggestions().is_empty());
    assert!(!input.show_suggestions);
}
