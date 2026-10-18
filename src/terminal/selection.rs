use super::*;

#[derive(Default)]
pub(crate) struct SelectionState {
    /// Selection start. Screen cells, or content cells (`screen_y + offset`)
    /// when `region` is set.
    pub anchor: Option<(u32, u32)>,
    /// Latest pointer position in screen cells.
    pub current: Option<(u32, u32)>,
    pub widget_rect: Option<Rect>,
    pub active: bool,
    /// A widget consumed the press of the current gesture (#438): the
    /// selection stays idle until the matching release, even when later drag
    /// events leave the widget and are not consumed individually.
    pub captured: bool,
    /// `ScrollState` identity of the scroll region the selection started in
    /// (#445). The selection is then kept in content coordinates so it stays
    /// attached to its text while the region scrolls.
    pub region: Option<u64>,
    /// The left button is held for the current selection gesture.
    pub holding: bool,
    /// Selection end frozen in content coordinates on release (#445).
    pub current_content: Option<(u32, u32)>,
}

/// Scroll-region geometry the selection projects through (#445).
#[derive(Clone, Copy, Default)]
pub(crate) struct ScrollView<'a> {
    geometry: Option<&'a crate::layout::GeometryFeedback>,
    rects: &'a [Rect],
}

impl<'a> ScrollView<'a> {
    pub(crate) fn new(geometry: &'a crate::layout::GeometryFeedback, rects: &'a [Rect]) -> Self {
        Self {
            geometry: Some(geometry),
            rects,
        }
    }

    /// Visible screen rect and current offset of the region bound to `id`.
    fn find(&self, id: u64) -> Option<(Rect, u32)> {
        let geometry = self.geometry?;
        let index = *geometry.scroll_by_id.get(&id)?;
        let scroll = geometry.scrolls.get(index)?;
        let rect = *self.rects.get(index)?;
        (!rect.is_empty()).then_some((rect, scroll.offset))
    }

    /// Innermost vertical, `ScrollState`-bound region containing `(x, y)`.
    fn region_at(&self, x: u32, y: u32) -> Option<(u64, Rect, u32)> {
        let geometry = self.geometry?;
        geometry
            .scrolls
            .iter()
            .zip(self.rects)
            .rev()
            .find(|(scroll, rect)| {
                scroll.state_id != 0 && !scroll.horizontal && rect.contains(x, y)
            })
            .map(|(scroll, rect)| (scroll.state_id, *rect, scroll.offset))
    }
}

/// What the selection tracker decided for this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SelectionFrame {
    /// A selection ended this frame and should be copied.
    pub should_copy: bool,
    /// Edge auto-scroll request for the next frame: region id and direction
    /// (`-1` up, `1` down) while a selection drag holds the pointer on or
    /// beyond the region's top or bottom row (#445).
    pub autoscroll: Option<(u64, i8)>,
}

/// Feed this frame's left-button events to the built-in text selection.
///
/// Events a widget consumed never start or extend a selection (#438), and a
/// consumed press keeps the tracker idle for the whole gesture. With
/// `enabled == false` (`RunConfig::text_selection(false)`) the tracker is off
/// and any selection is cleared. Returns whether the frame ended a selection
/// that should be copied.
pub(crate) fn track_selection(
    selection: &mut SelectionState,
    events: &[crate::event::Event],
    consumed: &[bool],
    content_map: &[(Rect, Rect)],
    view: ScrollView<'_>,
    enabled: bool,
) -> SelectionFrame {
    use crate::event::{Event, MouseButton, MouseKind};
    if !enabled {
        if selection.active || selection.anchor.is_some() || selection.captured {
            selection.clear();
        }
        return SelectionFrame::default();
    }
    let mut should_copy = false;
    for (i, event) in events.iter().enumerate() {
        let Event::Mouse(mouse) = event else {
            continue;
        };
        let taken = consumed.get(i).copied().unwrap_or(false);
        match mouse.kind {
            MouseKind::Down(MouseButton::Left) => {
                if taken {
                    // A widget owns this gesture: drop any stale selection.
                    selection.clear();
                    selection.captured = true;
                } else {
                    selection.captured = false;
                    selection.mouse_down_in(mouse.x, mouse.y, content_map, view);
                }
            }
            MouseKind::Drag(MouseButton::Left) if !taken && !selection.captured => {
                selection.mouse_drag_in(mouse.x, mouse.y, content_map, view);
            }
            MouseKind::Up(MouseButton::Left) => {
                if selection.captured || taken {
                    selection.captured = false;
                } else {
                    selection.mouse_up(view);
                    should_copy = selection.active;
                }
            }
            _ => {}
        }
    }
    SelectionFrame {
        should_copy,
        autoscroll: selection.autoscroll_request(view),
    }
}

