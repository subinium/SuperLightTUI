use super::*;

/// Inline capacity for per-call layout scratch buffers.
///
/// Most flex containers have a small number of children. When the child count
/// is `<= INLINE_CAP`, the scratch values live in the stack-allocated array
/// and no heap allocation occurs. Larger containers fall back to a `Vec` on
/// the overflow path, which keeps worst-case behavior identical to the old
/// `Vec::with_capacity(n)` path.
const INLINE_CAP: usize = 16;

/// Small-vec–style `u32` scratch used by `layout_row` / `layout_column`.
///
/// Each call to these functions creates four of these on the stack (min sizes
/// and allocated sizes for the main axis — plus an extra pair when a column
/// is scrollable). When a container has `<= INLINE_CAP` children the entire
/// scratch lives on the stack; otherwise the overflow spills to the heap.
///
/// Because every buffer is created inside a single function body and read
/// within that same body (before any recursive `compute` call for child nodes
/// happens on a *child*, not a peer), recursion does not reuse scratch
/// storage — each recursion frame has its own independent `U32Stack` values.
struct U32Stack {
    inline: [u32; INLINE_CAP],
    len: usize,
    overflow: Option<Vec<u32>>,
}

impl U32Stack {
    #[inline]
    fn with_capacity(n: usize) -> Self {
        Self {
            inline: [0; INLINE_CAP],
            len: 0,
            overflow: if n > INLINE_CAP {
                Some(Vec::with_capacity(n))
            } else {
                None
            },
        }
    }

    #[inline]
    fn push(&mut self, v: u32) {
        match &mut self.overflow {
            Some(over) => over.push(v),
            None => {
                if self.len < INLINE_CAP {
                    self.inline[self.len] = v;
                    self.len += 1;
                } else {
                    // Promotion: spill inline values + this one into a fresh Vec.
                    let mut heap = Vec::with_capacity(self.len + 1);
                    heap.extend_from_slice(&self.inline[..self.len]);
                    heap.push(v);
                    self.overflow = Some(heap);
                }
            }
        }
    }

    #[inline]
    fn get(&self, i: usize) -> u32 {
        match &self.overflow {
            Some(over) => over[i],
            None => self.inline[i],
        }
    }

    #[inline]
    fn iter(&self) -> U32StackIter<'_> {
        U32StackIter {
            stack: self,
            idx: 0,
            len: match &self.overflow {
                Some(over) => over.len(),
                None => self.len,
            },
        }
    }
}

struct U32StackIter<'a> {
    stack: &'a U32Stack,
    idx: usize,
    len: usize,
}

impl Iterator for U32StackIter<'_> {
    type Item = u32;
    #[inline]
    fn next(&mut self) -> Option<u32> {
        if self.idx >= self.len {
            return None;
        }
        let v = self.stack.get(self.idx);
        self.idx += 1;
        Some(v)
    }
}

/// Resolve `Pct` and `Ratio` width/height variants against a parent area
/// into concrete `Fixed` widths/heights.
///
/// Single match per axis (the v0.20 successor to the previous three
/// `if let Some(pct)` blocks). [`WidthSpec::Auto`] / [`WidthSpec::Fixed`] /
/// [`WidthSpec::MinMax`] pass through unchanged. A `Ratio` with `den == 0`
/// is treated as no constraint (`Auto`).
#[inline]
pub(crate) fn resolve_axis_specs(c: &mut Constraints, area: Rect) {
    match c.width {
        WidthSpec::Pct(pct) => {
            let resolved = (area.width as u64 * pct.min(100) as u64 / 100) as u32;
            c.width = WidthSpec::Fixed(resolved);
        }
        WidthSpec::Ratio(num, den) => {
            let resolved = if den == 0 {
                area.width
            } else {
                (area.width as u64 * num as u64 / den as u64) as u32
            };
            c.width = WidthSpec::Fixed(resolved);
        }
        WidthSpec::Auto | WidthSpec::Fixed(_) | WidthSpec::MinMax { .. } => {}
    }
    match c.height {
        HeightSpec::Pct(pct) => {
            let resolved = (area.height as u64 * pct.min(100) as u64 / 100) as u32;
            c.height = HeightSpec::Fixed(resolved);
        }
        HeightSpec::Ratio(num, den) => {
            let resolved = if den == 0 {
                area.height
            } else {
                (area.height as u64 * num as u64 / den as u64) as u32
            };
            c.height = HeightSpec::Fixed(resolved);
        }
        HeightSpec::Auto | HeightSpec::Fixed(_) | HeightSpec::MinMax { .. } => {}
    }
}

