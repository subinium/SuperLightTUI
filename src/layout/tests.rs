#![allow(clippy::print_stderr)]
#![allow(clippy::unwrap_used)]

use super::tree::{ContainerConfig, default_container_config};
use super::*;

#[test]
fn wrap_empty() {
    assert_eq!(wrap_lines("", 10), vec![""]);
}

#[test]
fn wrap_fits() {
    assert_eq!(wrap_lines("hello", 10), vec!["hello"]);
}

#[test]
fn wrap_word_boundary() {
    assert_eq!(wrap_lines("hello world", 7), vec!["hello", "world"]);
}

#[test]
fn wrap_multiple_words() {
    assert_eq!(
        wrap_lines("one two three four", 9),
        vec!["one two", "three", "four"]
    );
}

#[test]
fn wrap_long_word() {
    assert_eq!(wrap_lines("abcdefghij", 4), vec!["abcd", "efgh", "ij"]);
}

#[test]
fn wrap_zero_width() {
    assert_eq!(wrap_lines("hello", 0), vec!["hello"]);
}

#[test]
fn wrap_lines_single_word_exact_fit() {
    // Word width equals max_width: fits on one line.
    assert_eq!(wrap_lines("hello", 5), vec!["hello"]);
}

#[test]
fn wrap_lines_cjk_wide_chars() {
    // Each CJK char has display width 2. At max_width=4 only two chars per line.
    assert_eq!(wrap_lines("日本語文字", 4), vec!["日本", "語文", "字"]);
}

#[test]
fn wrap_lines_cjk_with_space() {
    // Break at the space after the CJK chunk when the next word won't fit.
    assert_eq!(wrap_lines("日本 hello", 5), vec!["日本", "hello"]);
}

#[test]
fn wrap_lines_consecutive_spaces_collapse() {
    // Multiple spaces between words collapse to a single separator.
    assert_eq!(wrap_lines("a  b   c", 10), vec!["a b c"]);
}

#[test]
fn wrap_lines_combining_mark_preserved() {
    // Combining marks have width 0 and must stay attached to their base char.
    let input = "e\u{0301}llo";
    assert_eq!(wrap_lines(input, 10), vec![input]);
}

#[test]
fn wrap_lines_single_wide_char_over_width() {
    // A single wide char wider than max_width is emitted on its own line.
    assert_eq!(wrap_lines("日", 1), vec!["日"]);
}

#[test]
fn wrap_lines_only_spaces() {
    // Input of only whitespace produces a single empty output line.
    assert_eq!(wrap_lines("   ", 5), vec![""]);
}

#[test]
fn wrap_lines_hard_break_basic() {
    // Embedded '\n' is a hard break even though "a b" fits within max_width.
    assert_eq!(wrap_lines("a\nb", 10), vec!["a", "b"]);
}

#[test]
fn wrap_lines_hard_break_blank_line() {
    // Consecutive newlines produce a genuine blank line.
    assert_eq!(wrap_lines("a\n\nb", 10), vec!["a", "", "b"]);
}

#[test]
fn wrap_lines_hard_break_leading_newline() {
    assert_eq!(wrap_lines("\nb", 10), vec!["", "b"]);
}

#[test]
fn wrap_lines_hard_break_trailing_newline() {
    // Trailing '\n' opens a fresh empty line — split('\n'), not str::lines().
    assert_eq!(wrap_lines("a\n", 10), vec!["a", ""]);
}

#[test]
fn wrap_lines_hard_break_then_soft_wrap() {
    // Soft wrapping still applies independently within each hard-break paragraph.
    assert_eq!(
        wrap_lines("hello world\nfoo", 7),
        vec!["hello", "world", "foo"]
    );
}

#[test]
fn wrap_lines_hard_break_cjk() {
    assert_eq!(wrap_lines("日本\nhello", 5), vec!["日本", "hello"]);
}

#[test]
fn wrap_lines_crlf_normalized() {
    // "\r\n" collapses to a single break with no stray '\r' in output.
    assert_eq!(wrap_lines("a\r\nb", 10), vec!["a", "b"]);
}

#[test]
fn wrap_lines_no_literal_control_char_in_output() {
    for line in wrap_lines("alpha\nbeta\r\ngamma", 80) {
        assert!(!line.contains('\n') && !line.contains('\r'));
    }
}

#[test]
fn wrap_lines_zero_width_honors_hard_break() {
    // max_width == 0 still splits on '\n' and never emits a literal control char.
    assert_eq!(wrap_lines("a\nb", 0), vec!["a", "b"]);
}

#[test]
fn wrap_segments_empty_returns_single_empty_line() {
    let segs: Vec<(String, Style)> = Vec::new();
    assert_eq!(
        wrap_segments(&segs, 10),
        vec![Vec::<(String, Style)>::new()]
    );
}

#[test]
fn wrap_segments_zero_width_returns_single_empty_line() {
    let segs = vec![("hello".to_string(), Style::new())];
    assert_eq!(wrap_segments(&segs, 0), vec![Vec::<(String, Style)>::new()]);
}

#[test]
fn wrap_segments_single_style_greedy_break() {
    // wrap_segments uses greedy fill + rewind-to-last-space, which differs from
    // wrap_lines. Verify the exact line split here.
    let style = Style::new();
    let segs = vec![("hello world foo bar".to_string(), style)];
    let lines = wrap_segments(&segs, 9);
    let joined: Vec<String> = lines
        .iter()
        .map(|l| l.iter().map(|(t, _)| t.as_str()).collect())
        .collect();
    assert_eq!(joined, vec!["hello", "world", "foo bar"]);
}

#[test]
fn wrap_segments_mixed_styles_preserve_run_boundaries() {
    // Two adjacent runs with different styles should remain separated on the same line.
    let s1 = Style::new();
    let s2 = Style::new().fg(Color::Red);
    let segs = vec![("abc".to_string(), s1), ("def".to_string(), s2)];
    let lines = wrap_segments(&segs, 10);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].len(), 2);
    assert_eq!(lines[0][0].0, "abc");
    assert_eq!(lines[0][1].0, "def");
    assert_eq!(lines[0][0].1, s1);
    assert_eq!(lines[0][1].1, s2);
}

#[test]
fn wrap_segments_trailing_whitespace_trimmed() {
    let s1 = Style::new();
    let s2 = Style::new().fg(Color::Red);
    let segs = vec![("abc ".to_string(), s1), ("   ".to_string(), s2)];
    let lines = wrap_segments(&segs, 20);
    // The entire trailing-whitespace seg is dropped and the first seg is trimmed.
    assert_eq!(lines, vec![vec![("abc".to_string(), s1)]]);
}

#[test]
fn wrap_segments_break_on_space_with_style_change() {
    // A line overflows mid-next-word: rewind to the space and carry the rest forward.
    let s1 = Style::new();
    let s2 = Style::new().fg(Color::Red);
    let segs = vec![("abc ".to_string(), s1), ("defgh".to_string(), s2)];
    let lines = wrap_segments(&segs, 5);
    assert_eq!(lines.len(), 2);
    // Line 1: "abc" (trimmed), line 2: "defgh".
    assert_eq!(lines[0], vec![("abc".to_string(), s1)]);
    assert_eq!(lines[1], vec![("defgh".to_string(), s2)]);
}

#[test]
fn wrap_segments_cjk_with_mixed_styles() {
    let s1 = Style::new();
    let s2 = Style::new().fg(Color::Red);
    let segs = vec![("日本".to_string(), s1), ("語".to_string(), s2)];
    let lines = wrap_segments(&segs, 4);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], vec![("日本".to_string(), s1)]);
    assert_eq!(lines[1], vec![("語".to_string(), s2)]);
}

#[test]
fn wrap_segments_hard_break_basic() {
    let s = Style::new();
    let segs = vec![("a\nb".to_string(), s)];
    assert_eq!(
        wrap_segments(&segs, 10),
        vec![vec![("a".to_string(), s)], vec![("b".to_string(), s)]]
    );
}

#[test]
fn wrap_segments_hard_break_blank_line() {
    let s = Style::new();
    let segs = vec![("a\n\nb".to_string(), s)];
    assert_eq!(
        wrap_segments(&segs, 10),
        vec![
            vec![("a".to_string(), s)],
            Vec::<(String, Style)>::new(),
            vec![("b".to_string(), s)],
        ]
    );
}

#[test]
fn wrap_segments_hard_break_across_segment_boundary() {
    // A '\n' that ends one styled segment still breaks the line; styles persist.
    let s1 = Style::new();
    let s2 = Style::new().fg(Color::Red);
    let segs = vec![("a\n".to_string(), s1), ("b".to_string(), s2)];
    assert_eq!(
        wrap_segments(&segs, 10),
        vec![vec![("a".to_string(), s1)], vec![("b".to_string(), s2)]]
    );
}

#[test]
fn wrap_segments_hard_break_then_soft_wrap() {
    // Soft word wrap still applies within each hard-break paragraph.
    let s = Style::new();
    let segs = vec![("hello world\nfoo".to_string(), s)];
    let joined: Vec<String> = wrap_segments(&segs, 9)
        .iter()
        .map(|l| l.iter().map(|(t, _)| t.as_str()).collect())
        .collect();
    assert_eq!(joined, vec!["hello", "world", "foo"]);
}

#[test]
fn wrap_segments_crlf_normalized() {
    let s = Style::new();
    let segs = vec![("a\r\nb".to_string(), s)];
    assert_eq!(
        wrap_segments(&segs, 10),
        vec![vec![("a".to_string(), s)], vec![("b".to_string(), s)]]
    );
}

#[test]
fn wrap_segments_no_literal_control_char_in_runs() {
    let s = Style::new();
    let segs = vec![("alpha\nbeta\r\ngamma".to_string(), s)];
    for line in wrap_segments(&segs, 80) {
        for (t, _) in line {
            assert!(!t.contains('\n') && !t.contains('\r'));
        }
    }
}

#[test]
fn diagnostic_demo_layout() {
    use crate::rect::Rect;
    use crate::style::{Align, Border, Constraints, Justify, Margin, Padding, Style};

    let mut root = LayoutNode::container(
        Direction::Column,
        ContainerConfig {
            gap: 0,
            align: Align::Start,
            align_self: None,
            justify: Justify::Start,
            border: None,
            border_sides: BorderSides::all(),
            border_style: Style::new(),
            bg_color: None,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
        },
    );

    let mut outer_container = LayoutNode::container(
        Direction::Column,
        ContainerConfig {
            gap: 0,
            align: Align::Start,
            align_self: None,
            justify: Justify::Start,
            border: Some(Border::Rounded),
            border_sides: BorderSides::all(),
            border_style: Style::new(),
            bg_color: None,
            padding: Padding::all(1),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 1,
        },
    );

    outer_container.children.push(LayoutNode::text(
        "header".to_string(),
        Style::new(),
        0,
        Align::Start,
        (None, false, false),
        Margin::default(),
        Constraints::default(),
    ));

    outer_container.children.push(LayoutNode::text(
        "separator".to_string(),
        Style::new(),
        0,
        Align::Start,
        (None, false, false),
        Margin::default(),
        Constraints::default(),
    ));

    let mut inner_container = LayoutNode::container(
        Direction::Column,
        ContainerConfig {
            gap: 0,
            align: Align::Start,
            align_self: None,
            justify: Justify::Start,
            border: None,
            border_sides: BorderSides::all(),
            border_style: Style::new(),
            bg_color: None,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 1,
        },
    );

    inner_container.children.push(LayoutNode::text(
        "content1".to_string(),
        Style::new(),
        0,
        Align::Start,
        (None, false, false),
        Margin::default(),
        Constraints::default(),
    ));
    inner_container.children.push(LayoutNode::text(
        "content2".to_string(),
        Style::new(),
        0,
        Align::Start,
        (None, false, false),
        Margin::default(),
        Constraints::default(),
    ));
    inner_container.children.push(LayoutNode::text(
        "content3".to_string(),
        Style::new(),
        0,
        Align::Start,
        (None, false, false),
        Margin::default(),
        Constraints::default(),
    ));

    outer_container.children.push(inner_container);

    outer_container.children.push(LayoutNode::text(
        "separator2".to_string(),
        Style::new(),
        0,
        Align::Start,
        (None, false, false),
        Margin::default(),
        Constraints::default(),
    ));

    outer_container.children.push(LayoutNode::text(
        "footer".to_string(),
        Style::new(),
        0,
        Align::Start,
        (None, false, false),
        Margin::default(),
        Constraints::default(),
    ));

    root.children.push(outer_container);

    compute(&mut root, Rect::new(0, 0, 80, 50));

    eprintln!("\n=== DIAGNOSTIC LAYOUT TEST ===");
    eprintln!("Root node:");
    eprintln!("  pos: {:?}, size: {:?}", root.pos, root.size);

    let outer = &root.children[0];
    eprintln!("\nOuter bordered container (grow:1):");
    eprintln!("  pos: {:?}, size: {:?}", outer.pos, outer.size);

    let inner = &outer.children[2];
    eprintln!("\nInner container (grow:1, simulates scrollable):");
    eprintln!("  pos: {:?}, size: {:?}", inner.pos, inner.size);

    eprintln!("\nAll children of outer container:");
    for (i, child) in outer.children.iter().enumerate() {
        eprintln!("  [{}] pos: {:?}, size: {:?}", i, child.pos, child.size);
    }

    assert_eq!(root.size, (80, 50));
    assert_eq!(outer.size, (80, 50));

    let expected_inner_height = 50 - 2 - 2 - 4;
    assert_eq!(inner.size.1, expected_inner_height as u32);

    let expected_inner_y = 1 + 1 + 1 + 1;
    assert_eq!(inner.pos.1, expected_inner_y as u32);
}