impl SelectionState {
    /// Record a left-mouse-down anchor point and identify the widget
    /// rect under the cursor; resets the active flag so a single click
    /// without drag clears any prior selection. Screen-space only (tests).
    #[cfg(test)]
    pub(crate) fn mouse_down(&mut self, x: u32, y: u32, hit_map: &[(Rect, Rect)]) {
        self.mouse_down_in(x, y, hit_map, ScrollView::default());
    }

    /// `mouse_down` that anchors in content coordinates when the press lands
    /// inside a scroll region (#445).
    pub(crate) fn mouse_down_in(
        &mut self,
        x: u32,
        y: u32,
        hit_map: &[(Rect, Rect)],
        view: ScrollView<'_>,
    ) {
        self.current = Some((x, y));
        self.current_content = None;
        self.active = false;
        self.holding = true;
        if let Some((id, rect, offset)) = view.region_at(x, y) {
            self.region = Some(id);
            self.anchor = Some((x, y.saturating_add(offset)));
            self.widget_rect = Some(rect);
        } else {
            self.region = None;
            self.anchor = Some((x, y));
            self.widget_rect = find_innermost_rect(hit_map, x, y);
        }
    }

    /// `mouse_drag` for a selection that may live in a scroll region.
    pub(crate) fn mouse_drag_in(
        &mut self,
        x: u32,
        y: u32,
        hit_map: &[(Rect, Rect)],
        view: ScrollView<'_>,
    ) {
        let Some(region) = self.region else {
            self.mouse_drag(x, y, hit_map);
            return;
        };
        let Some(anchor) = self.anchor else {
            return;
        };
        self.current = Some((x, y));
        let Some((rect, offset)) = view.find(region) else {
            return;
        };
        self.widget_rect = Some(rect);
        let point = content_point((x, y), rect, offset);
        // The end sticks to the content under the pointer at the drag event,
        // so wheel scrolling mid-gesture does not drag the selection along.
        self.current_content = Some(point);
        if point.0.abs_diff(anchor.0) > 1 || point.1 != anchor.1 {
            self.active = true;
        }
    }

    /// End the gesture; a region selection freezes its end in content
    /// coordinates so later scrolling does not move it (#445).
    pub(crate) fn mouse_up(&mut self, view: ScrollView<'_>) {
        if let Some(region) = self.region
            && let Some((rect, offset)) = view.find(region)
        {
            self.current_content = Some(self.region_end(rect, offset));
        }
        self.holding = false;
    }

    /// Whether the held pointer sits on or beyond the region's top or bottom
    /// row (the edge auto-scroll zone).
    fn at_edge(&self, rect: Rect) -> Option<i8> {
        let (_, y) = self.current?;
        if !self.holding {
            None
        } else if y <= rect.y {
            Some(-1)
        } else if y >= rect.bottom().saturating_sub(1) {
            Some(1)
        } else {
            None
        }
    }

    /// Selection end of a region selection in content cells: follows the
    /// pointer through auto-scroll at the edges, otherwise the content under
    /// the pointer at its last drag.
    fn region_end(&self, rect: Rect, offset: u32) -> (u32, u32) {
        let screen = self.current.unwrap_or_default();
        if self.at_edge(rect).is_some() {
            return content_point(screen, rect, offset);
        }
        self.current_content
            .unwrap_or_else(|| content_point(screen, rect, offset))
    }

    /// Edge auto-scroll request while a region selection is being dragged.
    fn autoscroll_request(&self, view: ScrollView<'_>) -> Option<(u64, i8)> {
        if !self.holding || !self.active {
            return None;
        }
        let region = self.region?;
        let (rect, _) = view.find(region)?;
        self.at_edge(rect).map(|direction| (region, direction))
    }

