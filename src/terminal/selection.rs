use super::*;

#[derive(Default)]
pub(crate) struct SelectionState {
    pub anchor: Option<(u32, u32)>,
    pub current: Option<(u32, u32)>,
    pub widget_rect: Option<Rect>,
    pub active: bool,
}

impl SelectionState {
    pub fn mouse_down(&mut self, x: u32, y: u32, hit_map: &[(Rect, Rect)]) {
        self.anchor = Some((x, y));
        self.current = Some((x, y));
        self.widget_rect = find_innermost_rect(hit_map, x, y);
        self.active = false;
    }

    pub fn mouse_drag(&mut self, x: u32, y: u32, hit_map: &[(Rect, Rect)]) {
        if let Some(anchor) = self.anchor {
            self.current = Some((x, y));
            if x.abs_diff(anchor.0) > 1 || y.abs_diff(anchor.1) > 0 {
                self.active = true;
            }
            if let Some(rect) = self.widget_rect {
                if y < rect.y || y >= rect.bottom() || x < rect.x || x >= rect.right() {
                    self.widget_rect = find_containing_rect(hit_map, anchor, (x, y));
                }
            }
        }
    }

    pub fn clear(&mut self) {
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
            if in_sel {
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
            if is_border_cell(x, y, content_map) {
                continue;
            }
            let sym = &buffer.get(x, y).symbol;
            if !sym.is_empty() {
                line.push_str(sym);
            }
        }
        lines.push(line.trim_end().to_string());
    }
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}
