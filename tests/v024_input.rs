#![allow(unused_must_use)]
#![allow(clippy::unwrap_used)]

use slt::widgets::{ColorPickerState, PickerMode, TextInputState, TextareaState};
use slt::{Color, Event, EventBuilder, KeyCode, KeyModifiers, Response, TestBackend};
use unicode_segmentation::UnicodeSegmentation;

fn input(state: &mut TextInputState, events: Vec<Event>) -> Response {
    let mut response = Response::default();
    TestBackend::new(40, 12).render_with_events(events, 0, 1, |ui| {
        response = ui.text_input(state);
    });
    assert!(state.cursor <= state.value.graphemes(true).count());
    response
}

fn area(state: &mut TextareaState, events: Vec<Event>) -> (Response, TestBackend) {
    area_rows(state, events, 8)
}

fn area_rows(
    state: &mut TextareaState,
    events: Vec<Event>,
    visible_rows: u32,
) -> (Response, TestBackend) {
    let mut backend = TestBackend::new(40, 12);
    let mut response = Response::default();
    backend.render_with_events(events, 0, 1, |ui| {
        response = ui.textarea(state, visible_rows);
    });
    assert!(state.cursor_row < state.lines.len());
    assert!(state.cursor_col <= state.lines[state.cursor_row].graphemes(true).count());
    (response, backend)
}

fn key(code: KeyCode) -> Vec<Event> {
    EventBuilder::new().key_code(code).build()
}
fn undo() -> Vec<Event> {
    EventBuilder::new()
        .key_with(KeyCode::Char('z'), KeyModifiers::CONTROL)
        .build()
}
fn redo() -> Vec<Event> {
    EventBuilder::new()
        .key_with(KeyCode::Char('y'), KeyModifiers::CONTROL)
        .build()
}

fn multiline_events(codes: &[u8]) -> Vec<Event> {
    codes
        .iter()
        .flat_map(|code| match code {
            0 => EventBuilder::new().paste("a\nb").build(),
            1 => key(KeyCode::Enter),
            2 => undo(),
            3 => redo(),
            4 => key(KeyCode::Backspace),
            5 => key(KeyCode::Delete),
            6 => key(KeyCode::Left),
            7 => key(KeyCode::Right),
            8 => key(KeyCode::Up),
            9 => key(KeyCode::Down),
            _ => unreachable!(),
        })
        .collect()
}

fn assert_caret_visible(backend: &TestBackend, expected_row: usize, expected_col: usize) {
    assert!(expected_row < 8);
    assert!(backend.line(expected_row as u32).contains('\u{258e}'));
    assert_eq!(backend.to_string_trimmed().matches('\u{258e}').count(), 1);
    assert_eq!(
        backend.buffer().cursor_position(),
        Some((expected_col as u32, expected_row as u32))
    );
}

fn assert_ci_viewport_case(codes: &[u8], expected_value: &str, expected_col: usize) {
    let mut batch = TextareaState::new()
        .word_wrap(3)
        .max_length(20)
        .history_max(3);
    let mut split = batch.clone();
    let events = multiline_events(codes);
    let (_, batch_backend) = area(&mut batch, events.clone());
    let mut offsets = Vec::new();
    for event in events {
        area(&mut split, vec![event]);
        offsets.push(split.scroll_offset);
    }
    let (_, split_backend) = area(&mut split, vec![]);
    assert_eq!(batch.value(), expected_value);
    assert_eq!(split.value(), expected_value);
    assert_eq!((batch.cursor_row, batch.cursor_col), (7, expected_col));
    assert_eq!((split.cursor_row, split.cursor_col), (7, expected_col));
    assert_eq!(offsets, [0, 0, 0, 0, 0, 0, 0, 1, 1]);
    assert_eq!((batch.scroll_offset, split.scroll_offset), (0, 1));
    assert_caret_visible(&batch_backend, 7, expected_col);
    assert_caret_visible(&split_backend, 6, expected_col);
    assert_ne!(
        batch_backend.to_string_trimmed(),
        split_backend.to_string_trimmed()
    );
    check_canonical_view(&batch, &split, 8).unwrap();
    assert_eq!((batch.scroll_offset, split.scroll_offset), (0, 1));
}

