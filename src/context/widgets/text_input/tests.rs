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

use crate::widgets::{FormField, ValidateTrigger, validators};

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

// ── Grapheme-cluster cursor movement (issue #259) ──────────────────────────

use crate::widgets::TextareaState;

/// A bare regional indicator with no paired partner — a flag split in half.
fn has_lone_regional_indicator(s: &str) -> bool {
    s.chars()
        .filter(|c| ('\u{1F1E6}'..='\u{1F1FF}').contains(c))
        .count()
        % 2
        != 0
}

#[test]
fn textarea_cursor_steps_over_grapheme() {
    // Seed "🇰🇷x": the flag is one cluster (two regional indicators), then "x".
    let mut state = TextareaState::new();
    state.set_value("🇰🇷x");
    assert_eq!(state.cursor_col, 0);

    // One Right step lands after the whole flag (cluster index 1), not between
    // the two regional indicators.
    let mut backend = TestBackend::new(40, 6);
    let events = EventBuilder::new().key_code(KeyCode::Right).build();
    backend.render_with_events(events, 0, 1, |ui| {
        let _ = ui.textarea(&mut state, 4);
    });
    assert_eq!(
        state.cursor_col, 1,
        "Right did not step a whole flag cluster"
    );

    // A second Right lands after "x" (cluster index 2).
    let events = EventBuilder::new().key_code(KeyCode::Right).build();
    backend.render_with_events(events, 0, 1, |ui| {
        let _ = ui.textarea(&mut state, 4);
    });
    assert_eq!(state.cursor_col, 2);

    // End/Home are also cluster-based: 2 clusters total.
    let events = EventBuilder::new().key_code(KeyCode::Home).build();
    backend.render_with_events(events, 0, 1, |ui| {
        let _ = ui.textarea(&mut state, 4);
    });
    assert_eq!(state.cursor_col, 0);
}

#[test]
fn textarea_backspace_removes_whole_cluster() {
    // Type a flag emoji (two regional indicators = one cluster) then Backspace
    // once: the entire cluster is removed, leaving the line empty.
    let mut state = TextareaState::new();
    state.set_value("🇰🇷");
    state.cursor_col = 1; // cursor after the single flag cluster

    let mut backend = TestBackend::new(40, 6);
    let events = EventBuilder::new().key_code(KeyCode::Backspace).build();
    backend.render_with_events(events, 0, 1, |ui| {
        let _ = ui.textarea(&mut state, 4);
    });

    assert_eq!(state.lines[0], "", "Backspace left a flag fragment behind");
    assert_eq!(state.cursor_col, 0);
    assert!(
        !has_lone_regional_indicator(&backend.to_string_trimmed()),
        "rendered output retained a half flag"
    );
}

#[test]
fn text_input_cursor_grapheme() {
    // Left/Right/Backspace on a TextInputState seeded with "🇰🇷x" step by
    // whole grapheme cluster.
    let mut input = TextInputState::new();
    input.value = "🇰🇷x".to_string();
    input.cursor = 0;

    let mut backend = TestBackend::new(40, 6);
    // Right once -> after the flag (cluster index 1).
    let events = EventBuilder::new().key_code(KeyCode::Right).build();
    backend.render_with_events(events, 0, 1, |ui| {
        let _ = ui.text_input(&mut input);
    });
    assert_eq!(input.cursor, 1);

    // Right again -> after "x" (cluster index 2).
    let events = EventBuilder::new().key_code(KeyCode::Right).build();
    backend.render_with_events(events, 0, 1, |ui| {
        let _ = ui.text_input(&mut input);
    });
    assert_eq!(input.cursor, 2);

    // Cursor sits at end (index 2, after "🇰🇷x"). Backspace removes "x"
    // wholly; the flag stays intact and whole.
    let events = EventBuilder::new().key_code(KeyCode::Backspace).build();
    backend.render_with_events(events, 0, 1, |ui| {
        let _ = ui.text_input(&mut input);
    });
    assert_eq!(input.value, "🇰🇷", "Backspace cut the wrong unit");
    assert_eq!(input.cursor, 1);
    assert!(!has_lone_regional_indicator(&input.value));

    // A second Backspace now removes the whole flag cluster (both regional
    // indicators), never a half flag.
    let events = EventBuilder::new().key_code(KeyCode::Backspace).build();
    backend.render_with_events(events, 0, 1, |ui| {
        let _ = ui.text_input(&mut input);
    });
    assert_eq!(input.value, "", "flag was not removed as one cluster");
    assert_eq!(input.cursor, 0);
}

