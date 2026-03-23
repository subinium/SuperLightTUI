mod support;

use slt::widgets::TabsState;
use slt::{AppState, Context, EventBuilder, TestBackend};

use support::{render_with_frame_backend, RecordingBackend};

fn render_counter(ui: &mut Context) {
    let count = ui.use_state(|| 0i32);
    if ui.key('+') {
        *count.get_mut(ui) += 1;
    }
    if ui.key('-') {
        *count.get_mut(ui) -= 1;
    }
    let current = *count.get(ui);
    let doubled = *ui.use_memo(&current, |value| value * 2);
    ui.text(format!("count={current} doubled={doubled}"));
}

fn render_tabs(ui: &mut Context, state: &mut TabsState) {
    let _ = ui.tabs(state);
    if let Some(label) = state.selected_label() {
        ui.text(format!("active={label}"));
    }
}

#[test]
fn frame_backend_matches_test_backend_for_stateful_hook_frames() {
    let mut headless = TestBackend::new(32, 4);
    let mut backend = RecordingBackend::new(32, 4);
    let mut app_state = AppState::new();

    let frames = [
        Vec::new(),
        EventBuilder::new().key('+').build(),
        EventBuilder::new().key('+').build(),
        EventBuilder::new().key('-').build(),
    ];

    for events in frames {
        headless.run_with_events(events.clone(), render_counter);
        let mut render = render_counter;
        let backend_output =
            render_with_frame_backend(&mut backend, &mut app_state, &events, &mut render)
                .expect("custom backend frame should render");
        assert_eq!(headless.to_string_trimmed(), backend_output);
    }
}

#[test]
fn frame_backend_matches_test_backend_for_prev_frame_hit_testing() {
    let mut headless = TestBackend::new(40, 4);
    let mut backend = RecordingBackend::new(40, 4);
    let mut app_state = AppState::new();
    let mut headless_tabs = TabsState::new(vec!["One", "Two", "Three"]);
    let mut backend_tabs = TabsState::new(vec!["One", "Two", "Three"]);

    headless.render(|ui| render_tabs(ui, &mut headless_tabs));
    let mut first_render = |ui: &mut Context| render_tabs(ui, &mut backend_tabs);
    let backend_output =
        render_with_frame_backend(&mut backend, &mut app_state, &[], &mut first_render)
            .expect("initial backend frame should render");
    assert_eq!(headless.to_string_trimmed(), backend_output);
    assert_eq!(headless_tabs.selected, backend_tabs.selected);

    let click_second_tab = EventBuilder::new().click(10, 0).build();
    headless.render_with_events(click_second_tab.clone(), 0, 1, |ui| {
        render_tabs(ui, &mut headless_tabs)
    });
    let mut second_render = |ui: &mut Context| render_tabs(ui, &mut backend_tabs);
    let backend_output = render_with_frame_backend(
        &mut backend,
        &mut app_state,
        &click_second_tab,
        &mut second_render,
    )
    .expect("click backend frame should render");

    assert_eq!(headless.to_string_trimmed(), backend_output);
    assert_eq!(headless_tabs.selected, 1);
    assert_eq!(headless_tabs.selected, backend_tabs.selected);
}