#[test]
fn collect_focus_rects_from_markers() {
    use Style;

    let mut commands = vec![
        Command::FocusMarker(0),
        Command::Text {
            content: "input1".into(),
            cursor_offset: None,
            style: Style::new(),
            grow: 0,
            align: Align::Start,
            wrap: false,
            truncate: false,
            margin: Default::default(),
            constraints: Default::default(),
        },
        Command::FocusMarker(1),
        Command::Text {
            content: "input2".into(),
            cursor_offset: None,
            style: Style::new(),
            grow: 0,
            align: Align::Start,
            wrap: false,
            truncate: false,
            margin: Default::default(),
            constraints: Default::default(),
        },
    ];

    let mut tree = build_tree(&mut commands);
    let area = crate::rect::Rect::new(0, 0, 40, 10);
    compute(&mut tree, area);

    let mut fd = FrameData::default();
    collect_all(&tree, &mut fd);
    assert_eq!(fd.focus_rects.len(), 2);
    assert_eq!(fd.focus_rects[0].0, 0);
    assert_eq!(fd.focus_rects[1].0, 1);
    assert!(fd.focus_rects[0].1.width > 0);
    assert!(fd.focus_rects[1].1.width > 0);
    assert_ne!(fd.focus_rects[0].1.y, fd.focus_rects[1].1.y);
}

#[test]
fn focus_marker_tags_container() {
    use crate::style::{Border, Style};

    let mut commands = vec![
        Command::FocusMarker(0),
        Command::BeginContainer(Box::new(BeginContainerArgs {
            direction: Direction::Column,
            gap: 0,
            align: Align::Start,
            align_self: None,
            justify: Justify::Start,
            border: Some(Border::Single),
            border_sides: BorderSides::all(),
            border_style: Style::new(),
            bg_color: None,
            padding: Padding::default(),
            margin: Default::default(),
            constraints: Default::default(),
            title: None,
            grow: 0,
            group_name: None,
        })),
        Command::Text {
            content: "inside".into(),
            cursor_offset: None,
            style: Style::new(),
            grow: 0,
            align: Align::Start,
            wrap: false,
            truncate: false,
            margin: Default::default(),
            constraints: Default::default(),
        },
        Command::EndContainer,
    ];

    let mut tree = build_tree(&mut commands);
    let area = crate::rect::Rect::new(0, 0, 40, 10);
    compute(&mut tree, area);

    let mut fd = FrameData::default();
    collect_all(&tree, &mut fd);
    assert_eq!(fd.focus_rects.len(), 1);
    assert_eq!(fd.focus_rects[0].0, 0);
    assert!(fd.focus_rects[0].1.width >= 8);
    assert!(fd.focus_rects[0].1.height >= 3);
}

#[test]
fn wrapped_text_cache_reused_for_same_width() {
    let mut node = LayoutNode::text(
        "alpha beta gamma".to_string(),
        Style::new(),
        0,
        Align::Start,
        (None, true, false),
        Margin::default(),
        Constraints::default(),
    );

    let height_a = node.min_height_for_width(6);
    let first_ptr = node
        .text_data()
        .and_then(|t| t.cached_wrapped.as_ref())
        .map(Vec::as_ptr)
        .unwrap();
    let height_b = node.min_height_for_width(6);
    let second_ptr = node
        .text_data()
        .and_then(|t| t.cached_wrapped.as_ref())
        .map(Vec::as_ptr)
        .unwrap();

    assert_eq!(height_a, height_b);
    assert_eq!(first_ptr, second_ptr);
    assert_eq!(node.text_data().and_then(|t| t.cached_wrap_width), Some(6));
}

#[test]
fn collect_all_clips_raw_draw_to_scroll_viewport() {
    // Replaces the prior `collect_all_matches_raw_draw_collection` legacy-vs-
    // current parity test. The legacy `collect_raw_draw_rects` walker (a
    // `#[cfg(test)]` duplicate of the production branch) was removed; this
    // test exercises the production `collect_all` path directly.
    //
    // Setup: scroll container at (0,0) sized 20x4 with `scroll_offset = 2`,
    // containing a 6x3 RawDraw child at layout coordinates (1, 3).
    // Hand trace:
    //   inner viewport (no border/padding) = (0,0)..(20,4)
    //   screen_y     = 3 - 2 = 1            (image top in screen coords)
    //   img_bottom   = 1 + 3 = 4
    //   visible_top  = max(1, 0)     = 1
    //   visible_bot  = min(4, 4)     = 4
    //   visible_h    = 4 - 1         = 3    (entire image fits inside)
    //   top_clip     = max(0 - 1, 0) = 0    (image top sits below vp top)
    //   original_h   = 3                    (drives pixel renderer crop)
    let mut root = LayoutNode::container(Direction::Column, default_container_config());
    let mut scroll = LayoutNode::container(Direction::Column, default_container_config());
    scroll.is_scrollable = true;
    scroll.pos = (0, 0);
    scroll.size = (20, 4);
    scroll.scroll_offset = 2;
    let mut raw = LayoutNode::raw_draw(7, Constraints::default(), 0, Margin::default(), None, None);
    raw.pos = (1, 3);
    raw.size = (6, 3);
    scroll.children.push(raw);
    root.children.push(scroll);

    let mut fd = FrameData::default();
    collect_all(&root, &mut fd);
    let rects: Vec<_> = fd
        .raw_draw_rects
        .into_iter()
        .map(|r| (r.draw_id, r.rect, r.top_clip_rows, r.original_height))
        .collect();

    assert_eq!(
        rects,
        vec![(7, crate::rect::Rect::new(1, 1, 6, 3), 0, 3)],
        "collect_all must clip RawDraw rect into the scroll viewport, \
         leave top_clip_rows = 0 when the image top is already inside \
         the viewport, and report the unclipped original height"
    );
}

#[test]
fn group_names_share_arc_across_focus_descendants() {
    use std::sync::Arc;

    // Build a frame with 50 grouped containers, each holding 3 focusable text
    // children. After the DFS in `collect_all`, every focus slot must carry
    // the group's Arc pointer — and all 3 descendants of the same group must
    // share the *same* allocation (Arc::ptr_eq), proving the reader inherits
    // via a pointer bump rather than allocating a new String per focus hit.
    const N_GROUPS: usize = 50;
    const FOCUSES_PER_GROUP: usize = 3;
    let mut commands: Vec<Command> = Vec::new();
    let mut focus_id = 0usize;
    for i in 0..N_GROUPS {
        commands.push(Command::BeginContainer(Box::new(BeginContainerArgs {
            direction: Direction::Column,
            gap: 0,
            align: Align::Start,
            align_self: None,
            justify: Justify::Start,
            border: None,
            border_sides: BorderSides::all(),
            border_style: Style::new(),
            bg_color: None,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
            group_name: Some(Arc::from(format!("group-{i}").as_str())),
        })));
        for _ in 0..FOCUSES_PER_GROUP {
            commands.push(Command::FocusMarker(focus_id));
            commands.push(Command::Text {
                content: format!("row {focus_id}"),
                cursor_offset: None,
                style: Style::new(),
                grow: 0,
                align: Align::Start,
                wrap: false,
                truncate: false,
                margin: Margin::default(),
                constraints: Constraints::default(),
            });
            focus_id += 1;
        }
        commands.push(Command::EndContainer);
    }

    let mut tree = build_tree(&mut commands);
    let area = crate::rect::Rect::new(0, 0, 80, (N_GROUPS * FOCUSES_PER_GROUP) as u32 + 4);
    compute(&mut tree, area);
    let mut fd = FrameData::default();
    collect_all(&tree, &mut fd);

    // All N_GROUPS group rects present with the expected names.
    assert_eq!(
        fd.group_rects.len(),
        N_GROUPS,
        "one group rect per container"
    );
    for (i, (name, _rect)) in fd.group_rects.iter().enumerate() {
        assert_eq!(name.as_ref(), format!("group-{i}"));
    }

    // Every focus slot must carry the correct group, and all focus slots
    // within the same group must share the SAME Arc allocation.
    assert_eq!(
        fd.focus_groups.len(),
        N_GROUPS * FOCUSES_PER_GROUP,
        "one focus slot per focusable"
    );
    for g in 0..N_GROUPS {
        let base = g * FOCUSES_PER_GROUP;
        let first = fd.focus_groups[base]
            .as_ref()
            .unwrap_or_else(|| panic!("focus slot {base} missing group"));
        assert_eq!(first.as_ref(), format!("group-{g}"));
        for k in 1..FOCUSES_PER_GROUP {
            let slot = fd.focus_groups[base + k]
                .as_ref()
                .unwrap_or_else(|| panic!("focus slot {} missing group", base + k));
            assert!(
                Arc::ptr_eq(first, slot),
                "focus slots within group-{g} must share the same Arc allocation"
            );
        }
    }
}

#[test]
fn flexbox_row_many_children_overflow_scratch() {
    // Row with 32 text children (> INLINE_CAP=16) must still lay out correctly,
    // exercising the U32Stack heap-overflow path in layout_row. Semantic equivalence
    // check: positions are strictly increasing and non-overlapping.
    const N: usize = 32;
    let mut commands: Vec<Command> = Vec::new();
    commands.push(Command::BeginContainer(Box::new(BeginContainerArgs {
        direction: Direction::Row,
        gap: 0,
        align: Align::Start,
        align_self: None,
        justify: Justify::Start,
        border: None,
        border_sides: BorderSides::all(),
        border_style: Style::new(),
        bg_color: None,
        padding: Padding::default(),
        margin: Margin::default(),
        constraints: Constraints::default(),
        title: None,
        grow: 0,
        group_name: None,
    })));
    for i in 0..N {
        commands.push(Command::Text {
            content: format!("c{i}"),
            cursor_offset: None,
            style: Style::new(),
            grow: 1,
            align: Align::Start,
            wrap: false,
            truncate: false,
            margin: Default::default(),
            constraints: Default::default(),
        });
    }
    commands.push(Command::EndContainer);

    let mut tree = build_tree(&mut commands);
    compute(&mut tree, crate::rect::Rect::new(0, 0, 200, 4));

    let row = &tree.children[0];
    assert_eq!(row.children.len(), N);
    // Monotonically increasing x positions, non-overlapping, total width bounded by area.
    let mut prev_end = 0u32;
    for child in &row.children {
        assert!(child.pos.0 >= prev_end, "children must not overlap");
        prev_end = child.pos.0 + child.size.0;
    }
    assert!(prev_end <= 200);
}

