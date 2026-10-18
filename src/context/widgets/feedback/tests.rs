use crate::widgets::AlertLevel;
use crate::{EventBuilder, TestBackend};

// confirm() hit-test regression tests (issue #175)
// Layout: "ok? [Yes] [No]" at x=0
//   q_width=3, space=1 → yes_start=4, yes_end=9, no_start=10, no_end=14

#[test]
fn confirm_click_yes_registers_yes() {
    let mut backend = TestBackend::new(40, 4);
    let mut answer = false;
    let events = EventBuilder::new().click(4, 0).build(); // yes_start
    let mut clicked = false;
    backend.run_with_events(events, |ui| {
        let r = ui.confirm("ok?", &mut answer);
        clicked = r.clicked;
    });
    assert!(clicked, "click at yes_start should register");
    assert!(answer, "answer should be true");
}

#[test]
fn confirm_click_yes_right_boundary_does_not_hit_no() {
    let mut backend = TestBackend::new(40, 4);
    let mut answer = false;
    // yes_end=9, no_start=10: clicking at col 9 (between Yes and No) should not register
    let events = EventBuilder::new().click(9, 0).build();
    let mut clicked = false;
    backend.run_with_events(events, |ui| {
        let r = ui.confirm("ok?", &mut answer);
        clicked = r.clicked;
    });
    assert!(
        !clicked,
        "click between [Yes] and [No] (col 9) should not register"
    );
}

#[test]
fn confirm_click_no_registers_no() {
    let mut backend = TestBackend::new(40, 4);
    let mut answer = true;
    let events = EventBuilder::new().click(10, 0).build(); // no_start
    let mut clicked = false;
    backend.run_with_events(events, |ui| {
        let r = ui.confirm("ok?", &mut answer);
        clicked = r.clicked;
    });
    assert!(clicked, "click at no_start should register");
    assert!(!answer, "answer should be false");
}

#[test]
fn confirm_click_beyond_no_does_not_register() {
    let mut backend = TestBackend::new(40, 4);
    let mut answer = true;
    let events = EventBuilder::new().click(20, 0).build(); // well past no_end=14
    let mut clicked = false;
    backend.run_with_events(events, |ui| {
        let r = ui.confirm("ok?", &mut answer);
        clicked = r.clicked;
    });
    assert!(!clicked, "click past no_end should not register");
    // answer unchanged
    assert!(answer, "answer should remain unchanged");
}

#[test]
fn alert_message_first_then_level() {
    // Regression guard for the API-review-rules arg-order drift: `(message,
    // level)` is the shipped order. Compiles == doc order matches code.
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        let _ = ui.alert("Disk full", AlertLevel::Error);
    });
    tb.assert_contains("Disk full");
}
