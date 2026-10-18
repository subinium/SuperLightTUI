//! Issue #465: text selection, clipboard and undo in `text_input` and
//! `textarea`. In-crate so tests can read the queued OSC 52 clipboard text.

use super::*;
use crate::{EventBuilder, KeyCode, KeyModifiers, TestBackend};
use proptest::prelude::*;

const SHIFT: KeyModifiers = KeyModifiers::SHIFT;
const CTRL: KeyModifiers = KeyModifiers::CONTROL;
const SHIFT_CTRL: KeyModifiers = KeyModifiers(KeyModifiers::SHIFT.0 | KeyModifiers::CONTROL.0);

fn ctrl(c: char) -> (KeyCode, KeyModifiers) {
    (KeyCode::Char(c), CTRL)
}

/// Render `f` once (so the widget owns focus and geometry), then feed
/// `events` without resetting focus. Returns the clipboard text queued by
/// the event frame.
fn drive(
    tb: &mut TestBackend,
    events: Vec<Event>,
    mut f: impl FnMut(&mut Context),
) -> Option<String> {
    tb.render(&mut f);
    let mut clip = None;
    tb.sequence()
        .events(events, |ui| {
            f(ui);
            clip = ui.clipboard_text.clone();
        })
        .run();
    // Presses after the first one in a batch run on the following frames
    // (one press per frame); render them like a run loop would.
    while tb.has_pending_input() {
        tb.run_with_events(Vec::new(), |ui| {
            f(ui);
            if let Some(text) = ui.clipboard_text.clone() {
                clip = Some(text);
            }
        });
    }
    clip
}

fn input_with(value: &str) -> TextInputState {
    let mut s = TextInputState::new();
    s.set_value(value);
    s
}

fn area_with(value: &str) -> TextareaState {
    let mut s = TextareaState::new();
    s.set_value(value);
    s
}

fn keys(list: &[(KeyCode, KeyModifiers)]) -> Vec<Event> {
    list.iter()
        .fold(EventBuilder::new(), |b, (code, mods)| {
            b.key_with(code.clone(), *mods)
        })
        .build()
}

// ── keyboard selection ──────────────────────────────────────────────────

#[test]
fn text_input_home_shift_end_then_typing_replaces_everything() {
    let mut tb = TestBackend::new(30, 5);
    let mut s = input_with("hello");
    let ev = EventBuilder::new()
        .key_code(KeyCode::Home)
        .key_with(KeyCode::End, SHIFT)
        .key('x')
        .build();
    drive(&mut tb, ev, |ui| {
        let _ = ui.text_input(&mut s);
    });
    assert_eq!(s.value, "x");
    assert_eq!(s.cursor, 1);
    assert_eq!(s.selection(), None);
}

#[test]
fn text_input_shift_ctrl_left_selects_one_word_and_backspace_removes_it() {
    let mut tb = TestBackend::new(30, 5);
    let mut s = input_with("hello big world");
    let ev = keys(&[(KeyCode::Left, SHIFT_CTRL)]);
    drive(&mut tb, ev, |ui| {
        let _ = ui.text_input(&mut s);
    });
    assert_eq!(s.selected_text(), Some("world"));
    let ev = keys(&[(KeyCode::Backspace, KeyModifiers::NONE)]);
    drive(&mut tb, ev, |ui| {
        let _ = ui.text_input(&mut s);
    });
    assert_eq!(s.value, "hello big ");
    assert_eq!(s.cursor, 10);
}

#[test]
fn text_input_plain_arrows_collapse_the_selection() {
    let mut tb = TestBackend::new(30, 5);
    let mut s = input_with("abcdef");
    s.set_selection(1, 4);
    drive(
        &mut tb,
        keys(&[(KeyCode::Left, KeyModifiers::NONE)]),
        |ui| {
            let _ = ui.text_input(&mut s);
        },
    );
    assert_eq!((s.cursor, s.selection()), (1, None));
    s.set_selection(1, 4);
    drive(
        &mut tb,
        keys(&[(KeyCode::Right, KeyModifiers::NONE)]),
        |ui| {
            let _ = ui.text_input(&mut s);
        },
    );
    assert_eq!((s.cursor, s.selection()), (4, None));
}

