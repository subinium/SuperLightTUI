mod support;

use proptest::prelude::*;
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
    let squared = *ui.use_memo(&current, |value| value * value);
    ui.text(format!("count={current} squared={squared}"));
}

fn action_strategy() -> impl Strategy<Value = Vec<Option<char>>> {
    prop::collection::vec(
        prop_oneof![
            Just(None),
            Just(Some('+')),
            Just(Some('-')),
            Just(Some('x')),
        ],
        0..40,
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn frame_backend_matches_test_backend_for_event_sequences(actions in action_strategy()) {
        let mut headless = TestBackend::new(32, 4);
        let mut backend = RecordingBackend::new(32, 4);
        let mut app_state = AppState::new();

        for action in actions {
            let events = match action {
                Some(ch) => EventBuilder::new().key(ch).build(),
                None => Vec::new(),
            };

            headless.run_with_events(events.clone(), render_counter);
            let mut render = render_counter;
            let backend_output = render_with_frame_backend(
                &mut backend,
                &mut app_state,
                &events,
                &mut render,
            ).expect("custom backend frame should render");

            prop_assert_eq!(headless.to_string_trimmed(), backend_output);
        }
    }
}