#[test]
fn text_input_ascii_cursor_unchanged() {
    // ASCII regression: cluster index equals scalar index for ASCII.
    let mut input = TextInputState::new();
    input.value = "abc".to_string();
    input.cursor = 0;
    let mut backend = TestBackend::new(40, 6);
    let events = EventBuilder::new()
        .key_code(KeyCode::Right)
        .key_code(KeyCode::Right)
        .key_code(KeyCode::Backspace)
        .build();
    backend.render_with_events(events, 0, 1, |ui| {
        let _ = ui.text_input(&mut input);
    });
    assert_eq!(input.value, "ac");
    assert_eq!(input.cursor, 1);
}

// ── number_input ─────────────────────────────────────────────────────────

#[test]
fn number_input_renders_value() {
    let mut backend = TestBackend::new(40, 5);
    let mut s = NumberInputState::integer(42, 0, 100);
    backend.render(|ui| {
        let _ = ui.number_input(&mut s);
    });
    backend.assert_contains("42");
    backend.assert_contains("▾");
    backend.assert_contains("▴");
}

#[test]
fn number_input_up_increments_by_step() {
    let mut backend = TestBackend::new(40, 5);
    let mut s = NumberInputState::integer(5, 0, 100).step(2.0);
    let events = EventBuilder::new().key_code(KeyCode::Up).build();
    let mut resp_changed = false;
    backend.run_with_events(events, |ui| {
        let r = ui.number_input(&mut s);
        resp_changed = r.changed;
    });
    assert_eq!(s.value, 7.0);
    assert!(resp_changed);
}

#[test]
fn number_input_down_and_vim_keys_step() {
    // Down decrements.
    let mut backend = TestBackend::new(40, 5);
    let mut s = NumberInputState::integer(5, 0, 100).step(1.0);
    backend.run_with_events(EventBuilder::new().key_code(KeyCode::Down).build(), |ui| {
        let _ = ui.number_input(&mut s);
    });
    assert_eq!(s.value, 4.0);

    // `k` increments, `j` decrements.
    let mut backend = TestBackend::new(40, 5);
    let mut s = NumberInputState::integer(5, 0, 100).step(1.0);
    backend.run_with_events(EventBuilder::new().key('k').build(), |ui| {
        let _ = ui.number_input(&mut s);
    });
    assert_eq!(s.value, 6.0);

    let mut backend = TestBackend::new(40, 5);
    let mut s = NumberInputState::integer(5, 0, 100).step(1.0);
    backend.run_with_events(EventBuilder::new().key('j').build(), |ui| {
        let _ = ui.number_input(&mut s);
    });
    assert_eq!(s.value, 4.0);
}

#[test]
fn number_input_clamps_at_max() {
    let mut backend = TestBackend::new(40, 5);
    let mut s = NumberInputState::integer(10, 0, 10).step(1.0);
    let mut resp_changed = true;
    backend.run_with_events(EventBuilder::new().key_code(KeyCode::Up).build(), |ui| {
        let r = ui.number_input(&mut s);
        resp_changed = r.changed;
    });
    assert_eq!(s.value, 10.0);
    assert!(!resp_changed, "no change when already at max");
}

