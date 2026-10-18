//! Issue #466: `code_editor` behaviour — editing keys, gutter, scrolling,
//! highlighting (with and without `syntax-*`) and the highlight cache.

use super::*;
use crate::{EventBuilder, KeyCode, KeyModifiers, TestBackend};

const SHIFT: KeyModifiers = KeyModifiers::SHIFT;
const CTRL: KeyModifiers = KeyModifiers::CONTROL;
const NONE: KeyModifiers = KeyModifiers::NONE;

const RUST_SNIPPET: &str = "fn main() {\n    let x = 1;\n}";

fn keys(list: &[(KeyCode, KeyModifiers)]) -> Vec<Event> {
    list.iter()
        .fold(EventBuilder::new(), |b, (code, mods)| {
            b.key_with(code.clone(), *mods)
        })
        .build()
}

fn chars(text: &str) -> Vec<(KeyCode, KeyModifiers)> {
    text.chars().map(|ch| (KeyCode::Char(ch), NONE)).collect()
}

/// Render once (focus + geometry), then feed `events` in one frame.
fn drive(tb: &mut TestBackend, events: Vec<Event>, mut f: impl FnMut(&mut Context)) {
    tb.render(&mut f);
    tb.sequence().events(events, f).run();
}

/// Feed each event in its own frame (keystroke by keystroke).
fn drive_each(tb: &mut TestBackend, events: Vec<Event>, mut f: impl FnMut(&mut Context)) {
    tb.render(&mut f);
    for event in events {
        tb.sequence().events(vec![event], &mut f).run();
    }
}

fn editor(source: &str) -> CodeEditorState {
    CodeEditorState::new(source).lang("rust")
}

// ── editing ──────────────────────────────────────────────────────────────

#[test]
fn typing_inserts_and_reports_changed() {
    let mut tb = TestBackend::new(40, 5);
    let mut ed = editor(RUST_SNIPPET);
    ed.set_cursor(1, 14);
    let mut changed = false;
    drive(&mut tb, keys(&chars(" // ok")), |ui| {
        changed = ui.code_editor(&mut ed).h(4).show().changed;
    });
    assert!(changed);
    assert_eq!(ed.lines()[1], "    let x = 1; // ok");
    tb.assert_contains("let x = 1; // ok");
}

#[test]
fn enter_copies_leading_whitespace() {
    let mut tb = TestBackend::new(40, 6);
    let mut ed = editor(RUST_SNIPPET);
    ed.set_cursor(1, 14);
    drive(
        &mut tb,
        keys(&[(KeyCode::Enter, NONE), (KeyCode::Char('y'), NONE)]),
        |ui| {
            let _ = ui.code_editor(&mut ed).h(5).show();
        },
    );
    assert_eq!(ed.value(), "fn main() {\n    let x = 1;\n    y\n}");
    assert_eq!(ed.cursor(), (2, 5));
}

#[test]
fn enter_after_open_bracket_adds_a_level_and_splits_the_closer() {
    let mut tb = TestBackend::new(40, 6);
    let mut ed = editor("fn f() {}");
    ed.set_cursor(0, 8);
    drive(&mut tb, keys(&[(KeyCode::Enter, NONE)]), |ui| {
        let _ = ui.code_editor(&mut ed).h(5).show();
    });
    assert_eq!(ed.value(), "fn f() {\n    \n}");
    assert_eq!(ed.cursor(), (1, 4));

    // Without a closer after the cursor only the level is added.
    let mut ed = editor("    if x {");
    ed.set_cursor(0, 10);
    drive(&mut tb, keys(&[(KeyCode::Enter, NONE)]), |ui| {
        let _ = ui.code_editor(&mut ed).h(5).show();
    });
    assert_eq!(ed.value(), "    if x {\n        ");
    assert_eq!(ed.cursor(), (1, 8));
}

