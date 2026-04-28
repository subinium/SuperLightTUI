//! v0.20.0 hook + focus tests covering issues #208, #215, #216, #217, #218.
//!
//! - #208: `Response::right_clicked` / `gained_focus` / `lost_focus`
//! - #215: `use_state_keyed` + `use_state_keyed_default`
//! - #216: `use_effect`
//! - #217: `register_focusable_named` + `focus_by_name` + `focused_name`
//! - #218: `key_presses_when` + `consume_event`

use slt::{
    Event, EventBuilder, KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseKind, TestBackend,
};
use std::cell::Cell;
use std::rc::Rc;

// ----------------------------------------------------------------
// #215 — use_state_keyed
// ----------------------------------------------------------------

#[test]
fn use_state_keyed_persists_across_frames() {
    let mut tb = TestBackend::new(40, 4);

    // Frame 0: initialize three keys.
    tb.render(|ui| {
        for i in 0i32..3 {
            let s = ui.use_state_keyed(format!("item-{i}"), || 0i32);
            *s.get_mut(ui) = i * 10;
        }
    });

    // Frame 1: re-read each — init must NOT re-run, persisted values stick.
    tb.render(|ui| {
        for i in 0i32..3 {
            let s = ui.use_state_keyed(format!("item-{i}"), || 999i32);
            assert_eq!(
                *s.get(ui),
                i * 10,
                "frame 1 lost persisted value for item-{i}"
            );
        }
    });
}

#[test]
fn use_state_keyed_compiles_with_string_literal_and_runtime_string() {
    let mut tb = TestBackend::new(40, 4);
    tb.render(|ui| {
        // String literal — accepts via `impl Into<String>`.
        let a = ui.use_state_keyed("static-key", || 1i32);
        // Runtime String.
        let i = 7usize;
        let b = ui.use_state_keyed(format!("dyn-{i}"), || 2i32);
        assert_eq!(*a.get(ui), 1);
        assert_eq!(*b.get(ui), 2);
    });
}

#[test]
fn use_state_keyed_default_uses_default_impl() {
    let mut tb = TestBackend::new(40, 4);
    tb.render(|ui| {
        let v = ui.use_state_keyed_default::<i32>("default-int");
        let s = ui.use_state_keyed_default::<String>("default-str");
        assert_eq!(*v.get(ui), 0);
        assert!(s.get(ui).is_empty());
    });
}

#[test]
fn use_state_keyed_independent_from_named_namespace() {
    // Named (`&'static str`) and keyed (`String`) are independent maps —
    // collisions in one don't affect the other.
    let mut tb = TestBackend::new(40, 4);
    tb.render(|ui| {
        let n = ui.use_state_named_with::<i32>("collision", || 1);
        let k = ui.use_state_keyed("collision", || 2);
        *n.get_mut(ui) = 100;
        assert_eq!(*k.get(ui), 2, "keyed must not see named writes");
    });
}

#[test]
fn use_state_keyed_safe_inside_conditional() {
    // Mirrors the `use_state_named` conditional safety test.
    let mut tb = TestBackend::new(40, 4);
    tb.render(|ui| {
        let v = ui.use_state_keyed("cond-v".to_string(), || 10i32);
        *v.get_mut(ui) = 42;
    });
    tb.render(|ui| {
        // Branch skipped this frame.
        ui.text("skip");
    });
    tb.render(|ui| {
        let v = ui.use_state_keyed("cond-v".to_string(), || 99i32);
        assert_eq!(*v.get(ui), 42);
    });
}

// ----------------------------------------------------------------
// #216 — use_effect
// ----------------------------------------------------------------

#[test]
fn use_effect_fires_once_with_unit_deps() {
    let counter = Rc::new(Cell::new(0u32));
    let mut tb = TestBackend::new(40, 4);
    for _ in 0..5 {
        let c = counter.clone();
        tb.render(move |ui| {
            ui.use_effect(
                |_| {
                    c.set(c.get() + 1);
                },
                &(),
            );
        });
    }
    assert_eq!(counter.get(), 1, "effect must fire exactly once for `&()`");
}

