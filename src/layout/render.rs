use super::flexbox::inner_area;
use super::*;

pub(crate) fn render(node: &LayoutNode, buf: &mut Buffer) {
    render_inner(node, buf, 0, None);
    buf.clip_stack.clear();
    for overlay in &node.overlays {
        if overlay.modal {
            dim_entire_buffer(buf);
        }
        render_inner(&overlay.node, buf, 0, None);
    }
}

fn dim_entire_buffer(buf: &mut Buffer) {
    for y in buf.area.y..buf.area.bottom() {
        for x in buf.area.x..buf.area.right() {
            let cell = buf.get_mut(x, y);
            cell.style.modifiers |= crate::style::Modifiers::DIM;
        }
    }
}

pub(crate) fn render_debug_overlay(
    node: &LayoutNode,
    buf: &mut Buffer,
    frame_time_us: u64,
    fps: f32,
) {
    for child in &node.children {
        render_debug_overlay_inner(child, buf, 0, 0);
    }
    render_debug_status_bar(node, buf, frame_time_us, fps);
}

fn render_debug_status_bar(node: &LayoutNode, buf: &mut Buffer, frame_time_us: u64, fps: f32) {
    if buf.area.height == 0 || buf.area.width == 0 {
        return;
    }

    let widgets: u32 = node.children.iter().map(count_leaf_widgets).sum();
    let width = buf.area.width;
    let height = buf.area.height;
    let y = buf.area.bottom() - 1;
    let style = Style::new().fg(Color::Black).bg(Color::Yellow).bold();

    let status = format!(
        "[SLT Debug] {}x{} | {} widgets | {:.1}ms | {:.0}fps",
        width,
        height,
        widgets,
        frame_time_us as f64 / 1_000.0,
        fps.max(0.0)
    );

    let row_fill = " ".repeat(width as usize);
    buf.set_string(buf.area.x, y, &row_fill, style);
    buf.set_string(buf.area.x, y, &status, style);
}

fn count_leaf_widgets(node: &LayoutNode) -> u32 {
    let mut total = if node.children.is_empty() {
        match node.kind {
            NodeKind::Spacer => 0,
            _ => 1,
        }
    } else {
        node.children.iter().map(count_leaf_widgets).sum()
    };

    for overlay in &node.overlays {
        total = total.saturating_add(count_leaf_widgets(&overlay.node));
    }

    total
}

fn render_debug_overlay_inner(node: &LayoutNode, buf: &mut Buffer, depth: u32, y_offset: u32) {
    let child_offset = if node.is_scrollable {
        y_offset.saturating_add(node.scroll_offset)
    } else {
        y_offset
    };

    if let NodeKind::Container(_) = node.kind {
        let sy = screen_y(node.pos.1, y_offset);
        if sy + node.size.1 as i64 > 0 {
            let color = debug_color_for_depth(depth);
            let style = Style::new().fg(color);
            let clamped_y = sy.max(0) as u32;
            draw_debug_border(node.pos.0, clamped_y, node.size.0, node.size.1, buf, style);
            if sy >= 0 {
                buf.set_string(node.pos.0, clamped_y, &depth.to_string(), style);
            }
        }
    }

    if node.is_scrollable {
        if let Some(area) = visible_area(node, y_offset) {
            let inner = inner_area(node, area);
            buf.push_clip(inner);
            for child in &node.children {
                render_debug_overlay_inner(child, buf, depth.saturating_add(1), child_offset);
            }
            buf.pop_clip();
        }
    } else {
        for child in &node.children {
            render_debug_overlay_inner(child, buf, depth.saturating_add(1), child_offset);
        }
    }
}

fn debug_color_for_depth(depth: u32) -> Color {
    match depth {
        0 => Color::Cyan,
        1 => Color::Yellow,
        2 => Color::Magenta,
        _ => Color::Red,
    }
}

fn draw_debug_border(x: u32, y: u32, w: u32, h: u32, buf: &mut Buffer, style: Style) {
    if w == 0 || h == 0 {
        return;
    }
    let right = x + w - 1;
    let bottom = y + h - 1;

    if w == 1 && h == 1 {
        buf.set_char(x, y, '┼', style);
        return;
    }
    if h == 1 {
        for xx in x..=right {
            buf.set_char(xx, y, '─', style);
        }
        return;
    }
    if w == 1 {
        for yy in y..=bottom {
            buf.set_char(x, yy, '│', style);
        }
        return;
    }

    buf.set_char(x, y, '┌', style);
    buf.set_char(right, y, '┐', style);
    buf.set_char(x, bottom, '└', style);
    buf.set_char(right, bottom, '┘', style);

    for xx in (x + 1)..right {
        buf.set_char(xx, y, '─', style);
        buf.set_char(xx, bottom, '─', style);
    }
    for yy in (y + 1)..bottom {
        buf.set_char(x, yy, '│', style);
        buf.set_char(right, yy, '│', style);
    }
}