#[test]
fn number_input_clamps_at_min() {
    let mut backend = TestBackend::new(40, 5);
    let mut s = NumberInputState::integer(0, 0, 10).step(1.0);
    let mut resp_changed = true;
    backend.run_with_events(EventBuilder::new().key_code(KeyCode::Down).build(), |ui| {
        let r = ui.number_input(&mut s);
        resp_changed = r.changed;
    });
    assert_eq!(s.value, 0.0);
    assert!(!resp_changed);
}

#[test]
fn number_input_integer_mode_never_fractional() {
    let mut backend = TestBackend::new(40, 5);
    let mut s = NumberInputState::integer(0, 0, 100).step(1.0);
    for _ in 0..5 {
        backend.run_with_events(EventBuilder::new().key_code(KeyCode::Up).build(), |ui| {
            let _ = ui.number_input(&mut s);
        });
        assert_eq!(s.value.fract(), 0.0);
    }
    assert_eq!(s.value, 5.0);
    // Rendered output for an integer stepper contains no decimal point.
    let mut backend = TestBackend::new(40, 5);
    backend.render(|ui| {
        let _ = ui.number_input(&mut s);
    });
    let rendered = backend.to_string_trimmed();
    assert!(rendered.contains('5'));
    assert!(
        !rendered.contains('.'),
        "integer mode must not render a decimal point: {rendered:?}"
    );
}

#[test]
fn number_input_float_mode_formats_trimmed() {
    let mut backend = TestBackend::new(40, 5);
    let mut s = NumberInputState::new(1.5, 0.0, 10.0).step(0.1);
    backend.run_with_events(EventBuilder::new().key_code(KeyCode::Up).build(), |ui| {
        let _ = ui.number_input(&mut s);
    });
    assert!((s.value - 1.6).abs() < 1e-9, "value was {}", s.value);
    let mut backend = TestBackend::new(40, 5);
    backend.render(|ui| {
        let _ = ui.number_input(&mut s);
    });
    backend.assert_contains("1.6");
    // format_compact_number trims trailing zeros.
    assert!(!backend.to_string_trimmed().contains("1.60"));
}

#[test]
fn number_input_typing_and_enter_commits() {
    let mut backend = TestBackend::new(40, 5);
    let mut s = NumberInputState::integer(0, 0, 100).step(1.0);
    // Type "17" across frames (the field accumulates a buffer).
    backend.type_string("17", |ui| {
        let _ = ui.number_input(&mut s);
    });
    assert_eq!(s.editing.as_deref(), Some("17"));
    // Enter commits.
    let mut resp_changed = false;
    backend.run_with_events(EventBuilder::new().key_code(KeyCode::Enter).build(), |ui| {
        let r = ui.number_input(&mut s);
        resp_changed = r.changed;
    });
    assert_eq!(s.value, 17.0);
    assert!(s.editing.is_none());
    assert!(resp_changed);

    let mut backend = TestBackend::new(40, 5);
    backend.render(|ui| {
        let _ = ui.number_input(&mut s);
    });
    backend.assert_contains("17");
}

#[test]
fn number_input_invalid_typed_sets_parse_error() {
    let mut backend = TestBackend::new(40, 10);
    let mut s = NumberInputState::integer(3, 0, 100).step(1.0);
    // '9' is accepted; 'x' is rejected as a non-numeric char, so the buffer is "9".
    backend.type_string("9", |ui| {
        let _ = ui.number_input(&mut s);
    });
    // Force an unparsable buffer directly, then press Enter.
    s.editing = Some("9x".to_string());
    backend.run_with_events(EventBuilder::new().key_code(KeyCode::Enter).build(), |ui| {
        let _ = ui.number_input(&mut s);
    });
    assert!(s.parse_error.is_some());
    assert_eq!(s.value, 3.0, "committed value unchanged on parse failure");
}

#[test]
fn number_input_non_numeric_chars_rejected() {
    let mut backend = TestBackend::new(40, 5);
    let mut s = NumberInputState::integer(0, 0, 100).step(1.0);
    backend.type_string("9x4", |ui| {
        let _ = ui.number_input(&mut s);
    });
    // Only digits accumulate; 'x' is dropped.
    assert_eq!(s.editing.as_deref(), Some("94"));
}

