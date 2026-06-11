use super::flexbox::inner_area;
use super::*;

pub(crate) fn render(node: &LayoutNode, buf: &mut Buffer) {
    render_inner(node, buf, 0, 0, None, 0);
    buf.clip_stack.clear();
    for overlay in &node.overlays {
        if overlay.modal {
            // Issue #228: use the modal's known rect (set by `flexbox::compute`
            // before render is called) to dim only the surrounding strips
            // rather than the entire buffer. For the common modal-with-margin
            // case this drops the write count from O(W*H) to O(perimeter).
            // Only fall back to the full-buffer scan when the modal has zero
            // size (degenerate case — should not happen post-compute, but the
            // fallback keeps the visual contract identical).
            let modal_rect = Rect::new(
                overlay.node.pos.0,
                overlay.node.pos.1,
                overlay.node.size.0,
                overlay.node.size.1,
            );
            if modal_rect.width == 0 || modal_rect.height == 0 {
                dim_entire_buffer(buf);
            } else {
                dim_buffer_around(buf, modal_rect);
            }
        }
        render_inner(&overlay.node, buf, 0, 0, None, 0);
    }
}

/// Apply the `DIM` modifier to every cell in `buf`.
///
/// Retained for the fallback path in [`render`] when a modal has zero size
/// (degenerate, but possible during transitions). The hot path uses
/// [`dim_buffer_around`] which scans only the four strips outside the modal.
///
/// Walks `buf.content` as one contiguous slice rather than per-cell
/// `get_mut` so the per-cell bounds assert is paid zero times and the
/// compiler can vectorize the modifier OR over the whole grid.
fn dim_entire_buffer(buf: &mut Buffer) {
    for cell in &mut buf.content {
        cell.style.modifiers |= crate::style::Modifiers::DIM;
    }
}

/// OR the `DIM` modifier into a contiguous run of cells `[start, end)` in
/// `buf.content`. `end` is clamped to the content length defensively, but
/// callers always pass in-bounds indices derived from `area`.
#[inline]
fn dim_cell_range(content: &mut [crate::cell::Cell], start: usize, end: usize) {
    let end = end.min(content.len());
    if start >= end {
        return;
    }
    for cell in &mut content[start..end] {
        cell.style.modifiers |= crate::style::Modifiers::DIM;
    }
}

/// Test-only re-export of [`dim_buffer_around`] for the perf alloc tests
/// and the `v020_perf_audit` demo. Not part of the stable API.
#[doc(hidden)]
pub(crate) fn __bench_dim_buffer_around(buf: &mut Buffer, modal_rect: Rect) {
    dim_buffer_around(buf, modal_rect)
}