#[allow(dead_code)]
fn draw_debug_padding_markers(node: &LayoutNode, y_offset: u32, buf: &mut Buffer, style: Style) {
    if node.size.0 == 0 || node.size.1 == 0 {
        return;
    }

    if node.padding == Padding::default() {
        return;
    }

    let Some(area) = visible_area(node, y_offset) else {
        return;
    };
    let inner = inner_area(node, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let right = inner.right() - 1;
    let bottom = inner.bottom() - 1;
    buf.set_char(inner.x, inner.y, 'p', style);
    buf.set_char(right, inner.y, 'p', style);
    buf.set_char(inner.x, bottom, 'p', style);
    buf.set_char(right, bottom, 'p', style);
}

#[allow(dead_code)]
fn draw_debug_margin_markers(node: &LayoutNode, y_offset: u32, buf: &mut Buffer, style: Style) {
    if node.margin == Margin::default() {
        return;
    }

    let margin_y_i = node.pos.1 as i64 - node.margin.top as i64 - y_offset as i64;
    let w = node
        .size
        .0
        .saturating_add(node.margin.horizontal())
        .max(node.margin.horizontal());
    let h = node
        .size
        .1
        .saturating_add(node.margin.vertical())
        .max(node.margin.vertical());

    if w == 0 || h == 0 || margin_y_i + h as i64 <= 0 {
        return;
    }

    let x = node.pos.0.saturating_sub(node.margin.left);
    let y = margin_y_i.max(0) as u32;
    let bottom_i = margin_y_i + h as i64 - 1;
    if bottom_i < 0 {
        return;
    }
    let right = x + w - 1;
    let bottom = bottom_i as u32;
    if margin_y_i >= 0 {
        buf.set_char(x, y, 'm', style);
        buf.set_char(right, y, 'm', style);
    }
    buf.set_char(x, bottom, 'm', style);
    buf.set_char(right, bottom, 'm', style);
}

fn screen_y(layout_y: u32, y_offset: u32) -> i64 {
    layout_y as i64 - y_offset as i64
}

fn visible_area(node: &LayoutNode, y_offset: u32) -> Option<Rect> {
    let sy = screen_y(node.pos.1, y_offset);
    let bottom = sy + node.size.1 as i64;
    if bottom <= 0 || node.size.0 == 0 || node.size.1 == 0 {
        return None;
    }
    let clamped_y = sy.max(0) as u32;
    let clamped_h = (bottom as u32).saturating_sub(clamped_y);
    Some(Rect::new(node.pos.0, clamped_y, node.size.0, clamped_h))
}

fn render_inner(node: &LayoutNode, buf: &mut Buffer, y_offset: u32, parent_bg: Option<Color>) {
    if node.size.0 == 0 || node.size.1 == 0 {
        return;
    }

    let sy = screen_y(node.pos.1, y_offset);
    let sx = i64::from(node.pos.0);
    let ex = sx.saturating_add(i64::from(node.size.0));
    let ey = sy.saturating_add(i64::from(node.size.1));
    let viewport_left = i64::from(buf.area.x);
    let viewport_top = i64::from(buf.area.y);
    let viewport_right = viewport_left.saturating_add(i64::from(buf.area.width));
    let viewport_bottom = viewport_top.saturating_add(i64::from(buf.area.height));

    if ex <= viewport_left || ey <= viewport_top || sx >= viewport_right || sy >= viewport_bottom {
        return;
    }

    match node.kind {
        NodeKind::Text => {
            if let Some(ref segs) = node.segments {
                if node.wrap {
                    let fallback;
                    let wrapped = if let Some(cached) = &node.cached_wrapped_segments {
                        cached.as_slice()
                    } else {
                        fallback = wrap_segments(segs, node.size.0);
                        &fallback
                    };
                    for (i, line_segs) in wrapped.iter().enumerate() {
                        let line_y = sy + i as i64;
                        if line_y < 0 {
                            continue;
                        }
                        let mut x = node.pos.0;
                        for (text, style) in line_segs {
                            let mut s = *style;
                            if s.bg.is_none() {
                                s.bg = parent_bg;
                            }
                            buf.set_string(x, line_y as u32, text, s);
                            x += UnicodeWidthStr::width(text.as_str()) as u32;
                        }
                    }
                } else {
                    if sy < 0 {
                        return;
                    }
                    let mut x = node.pos.0;
                    for (text, style) in segs {
                        let mut s = *style;
                        if s.bg.is_none() {
                            s.bg = parent_bg;
                        }
                        buf.set_string(x, sy as u32, text, s);
                        x += UnicodeWidthStr::width(text.as_str()) as u32;
                    }
                }
            } else if let Some(ref text) = node.content {
                let mut style = node.style;
                if style.bg.is_none() {
                    style.bg = parent_bg;
                }
                if node.wrap {
                    let fallback;
                    let lines = if let Some(cached) = &node.cached_wrapped {
                        cached.as_slice()
                    } else {
                        fallback = wrap_lines(text, node.size.0);
                        fallback.as_slice()
                    };
                    for (i, line) in lines.iter().enumerate() {
                        let line_y = sy + i as i64;
                        if line_y < 0 {
                            continue;
                        }
                        let text_width = UnicodeWidthStr::width(line.as_str()) as u32;
                        let x_offset = if text_width < node.size.0 {
                            match node.align {
                                Align::Start => 0,
                                Align::Center => (node.size.0 - text_width) / 2,
                                Align::End => node.size.0 - text_width,
                            }
                        } else {
                            0
                        };
                        buf.set_string(
                            node.pos.0.saturating_add(x_offset),
                            line_y as u32,
                            line,
                            style,
                        );
                    }
                } else {
                    if sy < 0 {
                        return;
                    }
                    let text_width = UnicodeWidthStr::width(text.as_str()) as u32;
                    if node.truncate && text_width > node.size.0 && node.size.0 > 1 {
                        let truncated = truncate_with_ellipsis(text, node.size.0 as usize);
                        let trunc_width = UnicodeWidthStr::width(truncated.as_str()) as u32;
                        let x_off = if trunc_width < node.size.0 {
                            match node.align {
                                Align::Start => 0,
                                Align::Center => (node.size.0 - trunc_width) / 2,
                                Align::End => node.size.0 - trunc_width,
                            }
                        } else {
                            0
                        };
                        let draw_x = node.pos.0.saturating_add(x_off);
                        if let Some(ref url) = node.link_url {
                            buf.set_string_linked(draw_x, sy as u32, &truncated, style, url);
                        } else {
                            buf.set_string(draw_x, sy as u32, &truncated, style);
                        }
                    } else {
                        let x_offset = if text_width < node.size.0 {
                            match node.align {
                                Align::Start => 0,
                                Align::Center => (node.size.0 - text_width) / 2,
                                Align::End => node.size.0 - text_width,
                            }
                        } else {
                            0
                        };
                        let draw_x = node.pos.0.saturating_add(x_offset);
                        if let Some(ref url) = node.link_url {
                            buf.set_string_linked(draw_x, sy as u32, text, style, url);
                        } else {
                            buf.set_string(draw_x, sy as u32, text, style);
                        }
                    }
                }
            }
        }
        NodeKind::Spacer | NodeKind::RawDraw(_) => {}
        NodeKind::Container(_) => {
            if let Some(color) = node.bg_color {
                if let Some(area) = visible_area(node, y_offset) {
                    let fill_style = Style::new().bg(color);
                    for y in area.y..area.bottom() {
                        for x in area.x..area.right() {
                            buf.set_string(x, y, " ", fill_style);
                        }
                    }
                }
            }
            let child_bg = node.bg_color.or(parent_bg);
            render_container_border(node, buf, y_offset, child_bg);
            if node.is_scrollable {
                let Some(area) = visible_area(node, y_offset) else {
                    return;
                };
                let inner = inner_area(node, area);
                let child_offset = y_offset.saturating_add(node.scroll_offset);
                let render_y_start = inner.y as i64;
                let render_y_end = inner.bottom() as i64;
                buf.push_clip(inner);
                for child in &node.children {
                    let child_top = child.pos.1 as i64 - child_offset as i64;
                    let child_bottom = child_top + child.size.1 as i64;
                    if child_bottom <= render_y_start || child_top >= render_y_end {
                        continue;
                    }
                    render_inner(child, buf, child_offset, child_bg);
                }
                buf.pop_clip();
                render_scroll_indicators(node, inner, buf, child_bg);
            } else {
                let Some(area) = visible_area(node, y_offset) else {
                    return;
                };
                let clip = inner_area(node, area);
                buf.push_clip(clip);
                for child in &node.children {
                    render_inner(child, buf, y_offset, child_bg);
                }
                buf.pop_clip();
            }
        }
    }
}

fn render_container_border(
    node: &LayoutNode,
    buf: &mut Buffer,
    y_offset: u32,
    inherit_bg: Option<Color>,
) {
    if node.border_inset() == 0 {
        return;
    }
    let Some(border) = node.border else {
        return;
    };
    let sides = node.border_sides;
    let chars = border.chars();
    let x = node.pos.0;
    let w = node.size.0;
    let h = node.size.1;
    if w == 0 || h == 0 {
        return;
    }

    let mut style = node.border_style;
    if style.bg.is_none() {
        style.bg = inherit_bg;
    }

    let top_i = screen_y(node.pos.1, y_offset);
    let bottom_i = top_i + h as i64 - 1;
    if bottom_i < 0 {
        return;
    }
    let right = x + w - 1;

    if sides.top && top_i >= 0 {
        let y = top_i as u32;
        for xx in x..=right {
            buf.set_char(xx, y, chars.h, style);
        }
    }
    if sides.bottom {
        let y = bottom_i as u32;
        for xx in x..=right {
            buf.set_char(xx, y, chars.h, style);
        }
    }
    if sides.left {
        let vert_start = top_i.max(0) as u32;
        let vert_end = bottom_i as u32;
        for yy in vert_start..=vert_end {
            buf.set_char(x, yy, chars.v, style);
        }
    }
    if sides.right {
        let vert_start = top_i.max(0) as u32;
        let vert_end = bottom_i as u32;
        for yy in vert_start..=vert_end {
            buf.set_char(right, yy, chars.v, style);
        }
    }

    if top_i >= 0 {
        let y = top_i as u32;
        let tl = match (sides.top, sides.left) {
            (true, true) => Some(chars.tl),
            (true, false) => Some(chars.h),
            (false, true) => Some(chars.v),
            (false, false) => None,
        };
        if let Some(ch) = tl {
            buf.set_char(x, y, ch, style);
        }

        let tr = match (sides.top, sides.right) {
            (true, true) => Some(chars.tr),
            (true, false) => Some(chars.h),
            (false, true) => Some(chars.v),
            (false, false) => None,
        };
        if let Some(ch) = tr {
            buf.set_char(right, y, ch, style);
        }
    }

    let y = bottom_i as u32;
    let bl = match (sides.bottom, sides.left) {
        (true, true) => Some(chars.bl),
        (true, false) => Some(chars.h),
        (false, true) => Some(chars.v),
        (false, false) => None,
    };
    if let Some(ch) = bl {
        buf.set_char(x, y, ch, style);
    }

    let br = match (sides.bottom, sides.right) {
        (true, true) => Some(chars.br),
        (true, false) => Some(chars.h),
        (false, true) => Some(chars.v),
        (false, false) => None,
    };
    if let Some(ch) = br {
        buf.set_char(right, y, ch, style);
    }

    if sides.top && top_i >= 0 {
        if let Some((title, title_style)) = &node.title {
            let mut ts = *title_style;
            if ts.bg.is_none() {
                ts.bg = inherit_bg;
            }
            let y = top_i as u32;
            let title_x = x.saturating_add(2);
            if title_x <= right {
                let max_width = (right - title_x + 1) as usize;
                let trimmed: String = title.chars().take(max_width).collect();
                buf.set_string(title_x, y, &trimmed, ts);
            }
        }
    }
}

fn render_scroll_indicators(
    node: &LayoutNode,
    inner: Rect,
    buf: &mut Buffer,
    inherit_bg: Option<Color>,
) {
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let mut style = node.border_style;
    if style.bg.is_none() {
        style.bg = inherit_bg;
    }

    let indicator_x = inner.right() - 1;
    if node.scroll_offset > 0 {
        buf.set_char(indicator_x, inner.y, '▲', style);
    }
    if node.scroll_offset.saturating_add(inner.height) < node.content_height {
        buf.set_char(indicator_x, inner.bottom() - 1, '▼', style);
    }
}

fn truncate_with_ellipsis(text: &str, max_width: usize) -> String {
    use unicode_width::UnicodeWidthChar;

    if max_width == 0 {
        return String::new();
    }
    if max_width == 1 {
        return "\u{2026}".to_string();
    }
    let target = max_width - 1;
    let mut result = String::new();
    let mut width = 0;
    for ch in text.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + ch_width > target {
            break;
        }
        result.push(ch);
        width += ch_width;
    }
    result.push('\u{2026}');
    result
}