#[test]
fn enter_inside_indentation_does_not_duplicate_it() {
    let mut tb = TestBackend::new(40, 6);
    let mut ed = editor("        x");
    ed.set_cursor(0, 4);
    drive(&mut tb, keys(&[(KeyCode::Enter, NONE)]), |ui| {
        let _ = ui.code_editor(&mut ed).h(5).show();
    });
    assert_eq!(ed.value(), "    \n        x");
    assert_eq!(ed.cursor(), (1, 4));
}

#[test]
fn auto_indent_off_inserts_a_plain_newline() {
    let mut tb = TestBackend::new(40, 6);
    let mut ed = editor(RUST_SNIPPET).auto_indent(false);
    ed.set_cursor(1, 14);
    drive(&mut tb, keys(&[(KeyCode::Enter, NONE)]), |ui| {
        let _ = ui.code_editor(&mut ed).h(5).show();
    });
    assert_eq!(ed.value(), "fn main() {\n    let x = 1;\n\n}");
}

#[test]
fn tab_inserts_spaces_to_the_next_tab_stop() {
    let mut tb = TestBackend::new(40, 4);
    let mut ed = editor("ab");
    ed.set_cursor(0, 2);
    drive(&mut tb, keys(&[(KeyCode::Tab, NONE)]), |ui| {
        let _ = ui.code_editor(&mut ed).h(3).show();
    });
    assert_eq!(ed.value(), "ab  ");
    assert_eq!(ed.cursor(), (0, 4));

    let mut ed = editor("ab").hard_tabs(true);
    ed.set_cursor(0, 1);
    drive(&mut tb, keys(&[(KeyCode::Tab, NONE)]), |ui| {
        let _ = ui.code_editor(&mut ed).h(3).show();
    });
    assert_eq!(ed.value(), "a\tb");
}

#[test]
fn tab_and_shift_tab_indent_and_dedent_a_selection_with_undo() {
    let mut tb = TestBackend::new(40, 6);
    let mut ed = editor("a\n  b\nc\nd");
    // Select from the middle of line 0 to the start of line 2: rows 0..=1.
    ed.textarea_mut().set_selection((0, 1), (2, 0));
    drive_each(
        &mut tb,
        keys(&[
            (KeyCode::Tab, NONE),
            (KeyCode::BackTab, SHIFT),
            (KeyCode::BackTab, SHIFT),
        ]),
        |ui| {
            let _ = ui.code_editor(&mut ed).h(5).show();
        },
    );
    assert_eq!(ed.value(), "a\nb\nc\nd");

    // Re-run the indent alone to check selection bookkeeping and undo.
    let mut ed = editor("a\n  b\nc\nd");
    ed.textarea_mut().set_selection((0, 1), (2, 0));
    drive(&mut tb, keys(&[(KeyCode::Tab, NONE)]), |ui| {
        let _ = ui.code_editor(&mut ed).h(5).show();
    });
    assert_eq!(ed.value(), "    a\n      b\nc\nd");
    assert_eq!(ed.textarea().selection(), Some((0, 5)..(2, 0)));
    drive(&mut tb, keys(&[(KeyCode::BackTab, SHIFT)]), |ui| {
        let _ = ui.code_editor(&mut ed).h(5).show();
    });
    assert_eq!(ed.value(), "a\n  b\nc\nd");
    // Each indent / dedent is one undo step.
    drive(&mut tb, keys(&[(KeyCode::Char('z'), CTRL)]), |ui| {
        let _ = ui.code_editor(&mut ed).h(5).show();
    });
    assert_eq!(ed.value(), "    a\n      b\nc\nd");
    drive(&mut tb, keys(&[(KeyCode::Char('z'), CTRL)]), |ui| {
        let _ = ui.code_editor(&mut ed).h(5).show();
    });
    assert_eq!(ed.value(), "a\n  b\nc\nd");
    drive(&mut tb, keys(&[(KeyCode::Char('y'), CTRL)]), |ui| {
        let _ = ui.code_editor(&mut ed).h(5).show();
    });
    assert_eq!(ed.value(), "    a\n      b\nc\nd");
}