#[test]
fn flexbox_column_many_children_overflow_scratch() {
    // Column with 20 fixed-height children exercises layout_column's heap-overflow
    // path in the U32Stack scratch (> INLINE_CAP=16).
    const N: usize = 20;
    let mut commands: Vec<Command> = Vec::new();
    commands.push(Command::BeginContainer(Box::new(BeginContainerArgs {
        direction: Direction::Column,
        gap: 0,
        align: Align::Start,
        align_self: None,
        justify: Justify::Start,
        border: None,
        border_sides: BorderSides::all(),
        border_style: Style::new(),
        bg_color: None,
        padding: Padding::default(),
        margin: Margin::default(),
        constraints: Constraints::default(),
        title: None,
        grow: 0,
        group_name: None,
    })));
    for i in 0..N {
        commands.push(Command::Text {
            content: format!("row{i}"),
            cursor_offset: None,
            style: Style::new(),
            grow: 0,
            align: Align::Start,
            wrap: false,
            truncate: false,
            margin: Default::default(),
            constraints: Default::default(),
        });
    }
    commands.push(Command::EndContainer);

    let mut tree = build_tree(&mut commands);
    compute(&mut tree, crate::rect::Rect::new(0, 0, 20, 50));

    let col = &tree.children[0];
    assert_eq!(col.children.len(), N);
    let mut prev_end = 0u32;
    for child in &col.children {
        assert!(
            child.pos.1 >= prev_end,
            "children must not overlap vertically"
        );
        prev_end = child.pos.1 + child.size.1;
    }
    assert!(prev_end <= 50);
}

#[test]
fn flexbox_grow_with_max_width_no_gap() {
    // Row: 40px wide, no gap, two grow:1 children.
    // Child 0: grow:1, max_width:10 → rendered 10px, x should advance by 10.
    // Child 1: grow:1, no constraint → starts at x=10, rendered ~30px.
    use crate::style::{Align, Constraints, Justify, Margin, Padding};

    let mut commands = vec![
        Command::BeginContainer(Box::new(BeginContainerArgs {
            direction: Direction::Row,
            gap: 0,
            align: Align::Start,
            align_self: None,
            justify: Justify::Start,
            border: None,
            border_sides: BorderSides::all(),
            border_style: crate::style::Style::new(),
            bg_color: None,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
            group_name: None,
        })),
        Command::Text {
            content: "A".into(),
            cursor_offset: None,
            style: crate::style::Style::new(),
            grow: 1,
            align: Align::Start,
            wrap: false,
            truncate: false,
            margin: Margin::default(),
            constraints: Constraints::default().max_w(10),
        },
        Command::Text {
            content: "B".into(),
            cursor_offset: None,
            style: crate::style::Style::new(),
            grow: 1,
            align: Align::Start,
            wrap: false,
            truncate: false,
            margin: Margin::default(),
            constraints: Constraints::default(),
        },
        Command::EndContainer,
    ];

    let mut tree = build_tree(&mut commands);
    compute(&mut tree, crate::rect::Rect::new(0, 0, 40, 4));

    let row = &tree.children[0];
    assert_eq!(row.children.len(), 2);
    let c0 = &row.children[0];
    let c1 = &row.children[1];

    // Child 0 clamped to max_width=10
    assert_eq!(
        c0.size.0, 10,
        "child 0 width should be clamped to max_width=10"
    );
    // Child 1 must start immediately after child 0 (no gap)
    assert_eq!(
        c1.pos.0,
        c0.pos.0 + c0.size.0,
        "child 1 x must equal child 0 x + child 0 width (no gap)"
    );
}

#[test]
fn flexbox_column_grow_with_max_height_no_gap() {
    // Column: 20px tall, no gap, two grow:1 children.
    // Child 0: grow:1, max_height:5 → rendered 5px, y should advance by 5.
    // Child 1: grow:1, no constraint → starts at y=5.
    //
    // The outer column itself uses grow:1 so the implicit root column built by
    // `build_tree` stretches it to the full 20px area; otherwise the outer
    // column shrinks to its min_height and there is no flex space for c0/c1
    // to grow into.
    use crate::style::{Align, Constraints, Justify, Margin, Padding};

    let mut commands = vec![
        Command::BeginContainer(Box::new(BeginContainerArgs {
            direction: Direction::Column,
            gap: 0,
            align: Align::Start,
            align_self: None,
            justify: Justify::Start,
            border: None,
            border_sides: BorderSides::all(),
            border_style: crate::style::Style::new(),
            bg_color: None,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 1,
            group_name: None,
        })),
        Command::Text {
            content: "A".into(),
            cursor_offset: None,
            style: crate::style::Style::new(),
            grow: 1,
            align: Align::Start,
            wrap: false,
            truncate: false,
            margin: Margin::default(),
            constraints: Constraints::default().max_h(5),
        },
        Command::Text {
            content: "B".into(),
            cursor_offset: None,
            style: crate::style::Style::new(),
            grow: 1,
            align: Align::Start,
            wrap: false,
            truncate: false,
            margin: Margin::default(),
            constraints: Constraints::default(),
        },
        Command::EndContainer,
    ];

    let mut tree = build_tree(&mut commands);
    compute(&mut tree, crate::rect::Rect::new(0, 0, 20, 20));

    let col = &tree.children[0];
    assert_eq!(col.children.len(), 2);
    let c0 = &col.children[0];
    let c1 = &col.children[1];

    // Child 0 clamped to max_height=5
    assert_eq!(
        c0.size.1, 5,
        "child 0 height should be clamped to max_height=5"
    );
    // Child 1 must start immediately after child 0
    assert_eq!(
        c1.pos.1,
        c0.pos.1 + c0.size.1,
        "child 1 y must equal child 0 y + child 0 height (no gap)"
    );
}

// --- Regression tests for v0.19.1 hotfix ---------------------------------

#[test]
fn collect_all_keeps_scroll_invariant_with_nested_scrollables() {
    // Regression for issue #151: `collect_all_inner` historically split the
    // `scroll_infos` push and the `scroll_rects` push into two separate
    // `if node.is_scrollable` branches, making it possible to update one
    // side without the other and silently break the
    // `scroll_infos.len() == scroll_rects.len()` invariant enforced by the
    // outer pipeline. Merging the branches makes drift impossible; this test
    // pins the invariant so a future re-split fails loudly.
    //
    // Build a tree with two nested scrollable containers + one sibling
    // scrollable (3 scrollable nodes total, all visible) and verify both
    // vectors agree in length.
    let mut root = LayoutNode::container(Direction::Column, default_container_config());

    let mut outer = LayoutNode::container(Direction::Column, default_container_config());
    outer.is_scrollable = true;
    outer.pos = (0, 0);
    outer.size = (40, 20);

    let mut inner = LayoutNode::container(Direction::Column, default_container_config());
    inner.is_scrollable = true;
    inner.pos = (1, 1);
    inner.size = (38, 18);
    outer.children.push(inner);

    let mut sibling = LayoutNode::container(Direction::Column, default_container_config());
    sibling.is_scrollable = true;
    sibling.pos = (0, 25);
    sibling.size = (40, 10);

    root.children.push(outer);
    root.children.push(sibling);

    let mut fd = FrameData::default();
    collect_all(&root, &mut fd);

    assert_eq!(
        fd.scroll_infos.len(),
        fd.scroll_rects.len(),
        "scroll_infos and scroll_rects must always have equal length \
         (collect_all_inner branch merge invariant)"
    );
    assert_eq!(
        fd.scroll_infos.len(),
        3,
        "expected 3 scrollable nodes (outer, inner, sibling)"
    );
}

#[test]
fn raw_draw_constructor_matches_inline_literal_shape() {
    // Regression for issue #156: `LayoutNode::raw_draw` must produce a node
    // identical to the prior inline 34-field literal in `build_children`.
    // This pins every field that the constructor sets so a future drift in
    // the constructor or the build_children call site is caught immediately.
    let constraints = Constraints::default().min_w(7).min_h(2);
    let margin = Margin {
        top: 1,
        right: 2,
        bottom: 3,
        left: 4,
    };
    let node = LayoutNode::raw_draw(42, constraints, 5, margin, Some(11), Some(13));

    assert!(matches!(node.kind, NodeKind::RawDraw(42)));
    assert_eq!(node.grow, 5);
    assert_eq!(node.margin.top, 1);
    assert_eq!(node.margin.right, 2);
    assert_eq!(node.margin.bottom, 3);
    assert_eq!(node.margin.left, 4);
    assert_eq!(node.constraints.min_width(), Some(7));
    assert_eq!(node.constraints.min_height(), Some(2));
    assert_eq!(node.size, (7, 2), "size must seed from constraints minima");
    assert_eq!(node.pos, (0, 0));
    assert_eq!(node.focus_id, Some(11));
    assert_eq!(node.interaction_id, Some(13));
    // Defaults — none of these should be populated by the constructor.
    // Non-text nodes have no `text_data`; the text-only fields are
    // unreachable from this variant (issue #153 split them off).
    assert!(node.text_data().is_none());
    assert_eq!(node.align, Align::Start);
    assert!(node.align_self.is_none());
    assert_eq!(node.justify, Justify::Start);
    assert!(!node.wrap);
    assert!(!node.truncate);
    assert_eq!(node.gap, 0);
    assert!(node.border.is_none());
    assert!(node.bg_color.is_none());
    assert!(node.title.is_none());
    assert!(node.children.is_empty());
    assert!(!node.is_scrollable);
    assert_eq!(node.scroll_offset, 0);
    assert_eq!(node.content_height, 0);
    // The wrap caches and segments are inside `text_data`, which is
    // `None` for `RawDraw` nodes.
    assert!(node.link_url.is_none());
    assert!(node.group_name.is_none());
    assert!(node.overlays.is_empty());
}

/// Build a `LayoutNode` chain of the given depth without going through
/// `build_children` (which has its own depth guard at construction time).
/// Used to exercise the depth guards in `compute` / `collect_all_inner` /
/// `render_inner` directly.
fn build_deep_node(depth: usize) -> LayoutNode {
    let mut root = LayoutNode::container(Direction::Column, default_container_config());
    let mut cursor = &mut root;
    for _ in 0..depth {
        cursor.children.push(LayoutNode::container(
            Direction::Column,
            default_container_config(),
        ));
        cursor = cursor.children.last_mut().unwrap();
    }
    root
}

#[test]
#[should_panic(expected = "layout tree depth exceeds 512")]
fn compute_panics_at_depth_guard() {
    // Regression for issue #154: `compute` must panic with the documented
    // diagnostic message when recursion depth exceeds `MAX_LAYOUT_DEPTH`.
    // build_deep_node(514) yields a chain of 515 nested containers (root +
    // 514 children); the inner-most container is reached at depth 514,
    // which is past the limit of 512.
    let mut node = build_deep_node(514);
    compute(&mut node, crate::rect::Rect::new(0, 0, 80, 24));
}

#[test]
#[should_panic(expected = "layout tree depth exceeds 512")]
fn collect_all_panics_at_depth_guard() {
    // Regression for issue #154: `collect_all_inner` must panic with the
    // documented diagnostic message when recursion depth exceeds
    // `MAX_LAYOUT_DEPTH`. Sizes are populated directly (bypassing `compute`,
    // which would also panic) so the DFS reaches the inner-most depth.
    let mut node = build_deep_node(514);
    fn populate_sizes(n: &mut LayoutNode) {
        n.size = (10, 10);
        for c in &mut n.children {
            populate_sizes(c);
        }
    }
    populate_sizes(&mut node);
    let mut fd = FrameData::default();
    collect_all(&node, &mut fd);
}

#[test]
#[should_panic(expected = "layout tree depth exceeds 512")]
fn render_panics_at_depth_guard() {
    // Regression for issue #154: `render_inner` must panic with the
    // documented diagnostic message when recursion depth exceeds
    // `MAX_LAYOUT_DEPTH`. Sizes are populated directly so the renderer does
    // not short-circuit on the size-zero check before reaching the guard.
    let mut node = build_deep_node(514);
    fn populate_sizes(n: &mut LayoutNode) {
        n.size = (10, 10);
        n.pos = (0, 0);
        for c in &mut n.children {
            populate_sizes(c);
        }
    }
    populate_sizes(&mut node);
    let mut buf = crate::buffer::Buffer::empty(crate::rect::Rect::new(0, 0, 80, 24));
    super::render::render(&node, &mut buf);
}

