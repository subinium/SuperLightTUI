//! Demo-parity tests for v0.20.0 hooks/focus demos.
//!
//! These tests don't import the demo binaries (examples are compiled as
//! standalone targets). Instead, they re-run the same UI shape inside a
//! `TestBackend` so any regression in the demo's underlying mechanism
//! shows up as a CI failure here, separate from the demo binary.

use slt::{Border, Color, KeyCode, KeyModifiers, TestBackend};

// ----------------------------------------------------------------
// examples/v020_use_state_keyed.rs — list with per-row counters
// ----------------------------------------------------------------

#[test]
fn demo_use_state_keyed_renders_three_rows_with_independent_state() {
    let mut tb = TestBackend::new(80, 12);

    // Frame 0: render three rows; mutate row 1 only.
    tb.render(|ui| {
        let _ = ui
            .bordered(Border::Rounded)
            .title("keyed demo")
            .p(1)
            .col(|ui| {
                for i in 0..3 {
                    let counter = ui.use_state_keyed(format!("counter-{i}"), || 0i32);
                    if i == 1 {
                        *counter.get_mut(ui) = 7;
                    }
                    let v = *counter.get(ui);
                    ui.text(format!("item {i} = {v}"));
                }
            });
    });
    let s = tb.to_string_trimmed();
    assert!(s.contains("item 0 = 0"), "row 0 should be 0\n{s}");
    assert!(
        s.contains("item 1 = 7"),
        "row 1 must show its mutated value\n{s}"
    );
    assert!(s.contains("item 2 = 0"), "row 2 should remain 0\n{s}");

    // Frame 1: render the same three rows — row 1's value persists.
    tb.render(|ui| {
        let _ = ui.col(|ui| {
            for i in 0..3 {
                let counter = ui.use_state_keyed(format!("counter-{i}"), || 999i32);
                ui.text(format!("item {i} = {}", counter.get(ui)));
            }
        });
    });
    let s = tb.to_string_trimmed();
    assert!(s.contains("item 1 = 7"), "persisted across frames\n{s}");
    // 999 init must NOT have re-run.
    assert!(!s.contains("999"), "init closure should not re-fire\n{s}");
}

// ----------------------------------------------------------------
// examples/v020_use_effect.rs — counter + effect log
// ----------------------------------------------------------------

#[test]
fn demo_use_effect_only_fires_on_dep_change() {
    use std::cell::RefCell;
    use std::rc::Rc;

    let log: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let mut tb = TestBackend::new(80, 8);

    let counts = [0i32, 0, 1, 1, 2, 2, 2];

    for &count in &counts {
        let log_setup = log.clone();
        let log_count = log.clone();
        tb.render(move |ui| {
            ui.use_effect(
                move |_| {
                    log_setup.borrow_mut().push("[setup]".into());
                },
                &(),
            );
            ui.use_effect(
                move |c| {
                    log_count.borrow_mut().push(format!("[count] {c}"));
                },
                &count,
            );
            ui.text(format!("count = {count}"));
        });
    }
    let entries = log.borrow();
    let setup = entries.iter().filter(|s| s.contains("[setup]")).count();
    let count_changes = entries.iter().filter(|s| s.contains("[count]")).count();
    assert_eq!(setup, 1, "setup effect must fire exactly once");
    // counts: 0 (first), 1 (0→1), 2 (1→2). Three transitions.
    assert_eq!(count_changes, 3, "count effect must fire on each change");
    assert!(entries.iter().any(|e| e.contains("[count] 0")));
    assert!(entries.iter().any(|e| e.contains("[count] 1")));
    assert!(entries.iter().any(|e| e.contains("[count] 2")));
}

// ----------------------------------------------------------------
// examples/v020_named_focus.rs — three named inputs + focus jumps
// ----------------------------------------------------------------