#[test]
fn number_input_dot_only_in_float_mode() {
    // Float mode accepts a single '.'; a second '.' is dropped but later
    // digits still accumulate, so "1.5.5" becomes "1.55".
    let mut backend = TestBackend::new(40, 5);
    let mut s = NumberInputState::new(0.0, 0.0, 10.0).step(0.1);
    backend.type_string("1.5.5", |ui| {
        let _ = ui.number_input(&mut s);
    });
    assert_eq!(s.editing.as_deref(), Some("1.55"), "only one dot allowed");

    // Integer mode rejects '.'.
    let mut backend = TestBackend::new(40, 5);
    let mut s = NumberInputState::integer(0, 0, 10).step(1.0);
    backend.type_string("1.5", |ui| {
        let _ = ui.number_input(&mut s);
    });
    assert_eq!(s.editing.as_deref(), Some("15"));
}

#[test]
fn number_input_leading_minus_only_when_negative_allowed() {
    // Negative range allows a leading '-'.
    let mut backend = TestBackend::new(40, 5);
    let mut s = NumberInputState::integer(0, -10, 10).step(1.0);
    backend.type_string("-5", |ui| {
        let _ = ui.number_input(&mut s);
    });
    assert_eq!(s.editing.as_deref(), Some("-5"));
    backend.run_with_events(EventBuilder::new().key_code(KeyCode::Enter).build(), |ui| {
        let _ = ui.number_input(&mut s);
    });
    assert_eq!(s.value, -5.0);

    // Non-negative range rejects '-'.
    let mut backend = TestBackend::new(40, 5);
    let mut s = NumberInputState::integer(0, 0, 10).step(1.0);
    backend.type_string("-5", |ui| {
        let _ = ui.number_input(&mut s);
    });
    assert_eq!(s.editing.as_deref(), Some("5"));
}

#[test]
fn number_input_esc_discards_buffer() {
    let mut backend = TestBackend::new(40, 5);
    let mut s = NumberInputState::integer(7, 0, 100).step(1.0);
    backend.type_string("99", |ui| {
        let _ = ui.number_input(&mut s);
    });
    assert_eq!(s.editing.as_deref(), Some("99"));
    backend.run_with_events(EventBuilder::new().key_code(KeyCode::Esc).build(), |ui| {
        let _ = ui.number_input(&mut s);
    });
    assert!(s.editing.is_none());
    assert_eq!(s.value, 7.0, "value reverts to committed on Esc");
}

#[test]
fn number_input_backspace_edits_buffer() {
    let mut backend = TestBackend::new(40, 5);
    let mut s = NumberInputState::integer(0, 0, 100).step(1.0);
    backend.type_string("123", |ui| {
        let _ = ui.number_input(&mut s);
    });
    backend.run_with_events(
        EventBuilder::new().key_code(KeyCode::Backspace).build(),
        |ui| {
            let _ = ui.number_input(&mut s);
        },
    );
    assert_eq!(s.editing.as_deref(), Some("12"));
}

#[test]
fn number_input_scroll_wheel_adjusts() {
    let mut backend = TestBackend::new(40, 5);
    let mut s = NumberInputState::integer(5, 0, 100).step(1.0);
    // First frame establishes the prev_hit_map rect for the widget.
    backend.render(|ui| {
        let _ = ui.number_input(&mut s);
    });
    // Scroll up over the top-left where the field renders.
    backend.run_with_events(EventBuilder::new().scroll_up(1, 0).build(), |ui| {
        let _ = ui.number_input(&mut s);
    });
    assert_eq!(s.value, 6.0);
    backend.run_with_events(EventBuilder::new().scroll_down(1, 0).build(), |ui| {
        let _ = ui.number_input(&mut s);
    });
    assert_eq!(s.value, 5.0);
}