/// Apply the `DIM` modifier only to cells outside `modal_rect` (issue #228).
///
/// Walks the four strips (top / bottom / left / right) bounded by the
/// intersection of `buf.area` and `modal_rect`. For a typical modal that
/// covers ~50% of the screen, this is roughly half as many writes as
/// [`dim_entire_buffer`]; for a small modal centered on a 200x60 terminal
/// the savings are dramatic. The visible output is identical because the
/// cells inside the modal are about to be painted by `render_inner`
/// immediately after this call returns.
fn dim_buffer_around(buf: &mut Buffer, modal_rect: Rect) {
    let area = buf.area;
    // Clip the modal rect to the buffer (safety: a layout bug that placed the
    // modal partly outside the screen must not cause us to skip dimming any
    // visible cell). Coordinates are exclusive on the right/bottom.
    let clip_x = modal_rect.x.max(area.x);
    let clip_y = modal_rect.y.max(area.y);
    let clip_right = modal_rect.right().min(area.right());
    let clip_bottom = modal_rect.bottom().min(area.bottom());

    // If the modal is fully off-screen, dim everything (matches the prior
    // behavior — the entire visible buffer is "background").
    if clip_right <= clip_x || clip_bottom <= clip_y {
        dim_entire_buffer(buf);
        return;
    }

    // Operate on `buf.content` directly as contiguous per-row slices: each
    // strip becomes a flat index range with no per-cell bounds assert, which
    // the compiler can vectorize. Row-major layout means a single row's
    // columns `[col_lo, col_hi)` are the contiguous range
    // `row_base + (col_lo - area.x) .. row_base + (col_hi - area.x)`.
    let width = area.width;
    let content = &mut buf.content;

    // Column offsets within a row, relative to the buffer's left edge
    // (`area.x`). The left strip starts at offset 0, so it has no explicit
    // `full_lo`.
    let full_hi = (area.right() - area.x) as usize;
    let left_hi = (clip_x - area.x) as usize;
    let right_lo = (clip_right - area.x) as usize;

    // Top strip: full-width rows above the modal. These rows are fully
    // contiguous from content index 0, so dim them as one span.
    if clip_y > area.y {
        let end = ((clip_y - area.y) * width) as usize;
        dim_cell_range(content, 0, end);
    }
    // Bottom strip: full-width rows below the modal — one contiguous span.
    if area.bottom() > clip_bottom {
        let start = ((clip_bottom - area.y) * width) as usize;
        let end = ((area.bottom() - area.y) * width) as usize;
        dim_cell_range(content, start, end);
    }
    // Left and right strips: only across the modal's row band. Each row's
    // left/right segment is contiguous within that row.
    for y in clip_y..clip_bottom {
        let row_base = ((y - area.y) * width) as usize;
        // Left segment: columns [area.x, clip_x) -> offsets [0, left_hi).
        dim_cell_range(content, row_base, row_base + left_hi);
        // Right segment: columns [clip_right, area.right()).
        dim_cell_range(content, row_base + right_lo, row_base + full_hi);
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
            render_debug_overlay_inner(child, buf, 0, 0, 0, LayerTint::Base);
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
            render_debug_overlay_inner(&overlay.node, buf, 0, 0, 0, tint);
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
    x_offset: u32,
    y_offset: u32,
    tint: LayerTint,
) {
    // #247: thread the x-axis offset alongside y so a horizontal scrollable's
    // children outline in their scrolled screen positions.
    let child_y_offset = if node.is_scrollable {
        y_offset.saturating_add(node.scroll_offset)
    } else {
        y_offset
    };
    let child_x_offset = if node.is_scrollable {
        x_offset.saturating_add(node.scroll_offset_x)
    } else {
        x_offset
    };

    if let NodeKind::Container(_) = node.kind {
        let sy = screen_y(node.pos.1, y_offset);
        let sx = screen_x(node.pos.0, x_offset);
        if sy + node.size.1 as i64 > 0 && sx + node.size.0 as i64 > 0 {
            let color = debug_color_for_depth(tint, depth);
            let style = Style::new().fg(color);
            let clamped_y = sy.max(0) as u32;
            let clamped_x = sx.max(0) as u32;
            draw_debug_border(clamped_x, clamped_y, node.size.0, node.size.1, buf, style);
            if sy >= 0 && sx >= 0 {
                buf.set_string(clamped_x, clamped_y, &depth.to_string(), style);
            }
        }
    }

    // Nested overlays inherit the outer layer's tint — a modal that opens an
    // inner non-modal overlay still reads as part of the modal stack to the
    // human eye, which is what we want.
    if node.is_scrollable {
        if let Some(area) = visible_area(node, x_offset, y_offset) {
            let inner = inner_area(node, area);
            buf.push_clip(inner);
            for child in &node.children {
                render_debug_overlay_inner(
                    child,
                    buf,
                    depth.saturating_add(1),
                    child_x_offset,
                    child_y_offset,
                    tint,
                );
            }
            buf.pop_clip();
        }
    } else {
        for child in &node.children {
            render_debug_overlay_inner(
                child,
                buf,
                depth.saturating_add(1),
                child_x_offset,
                child_y_offset,
                tint,
            );
        }
    }
}

/// Read-only snapshot of focus state threaded in from `FrameState` (issue #268).
///
/// The inspector overlay (Ctrl+F12) renders entirely from data the frame
/// pipeline already collected, so this struct only borrows — it allocates
/// nothing and triggers no new tree traversal beyond the single DFS in
/// [`find_focused_node`].
pub(crate) struct InspectorFocus<'a> {
    /// Index of the currently focused widget (settled value for the frame).
    pub focus_index: usize,
    /// Number of focusable widgets registered last frame (chain length).
    pub focus_count: usize,
    /// `name -> focus_index`, from `focus_name_map_prev`.
    pub names: &'a std::collections::HashMap<String, usize>,
    /// Live theme used for the panel chrome (surface / border / text).
    pub theme: &'a crate::style::Theme,
}

/// Render the devtools inspector overlay (issue #268).
///
/// Draws two panels on top of the frame: a *style panel* for the focused
/// widget (focus index/name, layout rect, resolved fg/bg, padding, and
/// constraints) and a *focus-chain panel* listing every focusable in order
/// with a `>` cursor on the current focus and the registered name for any
/// named entry. When nothing is focusable a single notice line is drawn.
///
/// This is a pure render-time overlay: it reuses the already-built layout
/// tree (one DFS to locate the focused node) and the focus snapshot threaded
/// in from `FrameState`. It is toggled independently of the F12 outline
/// overlay via Ctrl+F12.
pub(crate) fn render_inspector(root: &LayoutNode, buf: &mut Buffer, focus: &InspectorFocus<'_>) {
    if buf.area.width == 0 || buf.area.height == 0 {
        return;
    }
    if focus.focus_count == 0 {
        render_inspector_notice(buf, focus.theme, "[SLT Inspector] no focusable widgets");
        return;
    }
    // `focus_index` can exceed `focus_count` between frames (it wraps lazily
    // inside `register_focusable`); normalize so the lookup and the chain
    // cursor agree on which slot is current.
    let current = focus.focus_index % focus.focus_count;
    if let Some(node) = find_focused_node(root, current) {
        render_style_panel(node, buf, focus, current);
    }
    render_focus_chain_panel(buf, focus, current);
}