#[test]
fn multiline_linux_ci_minimal_retains_render_driven_scroll() {
    assert_ci_viewport_case(
        &[0, 0, 0, 0, 0, 0, 1, 1, 2],
        "a\nba\nba\nba\nba\nba\nb\n",
        0,
    );
}

#[test]
fn multiline_macos_ci_minimal_retains_render_driven_scroll() {
    assert_ci_viewport_case(
        &[0, 0, 0, 0, 0, 1, 0, 1, 2],
        "a\nba\nba\nba\nba\nb\na\nb",
        1,
    );
}

#[test]
fn viewport_history_depends_on_intermediate_renders_even_without_undo() {
    let mut batch = TextareaState::new().word_wrap(3);
    batch.set_value("a\na\na\na\na\na\na\na\na\na");
    let mut split = batch.clone();
    let events = multiline_events(&[9, 9, 9, 9, 9, 9, 9, 9, 8]);
    let (_, batch_backend) = area(&mut batch, events.clone());
    for event in events {
        area(&mut split, vec![event]);
    }
    let (_, split_backend) = area(&mut split, vec![]);
    assert_eq!(batch.lines, split.lines);
    assert_eq!((batch.cursor_row, batch.cursor_col), (7, 0));
    assert_eq!((split.cursor_row, split.cursor_col), (7, 0));
    assert_eq!((batch.scroll_offset, split.scroll_offset), (0, 1));
    assert_eq!((batch.history_len(), split.history_len()), (0, 0));
    assert_caret_visible(&batch_backend, 7, 0);
    assert_caret_visible(&split_backend, 6, 0);
    check_canonical_view(&batch, &split, 8).unwrap();
}

#[test]
fn undo_redo_preserves_current_viewport_not_a_snapshot_viewport() {
    let mut state = TextareaState::new();
    state.set_value(["a"; 12].join("\n"));
    state.cursor_row = 7;
    state.scroll_offset = 2;
    area(&mut state, key(KeyCode::Char('X')));
    state.scroll_offset = 3;
    let (_, undone) = area(&mut state, undo());
    assert_eq!(
        (state.cursor_row, state.cursor_col, state.scroll_offset),
        (7, 0, 3)
    );
    assert_eq!(state.lines[7], "a");
    assert_caret_visible(&undone, 4, 0);
    let (_, redone) = area(&mut state, redo());
    assert_eq!(
        (state.cursor_row, state.cursor_col, state.scroll_offset),
        (7, 1, 3)
    );
    assert_eq!(state.lines[7], "Xa");
    assert_caret_visible(&redone, 4, 1);
}

fn check_visible_view(
    backend: &TestBackend,
    visible_rows: u32,
) -> proptest::test_runner::TestCaseResult {
    proptest::prop_assert_eq!(backend.to_string_trimmed().matches('\u{258e}').count(), 1);
    let cursor = backend.buffer().cursor_position();
    proptest::prop_assert!(cursor.is_some());
    proptest::prop_assert!(cursor.is_some_and(|(x, y)| x < 40 && y < visible_rows));
    Ok(())
}

fn check_canonical_view(
    batch: &TextareaState,
    split: &TextareaState,
    visible_rows: u32,
) -> proptest::test_runner::TestCaseResult {
    // Minimal auto-scroll runs once per render, not once per event. Intermediate
    // renders may retain a different valid viewport, including after undo.
    // Compare final clones from a common viewport without resetting their
    // cursor affinity/history or changing either live execution's scroll state.
    let mut batch_view = batch.clone();
    let mut split_view = split.clone();
    batch_view.scroll_offset = 0;
    split_view.scroll_offset = 0;
    let (_, batch_backend) = area_rows(&mut batch_view, vec![], visible_rows);
    let (_, split_backend) = area_rows(&mut split_view, vec![], visible_rows);
    check_visible_view(&batch_backend, visible_rows)?;
    check_visible_view(&split_backend, visible_rows)?;
    proptest::prop_assert_eq!(batch_view.scroll_offset, split_view.scroll_offset);
    proptest::prop_assert_eq!(
        batch_backend.buffer().cursor_position(),
        split_backend.buffer().cursor_position()
    );
    proptest::prop_assert_eq!(
        batch_backend.to_string_trimmed(),
        split_backend.to_string_trimmed()
    );
    Ok(())
}

