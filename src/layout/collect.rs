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
    /// Full node rectangles indexed by interaction ID, in logical layout
    /// coordinates before scroll offsets or viewport clipping. Unlike hit
    /// areas, these preserve offscreen sizes for next-frame measurement.
    pub allocated_areas: Vec<Rect>,
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
        self.allocated_areas.clear();
        self.group_rects.clear();
        self.content_areas.clear();
        self.focus_rects.clear();
        self.focus_groups.clear();
        self.raw_draw_rects.clear();
    }

    /// Swap collected feedback vectors with the runtime's previous-frame set.
    ///
    /// Call this once before [`collect_all`] to recover last frame's capacities,
    /// then again after collection to publish the newly filled vectors. Raw draw
    /// rectangles stay in `FrameData` because callbacks consume them in-frame.
    #[allow(dead_code)]
    pub(crate) fn swap_feedback(&mut self, feedback: &mut crate::LayoutFeedbackState) {
        std::mem::swap(&mut self.scroll_infos, &mut feedback.prev_scroll_infos);
        std::mem::swap(&mut self.scroll_rects, &mut feedback.prev_scroll_rects);
        std::mem::swap(&mut self.hit_areas, &mut feedback.prev_hit_map);
        std::mem::swap(
            &mut self.allocated_areas,
            &mut feedback.prev_allocated_areas,
        );
        std::mem::swap(&mut self.group_rects, &mut feedback.prev_group_rects);
        std::mem::swap(&mut self.content_areas, &mut feedback.prev_content_map);
        std::mem::swap(&mut self.focus_rects, &mut feedback.prev_focus_rects);
        std::mem::swap(&mut self.focus_groups, &mut feedback.prev_focus_groups);
    }
}

/// Information about a raw-draw node's visible screen rect.
#[allow(dead_code)] // Horizontal crop fields are consumed by the pending lib.rs integration.
pub(crate) struct RawDrawRect {
    pub draw_id: usize,
    /// The visible portion of the node on screen (clipped to viewport).
    pub rect: Rect,
    /// How many cell columns are clipped from the left (for source crop).
    pub left_clip_cols: u32,
    /// How many cell rows are clipped from the top (for pixel crop).
    pub top_clip_rows: u32,
    /// The original unclipped width in cell columns.
    pub original_width: u32,
    /// The original unclipped height in cell rows.
    pub original_height: u32,
}

#[derive(Clone, Copy)]
struct SignedRect {
    left: i64,
    top: i64,
    right: i64,
    bottom: i64,
}

impl SignedRect {
    fn from_node(node: &LayoutNode, x_offset: i64, y_offset: i64) -> Self {
        let left = i64::from(node.pos.0).saturating_sub(x_offset);
        let top = i64::from(node.pos.1).saturating_sub(y_offset);
        Self {
            left,
            top,
            right: left.saturating_add(i64::from(node.size.0)),
            bottom: top.saturating_add(i64::from(node.size.1)),
        }
    }

    fn intersection(self, other: Self) -> Self {
        Self {
            left: self.left.max(other.left),
            top: self.top.max(other.top),
            right: self.right.min(other.right),
            bottom: self.bottom.min(other.bottom),
        }
    }

    fn is_empty(self) -> bool {
        self.left >= self.right || self.top >= self.bottom
    }

    fn inset(self, node: &LayoutNode) -> Self {
        let left_inset = i64::from(node.border_left_inset()) + i64::from(node.padding.left);
        let right_inset = i64::from(node.border_right_inset()) + i64::from(node.padding.right);
        let top_inset = i64::from(node.border_top_inset()) + i64::from(node.padding.top);
        let bottom_inset = i64::from(node.border_bottom_inset()) + i64::from(node.padding.bottom);
        let left = self.left.saturating_add(left_inset).min(self.right);
        let top = self.top.saturating_add(top_inset).min(self.bottom);
        Self {
            left,
            top,
            right: self.right.saturating_sub(right_inset).max(left),
            bottom: self.bottom.saturating_sub(bottom_inset).max(top),
        }
    }

    fn visible_within(self, viewport: Option<Self>) -> Self {
        viewport.map_or(self, |viewport| self.intersection(viewport))
    }

    fn to_rect(self) -> Option<Rect> {
        let left = self.left.clamp(0, i64::from(u32::MAX));
        let top = self.top.clamp(0, i64::from(u32::MAX));
        let right = self.right.clamp(0, i64::from(u32::MAX));
        let bottom = self.bottom.clamp(0, i64::from(u32::MAX));
        if left >= right || top >= bottom {
            return None;
        }
        Some(Rect::new(
            left as u32,
            top as u32,
            (right - left) as u32,
            (bottom - top) as u32,
        ))
    }
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

    let screen_rect = SignedRect::from_node(node, 0, 0);
    let visible_rect = screen_rect.to_rect().unwrap_or_default();