pub(crate) fn compute(node: &mut LayoutNode, area: Rect) {
    compute_inner(node, area, 0);
}

fn compute_inner(node: &mut LayoutNode, area: Rect, depth: usize) {
    // Hard upper bound — see `tree::MAX_LAYOUT_DEPTH`. `build_children`
    // already enforces this at construction; this guard catches synthetic
    // trees built directly in tests or via future programmatic paths so we
    // panic with a diagnostic instead of overflowing the stack.
    if depth > super::tree::MAX_LAYOUT_DEPTH {
        panic!(
            "layout tree depth exceeds {}: check for recursive container nesting",
            super::tree::MAX_LAYOUT_DEPTH
        );
    }
    compute_body(node, area, depth);
}

fn compute_body(node: &mut LayoutNode, area: Rect, depth: usize) {
    resolve_axis_specs(&mut node.constraints, area);

    let (min_w, max_w) = (
        node.constraints.min_width().unwrap_or(0),
        node.constraints.max_width().unwrap_or(u32::MAX),
    );
    let (min_h, max_h) = (
        node.constraints.min_height().unwrap_or(0),
        node.constraints.max_height().unwrap_or(u32::MAX),
    );
    node.pos = (area.x, area.y);
    node.size = (
        area.width.clamp(min_w, max_w),
        area.height.clamp(min_h, max_h),
    );

    if matches!(node.kind, NodeKind::Text) && node.wrap {
        let lines = node.ensure_wrapped_for_width(area.width);
        node.size = (area.width, lines);
    } else if let Some(td) = node.text_data.as_deref_mut() {
        // Only text nodes carry the wrap cache. Non-text variants have
        // `text_data = None` and have nothing to invalidate here.
        td.cached_wrap_width = None;
        td.cached_wrapped = None;
        td.cached_wrapped_segments = None;
    }

    match node.kind {
        NodeKind::Text | NodeKind::Spacer | NodeKind::RawDraw(_) => {}
        NodeKind::Container(Direction::Row) => {
            layout_row(
                node,
                inner_area(
                    node,
                    Rect::new(node.pos.0, node.pos.1, node.size.0, node.size.1),
                ),
                depth,
            );
            node.content_height = 0;
        }
        NodeKind::Container(Direction::Column) => {
            let viewport_area = inner_area(
                node,
                Rect::new(node.pos.0, node.pos.1, node.size.0, node.size.1),
            );
            if node.is_scrollable {
                // Inline-first scratch for saving `grow` values per child. Most
                // scrollable containers have few children; fall back to a Vec
                // only for wide ones. This avoids the per-frame allocation.
                let mut saved_inline: [u16; INLINE_CAP] = [0; INLINE_CAP];
                let mut saved_overflow: Option<Vec<u16>> = None;
                let child_count = node.children.len();
                if child_count > INLINE_CAP {
                    let mut v = Vec::with_capacity(child_count);
                    for c in &node.children {
                        v.push(c.grow);
                    }
                    saved_overflow = Some(v);
                } else {
                    for (i, c) in node.children.iter().enumerate() {
                        saved_inline[i] = c.grow;
                    }
                }
                for child in &mut node.children {
                    child.grow = 0;
                }
                let total_gaps = if node.children.is_empty() {
                    0
                } else {
                    (node.children.len() as u32 - 1) * node.gap
                };
                let natural_height: u32 = node
                    .children
                    .iter_mut()
                    .map(|c| c.min_height_for_width(viewport_area.width))
                    .sum::<u32>()
                    + total_gaps;

                if natural_height > viewport_area.height {
                    let virtual_area = Rect::new(
                        viewport_area.x,
                        viewport_area.y,
                        viewport_area.width,
                        natural_height,
                    );
                    layout_column(node, virtual_area, depth);
                } else {
                    match &saved_overflow {
                        Some(over) => {
                            for (child, &grow) in node.children.iter_mut().zip(over.iter()) {
                                child.grow = grow;
                            }
                        }
                        None => {
                            for (i, child) in node.children.iter_mut().enumerate() {
                                child.grow = saved_inline[i];
                            }
                        }
                    }
                    layout_column(node, viewport_area, depth);
                }
                node.content_height = scroll_content_height(node, viewport_area.y);
            } else {
                layout_column(node, viewport_area, depth);
                node.content_height = 0;
            }
        }
    }

    for overlay in &mut node.overlays {
        // Issue #200 / #201: When the user's overlay content uses `.grow(>=1)`
        // (or sets explicit `.w(area_w).h(area_h)` via constraints producing a
        // child whose min size already equals the area), the wrapper must
        // take the full area so flexbox `justify` / `align` inside the inner
        // container has space to push against. Previously the wrapper was
        // always shrunk to content min-size and centered, so:
        //   - `overlay(|ui| ui.container().grow(1).align(End).justify(End)…)`
        //     never reached the bottom-right (no slack to push into).
        //   - `overlay(|ui| ui.container().grow(1).draw(…))` rendered nothing
        //     (raw_draw with no constraints had min-size 0×0, so the wrapper
        //     became 0×0 and `lib.rs` skipped the empty-rect callback).
        //
        // Heuristic: if any direct child requests grow, fill the full area.
        // Otherwise preserve the historic shrink-and-center behavior so plain
        // `overlay(|ui| ui.bordered(...).p(1).col(...))` still floats in the
        // middle.
        let any_grow = overlay.node.children.iter().any(|c| c.grow > 0);
        let (x, y, width, height) = if any_grow {
            (area.x, area.y, area.width, area.height)
        } else {
            let width = overlay.node.min_width().min(area.width);
            let height = overlay.node.min_height_for_width(width).min(area.height);
            let x = area.x.saturating_add(area.width.saturating_sub(width) / 2);
            let y = area
                .y
                .saturating_add(area.height.saturating_sub(height) / 2);
            (x, y, width, height)
        };
        compute_inner(&mut overlay.node, Rect::new(x, y, width, height), depth + 1);
    }
}