#[test]
fn textarea_home_shift_end_then_typing_replaces_the_line() {
    let mut tb = TestBackend::new(30, 6);
    let mut s = area_with("hello");
    s.cursor_col = 5;
    let ev = EventBuilder::new()
        .key_code(KeyCode::Home)
        .key_with(KeyCode::End, SHIFT)
        .key('x')
        .build();
    drive(&mut tb, ev, |ui| {
        let _ = ui.textarea(&mut s, 3);
    });
    assert_eq!(s.value(), "x");
    assert_eq!((s.cursor_row, s.cursor_col), (0, 1));
}

#[test]
fn textarea_shift_ctrl_left_selects_a_word_and_shift_up_spans_lines() {
    let mut tb = TestBackend::new(30, 6);
    let mut s = area_with("alpha beta\nsecond word");
    (s.cursor_row, s.cursor_col) = (1, 11);
    drive(&mut tb, keys(&[(KeyCode::Left, SHIFT_CTRL)]), |ui| {
        let _ = ui.textarea(&mut s, 3);
    });
    assert_eq!(s.selected_text().as_deref(), Some("word"));
    drive(&mut tb, keys(&[(KeyCode::Up, SHIFT)]), |ui| {
        let _ = ui.textarea(&mut s, 3);
    });
    // Up keeps column 7, which on the first line sits after "alpha b".
    assert_eq!(s.selected_text().as_deref(), Some("eta\nsecond word"));
    drive(
        &mut tb,
        keys(&[(KeyCode::Backspace, KeyModifiers::NONE)]),
        |ui| {
            let _ = ui.textarea(&mut s, 3);
        },
    );
    assert_eq!(s.value(), "alpha b");
}

// ── clipboard ───────────────────────────────────────────────────────────

#[test]
fn select_all_and_cut_empties_the_input_and_queues_osc52_text() {
    let mut tb = TestBackend::new(30, 5);
    let mut s = input_with("secret plan");
    let clip = drive(&mut tb, keys(&[ctrl('a'), ctrl('x')]), |ui| {
        let _ = ui.text_input(&mut s);
    });
    assert_eq!(s.value, "");
    assert_eq!(clip.as_deref(), Some("secret plan"));

    let mut area = area_with("two\nlines");
    let clip = drive(&mut tb, keys(&[ctrl('a'), ctrl('c')]), |ui| {
        let _ = ui.textarea(&mut area, 3);
    });
    assert_eq!(clip.as_deref(), Some("two\nlines"));
    assert_eq!(area.value(), "two\nlines");
}

#[test]
fn masked_input_never_queues_plaintext() {
    let mut tb = TestBackend::new(30, 5);
    let mut s = TextInputState::password();
    s.set_value("hunter2");
    let clip = drive(&mut tb, keys(&[ctrl('a'), ctrl('c')]), |ui| {
        let _ = ui.text_input(&mut s);
    });
    assert_eq!(clip, None);
    assert_eq!(s.selection(), Some(0..7), "selection still works");
    let clip = drive(&mut tb, keys(&[ctrl('x')]), |ui| {
        let _ = ui.text_input(&mut s);
    });
    assert_eq!(clip, None);
    assert_eq!(s.value, "", "cut still deletes");
}

#[test]
fn ctrl_c_without_selection_reaches_the_app_with_selection_it_copies() {
    let mut tb = TestBackend::new(30, 5);
    let mut s = input_with("abc");
    let mut before = false;
    let mut after = false;
    let clip = drive(&mut tb, keys(&[ctrl('c')]), |ui| {
        before = ui.key_mod('c', CTRL);
        let _ = ui.text_input(&mut s);
        after = ui.key_mod('c', CTRL);
    });
    assert!(before && after, "no selection: Ctrl+C is an app shortcut");
    assert_eq!(clip, None);

    s.select_all();
    let clip = drive(&mut tb, keys(&[ctrl('c')]), |ui| {
        before = ui.key_mod('c', CTRL);
        let _ = ui.text_input(&mut s);
        after = ui.key_mod('c', CTRL);
    });
    assert!(
        !before && !after,
        "with a selection the input owns Ctrl+C in either call order"
    );
    assert_eq!(clip.as_deref(), Some("abc"));
}

