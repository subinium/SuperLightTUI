#![allow(clippy::print_stderr)]

use super::collect::collect_raw_draw_rects;
use super::tree::{default_container_config, ContainerConfig};
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

    let commands = vec![
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

    let mut tree = build_tree(commands);
    let area = crate::rect::Rect::new(0, 0, 40, 10);
    compute(&mut tree, area);

    let fd = collect_all(&tree);
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

    let commands = vec![
        Command::FocusMarker(0),
        Command::BeginContainer {
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
        },
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

    let mut tree = build_tree(commands);
    let area = crate::rect::Rect::new(0, 0, 40, 10);
    compute(&mut tree, area);

    let fd = collect_all(&tree);
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
    let first_ptr = node.cached_wrapped.as_ref().map(Vec::as_ptr).unwrap();
    let height_b = node.min_height_for_width(6);
    let second_ptr = node.cached_wrapped.as_ref().map(Vec::as_ptr).unwrap();

    assert_eq!(height_a, height_b);
    assert_eq!(first_ptr, second_ptr);
    assert_eq!(node.cached_wrap_width, Some(6));
}

#[test]
fn collect_all_matches_raw_draw_collection() {
    let mut root = LayoutNode::container(Direction::Column, default_container_config());
    let mut scroll = LayoutNode::container(Direction::Column, default_container_config());
    scroll.is_scrollable = true;
    scroll.pos = (0, 0);
    scroll.size = (20, 4);
    scroll.scroll_offset = 2;
    scroll.children.push(LayoutNode {
        kind: NodeKind::RawDraw(7),
        content: None,
        cursor_offset: None,
        style: Style::new(),
        grow: 0,
        align: Align::Start,
        align_self: None,
        justify: Justify::Start,
        wrap: false,
        truncate: false,
        gap: 0,
        border: None,
        border_sides: BorderSides::all(),
        border_style: Style::new(),
        bg_color: None,
        padding: Padding::default(),
        margin: Margin::default(),
        constraints: Constraints::default(),
        title: None,
        children: Vec::new(),
        pos: (1, 3),
        size: (6, 3),
        is_scrollable: false,
        scroll_offset: 0,
        content_height: 0,
        cached_wrap_width: None,
        cached_wrapped: None,
        segments: None,
        cached_wrapped_segments: None,
        focus_id: None,
        interaction_id: None,
        link_url: None,
        group_name: None,
        overlays: Vec::new(),
    });
    root.children.push(scroll);

    let via_collect_all = collect_all(&root)
        .raw_draw_rects
        .into_iter()
        .map(|r| (r.draw_id, r.rect, r.top_clip_rows, r.original_height))
        .collect::<Vec<_>>();
    let via_legacy = collect_raw_draw_rects(&root)
        .into_iter()
        .map(|r| (r.draw_id, r.rect, r.top_clip_rows, r.original_height))
        .collect::<Vec<_>>();

    assert_eq!(via_collect_all, via_legacy);
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
        commands.push(Command::BeginContainer {
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
            group_name: Some(format!("group-{i}")),
        });
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

    let mut tree = build_tree(commands);
    let area = crate::rect::Rect::new(0, 0, 80, (N_GROUPS * FOCUSES_PER_GROUP) as u32 + 4);
    compute(&mut tree, area);
    let fd = collect_all(&tree);

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