#[test]
fn collect_all_reuses_buffer_without_leaking_prior_frame_data() {
    // Regression for issue #155: `collect_all(&tree, &mut fd)` must call
    // `fd.clear()` before populating, so a recycled `FrameData` does not
    // carry data from a previous frame's tree. The capacity is reused; the
    // contents are not.
    use crate::style::{Constraints, Margin};

    // Frame A: tree with two focusables.
    let mut tree_a = LayoutNode::container(Direction::Column, default_container_config());
    let mut focus_a0 = LayoutNode::container(Direction::Column, default_container_config());
    focus_a0.focus_id = Some(0);
    focus_a0.pos = (0, 0);
    focus_a0.size = (10, 1);
    tree_a.children.push(focus_a0);
    let mut focus_a1 = LayoutNode::container(Direction::Column, default_container_config());
    focus_a1.focus_id = Some(1);
    focus_a1.pos = (0, 1);
    focus_a1.size = (10, 1);
    tree_a.children.push(focus_a1);

    let mut fd = FrameData::default();
    collect_all(&tree_a, &mut fd);
    assert_eq!(fd.focus_rects.len(), 2, "frame A: two focus rects");

    // Frame B: a different tree with a single raw_draw (no focuses, no
    // groups). After collect_all, the recycled `fd` must reflect *only*
    // tree B — not a mix of A and B.
    let mut tree_b = LayoutNode::container(Direction::Column, default_container_config());
    let mut raw =
        LayoutNode::raw_draw(42, Constraints::default(), 0, Margin::default(), None, None);
    raw.pos = (0, 0);
    raw.size = (4, 2);
    tree_b.children.push(raw);

    collect_all(&tree_b, &mut fd);
    assert_eq!(
        fd.focus_rects.len(),
        0,
        "frame B: focus_rects must be cleared before refill"
    );
    assert_eq!(
        fd.raw_draw_rects.len(),
        1,
        "frame B: one raw_draw rect must be present"
    );
    assert_eq!(fd.raw_draw_rects[0].draw_id, 42);
}

#[test]
fn f12_debug_overlay_outlines_overlay_layer() {
    // Regression for issue #201 Part A: `render_debug_overlay` previously
    // walked only `node.children`, leaving any active overlay/modal invisible
    // to the F12 outline pass. Build a root with one base child + one
    // overlay child, and verify the overlay's bounds receive border chars.

    let mut root = LayoutNode::container(Direction::Column, default_container_config());

    // Base layer: a container at (0,0) sized 40×5.
    let mut base = LayoutNode::container(Direction::Column, default_container_config());
    base.pos = (0, 0);
    base.size = (40, 5);
    root.children.push(base);

    // Overlay layer: a container at (10,10) sized 20×4.
    let mut overlay_node = LayoutNode::container(Direction::Column, default_container_config());
    overlay_node.pos = (10, 10);
    overlay_node.size = (20, 4);
    root.overlays.push(super::tree::OverlayLayer {
        node: overlay_node,
        modal: false,
    });

    let mut buf = crate::buffer::Buffer::empty(crate::rect::Rect::new(0, 0, 40, 20));
    super::render::render_debug_overlay(&root, &mut buf, 0, 60.0, crate::DebugLayer::All);

    // The overlay container's top-RIGHT corner should now carry the outline
    // corner char ('┐'). The top-left position is overwritten by the depth
    // label '0', so we sample the right corner instead. Pre-fix the entire
    // overlay rect was untouched.
    let cell_top_right = buf.get(10 + 20 - 1, 10);
    assert_eq!(
        cell_top_right.symbol, "┐",
        "F12 overlay outline must hit overlay's top-right corner; got {:?}",
        cell_top_right.symbol
    );
    // Bottom-right corner of overlay.
    let cell_bottom_right = buf.get(10 + 20 - 1, 10 + 4 - 1);
    assert_eq!(
        cell_bottom_right.symbol, "┘",
        "F12 overlay outline must hit overlay's bottom-right corner; got {:?}",
        cell_bottom_right.symbol
    );
}

#[test]
fn count_leaf_widgets_matches_outline_count_with_overlays() {
    // Regression for issue #201 Part C: the status-bar widget count must
    // include overlay nodes so the displayed total reflects what the renderer
    // actually drew.
    let mut root = LayoutNode::container(Direction::Column, default_container_config());

    // 2 base widgets.
    root.children.push(LayoutNode::text(
        "a".to_string(),
        Style::new(),
        0,
        Align::Start,
        (None, false, false),
        Margin::default(),
        Constraints::default(),
    ));
    root.children.push(LayoutNode::text(
        "b".to_string(),
        Style::new(),
        0,
        Align::Start,
        (None, false, false),
        Margin::default(),
        Constraints::default(),
    ));

    // 1 overlay widget.
    let mut overlay_root = LayoutNode::container(Direction::Column, default_container_config());
    overlay_root.children.push(LayoutNode::text(
        "overlay".to_string(),
        Style::new(),
        0,
        Align::Start,
        (None, false, false),
        Margin::default(),
        Constraints::default(),
    ));
    root.overlays.push(super::tree::OverlayLayer {
        node: overlay_root,
        modal: false,
    });

    // Render the debug status bar at the bottom. The widget count is the
    // first integer following "| ".
    let mut buf = crate::buffer::Buffer::empty(crate::rect::Rect::new(0, 0, 80, 5));
    super::render::render_debug_overlay(&root, &mut buf, 0, 60.0, crate::DebugLayer::All);

    let mut bottom = String::new();
    for x in 0..80 {
        bottom.push_str(&buf.get(x, 4).symbol);
    }
    // Expected: 2 base widgets + 1 overlay widget = 3 total.
    assert!(
        bottom.contains("3 widgets"),
        "status bar widget count must include overlay nodes; got {bottom:?}"
    );
    // Per-layer breakdown is appended in parens whenever more than one
    // layer family is non-empty (matches the doc update in DEBUGGING.md).
    assert!(
        bottom.contains("2 base") && bottom.contains("1 overlay"),
        "status bar must include per-layer breakdown when multiple layers \
         are populated; got {bottom:?}"
    );
}

#[test]
fn f12_debug_overlay_distinguishes_layers_by_color() {
    // Regression for the #201 follow-up: each layer family (base / overlay /
    // modal) must paint its outlines in a distinct hue so the F12 view stays
    // legible when several layers are stacked. We sample a known cell on
    // each layer's border ring and check that the foreground colors differ
    // meaningfully (different `Color::Rgb` channels — testing equality is
    // sufficient because `debug_color_for_depth` returns concrete RGBs).
    let mut root = LayoutNode::container(Direction::Column, default_container_config());

    // Base container: (2, 2) sized 10x4. Bottom-right corner at (11, 5).
    let mut base = LayoutNode::container(Direction::Column, default_container_config());
    base.pos = (2, 2);
    base.size = (10, 4);
    root.children.push(base);

    // Non-modal overlay: (15, 2) sized 10x4. Bottom-right corner at (24, 5).
    let mut overlay_node = LayoutNode::container(Direction::Column, default_container_config());
    overlay_node.pos = (15, 2);
    overlay_node.size = (10, 4);
    root.overlays.push(super::tree::OverlayLayer {
        node: overlay_node,
        modal: false,
    });

    // Modal overlay: (28, 2) sized 10x4. Bottom-right corner at (37, 5).
    let mut modal_node = LayoutNode::container(Direction::Column, default_container_config());
    modal_node.pos = (28, 2);
    modal_node.size = (10, 4);
    root.overlays.push(super::tree::OverlayLayer {
        node: modal_node,
        modal: true,
    });

    let mut buf = crate::buffer::Buffer::empty(crate::rect::Rect::new(0, 0, 60, 8));
    super::render::render_debug_overlay(&root, &mut buf, 0, 60.0, crate::DebugLayer::All);

    // Sample the bottom-right corner '┘' of each container — that cell is
    // never overwritten by the depth label (which sits at the top-left).
    let base_fg = buf.get(11, 5).style.fg;
    let overlay_fg = buf.get(24, 5).style.fg;
    let modal_fg = buf.get(37, 5).style.fg;

    assert!(
        base_fg.is_some() && overlay_fg.is_some() && modal_fg.is_some(),
        "all three layer outlines must carry a foreground color; \
         got base={base_fg:?} overlay={overlay_fg:?} modal={modal_fg:?}"
    );
    assert_ne!(
        base_fg, overlay_fg,
        "base and overlay outlines must be different colors"
    );
    assert_ne!(
        base_fg, modal_fg,
        "base and modal outlines must be different colors"
    );
    assert_ne!(
        overlay_fg, modal_fg,
        "overlay and modal outlines must be different colors"
    );

    // Sanity-check the hue families: green-dominant for Base, red-dominant
    // for Overlay, blue-dominant for Modal. Catches future palette drift
    // that keeps colors distinct but loses the convention.
    if let Some(crate::style::Color::Rgb(r, g, b)) = base_fg {
        assert!(
            g > r && g > b,
            "Base outline should be green-dominant; got rgb({r},{g},{b})"
        );
    } else {
        panic!("Base outline must be Color::Rgb; got {base_fg:?}");
    }
    if let Some(crate::style::Color::Rgb(r, g, b)) = overlay_fg {
        assert!(
            r > g && r > b,
            "Overlay outline should be red-dominant; got rgb({r},{g},{b})"
        );
    } else {
        panic!("Overlay outline must be Color::Rgb; got {overlay_fg:?}");
    }
    if let Some(crate::style::Color::Rgb(r, g, b)) = modal_fg {
        assert!(
            b > r && b > g,
            "Modal outline should be blue-dominant; got rgb({r},{g},{b})"
        );
    } else {
        panic!("Modal outline must be Color::Rgb; got {modal_fg:?}");
    }
}

#[test]
fn build_tree_drains_in_place_preserving_capacity() {
    // Regression for issue #150: `build_tree` must drain its input Vec via
    // `&mut Vec<Command>` rather than consume by value, so the caller can
    // recycle the allocation across frames. Pre-fix the function took
    // `commands: Vec<Command>` and `into_iter()`d, dropping the buffer at
    // the end of each frame.
    let initial_cap = 64;
    let mut commands: Vec<Command> = Vec::with_capacity(initial_cap);
    commands.push(Command::Text {
        content: "hello".into(),
        cursor_offset: None,
        style: Style::new(),
        grow: 0,
        align: Align::Start,
        wrap: false,
        truncate: false,
        margin: Default::default(),
        constraints: Default::default(),
    });
    commands.push(Command::Spacer { grow: 1 });

    let cap_before = commands.capacity();
    let _tree = build_tree(&mut commands);

    assert_eq!(
        commands.len(),
        0,
        "build_tree must drain the Vec to len = 0"
    );
    assert!(
        commands.capacity() >= cap_before,
        "build_tree must preserve at least the prior capacity (was {cap_before}, now {})",
        commands.capacity()
    );

    // Second pass: same Vec, more commands. Capacity that already covers
    // the new push count must not trigger a reallocation. This is the
    // contract the frame loop relies on for steady-state zero-alloc.
    let cap_after_first = commands.capacity();
    for _ in 0..cap_after_first {
        commands.push(Command::Spacer { grow: 0 });
    }
    let cap_after_pushes = commands.capacity();
    assert_eq!(
        cap_after_first, cap_after_pushes,
        "pushing within the prior capacity must not realloc \
         (was {cap_after_first}, now {cap_after_pushes})"
    );
    let _tree2 = build_tree(&mut commands);
    assert_eq!(commands.len(), 0, "second drain must also leave len = 0");
}

// =====================================================================
// #161 — opt-in proportional flex-shrink
// =====================================================================
//
// Three regression tests cover the spec checklist from #161:
//
//   (a) no shrink   — output identical to pre-#161 (overflow-by-design).
//   (b) all shrink  — every fixed child scales by `available / fixed_total`.
//   (c) mixed       — only flagged children scale; the rest keep their size.

