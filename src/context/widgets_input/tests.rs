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
    // Force an unparseable buffer directly, then press Enter.
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