#[test]
fn shift_tab_without_selection_dedents_the_cursor_line() {
    let mut tb = TestBackend::new(40, 4);
    let mut ed = editor("      x");
    ed.set_cursor(0, 7);
    drive(&mut tb, keys(&[(KeyCode::Tab, SHIFT)]), |ui| {
        let _ = ui.code_editor(&mut ed).h(3).show();
    });
    assert_eq!(ed.value(), "  x");
    assert_eq!(ed.cursor(), (0, 3));
}

#[test]
fn undo_reverts_auto_indent_enter_in_one_step() {
    let mut tb = TestBackend::new(40, 6);
    let mut ed = editor(RUST_SNIPPET);
    ed.set_cursor(1, 14);
    drive_each(
        &mut tb,
        keys(&[
            (KeyCode::Enter, NONE),
            (KeyCode::Char('z'), CTRL),
            (KeyCode::Char('y'), CTRL),
        ]),
        |ui| {
            let _ = ui.code_editor(&mut ed).h(5).show();
        },
    );
    assert_eq!(ed.value(), "fn main() {\n    let x = 1;\n    \n}");
    drive(&mut tb, keys(&[(KeyCode::Char('z'), CTRL)]), |ui| {
        let _ = ui.code_editor(&mut ed).h(5).show();
    });
    assert_eq!(ed.value(), RUST_SNIPPET);
    assert_eq!(ed.cursor(), (1, 14));
}

#[test]
fn tab_moves_focus_when_tab_indents_is_off() {
    let mut tb = TestBackend::new(40, 8);
    let mut ed = editor("x").tab_indents(false);
    let mut input = crate::widgets::TextInputState::new();
    drive(&mut tb, keys(&[(KeyCode::Tab, NONE)]), |ui| {
        let _ = ui.code_editor(&mut ed).h(3).show();
        let _ = ui.text_input(&mut input);
    });
    assert_eq!(ed.value(), "x");
    tb.sequence()
        .events(keys(&chars("q")), |ui| {
            let _ = ui.code_editor(&mut ed).h(3).show();
            let _ = ui.text_input(&mut input);
        })
        .run();
    assert_eq!(input.value, "q");
    assert_eq!(ed.value(), "x");
}

#[test]
fn focused_editor_owns_tab_for_shortcut_queries() {
    let mut tb = TestBackend::new(40, 4);
    let mut ed = editor("x");
    let mut seen = true;
    drive(&mut tb, keys(&[(KeyCode::Tab, NONE)]), |ui| {
        seen = ui.key_code(KeyCode::Tab);
        let _ = ui.code_editor(&mut ed).h(3).show();
    });
    assert!(!seen, "Tab is the editor's while it owns focus");
    assert_eq!(ed.value(), "    x");
}

#[test]
fn page_down_moves_by_the_viewport_height() {
    let mut tb = TestBackend::new(30, 6);
    let source: Vec<String> = (0..30).map(|i| format!("line {i}")).collect();
    let mut ed = CodeEditorState::new(source.join("\n"));
    drive(&mut tb, keys(&[(KeyCode::PageDown, NONE)]), |ui| {
        let _ = ui.code_editor(&mut ed).h(5).show();
    });
    assert_eq!(ed.cursor().0, 5);
    tb.assert_contains("line 5");
    drive(&mut tb, keys(&[(KeyCode::PageUp, SHIFT)]), |ui| {
        let _ = ui.code_editor(&mut ed).h(5).show();
    });
    assert_eq!(ed.cursor().0, 0);
    assert_eq!(ed.textarea().selection(), Some((0, 0)..(5, 0)));
}

// ── gutter and viewport ─────────────────────────────────────────────────

