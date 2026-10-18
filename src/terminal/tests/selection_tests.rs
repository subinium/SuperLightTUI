//! Text selection state and overlay tests.

use super::*;

fn pair(r: Rect) -> (Rect, Rect) {
    (r, r)
}

#[test]
fn find_innermost_rect_picks_smallest() {
    let rects = vec![
        pair(Rect::new(0, 0, 80, 24)),
        pair(Rect::new(5, 2, 30, 10)),
        pair(Rect::new(10, 4, 10, 5)),
    ];
    let result = find_innermost_rect(&rects, 12, 5);
    assert_eq!(result, Some(Rect::new(10, 4, 10, 5)));
}

#[test]
fn find_innermost_rect_no_match() {
    let rects = vec![pair(Rect::new(10, 10, 5, 5))];
    assert_eq!(find_innermost_rect(&rects, 0, 0), None);
}

#[test]
fn find_innermost_rect_empty() {
    assert_eq!(find_innermost_rect(&[], 5, 5), None);
}

#[test]
fn find_innermost_rect_returns_content_rect() {
    let rects = vec![
        (Rect::new(0, 0, 80, 24), Rect::new(1, 1, 78, 22)),
        (Rect::new(5, 2, 30, 10), Rect::new(6, 3, 28, 8)),
    ];
    let result = find_innermost_rect(&rects, 10, 5);
    assert_eq!(result, Some(Rect::new(6, 3, 28, 8)));
}

#[test]
fn normalize_selection_already_ordered() {
    let (s, e) = normalize_selection((2, 1), (5, 3));
    assert_eq!(s, (2, 1));
    assert_eq!(e, (5, 3));
}

#[test]
fn normalize_selection_reversed() {
    let (s, e) = normalize_selection((5, 3), (2, 1));
    assert_eq!(s, (2, 1));
    assert_eq!(e, (5, 3));
}

#[test]
fn normalize_selection_same_row() {
    let (s, e) = normalize_selection((10, 5), (3, 5));
    assert_eq!(s, (3, 5));
    assert_eq!(e, (10, 5));
}

#[test]
fn selection_state_mouse_down_finds_rect() {
    let hit_map = vec![pair(Rect::new(0, 0, 80, 24)), pair(Rect::new(5, 2, 20, 10))];
    let mut sel = SelectionState::default();
    sel.mouse_down(10, 5, &hit_map);
    assert_eq!(sel.anchor, Some((10, 5)));
    assert_eq!(sel.current, Some((10, 5)));
    assert_eq!(sel.widget_rect, Some(Rect::new(5, 2, 20, 10)));
    assert!(!sel.active);
}

#[test]
fn selection_state_drag_activates() {
    let hit_map = vec![pair(Rect::new(0, 0, 80, 24))];
    let mut sel = SelectionState {
        anchor: Some((10, 5)),
        current: Some((10, 5)),
        widget_rect: Some(Rect::new(0, 0, 80, 24)),
        ..Default::default()
    };
    sel.mouse_drag(10, 5, &hit_map);
    assert!(!sel.active, "no movement = not active");
    sel.mouse_drag(11, 5, &hit_map);
    assert!(!sel.active, "1 cell horizontal = not active yet");
    sel.mouse_drag(13, 5, &hit_map);
    assert!(sel.active, ">1 cell horizontal = active");
}

#[test]
fn selection_state_drag_vertical_activates() {
    let hit_map = vec![pair(Rect::new(0, 0, 80, 24))];
    let mut sel = SelectionState {
        anchor: Some((10, 5)),
        current: Some((10, 5)),
        widget_rect: Some(Rect::new(0, 0, 80, 24)),
        ..Default::default()
    };
    sel.mouse_drag(10, 6, &hit_map);
    assert!(sel.active, "any vertical movement = active");
}

#[test]
fn selection_state_drag_expands_widget_rect() {
    let hit_map = vec![
        pair(Rect::new(0, 0, 80, 24)),
        pair(Rect::new(5, 2, 30, 10)),
        pair(Rect::new(5, 2, 30, 3)),
    ];
    let mut sel = SelectionState {
        anchor: Some((10, 3)),
        current: Some((10, 3)),
        widget_rect: Some(Rect::new(5, 2, 30, 3)),
        ..Default::default()
    };
    sel.mouse_drag(10, 6, &hit_map);
    assert_eq!(sel.widget_rect, Some(Rect::new(5, 2, 30, 10)));
}

