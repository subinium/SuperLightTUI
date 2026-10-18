//! Issue #464: multi-step interaction tests without `RefCell` or focus resets.

use slt::widgets::TextInputState;
use slt::{Color, Context, Event, EventBuilder, KeyCode, TestBackend};

fn two_inputs<'a>(
    a: &'a mut TextInputState,
    b: &'a mut TextInputState,
) -> impl FnMut(&mut Context) + 'a {
    move |ui: &mut Context| {
        ui.text_input(a);
        ui.text_input(b);
    }
}

#[test]
fn tab_then_type_text_through_step_reaches_the_second_input() {
    let mut tb = TestBackend::new(30, 4);
    let (mut a, mut b) = (TextInputState::new(), TextInputState::new());
    let mut app = two_inputs(&mut a, &mut b);
    tb.step(vec![], &mut app)
        .press(KeyCode::Tab, &mut app)
        .type_text("x", &mut app);
    drop(app);
    assert_eq!(a.value, "");
    assert_eq!(b.value, "x");
}

#[test]
fn run_with_events_keeps_focus_between_calls() {
    let mut tb = TestBackend::new(30, 4);
    let (mut a, mut b) = (TextInputState::new(), TextInputState::new());
    tb.render(|ui| {
        ui.text_input(&mut a);
        ui.text_input(&mut b);
    });
    tb.run_with_events(EventBuilder::new().key_code(KeyCode::Tab).build(), |ui| {
        ui.text_input(&mut a);
        ui.text_input(&mut b);
    });
    tb.run_with_events(EventBuilder::new().key('x').build(), |ui| {
        ui.text_input(&mut a);
        ui.text_input(&mut b);
    });
    assert_eq!(a.value, "", "pre-0.26 reset typed into the first input");
    assert_eq!(b.value, "x");
}

#[test]
fn render_with_events_zero_still_forces_slot_zero() {
    let mut tb = TestBackend::new(30, 4);
    let (mut a, mut b) = (TextInputState::new(), TextInputState::new());
    let mut app = two_inputs(&mut a, &mut b);
    tb.step(vec![], &mut app).press(KeyCode::Tab, &mut app);
    tb.render_with_events(EventBuilder::new().key('x').build(), 0, 0, &mut app);
    drop(app);
    assert_eq!(a.value, "x");
    assert_eq!(b.value, "");
}

#[test]
fn hover_drives_group_hover_bg_like_mouse_move() {
    let render = |ui: &mut Context| {
        ui.group("card").col(|ui| {
            ui.text("card body").group_hover_bg(Color::Blue);
        });
    };
    let bg_after = |events: Vec<Event>| {
        let mut tb = TestBackend::new(20, 3);
        tb.render(render);
        tb.run_with_events(events, render);
        tb.buffer().get(0, 0).style.bg
    };
    let via_builder = bg_after(EventBuilder::new().hover(1, 0).build());
    let via_event = bg_after(vec![Event::mouse_move(1, 0)]);
    assert_eq!(via_builder, Some(Color::Blue));
    assert_eq!(via_builder, via_event);
    assert_eq!(
        EventBuilder::new().hover(3, 4).build(),
        EventBuilder::new().mouse_move(3, 4).build()
    );
}