#[test]
fn gutter_right_aligns_numbers_and_widens_past_99_lines() {
    let mut tb = TestBackend::new(30, 4);
    let source = vec!["x"; 99].join("\n");
    let mut ed = CodeEditorState::new(source);
    tb.render(|ui| {
        let _ = ui.code_editor(&mut ed).h(3).show();
    });
    assert_eq!(tb.line(0), " 1 │ x");
    assert_eq!(tb.line(2), " 3 │ x");

    // Crossing 99 -> 100 lines widens the gutter by one digit.
    ed.set_cursor(98, 1);
    drive(&mut tb, keys(&[(KeyCode::Enter, NONE)]), |ui| {
        let _ = ui.code_editor(&mut ed).h(3).show();
    });
    assert_eq!(ed.line_count(), 100);
    assert_eq!(tb.line(0).trim_end(), " 98 │ x");
    assert_eq!(tb.line(2).trim_end(), "100 │");

    let mut ed = CodeEditorState::new("x");
    tb.render(|ui| {
        let _ = ui.code_editor(&mut ed).line_numbers(false).h(3).show();
    });
    assert_eq!(tb.line(0).trim_end(), "x");
}

#[test]
fn long_lines_scroll_horizontally_with_the_cursor() {
    let mut tb = TestBackend::new(20, 3);
    let line = format!("let s = \"{}END\";", "a".repeat(30));
    let mut ed = CodeEditorState::new(line.clone());
    ed.set_cursor(0, line.len());
    tb.render(|ui| {
        let _ = ui.code_editor(&mut ed).h(2).show();
    });
    // 20 cells - 4 gutter = 16 text cells; the cursor cell is last.
    assert_eq!(state_hscroll(&ed), line.len() + 1 - 16);
    tb.assert_contains("END\";");
    tb.assert_not_contains("let s");

    drive(&mut tb, keys(&[(KeyCode::Home, NONE)]), |ui| {
        let _ = ui.code_editor(&mut ed).h(2).show();
    });
    assert_eq!(state_hscroll(&ed), 0);
    tb.assert_contains("let s");
}

fn state_hscroll(ed: &CodeEditorState) -> usize {
    ed.hscroll
}

#[test]
fn tabs_render_to_tab_stops() {
    let mut tb = TestBackend::new(30, 3);
    let mut ed = CodeEditorState::new("\tx\nab\ty").tab_width(4);
    tb.render(|ui| {
        let _ = ui.code_editor(&mut ed).line_numbers(false).h(2).show();
    });
    assert_eq!(tb.line(0).trim_end(), "    x");
    assert_eq!(tb.line(1).trim_end(), "ab  y");
}

#[test]
fn soft_wrap_continues_rows_with_a_blank_gutter() {
    let mut tb = TestBackend::new(12, 4);
    let mut ed = CodeEditorState::new("abcdefghijkl\nz");
    tb.render(|ui| {
        let _ = ui.code_editor(&mut ed).wrap(true).h(4).show();
    });
    // 12 cells - 4 gutter = 8 text cells per row.
    assert_eq!(tb.line(0).trim_end(), "1 │ abcdefgh");
    assert_eq!(tb.line(1).trim_end(), "  │ ijkl");
    assert_eq!(tb.line(2).trim_end(), "2 │ z");
}

#[test]
fn soft_wrap_draws_the_cursor_on_its_visual_row() {
    let mut tb = TestBackend::new(12, 4);
    let mut ed = CodeEditorState::new("abcdefghijkl\nz");
    ed.set_cursor(0, 12);
    tb.render(|ui| {
        let _ = ui.code_editor(&mut ed).wrap(true).h(4).show();
    });
    // End of "ijkl" on the second visual row: x = 4 (gutter) + 4.
    let cell = tb.buffer().get(8, 1).style;
    assert!(cell.modifiers.contains(crate::Modifiers::REVERSED));
    drive(&mut tb, keys(&chars("m")), |ui| {
        let _ = ui.code_editor(&mut ed).wrap(true).h(4).show();
    });
    assert_eq!(tb.line(1).trim_end(), "  │ ijklm");
}