#[test]
fn selection_state_clear_resets() {
    let mut sel = SelectionState {
        anchor: Some((1, 2)),
        current: Some((3, 4)),
        widget_rect: Some(Rect::new(0, 0, 10, 10)),
        active: true,
        ..SelectionState::default()
    };
    sel.clear();
    assert_eq!(sel.anchor, None);
    assert_eq!(sel.current, None);
    assert_eq!(sel.widget_rect, None);
    assert!(!sel.active);
}

#[test]
fn extract_selection_text_single_line() {
    let area = Rect::new(0, 0, 20, 5);
    let mut buf = Buffer::empty(area);
    buf.set_string(0, 0, "Hello World", Style::default());
    let sel = SelectionState {
        anchor: Some((0, 0)),
        current: Some((4, 0)),
        widget_rect: Some(area),
        active: true,
        ..SelectionState::default()
    };
    let text = extract_selection_text(&buf, &sel, &[]);
    assert_eq!(text, "Hello");
}

#[test]
fn extract_selection_text_multi_line() {
    let area = Rect::new(0, 0, 20, 5);
    let mut buf = Buffer::empty(area);
    buf.set_string(0, 0, "Line one", Style::default());
    buf.set_string(0, 1, "Line two", Style::default());
    buf.set_string(0, 2, "Line three", Style::default());
    let sel = SelectionState {
        anchor: Some((5, 0)),
        current: Some((3, 2)),
        widget_rect: Some(area),
        active: true,
        ..SelectionState::default()
    };
    let text = extract_selection_text(&buf, &sel, &[]);
    assert_eq!(text, "one\nLine two\nLine");
}

#[test]
fn extract_selection_text_clamped_to_widget() {
    let area = Rect::new(0, 0, 40, 10);
    let widget = Rect::new(5, 2, 10, 3);
    let mut buf = Buffer::empty(area);
    buf.set_string(5, 2, "ABCDEFGHIJ", Style::default());
    buf.set_string(5, 3, "KLMNOPQRST", Style::default());
    let sel = SelectionState {
        anchor: Some((3, 1)),
        current: Some((20, 5)),
        widget_rect: Some(widget),
        active: true,
        ..SelectionState::default()
    };
    let text = extract_selection_text(&buf, &sel, &[]);
    assert_eq!(text, "ABCDEFGHIJ\nKLMNOPQRST");
}

#[test]
fn extract_selection_text_inactive_returns_empty() {
    let area = Rect::new(0, 0, 10, 5);
    let buf = Buffer::empty(area);
    let sel = SelectionState {
        anchor: Some((0, 0)),
        current: Some((5, 2)),
        widget_rect: Some(area),
        active: false,
        ..SelectionState::default()
    };
    assert_eq!(extract_selection_text(&buf, &sel, &[]), "");
}

#[test]
fn apply_selection_overlay_reverses_cells() {
    let area = Rect::new(0, 0, 10, 3);
    let mut buf = Buffer::empty(area);
    buf.set_string(0, 0, "ABCDE", Style::default());
    let sel = SelectionState {
        anchor: Some((1, 0)),
        current: Some((3, 0)),
        widget_rect: Some(area),
        active: true,
        ..SelectionState::default()
    };
    apply_selection_overlay(&mut buf, &sel, &[]);
    assert!(!buf.get(0, 0).style.modifiers.contains(Modifiers::REVERSED));
    assert!(buf.get(1, 0).style.modifiers.contains(Modifiers::REVERSED));
    assert!(buf.get(2, 0).style.modifiers.contains(Modifiers::REVERSED));
    assert!(buf.get(3, 0).style.modifiers.contains(Modifiers::REVERSED));
    assert!(!buf.get(4, 0).style.modifiers.contains(Modifiers::REVERSED));
}

