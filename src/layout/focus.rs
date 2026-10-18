use super::*;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct LayoutClip {
    x: Option<(u32, u32)>,
    y: Option<(u32, u32)>,
}

impl LayoutClip {
    fn intersect(self, rect: Rect) -> Self {
        fn axis(old: Option<(u32, u32)>, start: u32, end: u32) -> Option<(u32, u32)> {
            Some(old.map_or((start, end), |(a, b)| (a.max(start), b.min(end))))
        }
        Self {
            x: axis(self.x, rect.x, rect.right()),
            y: axis(self.y, rect.y, rect.bottom()),
        }
    }

    fn apply(self, rect: Rect) -> Rect {
        let (left, right) = self.x.map_or((rect.x, rect.right()), |(a, b)| {
            (a.max(rect.x), b.min(rect.right()))
        });
        let (top, bottom) = self.y.map_or((rect.y, rect.bottom()), |(a, b)| {
            (a.max(rect.y), b.min(rect.bottom()))
        });
        Rect::new(
            left,
            top,
            right.saturating_sub(left),
            bottom.saturating_sub(top),
        )
    }

    fn apply_signed(self, mut rect: NavigationRect) -> NavigationRect {
        if let Some((start, end)) = self.x {
            rect.left = rect.left.max(i64::from(start));
            rect.right = rect.right.min(i64::from(end));
        }
        if let Some((start, end)) = self.y {
            rect.top = rect.top.max(i64::from(start));
            rect.bottom = rect.bottom.min(i64::from(end));
        }
        rect
    }
}

/// Geometry used for keyboard navigation, separate from clipped mouse hits.
#[derive(Default)]
pub(crate) struct GeometryFeedback {
    pub focus: Vec<Option<FocusGeometry>>,
    pub scrolls: Vec<ScrollGeometry>,
    pub scroll_by_id: std::collections::HashMap<u64, usize>,
    pub groups: Vec<(Arc<str>, Rect)>,
}

impl GeometryFeedback {
    pub(crate) fn clear(&mut self) {
        self.focus.clear();
        self.scrolls.clear();
        self.scroll_by_id.clear();
        self.groups.clear();
    }