/// Helper: push a Column container holding a single 20-char text.
fn push_textcol_20(commands: &mut Vec<Command>, shrink: bool) {
    if shrink {
        commands.push(Command::ShrinkMarker);
    }
    commands.push(Command::BeginContainer(Box::new(BeginContainerArgs {
        direction: Direction::Column,
        gap: 0,
        align: Align::Start,
        align_self: None,
        justify: Justify::Start,
        border: None,
        border_sides: BorderSides::all(),
        border_style: Style::new(),
        bg_color: None,
        padding: Padding::default(),
        margin: Margin::default(),
        constraints: Constraints::default(),
        title: None,
        grow: 0,
        group_name: None,
    })));
    commands.push(Command::Text {
        content: "x".repeat(20),
        cursor_offset: None,
        style: Style::new(),
        grow: 0,
        align: Align::Start,
        wrap: false,
        truncate: false,
        margin: Default::default(),
        constraints: Default::default(),
    });
    commands.push(Command::EndContainer);
}

fn open_row(commands: &mut Vec<Command>) {
    commands.push(Command::BeginContainer(Box::new(BeginContainerArgs {
        direction: Direction::Row,
        gap: 0,
        align: Align::Start,
        align_self: None,
        justify: Justify::Start,
        border: None,
        border_sides: BorderSides::all(),
        border_style: Style::new(),
        bg_color: None,
        padding: Padding::default(),
        margin: Margin::default(),
        constraints: Constraints::default(),
        title: None,
        grow: 0,
        group_name: None,
    })));
}

#[test]
fn flex_shrink_default_off_preserves_overflow() {
    let mut commands: Vec<Command> = Vec::new();
    open_row(&mut commands);
    push_textcol_20(&mut commands, false);
    push_textcol_20(&mut commands, false);
    commands.push(Command::EndContainer);

    let mut tree = build_tree(&mut commands);
    let area = crate::rect::Rect::new(0, 0, 30, 4);
    compute(&mut tree, area);

    let row = &tree.children[0];
    assert_eq!(row.children.len(), 2);
    assert!(!row.children[0].shrink);
    assert!(!row.children[1].shrink);
    assert_eq!(row.children[0].size.0, 20);
    assert_eq!(row.children[1].size.0, 20);
}

#[test]
fn flex_shrink_all_children_proportional_distribution() {
    let mut commands: Vec<Command> = Vec::new();
    open_row(&mut commands);
    push_textcol_20(&mut commands, true);
    push_textcol_20(&mut commands, true);
    commands.push(Command::EndContainer);

    let mut tree = build_tree(&mut commands);
    let area = crate::rect::Rect::new(0, 0, 30, 4);
    compute(&mut tree, area);

    let row = &tree.children[0];
    assert_eq!(row.children.len(), 2);
    assert!(row.children[0].shrink);
    assert!(row.children[1].shrink);
    assert_eq!(row.children[0].size.0, 15);
    assert_eq!(row.children[1].size.0, 15);
}

#[test]
fn flex_shrink_mixed_only_flagged_scale() {
    let mut commands: Vec<Command> = Vec::new();
    open_row(&mut commands);
    push_textcol_20(&mut commands, true);
    push_textcol_20(&mut commands, false);
    commands.push(Command::EndContainer);

    let mut tree = build_tree(&mut commands);
    let area = crate::rect::Rect::new(0, 0, 30, 4);
    compute(&mut tree, area);

    let row = &tree.children[0];
    assert_eq!(row.children.len(), 2);
    assert!(row.children[0].shrink);
    assert!(!row.children[1].shrink);
    assert_eq!(row.children[0].size.0, 10);
    assert_eq!(row.children[1].size.0, 20);
}

// =====================================================================
// #258 — flex-wrap (multi-line row) + flex-basis
// =====================================================================
//
// Tree-level tests mirroring the #161 flex_shrink_* style: build a command
// stream with the relevant marker, `build_tree`, `compute` against a fixed
// `Rect`, and assert child `.pos` / `.size`.

/// Push a Column container holding a single `width`-char text. When `basis`
/// is `Some`, a `BasisMarker` precedes the container; when `shrink`, a
/// `ShrinkMarker` precedes it.
fn push_textcol(commands: &mut Vec<Command>, width: usize, shrink: bool, basis: Option<u32>) {
    if shrink {
        commands.push(Command::ShrinkMarker);
    }
    if let Some(b) = basis {
        commands.push(Command::BasisMarker(b));
    }
    commands.push(Command::BeginContainer(Box::new(BeginContainerArgs {
        direction: Direction::Column,
        gap: 0,
        align: Align::Start,
        align_self: None,
        justify: Justify::Start,
        border: None,
        border_sides: BorderSides::all(),
        border_style: Style::new(),
        bg_color: None,
        padding: Padding::default(),
        margin: Margin::default(),
        constraints: Constraints::default(),
        title: None,
        grow: 0,
        group_name: None,
    })));
    commands.push(Command::Text {
        content: "x".repeat(width),
        cursor_offset: None,
        style: Style::new(),
        grow: 0,
        align: Align::Start,
        wrap: false,
        truncate: false,
        margin: Default::default(),
        constraints: Default::default(),
    });
    commands.push(Command::EndContainer);
}

/// Open a row, optionally wrapping (with the given cross-axis gap) and/or
/// with a within-line gap. `wrap` pushes a `WrapMarker(cross_gap)` first.
fn open_row_cfg(commands: &mut Vec<Command>, gap: i32, wrap: Option<i32>) {
    if let Some(cross_gap) = wrap {
        commands.push(Command::WrapMarker(cross_gap));
    }
    commands.push(Command::BeginContainer(Box::new(BeginContainerArgs {
        direction: Direction::Row,
        gap,
        align: Align::Start,
        align_self: None,
        justify: Justify::Start,
        border: None,
        border_sides: BorderSides::all(),
        border_style: Style::new(),
        bg_color: None,
        padding: Padding::default(),
        margin: Margin::default(),
        constraints: Constraints::default(),
        title: None,
        grow: 0,
        group_name: None,
    })));
}

#[test]
fn flex_wrap_two_lines_on_overflow() {
    // Three 14-wide children in a 30-wide wrapping row (gap 0): two fit on
    // line 0 (14 + 14 = 28 <= 30), the third overflows to line 1.
    let mut commands: Vec<Command> = Vec::new();
    open_row_cfg(&mut commands, 0, Some(0));
    push_textcol(&mut commands, 14, false, None);
    push_textcol(&mut commands, 14, false, None);
    push_textcol(&mut commands, 14, false, None);
    commands.push(Command::EndContainer);

    let mut tree = build_tree(&mut commands);
    let area = crate::rect::Rect::new(0, 0, 30, 8);
    compute(&mut tree, area);

    let row = &tree.children[0];
    assert!(row.wrap_children);
    assert_eq!(row.children.len(), 3);
    // Line 0: children 0 and 1 share the top line.
    assert_eq!(row.children[0].pos.1, area.y);
    assert_eq!(row.children[1].pos.1, area.y);
    assert_eq!(row.children[0].pos.0, area.x);
    assert_eq!(row.children[1].pos.0, area.x + 14);
    // Line 1: the overflowing child wraps down and its x resets to area.x.
    assert_eq!(row.children[2].pos.1, area.y + 1);
    assert_eq!(row.children[2].pos.0, area.x);
}

#[test]
fn flex_wrap_off_is_single_line() {
    // Same children, no WrapMarker: all on the top line, overflow preserved.
    let mut commands: Vec<Command> = Vec::new();
    open_row_cfg(&mut commands, 0, None);
    push_textcol(&mut commands, 14, false, None);
    push_textcol(&mut commands, 14, false, None);
    push_textcol(&mut commands, 14, false, None);
    commands.push(Command::EndContainer);

    let mut tree = build_tree(&mut commands);
    let area = crate::rect::Rect::new(0, 0, 30, 8);
    compute(&mut tree, area);

    let row = &tree.children[0];
    assert!(!row.wrap_children);
    for child in &row.children {
        assert_eq!(child.pos.1, area.y);
    }
    // Overflow-by-design: monotonic x advance, third child past the right edge.
    assert_eq!(row.children[0].pos.0, area.x);
    assert_eq!(row.children[1].pos.0, area.x + 14);
    assert_eq!(row.children[2].pos.0, area.x + 28);
}

#[test]
fn flex_wrap_row_gap_between_lines() {
    // Cross-axis gap of 2 between lines. Each line is 1 cell tall, so line 1
    // lands at area.y + line0_height (1) + cross_gap (2) = area.y + 3.
    let mut commands: Vec<Command> = Vec::new();
    open_row_cfg(&mut commands, 0, Some(2));
    push_textcol(&mut commands, 14, false, None);
    push_textcol(&mut commands, 14, false, None);
    push_textcol(&mut commands, 14, false, None);
    commands.push(Command::EndContainer);

    let mut tree = build_tree(&mut commands);
    let area = crate::rect::Rect::new(0, 0, 30, 12);
    compute(&mut tree, area);

    let row = &tree.children[0];
    assert_eq!(row.cross_gap, 2);
    assert_eq!(row.children[0].pos.1, area.y);
    assert_eq!(row.children[1].pos.1, area.y);
    assert_eq!(row.children[2].pos.1, area.y + 3);
}

#[test]
fn flex_wrap_oversize_child_own_line() {
    // One child wider than the full area occupies its own line, then a normal
    // child follows on the next line — no empty line, no panic.
    let mut commands: Vec<Command> = Vec::new();
    open_row_cfg(&mut commands, 0, Some(0));
    push_textcol(&mut commands, 50, false, None); // wider than the 30-wide row
    push_textcol(&mut commands, 10, false, None);
    commands.push(Command::EndContainer);

    let mut tree = build_tree(&mut commands);
    let area = crate::rect::Rect::new(0, 0, 30, 8);
    compute(&mut tree, area);

    let row = &tree.children[0];
    assert_eq!(row.children.len(), 2);
    // Oversize child on line 0 (clipped to area width), normal child on line 1.
    assert_eq!(row.children[0].pos.1, area.y);
    assert_eq!(row.children[1].pos.1, area.y + 1);
    assert_eq!(row.children[1].pos.0, area.x);
}

#[test]
fn flex_wrap_min_height_reserves_lines() {
    // A wrapping row inside the (implicit) root column must report a multi-line
    // height so the parent reserves enough rows. Three 14-wide children in a
    // 30-wide row wrap to 2 lines, each 1 cell tall → row height == 2.
    let mut commands: Vec<Command> = Vec::new();
    open_row_cfg(&mut commands, 0, Some(0));
    push_textcol(&mut commands, 14, false, None);
    push_textcol(&mut commands, 14, false, None);
    push_textcol(&mut commands, 14, false, None);
    commands.push(Command::EndContainer);

    let mut tree = build_tree(&mut commands);
    let area = crate::rect::Rect::new(0, 0, 30, 12);
    compute(&mut tree, area);

    let row = &mut tree.children[0];
    // min_height_for_width drives the parent column's reservation.
    assert_eq!(row.min_height_for_width(30), 2);
}

#[test]
fn flex_basis_feeds_grow() {
    // Two children with basis(10) and grow(1) each in a 40-wide row.
    // free = 40 - (10 + 10) = 20, split 10/10 → each grows to 10.
    // (Grow children resolve to their share of flex_space; basis sets the
    // base subtracted from available, so flex_space = 40 - 20 = 20.)
    let mut commands: Vec<Command> = Vec::new();
    open_row_cfg(&mut commands, 0, None);
    // Use grow on the wrapper containers — push them manually with basis+grow.
    for _ in 0..2 {
        commands.push(Command::BasisMarker(10));
        commands.push(Command::BeginContainer(Box::new(BeginContainerArgs {
            direction: Direction::Column,
            gap: 0,
            align: Align::Start,
            align_self: None,
            justify: Justify::Start,
            border: None,
            border_sides: BorderSides::all(),
            border_style: Style::new(),
            bg_color: None,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 1,
            group_name: None,
        })));
        commands.push(Command::Text {
            content: "x".to_string(),
            cursor_offset: None,
            style: Style::new(),
            grow: 0,
            align: Align::Start,
            wrap: false,
            truncate: false,
            margin: Default::default(),
            constraints: Default::default(),
        });
        commands.push(Command::EndContainer);
    }
    commands.push(Command::EndContainer);

    let mut tree = build_tree(&mut commands);
    let area = crate::rect::Rect::new(0, 0, 40, 4);
    compute(&mut tree, area);

    let row = &tree.children[0];
    assert_eq!(row.children[0].flex_basis(), Some(10));
    assert_eq!(row.children[1].flex_basis(), Some(10));
    // Grow children split the full available width 40 → 20 each.
    assert_eq!(row.children[0].size.0, 20);
    assert_eq!(row.children[1].size.0, 20);
}

