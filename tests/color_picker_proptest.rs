//! Property tests for the color picker widget.
//!
//! Invariant: for any palette of random RGB swatches and any sequence of
//! arrow-key navigation events, the selected index stays in bounds and the
//! widget never panics.

use proptest::prelude::*;
use slt::widgets::ColorPickerState;
use slt::{Color, EventBuilder, KeyCode, TestBackend};

fn swatch_strategy() -> impl Strategy<Value = Vec<Color>> {
    prop::collection::vec(
        (any::<u8>(), any::<u8>(), any::<u8>()).prop_map(|(r, g, b)| Color::Rgb(r, g, b)),
        1..40usize,
    )
}

fn key_strategy() -> impl Strategy<Value = Vec<KeyCode>> {
    prop::collection::vec(
        prop_oneof![
            Just(KeyCode::Left),
            Just(KeyCode::Right),
            Just(KeyCode::Up),
            Just(KeyCode::Down),
            Just(KeyCode::Char('h')),
            Just(KeyCode::Char('j')),
            Just(KeyCode::Char('k')),
            Just(KeyCode::Char('l')),
            Just(KeyCode::Enter),
        ],
        0..60,
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn color_picker_navigation_stays_in_bounds(
        colors in swatch_strategy(),
        keys in key_strategy(),
        columns in 1usize..12,
    ) {
        let len = colors.len();
        let mut state = ColorPickerState::new(colors).columns(columns);
        let mut tb = TestBackend::new(48, 16);

        for code in keys {
            let events = EventBuilder::new().key_code(code).build();
            tb.render_with_events(events, 0, 1, |ui| {
                let _ = ui.color_picker(&mut state);
            });
            // Selection never leaves the palette bounds.
            prop_assert!(state.selected < len);
        }
    }
}
