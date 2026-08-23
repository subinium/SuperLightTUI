use super::*;

/// Inline capacity for per-call layout scratch buffers.
///
/// Most flex containers have a small number of children. When the child count
/// is `<= INLINE_CAP`, the scratch values live in the stack-allocated array
/// and no heap allocation occurs. Larger containers fall back to a `Vec` on
/// the overflow path, which keeps worst-case behavior identical to the old
/// `Vec::with_capacity(n)` path.
const INLINE_CAP: usize = 16;

#[inline]
fn clamp_i128_to_u32(value: i128) -> u32 {
    value.clamp(0, i128::from(u32::MAX)) as u32
}

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
            let viewport_area = inner_area(
                node,
                Rect::new(node.pos.0, node.pos.1, node.size.0, node.size.1),
            );
            if node.is_scrollable {
                // Extracted to keep `compute_body`'s stack frame small: the
                // `[u16; INLINE_CAP]` save-buffer lives in the callee, not in
                // every recursion frame (matters for deep non-scrollable trees,
                // see `tree::MAX_LAYOUT_DEPTH`).
                layout_scrollable_row(node, viewport_area, depth);
            } else {
                layout_row(node, viewport_area, depth);
                node.content_width = 0;
            }
            node.content_height = 0;
        }
        NodeKind::Container(Direction::Column) => {
            let viewport_area = inner_area(
                node,
                Rect::new(node.pos.0, node.pos.1, node.size.0, node.size.1),
            );
            if node.is_scrollable {
                // Extracted (see `layout_scrollable_row`): the per-child grow
                // save-buffer stays out of `compute_body`'s recursion frame.
                layout_scrollable_column(node, viewport_area, depth);
            } else {
                layout_column(node, viewport_area, depth);
                node.content_height = 0;
            }
            // A scrollable column scrolls vertically only — its x-axis content
            // width stays 0 so `ScrollState` never reports horizontal overflow
            // for it (#247).
            node.content_width = 0;
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

/// Lay out a scrollable `Direction::Column` into `viewport_area`.
///
/// Extracted from `compute_body` so its `[u16; INLINE_CAP]` grow-save buffer
/// does not inflate the recursion frame for deep non-scrollable trees. The
/// body is byte-identical to the pre-extraction inline branch: zero children's
/// `grow`, measure the natural content height, lay out into an over-tall
/// virtual area on overflow (else restore `grow` and lay out into the
/// viewport), then record `content_height`.
#[inline(never)]
fn layout_scrollable_column(node: &mut LayoutNode, viewport_area: Rect, depth: usize) {
    // Inline-first scratch for saving `grow` values per child. Most scrollable
    // containers have few children; fall back to a Vec only for wide ones.
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
    let total_gaps: i64 = if node.children.is_empty() {
        0
    } else {
        (node.children.len() as i64 - 1) * node.gap as i64
    };
    let children_height = node.children.iter_mut().fold(0u64, |total, child| {
        total.saturating_add(u64::from(child.min_height_for_width(viewport_area.width)))
    });
    // `total_gaps` may be negative for overlap (#222); clamp at 0.
    let natural_height = clamp_i128_to_u32(i128::from(children_height) + i128::from(total_gaps));

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
}

/// Lay out a scrollable `Direction::Row` into `viewport_area` — the x-axis
/// mirror of [`layout_scrollable_column`] (#247).
///
/// Zeroes children's `grow` (so they keep their natural width rather than
/// fighting the overflow measurement), sums child `min_width` + gaps into the
/// natural content width, lays out into an over-wide virtual area when it
/// overflows the viewport (else restores `grow` and lays out into the
/// viewport), then records `content_width`. Extracted for the same stack-frame
/// reason as the column path.
#[inline(never)]
fn layout_scrollable_row(node: &mut LayoutNode, viewport_area: Rect, depth: usize) {
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
    let total_gaps: i64 = if node.children.is_empty() {
        0
    } else {
        (node.children.len() as i64 - 1) * node.gap as i64
    };
    let children_width = node.children.iter().fold(0u64, |total, child| {
        total.saturating_add(u64::from(child.min_width()))
    });
    // `total_gaps` may be negative for overlap (#222); clamp at 0.
    let natural_width = clamp_i128_to_u32(i128::from(children_width) + i128::from(total_gaps));

    if natural_width > viewport_area.width {
        let virtual_area = Rect::new(
            viewport_area.x,
            viewport_area.y,
            natural_width,
            viewport_area.height,
        );
        layout_row(node, virtual_area, depth);
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
        layout_row(node, viewport_area, depth);
    }
    node.content_width = scroll_content_width(node, viewport_area.x);
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

/// X-axis mirror of [`scroll_content_height`] (#247).
///
/// Returns the rightmost child edge (position + width + right margin) relative
/// to the scrollable row's inner-x origin, i.e. the total content width the
/// viewport scrolls across. `0` when the row has no children.
fn scroll_content_width(node: &LayoutNode, inner_x: u32) -> u32 {
    let Some(max_right) = node
        .children
        .iter()
        .map(|child| {
            child
                .pos
                .0
                .saturating_add(child.size.0)
                .saturating_add(child.margin.right)
        })
        .max()
    else {
        return 0;
    };

    max_right.saturating_sub(inner_x)
}

/// Compute the leading offset and inter-child gap for a flex line.
///
/// `gap` is signed (#222): a negative value overlaps adjacent children.
/// The returned start offset is unsigned (clamped at 0); the returned
/// inter-child gap stays signed so the caller advances positions with
/// `saturating_add_signed`.
fn justify_offsets(justify: Justify, remaining: u32, n: u32, gap: i32) -> (u32, i32) {
    if n <= 1 {
        let start = match justify {
            Justify::Center => remaining / 2,
            Justify::End => remaining,
            _ => 0,
        };
        return (start, gap);
    }

    // For Center/End, `(n - 1) * gap` may be negative when children overlap;
    // clamp the consumed gap span at 0 so the unsigned `remaining` math holds.
    let total_gap_span = clamp_i128_to_u32(i128::from(n - 1) * i128::from(gap));
    match justify {
        Justify::Start => (0, gap),
        Justify::Center => (remaining.saturating_sub(total_gap_span) / 2, gap),
        Justify::End => (remaining.saturating_sub(total_gap_span), gap),
        Justify::SpaceBetween => (0, (remaining / (n - 1)) as i32),
        Justify::SpaceAround => {
            let slot = remaining / n;
            (slot / 2, slot as i32)
        }
        Justify::SpaceEvenly => {
            let slot = remaining / n.saturating_add(1);
            (slot, slot as i32)
        }
    }
}

pub(super) fn inner_area(node: &LayoutNode, area: Rect) -> Rect {
    let x = area
        .x
        .saturating_add(node.border_left_inset())
        .saturating_add(node.padding.left);
    let y = area
        .y
        .saturating_add(node.border_top_inset())
        .saturating_add(node.padding.top);
    let width = area
        .width
        .saturating_sub(
            node.border_left_inset()
                .saturating_add(node.border_right_inset()),
        )
        .saturating_sub(node.padding.left.saturating_add(node.padding.right));
    let height = area
        .height
        .saturating_sub(
            node.border_top_inset()
                .saturating_add(node.border_bottom_inset()),
        )
        .saturating_sub(node.padding.top.saturating_add(node.padding.bottom));

    Rect::new(x, y, width, height)
}

fn layout_row(node: &mut LayoutNode, area: Rect, depth: usize) {
    if node.children.is_empty() {
        return;
    }

    // Opt-in flex-wrap (#258): a row whose children overflow `area.width`
    // flows the overflow onto subsequent lines. Default-off rows take the
    // single-line path below, byte-identical to pre-#258 behavior.
    if node.wrap_children {
        layout_row_wrapped(node, area, depth);
    } else {
        layout_row_line(
            &mut node.children,
            area,
            node.gap,
            node.justify,
            node.align,
            depth,
        );
    }
}

/// Lay out one horizontal line of children within `area`.
///
/// This is the single-line row body extracted from `layout_row` (#258) so
/// both the non-wrapping path and each line of the wrapping path share the
/// same basis → grow → shrink → justify → align math. `flex_basis` (#258)
/// feeds the resolution as the per-child base size in place of `min_width`
/// when set; `shrink` (#161) and the signed `gap` (#222) are preserved.
fn layout_row_line(
    children: &mut [LayoutNode],
    area: Rect,
    gap: i32,
    justify: Justify,
    align: Align,
    depth: usize,
) {
    if children.is_empty() {
        return;
    }

    for child in children.iter_mut() {
        resolve_axis_specs(&mut child.constraints, area);
        // Resolving `Pct`/`Ratio` -> `Fixed` is the only mid-frame mutation
        // that can change a child's intrinsic min sizes, so drop any value
        // memoized during an ancestor's measurement pass (which saw the
        // pre-resolution constraint). For `Auto`/`Fixed`/`MinMax` this is a
        // no-op resolution and a cheap cache clear.
        child.invalidate_size_cache();
    }

    let n = u32::try_from(children.len()).unwrap_or(u32::MAX);
    // Signed gap (#222): `total_gaps` may be negative when children overlap,
    // which gives them *more* available space. `i64` intermediate avoids any
    // overflow before clamping the result back into the `u32` width budget.
    let total_gaps = ((n - 1) as i64) * gap as i64;
    let available = clamp_i128_to_u32(i128::from(area.width) - i128::from(total_gaps));
    let child_count = children.len();
    // Base main-axis size feeding the resolution: `flex_basis` when set
    // (#258), else the child's intrinsic `min_width` (historic behavior).
    let mut base_widths = U32Stack::with_capacity(child_count);
    for child in children.iter() {
        base_widths.push(child.flex_basis().unwrap_or_else(|| child.min_width()));
    }

    let mut total_grow: u32 = 0;
    let mut fixed_width: u32 = 0;
    // Sum of grow children's *explicit* flex-basis (#258). Grow children with
    // no explicit basis reserve nothing and keep the historic pure-share
    // behavior (they do not floor at their min_width). Children with an
    // explicit basis reserve it as a floor: `grow` then distributes the space
    // *above* the summed bases, matching CSS `flex: <grow> 0 <basis>`.
    let mut grow_basis_total: u32 = 0;
    for (child, base_width) in children.iter().zip(base_widths.iter()) {
        if child.grow > 0 {
            total_grow = total_grow.saturating_add(u32::from(child.grow));
            grow_basis_total = grow_basis_total.saturating_add(child.flex_basis().unwrap_or(0));
        } else {
            fixed_width = fixed_width.saturating_add(base_width);
        }
    }

    // Opt-in proportional flex-shrink (#161). When the row's fixed children
    // overflow the available width, scale shrink-flagged children by
    // `available / fixed_width` so the row no longer overflows. Children
    // without `.shrink()` keep the historic overflow-by-design behavior.
    //
    // Only `grow == 0` children participate (grow children consume
    // leftover, not fixed). The scale factor follows the spec in #161:
    // `min(available, shrink_total) / fixed_width`. With #258 the shrink base
    // is the flex-basis (else min_width), so shrink scales from `basis`.
    let shrink_scale: Option<f64> = if fixed_width > available && available > 0 {
        let shrink_total: u32 = children
            .iter()
            .zip(base_widths.iter())
            .filter(|(c, _)| c.shrink && c.grow == 0)
            .map(|(_, bw)| bw)
            .fold(0u32, u32::saturating_add);
        if shrink_total > 0 {
            let numerator = (available as f64).min(shrink_total as f64);
            Some(numerator / fixed_width as f64)
        } else {
            None
        }
    } else {
        None
    };

    // Free space distributed by `grow` is what remains after both the fixed
    // children's bases *and* the grow children's explicit bases are reserved.
    let mut flex_space = available
        .saturating_sub(fixed_width)
        .saturating_sub(grow_basis_total);
    let mut remaining_grow = total_grow;

    let mut child_widths = U32Stack::with_capacity(child_count);
    for (i, child) in children.iter().enumerate() {
        let w = if child.grow > 0 && total_grow > 0 {
            let share = u32::try_from(
                (u64::from(flex_space) * u64::from(child.grow)) / u64::from(remaining_grow),
            )
            .unwrap_or(u32::MAX);
            flex_space = flex_space.saturating_sub(share);
            remaining_grow = remaining_grow.saturating_sub(child.grow as u32);
            // Grow children grow *from* their explicit basis (#258); with no
            // explicit basis this adds 0, preserving the historic pure-share.
            child.flex_basis().unwrap_or(0).saturating_add(share)
        } else {
            let raw = base_widths.get(i).min(available);
            match shrink_scale {
                Some(scale) if child.shrink => ((raw as f64) * scale).floor() as u32,
                _ => raw,
            }
        };
        child_widths.push(w);
    }

    let total_children_width = child_widths.iter().fold(0u32, u32::saturating_add);
    let remaining = area.width.saturating_sub(total_children_width);
    let (start_offset, inter_gap) = justify_offsets(justify, remaining, n, gap);

    let mut x = area.x.saturating_add(start_offset);
    for (i, child) in children.iter_mut().enumerate() {
        let w = child_widths.get(i);
        let child_cross_align = child.align_self.unwrap_or(align);
        let child_outer_h = match child_cross_align {
            Align::Start => area.height,
            _ => child.min_height_for_width(w).min(area.height),
        };
        let child_x = x.saturating_add(child.margin.left);
        let child_y = area.y.saturating_add(child.margin.top);
        let child_w = w.saturating_sub(child.margin_horizontal());
        let child_h = child_outer_h.saturating_sub(child.margin_vertical());
        compute_inner(
            child,
            Rect::new(child_x, child_y, child_w, child_h),
            depth + 1,
        );
        let child_total_h = child.size.1.saturating_add(child.margin_vertical());
        let y_offset = match child_cross_align {
            Align::Start => 0,
            Align::Center => area.height.saturating_sub(child_total_h) / 2,
            Align::End => area.height.saturating_sub(child_total_h),
        };
        child.pos.1 = child.pos.1.saturating_add(y_offset);
        let effective_w = child.size.0.saturating_add(child.margin_horizontal());
        // `inter_gap` is signed (#222); advance with a saturating signed add so
        // an overlap (negative gap) larger than the cursor never wraps `u32`.
        x = x
            .saturating_add(effective_w)
            .saturating_add_signed(inter_gap);
    }
}

/// Lay out a wrapping row (#258): partition children into lines that each
/// fit `area.width`, then lay out each line with [`layout_row_line`] while
/// advancing a cross-axis cursor.
///
/// Partitioning is greedy on the main-axis base size (`flex_basis` else
/// `min_width`) plus the within-line `gap`. A child wider than `area.width`
/// occupies its own line (clipped, as a single-line row would clip). Lines
/// stack top-to-bottom; the between-line gap is `cross_gap` (resolves to
/// `row_gap` else `gap` at builder time). Each line's height is the tallest
/// child in that line; lines are laid out against a one-line-tall band so
/// per-line grow children fill the line height, not the whole row.
fn layout_row_wrapped(node: &mut LayoutNode, area: Rect, depth: usize) {
    let gap = node.gap;
    let cross_gap = node.cross_gap;
    let justify = node.justify;
    let align = node.align;

    // Greedy line partition by base width. Each entry is the half-open child
    // index range `[start, end)` plus the line's tallest child height.
    let mut lines: Vec<(usize, usize, u32)> = Vec::new();
    let mut line_start = 0usize;
    let mut cur_width: i64 = 0;
    let mut cur_height: u32 = 0;
    let mut has_child = false;
    for (i, child) in node.children.iter_mut().enumerate() {
        let base = child.flex_basis().unwrap_or_else(|| child.min_width());
        let child_h = if child.wrap {
            child.min_height_for_width(base.min(area.width))
        } else {
            child.min_height()
        };
        if has_child {
            let prospective = cur_width + gap as i64 + base as i64;
            if prospective > area.width as i64 {
                lines.push((line_start, i, cur_height));
                line_start = i;
                cur_width = base as i64;
                cur_height = child_h;
            } else {
                cur_width = prospective;
                cur_height = cur_height.max(child_h);
            }
        } else {
            cur_width = base as i64;
            cur_height = child_h;
            has_child = true;
        }
    }
    if has_child {
        lines.push((line_start, node.children.len(), cur_height));
    }

    // Lay out each line in its own one-line-tall band, advancing `y` by the
    // line's height plus the cross-axis gap. The cross-axis gap is signed
    // (#222 overlap semantics carry over); a saturating signed advance keeps
    // an overlap from wrapping `u32`.
    let mut y = area.y;
    for (start, end, line_height) in lines {
        let line_area = Rect::new(area.x, y, area.width, line_height);
        layout_row_line(
            &mut node.children[start..end],
            line_area,
            gap,
            justify,
            align,
            depth,
        );
        y = y
            .saturating_add(line_height)
            .saturating_add_signed(cross_gap);
    }
}

fn layout_column(node: &mut LayoutNode, area: Rect, depth: usize) {
    if node.children.is_empty() {
        return;
    }

    for child in &mut node.children {
        resolve_axis_specs(&mut child.constraints, area);
        // See `layout_row_line`: discard any intrinsic-size memo cached before
        // this child's `Pct`/`Ratio` constraints were resolved.
        child.invalidate_size_cache();
    }

    let n = u32::try_from(node.children.len()).unwrap_or(u32::MAX);
    // Signed gap (#222) — see `layout_row` for the `i64`-clamp rationale.
    let total_gaps = ((n - 1) as i64) * node.gap as i64;
    let available = clamp_i128_to_u32(i128::from(area.height) - i128::from(total_gaps));
    let child_count = node.children.len();
    let mut min_heights = U32Stack::with_capacity(child_count);
    for child in &mut node.children {
        min_heights.push(child.min_height_for_width(area.width));
    }

    let mut total_grow: u32 = 0;
    let mut fixed_height: u32 = 0;
    for (child, min_height) in node.children.iter().zip(min_heights.iter()) {
        if child.grow > 0 {
            total_grow = total_grow.saturating_add(u32::from(child.grow));
        } else {
            fixed_height = fixed_height.saturating_add(min_height);
        }
    }

    // Opt-in proportional flex-shrink (#161). Cross-axis sibling of the
    // `layout_row` block above — same scale formula on the height axis.
    let shrink_scale: Option<f64> = if fixed_height > available && available > 0 {
        let shrink_total: u32 = node
            .children
            .iter()
            .zip(min_heights.iter())
            .filter(|(c, _)| c.shrink && c.grow == 0)
            .map(|(_, mh)| mh)
            .fold(0u32, u32::saturating_add);
        if shrink_total > 0 {
            let numerator = (available as f64).min(shrink_total as f64);
            Some(numerator / fixed_height as f64)
        } else {
            None
        }
    } else {
        None
    };

    let mut flex_space = available.saturating_sub(fixed_height);
    let mut remaining_grow = total_grow;

    let mut child_heights = U32Stack::with_capacity(child_count);
    for (i, child) in node.children.iter().enumerate() {
        let h = if child.grow > 0 && total_grow > 0 {
            let share = u32::try_from(
                (u64::from(flex_space) * u64::from(child.grow)) / u64::from(remaining_grow),
            )
            .unwrap_or(u32::MAX);
            flex_space = flex_space.saturating_sub(share);
            remaining_grow = remaining_grow.saturating_sub(child.grow as u32);
            share
        } else {
            let raw = min_heights.get(i).min(available);
            match shrink_scale {
                Some(scale) if child.shrink => ((raw as f64) * scale).floor() as u32,
                _ => raw,
            }
        };
        child_heights.push(h);
    }

    let total_children_height = child_heights.iter().fold(0u32, u32::saturating_add);
    let remaining = area.height.saturating_sub(total_children_height);
    let (start_offset, inter_gap) = justify_offsets(node.justify, remaining, n, node.gap);

    let mut y = area.y.saturating_add(start_offset);
    for (i, child) in node.children.iter_mut().enumerate() {
        let h = child_heights.get(i);
        let child_cross_align = child.align_self.unwrap_or(node.align);
        let child_outer_w = match child_cross_align {
            Align::Start => area.width,
            _ => child.min_width().min(area.width),
        };
        let child_x = area.x.saturating_add(child.margin.left);
        let child_y = y.saturating_add(child.margin.top);
        let child_w = child_outer_w.saturating_sub(child.margin_horizontal());
        let child_h = h.saturating_sub(child.margin_vertical());
        compute_inner(
            child,
            Rect::new(child_x, child_y, child_w, child_h),
            depth + 1,
        );
        let child_total_w = child.size.0.saturating_add(child.margin_horizontal());
        let x_offset = match child_cross_align {
            Align::Start => 0,
            Align::Center => area.width.saturating_sub(child_total_w) / 2,
            Align::End => area.width.saturating_sub(child_total_w),
        };
        child.pos.0 = child.pos.0.saturating_add(x_offset);
        let effective_h = child.size.1.saturating_add(child.margin_vertical());
        // Signed `inter_gap` (#222) — saturating signed advance, see `layout_row`.
        y = y
            .saturating_add(effective_h)
            .saturating_add_signed(inter_gap);
    }
}