    if node.is_scrollable {
        push_scroll_info(node, data);
        data.scroll_rects.push(visible_rect);
    }
    if let Some(id) = node.focus_id
        && !visible_rect.is_empty()
    {
        data.focus_rects.push((id, visible_rect));
    }
    if let Some(id) = node.interaction_id {
        record_allocated_area(node, data, id);
        if id >= data.hit_areas.len() {
            data.hit_areas.resize(id + 1, Rect::new(0, 0, 0, 0));
        }
        data.hit_areas[id] = visible_rect;
    }

    // #247: scrollable nodes shift their children on exactly one axis.
    // `scroll_offset` is non-zero only for a column, `scroll_offset_x` only
    // for a row, so seeding both is correct for either orientation.
    let (child_x_offset, child_y_offset) = if node.is_scrollable {
        (
            i64::from(node.scroll_offset_x),
            i64::from(node.scroll_offset),
        )
    } else {
        (0, 0)
    };
    let child_viewport = (matches!(node.kind, NodeKind::Container(_)) && !screen_rect.is_empty())
        .then(|| screen_rect.inset(node));
    for child in &node.children {
        collect_all_inner(
            child,
            data,
            child_x_offset,
            child_y_offset,
            None,
            child_viewport,
            1,
        );
    }

    for overlay in &node.overlays {
        collect_all_inner(&overlay.node, data, 0, 0, None, None, 1);
    }
}

fn record_allocated_area(node: &LayoutNode, data: &mut FrameData, id: usize) {
    if id >= data.allocated_areas.len() {
        data.allocated_areas.resize(id + 1, Rect::default());
    }
    data.allocated_areas[id] = Rect::new(node.pos.0, node.pos.1, node.size.0, node.size.1);
}

/// Record the content and viewport extent for the container's scroll axis.
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
    x_offset: i64,
    y_offset: i64,
    active_group: Option<&Arc<str>>,
    viewport: Option<SignedRect>,
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
    let screen_rect = SignedRect::from_node(node, x_offset, y_offset);
    let visible = screen_rect.visible_within(viewport);
    let visible_rect = visible.to_rect();
    // Single combined block — keep `scroll_infos` and `scroll_rects` writes
    // adjacent so the `assert_eq!(scroll_infos.len(), scroll_rects.len())`
    // invariant in `lib.rs` cannot drift if one side is edited without the
    // other. Mirrors the root-node path in `collect_all`.
    if node.is_scrollable {
        push_scroll_info(node, data);
        data.scroll_rects.push(visible_rect.unwrap_or_default());
    }

    if let Some(id) = node.interaction_id {
        record_allocated_area(node, data, id);
        if id >= data.hit_areas.len() {
            data.hit_areas.resize(id + 1, Rect::new(0, 0, 0, 0));
        }
        data.hit_areas[id] = visible_rect.unwrap_or_default();
    }

    if let NodeKind::RawDraw(draw_id) = node.kind
        && let Some(rect) = visible_rect
    {
        data.raw_draw_rects.push(RawDrawRect {
            draw_id,
            rect,
            left_clip_cols: i64::from(rect.x).saturating_sub(screen_rect.left) as u32,
            top_clip_rows: i64::from(rect.y).saturating_sub(screen_rect.top) as u32,
            original_width: node.size.0,
            original_height: node.size.1,
        });
    }

    // The build-time conversion in `ContainerBuilder::group_name` /
    // `Context::begin_container` already produced an `Arc<str>`. Cloning here
    // is a pointer bump (atomic increment), not a heap allocation — so this
    // collect-time handoff costs zero allocations regardless of group depth.
    let node_group_arc: Option<Arc<str>> = node.group_name.clone();

    if let Some(name) = &node_group_arc
        && let Some(rect) = visible_rect
    {
        data.group_rects.push((Arc::clone(name), rect));
    }

    if matches!(node.kind, NodeKind::Container(_))
        && let Some(full) = visible_rect
        && let Some(content) = screen_rect.inset(node).visible_within(viewport).to_rect()
    {
        data.content_areas.push((full, content));
    }

    if let Some(id) = node.focus_id
        && let Some(rect) = visible_rect
    {
        data.focus_rects.push((id, rect));
    }

    let current_group = node_group_arc.as_ref().or(active_group);
    if let Some(id) = node.focus_id {
        if id >= data.focus_groups.len() {
            data.focus_groups.resize(id + 1, None);
        }
        // Arc<str> clone is a pointer bump, not a heap allocation.
        data.focus_groups[id] = current_group.cloned();
    }

    let child_x_offset = if node.is_scrollable {
        x_offset.saturating_add(i64::from(node.scroll_offset_x))
    } else {
        x_offset
    };
    let child_y_offset = if node.is_scrollable {
        y_offset.saturating_add(i64::from(node.scroll_offset))
    } else {
        y_offset
    };
    let child_viewport = if matches!(node.kind, NodeKind::Container(_)) {
        let own_viewport = screen_rect.inset(node);
        Some(viewport.map_or(own_viewport, |parent| own_viewport.intersection(parent)))
    } else {
        viewport
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