fn check_multiline_partition(
    mut batch: TextareaState,
    events: Vec<Event>,
    chunk_size: usize,
    visible_rows: u32,
) -> proptest::test_runner::TestCaseResult {
    let mut split = batch.clone();
    let (_, batch_backend) = area_rows(&mut batch, events.clone(), visible_rows);
    check_visible_view(&batch_backend, visible_rows)?;
    for chunk in events.chunks(chunk_size) {
        let (_, backend) = area_rows(&mut split, chunk.to_vec(), visible_rows);
        check_visible_view(&backend, visible_rows)?;
    }
    check_visible_view(&area_rows(&mut split, vec![], visible_rows).1, visible_rows)?;
    proptest::prop_assert_eq!(
        (&batch.lines, batch.cursor_row, batch.cursor_col),
        (&split.lines, split.cursor_row, split.cursor_col)
    );
    proptest::prop_assert_eq!(batch.history_len(), split.history_len());
    check_canonical_view(&batch, &split, visible_rows)
}

fn check_multiline_batch(codes: &[u8]) -> proptest::test_runner::TestCaseResult {
    check_multiline_partition(
        TextareaState::new()
            .word_wrap(3)
            .max_length(20)
            .history_max(3),
        multiline_events(codes),
        1,
        8,
    )
}

fn replay_multiline_ci_seed(seed: &str) {
    use proptest::test_runner::{Config, MapFailurePersistence, TestRunner};
    let mut persistence = MapFailurePersistence::default();
    persistence
        .map
        .entry(file!())
        .or_default()
        .insert(seed.parse().unwrap());
    let mut runner = TestRunner::new(Config {
        cases: 0,
        max_shrink_iters: 4096,
        source_file: Some(file!()),
        failure_persistence: Some(Box::new(persistence)),
        ..Config::default()
    });
    runner
        .run(&proptest::collection::vec(0u8..10, 0..30), |codes| {
            check_multiline_batch(&codes)
        })
        .unwrap();
}

#[test]
fn multiline_linux_ci_seed() {
    replay_multiline_ci_seed("cc 80f0be257286114ea64bf6ca11b2ae11a71a50af69e0ed91ff1c706169b26fe2");
}

#[test]
fn multiline_macos_ci_seed() {
    replay_multiline_ci_seed("cc 3b1b04d2aaa3270b9b2d925f8c81cf68bacd5f35097bcb2cd37378be7f55a430");
}

#[test]
fn unicode_hex_is_rejected_without_changing_ascii_contracts() {
    for text in [
        "#\u{d55c}\u{ae00}",
        "#\u{ac00}",
        "#a\u{e9}",
        "#\u{1f469}ab",
        "#12\u{ac00}a",
    ] {
        assert_eq!(Color::from_hex(text), None);
        assert!(text.parse::<Color>().is_err());
        let mut picker = ColorPickerState::new(vec![Color::Red]);
        picker.mode = PickerMode::Hex;
        picker.hex_input.value = text.to_owned();
        assert_eq!(picker.selected(), Color::Red);
        TestBackend::new(40, 12).render(|ui| {
            let _ = ui.color_picker(&mut picker);
        });
    }
    assert_eq!(Color::from_hex("#aBc"), Some(Color::Rgb(170, 187, 204)));
    assert_eq!(Color::from_hex("#Aa00fF"), Some(Color::Rgb(170, 0, 255)));
    assert_eq!(Color::from_hex("abc"), None);
    assert_eq!(Color::from_hex(" #abc "), None);
    assert_eq!(" abc ".parse::<Color>(), Ok(Color::Rgb(170, 187, 204)));
    assert_eq!("RED".parse::<Color>(), Ok(Color::Red));
    let mut picker = ColorPickerState::new(vec![]);
    picker.mode = PickerMode::Hex;
    picker.hex_input.value = " #abc ".into();
    assert_eq!(picker.selected(), Color::Rgb(170, 187, 204));
}

#[cfg(feature = "serde")]
#[test]
fn unicode_theme_color_is_a_recoverable_parse_error() {
    let valid = slt::ThemeFile::from_toml_str("[theme]\nprimary = '#abcdef'\n").unwrap();
    for bad in ["#\u{d55c}\u{ae00}", "#\u{ac00}"] {
        assert!(slt::ThemeFile::from_toml_str(&format!("[theme]\nprimary = '{bad}'\n")).is_err());
    }
    assert_eq!(valid.theme.primary, Color::Rgb(171, 205, 239));
}