    pub(crate) fn record(
        &mut self,
        node: &LayoutNode,
        parent_scroll: Option<usize>,
        clip: LayoutClip,
        active_focus: Option<usize>,
    ) -> (Option<usize>, LayoutClip, Option<usize>) {
        let rect = Rect::new(node.pos.0, node.pos.1, node.size.0, node.size.1);
        let active_focus = node.focus_id.or(active_focus);
        if let Some(id) = node.focus_id {
            if self.focus.len() <= id {
                self.focus.resize(id + 1, None);
            }
            self.focus[id] = Some(FocusGeometry {
                rect,
                scroll: parent_scroll,
                clip,
                caret: None,
            });
        }
        if let Some(id) = active_focus
            && let Some(text) = node.text_data.as_deref()
            && let Some(offset) = text.cursor_offset
            && let Some(content) = text.content.as_deref()
            && !node.wrap
            // `text.width` is `content`'s width, measured once at build (#494).
            && !(node.truncate && text.width > node.size.0)
        {
            let width = text.width;
            let align = match node.align {
                Align::Start => 0,
                Align::Center => node.size.0.saturating_sub(width) / 2,
                Align::End => node.size.0.saturating_sub(width),
            };
            let cursor = content
                .graphemes(true)
                .take(offset)
                .map(|part| u32::try_from(crate::cell::display_width(part)).unwrap_or(u32::MAX))
                .fold(0u32, u32::saturating_add);
            if let Some(Some(target)) = self.focus.get_mut(id) {
                target.caret = Some(Rect::new(
                    rect.x.saturating_add(align).saturating_add(cursor),
                    rect.y,
                    1,
                    1,
                ));
            }
        }
        if let Some(name) = &node.group_name {
            self.groups.push((Arc::clone(name), rect));
        }
        if !matches!(node.kind, NodeKind::Container(_)) {
            return (parent_scroll, clip, active_focus);
        }
        let viewport = Rect::new(
            rect.x
                .saturating_add(node.border_left_inset())
                .saturating_add(node.padding.left),
            rect.y
                .saturating_add(node.border_top_inset())
                .saturating_add(node.padding.top),
            rect.width.saturating_sub(node.frame_horizontal()),
            rect.height.saturating_sub(node.frame_vertical()),
        );
        if node.is_scrollable {
            let horizontal = matches!(node.kind, NodeKind::Container(Direction::Row));
            let index = self.scrolls.len();
            let anchor = if horizontal || node.scroll_state_id == 0 {
                None
            } else {
                anchor_sample(node, viewport.y, node.scroll_offset)
            };
            self.scrolls.push(ScrollGeometry {
                anchor,
                state_id: node.scroll_state_id,
                parent: parent_scroll,
                viewport,
                parent_clip: clip,
                content: if horizontal {
                    node.content_width
                } else {
                    node.content_height
                },
                offset: if horizontal {
                    node.scroll_offset_x
                } else {
                    node.scroll_offset
                },
                horizontal,
                follow_focus: node.scroll_follow_focus,
            });
            if node.scroll_state_id != 0 {
                self.scroll_by_id.insert(node.scroll_state_id, index);
            }
            let mut child_clip = clip.intersect(viewport);
            // Scrolling removes constraints only on its own axis. A horizontal
            // ancestor cannot make vertically clipped content visible.
            if horizontal {
                child_clip.x = None;
            } else {
                child_clip.y = None;
            }
            (Some(index), child_clip, active_focus)
        } else {
            (parent_scroll, clip.intersect(viewport), active_focus)
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct FocusGeometry {
    pub rect: Rect,
    pub scroll: Option<usize>,
    pub clip: LayoutClip,
    pub caret: Option<Rect>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ScrollGeometry {
    /// Scroll-anchoring sample: the first child at or below the viewport top
    /// (#443). `None` at offset 0, for horizontal scrollers and raw offset
    /// containers.
    pub anchor: Option<AnchorSample>,
    pub state_id: u64,
    pub parent: Option<usize>,
    pub viewport: Rect,
    pub parent_clip: LayoutClip,
    pub content: u32,
    pub offset: u32,
    pub horizontal: bool,
    /// None denotes a raw offset container with no bound ScrollState.
    pub follow_focus: Option<bool>,
}

impl ScrollGeometry {
    pub(super) fn effective_viewport(self) -> Rect {
        self.parent_clip.apply(self.viewport)
    }

    pub(super) fn viewport_extent(self) -> u32 {
        let viewport = if self.follow_focus.is_some() {
            self.effective_viewport()
        } else {
            self.viewport
        };
        if self.horizontal {
            viewport.width
        } else {
            viewport.height
        }
    }

    pub(crate) fn max_offset(self) -> u32 {
        let visible = if self.follow_focus.is_some() {
            self.effective_viewport()
        } else {
            self.viewport
        };
        let extent = if visible.is_empty() {
            0
        } else if self.horizontal {
            visible.right().saturating_sub(self.viewport.x)
        } else {
            visible.bottom().saturating_sub(self.viewport.y)
        };
        self.content.saturating_sub(extent)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ScrollAnchor {
    state_id: u64,
    viewport: Rect,
    effective_viewport: Rect,
    horizontal: bool,
    follow_focus: Option<bool>,
}

impl From<ScrollGeometry> for ScrollAnchor {
    fn from(scroll: ScrollGeometry) -> Self {
        Self {
            state_id: scroll.state_id,
            viewport: scroll.viewport,
            effective_viewport: scroll.effective_viewport(),
            horizontal: scroll.horizontal,
            follow_focus: scroll.follow_focus,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ScrollAdjustment {
    pub from: u32,
    pub to: u32,
    pub horizontal: bool,
}

#[derive(Default)]
pub(crate) struct FocusScrollState {
    anchor: Option<FocusAnchor>,
    path: Vec<ScrollAnchor>,
    scratch_path: Vec<ScrollAnchor>,
    /// Keyed by ScrollState identity, consumed by its next binding.
    pub pending: std::collections::HashMap<u64, ScrollAdjustment>,
    /// Whether the last `prepare` revealed a newly focused target; the
    /// scrollers on its path then skip anchoring for that frame (#443).
    revealed: bool,
    /// Per-scroller samples from the previous frame (#443).
    anchors_prev: Vec<PreviousScroll>,
}

/// Per-frame scroll policy recorded when a `ScrollState` binds (#443).
#[derive(Clone, Copy, Debug)]
pub(crate) struct ScrollPolicy {
    pub state_id: u64,
    /// `ScrollState::anchor`.
    pub anchor: bool,
    /// `ScrollState::stick_to_bottom`.
    pub stick_to_bottom: bool,
    /// The scrollbar thumb is being dragged: leave the offset alone.
    pub dragging: bool,
}

#[derive(Clone, Copy, Debug)]
struct PreviousScroll {
    state_id: u64,
    offset: u32,
    max: u32,
    anchor: Option<AnchorSample>,
}

/// Deepest descent below a scroller when picking an anchor child.
const ANCHOR_DEPTH: usize = 4;
/// How far from its old index an anchor child is searched for after
/// insertions or removals above it.
const ANCHOR_SEARCH: usize = 64;
/// Text nodes folded into a child's fingerprint.
const ANCHOR_PRINT_TEXTS: usize = 4;

/// Identity and content-space position of a scroll anchor child.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct AnchorSample {
    len: u8,
    /// Child index at each level below the scroller.
    path: [u32; ANCHOR_DEPTH],
    /// Fingerprint (group name, else leading text) at each level.
    prints: [u64; ANCHOR_DEPTH],
    /// Structure (kind, child count, height, group name) at each level, the
    /// fallback when the text changed in place (spinners, timers).
    shapes: [u64; ANCHOR_DEPTH],
    /// Top edge relative to the scroller's content origin.
    top: u32,
}

fn node_structure(node: &LayoutNode) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::mem::discriminant(&node.kind).hash(&mut hasher);
    node.children.len().hash(&mut hasher);
    node.size.1.hash(&mut hasher);
    node.group_name.hash(&mut hasher);
    hasher.finish()
}

fn node_fingerprint(node: &LayoutNode) -> u64 {
    use std::hash::{Hash, Hasher};
    fn texts(node: &LayoutNode, hasher: &mut impl Hasher, budget: &mut usize, depth: usize) {
        if *budget == 0 || depth > ANCHOR_DEPTH * 2 {
            return;
        }
        if let Some(content) = node.text_data.as_deref().and_then(|t| t.content.as_deref()) {
            content.hash(hasher);
            *budget -= 1;
        }
        for child in &node.children {
            texts(child, hasher, budget, depth + 1);
        }
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    if let Some(name) = &node.group_name {
        1u8.hash(&mut hasher);
        name.hash(&mut hasher);
    } else {
        std::mem::discriminant(&node.kind).hash(&mut hasher);
        let mut budget = ANCHOR_PRINT_TEXTS;
        texts(node, &mut hasher, &mut budget, 0);
    }
    hasher.finish()
}

fn content_top(node: &LayoutNode, origin_y: u32) -> u32 {
    node.pos.1.saturating_sub(origin_y)
}

/// Pick the first child (descending into a child that straddles the viewport
/// top) whose top edge is at or below `offset`.
fn anchor_sample(scroller: &LayoutNode, origin_y: u32, offset: u32) -> Option<AnchorSample> {
    if offset == 0 {
        return None;
    }
    let mut sample = AnchorSample::default();
    let mut parent = scroller;
    loop {
        let mut next = None;
        for (index, child) in parent.children.iter().enumerate() {
            let top = content_top(child, origin_y);
            let bottom = top.saturating_add(child.size.1);
            if top >= offset {
                next = Some((index, child, false));
                break;
            }
            if bottom > offset {
                let descend = matches!(child.kind, NodeKind::Container(_))
                    && !child.is_scrollable
                    && !child.children.is_empty();
                next = Some((index, child, descend));
                break;
            }
        }
        let Some((index, child, descend)) = next else {
            // Nothing below the viewport top at this level: anchor on the
            // straddling parent itself, if we descended into one.
            if sample.len == 0 {
                return None;
            }
            return Some(sample);
        };
        let level = usize::from(sample.len);
        sample.path[level] = u32::try_from(index).unwrap_or(u32::MAX);
        sample.prints[level] = node_fingerprint(child);
        sample.shapes[level] = node_structure(child);
        sample.len += 1;
        sample.top = content_top(child, origin_y);
        if !descend || usize::from(sample.len) == ANCHOR_DEPTH {
            return Some(sample);
        }
        parent = child;
    }
}

/// Find `sample` again in `scroller`'s new children and return its new top.
fn locate_anchor(scroller: &LayoutNode, origin_y: u32, sample: &AnchorSample) -> Option<u32> {
    let mut parent = scroller;
    for level in 0..usize::from(sample.len) {
        let want = sample.path[level] as usize;
        let print = sample.prints[level];
        let children = &parent.children;
        let matches = |index: usize| {
            children
                .get(index)
                .is_some_and(|child| node_fingerprint(child) == print)
        };
        let same_structure = |index: usize| {
            children
                .get(index)
                .is_some_and(|child| node_structure(child) == sample.shapes[level])
        };
        // Same index and content, else the same content nearby (inserted or
        // removed siblings), else the same index and structure (text that
        // changes in place, like a spinner or a timer).
        let index = if matches(want) {
            want
        } else if let Some(index) = (1..=ANCHOR_SEARCH)
            .flat_map(|distance| [want.checked_sub(distance), want.checked_add(distance)])
            .flatten()
            .find(|&index| matches(index))
        {
            index
        } else if same_structure(want) {
            want
        } else {
            return None;
        };
        parent = &children[index];
    }
    Some(content_top(parent, origin_y))
}

fn scroll_origin_y(node: &LayoutNode) -> u32 {
    node.pos
        .1
        .saturating_add(node.border_top_inset())
        .saturating_add(node.padding.top)
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct FocusAnchor {
    id: usize,
    rect: Rect,
    clipped: Rect,
    caret: Option<Rect>,
}

impl FocusScrollState {
    pub(crate) fn invalidate(&mut self) {
        self.anchor = None;
        self.path.clear();
        self.pending.clear();
        self.revealed = false;
        self.anchors_prev.clear();
    }

    /// Keep the content under the viewport stable when content above it
    /// changes height, and keep `stick_to_bottom` scrollers pinned to the end
    /// (#443). Runs after `prepare` on the freshly laid-out tree; adjustments
    /// join `pending` so they render this frame and sync into the
    /// `ScrollState` on its next binding. Returns whether any were added.
    pub(crate) fn anchor(
        &mut self,
        tree: &LayoutNode,
        geometry: &GeometryFeedback,
        policies: &[ScrollPolicy],
    ) -> bool {
        if self.anchors_prev.is_empty() || policies.is_empty() {
            return false;
        }
        let mut changed = false;
        let mut visit = |node: &LayoutNode, this: &mut Self| {
            let id = node.scroll_state_id;
            let Some(previous) = this.anchors_prev.iter().find(|p| p.state_id == id).copied()
            else {
                return;
            };
            let Some(policy) = policies.iter().rev().find(|p| p.state_id == id) else {
                return;
            };
            let Some(current) = geometry
                .scroll_by_id
                .get(&id)
                .and_then(|&index| geometry.scrolls.get(index))
                .copied()
            else {
                return;
            };
            if this.revealed && this.path.iter().any(|scroll| scroll.state_id == id) {
                // Focus reveal wins on the frame focus moved.
                return;
            }
            let max = current.max_offset();
            let base = this.pending.get(&id).map_or(current.offset, |a| a.to);
            // Stick only while the view was at the end last frame and nothing
            // scrolled it back up this frame (a wheel-up disengages at once).
            let target = if policy.stick_to_bottom
                && previous.offset >= previous.max
                && current.offset >= previous.offset
            {
                max
            } else if policy.anchor
                && !policy.dragging
                && previous.offset > 0
                && let Some(sample) = previous.anchor
                && let Some(top) = locate_anchor(node, scroll_origin_y(node), &sample)
            {
                let delta = i64::from(top) - i64::from(sample.top);
                (i64::from(base) + delta).clamp(0, i64::from(max)) as u32
            } else {
                return;
            };
            if target != base {
                this.pending.insert(
                    id,
                    ScrollAdjustment {
                        from: current.offset,
                        to: target,
                        horizontal: false,
                    },
                );
                changed = true;
            }
        };
        fn walk(
            node: &LayoutNode,
            this: &mut FocusScrollState,
            visit: &mut impl FnMut(&LayoutNode, &mut FocusScrollState),
        ) {
            if node.is_scrollable && node.scroll_state_id != 0 {
                visit(node, this);
            }
            for child in &node.children {
                walk(child, this, visit);
            }
            for overlay in &node.overlays {
                walk(&overlay.node, this, visit);
            }
        }
        walk(tree, self, &mut visit);
        changed
    }

    /// Remember this frame's final offsets, limits and anchor samples for the
    /// next frame's `anchor` pass.
    pub(crate) fn record_anchors(&mut self, geometry: &GeometryFeedback) {
        self.anchors_prev.clear();
        for scroll in &geometry.scrolls {
            if scroll.state_id == 0 || scroll.horizontal {
                continue;
            }
            self.anchors_prev.push(PreviousScroll {
                state_id: scroll.state_id,
                offset: scroll.offset,
                max: scroll.max_offset(),
                anchor: scroll.anchor,
            });
        }
    }

    pub(crate) fn prepare(&mut self, focused: Option<usize>, geometry: &GeometryFeedback) -> bool {
        self.pending.clear();
        self.revealed = false;
        // Current layout bounds can shrink before the next state-binding pass.
        for &scroll in &geometry.scrolls {
            if scroll.follow_focus.is_some() {
                self.adjust(scroll, scroll.offset.min(scroll.max_offset()));
            }
        }
        self.scratch_path.clear();
        let target = focused.and_then(|id| {
            geometry
                .focus
                .get(id)
                .copied()
                .flatten()
                .map(|target| (id, target))
        });
        let Some((id, target)) = target else {
            self.anchor = None;
            self.path.clear();
            return !self.pending.is_empty();
        };
        let clipped = target.clip.apply(target.rect);
        let anchor = FocusAnchor {
            id,
            rect: target.rect,
            clipped,
            caret: target.caret,
        };
        let mut parent = target.scroll;
        while let Some(index) = parent {
            let scroll = geometry.scrolls[index];
            self.scratch_path.push(scroll.into());
            parent = scroll.parent;
        }
        if self.anchor == Some(anchor) && self.path == self.scratch_path {
            return !self.pending.is_empty();
        }
        self.anchor = Some(anchor);
        self.revealed = true;
        std::mem::swap(&mut self.path, &mut self.scratch_path);

        // Walk from inner to outer: each ancestor sees the already-scrolled
        // and clipped child, not its original unbounded document position.
        let mut rect = NavigationRect::from(clipped);
        let mut caret = target.caret.map(NavigationRect::from);
        let mut parent = target.scroll;
        while let Some(index) = parent {
            let scroll = geometry.scrolls[index];
            let mut offset = if scroll.follow_focus.is_some() {
                scroll.offset.min(scroll.max_offset())
            } else {
                scroll.offset
            };
            if scroll.follow_focus == Some(true) && !rect.empty() {
                let viewport = NavigationRect::from(scroll.effective_viewport());
                let measured = if (scroll.horizontal
                    && rect.right - rect.left > viewport.right - viewport.left)
                    || (!scroll.horizontal
                        && rect.bottom - rect.top > viewport.bottom - viewport.top)
                {
                    caret.filter(|caret| !caret.empty()).unwrap_or(rect)
                } else {
                    rect
                };
                let (start, end, near, far) = if scroll.horizontal {
                    (measured.left, measured.right, viewport.left, viewport.right)
                } else {
                    (measured.top, measured.bottom, viewport.top, viewport.bottom)
                };
                offset = reveal_offset(start, end, near, far, offset, scroll.max_offset());
            }
            if scroll.follow_focus.is_some() {
                self.adjust(scroll, offset);
            }
            rect.translate(scroll.horizontal, -i64::from(offset));
            rect = rect.intersection(scroll.viewport.into());
            rect = scroll.parent_clip.apply_signed(rect);
            if let Some(caret) = &mut caret {
                caret.translate(scroll.horizontal, -i64::from(offset));
                *caret = caret.intersection(scroll.viewport.into());
                *caret = scroll.parent_clip.apply_signed(*caret);
            }
            parent = scroll.parent;
        }
        !self.pending.is_empty()
    }

    fn adjust(&mut self, scroll: ScrollGeometry, to: u32) {
        let id = scroll.state_id;
        if id == 0 {
            return;
        }
        if scroll.offset == to {
            return;
        }
        self.pending.insert(
            id,
            ScrollAdjustment {
                from: scroll.offset,
                to,
                horizontal: scroll.horizontal,
            },
        );
    }
}

fn reveal_offset(start: i64, end: i64, near: i64, far: i64, offset: u32, maximum: u32) -> u32 {
    if far <= near || end <= start {
        return offset;
    }
    let offset = i64::from(offset);
    let next = if end - start > far - near || start < near + offset {
        start - near
    } else if end > far + offset {
        end - far
    } else {
        offset
    };
    next.clamp(0, i64::from(maximum)) as u32
}

#[derive(Clone, Copy)]
struct NavigationRect {
    left: i64,
    top: i64,
    right: i64,
    bottom: i64,
}
impl From<Rect> for NavigationRect {
    fn from(rect: Rect) -> Self {
        Self {
            left: i64::from(rect.x),
            top: i64::from(rect.y),
            right: i64::from(rect.x) + i64::from(rect.width),
            bottom: i64::from(rect.y) + i64::from(rect.height),
        }
    }
}
impl NavigationRect {
    fn empty(self) -> bool {
        self.left >= self.right || self.top >= self.bottom
    }
    fn intersection(self, other: Self) -> Self {
        Self {
            left: self.left.max(other.left),
            top: self.top.max(other.top),
            right: self.right.min(other.right),
            bottom: self.bottom.min(other.bottom),
        }
    }
    fn translate(&mut self, horizontal: bool, amount: i64) {
        if horizontal {
            self.left += amount;
            self.right += amount;
        } else {
            self.top += amount;
            self.bottom += amount;
        }
    }
}

pub(crate) fn apply_scroll_adjustments(
    node: &mut LayoutNode,
    pending: &std::collections::HashMap<u64, ScrollAdjustment>,
) {
    if node.is_scrollable
        && let Some(adjustment) = pending.get(&node.scroll_state_id).copied()
    {
        if adjustment.horizontal {
            node.scroll_offset_x = adjustment.to;
        } else {
            node.scroll_offset = adjustment.to;
        }
    }
    for child in &mut node.children {
        apply_scroll_adjustments(child, pending);
    }
    for overlay in &mut node.overlays {
        apply_scroll_adjustments(&mut overlay.node, pending);
    }
}