/// DFS for the layout node whose `focus_id` matches `focus_id` (issue #268).
///
/// Walks children first, then overlays, so a focusable nested inside a
/// tooltip/modal overlay is still resolved.
fn find_focused_node(node: &LayoutNode, focus_id: usize) -> Option<&LayoutNode> {
    if node.focus_id == Some(focus_id) {
        return Some(node);
    }
    node.children
        .iter()
        .find_map(|c| find_focused_node(c, focus_id))
        .or_else(|| {
            node.overlays
                .iter()
                .find_map(|o| find_focused_node(&o.node, focus_id))
        })
}

/// Format a [`Color`] human-readably for the inspector (issue #268).
///
/// Named colors print as their name (`Cyan`), RGB as `#rrggbb`, indexed as
/// `idx(N)`, and [`Color::Reset`] as `default`.
fn fmt_color(c: Color) -> String {
    match c {
        Color::Reset => "default".to_string(),
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        Color::Indexed(i) => format!("idx({i})"),
        named => format!("{named:?}"),
    }
}

/// Format the optional resolved color of a style slot, or `default` when unset.
fn fmt_opt_color(c: Option<Color>) -> String {
    c.map(fmt_color).unwrap_or_else(|| "default".to_string())
}

/// Draw a single themed notice line at the top-left (no-focusable case).
fn render_inspector_notice(buf: &mut Buffer, theme: &crate::style::Theme, msg: &str) {
    let style = Style::new().fg(theme.surface_text).bg(theme.surface);
    let width = buf.area.width as usize;
    let fill: String = " ".repeat(msg.chars().count().min(width));
    buf.set_string(buf.area.x, buf.area.y, &fill, style);
    buf.set_string(buf.area.x, buf.area.y, msg, style);
}

/// Render the focused-widget resolved-style panel at the top-left corner.
///
/// Each line is clamped to the buffer width (`set_string` clips the right
/// edge, mirroring `render_debug_status_bar`). The panel never grows past the
/// buffer height because it stops drawing once `y` reaches the bottom.
fn render_style_panel(
    node: &LayoutNode,
    buf: &mut Buffer,
    focus: &InspectorFocus<'_>,
    current: usize,
) {
    let theme = focus.theme;
    let text = Style::new().fg(theme.surface_text).bg(theme.surface);
    let head = Style::new().fg(theme.border).bg(theme.surface).bold();

    let name = focus
        .names
        .iter()
        .find_map(|(n, &i)| (i == current).then_some(n.as_str()))
        .unwrap_or("<unnamed>");
    let p = node.padding;

    let mut lines: Vec<(String, Style)> = Vec::with_capacity(7);
    lines.push(("[SLT Inspector] focused widget".to_string(), head));
    lines.push((format!("index: {current}  name: {name}"), text));
    lines.push((
        format!(
            "rect: {},{} {}x{}",
            node.pos.0, node.pos.1, node.size.0, node.size.1
        ),
        text,
    ));
    lines.push((format!("fg: {}", fmt_opt_color(node.style.fg)), text));
    lines.push((
        format!("bg: {}", fmt_opt_color(node.bg_color.or(node.style.bg))),
        text,
    ));
    lines.push((
        format!("padding: l{} r{} t{} b{}", p.left, p.right, p.top, p.bottom),
        text,
    ));
    lines.push((format!("constraints: {:?}", node.constraints), text));

    let max_w = lines
        .iter()
        .map(|(s, _)| s.chars().count())
        .max()
        .unwrap_or(0);
    let width = (max_w as u32).min(buf.area.width);
    let x = buf.area.x;
    for (i, (line, style)) in lines.iter().enumerate() {
        let y = buf.area.y + i as u32;
        if y >= buf.area.bottom() {
            break;
        }
        let fill: String = " ".repeat(width as usize);
        buf.set_string(x, y, &fill, *style);
        buf.set_string(x, y, line, *style);
    }
}