#[test]
fn use_effect_fires_when_deps_change() {
    let counter = Rc::new(Cell::new(0u32));
    let vals = [0i32, 0, 1, 1, 2];
    let mut tb = TestBackend::new(40, 4);
    for v in &vals {
        let c = counter.clone();
        let v = *v;
        tb.render(move |ui| {
            ui.use_effect(
                move |_| {
                    c.set(c.get() + 1);
                },
                &v,
            );
        });
    }
    // frame 0 (first), frame 2 (0→1), frame 4 (1→2) = 3 fires.
    assert_eq!(counter.get(), 3);
}

#[test]
fn use_effect_does_not_fire_when_deps_unchanged() {
    let counter = Rc::new(Cell::new(0u32));
    let mut tb = TestBackend::new(40, 4);
    for _ in 0..3 {
        let c = counter.clone();
        tb.render(move |ui| {
            ui.use_effect(
                move |_| {
                    c.set(c.get() + 1);
                },
                &42i32,
            );
        });
    }
    assert_eq!(counter.get(), 1);
}

#[test]
#[should_panic(expected = "Hook type mismatch")]
fn use_effect_type_mismatch_panics() {
    let mut tb = TestBackend::new(40, 4);
    tb.render(|ui| {
        ui.use_effect(|_| {}, &1i32);
    });
    // Wrong type at the same hook index in frame 2 — must panic.
    tb.render(|ui| {
        ui.use_effect(|_| {}, &"hello");
    });
}

// ----------------------------------------------------------------
// #217 — register_focusable_named + focus_by_name
// ----------------------------------------------------------------

#[test]
fn focus_by_name_acquires_focus_after_one_frame() {
    let mut tb = TestBackend::new(80, 4);

    // Frame 0: register two names; default focus is index 0 ("a").
    tb.render(|ui| {
        let _ = ui.register_focusable_named("a");
        let _ = ui.register_focusable_named("b");
    });

    // Frame 1: request focus on "b". Resolved against the previous frame's
    // map → focus_index becomes the index that "b" registered under.
    let resolved = std::cell::Cell::new(false);
    let r = &resolved;
    tb.render(move |ui| {
        let did = ui.focus_by_name("b");
        r.set(did);
        let _ = ui.register_focusable_named("a");
        let _ = ui.register_focusable_named("b");
    });
    assert!(
        resolved.get(),
        "focus_by_name should resolve against prev-frame map"
    );

    // Frame 2: confirm "b" actually received focus.
    let saw_b_focused = std::cell::Cell::new(false);
    let s = &saw_b_focused;
    tb.render(move |ui| {
        let _ = ui.register_focusable_named("a");
        let b_focused = ui.register_focusable_named("b");
        s.set(b_focused);
    });
    assert!(saw_b_focused.get(), "named widget 'b' should be focused");
}

#[test]
fn focus_by_name_unknown_name_keeps_pending_request() {
    let mut tb = TestBackend::new(80, 4);

    // Frame 0: only "a" exists; request focus on a name that has never
    // registered. The request must NOT panic; `focus_by_name` returns false
    // because the name didn't resolve against the prev map.
    tb.render(|ui| {
        let _ = ui.register_focusable_named("a");
        let resolved = ui.focus_by_name("ghost");
        assert!(!resolved, "unknown name should not resolve immediately");
    });

    // Frame 1: register "ghost". Pending name now finds an index.
    tb.render(|ui| {
        let _ = ui.register_focusable_named("a");
        let _ = ui.register_focusable_named("ghost");
    });

    // Frame 2: "ghost" should now have focus thanks to the held pending name.
    let saw_ghost_focused = std::cell::Cell::new(false);
    let g = &saw_ghost_focused;
    tb.render(move |ui| {
        let _ = ui.register_focusable_named("a");
        let ghost_focused = ui.register_focusable_named("ghost");
        g.set(ghost_focused);
    });
    assert!(saw_ghost_focused.get());
}