fn scroll_content_height(node: &LayoutNode, inner_y: u32) -> u32 {
    let Some(max_bottom) = node
        .children
        .iter()
        .map(|child| {
            child
                .pos
                .1
                .saturating_add(child.size.1)
                .saturating_add(child.margin.bottom)
        })
        .max()
    else {
        return 0;
    };

    max_bottom.saturating_sub(inner_y)
}

fn justify_offsets(justify: Justify, remaining: u32, n: u32, gap: u32) -> (u32, u32) {
    if n <= 1 {
        let start = match justify {
            Justify::Center => remaining / 2,
            Justify::End => remaining,
            _ => 0,
        };
        return (start, gap);
    }

    match justify {
        Justify::Start => (0, gap),
        Justify::Center => (remaining.saturating_sub((n - 1) * gap) / 2, gap),
        Justify::End => (remaining.saturating_sub((n - 1) * gap), gap),
        Justify::SpaceBetween => (0, remaining / (n - 1)),
        Justify::SpaceAround => {
            let slot = remaining / n;
            (slot / 2, slot)
        }
        Justify::SpaceEvenly => {
            let slot = remaining / (n + 1);
            (slot, slot)
        }
    }
}

pub(super) fn inner_area(node: &LayoutNode, area: Rect) -> Rect {
    let x = area.x + node.border_left_inset() + node.padding.left;
    let y = area.y + node.border_top_inset() + node.padding.top;
    let width = area
        .width
        .saturating_sub(node.border_left_inset() + node.border_right_inset())
        .saturating_sub(node.padding.horizontal());
    let height = area
        .height
        .saturating_sub(node.border_top_inset() + node.border_bottom_inset())
        .saturating_sub(node.padding.vertical());

    Rect::new(x, y, width, height)
}