/// Render the ordered focus-chain panel at the top-right corner.
///
/// Lists indices `0..focus_count`, marks the current focus with a `>`
/// cursor, and appends the registered name for any named entry. Truncates to
/// the available height; the first line is a header.
fn render_focus_chain_panel(buf: &mut Buffer, focus: &InspectorFocus<'_>, current: usize) {
    let theme = focus.theme;
    let text = Style::new().fg(theme.surface_text).bg(theme.surface);
    let head = Style::new().fg(theme.border).bg(theme.surface).bold();
    let cursor = Style::new().fg(theme.border).bg(theme.surface).bold();

    let mut lines: Vec<(String, Style)> = Vec::with_capacity(focus.focus_count + 1);
    lines.push((
        format!("[SLT Inspector] focus chain ({})", focus.focus_count),
        head,
    ));
    // Truncate the list to what fits below the header.
    let max_rows = buf.area.height.saturating_sub(1) as usize;
    for idx in 0..focus.focus_count.min(max_rows) {
        let marker = if idx == current { ">" } else { " " };
        let name = focus
            .names
            .iter()
            .find_map(|(n, &i)| (i == idx).then_some(n.as_str()));
        let line = match name {
            Some(n) => format!("{marker} {idx}: {n}"),
            None => format!("{marker} {idx}"),
        };
        let style = if idx == current { cursor } else { text };
        lines.push((line, style));
    }

    let max_w = lines
        .iter()
        .map(|(s, _)| s.chars().count())
        .max()
        .unwrap_or(0) as u32;
    let panel_w = max_w.min(buf.area.width);
    // Right-align the panel; clamp so a narrow buffer keeps it on-screen.
    let x = buf.area.right().saturating_sub(panel_w).max(buf.area.x);
    for (i, (line, style)) in lines.iter().enumerate() {
        let y = buf.area.y + i as u32;
        if y >= buf.area.bottom() {
            break;
        }
        let fill: String = " ".repeat(panel_w as usize);
        buf.set_string(x, y, &fill, *style);
        buf.set_string(x, y, line, *style);
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

/// X-axis mirror of [`screen_y`] (#247): translate a layout x by the active
/// horizontal scroll offset.
fn screen_x(layout_x: u32, x_offset: u32) -> i64 {
    layout_x as i64 - x_offset as i64
}

/// Draw `text` at a possibly-negative screen x (#247).
///
/// When `screen_x >= 0` this is `buf.set_string(screen_x, y, text, style)`.
/// When `screen_x < 0` (the leading cells scrolled off the left of a
/// horizontal scrollable) the leading display columns are trimmed by grapheme
/// cluster and the visible remainder is drawn starting at column 0, so a wide
/// cluster never splits across the clip boundary. Returns the next x cursor
/// position after the drawn text (display columns from the original
/// `screen_x`), so segment runs can chain like the non-clipped path.
fn set_string_clipped_x(
    buf: &mut Buffer,
    screen_x: i64,
    y: u32,
    text: &str,
    style: Style,
    link: Option<&str>,
) -> i64 {
    let full_width = UnicodeWidthStr::width(text) as i64;
    if screen_x >= 0 {
        let sx = screen_x as u32;
        if let Some(url) = link {
            buf.set_string_linked(sx, y, text, style, url);
        } else {
            buf.set_string(sx, y, text, style);
        }
        return screen_x + full_width;
    }
    // screen_x < 0: skip leading display columns until the cursor reaches 0.
    let skip = (-screen_x) as u32;
    let mut consumed: u32 = 0;
    let mut byte_start = text.len();
    for (idx, g) in text.grapheme_indices(true) {
        if consumed >= skip {
            byte_start = idx;
            break;
        }
        consumed += UnicodeWidthStr::width(g) as u32;
    }
    if byte_start >= text.len() {
        // Entire run scrolled off the left edge.
        return screen_x + full_width;
    }
    let visible = &text[byte_start..];
    if let Some(url) = link {
        buf.set_string_linked(0, y, visible, style, url);
    } else {
        buf.set_string(0, y, visible, style);
    }
    screen_x + full_width
}

fn visible_area(node: &LayoutNode, x_offset: u32, y_offset: u32) -> Option<Rect> {
    let sy = screen_y(node.pos.1, y_offset);
    let bottom = sy + node.size.1 as i64;
    let sx = screen_x(node.pos.0, x_offset);
    let right = sx + node.size.0 as i64;
    if bottom <= 0 || right <= 0 || node.size.0 == 0 || node.size.1 == 0 {
        return None;
    }
    let clamped_y = sy.max(0) as u32;
    let clamped_h = (bottom as u32).saturating_sub(clamped_y);
    let clamped_x = sx.max(0) as u32;
    let clamped_w = (right as u32).saturating_sub(clamped_x);
    Some(Rect::new(clamped_x, clamped_y, clamped_w, clamped_h))
}

fn render_inner(
    node: &LayoutNode,
    buf: &mut Buffer,
    x_offset: u32,
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
    // #247: x positions are translated by the active horizontal scroll offset,
    // mirroring `sy`. `draw_x` (computed below for text) starts from `sx`.
    let sx = screen_x(node.pos.0, x_offset);
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
                        // #247: `sx` carries the horizontal scroll offset; the
                        // per-segment cursor advances in screen space so a run
                        // straddling the left clip edge is trimmed correctly.
                        let mut x = sx;
                        for (text, style) in line_segs {
                            let mut s = *style;
                            if s.bg.is_none() {
                                s.bg = parent_bg;
                            }
                            x = set_string_clipped_x(buf, x, line_y as u32, text, s, None);
                        }
                    }
                } else {
                    if sy < 0 {
                        return;
                    }
                    let mut x = sx;
                    for (text, style) in segs {
                        let mut s = *style;
                        if s.bg.is_none() {
                            s.bg = parent_bg;
                        }
                        x = set_string_clipped_x(buf, x, sy as u32, text, s, None);
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
                        let x_align = if text_width < node.size.0 {
                            match node.align {
                                Align::Start => 0,
                                Align::Center => (node.size.0 - text_width) / 2,
                                Align::End => node.size.0 - text_width,
                            }
                        } else {
                            0
                        };
                        set_string_clipped_x(
                            buf,
                            sx + i64::from(x_align),
                            line_y as u32,
                            line,
                            style,
                            None,
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
                        let x_align = if trunc_width < node.size.0 {
                            match node.align {
                                Align::Start => 0,
                                Align::Center => (node.size.0 - trunc_width) / 2,
                                Align::End => node.size.0 - trunc_width,
                            }
                        } else {
                            0
                        };
                        set_string_clipped_x(
                            buf,
                            sx + i64::from(x_align),
                            sy as u32,
                            &truncated,
                            style,
                            node.link_url.as_deref(),
                        );
                    } else {
                        let x_align = if text_width < node.size.0 {
                            match node.align {
                                Align::Start => 0,
                                Align::Center => (node.size.0 - text_width) / 2,
                                Align::End => node.size.0 - text_width,
                            }
                        } else {
                            0
                        };
                        let draw_x = sx + i64::from(x_align);
                        if let Some(cursor_offset) = td.cursor_offset {
                            let cursor_x = text
                                .chars()
                                .take(cursor_offset)
                                .map(|ch| UnicodeWidthChar::width(ch).unwrap_or(0) as u32)
                                .sum::<u32>();
                            let cursor_screen_x = draw_x + i64::from(cursor_x);
                            if cursor_screen_x >= 0 {
                                buf.set_cursor_pos(cursor_screen_x as u32, sy as u32);
                            }
                        }
                        set_string_clipped_x(
                            buf,
                            draw_x,
                            sy as u32,
                            text,
                            style,
                            node.link_url.as_deref(),
                        );
                    }
                }
            }
        }
        NodeKind::Spacer | NodeKind::RawDraw(_) => {}
        NodeKind::Container(_) => {
            if let Some(color) = node.bg_color
                && let Some(area) = visible_area(node, x_offset, y_offset)
            {
                let fill_style = Style::new().bg(color);
                for y in area.y..area.bottom() {
                    for x in area.x..area.right() {
                        buf.set_string(x, y, " ", fill_style);
                    }
                }
            }
            let child_bg = node.bg_color.or(parent_bg);
            render_container_border(node, buf, x_offset, y_offset, child_bg);
            if node.is_scrollable {
                let Some(area) = visible_area(node, x_offset, y_offset) else {
                    return;
                };
                let inner = inner_area(node, area);
                // #247: a scrollable node moves its children on exactly one
                // axis. `scroll_offset` is non-zero only for a column,
                // `scroll_offset_x` only for a row, so adding both is correct
                // for either orientation.
                let child_y_offset = y_offset.saturating_add(node.scroll_offset);
                let child_x_offset = x_offset.saturating_add(node.scroll_offset_x);
                let render_y_start = inner.y as i64;
                let render_y_end = inner.bottom() as i64;
                let render_x_start = inner.x as i64;
                let render_x_end = inner.right() as i64;
                buf.push_clip(inner);
                for child in &node.children {
                    let child_top = child.pos.1 as i64 - child_y_offset as i64;
                    let child_bottom = child_top + child.size.1 as i64;
                    if child_bottom <= render_y_start || child_top >= render_y_end {
                        continue;
                    }
                    let child_left = child.pos.0 as i64 - child_x_offset as i64;
                    let child_right = child_left + child.size.0 as i64;
                    if child_right <= render_x_start || child_left >= render_x_end {
                        continue;
                    }
                    render_inner(
                        child,
                        buf,
                        child_x_offset,
                        child_y_offset,
                        child_bg,
                        depth + 1,
                    );
                }
                buf.pop_clip();
                render_scroll_indicators(node, inner, buf, child_bg);
            } else {
                let Some(area) = visible_area(node, x_offset, y_offset) else {
                    return;
                };
                let clip = inner_area(node, area);
                buf.push_clip(clip);
                for child in &node.children {
                    render_inner(child, buf, x_offset, y_offset, child_bg, depth + 1);
                }
                buf.pop_clip();
            }
        }
    }
}