#[test]
fn click_places_the_cursor_after_the_gutter() {
    let mut tb = TestBackend::new(30, 5);
    let mut ed = CodeEditorState::new("hello\nworld");
    tb.render(|ui| {
        let _ = ui.code_editor(&mut ed).h(4).show();
    });
    // Gutter is "1 │ " (4 cells): x = 4 + 3 is column 3.
    tb.sequence()
        .events(EventBuilder::new().click(7, 1).build(), |ui| {
            let _ = ui.code_editor(&mut ed).h(4).show();
        })
        .run();
    assert_eq!(ed.cursor(), (1, 3));
}

#[test]
fn wheel_scrolls_away_from_the_cursor() {
    let mut tb = TestBackend::new(30, 6);
    let source: Vec<String> = (0..40).map(|i| format!("row {i}")).collect();
    let mut ed = CodeEditorState::new(source.join("\n"));
    tb.render(|ui| {
        let _ = ui.code_editor(&mut ed).h(5).show();
    });
    tb.sequence()
        .events(EventBuilder::new().scroll_down(5, 2).build(), |ui| {
            let _ = ui.code_editor(&mut ed).h(5).show();
        })
        .run();
    assert_eq!(ed.textarea().scroll_offset, WHEEL_ROWS);
    assert_eq!(ed.cursor(), (0, 0));
    tb.assert_contains(&format!("row {WHEEL_ROWS}"));
}

// ── decorations ─────────────────────────────────────────────────────────

#[test]
fn cursor_line_and_matching_bracket_are_highlighted() {
    let mut tb = TestBackend::new(30, 4);
    let mut ed = CodeEditorState::new("f(a)\nb");
    ed.set_cursor(0, 1);
    tb.render(|ui| {
        let _ = ui.code_editor(&mut ed).line_numbers(false).h(3).show();
    });
    let theme = crate::Theme::dark();
    // The cursor cell `(` is reversed; its match `)` gets the bracket bg.
    let cursor = tb.buffer().get(1, 0).style;
    assert!(cursor.modifiers.contains(crate::Modifiers::REVERSED));
    let close = tb.buffer().get(3, 0).style;
    assert_eq!(close.bg, Some(theme.surface_hover));
    assert!(close.modifiers.contains(crate::Modifiers::BOLD));
    // The rest of the cursor line carries the current-line background.
    assert_eq!(tb.buffer().get(10, 0).style.bg, Some(theme.surface));
    assert_eq!(tb.buffer().get(1, 1).style.bg, None);

    let mut ed = CodeEditorState::new("f(a)\nb");
    ed.set_cursor(0, 1);
    tb.render(|ui| {
        let _ = ui
            .code_editor(&mut ed)
            .line_numbers(false)
            .current_line(false)
            .bracket_match(false)
            .h(3)
            .show();
    });
    assert_eq!(tb.buffer().get(3, 0).style.bg, None);
    assert_eq!(tb.buffer().get(10, 0).style.bg, None);
}

#[test]
fn matching_bracket_scans_across_lines() {
    let lines: Vec<String> = ["fn f() {", "    g(1);", "}"]
        .iter()
        .map(|line| line.to_string())
        .collect();
    assert_eq!(matching_bracket(&lines, (0, 7)), Some([(0, 7), (2, 0)]));
    // Cursor just after the closer.
    assert_eq!(matching_bracket(&lines, (2, 1)), Some([(2, 0), (0, 7)]));
    assert_eq!(matching_bracket(&lines, (1, 6)), Some([(1, 5), (1, 7)]));
    assert_eq!(matching_bracket(&lines, (1, 0)), None);
}

// ── highlighting ────────────────────────────────────────────────────────

