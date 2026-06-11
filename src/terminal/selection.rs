use super::*;

#[derive(Default)]
pub(crate) struct SelectionState {
    pub anchor: Option<(u32, u32)>,
    pub current: Option<(u32, u32)>,
    pub widget_rect: Option<Rect>,
    pub active: bool,
}

impl SelectionState {
    /// Record a left-mouse-down anchor point and identify the widget
    /// rect under the cursor; resets the active flag so a single click
    /// without drag clears any prior selection.
    pub(crate) fn mouse_down(&mut self, x: u32, y: u32, hit_map: &[(Rect, Rect)]) {
        self.anchor = Some((x, y));
        self.current = Some((x, y));
        self.widget_rect = find_innermost_rect(hit_map, x, y);
        self.active = false;
    }

    /// Update the current cursor position for an in-progress selection.
    /// Marks the selection active once the cursor has moved more than one
    /// cell horizontally or any cell vertically. Re-resolves the owning
    /// widget rect if the cursor has left the original widget.
    pub(crate) fn mouse_drag(&mut self, x: u32, y: u32, hit_map: &[(Rect, Rect)]) {
        if let Some(anchor) = self.anchor {
            self.current = Some((x, y));
            if x.abs_diff(anchor.0) > 1 || y.abs_diff(anchor.1) > 0 {
                self.active = true;
            }
            if let Some(rect) = self.widget_rect
                && (y < rect.y || y >= rect.bottom() || x < rect.x || x >= rect.right())
            {
                self.widget_rect = find_containing_rect(hit_map, anchor, (x, y));
            }
        }
    }

    /// Reset the selection back to the empty default state.
    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }
}

pub(crate) fn find_containing_rect(
    hit_map: &[(Rect, Rect)],
    a: (u32, u32),
    b: (u32, u32),
) -> Option<Rect> {
    hit_map
        .iter()
        .filter(|(full, _)| {
            a.0 >= full.x
                && a.0 < full.right()
                && a.1 >= full.y
                && a.1 < full.bottom()
                && b.0 >= full.x
                && b.0 < full.right()
                && b.1 >= full.y
                && b.1 < full.bottom()
        })
        .min_by_key(|(full, _)| (full.width as u64) * (full.height as u64))
        .map(|(_, content)| *content)
}

pub(crate) fn find_innermost_rect(hit_map: &[(Rect, Rect)], x: u32, y: u32) -> Option<Rect> {
    hit_map
        .iter()
        .filter(|(full, _)| x >= full.x && x < full.right() && y >= full.y && y < full.bottom())
        .min_by_key(|(full, _)| (full.width as u64) * (full.height as u64))
        .map(|(_, content)| *content)
}

pub(crate) fn is_border_cell(x: u32, y: u32, content_map: &[(Rect, Rect)]) -> bool {
    for &(full, content) in content_map {
        if x >= full.x
            && x < full.right()
            && y >= full.y
            && y < full.bottom()
            && !(x >= content.x && x < content.right() && y >= content.y && y < content.bottom())
        {
            return true;
        }
    }
    false
}

pub(crate) fn normalize_selection(
    anchor: (u32, u32),
    current: (u32, u32),
) -> ((u32, u32), (u32, u32)) {
    if (anchor.1, anchor.0) <= (current.1, current.0) {
        (anchor, current)
    } else {
        (current, anchor)
    }
}

pub(crate) fn apply_selection_overlay(
    buffer: &mut Buffer,
    sel: &SelectionState,
    content_map: &[(Rect, Rect)],
) {
    if !sel.active {
        return;
    }
    let (Some(anchor), Some(current), Some(rect)) = (sel.anchor, sel.current, sel.widget_rect)
    else {
        return;
    };
    let (start, end) = normalize_selection(anchor, current);

    for y in rect.y..rect.bottom() {
        if y < start.1 || y > end.1 {
            continue;
        }
        for x in rect.x..rect.right() {
            if is_border_cell(x, y, content_map) {
                continue;
            }
            let in_sel = if start.1 == end.1 {
                y == start.1 && x >= start.0 && x <= end.0
            } else if y == start.1 {
                x >= start.0
            } else if y == end.1 {
                x <= end.0
            } else {
                true
            };
            if in_sel && buffer.in_bounds(x, y) {
                let cell = buffer.get_mut(x, y);
                cell.style.modifiers |= Modifiers::REVERSED;
            }
        }
    }
}

pub(crate) fn extract_selection_text(
    buffer: &Buffer,
    sel: &SelectionState,
    content_map: &[(Rect, Rect)],
) -> String {
    if !sel.active {
        return String::new();
    }
    let (Some(anchor), Some(current), Some(rect)) = (sel.anchor, sel.current, sel.widget_rect)
    else {
        return String::new();
    };
    let (start, end) = normalize_selection(anchor, current);
    let y_lo = start.1.max(rect.y);
    let y_hi = end.1.min(rect.bottom().saturating_sub(1));

    let mut lines: Vec<String> = Vec::new();
    for y in y_lo..=y_hi {
        let mut line = String::new();
        let x_lo = if y == start.1 {
            start.0.max(rect.x)
        } else {
            rect.x
        };
        let x_hi = if y == end.1 {
            end.0.min(rect.right().saturating_sub(1))
        } else {
            rect.right().saturating_sub(1)
        };
        for x in x_lo..=x_hi {
            if is_border_cell(x, y, content_map) || !buffer.in_bounds(x, y) {
                continue;
            }
            let sym = &buffer.get(x, y).symbol;
            if !sym.is_empty() {
                line.push_str(sym);
            }
        }
        // Trim trailing spaces in place: `trim_end().to_string()` would
        // allocate a second `String` and immediately drop the original.
        let trimmed_len = line.trim_end().len();
        line.truncate(trimmed_len);
        lines.push(line);
    }
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::Style;

    fn make_state(anchor: (u32, u32), current: (u32, u32), rect: Rect) -> SelectionState {
        SelectionState {
            anchor: Some(anchor),
            current: Some(current),
            widget_rect: Some(rect),
            active: true,
        }
    }

    #[test]
    fn extract_selection_text_trims_trailing_spaces_in_place() {
        // Verifies the in-place truncate path matches the previous
        // `trim_end().to_string()` behavior: lines lose trailing spaces and
        // empty trailing lines are dropped.
        let area = Rect::new(0, 0, 10, 3);
        let mut buf = Buffer::empty(area);
        buf.set_string(0, 0, "hello", Style::new());
        // row 1: "hi" then 8 trailing spaces (default cells are blanks).
        buf.set_string(0, 1, "hi", Style::new());
        // row 2 left blank → must be dropped after pop loop.

        let sel = make_state((0, 0), (9, 2), area);
        let out = extract_selection_text(&buf, &sel, &[]);
        assert_eq!(out, "hello\nhi");
    }

    #[test]
    fn extract_selection_text_preserves_multibyte_content() {
        // truncate(len) operates on UTF-8 byte indices; trim_end() returns a
        // byte-aligned slice, so this must hold for CJK and emoji.
        let area = Rect::new(0, 0, 6, 1);
        let mut buf = Buffer::empty(area);
        buf.set_string(0, 0, "世界", Style::new());
        let sel = make_state((0, 0), (5, 0), area);
        let out = extract_selection_text(&buf, &sel, &[]);
        assert_eq!(out, "世界");
    }
}
