//! Issue #461: text-input builders, `clear`/`set_value`, and keyed form
//! access.

use slt::widgets::{FormField, FormState, TextInputState, validators};
use slt::{EventBuilder, TestBackend};

#[test]
fn clear_resets_value_cursor_and_validation_so_typing_starts_at_zero() {
    let mut tb = TestBackend::new(30, 6);
    let mut input = TextInputState::new();
    input.add_validator(validators::min_len(10, "too short"));
    input.set_value("hello");
    input.run_validators();
    input.validate_now(|_| Err("bad".into()));
    assert!(input.validation_error.is_some());
    assert!(!input.errors().is_empty());

    input.clear();
    assert_eq!(input.value, "");
    assert_eq!(input.cursor, 0);
    assert_eq!(input.validation_error, None);
    assert!(input.errors().is_empty());

    tb.render(|ui| {
        let _ = ui.text_input(&mut input);
    });
    tb.sequence()
        .events(EventBuilder::new().key('z').build(), |ui| {
            let _ = ui.text_input(&mut input);
        })
        .run();
    assert_eq!(input.value, "z");
    assert_eq!(input.cursor, 1);
    assert_eq!(input.validation_error, None);
}

#[test]
fn clear_keeps_registered_validators_and_placeholder() {
    let mut input = TextInputState::with_placeholder("todo");
    input.add_validator(validators::required("required"));
    input.set_value("x");
    input.clear();
    assert_eq!(input.placeholder, "todo");
    input.run_validators();
    assert_eq!(input.errors(), ["required".to_string()]);
}

#[test]
fn set_value_respects_max_length_by_grapheme_count_and_moves_cursor_to_end() {
    // Family emoji (ZWJ) + flag + combining mark: 3 graphemes, many scalars.
    let mut input = TextInputState::new().max_length(3);
    input.set_value("👨‍👩‍👧🇰🇷e\u{301}xyz");
    assert_eq!(input.value, "👨‍👩‍👧🇰🇷e\u{301}");
    assert_eq!(input.cursor, 3);

    let mut unlimited = TextInputState::new();
    unlimited.set_value("한국어");
    assert_eq!(unlimited.cursor, 3);

    let mut single_line = TextInputState::new();
    single_line.set_value("a\nb\r\nc");
    assert_eq!(single_line.value, "a b c");
}

#[test]
fn masked_and_password_builders_render_bullets() {
    let mut tb = TestBackend::new(40, 12);
    let mut a = TextInputState::password();
    a.set_value("abc");
    let mut b = TextInputState::with_placeholder("pin").masked(true);
    b.set_value("1234");
    let mut field = FormField::new("Password").masked(true);
    field.input.set_value("xyz12");
    assert!(a.masked && b.masked && field.input.masked);
    assert!(FormField::new("p").password().input.masked);
    tb.render(|ui| {
        let _ = ui.text_input(&mut a);
        let _ = ui.text_input(&mut b);
        ui.form_field(&mut field);
    });
    tb.assert_contains("•••••");
    tb.assert_not_contains("abc");
    tb.assert_not_contains("1234");
    tb.assert_not_contains("xyz12");
}

#[test]
fn form_get_reads_by_key_after_fields_are_reordered() {
    let mut form = FormState::new()
        .field(FormField::new("Email").key("email"))
        .field(FormField::new("Name"));
    form.field_mut("email")
        .unwrap()
        .input
        .set_value("me@example.com");
    form.field_mut("Name").unwrap().input.set_value("Ada");
    assert_eq!(form.get("email"), Some("me@example.com"));

    form.fields.reverse();
    assert_eq!(form.value(0), "Ada", "positional access moved");
    assert_eq!(form.get("email"), Some("me@example.com"));
    assert_eq!(form.get("Name"), Some("Ada"));
    assert_eq!(
        form.get("Email"),
        None,
        "an explicit key replaces the label"
    );
    assert_eq!(form.get("missing"), None);
}

#[test]
fn explicit_keys_win_over_labels() {
    // A field labelled "email" must not shadow the field keyed "email".
    let form = FormState::new()
        .field(FormField::new("email"))
        .field(FormField::new("E-Mail").key("email"));
    let mut form = form;
    form.fields[1].input.set_value("keyed");
    assert_eq!(form.get("email"), Some("keyed"));
    assert_eq!(form.fields[1].lookup_key(), "email");
    assert_eq!(form.fields[0].lookup_key(), "email");
}