#[test]
fn extract_selection_text_skips_border_cells() {
    // Simulate two bordered columns side by side:
    // Col1: full=(0,0,20,5) content=(1,1,18,3)
    // Col2: full=(20,0,20,5) content=(21,1,18,3)
    // Parent widget_rect covers both: (0,0,40,5)
    let area = Rect::new(0, 0, 40, 5);
    let mut buf = Buffer::empty(area);
    // Col1 border characters
    buf.set_string(0, 0, "╭", Style::default());
    buf.set_string(0, 1, "│", Style::default());
    buf.set_string(0, 2, "│", Style::default());
    buf.set_string(0, 3, "│", Style::default());
    buf.set_string(0, 4, "╰", Style::default());
    buf.set_string(19, 0, "╮", Style::default());
    buf.set_string(19, 1, "│", Style::default());
    buf.set_string(19, 2, "│", Style::default());
    buf.set_string(19, 3, "│", Style::default());
    buf.set_string(19, 4, "╯", Style::default());
    // Col2 border characters
    buf.set_string(20, 0, "╭", Style::default());
    buf.set_string(20, 1, "│", Style::default());
    buf.set_string(20, 2, "│", Style::default());
    buf.set_string(20, 3, "│", Style::default());
    buf.set_string(20, 4, "╰", Style::default());
    buf.set_string(39, 0, "╮", Style::default());
    buf.set_string(39, 1, "│", Style::default());
    buf.set_string(39, 2, "│", Style::default());
    buf.set_string(39, 3, "│", Style::default());
    buf.set_string(39, 4, "╯", Style::default());
    // Content inside Col1
    buf.set_string(1, 1, "Hello Col1", Style::default());
    buf.set_string(1, 2, "Line2 Col1", Style::default());
    // Content inside Col2
    buf.set_string(21, 1, "Hello Col2", Style::default());
    buf.set_string(21, 2, "Line2 Col2", Style::default());

    let content_map = vec![
        (Rect::new(0, 0, 20, 5), Rect::new(1, 1, 18, 3)),
        (Rect::new(20, 0, 20, 5), Rect::new(21, 1, 18, 3)),
    ];

    // Select across both columns, rows 1-2
    let sel = SelectionState {
        anchor: Some((0, 1)),
        current: Some((39, 2)),
        widget_rect: Some(area),
        active: true,
        ..SelectionState::default()
    };
    let text = extract_selection_text(&buf, &sel, &content_map);
    // Should NOT contain border characters (│, ╭, ╮, etc.)
    assert!(!text.contains('│'), "Border char │ found in: {text}");
    assert!(!text.contains('╭'), "Border char ╭ found in: {text}");
    assert!(!text.contains('╮'), "Border char ╮ found in: {text}");
    // Should contain actual content
    assert!(
        text.contains("Hello Col1"),
        "Missing Col1 content in: {text}"
    );
    assert!(
        text.contains("Hello Col2"),
        "Missing Col2 content in: {text}"
    );
    assert!(text.contains("Line2 Col1"), "Missing Col1 line2 in: {text}");
    assert!(text.contains("Line2 Col2"), "Missing Col2 line2 in: {text}");
}

#[test]
fn apply_selection_overlay_skips_border_cells() {
    let area = Rect::new(0, 0, 20, 3);
    let mut buf = Buffer::empty(area);
    buf.set_string(0, 0, "│", Style::default());
    buf.set_string(1, 0, "ABC", Style::default());
    buf.set_string(19, 0, "│", Style::default());

    let content_map = vec![(Rect::new(0, 0, 20, 3), Rect::new(1, 0, 18, 3))];
    let sel = SelectionState {
        anchor: Some((0, 0)),
        current: Some((19, 0)),
        widget_rect: Some(area),
        active: true,
        ..SelectionState::default()
    };
    apply_selection_overlay(&mut buf, &sel, &content_map);
    // Border cells at x=0 and x=19 should NOT be reversed
    assert!(
        !buf.get(0, 0).style.modifiers.contains(Modifiers::REVERSED),
        "Left border cell should not be reversed"
    );
    assert!(
        !buf.get(19, 0).style.modifiers.contains(Modifiers::REVERSED),
        "Right border cell should not be reversed"
    );
    // Content cells should be reversed
    assert!(buf.get(1, 0).style.modifiers.contains(Modifiers::REVERSED));
    assert!(buf.get(2, 0).style.modifiers.contains(Modifiers::REVERSED));
    assert!(buf.get(3, 0).style.modifiers.contains(Modifiers::REVERSED));
}