/// Same snippet, same keys: the text and cursor match whether or not the
/// language highlights (plain fallback edits identically).
#[test]
fn editing_is_identical_with_and_without_highlighting() {
    let script = keys(&[
        (KeyCode::End, NONE),
        (KeyCode::Enter, NONE),
        (KeyCode::Char('l'), NONE),
        (KeyCode::Tab, NONE),
        (KeyCode::Char('z'), CTRL),
        (KeyCode::Down, NONE),
        (KeyCode::Enter, NONE),
    ]);
    let run = |lang: Option<&str>| {
        let mut tb = TestBackend::new(40, 6);
        let mut ed = CodeEditorState::new(RUST_SNIPPET);
        ed.set_lang(lang.map(str::to_string));
        drive_each(&mut tb, script.clone(), |ui| {
            let _ = ui.code_editor(&mut ed).h(5).show();
        });
        (ed.value(), ed.cursor(), tb.to_string_trimmed())
    };
    assert_eq!(run(Some("rust")), run(None));
    assert_eq!(run(Some("rust")), run(Some("not-a-language")));
}

#[test]
fn plain_render_snapshot() {
    let mut tb = TestBackend::new(24, 4);
    let mut ed = editor(RUST_SNIPPET);
    tb.render(|ui| {
        let _ = ui.code_editor(&mut ed).h(3).show();
    });
    assert_eq!(
        tb.to_string_trimmed(),
        "1 │ fn main() {\n2 │     let x = 1;\n3 │ }"
    );
}

#[cfg(feature = "syntax-rust")]
#[test]
fn keyword_is_styled_with_the_syntax_palette() {
    let mut tb = TestBackend::new(30, 4);
    let mut ed = editor(RUST_SNIPPET);
    tb.render(|ui| {
        let _ = ui.code_editor(&mut ed).h(3).show();
    });
    let theme = crate::Theme::dark();
    // Line 2 (not the cursor line): `let` at x = 4 (gutter) + 4 (indent).
    tb.assert_style_at(8, 1, Style::new().fg(theme.syntax.keyword));
}

#[cfg(not(feature = "syntax-rust"))]
#[test]
fn without_the_syntax_feature_text_is_plain() {
    let mut tb = TestBackend::new(30, 4);
    let mut ed = editor(RUST_SNIPPET);
    tb.render(|ui| {
        let _ = ui.code_editor(&mut ed).h(3).show();
    });
    let theme = crate::Theme::dark();
    tb.assert_style_at(8, 1, Style::new().fg(theme.text));
}

#[cfg(feature = "syntax-rust")]
#[test]
fn an_edit_rehighlights_only_its_chunk() {
    // Three top-level items -> three chunks, all visible.
    let source = "fn a() {\n    1;\n}\n\nfn b() {\n    2;\n}\n\nfn c() {\n    3;\n}";
    let mut tb = TestBackend::new(40, 12);
    let mut ed = editor(source);
    tb.render(|ui| {
        let _ = ui.code_editor(&mut ed).h(11).show();
    });
    let first = ed.highlight.stats;
    assert_eq!(first.runs, 3);

    // Idle frame: everything comes from the cache.
    tb.render(|ui| {
        let _ = ui.code_editor(&mut ed).h(11).show();
    });
    assert_eq!(ed.highlight.stats.runs, 3);
    assert_eq!(ed.highlight.stats.hits, first.hits + 3);

    // Edit inside `fn b`: only that chunk re-runs; `a` and `c` are hits.
    ed.set_cursor(5, 5);
    let before = ed.highlight.stats;
    tb.sequence()
        .events(keys(&chars("0")), |ui| {
            let _ = ui.code_editor(&mut ed).h(11).show();
        })
        .run();
    let after = ed.highlight.stats;
    assert_eq!(after.runs - before.runs, 1);
    // `fn b` plus its trailing blank line.
    assert_eq!(after.lines - before.lines, 4);
    assert_eq!(after.hits - before.hits, 2);
    tb.assert_contains("20;");
}