#[test]
fn flex_basis_feeds_shrink() {
    // Two children, basis(20), both shrink, in a 30-wide row. The narrow text
    // (5 chars) makes min_width 5, but basis(20) replaces it as the shrink
    // base: fixed = 40, scale = min(30,40)/40 = 0.75, each → floor(20*0.75) = 15.
    let mut commands: Vec<Command> = Vec::new();
    open_row_cfg(&mut commands, 0, None);
    push_textcol(&mut commands, 5, true, Some(20));
    push_textcol(&mut commands, 5, true, Some(20));
    commands.push(Command::EndContainer);

    let mut tree = build_tree(&mut commands);
    let area = crate::rect::Rect::new(0, 0, 30, 4);
    compute(&mut tree, area);

    let row = &tree.children[0];
    assert!(row.children[0].shrink);
    assert_eq!(row.children[0].flex_basis(), Some(20));
    assert_eq!(row.children[0].size.0, 15);
    assert_eq!(row.children[1].size.0, 15);
}

#[test]
fn flex_basis_none_falls_back_to_min_width() {
    // No BasisMarker: base size is the child's min_width (20), identical to
    // the pre-#258 path.
    let mut commands: Vec<Command> = Vec::new();
    open_row_cfg(&mut commands, 0, None);
    push_textcol(&mut commands, 20, false, None);
    push_textcol(&mut commands, 20, false, None);
    commands.push(Command::EndContainer);

    let mut tree = build_tree(&mut commands);
    let area = crate::rect::Rect::new(0, 0, 60, 4);
    compute(&mut tree, area);

    let row = &tree.children[0];
    assert_eq!(row.children[0].flex_basis(), None);
    assert_eq!(row.children[0].size.0, 20);
    assert_eq!(row.children[1].size.0, 20);
}

// ---------------------------------------------------------------------------
// Grapheme-cluster segmentation (issue #259)
//
// A break / truncate must never fall inside an extended grapheme cluster.
// Helpers below re-segment the output and assert no output cluster boundary
// was introduced that wasn't already an input cluster boundary.
// ---------------------------------------------------------------------------

/// A regional indicator (U+1F1E6..=U+1F1FF) on its own — i.e. a flag emoji that
/// was split in half. Its presence in a wrapped line is the failure signature.
fn contains_lone_regional_indicator(s: &str) -> bool {
    use unicode_segmentation::UnicodeSegmentation;
    s.graphemes(true).any(|g| {
        let mut chars = g.chars();
        match (chars.next(), chars.next()) {
            // A single regional indicator with no paired partner: a half flag.
            (Some(c), None) => ('\u{1F1E6}'..='\u{1F1FF}').contains(&c),
            _ => false,
        }
    })
}

#[test]
fn wrap_lines_zwj_flag_not_split() {
    // 🇰🇷 and 🇯🇵 are each two regional-indicator scalars forming one cluster.
    // At max_width=2 (one flag is width 2) each flag must stay whole.
    let lines = wrap_lines("🇰🇷🇯🇵", 2);
    for line in &lines {
        assert!(
            !contains_lone_regional_indicator(line),
            "flag split mid-cluster: {line:?}"
        );
    }
    // Re-joined output preserves every flag.
    let joined: String = lines.join("");
    assert!(joined.contains("🇰🇷"));
    assert!(joined.contains("🇯🇵"));
}

#[test]
fn wrap_lines_family_emoji_at_boundary() {
    // Family emoji: 👨‍👩‍👧‍👦 is one cluster joined by ZWJ. Put a width-2 char
    // before it so the family base lands at/over the boundary; the whole
    // cluster must stay intact on one line (no orphaned ZWJ tail).
    let input = "日👨\u{200D}👩\u{200D}👧\u{200D}👦";
    let lines = wrap_lines(input, 2);
    // The family cluster appears whole on exactly one output line.
    let whole = "👨\u{200D}👩\u{200D}👧\u{200D}👦";
    assert!(
        lines.iter().any(|l| l.contains(whole)),
        "family emoji was broken across lines: {lines:?}"
    );
    // No output line carries a dangling ZWJ at an edge of the cluster.
    for line in &lines {
        // A ZWJ must always be flanked by its joined glyphs — re-segmenting
        // must not yield a fragment that starts or ends with a bare ZWJ.
        assert!(
            !line.starts_with('\u{200D}') && !line.ends_with('\u{200D}'),
            "dangling ZWJ in {line:?}"
        );
    }
}

#[test]
fn wrap_lines_devanagari_cluster() {
    // "क्षि" is base + virama + consonant + vowel sign: one (or few) clusters.
    // It must not be broken between a base and its combining marks.
    use unicode_segmentation::UnicodeSegmentation;
    let input = "क्षि";
    let lines = wrap_lines(input, 1);
    let input_clusters: Vec<&str> = input.graphemes(true).collect();
    // Every output line is a whole sequence of input clusters.
    for line in &lines {
        for g in line.graphemes(true) {
            assert!(
                input_clusters.contains(&g),
                "devanagari cluster fragmented: {g:?} not an input cluster"
            );
        }
    }
}

#[test]
fn wrap_lines_thai_cluster() {
    // "กำ" is a Thai consonant + sara am (one cluster). Must stay intact.
    use unicode_segmentation::UnicodeSegmentation;
    let input = "กำ";
    let lines = wrap_lines(input, 1);
    let input_clusters: Vec<&str> = input.graphemes(true).collect();
    for line in &lines {
        for g in line.graphemes(true) {
            assert!(
                input_clusters.contains(&g),
                "thai cluster fragmented: {g:?}"
            );
        }
    }
}

#[test]
fn split_long_word_keeps_clusters() {
    // An over-wide word of concatenated flags chunks only between flags.
    let lines = wrap_lines("🇰🇷🇯🇵🇺🇸", 2);
    for line in &lines {
        assert!(
            !contains_lone_regional_indicator(line),
            "long-word chunk split a flag: {line:?}"
        );
    }
    let joined: String = lines.join("");
    assert!(joined.contains("🇰🇷") && joined.contains("🇯🇵") && joined.contains("🇺🇸"));
}

#[test]
fn wrap_segments_zwj_across_break() {
    // A styled run containing a ZWJ family emoji never splits the cluster
    // across two visual lines.
    let whole = "👨\u{200D}👩\u{200D}👧\u{200D}👦";
    let segs = vec![
        ("日本".to_string(), Style::new()),
        (whole.to_string(), Style::new()),
    ];
    let lines = wrap_segments(&segs, 2);
    // Find the cluster reassembled on a single line.
    let any_whole = lines.iter().any(|line| {
        let joined: String = line.iter().map(|(t, _)| t.as_str()).collect();
        joined.contains(whole)
    });
    assert!(any_whole, "ZWJ cluster split across lines: {lines:?}");
    // No line fragment dangles a ZWJ at an edge.
    for line in &lines {
        for (t, _) in line {
            assert!(
                !t.starts_with('\u{200D}') && !t.ends_with('\u{200D}'),
                "dangling ZWJ in segment {t:?}"
            );
        }
    }
}

#[test]
fn truncate_with_ellipsis_drops_whole_cluster() {
    // "🇰🇷abc" truncated to width 2: the flag is width 2 so it cannot fit
    // alongside the ellipsis (target = 1). It is dropped whole, not halved.
    let out = super::render::truncate_with_ellipsis("🇰🇷abc", 2);
    assert_eq!(out, "\u{2026}");
    assert!(!contains_lone_regional_indicator(&out));
}

#[test]
fn truncate_with_ellipsis_ascii_unchanged() {
    // ASCII regression: identical to scalar behavior.
    assert_eq!(
        super::render::truncate_with_ellipsis("hello", 4),
        "hel\u{2026}"
    );
    assert_eq!(
        super::render::truncate_with_ellipsis("hi", 10),
        "hi\u{2026}"
    );
}

// --- gap_overlap (signed gap) tests (#222) -------------------------------

/// Build a `Direction` container with `gap` (signed) holding `n` fixed-size
/// children, lay it out in `area`, and return the computed tree. Each child
/// is a text node constrained to exactly `child_w` x `child_h` so positions
/// are deterministic regardless of content metrics.
fn gap_overlap_tree(
    direction: Direction,
    gap: i32,
    child_w: u32,
    child_h: u32,
    n: usize,
    area: crate::rect::Rect,
) -> LayoutNode {
    use crate::style::{Align, Constraints, Justify, Margin, Padding};

    let mut commands = vec![Command::BeginContainer(Box::new(BeginContainerArgs {
        direction,
        gap,
        align: Align::Start,
        align_self: None,
        justify: Justify::Start,
        border: None,
        border_sides: BorderSides::all(),
        border_style: crate::style::Style::new(),
        bg_color: None,
        padding: Padding::default(),
        margin: Margin::default(),
        constraints: Constraints::default(),
        title: None,
        grow: 0,
        group_name: None,
    }))];
    for _ in 0..n {
        commands.push(Command::Text {
            content: "x".into(),
            cursor_offset: None,
            style: crate::style::Style::new(),
            grow: 0,
            align: Align::Start,
            wrap: false,
            truncate: false,
            margin: Margin::default(),
            constraints: Constraints::default().w(child_w).h(child_h),
        });
    }
    commands.push(Command::EndContainer);

    let mut tree = build_tree(&mut commands);
    compute(&mut tree, area);
    tree
}

#[test]
fn gap_overlap_1_reduces_total_width() {
    // Row of two w(10) children in a w(25) parent with gap = -1.
    // Total used = 10 + 10 - 1 = 19; child 1 starts at child 0 end - 1.
    let tree = gap_overlap_tree(
        Direction::Row,
        -1,
        10,
        1,
        2,
        crate::rect::Rect::new(0, 0, 25, 4),
    );
    let row = &tree.children[0];
    let c0 = &row.children[0];
    let c1 = &row.children[1];
    assert_eq!(c0.size.0, 10);
    assert_eq!(c1.size.0, 10);
    // Overlap by 1: child 1 starts one column before child 0's end.
    assert_eq!(
        c1.pos.0,
        c0.pos.0 + c0.size.0 - 1,
        "child 1 must overlap child 0 by one column"
    );
    // Total extent from first child start to last child end = 19.
    assert_eq!(c1.pos.0 + c1.size.0 - c0.pos.0, 19);
}

#[test]
fn gap_overlap_zero_is_gap_zero() {
    // gap_overlap(0) (gap = 0) must lay out identically to gap(0).
    let area = crate::rect::Rect::new(0, 0, 40, 4);
    let a = gap_overlap_tree(Direction::Row, 0, 8, 1, 3, area);
    let b = gap_overlap_tree(Direction::Row, 0, 8, 1, 3, area);
    for i in 0..3 {
        assert_eq!(a.children[0].children[i].pos, b.children[0].children[i].pos);
        assert_eq!(
            a.children[0].children[i].size,
            b.children[0].children[i].size
        );
    }
    // And child 1 starts exactly at child 0's end (no overlap, no gap).
    let row = &a.children[0];
    assert_eq!(row.children[1].pos.0, row.children[0].pos.0 + 8);
}

#[test]
fn gap_overlap_large_value_does_not_panic() {
    // gap = -1000 with small children: positions saturate at 0, no panic/wrap.
    let tree = gap_overlap_tree(
        Direction::Row,
        -1000,
        5,
        1,
        3,
        crate::rect::Rect::new(0, 0, 20, 4),
    );
    let row = &tree.children[0];
    assert_eq!(row.children.len(), 3);
    // All children collapse to the same starting column (positions clamped).
    for child in &row.children {
        assert_eq!(child.pos.0, row.children[0].pos.0);
    }
}

#[test]
fn gap_positive_unchanged() {
    // gap = 2 (positive) behaves as before: child 1 starts 2 cols after child 0.
    let tree = gap_overlap_tree(
        Direction::Row,
        2,
        6,
        1,
        2,
        crate::rect::Rect::new(0, 0, 40, 4),
    );
    let row = &tree.children[0];
    let c0 = &row.children[0];
    let c1 = &row.children[1];
    assert_eq!(c1.pos.0, c0.pos.0 + c0.size.0 + 2);
}