fn layout_row(node: &mut LayoutNode, area: Rect, depth: usize) {
    if node.children.is_empty() {
        return;
    }

    for child in &mut node.children {
        resolve_axis_specs(&mut child.constraints, area);
    }

    let n = node.children.len() as u32;
    let total_gaps = (n - 1) * node.gap;
    let available = area.width.saturating_sub(total_gaps);
    let child_count = node.children.len();
    let mut min_widths = U32Stack::with_capacity(child_count);
    for child in &node.children {
        min_widths.push(child.min_width());
    }

    let mut total_grow: u32 = 0;
    let mut fixed_width: u32 = 0;
    for (child, min_width) in node.children.iter().zip(min_widths.iter()) {
        if child.grow > 0 {
            total_grow += child.grow as u32;
        } else {
            fixed_width += min_width;
        }
    }

    let mut flex_space = available.saturating_sub(fixed_width);
    let mut remaining_grow = total_grow;

    let mut child_widths = U32Stack::with_capacity(child_count);
    for (i, child) in node.children.iter().enumerate() {
        let w = if child.grow > 0 && total_grow > 0 {
            let share = (flex_space * child.grow as u32)
                .checked_div(remaining_grow)
                .unwrap_or(0);
            flex_space = flex_space.saturating_sub(share);
            remaining_grow = remaining_grow.saturating_sub(child.grow as u32);
            share
        } else {
            min_widths.get(i).min(available)
        };
        child_widths.push(w);
    }

    let total_children_width: u32 = child_widths.iter().sum();
    let remaining = area.width.saturating_sub(total_children_width);
    let (start_offset, inter_gap) = justify_offsets(node.justify, remaining, n, node.gap);

    let mut x = area.x + start_offset;
    for (i, child) in node.children.iter_mut().enumerate() {
        let w = child_widths.get(i);
        let child_cross_align = child.align_self.unwrap_or(node.align);
        let child_outer_h = match child_cross_align {
            Align::Start => area.height,
            _ => child.min_height_for_width(w).min(area.height),
        };
        let child_x = x.saturating_add(child.margin.left);
        let child_y = area.y.saturating_add(child.margin.top);
        let child_w = w.saturating_sub(child.margin.horizontal());
        let child_h = child_outer_h.saturating_sub(child.margin.vertical());
        compute_inner(
            child,
            Rect::new(child_x, child_y, child_w, child_h),
            depth + 1,
        );
        let child_total_h = child.size.1.saturating_add(child.margin.vertical());
        let y_offset = match child_cross_align {
            Align::Start => 0,
            Align::Center => area.height.saturating_sub(child_total_h) / 2,
            Align::End => area.height.saturating_sub(child_total_h),
        };
        child.pos.1 = child.pos.1.saturating_add(y_offset);
        let effective_w = child.size.0.saturating_add(child.margin.horizontal());
        x += effective_w + inter_gap;
    }
}

fn layout_column(node: &mut LayoutNode, area: Rect, depth: usize) {
    if node.children.is_empty() {
        return;
    }

    for child in &mut node.children {
        resolve_axis_specs(&mut child.constraints, area);
    }

    let n = node.children.len() as u32;
    let total_gaps = (n - 1) * node.gap;
    let available = area.height.saturating_sub(total_gaps);
    let child_count = node.children.len();
    let mut min_heights = U32Stack::with_capacity(child_count);
    for child in &mut node.children {
        min_heights.push(child.min_height_for_width(area.width));
    }

    let mut total_grow: u32 = 0;
    let mut fixed_height: u32 = 0;
    for (child, min_height) in node.children.iter().zip(min_heights.iter()) {
        if child.grow > 0 {
            total_grow += child.grow as u32;
        } else {
            fixed_height += min_height;
        }
    }

    let mut flex_space = available.saturating_sub(fixed_height);
    let mut remaining_grow = total_grow;

    let mut child_heights = U32Stack::with_capacity(child_count);
    for (i, child) in node.children.iter().enumerate() {
        let h = if child.grow > 0 && total_grow > 0 {
            let share = (flex_space * child.grow as u32)
                .checked_div(remaining_grow)
                .unwrap_or(0);
            flex_space = flex_space.saturating_sub(share);
            remaining_grow = remaining_grow.saturating_sub(child.grow as u32);
            share
        } else {
            min_heights.get(i).min(available)
        };
        child_heights.push(h);
    }

    let total_children_height: u32 = child_heights.iter().sum();
    let remaining = area.height.saturating_sub(total_children_height);
    let (start_offset, inter_gap) = justify_offsets(node.justify, remaining, n, node.gap);

    let mut y = area.y + start_offset;
    for (i, child) in node.children.iter_mut().enumerate() {
        let h = child_heights.get(i);
        let child_cross_align = child.align_self.unwrap_or(node.align);
        let child_outer_w = match child_cross_align {
            Align::Start => area.width,
            _ => child.min_width().min(area.width),
        };
        let child_x = area.x.saturating_add(child.margin.left);
        let child_y = y.saturating_add(child.margin.top);
        let child_w = child_outer_w.saturating_sub(child.margin.horizontal());
        let child_h = h.saturating_sub(child.margin.vertical());
        compute_inner(
            child,
            Rect::new(child_x, child_y, child_w, child_h),
            depth + 1,
        );
        let child_total_w = child.size.0.saturating_add(child.margin.horizontal());
        let x_offset = match child_cross_align {
            Align::Start => 0,
            Align::Center => area.width.saturating_sub(child_total_w) / 2,
            Align::End => area.width.saturating_sub(child_total_w),
        };
        child.pos.0 = child.pos.0.saturating_add(x_offset);
        let effective_h = child.size.1.saturating_add(child.margin.vertical());
        y += effective_h + inter_gap;
    }
}