// ── undo ────────────────────────────────────────────────────────────────

#[test]
fn text_input_undo_redo_and_one_step_selection_replacement() {
    let mut tb = TestBackend::new(30, 5);
    let mut s = TextInputState::new();
    drive(
        &mut tb,
        EventBuilder::new().key('a').key('b').key('c').build(),
        |ui| {
            let _ = ui.text_input(&mut s);
        },
    );
    assert_eq!(s.value, "abc");
    s.set_selection(0, 2);
    drive(&mut tb, EventBuilder::new().key('Z').build(), |ui| {
        let _ = ui.text_input(&mut s);
    });
    assert_eq!(s.value, "Zc");
    drive(&mut tb, keys(&[ctrl('z')]), |ui| {
        let _ = ui.text_input(&mut s);
    });
    assert_eq!(s.value, "abc", "replacement undoes in one step");
    assert_eq!(s.selection(), Some(0..2), "and restores the selection");
    drive(&mut tb, keys(&[ctrl('z')]), |ui| {
        let _ = ui.text_input(&mut s);
    });
    assert_eq!(s.value, "", "typed burst is one step");
    drive(&mut tb, keys(&[ctrl('y'), ctrl('y')]), |ui| {
        let _ = ui.text_input(&mut s);
    });
    assert_eq!(s.value, "Zc");
}

#[test]
fn textarea_selection_replaced_by_typing_undoes_in_one_ctrl_z() {
    let mut tb = TestBackend::new(30, 6);
    let mut s = area_with("alpha\nbeta");
    s.set_selection((0, 2), (1, 2));
    drive(
        &mut tb,
        EventBuilder::new().key('X').key('Y').build(),
        |ui| {
            let _ = ui.textarea(&mut s, 3);
        },
    );
    assert_eq!(s.value(), "alXYta");
    drive(&mut tb, keys(&[ctrl('z')]), |ui| {
        let _ = ui.textarea(&mut s, 3);
    });
    assert_eq!(s.value(), "alpha\nbeta");
}

// ── mouse ───────────────────────────────────────────────────────────────

#[test]
fn text_input_mouse_drag_double_and_triple_click() {
    let mut tb = TestBackend::new(40, 5);
    let mut s = input_with("hello world");
    tb.render(|ui| {
        let _ = ui.text_input(&mut s);
    });
    let (x, y) = tb.find_text("hello").expect("rendered");
    let ev = EventBuilder::new()
        .click(x, y)
        .drag(x + 4, y)
        .mouse_up(x + 4, y)
        .build();
    drive(&mut tb, ev, |ui| {
        let _ = ui.text_input(&mut s);
    });
    assert_eq!(s.selection(), Some(0..4));

    let wx = x + 7; // inside "world"
    let ev = EventBuilder::new().click(wx, y).click(wx, y).build();
    drive(&mut tb, ev, |ui| {
        let _ = ui.text_input(&mut s);
    });
    assert_eq!(s.selected_text(), Some("world"));

    let ev = EventBuilder::new()
        .click(x + 1, y)
        .click(x + 1, y)
        .click(x + 1, y)
        .build();
    drive(&mut tb, ev, |ui| {
        let _ = ui.text_input(&mut s);
    });
    assert_eq!(s.selection(), Some(0..11));
}