#[test]
fn focused_name_returns_current_focused_widget_name() {
    let mut tb = TestBackend::new(80, 4);
    tb.render(|ui| {
        let _ = ui.register_focusable_named("alpha");
        let _ = ui.register_focusable_named("beta");
        // Default focus_index is 0 → "alpha".
        assert_eq!(ui.focused_name(), Some("alpha"));
    });
    tb.render(|ui| {
        let _ = ui.focus_by_name("beta");
        let _ = ui.register_focusable_named("alpha");
        let _ = ui.register_focusable_named("beta");
    });
    tb.render(|ui| {
        let _ = ui.register_focusable_named("alpha");
        let _ = ui.register_focusable_named("beta");
        assert_eq!(ui.focused_name(), Some("beta"));
    });
}

#[test]
fn duplicate_named_registration_first_wins() {
    let mut tb = TestBackend::new(80, 4);
    tb.render(|ui| {
        let first = ui.register_focusable_named("dup");
        let second = ui.register_focusable_named("dup");
        // Both calls register a focusable; only the first owns the name.
        // The second receives focus only if focus_index lined up with it.
        let _ = (first, second);
        // After two registrations (indices 0 and 1), the map should point at 0.
        assert_eq!(ui.focused_name(), Some("dup"));
    });
}

// ----------------------------------------------------------------
// #218 — key_presses_when + consume_event
// ----------------------------------------------------------------

fn key_event(c: char) -> Event {
    EventBuilder::new()
        .key(c)
        .build()
        .into_iter()
        .next()
        .unwrap()
}

#[test]
fn key_presses_when_inactive_yields_nothing() {
    let mut tb = TestBackend::new(40, 4);
    tb.run_with_events(vec![key_event('a')], |ui| {
        let count = ui.key_presses_when(false).count();
        assert_eq!(count, 0);
    });
}

#[test]
fn key_presses_when_active_yields_events() {
    let mut tb = TestBackend::new(40, 4);
    tb.run_with_events(vec![key_event('a'), key_event('b')], |ui| {
        let codes: Vec<_> = ui
            .key_presses_when(true)
            .map(|(_, k)| k.code.clone())
            .collect();
        assert_eq!(codes.len(), 2);
        assert_eq!(codes[0], KeyCode::Char('a'));
        assert_eq!(codes[1], KeyCode::Char('b'));
    });
}

#[test]
fn consume_event_prevents_subsequent_iterators_from_seeing_event() {
    let mut tb = TestBackend::new(40, 4);
    tb.run_with_events(vec![key_event('x')], |ui| {
        // First consumer (gated active=true) consumes the only event.
        let indices: Vec<usize> = ui.key_presses_when(true).map(|(i, _)| i).collect();
        for i in indices {
            ui.consume_event(i);
        }
        // Second consumer must see nothing.
        assert_eq!(ui.key_presses_when(true).count(), 0);
    });
}

#[test]
fn consume_event_out_of_bounds_silent_noop() {
    let mut tb = TestBackend::new(40, 4);
    tb.render(|ui| {
        // No events queued; out-of-range index must not panic.
        ui.consume_event(usize::MAX);
        ui.consume_event(0);
    });
}

// ----------------------------------------------------------------
// #208 — Response signal expansion (right_clicked / gained_focus / lost_focus)
// ----------------------------------------------------------------

fn right_click_at(x: u32, y: u32) -> Event {
    Event::Mouse(MouseEvent::new(
        MouseKind::Down(MouseButton::Right),
        x,
        y,
        KeyModifiers::NONE,
        None,
        None,
    ))
}