#[test]
fn gap_overlap_col_direction() {
    // gap = -1 on a column container overlaps children vertically by one row.
    let tree = gap_overlap_tree(
        Direction::Column,
        -1,
        4,
        3,
        2,
        crate::rect::Rect::new(0, 0, 10, 25),
    );
    let col = &tree.children[0];
    let c0 = &col.children[0];
    let c1 = &col.children[1];
    assert_eq!(c0.size.1, 3);
    assert_eq!(c1.size.1, 3);
    assert_eq!(
        c1.pos.1,
        c0.pos.1 + c0.size.1 - 1,
        "child 1 must overlap child 0 by one row"
    );
}

mod wrap_lines_grapheme_property {
    use super::wrap_lines;
    use proptest::prelude::*;
    use unicode_segmentation::UnicodeSegmentation;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        // For arbitrary strings (newline-free so we test the soft-wrap kernel),
        // every cluster on every output line must be a whole input cluster:
        // no output cluster boundary that wasn't an input boundary.
        #[test]
        fn no_partial_cluster(
            s in "[a-c \\x{1F1E6}-\\x{1F1FF}\\x{0301}\\x{0E33}\\x{0E01}]{0,32}",
            w in 1u32..6,
        ) {
            let input_clusters: std::collections::HashSet<String> =
                s.graphemes(true).map(str::to_string).collect();
            for line in wrap_lines(&s, w) {
                for g in line.graphemes(true) {
                    // Spaces are inserted as word separators by the wrapper, so
                    // a bare space is always a legal output cluster.
                    prop_assert!(
                        g == " " || input_clusters.contains(g),
                        "output cluster {:?} was not a whole input cluster (input {:?}, width {})",
                        g, s, w
                    );
                }
            }
        }
    }
}

// ─── #247: horizontal scroll (scroll_row) layout / collect kernel ──────────

/// Build a scrollable row of fixed-width text children for the #247 tests.
///
/// Each child is a 1-row text node sized `cell_w` wide; `gap` is the
/// inter-child main-axis gap. The container is positioned at the origin and
/// marked `is_scrollable`, mirroring what `build_children` produces for
/// `scroll_row`.
#[cfg(test)]
fn scrollable_row(children_widths: &[u32], gap: i32, viewport_w: u32) -> LayoutNode {
    let mut row = LayoutNode::container(Direction::Row, default_container_config());
    row.is_scrollable = true;
    row.gap = gap;
    for (i, &w) in children_widths.iter().enumerate() {
        let mut t = LayoutNode::text(
            format!("C{i}"),
            Style::new(),
            0,
            Align::Start,
            (None, false, false),
            Margin::default(),
            Constraints::default().min_w(w).max_w(w),
        );
        // Seed the intrinsic width so `min_width` reports `w` regardless of the
        // 2-char label.
        t.size = (w, 1);
        row.children.push(t);
    }
    let mut root = LayoutNode::container(Direction::Column, default_container_config());
    root.children.push(row);
    compute(&mut root, crate::rect::Rect::new(0, 0, viewport_w, 4));
    root
}

#[test]
fn scrollable_row_overflow_sets_content_width_and_lays_out_horizontally() {
    // Four 10-wide children + 3 gaps of 1 = 43 natural width into a 20-wide
    // viewport. The scrollable row must lay out into the oversized virtual
    // area (children march left→right past the viewport) and record
    // `content_width == natural_width`.
    let root = scrollable_row(&[10, 10, 10, 10], 1, 20);
    let row = &root.children[0];
    assert!(row.is_scrollable);
    assert_eq!(
        row.content_width, 43,
        "content_width must equal the natural width (4*10 + 3*1)"
    );
    assert_eq!(
        row.content_height, 0,
        "a scrollable row has no content_height"
    );

    // Children laid out left→right at the same y, spanning past the viewport.
    let c = &row.children;
    assert_eq!(c[0].pos, (0, 0));
    assert_eq!(c[1].pos.0, 11, "second child after first(10)+gap(1)");
    assert_eq!(c[2].pos.0, 22);
    assert_eq!(c[3].pos.0, 33);
    for child in c {
        assert_eq!(child.pos.1, 0, "all children share the same row (y)");
    }
    assert!(
        c[3].pos.0 + c[3].size.0 > 20,
        "the far-right child must overflow the 20-wide viewport"
    );
}

#[test]
fn scrollable_row_fits_reports_content_within_viewport() {
    // 3*5 + 2*1 = 17 ≤ 20: no overflow. Mirroring the scrollable-column path,
    // `content_width` records the real content extent (not forced to 0), and
    // because it is ≤ the viewport width the row reports no scrollable slack:
    // `can_scroll_right` will be false.
    let root = scrollable_row(&[5, 5, 5], 1, 20);
    let row = &root.children[0];
    assert_eq!(
        row.content_width, 17,
        "a fitting scrollable row records its real content width (3*5 + 2*1)"
    );
    let mut s = crate::widgets::ScrollState::new();
    s.set_bounds_x(row.content_width, 20);
    assert!(
        !s.can_scroll_right(),
        "content (17) ≤ viewport (20): no horizontal slack"
    );
}

#[test]
fn scrollable_column_reports_zero_content_width() {
    // Regression guard: a vertical scrollable must keep content_width == 0 and
    // a non-zero content_height, so `ScrollState` never sees phantom
    // horizontal overflow (#247 must not disturb the y-axis).
    let mut root = LayoutNode::container(Direction::Column, default_container_config());
    let mut col = LayoutNode::container(Direction::Column, default_container_config());
    col.is_scrollable = true;
    for i in 0..10 {
        let mut t = LayoutNode::text(
            format!("row {i}"),
            Style::new(),
            0,
            Align::Start,
            (None, false, false),
            Margin::default(),
            Constraints::default(),
        );
        t.size = (6, 1);
        col.children.push(t);
    }
    root.children.push(col);
    compute(&mut root, crate::rect::Rect::new(0, 0, 20, 4));
    let col = &root.children[0];
    assert_eq!(
        col.content_width, 0,
        "vertical scrollable: content_width == 0"
    );
    assert!(
        col.content_height > 0,
        "vertical scrollable: content_height > 0"
    );
}

#[test]
fn collect_all_reports_horizontal_axis_for_scrollable_row() {
    // The collect pass must tag a scrollable row with `is_horizontal = true`
    // and report content/viewport *widths*, so `Context::scrollable` binds the
    // x-axis next frame.
    let root = scrollable_row(&[10, 10, 10, 10], 1, 20);
    let mut fd = FrameData::default();
    collect_all(&root, &mut fd);
    assert_eq!(fd.scroll_infos.len(), 1);
    let (content, viewport, is_horizontal) = fd.scroll_infos[0];
    assert!(is_horizontal, "scrollable row must be tagged horizontal");
    assert_eq!(content, 43, "reports content width");
    assert_eq!(viewport, 20, "reports viewport width");
}

#[test]
fn collect_all_keeps_vertical_axis_flag_false_for_scrollable_column() {
    let mut root = LayoutNode::container(Direction::Column, default_container_config());
    let mut col = LayoutNode::container(Direction::Column, default_container_config());
    col.is_scrollable = true;
    col.pos = (0, 0);
    col.size = (20, 4);
    col.content_height = 30;
    root.children.push(col);
    let mut fd = FrameData::default();
    collect_all(&root, &mut fd);
    let (content, viewport, is_horizontal) = fd.scroll_infos[0];
    assert!(!is_horizontal, "scrollable column stays vertical (false)");
    assert_eq!(content, 30);
    assert_eq!(viewport, 4);
}

#[test]
fn collect_all_shifts_child_x_by_scroll_offset_x() {
    // A scrollable row with `scroll_offset_x = 7` must report its child focus
    // rect shifted left by 7 (the on-screen x), mirroring the y-axis offset.
    let mut root = LayoutNode::container(Direction::Column, default_container_config());
    let mut row = LayoutNode::container(Direction::Row, default_container_config());
    row.is_scrollable = true;
    row.pos = (0, 0);
    row.size = (20, 1);
    row.scroll_offset_x = 7;
    let mut child = LayoutNode::text(
        "focusable".to_string(),
        Style::new(),
        0,
        Align::Start,
        (None, false, false),
        Margin::default(),
        Constraints::default(),
    );
    child.pos = (10, 0);
    child.size = (9, 1);
    child.focus_id = Some(3);
    row.children.push(child);
    root.children.push(row);

    let mut fd = FrameData::default();
    collect_all(&root, &mut fd);
    let (_, rect) = fd
        .focus_rects
        .iter()
        .find(|(id, _)| *id == 3)
        .expect("focus rect collected");
    assert_eq!(
        rect.x, 3,
        "child at layout x=10 with x-offset 7 must collect at screen x=3"
    );
}

#[test]
fn scroll_row_renders_left_to_right_not_stacked() {
    // Regression for the core #247 bug: `scroll_row` must NOT silently build a
    // vertical column. Children must increase in x with a constant y.
    let mut tb = crate::test_utils::TestBackend::new(20, 6);
    let mut scroll = crate::widgets::ScrollState::new();
    tb.render(|ui| {
        let _ = ui.scroll_row(&mut scroll, |ui| {
            for i in 0..6 {
                ui.text(format!("COL{i}"));
            }
        });
    });
    // The leftmost columns are visible on the same single line; a vertical
    // column would have stacked them on separate lines instead.
    let line0 = tb.line(0);
    assert!(
        line0.contains("COL0") && line0.contains("COL1"),
        "scroll_row must lay COL0 and COL1 on the SAME line (got {line0:?})"
    );
    assert_eq!(
        tb.line(1),
        "",
        "a horizontal scroll_row must not stack children onto row 1"
    );
}

#[test]
fn scroll_row_clips_far_right_then_reveals_after_scroll() {
    // Wide content into a narrow viewport: a far-right column is clipped until
    // we scroll right, then it appears and the leftmost clips out.
    let mut tb = crate::test_utils::TestBackend::new(16, 4);
    let mut scroll = crate::widgets::ScrollState::new();

    // Frame 1 records the scroll bounds; frame 2's `scrollable()` reads them
    // back via `prev_scroll_infos` and calls `set_bounds_x`, so after frame 2
    // `scroll` knows its content/viewport widths and `scroll_right` can clamp
    // correctly. (Same two-frame bounds lifecycle as vertical scroll.)
    tb.render(|ui| {
        let _ = ui.scroll_row(&mut scroll, |ui| {
            for i in 0..8 {
                ui.text(format!("[item{i:02}]"));
            }
        });
    });
    tb.render(|ui| {
        let _ = ui.scroll_row(&mut scroll, |ui| {
            for i in 0..8 {
                ui.text(format!("[item{i:02}]"));
            }
        });
    });
    tb.assert_contains("[item00]");
    tb.assert_not_contains("[item07]");

    // Now bounds are known: scroll right far enough to reveal the last item.
    scroll.scroll_right(60);
    tb.render(|ui| {
        let _ = ui.scroll_row(&mut scroll, |ui| {
            for i in 0..8 {
                ui.text(format!("[item{i:02}]"));
            }
        });
    });
    tb.assert_contains("[item07]");
    tb.assert_not_contains("[item00]");
}

#[test]
fn scroll_state_horizontal_bounds_clamp_and_predicates() {
    // Drive the bounds/predicate surface directly (no rendering): the x-axis
    // mirror of the vertical clamp/can_scroll tests.
    let mut s = crate::widgets::ScrollState::new();
    s.set_bounds_x(100, 20); // content 100 wide, viewport 20 → max offset 80.

    assert!(!s.can_scroll_left(), "at start, cannot scroll left");
    assert!(s.can_scroll_right(), "content overflows, can scroll right");

    s.scroll_right(1000); // way past the end
    assert_eq!(s.offset_x, 80, "scroll_right clamps to content - viewport");
    assert!(!s.can_scroll_right(), "at end, cannot scroll right");
    assert!(s.can_scroll_left(), "at end, can scroll left");

    s.scroll_left(1000); // way past the start
    assert_eq!(s.offset_x, 0, "scroll_left clamps to 0");
    assert!((s.progress_x() - 0.0).abs() < f64::EPSILON);

    s.scroll_right(40);
    assert!((s.progress_x() - 0.5).abs() < 1e-9, "40/80 == 0.5 progress");
}