#[test]
fn textarea_mouse_drag_double_and_triple_click() {
    let mut tb = TestBackend::new(40, 6);
    let mut s = area_with("one two\nthree");
    tb.render(|ui| {
        let _ = ui.textarea(&mut s, 3);
    });
    let (x, y) = tb.find_text("one").expect("rendered");
    // The focused caret glyph is drawn before "one" on the first row.
    let x = x - 1;
    let ev = EventBuilder::new()
        .click(x, y)
        .drag(x + 2, y + 1)
        .mouse_up(x + 2, y + 1)
        .build();
    drive(&mut tb, ev, |ui| {
        let _ = ui.textarea(&mut s, 3);
    });
    assert_eq!(s.selection(), Some((0, 0)..(1, 2)));

    let ev = EventBuilder::new()
        .click(x + 1, y + 1)
        .click(x + 1, y + 1)
        .build();
    drive(&mut tb, ev, |ui| {
        let _ = ui.textarea(&mut s, 3);
    });
    assert_eq!(s.selected_text().as_deref(), Some("three"));

    let ev = EventBuilder::new()
        .click(x + 5, y)
        .click(x + 5, y)
        .click(x + 5, y)
        .build();
    drive(&mut tb, ev, |ui| {
        let _ = ui.textarea(&mut s, 3);
    });
    assert_eq!(s.selected_text().as_deref(), Some("one two"));
}

/// A plain click places the caret without selecting. Typing after it must
/// insert every character; a degenerate `anchor == cursor` left by the
/// click used to turn the first typed character into a selection that the
/// second one replaced (`"abc"` became `"bc"`, seen in the browser runtime).
#[test]
fn text_input_click_then_typing_keeps_every_character() {
    let mut tb = TestBackend::new(30, 5);
    let mut s = TextInputState::new();
    tb.render(|ui| {
        let _ = ui.text_input(&mut s);
    });
    let (x, y) = tb.find_text("▎").expect("caret rendered");
    let ev = EventBuilder::new()
        .click(x, y)
        .mouse_up(x, y)
        .key('a')
        .key('b')
        .key('c')
        .build();
    drive(&mut tb, ev, |ui| {
        let _ = ui.text_input(&mut s);
    });
    assert_eq!(s.value, "abc");
    assert_eq!(s.selection(), None);

    // A drag that returns to its start selects nothing either.
    let (x, y) = tb.find_text("abc").expect("typed text rendered");
    let ev = EventBuilder::new()
        .click(x + 1, y)
        .drag(x + 2, y)
        .drag(x + 1, y)
        .mouse_up(x + 1, y)
        .key('X')
        .key('Y')
        .build();
    drive(&mut tb, ev, |ui| {
        let _ = ui.text_input(&mut s);
    });
    assert_eq!(s.value, "aXYbc");
    assert_eq!(s.selection(), None);
}

#[test]
fn textarea_click_then_typing_keeps_every_character() {
    let mut tb = TestBackend::new(30, 6);
    let mut s = area_with("one");
    tb.render(|ui| {
        let _ = ui.textarea(&mut s, 3);
    });
    let (x, y) = tb.find_text("one").expect("rendered");
    // The focused caret glyph is drawn before "one"; x + 1 sits between
    // 'o' and 'n'.
    let ev = EventBuilder::new()
        .click(x + 1, y)
        .mouse_up(x + 1, y)
        .key('a')
        .key('b')
        .build();
    drive(&mut tb, ev, |ui| {
        let _ = ui.textarea(&mut s, 3);
    });
    assert_eq!(s.value(), "oabne");
    assert_eq!(s.selection(), None);
}

// ── rendering ───────────────────────────────────────────────────────────

#[test]
fn selected_cells_use_the_selection_style_including_wide_graphemes() {
    let mut tb = TestBackend::new(30, 5);
    let mut s = input_with("a한b");
    s.set_selection(1, 2);
    let mut theme = Theme::dark();
    tb.render(|ui| {
        theme = *ui.theme();
        let _ = ui.text_input(&mut s);
    });
    let (x, y) = tb.find_text("a").expect("rendered");
    let selected = theme.text_selection_style();
    tb.assert_style_at(x + 1, y, selected);
    tb.assert_style_at(x + 2, y, selected);
    assert_ne!(tb.buffer().get(x, y).style.bg, selected.bg);
    assert_ne!(tb.buffer().get(x + 3, y).style.bg, selected.bg);

    // A theme token overrides the default selection background.
    let custom = Theme::builder()
        .selection_bg(Color::Rgb(10, 20, 30))
        .build();
    tb.render(|ui| {
        ui.set_theme(custom);
        let _ = ui.text_input(&mut s);
    });
    assert_eq!(
        tb.buffer().get(x + 1, y).style.bg,
        Some(Color::Rgb(10, 20, 30))
    );
}

