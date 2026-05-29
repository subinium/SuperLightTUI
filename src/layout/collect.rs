use super::flexbox::inner_area;
use super::*;
use std::sync::Arc;

#[derive(Default)]
pub(crate) struct FrameData {
    /// Per-scrollable feedback: `(content_extent, viewport_extent, is_horizontal)`.
    ///
    /// For a vertical scrollable (`Direction::Column`) the extents are content
    /// height / viewport height and `is_horizontal` is `false`; for a
    /// horizontal scrollable (`Direction::Row`, #247) they are content width /
    /// viewport width and `is_horizontal` is `true`. The axis flag lets
    /// `Context::scrollable` bind the right `ScrollState` bounds next frame even
    /// though the builder reads this back before `.row()` / `.col()` is known.
    pub scroll_infos: Vec<(u32, u32, bool)>,
    pub scroll_rects: Vec<Rect>,
    pub hit_areas: Vec<Rect>,
    pub group_rects: Vec<(Arc<str>, Rect)>,
    pub content_areas: Vec<(Rect, Rect)>,
    pub focus_rects: Vec<(usize, Rect)>,
    pub focus_groups: Vec<Option<Arc<str>>>,
    pub raw_draw_rects: Vec<RawDrawRect>,
}

impl FrameData {
    /// Reset all collection vectors to `len = 0` while keeping their
    /// allocated capacities (issue #155). The next frame's `collect_all`
    /// call writes into these slots, so the per-frame allocation churn of
    /// 8 fresh `Vec::new()`s is amortized to zero after warm-up.
    pub(crate) fn clear(&mut self) {
        self.scroll_infos.clear();
        self.scroll_rects.clear();
        self.hit_areas.clear();
        self.group_rects.clear();
        self.content_areas.clear();
        self.focus_rects.clear();
        self.focus_groups.clear();
        self.raw_draw_rects.clear();
    }
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
///
/// As of issue #155 the caller owns the `FrameData` allocation: we clear
/// (preserving capacity) and write into it directly, so steady-state frames
/// pay zero allocation churn for the 8 collection vectors.
pub(crate) fn collect_all(node: &LayoutNode, data: &mut FrameData) {
    data.clear();

    if node.is_scrollable {
        push_scroll_info(node, data);
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

    // #247: scrollable nodes shift their children on exactly one axis.
    // `scroll_offset` is non-zero only for a column, `scroll_offset_x` only
    // for a row, so seeding both is correct for either orientation.
    let (child_x_offset, child_y_offset) = if node.is_scrollable {
        (node.scroll_offset_x, node.scroll_offset)
    } else {
        (0, 0)
    };
    for child in &node.children {
        collect_all_inner(child, data, child_x_offset, child_y_offset, None, None, 1);
    }

    for overlay in &node.overlays {
        collect_all_inner(&overlay.node, data, 0, 0, None, None, 1);
    }
}

/// Push the `(content_extent, viewport_extent, is_horizontal)` tuple for a
/// scrollable node, choosing the axis from its layout direction (#247).
///
/// A scrollable `Direction::Row` reports content width / viewport width; any
/// other scrollable (the historic `Direction::Column`) reports content height
/// / viewport height. Keeps the axis-selection logic in one place so both
/// `collect_all` and `collect_all_inner` push identical tuples.
fn push_scroll_info(node: &LayoutNode, data: &mut FrameData) {
    if matches!(node.kind, NodeKind::Container(Direction::Row)) {
        let viewport_w = node.size.0.saturating_sub(node.frame_horizontal());
        data.scroll_infos
            .push((node.content_width, viewport_w, true));
    } else {
        let viewport_h = node.size.1.saturating_sub(node.frame_vertical());
        data.scroll_infos
            .push((node.content_height, viewport_h, false));
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_all_inner(
    node: &LayoutNode,
    data: &mut FrameData,
    x_offset: u32,
    y_offset: u32,
    active_group: Option<&Arc<str>>,
    viewport: Option<Rect>,
    depth: usize,
) {
    // Hard upper bound — see `tree::MAX_LAYOUT_DEPTH`. `build_children`
    // already enforces this at construction, so reaching it here means
    // either a future refactor introduced a new tree-mutation path or the
    // build-side guard regressed; surfacing an explicit panic with the
    // same message keeps diagnostics consistent across the pipeline.
    if depth > super::tree::MAX_LAYOUT_DEPTH {
        panic!(
            "layout tree depth exceeds {}: check for recursive container nesting",
            super::tree::MAX_LAYOUT_DEPTH
        );
    }
    // #247: every collected rect's x is shifted by `x_offset` the same way its
    // y is shifted by `y_offset`, so a horizontally-scrolled child's
    // interaction / focus / group / content / scroll rect tracks its on-screen
    // position. `adj_x` is the screen x for this node.
    let adj_x = node.pos.0.saturating_sub(x_offset);
    let adj_y = node.pos.1.saturating_sub(y_offset);
    // Single combined block — keep `scroll_infos` and `scroll_rects` writes
    // adjacent so the `assert_eq!(scroll_infos.len(), scroll_rects.len())`
    // invariant in `lib.rs` cannot drift if one side is edited without the
    // other. Mirrors the root-node path in `collect_all`.
    if node.is_scrollable {
        push_scroll_info(node, data);
        data.scroll_rects
            .push(Rect::new(adj_x, adj_y, node.size.0, node.size.1));
    }

    if let Some(id) = node.interaction_id {
        let rect = if node.pos.1 + node.size.1 > y_offset && node.pos.0 + node.size.0 > x_offset {
            Rect::new(adj_x, adj_y, node.size.0, node.size.1)
        } else {
            Rect::new(0, 0, 0, 0)
        };
        if id >= data.hit_areas.len() {
            data.hit_areas.resize(id + 1, Rect::new(0, 0, 0, 0));
        }
        data.hit_areas[id] = rect;
    }

    if let NodeKind::RawDraw(draw_id) = node.kind {
        let node_x = adj_x;
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

    // The build-time conversion in `ContainerBuilder::group_name` /
    // `Context::begin_container` already produced an `Arc<str>`. Cloning here
    // is a pointer bump (atomic increment), not a heap allocation — so this
    // collect-time handoff costs zero allocations regardless of group depth.
    let node_group_arc: Option<Arc<str>> = node.group_name.clone();

    if let Some(name) = &node_group_arc {
        if node.pos.1 + node.size.1 > y_offset && node.pos.0 + node.size.0 > x_offset {
            data.group_rects.push((
                Arc::clone(name),
                Rect::new(adj_x, adj_y, node.size.0, node.size.1),
            ));
        }
    }

    if matches!(node.kind, NodeKind::Container(_)) {
        let full = Rect::new(adj_x, adj_y, node.size.0, node.size.1);
        let inset_x = node.padding.left + node.border_left_inset();
        let inset_y = node.padding.top + node.border_top_inset();
        let inner_w = node.size.0.saturating_sub(node.frame_horizontal());
        let inner_h = node.size.1.saturating_sub(node.frame_vertical());
        let content = Rect::new(adj_x + inset_x, adj_y + inset_y, inner_w, inner_h);
        data.content_areas.push((full, content));
    }

    if let Some(id) = node.focus_id {
        if node.pos.1 + node.size.1 > y_offset && node.pos.0 + node.size.0 > x_offset {
            data.focus_rects
                .push((id, Rect::new(adj_x, adj_y, node.size.0, node.size.1)));
        }
    }

    let current_group = node_group_arc.as_ref().or(active_group);
    if let Some(id) = node.focus_id {
        if id >= data.focus_groups.len() {
            data.focus_groups.resize(id + 1, None);
        }
        // Arc<str> clone is a pointer bump, not a heap allocation.
        data.focus_groups[id] = current_group.cloned();
    }

    let (child_x_offset, child_y_offset, child_viewport) = if node.is_scrollable {
        let area = Rect::new(adj_x, adj_y, node.size.0, node.size.1);
        let inner = inner_area(node, area);
        // #247: seed both offsets — only the matching axis is non-zero on a
        // single-axis scroller.
        (
            x_offset.saturating_add(node.scroll_offset_x),
            y_offset.saturating_add(node.scroll_offset),
            Some(inner),
        )
    } else {
        (x_offset, y_offset, viewport)
    };
    for child in &node.children {
        collect_all_inner(
            child,
            data,
            child_x_offset,
            child_y_offset,
            current_group,
            child_viewport,
            depth + 1,
        );
    }
}