#[test]
fn response_right_clicked_fires_on_right_button_down() {
    let mut tb = TestBackend::new(40, 4);

    // Frame 0: paint a single button to populate the prev_hit_map.
    tb.render(|ui| {
        let _ = ui.button("hit me");
    });

    // Frame 1: send a right-click into the button rect.
    let saw_right_clicked = std::cell::Cell::new(false);
    let saw_left_clicked = std::cell::Cell::new(false);
    let r = &saw_right_clicked;
    let l = &saw_left_clicked;
    tb.run_with_events(vec![right_click_at(0, 0)], move |ui| {
        let resp = ui.button("hit me");
        r.set(resp.right_clicked);
        l.set(resp.clicked);
    });
    assert!(saw_right_clicked.get(), "expected right_clicked=true");
    assert!(
        !saw_left_clicked.get(),
        "left clicked must remain false on right-button-only events"
    );
}

#[test]
fn response_none_has_all_signals_false() {
    let r = slt::Response::none();
    assert!(!r.clicked);
    assert!(!r.right_clicked);
    assert!(!r.hovered);
    assert!(!r.changed);
    assert!(!r.focused);
    assert!(!r.gained_focus);
    assert!(!r.lost_focus);
}

#[test]
fn gained_and_lost_focus_track_focus_transitions() {
    // Buttons go through `begin_widget_interaction`, so the new focus
    // transition signals appear on their `Response`. Custom widgets that
    // bypass `begin_widget_interaction` (e.g. text_input which assembles
    // a Response from the container path) are intentionally out of scope
    // for this test — those widgets will pick up the signals as they're
    // migrated to the unified interaction path in follow-up issues.
    let mut tb = TestBackend::new(40, 4);

    // Frame 0: two buttons. Default `focus_index` is 0, so #1 gains focus.
    let g0 = std::cell::Cell::new(false);
    let l0 = std::cell::Cell::new(false);
    let g0r = &g0;
    let l0r = &l0;
    tb.render(|ui| {
        let r1 = ui.button("first");
        let r2 = ui.button("second");
        g0r.set(r1.gained_focus);
        l0r.set(r2.lost_focus);
    });
    assert!(
        g0.get(),
        "button #1 should report gained_focus on first frame"
    );
    assert!(!l0.get(), "button #2 was never focused, can't lose focus");

    // Frame 1: focus stable on #1 — no transitions either way.
    let g1 = std::cell::Cell::new(true);
    let l1 = std::cell::Cell::new(true);
    let g1r = &g1;
    let l1r = &l1;
    tb.render(|ui| {
        let r1 = ui.button("first");
        let r2 = ui.button("second");
        g1r.set(r1.gained_focus);
        l1r.set(r2.lost_focus);
    });
    assert!(!g1.get(), "no fresh gain on a stable focus");
    assert!(!l1.get(), "no loss on the still-unfocused button");

    // Frame 2: jump focus to #2 — fires lost_focus on #1 and gained_focus on #2.
    let lost1 = std::cell::Cell::new(false);
    let gained2 = std::cell::Cell::new(false);
    let lr = &lost1;
    let gr = &gained2;
    tb.render(|ui| {
        ui.set_focus_index(1);
        let r1 = ui.button("first");
        let r2 = ui.button("second");
        lr.set(r1.lost_focus);
        gr.set(r2.gained_focus);
    });
    assert!(
        lost1.get(),
        "button #1 lost focus when set_focus_index(1) ran"
    );
    assert!(gained2.get(), "button #2 gained focus this frame");
}

#[test]
fn gained_focus_and_lost_focus_are_mutually_exclusive() {
    // For any single Response, gained_focus AND lost_focus cannot both be
    // true on the same frame.
    let mut tb = TestBackend::new(40, 4);
    tb.render(|ui| {
        let r = ui.button("only");
        assert!(
            !(r.gained_focus && r.lost_focus),
            "gained_focus and lost_focus must be mutually exclusive"
        );
    });
    tb.render(|ui| {
        let r = ui.button("only");
        assert!(
            !(r.gained_focus && r.lost_focus),
            "still mutually exclusive on stable frame"
        );
    });
}