    /// Resolve the selection for drawing or copying: `(start, end)` in the
    /// coordinates of `rect` rows shifted by `offset` (content cells for a
    /// region selection, screen cells otherwise).
    fn resolve(&self, view: ScrollView<'_>) -> Option<Resolved> {
        if !self.active {
            return None;
        }
        let anchor = self.anchor?;
        let (rect, offset, current) = match self.region {
            Some(region) => {
                let (rect, offset) = view
                    .find(region)
                    .or_else(|| self.widget_rect.map(|rect| (rect, 0)))?;
                (rect, offset, self.region_end(rect, offset))
            }
            None => (self.widget_rect?, 0, self.current?),
        };
        let (start, end) = normalize_selection(anchor, current);
        Some(Resolved {
            start,
            end,
            rect,
            offset,
        })
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

/// A resolved selection: endpoints and the rows they are drawn in.
struct Resolved {
    start: (u32, u32),
    end: (u32, u32),
    rect: Rect,
    /// Added to a screen row of `rect` to get the selection's row space.
    offset: u32,
}

/// Map a screen point into a region's content cells, clamping the row to the
/// region so a pointer above or below it selects up to the visible edge.
fn content_point((x, y): (u32, u32), rect: Rect, offset: u32) -> (u32, u32) {
    let y = y.clamp(rect.y, rect.bottom().saturating_sub(1).max(rect.y));
    (x, y.saturating_add(offset))
}

/// Screen-space overlay without scroll-region projection (tests).
#[cfg(test)]
pub(crate) fn apply_selection_overlay(
    buffer: &mut Buffer,
    sel: &SelectionState,
    content_map: &[(Rect, Rect)],
) {
    apply_selection_overlay_in(buffer, sel, content_map, ScrollView::default());
}

/// Draw the selection highlight. A region selection is re-projected through
/// the region's current offset, so it follows its text when the region
/// scrolls; rows scrolled out of view are not drawn but stay selected.
pub(crate) fn apply_selection_overlay_in(
    buffer: &mut Buffer,
    sel: &SelectionState,
    content_map: &[(Rect, Rect)],
    view: ScrollView<'_>,
) {
    let Some(Resolved {
        start,
        end,
        rect,
        offset,
    }) = sel.resolve(view)
    else {
        return;
    };

    for y in rect.y..rect.bottom() {
        let row = y.saturating_add(offset);
        if row < start.1 || row > end.1 {
            continue;
        }
        for x in rect.x..rect.right() {
            if is_border_cell(x, y, content_map) {
                continue;
            }
            let in_sel = if start.1 == end.1 {
                row == start.1 && x >= start.0 && x <= end.0
            } else if row == start.1 {
                x >= start.0
            } else if row == end.1 {
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

/// Screen-space extraction without scroll-region projection (tests).
#[cfg(test)]
pub(crate) fn extract_selection_text(
    buffer: &Buffer,
    sel: &SelectionState,
    content_map: &[(Rect, Rect)],
) -> String {
    extract_selection_text_in(buffer, sel, content_map, ScrollView::default())
}

/// Copy the selected text from the current screen buffer.
///
/// Only the part of a region selection that is visible this frame can be
/// copied: rows scrolled out of view are not retained, so a selection that
/// extends past the viewport copies its visible portion.
pub(crate) fn extract_selection_text_in(
    buffer: &Buffer,
    sel: &SelectionState,
    content_map: &[(Rect, Rect)],
    view: ScrollView<'_>,
) -> String {
    let Some(Resolved {
        start,
        end,
        rect,
        offset,
    }) = sel.resolve(view)
    else {
        return String::new();
    };
    let y_lo = start.1.max(rect.y.saturating_add(offset));
    let y_hi = end
        .1
        .min(rect.bottom().saturating_sub(1).saturating_add(offset));
    if y_lo > y_hi {
        return String::new();
    }

    let mut lines: Vec<String> = Vec::new();
    for row in y_lo..=y_hi {
        let y = row - offset;
        let mut line = String::new();
        let x_lo = if row == start.1 {
            start.0.max(rect.x)
        } else {
            rect.x
        };
        let x_hi = if row == end.1 {
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
            ..SelectionState::default()
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
