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
            && !(node.truncate && UnicodeWidthStr::width(content) > node.size.0 as usize)
        {
            let width = u32::try_from(UnicodeWidthStr::width(content)).unwrap_or(u32::MAX);
            let align = match node.align {
                Align::Start => 0,
                Align::Center => node.size.0.saturating_sub(width) / 2,
                Align::End => node.size.0.saturating_sub(width),
            };
            let cursor = content
                .graphemes(true)
                .take(offset)
                .map(|part| u32::try_from(UnicodeWidthStr::width(part)).unwrap_or(u32::MAX))
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
            self.scrolls.push(ScrollGeometry {
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
    }

    pub(crate) fn prepare(&mut self, focused: Option<usize>, geometry: &GeometryFeedback) -> bool {
        self.pending.clear();
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