#[test]
fn demo_named_focus_focus_by_name_jumps_directly() {
    use slt::widgets::TextInputState;
    let mut name = TextInputState::default();
    let mut email = TextInputState::default();
    let mut city = TextInputState::default();
    let mut tb = TestBackend::new(80, 12);

    // Frame 0: register three names. Default focus is on the first.
    tb.render(|ui| {
        let _ = ui.col(|ui| {
            let _ = ui.register_focusable_named("name");
            let _ = ui.text_input(&mut name);
            let _ = ui.register_focusable_named("email");
            let _ = ui.text_input(&mut email);
            let _ = ui.register_focusable_named("city");
            let _ = ui.text_input(&mut city);
        });
    });

    // Frame 1: request focus on "city". Resolves immediately because the
    // map from frame 0 already has it.
    let resolved = std::cell::Cell::new(false);
    let r = &resolved;
    tb.render(|ui| {
        r.set(ui.focus_by_name("city"));
        let _ = ui.col(|ui| {
            let _ = ui.register_focusable_named("name");
            let _ = ui.text_input(&mut name);
            let _ = ui.register_focusable_named("email");
            let _ = ui.text_input(&mut email);
            let _ = ui.register_focusable_named("city");
            let _ = ui.text_input(&mut city);
        });
    });
    assert!(resolved.get(), "focus_by_name resolves on the first call");

    // Frame 2: confirm "city" actually got focus.
    let saw_city_focused = std::cell::Cell::new(false);
    let saw_name_focused = std::cell::Cell::new(false);
    let c = &saw_city_focused;
    let n = &saw_name_focused;
    tb.render(|ui| {
        let _ = ui.col(|ui| {
            n.set(ui.register_focusable_named("name"));
            let _ = ui.text_input(&mut name);
            let _ = ui.register_focusable_named("email");
            let _ = ui.text_input(&mut email);
            c.set(ui.register_focusable_named("city"));
            let _ = ui.text_input(&mut city);
        });
    });
    assert!(saw_city_focused.get(), "city should be focused");
    assert!(!saw_name_focused.get(), "name should not be focused");
    // focused_name() reflects the same.
    tb.render(|ui| {
        let _ = ui.col(|ui| {
            let _ = ui.register_focusable_named("name");
            let _ = ui.text_input(&mut name);
            let _ = ui.register_focusable_named("email");
            let _ = ui.text_input(&mut email);
            let _ = ui.register_focusable_named("city");
            let _ = ui.text_input(&mut city);
        });
        assert_eq!(ui.focused_name(), Some("city"));
    });
}

#[test]
fn demo_named_focus_button_renders_with_color() {
    // Smoke test that the color-coding logic from the keyed-demo compiles
    // and renders without panic when stitched into a real frame.
    let mut tb = TestBackend::new(40, 6);
    tb.render(|ui| {
        let counter = ui.use_state_keyed("smoke-0", || 5i32);
        let v = *counter.get(ui);
        let color = if v > 0 { Color::Green } else { Color::Red };
        ui.text(format!("value={v}")).fg(color);
    });
    assert!(tb.to_string_trimmed().contains("value=5"));
}

// ----------------------------------------------------------------
// Cross-cutting: key_presses_when filters correctly under both Ctrl-Q
// and the `register_focusable` path used by the demos.
// ----------------------------------------------------------------

#[test]
fn key_presses_when_in_focused_widget_handles_ctrl_q() {
    use slt::EventBuilder;
    let mut tb = TestBackend::new(40, 4);
    let events = EventBuilder::new()
        .key_with(KeyCode::Char('q'), KeyModifiers::CONTROL)
        .build();
    let saw_q = std::cell::Cell::new(false);
    let q = &saw_q;
    tb.run_with_events(events, |ui| {
        let focused = ui.register_focusable();
        // Iterators hold a shared borrow on `ui.events`, so collect first
        // and consume in a second pass with mutable access.
        let hits: Vec<usize> = ui
            .key_presses_when(focused)
            .filter_map(|(i, key)| {
                if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();
        if !hits.is_empty() {
            q.set(true);
            for i in hits {
                ui.consume_event(i);
            }
        }
    });
    assert!(saw_q.get());
}
