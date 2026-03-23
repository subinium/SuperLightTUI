use super::flexbox::inner_area;
use super::*;

#[derive(Default)]
pub(crate) struct FrameData {
    pub scroll_infos: Vec<(u32, u32)>,
    pub scroll_rects: Vec<Rect>,
    pub hit_areas: Vec<Rect>,
    pub group_rects: Vec<(String, Rect)>,
    pub content_areas: Vec<(Rect, Rect)>,
    pub focus_rects: Vec<(usize, Rect)>,
    pub focus_groups: Vec<Option<String>>,
    pub raw_draw_rects: Vec<RawDrawRect>,
}

/// Information about a raw-draw node's visible screen rect.
pub(crate) struct RawDrawRect {
    pub draw_id: usize,
    /// The visible portion of the node on screen (clipped to viewport).
    pub rect: Rect,
    /// How many cell rows are clipped from the top (for pixel crop).
    pub top_clip_rows: u32,
    /// The original unclipped height in cell rows.
    pub original_height: u32,
}

/// Collect all per-frame data from a laid-out tree in a single DFS pass.
///
/// Replaces the 7 individual `collect_*` functions that each traversed the
/// tree independently, reducing per-frame traversals from 7x to 1x.
pub(crate) fn collect_all(node: &LayoutNode) -> FrameData {
    let mut data = FrameData::default();

    if node.is_scrollable {
        let viewport_h = node.size.1.saturating_sub(node.frame_vertical());
        data.scroll_infos.push((node.content_height, viewport_h));
        data.scroll_rects
            .push(Rect::new(node.pos.0, node.pos.1, node.size.0, node.size.1));
    }
    if let Some(id) = node.focus_id {
        if node.pos.1 + node.size.1 > 0 {
            data.focus_rects.push((
                id,
                Rect::new(node.pos.0, node.pos.1, node.size.0, node.size.1),
            ));
        }
    }
    if let Some(id) = node.interaction_id {
        let rect = if node.pos.1 + node.size.1 > 0 {
            Rect::new(node.pos.0, node.pos.1, node.size.0, node.size.1)
        } else {
            Rect::new(0, 0, 0, 0)
        };
        if id >= data.hit_areas.len() {
            data.hit_areas.resize(id + 1, Rect::new(0, 0, 0, 0));
        }
        data.hit_areas[id] = rect;
    }

    let child_offset = if node.is_scrollable {
        node.scroll_offset
    } else {
        0
    };
    for child in &node.children {
        collect_all_inner(child, &mut data, child_offset, None, None);
    }

    for overlay in &node.overlays {
        collect_all_inner(&overlay.node, &mut data, 0, None, None);
    }

    data
}

