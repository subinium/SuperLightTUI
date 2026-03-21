use super::*;

pub(crate) fn compute(node: &mut LayoutNode, area: Rect) {
    if let Some(pct) = node.constraints.width_pct {
        let resolved = (area.width as u64 * pct.min(100) as u64 / 100) as u32;
        node.constraints.min_width = Some(resolved);
        node.constraints.max_width = Some(resolved);
        node.constraints.width_pct = None;
    }
    if let Some(pct) = node.constraints.height_pct {
        let resolved = (area.height as u64 * pct.min(100) as u64 / 100) as u32;
        node.constraints.min_height = Some(resolved);
        node.constraints.max_height = Some(resolved);
        node.constraints.height_pct = None;
    }

    node.pos = (area.x, area.y);
    node.size = (
        area.width.clamp(
            node.constraints.min_width.unwrap_or(0),
            node.constraints.max_width.unwrap_or(u32::MAX),
        ),
        area.height.clamp(
            node.constraints.min_height.unwrap_or(0),
            node.constraints.max_height.unwrap_or(u32::MAX),
        ),
    );

    if matches!(node.kind, NodeKind::Text) && node.wrap {
        let lines = node.ensure_wrapped_for_width(area.width);
        node.size = (area.width, lines);
    } else {
        node.cached_wrap_width = None;
        node.cached_wrapped = None;
        node.cached_wrapped_segments = None;
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
            );
            node.content_height = 0;
        }
        NodeKind::Container(Direction::Column) => {
            let viewport_area = inner_area(
                node,
                Rect::new(node.pos.0, node.pos.1, node.size.0, node.size.1),
            );
            if node.is_scrollable {
                let saved_grows: Vec<u16> = node.children.iter().map(|c| c.grow).collect();
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
                    layout_column(node, virtual_area);
                } else {
                    for (child, &grow) in node.children.iter_mut().zip(saved_grows.iter()) {
                        child.grow = grow;
                    }
                    layout_column(node, viewport_area);
                }
                node.content_height = scroll_content_height(node, viewport_area.y);
            } else {
                layout_column(node, viewport_area);
                node.content_height = 0;
            }
        }
    }

    for overlay in &mut node.overlays {
        let width = overlay.node.min_width().min(area.width);
        let height = overlay.node.min_height_for_width(width).min(area.height);
        let x = area.x.saturating_add(area.width.saturating_sub(width) / 2);
        let y = area
            .y
            .saturating_add(area.height.saturating_sub(height) / 2);
        compute(&mut overlay.node, Rect::new(x, y, width, height));
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

fn layout_row(node: &mut LayoutNode, area: Rect) {
    if node.children.is_empty() {
        return;
    }

    for child in &mut node.children {
        if let Some(pct) = child.constraints.width_pct {
            let resolved = (area.width as u64 * pct.min(100) as u64 / 100) as u32;
            child.constraints.min_width = Some(resolved);
            child.constraints.max_width = Some(resolved);
            child.constraints.width_pct = None;
        }
        if let Some(pct) = child.constraints.height_pct {
            let resolved = (area.height as u64 * pct.min(100) as u64 / 100) as u32;
            child.constraints.min_height = Some(resolved);
            child.constraints.max_height = Some(resolved);
            child.constraints.height_pct = None;
        }
    }

    let n = node.children.len() as u32;
    let total_gaps = (n - 1) * node.gap;
    let available = area.width.saturating_sub(total_gaps);
    let min_widths: Vec<u32> = node
        .children
        .iter()
        .map(|child| child.min_width())
        .collect();

    let mut total_grow: u32 = 0;
    let mut fixed_width: u32 = 0;
    for (child, &min_width) in node.children.iter().zip(min_widths.iter()) {
        if child.grow > 0 {
            total_grow += child.grow as u32;
        } else {
            fixed_width += min_width;
        }
    }

    let mut flex_space = available.saturating_sub(fixed_width);
    let mut remaining_grow = total_grow;

    let mut child_widths: Vec<u32> = Vec::with_capacity(node.children.len());
    for (i, child) in node.children.iter().enumerate() {
        let w = if child.grow > 0 && total_grow > 0 {
            let share = if remaining_grow == 0 {
                0
            } else {
                flex_space * child.grow as u32 / remaining_grow
            };
            flex_space = flex_space.saturating_sub(share);
            remaining_grow = remaining_grow.saturating_sub(child.grow as u32);
            share
        } else {
            min_widths[i].min(available)
        };
        child_widths.push(w);
    }

    let total_children_width: u32 = child_widths.iter().sum();
    let remaining = area.width.saturating_sub(total_children_width);
    let (start_offset, inter_gap) = justify_offsets(node.justify, remaining, n, node.gap);

    let mut x = area.x + start_offset;
    for (i, child) in node.children.iter_mut().enumerate() {
        let w = child_widths[i];
        let child_cross_align = child.align_self.unwrap_or(node.align);
        let child_outer_h = match child_cross_align {
            Align::Start => area.height,
            _ => child.min_height_for_width(w).min(area.height),
        };
        let child_x = x.saturating_add(child.margin.left);
        let child_y = area.y.saturating_add(child.margin.top);
        let child_w = w.saturating_sub(child.margin.horizontal());
        let child_h = child_outer_h.saturating_sub(child.margin.vertical());
        compute(child, Rect::new(child_x, child_y, child_w, child_h));
        let child_total_h = child.size.1.saturating_add(child.margin.vertical());
        let y_offset = match child_cross_align {
            Align::Start => 0,
            Align::Center => area.height.saturating_sub(child_total_h) / 2,
            Align::End => area.height.saturating_sub(child_total_h),
        };
        child.pos.1 = child.pos.1.saturating_add(y_offset);
        x += w + inter_gap;
    }
}

fn layout_column(node: &mut LayoutNode, area: Rect) {
    if node.children.is_empty() {
        return;
    }

    for child in &mut node.children {
        if let Some(pct) = child.constraints.width_pct {
            let resolved = (area.width as u64 * pct.min(100) as u64 / 100) as u32;
            child.constraints.min_width = Some(resolved);
            child.constraints.max_width = Some(resolved);
            child.constraints.width_pct = None;
        }
        if let Some(pct) = child.constraints.height_pct {
            let resolved = (area.height as u64 * pct.min(100) as u64 / 100) as u32;
            child.constraints.min_height = Some(resolved);
            child.constraints.max_height = Some(resolved);
            child.constraints.height_pct = None;
        }
    }

    let n = node.children.len() as u32;
    let total_gaps = (n - 1) * node.gap;
    let available = area.height.saturating_sub(total_gaps);
    let min_heights: Vec<u32> = node
        .children
        .iter_mut()
        .map(|child| child.min_height_for_width(area.width))
        .collect();

    let mut total_grow: u32 = 0;
    let mut fixed_height: u32 = 0;
    for (child, &min_height) in node.children.iter().zip(min_heights.iter()) {
        if child.grow > 0 {
            total_grow += child.grow as u32;
        } else {
            fixed_height += min_height;
        }
    }

    let mut flex_space = available.saturating_sub(fixed_height);
    let mut remaining_grow = total_grow;

    let mut child_heights: Vec<u32> = Vec::with_capacity(node.children.len());
    for (i, child) in node.children.iter().enumerate() {
        let h = if child.grow > 0 && total_grow > 0 {
            let share = if remaining_grow == 0 {
                0
            } else {
                flex_space * child.grow as u32 / remaining_grow
            };
            flex_space = flex_space.saturating_sub(share);
            remaining_grow = remaining_grow.saturating_sub(child.grow as u32);
            share
        } else {
            min_heights[i].min(available)
        };
        child_heights.push(h);
    }

    let total_children_height: u32 = child_heights.iter().sum();
    let remaining = area.height.saturating_sub(total_children_height);
    let (start_offset, inter_gap) = justify_offsets(node.justify, remaining, n, node.gap);

    let mut y = area.y + start_offset;
    for (i, child) in node.children.iter_mut().enumerate() {
        let h = child_heights[i];
        let child_cross_align = child.align_self.unwrap_or(node.align);
        let child_outer_w = match child_cross_align {
            Align::Start => area.width,
            _ => child.min_width().min(area.width),
        };
        let child_x = area.x.saturating_add(child.margin.left);
        let child_y = y.saturating_add(child.margin.top);
        let child_w = child_outer_w.saturating_sub(child.margin.horizontal());
        let child_h = h.saturating_sub(child.margin.vertical());
        compute(child, Rect::new(child_x, child_y, child_w, child_h));
        let child_total_w = child.size.0.saturating_add(child.margin.horizontal());
        let x_offset = match child_cross_align {
            Align::Start => 0,
            Align::Center => area.width.saturating_sub(child_total_w) / 2,
            Align::End => area.width.saturating_sub(child_total_w),
        };
        child.pos.0 = child.pos.0.saturating_add(x_offset);
        y += h + inter_gap;
    }
}