#[test]
fn scroll_state_vertical_api_unchanged_by_horizontal_addition() {
    // The vertical API must remain byte-identical: a column scrollable updates
    // `offset` only, never `offset_x`.
    let mut s = crate::widgets::ScrollState::new();
    s.set_bounds(50, 10);
    s.scroll_down(5);
    assert_eq!(s.offset, 5);
    assert_eq!(s.offset_x, 0, "vertical scroll must not touch offset_x");
    assert_eq!(s.content_width(), 0);
    assert_eq!(s.viewport_width(), 0);
}

mod hscroll_proptest {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(128))]
        // Invariant 1: for any child widths and viewport, after layout the
        // scrollable row's `content_width` equals the natural width (sum of
        // child widths + gaps) — the x-axis mirror of the column path, which
        // records the real content extent regardless of overflow.
        //
        // Invariant 2: `ScrollState::scroll_right(offset)` always clamps
        // `offset_x` into `[0, content_width - viewport_width]`, so the
        // rendered viewport never scrolls past the content edge.
        #[test]
        fn content_width_and_offset_clamp(
            widths in prop::collection::vec(1u32..12, 1..8),
            viewport_w in 4u32..30,
            requested_offset in 0usize..200,
        ) {
            let root = scrollable_row(&widths, 1, viewport_w);
            let row = &root.children[0];

            let gap: i64 = 1;
            let gaps = (widths.len() as i64 - 1).max(0) * gap;
            let natural: i64 = widths.iter().map(|&w| w as i64).sum::<i64>() + gaps;
            prop_assert_eq!(row.content_width as i64, natural,
                "scrollable row records the natural content width");

            // Offset clamp invariant via collect (the binding path).
            let mut fd = FrameData::default();
            collect_all(&root, &mut fd);
            let (content, viewport, is_horizontal) = fd.scroll_infos[0];
            prop_assert!(is_horizontal);

            let mut s = crate::widgets::ScrollState::new();
            s.set_bounds_x(content, viewport);
            s.scroll_right(requested_offset);
            let max = content.saturating_sub(viewport) as usize;
            prop_assert!(s.offset_x <= max,
                "offset_x {} must clamp to max {}", s.offset_x, max);
            // No leak past the clip: every visible cell's screen x is within
            // [0, viewport). A child at layout x `cx` renders at `cx - offset_x`;
            // require the visible window to stay within bounds.
            prop_assert!(s.offset_x as u32 <= content,
                "offset_x must never exceed total content width");
        }
    }
}

// ---------------------------------------------------------------------------
// Per-frame intrinsic-size memo (v0.22.0): the memoization of `min_width` /
// `min_height` / `min_height_for_width` must stay byte-identical to the
// uncached computation. The subtle case is a `Pct`/`Ratio` constraint: such a
// node is measured by its grandparent while still unresolved, then resolved
// (`Pct` -> `Fixed`) by its parent's layout pass. The cache must be discarded
// at the resolution point so the post-resolution query reflects the resolved
// width, not the stale pre-resolution value.
// ---------------------------------------------------------------------------

/// Build a `Direction::Row` container holding `children`.
fn row_with(children: Vec<LayoutNode>) -> LayoutNode {
    let mut n = LayoutNode::container(Direction::Row, default_container_config());
    n.children = children;
    n
}

/// Build a `Direction::Column` container holding `children`.
fn col_with(children: Vec<LayoutNode>) -> LayoutNode {
    let mut n = LayoutNode::container(Direction::Column, default_container_config());
    n.children = children;
    n
}

#[test]
fn pct_width_child_resolves_after_being_measured_unresolved() {
    // Tree: col > row > [ pct-50 col > text , fixed text ]
    // The inner pct column is measured by the outer row/col chain while its
    // `Pct(50)` width is still unresolved (min_width returns None for Pct),
    // then the row resolves it to Fixed(width/2). The memo must not pin the
    // pre-resolution value.
    let leaf = LayoutNode::text(
        "x".to_string(),
        Style::new(),
        0,
        Align::Start,
        (None, false, false),
        Margin::default(),
        Constraints::default(),
    );
    let mut pct_col = col_with(vec![leaf]);
    pct_col.constraints = Constraints::default().w_pct(50);

    let fixed_text = LayoutNode::text(
        "y".to_string(),
        Style::new(),
        0,
        Align::Start,
        (None, false, false),
        Margin::default(),
        Constraints::default(),
    );

    let mut root = col_with(vec![row_with(vec![pct_col, fixed_text])]);
    let area = crate::rect::Rect::new(0, 0, 40, 10);
    compute(&mut root, area);

    // The pct child must occupy exactly 50% of the 40-wide area = 20 cells,
    // identical to the uncached path. A stale `min_width` (cached as the
    // unresolved Pct min of 0) would have collapsed it.
    let row = &root.children[0];
    assert_eq!(
        row.children[0].constraints.width,
        crate::style::WidthSpec::Fixed(20),
        "pct constraint must resolve to Fixed(20)"
    );
    assert_eq!(
        row.children[0].size.0, 20,
        "pct child must be sized to its resolved 50% width, not a stale memo"
    );
}

#[test]
fn min_width_memo_matches_uncached_after_resolution() {
    // Direct check that the cached `min_width` equals a freshly-resolved
    // computation. Build a node, force a cold `min_width` while its Pct is
    // unresolved, then resolve + invalidate and confirm the value updates.
    let mut node = LayoutNode::container(Direction::Column, default_container_config());
    node.constraints = Constraints::default().w_pct(50);
    node.children.push(LayoutNode::text(
        "abcd".to_string(),
        Style::new(),
        0,
        Align::Start,
        (None, false, false),
        Margin::default(),
        Constraints::default(),
    ));

    // Cold query with Pct unresolved: Pct contributes no min, so the min_width
    // is driven by the child text width (4).
    let unresolved = node.min_width();
    assert_eq!(unresolved, 4);
    // A second call hits the memo and returns the same value.
    assert_eq!(node.min_width(), 4);

    // Resolve the Pct against a 40-wide area -> Fixed(20), then invalidate as
    // the flex loop does. The memo must now reflect the resolved min (20).
    super::flexbox::resolve_axis_specs(&mut node.constraints, crate::rect::Rect::new(0, 0, 40, 4));
    node.invalidate_size_cache();
    assert_eq!(
        node.min_width(),
        20,
        "after Pct resolves to Fixed(20), min_width must update, not return the stale cached 4"
    );
}

#[test]
fn nested_viewports_clip_all_feedback_and_raw_crop_in_two_dimensions() {
    use std::sync::Arc;

    let mut root = LayoutNode::container(Direction::Column, default_container_config());
    root.size = (30, 20);

    let mut outer = LayoutNode::container(Direction::Column, default_container_config());
    outer.pos = (2, 2);
    outer.size = (10, 6);
    outer.is_scrollable = true;
    outer.scroll_offset_x = 3;
    outer.scroll_offset = 2;

    let mut nested = LayoutNode::container(Direction::Column, default_container_config());
    nested.pos = (6, 5);
    nested.size = (10, 5);
    nested.is_scrollable = true;
    nested.scroll_offset_x = 2;
    nested.scroll_offset = 1;

    let mut raw = LayoutNode::raw_draw(9, Constraints::default(), 0, Margin::default(), None, None);
    raw.pos = (6, 4);
    raw.size = (14, 8);
    nested.children.push(raw);

    let mut visible = LayoutNode::container(Direction::Column, default_container_config());
    visible.pos = (6, 4);
    visible.size = (14, 8);
    visible.interaction_id = Some(4);
    visible.focus_id = Some(4);
    visible.group_name = Some(Arc::from("visible"));
    nested.children.push(visible);

    let mut hidden = LayoutNode::container(Direction::Column, default_container_config());
    hidden.pos = (0, 0);
    hidden.size = (2, 2);
    hidden.interaction_id = Some(5);
    hidden.focus_id = Some(5);
    hidden.group_name = Some(Arc::from("hidden"));
    nested.children.push(hidden);

    outer.children.push(nested);
    root.children.push(outer);

    let mut data = FrameData::default();
    collect_all(&root, &mut data);

    let raw = &data.raw_draw_rects[0];
    assert_eq!(raw.rect, crate::rect::Rect::new(3, 3, 9, 5));
    assert_eq!(raw.left_clip_cols, 2);
    assert_eq!(raw.top_clip_rows, 2);
    assert_eq!((raw.original_width, raw.original_height), (14, 8));
    assert_eq!(data.hit_areas[4], crate::rect::Rect::new(3, 3, 9, 5));
    assert!(data.hit_areas[5].is_empty());
    assert!(data.focus_rects.iter().any(|(id, _)| *id == 4));
    assert!(!data.focus_rects.iter().any(|(id, _)| *id == 5));
    assert!(
        data.group_rects
            .iter()
            .any(|(name, _)| name.as_ref() == "visible")
    );
    assert!(
        !data
            .group_rects
            .iter()
            .any(|(name, _)| name.as_ref() == "hidden")
    );
    assert!(
        data.content_areas.iter().any(|(full, content)| *full
            == crate::rect::Rect::new(3, 3, 9, 5)
            && *content == *full)
    );
}

#[test]
fn partially_clipped_container_applies_padding_before_viewport_clip() {
    let mut root = LayoutNode::container(Direction::Row, default_container_config());
    root.pos = (0, 0);
    root.size = (5, 1);
    root.is_scrollable = true;
    root.scroll_offset_x = 2;

    let mut child = LayoutNode::container(Direction::Row, default_container_config());
    child.pos = (0, 0);
    child.size = (8, 1);
    child.padding.left = 2;
    let mut text = LayoutNode::text(
        "A".to_string(),
        Style::new(),
        0,
        Align::Start,
        (None, false, false),
        Margin::default(),
        Constraints::default(),
    );
    text.pos = (3, 0);
    text.size = (1, 1);
    child.children.push(text);
    root.children.push(child);

    let mut buffer = crate::buffer::Buffer::empty(crate::rect::Rect::new(0, 0, 5, 1));
    super::render::render(&root, &mut buffer);
    assert_eq!(buffer.get(1, 0).symbol, "A");
}

#[test]
fn frame_data_swaps_feedback_capacity_back_for_reuse() {
    let mut data = FrameData::default();
    let mut feedback = crate::LayoutFeedbackState::default();
    feedback.prev_hit_map = Vec::with_capacity(64);
    feedback
        .prev_hit_map
        .push(crate::rect::Rect::new(0, 0, 1, 1));

    data.swap_feedback(&mut feedback);
    assert!(data.hit_areas.capacity() >= 64);
    data.clear();
    data.hit_areas.push(crate::rect::Rect::new(1, 1, 2, 2));
    data.swap_feedback(&mut feedback);
    assert_eq!(feedback.prev_hit_map, [crate::rect::Rect::new(1, 1, 2, 2)]);

    data.swap_feedback(&mut feedback);
    assert!(data.hit_areas.capacity() >= 64);
}

#[test]
fn intrinsic_and_flex_arithmetic_saturates_extreme_constraints() {
    let extreme = Constraints::default().w(u32::MAX).h(u32::MAX);
    let child = LayoutNode::text(
        "x".to_string(),
        Style::new(),
        0,
        Align::Start,
        (None, false, false),
        Margin::new(u32::MAX, u32::MAX, u32::MAX, u32::MAX),
        extreme,
    );
    let mut row = LayoutNode::container(Direction::Row, default_container_config());
    row.padding = Padding::new(u32::MAX, u32::MAX, u32::MAX, u32::MAX);
    row.children = vec![child.clone(), child];

    assert_eq!(row.min_width(), u32::MAX);
    assert_eq!(row.min_height(), u32::MAX);
    compute(&mut row, crate::rect::Rect::new(0, 0, 80, 24));
    assert!(
        row.children
            .iter()
            .all(|child| child.pos.0 == u32::MAX && child.size.0 == u32::MAX)
    );
}