#[test]
fn textarea_highlights_soft_wrapped_selection_per_visual_line() {
    let mut tb = TestBackend::new(30, 6);
    let mut s = TextareaState::new().word_wrap(5);
    s.set_value("abcdefgh");
    s.set_selection((0, 3), (0, 7));
    let mut theme = Theme::dark();
    tb.render(|ui| {
        theme = *ui.theme();
        let _ = ui.textarea(&mut s, 3);
    });
    let selected = theme.text_selection_style().bg;
    let (x, y) = tb.find_text("abc").expect("first visual line");
    assert_ne!(tb.buffer().get(x + 2, y).style.bg, selected);
    assert_eq!(tb.buffer().get(x + 3, y).style.bg, selected);
    assert_eq!(tb.buffer().get(x + 4, y).style.bg, selected);
    assert_eq!(tb.buffer().get(x, y + 1).style.bg, selected);
    assert_eq!(tb.buffer().get(x + 1, y + 1).style.bg, selected);
}

// ── grapheme safety ─────────────────────────────────────────────────────

fn tricky_text() -> impl Strategy<Value = String> {
    proptest::collection::vec(
        proptest::sample::select(vec![
            "a",
            "b",
            " ",
            "한",
            "字",
            "e\u{301}",
            "👩\u{200d}💻",
            "👨\u{200d}👩\u{200d}👧",
            "🇰🇷",
            "🇯🇵",
            "\u{1100}\u{1161}",
            "क\u{94d}ष",
        ]),
        0..12,
    )
    .prop_map(|parts| parts.concat())
}

fn movement() -> impl Strategy<Value = (KeyCode, KeyModifiers)> {
    proptest::sample::select(vec![
        (KeyCode::Left, SHIFT),
        (KeyCode::Right, SHIFT),
        (KeyCode::Left, SHIFT_CTRL),
        (KeyCode::Right, SHIFT_CTRL),
        (KeyCode::Home, SHIFT),
        (KeyCode::End, SHIFT),
        (KeyCode::Left, KeyModifiers::NONE),
        (KeyCode::Right, KeyModifiers::NONE),
    ])
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    #[test]
    fn selection_bounds_are_grapheme_boundaries_and_round_trip(
        value in tricky_text(),
        start in 0usize..14,
        moves in proptest::collection::vec(movement(), 0..8),
    ) {
        let mut tb = TestBackend::new(60, 5);
        let mut s = input_with(&value);
        let len = s.value.graphemes(true).count();
        s.cursor = start.min(len);
        drive(&mut tb, keys(&moves), |ui| { let _ = ui.text_input(&mut s); });
        prop_assert_eq!(&s.value, &value, "movement never edits");
        if let Some(range) = s.selection() {
            prop_assert!(range.end <= len);
            let bounds: Vec<usize> = value.grapheme_indices(true).map(|(b, _)| b)
                .chain(std::iter::once(value.len())).collect();
            let lo = bounds[range.start];
            let hi = bounds[range.end];
            let selected = s.selected_text().expect("selection").to_string();
            prop_assert_eq!(&selected, &value[lo..hi]);

            // Replacing the selection with its own text is the identity.
            let mut copy = s.clone();
            copy.insert_text(&selected);
            prop_assert_eq!(&copy.value, &value);

            // Deleting removes exactly that byte range.
            let mut cut = s.clone();
            cut.delete_backward();
            prop_assert_eq!(cut.value, format!("{}{}", &value[..lo], &value[hi..]));
        }
    }
}