fn render_container_border(
    node: &LayoutNode,
    buf: &mut Buffer,
    x_offset: u32,
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
    // #247: the left/right edges are translated by the horizontal scroll
    // offset, mirroring `top_i` / `bottom_i`. When `x_offset == 0`, `left_i`
    // equals `node.pos.0` and every write is byte-identical to the pre-#247
    // path. `set_char_at` guards the negative-column case so a border that
    // scrolled past the left edge is dropped rather than wrapping the `u32`.
    let left_i = screen_x(node.pos.0, x_offset);
    let right_i = left_i + w as i64 - 1;
    if right_i < 0 {
        return;
    }

    let set_char_at = |buf: &mut Buffer, col: i64, y: u32, ch: char| {
        if col >= 0 {
            buf.set_char(col as u32, y, ch, style);
        }
    };

    let h_start = left_i.max(0) as u32;
    let h_end = right_i as u32;
    if sides.top && top_i >= 0 {
        let y = top_i as u32;
        for xx in h_start..=h_end {
            buf.set_char(xx, y, chars.h, style);
        }
    }
    if sides.bottom {
        let y = bottom_i as u32;
        for xx in h_start..=h_end {
            buf.set_char(xx, y, chars.h, style);
        }
    }
    if sides.left {
        let vert_start = top_i.max(0) as u32;
        let vert_end = bottom_i as u32;
        for yy in vert_start..=vert_end {
            set_char_at(buf, left_i, yy, chars.v);
        }
    }
    if sides.right {
        let vert_start = top_i.max(0) as u32;
        let vert_end = bottom_i as u32;
        for yy in vert_start..=vert_end {
            set_char_at(buf, right_i, yy, chars.v);
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
            set_char_at(buf, left_i, y, ch);
        }

        let tr = match (sides.top, sides.right) {
            (true, true) => Some(chars.tr),
            (true, false) => Some(chars.h),
            (false, true) => Some(chars.v),
            (false, false) => None,
        };
        if let Some(ch) = tr {
            set_char_at(buf, right_i, y, ch);
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
            set_char_at(buf, left_i, y, ch);
        }

        let br = match (sides.bottom, sides.right) {
            (true, true) => Some(chars.br),
            (true, false) => Some(chars.h),
            (false, true) => Some(chars.v),
            (false, false) => None,
        };
        if let Some(ch) = br {
            set_char_at(buf, right_i, y, ch);
        }
    }

    if sides.top
        && top_i >= 0
        && let Some((title, title_style)) = &node.title
    {
        let mut ts = *title_style;
        if ts.bg.is_none() {
            ts.bg = inherit_bg;
        }
        let y = top_i as u32;
        let title_x = left_i + 2;
        // The right corner sits at `right_i`. When the right side is drawn
        // we must keep that column intact, so the writable title area ends
        // at `right_i - 1`. With no right border we can use the full row.
        let title_right = if sides.right { right_i - 1 } else { right_i };
        if title_x <= title_right && title_right >= 0 {
            // `max_width` is the title window width measured from `title_x`
            // (which may be negative when scrolled left); the clipped writer
            // trims any leading columns past the left edge (#247).
            let max_width = (title_right - title_x + 1).max(0) as usize;
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
            set_string_clipped_x(buf, title_x, y, &trimmed, ts, None);
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

    // Vertical indicators (scrollable column). Unchanged from pre-#247.
    let indicator_x = inner.right() - 1;
    if node.scroll_offset > 0 {
        buf.set_char(indicator_x, inner.y, '▲', style);
    }
    if node.scroll_offset.saturating_add(inner.height) < node.content_height {
        buf.set_char(indicator_x, inner.bottom() - 1, '▼', style);
    }
    // Horizontal indicators (scrollable row, #247): drawn on the bottom edge,
    // the x-axis mirror of the right-edge vertical arrows.
    let indicator_y = inner.bottom() - 1;
    if node.scroll_offset_x > 0 {
        buf.set_char(inner.x, indicator_y, '◀', style);
    }
    if node.scroll_offset_x.saturating_add(inner.width) < node.content_width {
        buf.set_char(inner.right() - 1, indicator_y, '▶', style);
    }
}

pub(super) fn truncate_with_ellipsis(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if max_width == 1 {
        return "\u{2026}".to_string();
    }
    let target = max_width - 1;
    let mut result = String::new();
    let mut width = 0;
    // Stop on grapheme-cluster boundaries: a cluster (ZWJ flag, family emoji,
    // Indic / Thai syllable) that would overflow `target` is dropped whole
    // before the ellipsis, never half-emitted.
    for g in text.graphemes(true) {
        let ch_width = UnicodeWidthStr::width(g);
        if width + ch_width > target {
            break;
        }
        result.push_str(g);
        width += ch_width;
    }
    result.push('\u{2026}');
    result
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::tree::default_container_config;
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

    // ─── Issue #268: devtools inspector ───

    #[test]
    fn inspector_color_formatting() {
        assert_eq!(fmt_color(Color::Rgb(255, 0, 0)), "#ff0000");
        assert_eq!(fmt_color(Color::Rgb(18, 52, 86)), "#123456");
        assert_eq!(fmt_color(Color::Cyan), "Cyan");
        assert_eq!(fmt_color(Color::Reset), "default");
        assert_eq!(fmt_color(Color::Indexed(8)), "idx(8)");
        // Optional slot: `None` reads as `default`, `Some` delegates.
        assert_eq!(fmt_opt_color(None), "default");
        assert_eq!(fmt_opt_color(Some(Color::Red)), "Red");
    }

    #[test]
    fn find_focused_node_walks_overlays() {
        // A focusable nested inside an overlay must still resolve, since the
        // DFS walks `node.children` then `node.overlays`.
        let mut root = LayoutNode::container(Direction::Column, default_container_config());

        // Base child with a different focus id.
        let mut base = LayoutNode::container(Direction::Column, default_container_config());
        base.focus_id = Some(0);
        root.children.push(base);

        // Overlay carrying the focusable we want (id 1).
        let mut overlay_child =
            LayoutNode::container(Direction::Column, default_container_config());
        overlay_child.focus_id = Some(1);
        let mut overlay_root = LayoutNode::container(Direction::Column, default_container_config());
        overlay_root.children.push(overlay_child);
        root.overlays.push(super::tree::OverlayLayer {
            node: overlay_root,
            modal: false,
        });

        let found = find_focused_node(&root, 1).expect("overlay focusable must resolve");
        assert_eq!(found.focus_id, Some(1));
        // Base id still resolves via the children branch.
        assert_eq!(
            find_focused_node(&root, 0).and_then(|n| n.focus_id),
            Some(0)
        );
        // Missing id returns None (no panic).
        assert!(find_focused_node(&root, 9).is_none());
    }

    #[test]
    fn inspector_no_focusables_renders_notice() {
        let theme = crate::style::Theme::dark();
        let names = std::collections::HashMap::new();
        let focus = InspectorFocus {
            focus_index: 0,
            focus_count: 0,
            names: &names,
            theme: &theme,
        };
        let root = LayoutNode::container(Direction::Column, default_container_config());
        let mut buf = Buffer::empty(Rect::new(0, 0, 60, 10));
        render_inspector(&root, &mut buf, &focus);

        let mut top = String::new();
        for x in 0..60 {
            top.push_str(&buf.get(x, 0).symbol);
        }
        assert!(
            top.contains("no focusable widgets"),
            "expected no-focusable notice; got {top:?}"
        );
    }

    #[test]
    fn inspector_style_panel_shows_focused_widget() {
        use crate::style::Padding;
        let theme = crate::style::Theme::dark();
        let names = std::collections::HashMap::new();

        let mut root = LayoutNode::container(Direction::Column, default_container_config());
        let mut focused = LayoutNode::container(Direction::Column, default_container_config());
        focused.focus_id = Some(0);
        focused.pos = (4, 2);
        focused.size = (12, 3);
        focused.padding = Padding {
            top: 1,
            right: 2,
            bottom: 3,
            left: 4,
        };
        focused.style.fg = Some(Color::Cyan);
        focused.bg_color = Some(Color::Rgb(255, 0, 0));
        root.children.push(focused);

        let focus = InspectorFocus {
            focus_index: 0,
            focus_count: 1,
            names: &names,
            theme: &theme,
        };
        // Wide buffer so the left style panel and the right-aligned chain
        // panel do not overlap on shared rows.
        let mut buf = Buffer::empty(Rect::new(0, 0, 120, 10));
        render_inspector(&root, &mut buf, &focus);

        // Collect the whole buffer into lines for assertions.
        let mut text = String::new();
        for y in 0..10 {
            for x in 0..120 {
                text.push_str(&buf.get(x, y).symbol);
            }
            text.push('\n');
        }
        assert!(
            text.contains("focused widget"),
            "panel header; got {text:?}"
        );
        assert!(text.contains("index: 0"), "focus index; got {text:?}");
        assert!(text.contains("<unnamed>"), "unnamed marker; got {text:?}");
        assert!(text.contains("rect: 4,2 12x3"), "rect line; got {text:?}");
        assert!(text.contains("fg: Cyan"), "fg color; got {text:?}");
        assert!(text.contains("bg: #ff0000"), "bg color; got {text:?}");
        assert!(
            text.contains("padding: l4 r2 t1 b3"),
            "padding line; got {text:?}"
        );
        assert!(
            text.contains("constraints:"),
            "constraints line; got {text:?}"
        );
        // Focus-chain panel header is present too.
        assert!(
            text.contains("focus chain (1)"),
            "chain header; got {text:?}"
        );
    }

    #[test]
    fn inspector_focus_chain_marks_current() {
        let theme = crate::style::Theme::dark();
        let names = std::collections::HashMap::new();

        let mut root = LayoutNode::container(Direction::Column, default_container_config());
        for id in 0..3 {
            let mut n = LayoutNode::container(Direction::Column, default_container_config());
            n.focus_id = Some(id);
            n.pos = (0, id as u32);
            n.size = (5, 1);
            root.children.push(n);
        }

        let focus = InspectorFocus {
            focus_index: 1,
            focus_count: 3,
            names: &names,
            theme: &theme,
        };
        // Wide buffer so the left style panel and the right-aligned chain panel
        // never share columns; this isolates the chain cursor for the scan.
        let mut buf = Buffer::empty(Rect::new(0, 0, 120, 10));
        render_inspector(&root, &mut buf, &focus);

        // The chain panel is right-aligned, so each chain row's text lives in
        // the right half of the buffer. Scan the right half only and record
        // which `idx` carries the `>` cursor marker.
        let split = 80u32; // right of any left-panel content
        let mut cursor_indices = Vec::new();
        for y in 0..10 {
            let mut right = String::new();
            for x in split..120 {
                right.push_str(&buf.get(x, y).symbol);
            }
            let trimmed = right.trim();
            if trimmed.contains("chain") || trimmed.is_empty() {
                continue; // header / blank rows
            }
            if let Some(rest) = trimmed.strip_prefix("> ") {
                // "> 1" → capture the index after the cursor marker.
                if let Ok(idx) = rest.trim().parse::<usize>() {
                    cursor_indices.push(idx);
                }
            }
        }
        assert_eq!(
            cursor_indices,
            vec![1],
            "exactly index 1 must carry the `>` cursor (not 0/2); got {cursor_indices:?}"
        );
    }

    #[test]
    fn inspector_named_focus_in_chain() {
        let theme = crate::style::Theme::dark();
        let mut names = std::collections::HashMap::new();
        names.insert("search".to_string(), 1usize);

        let mut root = LayoutNode::container(Direction::Column, default_container_config());
        for id in 0..2 {
            let mut n = LayoutNode::container(Direction::Column, default_container_config());
            n.focus_id = Some(id);
            n.size = (5, 1);
            root.children.push(n);
        }

        let focus = InspectorFocus {
            focus_index: 1,
            focus_count: 2,
            names: &names,
            theme: &theme,
        };
        // Wide buffer so the left style panel and right chain panel are clear.
        let mut buf = Buffer::empty(Rect::new(0, 0, 120, 10));
        render_inspector(&root, &mut buf, &focus);

        let mut text = String::new();
        for y in 0..10 {
            for x in 0..120 {
                text.push_str(&buf.get(x, y).symbol);
            }
            text.push('\n');
        }
        assert!(
            text.contains("search"),
            "named focus chain entry must show its name; got {text:?}"
        );
        // The style panel name line resolves the same map.
        assert!(
            text.contains("name: search"),
            "style panel must show focused widget's name; got {text:?}"
        );
    }

    #[test]
    fn inspector_clamps_to_tiny_buffer() {
        // A 1x1 buffer must not panic and must not write out of bounds.
        let theme = crate::style::Theme::dark();
        let names = std::collections::HashMap::new();
        let mut root = LayoutNode::container(Direction::Column, default_container_config());
        let mut n = LayoutNode::container(Direction::Column, default_container_config());
        n.focus_id = Some(0);
        n.size = (1, 1);
        root.children.push(n);

        let focus = InspectorFocus {
            focus_index: 0,
            focus_count: 1,
            names: &names,
            theme: &theme,
        };
        let mut buf = Buffer::empty(Rect::new(0, 0, 1, 1));
        render_inspector(&root, &mut buf, &focus);
    }
}