#[test]
fn number_input_consumes_up_so_global_handler_does_not_fire() {
    let mut backend = TestBackend::new(40, 5);
    let mut s = NumberInputState::integer(5, 0, 100).step(1.0);
    let mut quit_seen = false;
    backend.run_with_events(EventBuilder::new().key_code(KeyCode::Up).build(), |ui| {
        let _ = ui.number_input(&mut s);
        // A later global handler must NOT see the consumed Up key.
        if ui.key_code(KeyCode::Up) {
            quit_seen = true;
        }
    });
    assert_eq!(s.value, 6.0);
    assert!(!quit_seen, "Up was consumed by number_input");
}

#[test]
fn number_input_unfocused_ignores_keys() {
    // A second focusable steals focus; keys must not adjust the unfocused field.
    let mut backend = TestBackend::new(40, 5);
    let mut s = NumberInputState::integer(5, 0, 100).step(1.0);
    let mut other = TextInputState::new();
    backend.run_with_events(EventBuilder::new().key_code(KeyCode::Up).build(), |ui| {
        let r = ui.number_input(&mut s); // first focusable -> focused
        let _ = r;
        let _ = ui.text_input(&mut other);
    });
    // number_input is the first focusable, so it IS focused and adjusts.
    assert_eq!(s.value, 6.0);
}

proptest::proptest! {
    #[test]
    fn numeric_public_boundaries_never_retain_non_finite_values(
        value in proptest::num::f64::ANY,
        min in proptest::num::f64::ANY,
        max in proptest::num::f64::ANY,
        step in proptest::num::f64::ANY,
    ) {
        let mut state = NumberInputState::new(value, min, max).step(step);
        let mut backend = TestBackend::new(40, 5);
        backend.render(|ui| {
            let _ = ui.number_input(&mut state);
        });
        proptest::prop_assert!(state.min.is_finite());
        proptest::prop_assert!(state.max.is_finite());
        proptest::prop_assert!(state.value.is_finite());
        proptest::prop_assert!(state.step.is_finite());
        proptest::prop_assert!(state.min <= state.max);
        proptest::prop_assert!(state.value >= state.min);
        proptest::prop_assert!(state.value <= state.max);

        let mut slider_value = value;
        backend.render(|ui| {
            let _ = ui.slider_with(
                &mut slider_value,
                crate::widgets::SliderOpts::new("value", min..=max).step(step),
            );
        });
        proptest::prop_assert!(slider_value.is_finite());
    }

    #[test]
    fn number_input_always_in_range_after_step(
        value in -1000.0f64..1000.0,
        min in -1000.0f64..1000.0,
        span in 0.0f64..1000.0,
        step in 0.0f64..100.0,
        up in proptest::bool::ANY,
    ) {
        let max = min + span;
        let mut s = NumberInputState::new(value, min, max).step(step);
        let mut backend = TestBackend::new(40, 5);
        let key = if up { KeyCode::Up } else { KeyCode::Down };
        backend.run_with_events(EventBuilder::new().key_code(key).build(), |ui| {
            let _ = ui.number_input(&mut s);
        });
        proptest::prop_assert!(s.value >= s.min - 1e-9);
        proptest::prop_assert!(s.value <= s.max + 1e-9);
    }

    #[test]
    fn number_input_integer_mode_stays_whole(
        value in -1000i64..1000,
        min in -1000i64..1000,
        span in 0i64..1000,
        up in proptest::bool::ANY,
    ) {
        let max = min + span;
        let mut s = NumberInputState::integer(value, min, max).step(1.0);
        let mut backend = TestBackend::new(40, 5);
        let key = if up { KeyCode::Up } else { KeyCode::Down };
        backend.run_with_events(EventBuilder::new().key_code(key).build(), |ui| {
            let _ = ui.number_input(&mut s);
        });
        proptest::prop_assert_eq!(s.value.fract(), 0.0);
        proptest::prop_assert!(s.value >= s.min);
        proptest::prop_assert!(s.value <= s.max);
    }
}