fn collect_all_inner(
    node: &LayoutNode,
    data: &mut FrameData,
    y_offset: u32,
    active_group: Option<&str>,
    viewport: Option<Rect>,
) {
    if node.is_scrollable {
        let viewport_h = node.size.1.saturating_sub(node.frame_vertical());
        data.scroll_infos.push((node.content_height, viewport_h));
    }

    if node.is_scrollable {
        let adj_y = node.pos.1.saturating_sub(y_offset);
        data.scroll_rects
            .push(Rect::new(node.pos.0, adj_y, node.size.0, node.size.1));
    }

    if let Some(id) = node.interaction_id {
        let rect = if node.pos.1 + node.size.1 > y_offset {
            Rect::new(
                node.pos.0,
                node.pos.1.saturating_sub(y_offset),
                node.size.0,
                node.size.1,
            )
        } else {
            Rect::new(0, 0, 0, 0)
        };
        if id >= data.hit_areas.len() {
            data.hit_areas.resize(id + 1, Rect::new(0, 0, 0, 0));
        }
        data.hit_areas[id] = rect;
    }

    if let NodeKind::RawDraw(draw_id) = node.kind {
        let node_x = node.pos.0;
        let node_w = node.size.0;
        let node_h = node.size.1;
        let screen_y = node.pos.1 as i64 - y_offset as i64;

        if let Some(vp) = viewport {
            let img_top = screen_y;
            let img_bottom = screen_y + node_h as i64;
            let vp_top = vp.y as i64;
            let vp_bottom = vp.bottom() as i64;

            if img_bottom > vp_top && img_top < vp_bottom {
                let visible_top = img_top.max(vp_top) as u32;
                let visible_bottom = img_bottom.min(vp_bottom) as u32;
                let visible_height = visible_bottom.saturating_sub(visible_top);
                let top_clip_rows = (vp_top - img_top).max(0) as u32;

                data.raw_draw_rects.push(RawDrawRect {
                    draw_id,
                    rect: Rect::new(node_x, visible_top, node_w, visible_height),
                    top_clip_rows,
                    original_height: node_h,
                });
            }
        } else {
            data.raw_draw_rects.push(RawDrawRect {
                draw_id,
                rect: Rect::new(node_x, screen_y.max(0) as u32, node_w, node_h),
                top_clip_rows: 0,
                original_height: node_h,
            });
        }
    }

    if let Some(name) = &node.group_name {
        if node.pos.1 + node.size.1 > y_offset {
            data.group_rects.push((
                name.clone(),
                Rect::new(
                    node.pos.0,
                    node.pos.1.saturating_sub(y_offset),
                    node.size.0,
                    node.size.1,
                ),
            ));
        }
    }

    if matches!(node.kind, NodeKind::Container(_)) {
        let adj_y = node.pos.1.saturating_sub(y_offset);
        let full = Rect::new(node.pos.0, adj_y, node.size.0, node.size.1);
        let inset_x = node.padding.left + node.border_left_inset();
        let inset_y = node.padding.top + node.border_top_inset();
        let inner_w = node.size.0.saturating_sub(node.frame_horizontal());
        let inner_h = node.size.1.saturating_sub(node.frame_vertical());
        let content = Rect::new(node.pos.0 + inset_x, adj_y + inset_y, inner_w, inner_h);
        data.content_areas.push((full, content));
    }

    if let Some(id) = node.focus_id {
        if node.pos.1 + node.size.1 > y_offset {
            data.focus_rects.push((
                id,
                Rect::new(
                    node.pos.0,
                    node.pos.1.saturating_sub(y_offset),
                    node.size.0,
                    node.size.1,
                ),
            ));
        }
    }

    let current_group = node.group_name.as_deref().or(active_group);
    if let Some(id) = node.focus_id {
        if id >= data.focus_groups.len() {
            data.focus_groups.resize(id + 1, None);
        }
        data.focus_groups[id] = current_group.map(ToString::to_string);
    }

    let (child_offset, child_viewport) = if node.is_scrollable {
        let screen_y = node.pos.1.saturating_sub(y_offset);
        let area = Rect::new(node.pos.0, screen_y, node.size.0, node.size.1);
        let inner = inner_area(node, area);
        (y_offset.saturating_add(node.scroll_offset), Some(inner))
    } else {
        (y_offset, viewport)
    };
    for child in &node.children {
        collect_all_inner(child, data, child_offset, current_group, child_viewport);
    }
}

#[cfg(test)]
pub(crate) fn collect_raw_draw_rects(node: &LayoutNode) -> Vec<RawDrawRect> {
    let mut rects = Vec::new();
    collect_raw_draw_rects_inner(node, &mut rects, 0, None);
    for overlay in &node.overlays {
        collect_raw_draw_rects_inner(&overlay.node, &mut rects, 0, None);
    }
    rects
}

#[cfg(test)]
fn collect_raw_draw_rects_inner(
    node: &LayoutNode,
    rects: &mut Vec<RawDrawRect>,
    y_offset: u32,
    viewport: Option<Rect>,
) {
    if let NodeKind::RawDraw(draw_id) = node.kind {
        let node_x = node.pos.0;
        let node_w = node.size.0;
        let node_h = node.size.1;
        let screen_y = node.pos.1 as i64 - y_offset as i64;

        if let Some(vp) = viewport {
            let img_top = screen_y;
            let img_bottom = screen_y + node_h as i64;
            let vp_top = vp.y as i64;
            let vp_bottom = vp.bottom() as i64;

            if img_bottom <= vp_top || img_top >= vp_bottom {
                return;
            }

            let visible_top = img_top.max(vp_top) as u32;
            let visible_bottom = (img_bottom.min(vp_bottom)) as u32;
            let visible_height = visible_bottom.saturating_sub(visible_top);
            let top_clip_rows = (vp_top - img_top).max(0) as u32;

            rects.push(RawDrawRect {
                draw_id,
                rect: Rect::new(node_x, visible_top, node_w, visible_height),
                top_clip_rows,
                original_height: node_h,
            });
        } else {
            let screen_y_clamped = screen_y.max(0) as u32;
            rects.push(RawDrawRect {
                draw_id,
                rect: Rect::new(node_x, screen_y_clamped, node_w, node_h),
                top_clip_rows: 0,
                original_height: node_h,
            });
        }
    }

    let (child_offset, child_viewport) = if node.is_scrollable {
        let screen_y = node.pos.1.saturating_sub(y_offset);
        let area = Rect::new(node.pos.0, screen_y, node.size.0, node.size.1);
        let inner = inner_area(node, area);
        (y_offset.saturating_add(node.scroll_offset), Some(inner))
    } else {
        (y_offset, viewport)
    };

    for child in &node.children {
        collect_raw_draw_rects_inner(child, rects, child_offset, child_viewport);
    }
}
