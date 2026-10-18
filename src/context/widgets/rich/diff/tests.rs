use super::*;
use crate::test_utils::{EventBuilder, TestBackend};
use crate::widgets::HunkId;
use proptest::prelude::*;

const TWO_HUNKS: &str = "\
diff --git a/src/app.txt b/src/app.txt
--- a/src/app.txt
+++ b/src/app.txt
@@ -1,3 +1,3 @@
 alpha
-beta
+BETA
 gamma
@@ -10,2 +10,3 @@ section
 ten
+inserted
 eleven
";

fn two_hunks() -> DiffState {
    DiffState::from_unified(TWO_HUNKS).unwrap()
}

/// Render one frame with `events`, returning the response.
fn frame(
    tb: &mut TestBackend,
    state: &mut DiffState,
    events: Vec<crate::Event>,
    build: impl FnOnce(DiffView<'_>) -> DiffView<'_>,
) -> DiffResponse {
    let mut response = DiffResponse::default();
    tb.run_with_events(events, |ui| {
        response = build(ui.diff(state)).show();
    });
    response
}

#[test]
fn unified_layout_renders_gutters_and_line_numbers() {
    let mut state = two_hunks();
    let mut tb = TestBackend::new(40, 12);
    tb.render(|ui| {
        let _ = ui.diff(&mut state).show();
    });
    insta::assert_snapshot!(tb.to_string_trimmed(), @r"
    M src/app.txt  +2 -1
    ▾ @@ -1,3 +1,3 @@
     1  1 │  alpha
     2    │- beta
        2 │+ BETA
     3  3 │  gamma
    ▾ @@ -10,2 +10,3 @@ section
    10 10 │  ten
       11 │+ inserted
    11 12 │  eleven
    ");
}

#[test]
fn split_layout_renders_both_sides() {
    let mut state = two_hunks();
    let mut tb = TestBackend::new(41, 12);
    tb.render(|ui| {
        let _ = ui
            .diff(&mut state)
            .layout(DiffLayout::Split)
            .split_min_width(40)
            .show();
    });
    insta::assert_snapshot!(tb.to_string_trimmed(), @r"
    M src/app.txt  +2 -1
    ▾ @@ -1,3 +1,3 @@
     1 │  alpha         │ 1 │  alpha
     2 │- beta          │ 2 │+ BETA
     3 │  gamma         │ 3 │  gamma
    ▾ @@ -10,2 +10,3 @@ section
    10 │  ten           │10 │  ten
                        │11 │+ inserted
    11 │  eleven        │12 │  eleven
    ");
}

#[test]
fn split_falls_back_to_unified_below_threshold() {
    let mut state = two_hunks();
    let mut tb = TestBackend::new(40, 12);
    tb.render(|ui| {
        let _ = ui.diff(&mut state).layout(DiffLayout::Split).show();
    });
    // Default threshold is 80 cells: the 40-cell area renders unified.
    let mut unified = TestBackend::new(40, 12);
    let mut state2 = two_hunks();
    unified.render(|ui| {
        let _ = ui.diff(&mut state2).show();
    });
    assert_eq!(tb.to_string(), unified.to_string());
}

#[test]
fn added_and_removed_rows_use_diff_backgrounds() {
    let mut state = two_hunks();
    let mut tb = TestBackend::new(40, 12);
    let theme = crate::Theme::dark();
    tb.render(|ui| {
        let _ = ui.diff(&mut state).show();
    });
    let added_bg = theme.success.blend_f64(theme.surface, LINE_BG_ALPHA);
    let removed_bg = theme.error.blend_f64(theme.surface, LINE_BG_ALPHA);
    // Row 4 is `+ BETA`, row 3 is `- beta`; check a padding cell far right.
    assert_eq!(tb.buffer().get(39, 4).style.bg, Some(added_bg));
    assert_eq!(tb.buffer().get(39, 3).style.bg, Some(removed_bg));
    assert_eq!(tb.buffer().get(39, 2).style.bg, None);
}

#[test]
fn bracket_moves_to_next_hunk_and_accept_reports_once() {
    let mut state = two_hunks();
    let mut tb = TestBackend::new(40, 12);
    let r = frame(&mut tb, &mut state, vec![], |v| v);
    assert_eq!(r.focused_hunk, Some(HunkId { file: 0, hunk: 0 }));

    let r = frame(
        &mut tb,
        &mut state,
        EventBuilder::new().key(']').build(),
        |v| v,
    );
    let second = HunkId { file: 0, hunk: 1 };
    assert_eq!(r.focused_hunk, Some(second));
    assert!(r.hunk_actions.is_empty());

    let r = frame(
        &mut tb,
        &mut state,
        EventBuilder::new().key('a').build(),
        |v| v,
    );
    assert_eq!(r.hunk_actions, vec![(second, HunkAction::Accept)]);
    assert!(r.changed);
    assert_eq!(state.decision(second), Some(HunkDecision::Accepted));
    tb.assert_contains("✓ accepted");

    // Next frame without input reports nothing; the decision is stored.
    let r = frame(&mut tb, &mut state, vec![], |v| v);
    assert!(r.hunk_actions.is_empty());
    assert!(!r.changed);
    // Re-accepting an accepted hunk is not a new action.
    let r = frame(
        &mut tb,
        &mut state,
        EventBuilder::new().key('a').build(),
        |v| v,
    );
    assert!(r.hunk_actions.is_empty());

    // `[` goes back; `r` rejects the first hunk.
    let r = frame(
        &mut tb,
        &mut state,
        EventBuilder::new().key('[').key('r').build(),
        |v| v,
    );
    let first = HunkId { file: 0, hunk: 0 };
    assert_eq!(r.hunk_actions, vec![(first, HunkAction::Reject)]);
    assert_eq!(state.decision(first), Some(HunkDecision::Rejected));
}

#[test]
fn enter_collapses_and_expands_the_focused_hunk() {
    let mut state = two_hunks();
    let mut tb = TestBackend::new(40, 12);
    let _ = frame(&mut tb, &mut state, vec![], |v| v);
    tb.assert_contains("BETA");

    let first = HunkId { file: 0, hunk: 0 };
    let r = frame(
        &mut tb,
        &mut state,
        EventBuilder::new().key_code(KeyCode::Enter).build(),
        |v| v,
    );
    assert_eq!(r.toggled, vec![first]);
    assert!(state.is_collapsed(first));
    tb.assert_not_contains("BETA");
    tb.assert_not_contains("alpha");
    tb.assert_contains("▸ @@ -1,3 +1,3 @@ (4 lines)");
    tb.assert_contains("inserted");

    let _ = frame(
        &mut tb,
        &mut state,
        EventBuilder::new().key(' ').build(),
        |v| v,
    );
    assert!(!state.is_collapsed(first));
    tb.assert_contains("BETA");
}

#[test]
fn clicking_a_hunk_header_toggles_it() {
    let mut state = two_hunks();
    let mut tb = TestBackend::new(40, 12);
    let _ = frame(&mut tb, &mut state, vec![], |v| v);
    let (_, y) = tb.find_text("@@ -10,2").expect("second hunk header");
    let r = frame(
        &mut tb,
        &mut state,
        EventBuilder::new().click(2, y).build(),
        |v| v,
    );
    let second = HunkId { file: 0, hunk: 1 };
    assert_eq!(r.toggled, vec![second]);
    assert_eq!(r.focused_hunk, Some(second));
    tb.assert_not_contains("inserted");
    // Clicking a body line moves the cursor without toggling.
    let (_, y) = tb.find_text("BETA").expect("body line");
    let r = frame(
        &mut tb,
        &mut state,
        EventBuilder::new().click(5, y).build(),
        |v| v,
    );
    assert!(r.toggled.is_empty());
    assert_eq!(r.focused_hunk, Some(HunkId { file: 0, hunk: 0 }));
}

#[test]
fn custom_keys_replace_defaults() {
    let mut state = two_hunks();
    state.keys.accept = vec![KeyCode::Char('y')];
    let mut tb = TestBackend::new(40, 12);
    let _ = frame(&mut tb, &mut state, vec![], |v| v);
    let r = frame(
        &mut tb,
        &mut state,
        EventBuilder::new().key('a').build(),
        |v| v,
    );
    assert!(r.hunk_actions.is_empty());
    let r = frame(
        &mut tb,
        &mut state,
        EventBuilder::new().key('y').build(),
        |v| v,
    );
    assert_eq!(r.hunk_actions.len(), 1);
}

/// A single-hunk patch with `steps` steps alternating a `-`/`+` pair and a
/// context line.
fn large_patch(steps: usize) -> String {
    let mut body = String::new();
    let mut side_count = 0;
    for i in 0..steps {
        if i % 2 == 0 {
            body.push_str(&format!("-old line {i}\n+new line {i}\n"));
        } else {
            body.push_str(&format!(" same line {i}\n"));
        }
        side_count += 1;
    }
    format!("--- a/big.txt\n+++ b/big.txt\n@@ -1,{side_count} +1,{side_count} @@\n{body}")
}

#[test]
fn large_patch_emits_commands_only_for_visible_rows() {
    let patch = large_patch(10_000);
    let mut state = DiffState::from_unified(&patch).unwrap();
    let total_lines: usize = state.files()[0].hunks[0].lines.len();
    assert!(total_lines >= 10_000);

    for layout in [DiffLayout::Unified, DiffLayout::Split] {
        let mut tb = TestBackend::new(120, 40);
        let mut emitted = 0;
        tb.render(|ui| {
            let before = ui.commands.len();
            let _ = ui.diff(&mut state).layout(layout).h(30).show();
            emitted = ui.commands.len() - before;
        });
        // 30 row commands + container begin/end + focus/interaction markers.
        assert!(
            (30..=36).contains(&emitted),
            "{layout:?}: emitted {emitted} commands for a 30-row viewport"
        );
    }

    // Scrolling to the end stays bounded too.
    let mut tb = TestBackend::new(120, 40);
    let _ = frame(&mut tb, &mut state, vec![], |v| v.h(30));
    let mut emitted = 0;
    tb.run_with_events(EventBuilder::new().key_code(KeyCode::End).build(), |ui| {
        let before = ui.commands.len();
        let _ = ui.diff(&mut state).h(30).show();
        emitted = ui.commands.len() - before;
    });
    assert!(emitted <= 36, "emitted {emitted}");
    tb.assert_contains("line 9999");
}

#[test]
fn cjk_and_wide_graphemes_truncate_at_grapheme_boundaries() {
    // `한` is 2 cells wide; with a 1-cell remainder the next one must be
    // dropped whole, never split. The flag emoji is one grapheme.
    let patch = "--- a/k.txt\n+++ b/k.txt\n@@ -1 +1 @@\n-한글한글한글한글한글한글한글한글\n+🇰🇷🇰🇷🇰🇷🇰🇷🇰🇷🇰🇷🇰🇷🇰🇷🇰🇷🇰🇷🇰🇷\n";
    for (layout, width) in [(DiffLayout::Unified, 21u32), (DiffLayout::Split, 43u32)] {
        let mut state = DiffState::from_unified(patch).unwrap();
        let mut tb = TestBackend::new(width, 6);
        tb.render(|ui| {
            let _ = ui
                .diff(&mut state)
                .layout(layout)
                .split_min_width(40)
                .show();
        });
        for y in 0..tb.height() {
            let line = tb.line(y);
            let cells = UnicodeWidthStr::width(line.as_str());
            assert!(
                cells <= width as usize,
                "{layout:?} row {y} overflows: {line:?} ({cells} cells)"
            );
            assert!(
                !line.contains('\u{FFFD}'),
                "{layout:?} split a grapheme: {line:?}"
            );
        }
        // Each wide cell pair is intact: no orphan half of a CJK glyph.
        tb.assert_contains("한글");
        tb.assert_contains("🇰🇷");
    }
}

#[test]
fn unified_row_width_is_exact_after_truncation() {
    let patch = "@@ -1 +1 @@\n-가나다라마바사아자차카타파하\n+abc\n";
    let mut state = DiffState::from_unified(patch).unwrap();
    let mut tb = TestBackend::new(20, 5);
    tb.render(|ui| {
        let _ = ui.diff(&mut state).line_numbers(false).show();
    });
    // Row 2: "- " + 18 content cells -> 9 Hangul syllables exactly.
    assert_eq!(tb.line(2), "- 가나다라마바사아자");
    // Odd budget: 19 cells for content would split the 10th syllable.
    let mut tb = TestBackend::new(21, 5);
    let mut state = DiffState::from_unified(patch).unwrap();
    tb.render(|ui| {
        let _ = ui.diff(&mut state).line_numbers(false).show();
    });
    assert_eq!(tb.line(2), "- 가나다라마바사아자");
}

#[test]
fn tabs_expand_to_configured_width() {
    let patch = "@@ -1 +1 @@\n-\tx\n+a\tb\n";
    let mut state = DiffState::from_unified(patch).unwrap();
    let mut tb = TestBackend::new(30, 5);
    tb.render(|ui| {
        let _ = ui.diff(&mut state).line_numbers(false).tab_width(4).show();
    });
    assert_eq!(tb.line(2), "-     x");
    assert_eq!(tb.line(3), "+ a   b");

    let mut tb = TestBackend::new(30, 5);
    tb.render(|ui| {
        let _ = ui.diff(&mut state).line_numbers(false).tab_width(2).show();
    });
    assert_eq!(tb.line(3), "+ a b");
}

#[test]
fn binary_files_are_reported_not_rendered() {
    let patch = "diff --git a/logo.png b/logo.png\nBinary files a/logo.png and b/logo.png differ\n";
    let mut state = DiffState::from_unified(patch).unwrap();
    let mut tb = TestBackend::new(40, 4);
    tb.render(|ui| {
        let _ = ui.diff(&mut state).show();
    });
    tb.assert_contains("M logo.png  binary");
    tb.assert_contains("binary file not shown");
}

#[test]
fn empty_diff_renders_placeholder() {
    let mut state = DiffState::new();
    let mut tb = TestBackend::new(20, 3);
    tb.render(|ui| {
        let _ = ui.diff(&mut state).show();
    });
    tb.assert_contains("No changes");
}

#[test]
fn drop_renders_like_show() {
    let mut a = two_hunks();
    let mut b = two_hunks();
    let mut tb_drop = TestBackend::new(40, 12);
    tb_drop.render(|ui| {
        ui.diff(&mut a).h(10);
    });
    let mut tb_show = TestBackend::new(40, 12);
    tb_show.render(|ui| {
        let _ = ui.diff(&mut b).h(10).show();
    });
    assert_eq!(tb_drop.to_string(), tb_show.to_string());
}

#[test]
fn wheel_scrolls_viewport() {
    let patch = large_patch(200);
    let mut state = DiffState::from_unified(&patch).unwrap();
    let mut tb = TestBackend::new(40, 10);
    let _ = frame(&mut tb, &mut state, vec![], |v| v);
    tb.assert_contains("line 0");
    let _ = frame(
        &mut tb,
        &mut state,
        EventBuilder::new()
            .scroll_down(5, 5)
            .scroll_down(5, 5)
            .build(),
        |v| v,
    );
    tb.assert_not_contains("old line 0");
    assert_eq!(state.scroll, 2 * WHEEL_ROWS);
}

#[test]
fn layout_switch_keeps_cursor_on_the_same_hunk() {
    let mut state = two_hunks();
    let mut tb = TestBackend::new(100, 12);
    let _ = frame(
        &mut tb,
        &mut state,
        EventBuilder::new().key(']').build(),
        |v| v,
    );
    let second = HunkId { file: 0, hunk: 1 };
    let r = frame(&mut tb, &mut state, vec![], |v| v.layout(DiffLayout::Split));
    assert_eq!(r.focused_hunk, Some(second));
}

#[cfg(feature = "syntax-rust")]
#[test]
fn rust_files_are_syntax_highlighted() {
    let patch =
        "--- a/m.rs\n+++ b/m.rs\n@@ -1,2 +1,2 @@\n fn main() {\n-    let x = 1;\n+    let y = 2;\n";
    let mut state = DiffState::from_unified(patch).unwrap();
    let mut tb = TestBackend::new(40, 6);
    tb.render(|ui| {
        let _ = ui.diff(&mut state).line_numbers(false).show();
    });
    let theme = crate::Theme::dark();
    // `fn` keyword on the context line uses the syntax palette, not plain text.
    let (x, y) = tb.find_text("fn main").expect("context line");
    assert_ne!(tb.buffer().get(x, y).style.fg, Some(theme.text));
    // Forcing an unknown language falls back to plain text.
    let mut tb = TestBackend::new(40, 6);
    tb.render(|ui| {
        let _ = ui.diff(&mut state).line_numbers(false).lang("none").show();
    });
    let (x, y) = tb.find_text("fn main").expect("context line");
    assert_eq!(tb.buffer().get(x, y).style.fg, Some(theme.text));
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn rendering_any_parsed_patch_never_panics(
        lines in proptest::collection::vec(
            proptest::string::string_regex(
                "(diff --git a/x b/y|--- a/x|\\+\\+\\+ b/x\
|@@ -[0-9]{1,3}(,[0-9])? \\+[0-9]{1,3}(,[0-9])? @@ ?[a-z]{0,3}\
|[ +\\-][ -~\t한🇰🇷]{0,30}|\\\\ No newline|Binary files a/x and b/x differ|)"
            ).unwrap(),
            0..30,
        ),
        width in 0u32..120,
        height in 0u32..20,
        split in any::<bool>(),
        keys in proptest::collection::vec(
            prop_oneof![
                Just(']'), Just('['), Just('a'), Just('r'), Just(' '), Just('j'), Just('k')
            ],
            0..6,
        ),
    ) {
        let input = lines.join("\n");
        if let Ok(mut state) = DiffState::from_unified(&input) {
            let layout = if split { DiffLayout::Split } else { DiffLayout::Unified };
            let mut tb = TestBackend::new(width.max(1), height.max(1));
            tb.render(|ui| {
                let _ = ui.diff(&mut state).layout(layout).split_min_width(20).w(width).h(height).show();
            });
            let mut events = EventBuilder::new();
            for key in keys {
                events = events.key(key);
            }
            tb.run_with_events(events.build(), |ui| {
                let _ = ui.diff(&mut state).layout(layout).split_min_width(20).w(width).h(height).show();
            });
        }
    }
}
