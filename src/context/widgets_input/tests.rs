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

// ── Form field validation triggers ────────────────────────────────────────

use crate::widgets::{validators, FormField, ValidateTrigger};

/// Render one form field (focusable #0) followed by a button (focusable #1).
/// `focus_index` selects which is focused; `prev_focus_count` is 2.
fn render_field_with_focus(backend: &mut TestBackend, field: &mut FormField, focus_index: usize) {
    backend.render_with_events(Vec::new(), focus_index, 2, |ui| {
        ui.form_field(field);
        let _ = ui.button("next");
    });
}

#[test]
fn form_field_on_blur_validates_only_after_focus_leaves() {
    let mut backend = TestBackend::new(40, 10);
    let mut field = FormField::new("Name").validate(validators::required("Name is required"));
    field.input.value = String::new(); // invalid (empty)

    // Frame 0: field focused (index 0). OnBlur must NOT validate yet.
    render_field_with_focus(&mut backend, &mut field, 0);
    assert_eq!(field.error, None);
    assert!(!backend.to_string_trimmed().contains("Name is required"));

    // Frame 1: focus moves to the button (index 1) -> field loses focus and
    // validation runs *after* the field paints, so `error` is set now but the
    // message only appears on the next paint.
    render_field_with_focus(&mut backend, &mut field, 1);
    assert_eq!(field.error.as_deref(), Some("Name is required"));

    // Frame 2: re-render with the error present -> the message is shown.
    render_field_with_focus(&mut backend, &mut field, 1);
    backend.assert_contains("Name is required");
}

#[test]
fn form_field_on_change_validates_immediately_and_clears() {
    let mut backend = TestBackend::new(40, 12);
    let mut field = FormField::new("Email")
        .validate(validators::email())
        .on_change();
    assert_eq!(field.trigger, ValidateTrigger::OnChange);

    // First frame: register the field as the sole focusable so subsequent
    // keystrokes are routed to it.
    backend.render(|ui| {
        ui.form_field(&mut field);
    });

    // Type an invalid value one char at a time; the field is the only
    // focusable so it stays focused and each keystroke sets `changed`.
    backend.type_string("abc", |ui| {
        ui.form_field(&mut field);
    });
    assert_eq!(field.input.value, "abc");
    assert_eq!(field.error.as_deref(), Some("invalid email"));
    // Re-render so the error set after the last keystroke's paint is shown.
    backend.render(|ui| {
        ui.form_field(&mut field);
    });
    backend.assert_contains("invalid email");

    // Continue typing to a valid address; OnChange clears the error.
    backend.type_string("@b.co", |ui| {
        ui.form_field(&mut field);
    });
    assert_eq!(field.input.value, "abc@b.co");
    assert_eq!(field.error, None);
    // Re-render so the cleared error is reflected in the painted buffer.
    backend.render(|ui| {
        ui.form_field(&mut field);
    });
    assert!(!backend.to_string_trimmed().contains("invalid email"));
}

#[test]
fn form_field_manual_never_auto_validates() {
    let mut backend = TestBackend::new(40, 10);
    let mut field = FormField::new("Name")
        .validate(validators::required("required"))
        .manual();
    field.input.value = String::new(); // invalid

    // Focus, then blur — manual must not validate on either.
    render_field_with_focus(&mut backend, &mut field, 0);
    render_field_with_focus(&mut backend, &mut field, 1);
    assert_eq!(field.error, None);
    assert!(!backend.to_string_trimmed().contains("required"));

    // Explicit run populates the error.
    assert!(!field.run_validators());
    assert_eq!(field.error.as_deref(), Some("required"));
}
