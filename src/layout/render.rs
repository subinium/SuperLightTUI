use super::flexbox::inner_area;
use super::*;

pub(crate) fn render(node: &LayoutNode, buf: &mut Buffer) {
    render_inner(node, buf, 0, None, 0);
    buf.clip_stack.clear();
    for overlay in &node.overlays {
        if overlay.modal {
            dim_entire_buffer(buf);
        }
        render_inner(&overlay.node, buf, 0, None, 0);
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

/// Layer category used to tint F12 debug outlines.
///
/// Inspired by Chrome DevTools layout overlay, React DevTools component
/// highlighter, and the Flutter Inspector — each layer family gets its own
/// hue so a glance at the screen tells you which container is part of the
/// base tree, an overlay, or a modal. Within each family the depth still
/// varies the brightness so nested containers remain distinguishable.
#[derive(Debug, Clone, Copy)]
enum LayerTint {
    /// Base tree (root + children) — green family ("default, healthy").
    Base,
    /// Floating overlay — red family ("attention"). Tooltips share this
    /// tint because they ride the same `overlay()` plumbing as
    /// [`Context::overlay`] (no separate variant tag yet).
    Overlay,
    /// Modal dialog — blue family ("deliberate, dimmed background").
    Modal,
}

/// Per-layer widget breakdown for the debug status bar.
///
/// Returned by [`count_leaf_widgets_layered`]. The legacy `total` accessor
/// preserves the v0.19.3 "N widgets" status line so existing snapshot tests
/// stay green; the per-layer fields surface in the new
/// "(N base, M overlay, K modal)" suffix.
#[derive(Debug, Clone, Copy, Default)]
struct LayerCounts {
    base: u32,
    overlay: u32,
    modal: u32,
}

impl LayerCounts {
    fn total(self) -> u32 {
        self.base
            .saturating_add(self.overlay)
            .saturating_add(self.modal)
    }
}

pub(crate) fn render_debug_overlay(
    node: &LayoutNode,
    buf: &mut Buffer,
    frame_time_us: u64,
    fps: f32,
    layer: crate::DebugLayer,
) {
    // Issue #201 Part A: previously this only walked `node.children`, so any
    // active overlay/modal was invisible to the F12 outline pass even though
    // the underlying renderer DID draw it. Walk overlays too unless the user
    // explicitly opted into a narrower layer via [`Context::set_debug_layer`].
    let walk_base = !matches!(layer, crate::DebugLayer::TopMost) || node.overlays.is_empty();
    let walk_overlays = !matches!(layer, crate::DebugLayer::BaseOnly);
    if walk_base {
        for child in &node.children {
            render_debug_overlay_inner(child, buf, 0, 0, LayerTint::Base);
        }
    }
    if walk_overlays {
        for overlay in &node.overlays {
            // Distinguish modal from non-modal overlays — `OverlayLayer.modal`
            // is the only tag the layout tree carries today (tooltips fall
            // under the non-modal `Overlay` bucket).
            let tint = if overlay.modal {
                LayerTint::Modal
            } else {
                LayerTint::Overlay
            };
            render_debug_overlay_inner(&overlay.node, buf, 0, 0, tint);
        }
    }
    render_debug_status_bar(node, buf, frame_time_us, fps);
}

fn render_debug_status_bar(node: &LayoutNode, buf: &mut Buffer, frame_time_us: u64, fps: f32) {
    if buf.area.height == 0 || buf.area.width == 0 {
        return;
    }

    // Issue #201 Part C: include overlay widgets in the status-bar count so
    // the displayed total matches what the renderer actually drew. The
    // recursive `count_leaf_widgets` already walks overlays at every nested
    // level — only the root sum was missing the overlay branch.
    let counts = count_leaf_widgets_layered(node);
    let widgets = counts.total();
    let width = buf.area.width;
    let height = buf.area.height;
    let y = buf.area.bottom() - 1;
    let style = Style::new().fg(Color::Black).bg(Color::Yellow).bold();

    // Per-layer breakdown only renders the layers that actually have
    // widgets, so a base-only scene keeps the original short status line.
    let mut breakdown_parts: Vec<String> = Vec::with_capacity(3);
    if counts.base > 0 {
        breakdown_parts.push(format!("{} base", counts.base));
    }
    if counts.overlay > 0 {
        breakdown_parts.push(format!("{} overlay", counts.overlay));
    }
    if counts.modal > 0 {
        breakdown_parts.push(format!("{} modal", counts.modal));
    }
    let breakdown = if breakdown_parts.len() > 1 {
        format!(" ({})", breakdown_parts.join(", "))
    } else {
        String::new()
    };

    let status = format!(
        "[SLT Debug] {}x{} | {} widgets{} | {:.1}ms | {:.0}fps",
        width,
        height,
        widgets,
        breakdown,
        frame_time_us as f64 / 1_000.0,
        fps.max(0.0)
    );

    let row_fill = " ".repeat(width as usize);
    buf.set_string(buf.area.x, y, &row_fill, style);
    buf.set_string(buf.area.x, y, &status, style);
}

/// Count leaf widgets per layer category, walking nested overlays.
///
/// The base count comes from `node.children`. Overlays are split by their
/// `modal` flag — tooltips ride the non-modal path (see [`LayerTint`]).
/// Nested overlays inside an overlay's subtree inherit the outer overlay's
/// bucket: the goal is "how many widgets did each top-level layer
/// contribute," matching what the user sees on screen.
fn count_leaf_widgets_layered(node: &LayoutNode) -> LayerCounts {
    let base: u32 = node.children.iter().map(count_leaf_widgets).sum();
    let mut overlay: u32 = 0;
    let mut modal: u32 = 0;
    for layer in &node.overlays {
        let n = count_leaf_widgets(&layer.node);
        if layer.modal {
            modal = modal.saturating_add(n);
        } else {
            overlay = overlay.saturating_add(n);
        }
    }
    LayerCounts {
        base,
        overlay,
        modal,
    }
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

fn render_debug_overlay_inner(
    node: &LayoutNode,
    buf: &mut Buffer,
    depth: u32,
    y_offset: u32,
    tint: LayerTint,
) {
    let child_offset = if node.is_scrollable {
        y_offset.saturating_add(node.scroll_offset)
    } else {
        y_offset
    };

    if let NodeKind::Container(_) = node.kind {
        let sy = screen_y(node.pos.1, y_offset);
        if sy + node.size.1 as i64 > 0 {
            let color = debug_color_for_depth(tint, depth);
            let style = Style::new().fg(color);
            let clamped_y = sy.max(0) as u32;
            draw_debug_border(node.pos.0, clamped_y, node.size.0, node.size.1, buf, style);
            if sy >= 0 {
                buf.set_string(node.pos.0, clamped_y, &depth.to_string(), style);
            }
        }
    }

    // Nested overlays inherit the outer layer's tint — a modal that opens an
    // inner non-modal overlay still reads as part of the modal stack to the
    // human eye, which is what we want.
    if node.is_scrollable {
        if let Some(area) = visible_area(node, y_offset) {
            let inner = inner_area(node, area);
            buf.push_clip(inner);
            for child in &node.children {
                render_debug_overlay_inner(child, buf, depth.saturating_add(1), child_offset, tint);
            }
            buf.pop_clip();
        }
    } else {
        for child in &node.children {
            render_debug_overlay_inner(child, buf, depth.saturating_add(1), child_offset, tint);
        }
    }
}

/// Pick an outline color from the layer family + depth.
///
/// Each [`LayerTint`] gets a distinct base hue so layers stay visually
/// separable; depth then pulls the color toward white to keep nested
/// containers distinguishable inside the same family. Two depth bands
/// (`<= 1` = base, `<= 3` = lighter, `> 3` = lightest) give enough
/// gradation for typical 5–15-deep TUI trees without overshooting into
/// pure white where the hue would be lost.
fn debug_color_for_depth(tint: LayerTint, depth: u32) -> Color {
    let base = match tint {
        LayerTint::Base => Color::Rgb(64, 200, 64),
        LayerTint::Overlay => Color::Rgb(220, 80, 80),
        LayerTint::Modal => Color::Rgb(80, 140, 220),
    };
    match depth {
        0..=1 => base,
        2..=3 => base.lighten(0.25),
        _ => base.lighten(0.5),
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

fn render_inner(
    node: &LayoutNode,
    buf: &mut Buffer,
    y_offset: u32,
    parent_bg: Option<Color>,
    depth: usize,
) {
    // Hard upper bound — see `tree::MAX_LAYOUT_DEPTH`. Same rationale as the
    // build/compute/collect guards: surface a diagnostic panic instead of a
    // silent stack overflow if a synthetic tree slips past `build_children`.
    if depth > super::tree::MAX_LAYOUT_DEPTH {
        panic!(
            "layout tree depth exceeds {}: check for recursive container nesting",
            super::tree::MAX_LAYOUT_DEPTH
        );
    }
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
            // For Text nodes the constructors guarantee `text_data = Some`.
            // Read-only access through `text_data()` keeps us off the borrow
            // checker's bad side and lets the segments / content branches
            // share the same payload reference.
            let Some(td) = node.text_data() else {
                return;
            };
            if let Some(ref segs) = td.segments {
                if node.wrap {
                    let fallback;
                    let wrapped = if let Some(cached) = &td.cached_wrapped_segments {
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
            } else if let Some(ref text) = td.content {
                let mut style = node.style;
                if style.bg.is_none() {
                    style.bg = parent_bg;
                }
                if node.wrap {
                    let fallback;
                    let lines = if let Some(cached) = &td.cached_wrapped {
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
                        if let Some(cursor_offset) = td.cursor_offset {
                            let cursor_x = text
                                .chars()
                                .take(cursor_offset)
                                .map(|ch| UnicodeWidthChar::width(ch).unwrap_or(0) as u32)
                                .sum::<u32>();
                            buf.set_cursor_pos(draw_x.saturating_add(cursor_x), sy as u32);
                        }
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
                    render_inner(child, buf, child_offset, child_bg, depth + 1);
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
                    render_inner(child, buf, y_offset, child_bg, depth + 1);
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

    // Issue #162: skip the bottom corner writes entirely when `bottom_i` is
    // already off-screen. `buf.set_char` silently drops out-of-bounds writes,
    // so this is a perf-only guard — saves two redundant `set_char` calls per
    // scrolled container border per frame. `viewport_bottom` is exclusive
    // (matches `render_inner`'s convention).
    let viewport_bottom = i64::from(buf.area.y).saturating_add(i64::from(buf.area.height));
    if bottom_i < viewport_bottom {
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
    }

    if sides.top && top_i >= 0 {
        if let Some((title, title_style)) = &node.title {
            let mut ts = *title_style;
            if ts.bg.is_none() {
                ts.bg = inherit_bg;
            }
            let y = top_i as u32;
            let title_x = x.saturating_add(2);
            // The right corner sits at `right`. When the right side is drawn we
            // must keep that column intact, so the writable title area ends at
            // `right - 1`. With no right border we can use the full row.
            let title_right = if sides.right {
                right.saturating_sub(1)
            } else {
                right
            };
            if title_x <= title_right {
                let max_width = (title_right - title_x + 1) as usize;
                let mut trimmed = String::new();
                let mut col_used = 0usize;
                for ch in title.chars() {
                    let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
                    if col_used + cw > max_width {
                        break;
                    }
                    trimmed.push(ch);
                    col_used += cw;
                }
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn render_tracks_cursor_position_from_text_node() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 4));
        let mut node = LayoutNode::text(
            "ab▎cd".to_string(),
            Style::new(),
            0,
            Align::Start,
            (Some(2), false, false),
            Margin::default(),
            Constraints::default(),
        );
        node.pos = (3, 1);
        node.size = (5, 1);

        render(&node, &mut buf);

        assert_eq!(buf.cursor_pos(), Some((5, 1)));
    }

    #[test]
    fn border_title_cjk_truncates_within_box() {
        use crate::style::{Align, Border, Constraints, Justify, Margin, Padding};
        use unicode_width::UnicodeWidthStr;

        // Box: w=8, border=Single => inner w=6, title area = right - title_x + 1 = 5
        // "설정창" = 3 CJK × 2 cols = 6 display cols > 5 → must truncate to "설정" (4 cols)
        let mut root = LayoutNode::container(
            Direction::Row,
            super::tree::ContainerConfig {
                gap: 0,
                align: Align::Start,
                align_self: None,
                justify: Justify::Start,
                border: Some(Border::Single),
                border_sides: BorderSides::all(),
                border_style: Style::new(),
                bg_color: None,
                padding: Padding::default(),
                margin: Margin::default(),
                constraints: Constraints::default(),
                title: Some(("설정창".to_string(), Style::new())),
                grow: 0,
            },
        );

        let area = Rect::new(0, 0, 8, 4);
        super::flexbox::compute(&mut root, area);
        let mut buf = Buffer::empty(area);
        render(&root, &mut buf);

        // Collect all chars in top row (y=0)
        let top_row: String = (0..8u32)
            .map(|x| buf.get(x, 0).symbol.chars().next().unwrap_or(' '))
            .collect();

        // Right border char (x=7) must be '┐' (Single border corner), not a CJK char
        let right_border = buf.get(7, 0).symbol.chars().next().unwrap_or(' ');
        assert_eq!(
            right_border, '┐',
            "right border overwritten by CJK title overflow; top row: {top_row:?}"
        );

        // Lead glyphs of "설" (x=2) and "정" (x=4) should be present; "창" must NOT
        // appear because it would push past the writable title area.
        assert_eq!(buf.get(2, 0).symbol.chars().next(), Some('설'));
        assert_eq!(buf.get(4, 0).symbol.chars().next(), Some('정'));
        assert_ne!(buf.get(6, 0).symbol.chars().next(), Some('창'));
        let _ = UnicodeWidthStr::width(""); // keep import in scope
    }

    #[test]
    fn border_title_ascii_unchanged() {
        use crate::style::{Align, Border, Constraints, Justify, Margin, Padding};

        // Box: w=10, title="Hello" (5 ASCII chars = 5 cols), fits in title area (7 cols)
        let mut root = LayoutNode::container(
            Direction::Row,
            super::tree::ContainerConfig {
                gap: 0,
                align: Align::Start,
                align_self: None,
                justify: Justify::Start,
                border: Some(Border::Single),
                border_sides: BorderSides::all(),
                border_style: Style::new(),
                bg_color: None,
                padding: Padding::default(),
                margin: Margin::default(),
                constraints: Constraints::default(),
                title: Some(("Hello".to_string(), Style::new())),
                grow: 0,
            },
        );

        let area = Rect::new(0, 0, 10, 3);
        super::flexbox::compute(&mut root, area);
        let mut buf = Buffer::empty(area);
        render(&root, &mut buf);

        // Title chars at x=2..7 must spell "Hello"
        let rendered: String = (2..7u32)
            .map(|x| buf.get(x, 0).symbol.chars().next().unwrap_or(' '))
            .collect();
        assert_eq!(rendered, "Hello", "ASCII title should render unchanged");

        // Right border must be intact
        let right_border = buf.get(9, 0).symbol.chars().next().unwrap_or(' ');
        assert_eq!(right_border, '┐', "right border must not be overwritten");
    }
}