#[cfg(feature = "theme-watch")]
#[test]
fn invalid_unicode_hot_reload_retains_last_good_theme_and_recovers() {
    use std::time::{Duration, Instant};
    struct Directory(std::path::PathBuf);
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let directory = Directory(std::env::temp_dir().join(format!(
            "slt-v024-theme-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
    std::fs::create_dir(&directory.0).unwrap();
    let path = directory.0.join("theme.toml");
    std::fs::write(&path, "[theme]\nprimary = '#abcdef'\n").unwrap();
    let mut watcher = slt::ThemeWatcher::new(&path).unwrap();
    std::fs::write(&path, "[theme]\nprimary = '#\u{d55c}\u{ae00}'\n").unwrap();
    let deadline = Instant::now() + Duration::from_millis(500);
    while Instant::now() < deadline {
        assert!(watcher.poll().is_none());
        assert_eq!(watcher.current().theme.primary, Color::Rgb(171, 205, 239));
        std::thread::sleep(Duration::from_millis(10));
    }
    std::fs::write(&path, "[theme]\nprimary = '#123456'\n").unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(theme) = watcher.poll() {
            assert_eq!(theme.theme.primary, Color::Rgb(18, 52, 86));
            break;
        }
        assert!(
            Instant::now() < deadline,
            "valid theme did not reload after rejected Unicode"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn narrow_inputs_keep_complete_graphemes_and_visible_caret() {
    for width in [1, 2, 5, 6, 8, 20] {
        for value in [
            "\u{d55c}\u{d55c}",
            "\u{1f469}\u{200d}\u{1f4bb}ab",
            "e\u{301}\u{d55c}x",
            "abcdef",
        ] {
            for masked in [false, true] {
                let mut state = TextInputState::new();
                state.value = value.into();
                state.masked = masked;
                for cursor in 0..=value.graphemes(true).count() {
                    state.cursor = cursor;
                    let mut backend = TestBackend::new(width, 4);
                    backend.render_with_events(vec![], 0, 1, |ui| {
                        let _ = ui.text_input(&mut state);
                    });
                    if width >= 5 {
                        backend.assert_contains("\u{258e}");
                    }
                    assert_eq!(state.cursor, cursor);
                }
            }
        }
    }
    let mut state = TextInputState::new();
    state.value = "\u{d55c}\u{d55c}".into();
    state.cursor = 2;
    let mut backend = TestBackend::new(8, 4);
    backend.render_with_events(vec![], 0, 1, |ui| {
        let _ = ui.text_input(&mut state);
    });
    backend.assert_line_contains(1, " \u{d55c}\u{258e}");
}

#[test]
fn paste_and_keys_follow_stream_order_in_both_inputs() {
    for events in [
        EventBuilder::new()
            .paste("ab")
            .key_code(KeyCode::Left)
            .key('X')
            .build(),
        EventBuilder::new()
            .key('a')
            .paste("b")
            .key_code(KeyCode::Left)
            .paste("X")
            .build(),
        EventBuilder::new()
            .paste("a")
            .paste("b")
            .key_code(KeyCode::Left)
            .key('X')
            .build(),
    ] {
        let mut text = TextInputState::new();
        let mut textarea = TextareaState::new();
        input(&mut text, events.clone());
        area(&mut textarea, events);
        assert_eq!(text.value, "aXb");
        assert_eq!(textarea.value(), "aXb");
    }
    let mut state = TextareaState::new();
    area(
        &mut state,
        EventBuilder::new()
            .paste("a\r\nb\rc")
            .key_code(KeyCode::Left)
            .key('X')
            .build(),
    );
    assert_eq!(state.value(), "a\nb\nXc");
}

#[test]
fn ordered_inputs_respect_consumption_focus_and_shortcuts() {
    let mut first = TextInputState::new();
    let mut second = TextareaState::new();
    let events = EventBuilder::new()
        .paste("ab")
        .key_code(KeyCode::Left)
        .key('X')
        .key_with(KeyCode::Char('q'), KeyModifiers::CONTROL)
        .key_release('Y')
        .build();
    TestBackend::new(40, 12).render_with_events(events, 0, 2, |ui| {
        let _ = ui.text_input(&mut first);
        let _ = ui.textarea(&mut second, 3);
        assert!(ui.key_mod('q', KeyModifiers::CONTROL));
        assert!(!ui.key('X'));
        assert!(ui.key_release('Y'));
    });
    assert_eq!(first.value, "aXb");
    assert_eq!(second.value(), "");
    TestBackend::new(40, 12).render_with_events(key(KeyCode::Char('x')), 0, 1, |ui| {
        assert!(ui.consume_key('x'));
        let _ = ui.text_input(&mut first);
    });
    assert_eq!(first.value, "aXb");
}

#[test]
fn wrap_map_is_current_after_join_split_paste_and_undo() {
    let mut state = TextareaState::new().word_wrap(10);
    state.set_value("a\nb");
    state.cursor_col = 1;
    area(
        &mut state,
        EventBuilder::new()
            .key_code(KeyCode::Delete)
            .key_code(KeyCode::Down)
            .key('x')
            .build(),
    );
    assert_eq!(state.value(), "axb");
    let mut events = undo();
    events.extend(undo());
    events.extend(EventBuilder::new().key_code(KeyCode::Down).key('x').build());
    area(&mut state, events);
    assert_eq!(state.value(), "a\nbx");

    state.set_value("abcd");
    state.cursor_col = 2;
    area(
        &mut state,
        EventBuilder::new()
            .key_code(KeyCode::Enter)
            .key_code(KeyCode::Up)
            .key('X')
            .build(),
    );
    assert_eq!(state.value(), "Xab\ncd");
    state.set_value("a");
    area(
        &mut state,
        EventBuilder::new()
            .paste("b\nc")
            .key_code(KeyCode::Up)
            .key('X')
            .build(),
    );
    assert_eq!(state.value(), "bX\nca");
}

#[test]
fn rejected_paste_preserves_history_redo_and_typing_group() {
    for cap in [0, 1, 100] {
        let mut state = TextareaState::new().max_length(1).history_max(cap);
        area(&mut state, key(KeyCode::Char('a')));
        let len = state.history_len();
        for rejected in ["", "b", "\r\n"] {
            assert!(
                !area(&mut state, EventBuilder::new().paste(rejected).build())
                    .0
                    .changed
            );
            assert_eq!(state.history_len(), len);
        }
        area(&mut state, undo());
        assert_eq!(state.value(), if cap == 0 { "a" } else { "" });
        if cap > 0 {
            state.max_length = Some(0);
            area(&mut state, EventBuilder::new().paste("b").build());
            area(&mut state, redo());
            assert_eq!(state.value(), "a");
        }
    }
    let mut state = TextareaState::new().max_length(3);
    area(&mut state, key(KeyCode::Char('a')));
    area(&mut state, EventBuilder::new().paste("bcdef").build());
    assert_eq!(state.value(), "abc");
    area(&mut state, undo());
    assert_eq!(state.value(), "a");
}

#[test]
fn bulk_multiline_paste_preserves_suffix_caret_and_single_undo_unit() {
    let document = "old\n".repeat(10_000);
    let pasted = "e\u{301}\n\u{1f469}\u{200d}\u{1f4bb}".repeat(1_000);
    let mut state = TextareaState::new();
    state.set_value(&document);
    state.cursor_row = 5_000;
    state.cursor_col = 1;
    let edge = 5_000 * 4 + 1;
    let expected = format!("{}{}{}", &document[..edge], pasted, &document[edge..]);
    assert!(
        area(&mut state, EventBuilder::new().paste(&pasted).build())
            .0
            .changed
    );
    assert_eq!(state.value(), expected);
    assert_eq!((state.cursor_row, state.cursor_col), (6_000, 1));
    assert_eq!(state.history_len(), 1);
    area(&mut state, undo());
    assert_eq!(state.value(), document);
    area(&mut state, redo());
    assert_eq!(state.value(), expected);
}

#[test]
fn textarea_limit_counts_a_logical_newline_after_existing_carriage_return() {
    for paste in [false, true] {
        let mut state = TextareaState::new().max_length(1);
        state.set_value("\r");
        state.cursor_col = 1;
        let events = if paste {
            EventBuilder::new().paste("\n").build()
        } else {
            key(KeyCode::Enter)
        };
        assert!(!area(&mut state, events).0.changed);
        assert_eq!(state.value(), "\r");
        assert_eq!(state.grapheme_len(), 1);
        assert_eq!(state.history_len(), 0);
        state.max_length = Some(2);
        assert!(area(&mut state, key(KeyCode::Enter)).0.changed);
        assert_eq!(state.grapheme_len(), 2);
        assert_eq!(state.value(), "\r\n");
    }
}

#[test]
fn resulting_graphemes_define_caret_and_length_for_keys_and_paste() {
    for paste in [false, true] {
        for (before, col, inserted, expected, after_col) in [
            ("ab", 1, "\u{301}", "a\u{301}b", 1),
            (
                "\u{1f469}\u{1f4bb}x",
                1,
                "\u{200d}",
                "\u{1f469}\u{200d}\u{1f4bb}x",
                1,
            ),
            ("\u{1f1f0}x", 1, "\u{1f1f7}", "\u{1f1f0}\u{1f1f7}x", 1),
        ] {
            let mut text = TextInputState::new();
            text.value = before.into();
            text.cursor = col;
            let mut textarea = TextareaState::new();
            textarea.set_value(before);
            textarea.cursor_col = col;
            let events = if paste {
                EventBuilder::new().paste(inserted).build()
            } else {
                key(KeyCode::Char(inserted.chars().next().unwrap()))
            };
            input(&mut text, events.clone());
            area(&mut textarea, events);
            assert_eq!(text.value, expected);
            assert_eq!(textarea.value(), expected);
            assert_eq!(text.cursor, after_col);
            assert_eq!(textarea.cursor_col, after_col);
            input(&mut text, key(KeyCode::Backspace));
            area(&mut textarea, key(KeyCode::Backspace));
            assert_eq!(text.value, if before == "ab" { "b" } else { "x" });
            assert_eq!(text.value, textarea.value());
        }
        let mut text = TextInputState::new().max_length(1);
        let mut textarea = TextareaState::new().max_length(1);
        for c in ['e', '\u{301}'] {
            let events = if paste {
                EventBuilder::new().paste(&c.to_string()).build()
            } else {
                key(KeyCode::Char(c))
            };
            input(&mut text, events.clone());
            area(&mut textarea, events);
        }
        assert_eq!(text.value, "e\u{301}");
        assert_eq!(textarea.value(), "e\u{301}");
        assert_eq!(text.cursor, 1);
        assert_eq!(textarea.cursor_col, 1);
    }
}

#[test]
fn autocomplete_accept_dismiss_and_submit_are_batch_independent() {
    for first in [KeyCode::Esc, KeyCode::Enter, KeyCode::Tab] {
        for batched in [false, true] {
            let mut state = TextInputState::new();
            state.value = "a".into();
            state.cursor = 1;
            state.set_suggestions(vec!["apple".into(), "apricot".into()]);
            state.show_suggestions = true;
            let events = EventBuilder::new()
                .key_code(first.clone())
                .key_code(KeyCode::Enter)
                .build();
            let submitted = if batched {
                input(&mut state, events).submitted
            } else {
                let mut submitted = false;
                for event in events {
                    submitted |= input(&mut state, vec![event]).submitted;
                }
                submitted
            };
            assert!(submitted);
            assert_eq!(
                state.value,
                if first == KeyCode::Esc { "a" } else { "apple" }
            );
            assert!(!state.show_suggestions);
        }
    }
}

#[test]
fn wrapped_caret_preserves_affinity_and_desired_display_column() {
    let mut state = TextareaState::new().word_wrap(5);
    state.set_value("abcdefghij");
    state.cursor_col = 10;
    let (_, backend) = area(&mut state, key(KeyCode::Up));
    assert_eq!(state.cursor_col, 5);
    backend.assert_line_contains(0, "abcde\u{258e}");
    assert!(!backend.line(1).contains('\u{258e}'));
    area(&mut state, vec![])
        .1
        .assert_line_contains(0, "abcde\u{258e}");
    area(&mut state, key(KeyCode::Down))
        .1
        .assert_line_contains(1, "fghij\u{258e}");
    assert_eq!(state.cursor_col, 10);
    state.set_value("abcd\n\u{d55c}\u{d55c}\na");
    state.cursor_col = 4;
    area(&mut state, key(KeyCode::Down));
    assert_eq!((state.cursor_row, state.cursor_col), (1, 2));
    area(&mut state, key(KeyCode::Down));
    assert_eq!((state.cursor_row, state.cursor_col), (2, 1));
    area(&mut state, key(KeyCode::Up));
    assert_eq!((state.cursor_row, state.cursor_col), (1, 2));
    area(&mut state, key(KeyCode::Up));
    assert_eq!((state.cursor_row, state.cursor_col), (0, 4));
}

#[test]
fn wrapped_affinity_survives_undo_and_start_boundary_moves_up() {
    let mut state = TextareaState::new().word_wrap(5);
    state.set_value("abcdefghij");
    state.cursor_col = 10;
    area(&mut state, key(KeyCode::Up));
    area(&mut state, key(KeyCode::Char('X')));
    area(&mut state, undo())
        .1
        .assert_line_contains(0, "abcde\u{258e}");
    area(&mut state, redo())
        .1
        .assert_line_contains(1, "X\u{258e}fghi");
    state.set_value("abcdefghij");
    state.cursor_col = 5;
    area(&mut state, key(KeyCode::Up))
        .1
        .assert_line_contains(0, "\u{258e}abcde");
    assert_eq!(state.cursor_col, 0);
}

#[test]
fn public_text_and_wrap_width_changes_do_not_reuse_stale_maps() {
    let mut state = TextareaState::new().word_wrap(5);
    state.set_value("abcdefghij");
    state.cursor_col = 10;
    area(&mut state, key(KeyCode::Up));
    state.wrap_width = Some(3);
    let (_, backend) = area(&mut state, vec![]);
    backend.assert_line_contains(1, "de\u{258e}f");
    state.lines = vec!["\u{d55c}x".into()];
    state.cursor_row = usize::MAX;
    state.cursor_col = usize::MAX;
    area(&mut state, key(KeyCode::Up));
    assert_eq!(state.cursor_row, 0);
    state.lines.clear();
    area(&mut state, key(KeyCode::Char('x')));
    assert_eq!(state.value(), "x");
}

#[test]
fn net_zero_batches_do_not_report_changed() {
    let mut state = TextareaState::new();
    assert!(
        !area(
            &mut state,
            EventBuilder::new()
                .key('x')
                .key_code(KeyCode::Backspace)
                .build()
        )
        .0
        .changed
    );
    assert!(!area(&mut state, vec![]).0.changed);
    state.lines = vec!["external".into()];
    assert!(!area(&mut state, vec![]).0.changed);
    assert_eq!(state.value(), "external");
}

#[test]
fn state_and_memo_handle_traits_do_not_require_value_traits() {
    struct Value(std::cell::Cell<u32>);
    fn handle_traits<T: Clone + std::fmt::Debug + Eq>(_: &T) {}
    TestBackend::new(20, 4).render(|ui| {
        let state = ui.use_state(|| Value(std::cell::Cell::new(1)));
        handle_traits(&state);
        let clone = state.clone();
        clone.get_mut(ui).0.set(2);
        assert_eq!(state.get(ui).0.get(), 2);
        assert_eq!(state, clone);
        let named = ui.use_state_named("non-clone", || Value(std::cell::Cell::new(8)));
        let keyed = ui.use_state_keyed("dynamic", || Value(std::cell::Cell::new(9)));
        assert_eq!(named, named.clone());
        assert_eq!(keyed, keyed.clone());
        let memo = ui.use_memo(&(), |_| Value(std::cell::Cell::new(3)));
        handle_traits(&memo);
        let clone = memo.clone();
        clone.get(ui).0.set(4);
        assert_eq!(memo.get(ui).0.get(), 4);
        assert_eq!(memo, clone);
    });
}

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(96))]
    #[test]
    fn arbitrary_unicode_color_input_never_panics(text in ".*") {
        let _ = Color::from_hex(&text);
        let _ = text.parse::<Color>();
        let mut picker = ColorPickerState::new(vec![]);
        picker.mode = PickerMode::Hex;
        picker.hex_input.value = text;
        let _ = picker.selected();
        let prefixed = format!("#{}", picker.hex_input.value);
        let _ = Color::from_hex(&prefixed);
        let _ = prefixed.parse::<Color>();
        picker.hex_input.value = prefixed;
        let _ = picker.selected();
    }

    #[test]
    fn mixed_event_batch_matches_single_event_frames(codes in proptest::collection::vec(0u8..12, 0..30)) {
        let events: Vec<Event> = codes.into_iter().flat_map(|code| match code {
            0 => EventBuilder::new().paste("ab").build(),
            1 => EventBuilder::new().paste("e\u{301}").build(),
            2 => key(KeyCode::Char('\u{301}')),
            3 => key(KeyCode::Char('\u{d55c}')),
            4 => key(KeyCode::Backspace),
            5 => key(KeyCode::Delete),
            6 => key(KeyCode::Left),
            7 => key(KeyCode::Right),
            8 => key(KeyCode::Up),
            9 => key(KeyCode::Down),
            10 => key(KeyCode::Home),
            _ => key(KeyCode::End),
        }).collect();
        let mut text_batch = TextInputState::new().max_length(12);
        let mut text_split = TextInputState::new().max_length(12);
        let mut area_batch = TextareaState::new().word_wrap(5).max_length(12);
        let mut area_split = area_batch.clone();
        input(&mut text_batch, events.clone()); area(&mut area_batch, events.clone());
        for event in events { input(&mut text_split, vec![event.clone()]); area(&mut area_split, vec![event]); }
        proptest::prop_assert_eq!((text_batch.value, text_batch.cursor), (text_split.value, text_split.cursor));
        proptest::prop_assert_eq!((&area_batch.lines, area_batch.cursor_row, area_batch.cursor_col),
            (&area_split.lines, area_split.cursor_row, area_split.cursor_col));
    }
}

proptest::proptest! {
    // Honor PROPTEST_CASES for stress runs; default is 256, rather than the
    // former explicit 96 which masked the environment override.
    #![proptest_config(proptest::test_runner::Config::default())]
    #[test]
    fn multiline_edit_undo_navigation_batch_matches_split(codes in proptest::collection::vec(0u8..10, 0..30)) {
        check_multiline_batch(&codes)?;
    }

    #[test]
    fn multiline_viewport_partitions_preserve_editing_and_affinity(
        codes in proptest::collection::vec(0u8..10, 0..60),
        prefill in 0usize..7,
        chunk_size in 1usize..12,
        visible_rows in 1u32..9,
        wrap_width in proptest::option::of(1u32..8),
        history_cap in 0usize..5,
    ) {
        let mut state = TextareaState::new().max_length(20).history_max(history_cap);
        state.set_value("\u{d55c}e\u{301}\n\u{1f469}\u{200d}\u{1f4bb}");
        state.wrap_width = wrap_width;
        let mut events = multiline_events(&vec![0; prefill]);
        events.extend(multiline_events(&codes));
        check_multiline_partition(state, events, chunk_size, visible_rows)?;
    }
}

#[test]
#[ignore = "release-mode timing probe; run explicitly with --ignored --nocapture"]
fn textarea_workload_timings() {
    use std::hint::black_box;
    use std::time::Instant;

    for (kind, unit) in [
        ("ascii", "a"),
        ("cjk", "\u{d55c}"),
        ("zwj", "\u{1f469}\u{200d}\u{1f4bb}"),
    ] {
        for bytes in [1_024usize, 102_400, 1_048_576] {
            let line = format!("{}\n", unit.repeat(40));
            let document = line.repeat(bytes.div_ceil(line.len()));
            for workload in ["idle", "edit", "paste_end", "paste_middle"] {
                let mut times = Vec::new();
                for _ in 0..11 {
                    let mut state = TextareaState::new().word_wrap(60);
                    state.set_value(document.clone());
                    state.cursor_row = state.lines.len() / 2;
                    state.cursor_col = if workload == "paste_middle" { 20 } else { 40 };
                    let mut backend = TestBackend::new(80, 24);
                    backend.render_with_events(vec![], 0, 1, |ui| {
                        let _ = ui.textarea(&mut state, 20);
                    });
                    let events = match workload {
                        "edit" => EventBuilder::new().key('x').build(),
                        "paste_end" | "paste_middle" => {
                            EventBuilder::new().paste(&unit.repeat(128)).build()
                        }
                        _ => vec![],
                    };
                    let start = Instant::now();
                    backend.render_with_events(events, 0, 1, |ui| {
                        let _ = black_box(ui.textarea(&mut state, 20));
                    });
                    times.push(start.elapsed().as_nanos());
                    black_box(state);
                }
                times.sort_unstable();
                println!(
                    "{kind},{bytes},{workload},p50_ns={},p95_ns={}",
                    times[5], times[10]
                );
            }
        }
    }
}
