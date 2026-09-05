use slt::{Buffer, Color, Rect, TestBackend, TextInputState, TextareaState};

fn render_input(backend: &mut TestBackend, input: &mut TextInputState) {
    backend.render_with_events(vec![], 0, 1, |ui| {
        let _ = ui.text_input(input);
    });
}

#[test]
fn empty_and_nonempty_masked_inputs_publish_explicit_cursor_privacy() {
    let mut backend = TestBackend::new(40, 8);
    let mut input = TextInputState::new();
    input.masked = true;
    render_input(&mut backend, &mut input);
    assert!(backend.buffer().cursor_position().is_some());
    assert!(backend.buffer().cursor_is_masked());

    input.value = "secret".into();
    input.cursor = 6;
    render_input(&mut backend, &mut input);
    assert!(backend.buffer().cursor_position().is_some());
    assert!(backend.buffer().cursor_is_masked());
    assert!(!backend.to_string_trimmed().contains("secret"));

    input.masked = false;
    render_input(&mut backend, &mut input);
    assert!(backend.buffer().cursor_position().is_some());
    assert!(!backend.buffer().cursor_is_masked());
    backend.assert_contains("secret");
}

#[test]
fn privacy_follows_the_focused_input_not_neighboring_masked_widgets() {
    let mut backend = TestBackend::new(40, 10);
    let mut private = TextInputState::new();
    private.masked = true;
    let mut plain = TextInputState::new();
    for (focus, masked) in [(0, true), (1, false), (0, true)] {
        backend.render_with_events(vec![], focus, 2, |ui| {
            let _ = ui.text_input(&mut private);
            let _ = ui.text_input(&mut plain);
        });
        assert!(backend.buffer().cursor_position().is_some());
        assert_eq!(backend.buffer().cursor_is_masked(), masked);
    }
}

#[test]
fn removed_and_plain_textarea_cursors_do_not_retain_masking() {
    let mut backend = TestBackend::new(40, 8);
    let mut input = TextInputState::new();
    input.masked = true;
    render_input(&mut backend, &mut input);
    assert!(backend.buffer().cursor_is_masked());
    backend.render(|ui| {
        ui.text("No input");
    });
    assert_eq!(backend.buffer().cursor_position(), None);
    assert!(!backend.buffer().cursor_is_masked());

    render_input(&mut backend, &mut input);
    let mut textarea = TextareaState::new();
    backend.render_with_events(vec![], 0, 1, |ui| {
        let _ = ui.textarea(&mut textarea, 3);
    });
    assert!(backend.buffer().cursor_position().is_some());
    assert!(!backend.buffer().cursor_is_masked());
}

#[test]
fn every_buffer_reset_clears_privacy_metadata() {
    struct Target(Buffer);
    impl slt::Backend for Target {
        fn size(&self) -> (u32, u32) {
            (self.0.area.width, self.0.area.height)
        }
        fn buffer_mut(&mut self) -> &mut Buffer {
            &mut self.0
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    for reset in [
        Buffer::reset as fn(&mut Buffer),
        |buffer| buffer.reset_with_bg(Color::Black),
        |buffer| buffer.resize(Rect::new(0, 0, 40, 8)),
        |buffer| buffer.resize(Rect::new(0, 0, 12, 4)),
    ] {
        let mut backend = Target(Buffer::empty(Rect::new(0, 0, 40, 8)));
        let mut input = TextInputState::new();
        input.masked = true;
        slt::frame(
            &mut backend,
            &mut slt::AppState::new(),
            &slt::RunConfig::default(),
            &[],
            &mut |ui| {
                let _ = ui.text_input(&mut input);
            },
        )
        .unwrap();
        assert!(backend.0.cursor_is_masked());
        reset(&mut backend.0);
        assert_eq!(backend.0.cursor_position(), None);
        assert!(!backend.0.cursor_is_masked());
    }
}

#[test]
fn offscreen_masked_caret_has_no_public_cursor_metadata() {
    let mut backend = TestBackend::new(1, 1);
    let mut input = TextInputState::new();
    input.masked = true;
    render_input(&mut backend, &mut input);
    assert_eq!(backend.buffer().cursor_position(), None);
    assert!(!backend.buffer().cursor_is_masked());
}

fn assert_physical_caret_matches_marker(backend: &TestBackend) {
    let buffer = backend.buffer();
    let mut marker = None;
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            if buffer.get(x, y).symbol.as_str() == "\u{258e}" {
                assert!(marker.is_none());
                marker = Some((x, y));
            }
        }
    }
    assert!(marker.is_some());
    assert_eq!(buffer.cursor_position(), marker);
}

#[test]
fn unicode_caret_anchor_uses_graphemes_for_both_input_widgets() {
    for value in [
        "e\u{301}",
        "\u{1f469}\u{200d}\u{1f4bb}",
        "\u{d55c}",
        "e\u{301}\u{1f469}\u{200d}\u{1f4bb}",
    ] {
        let mut backend = TestBackend::new(40, 8);
        let mut input = TextInputState::new();
        input.value = value.into();
        input.cursor = unicode_segmentation::UnicodeSegmentation::graphemes(value, true).count();
        for masked in [false, true] {
            input.masked = masked;
            render_input(&mut backend, &mut input);
            assert_eq!(backend.buffer().cursor_is_masked(), masked);
            assert_physical_caret_matches_marker(&backend);
        }
        let mut textarea = TextareaState::new();
        textarea.set_value(value);
        textarea.cursor_col = input.cursor;
        backend.render_with_events(vec![], 0, 1, |ui| {
            let _ = ui.textarea(&mut textarea, 3);
        });
        assert!(!backend.buffer().cursor_is_masked());
        assert_physical_caret_matches_marker(&backend);
    }
}
