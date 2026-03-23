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
    use crate::style::Style;

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
