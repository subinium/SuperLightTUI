use super::*;

/// Regression guard for the size of [`LayoutNode`] (issues #153, #488).
///
/// A frame may build hundreds of layout nodes, and `LayoutNode` is moved /
/// recursed over throughout the layout pipeline. Text-only fields live in
/// [`TextNodeData`] behind a `Box` (#153), so non-text variants (`Spacer`,
/// `Container`, `RawDraw`), which are the vast majority of nodes, pay only
/// the 8-byte `Option<Box<TextNodeData>>`.
///
/// History: 432 B before the #153 split; grown by scalar layout fields for
/// flex-wrap / flex-basis (#258), horizontal scroll (#247), the per-frame
/// intrinsic-size memo (v0.22) and bound scroll identity (v0.25) to 368 B.
///
/// v0.26 (#488) lowered the bound to 336 B: the text-only `link_url`
/// moved into [`TextNodeData`] (24 B), and the three intrinsic-size memos
/// switched from `Cell<Option<..>>` to `u32::MAX` sentinels (28 B -> 16 B),
/// which pays for the new 4-byte [`LayoutNode::arena_slot`]. If a future
/// field pushes past the bound, move it to a side payload or audit whether
/// it needs to live on `LayoutNode` at all.
const _ASSERT_LAYOUT_NODE_SIZE: () = assert!(std::mem::size_of::<LayoutNode>() <= 336);

/// Sentinel for an empty intrinsic-size memo slot (#488).
///
/// A measured value of `u32::MAX` (saturated) is indistinguishable from the
/// sentinel and is simply recomputed on the next query, so the memo stays
/// byte-identical to the uncached computation.
const NO_MEMO: u32 = u32::MAX;

#[derive(Debug, Clone)]
pub(crate) struct OverlayLayer {
    pub(crate) node: LayoutNode,
    pub(crate) modal: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NodeKind {
    Text,
    Container(Direction),
    Spacer,
    RawDraw(usize),
}

/// Text-only data for [`NodeKind::Text`] nodes (issue #153).
///
/// These fields are unused by `Spacer`, `Container`, and `RawDraw`
/// nodes, so they sit behind a `Box` on `LayoutNode` to keep the hot
/// non-text paths small. The boxes are recycled across frames through
/// [`LayoutArena`] (#488), so the indirection costs no steady-state
/// allocation.
#[derive(Debug, Clone, Default)]
pub(crate) struct TextNodeData {
    pub(crate) content: Option<String>,
    /// Display width of `content` (or the summed width of `segments`),
    /// measured once when the node is built (#494). Render reuses it for
    /// alignment and clipping instead of re-measuring the string.
    pub(crate) width: u32,
    pub(crate) cursor_offset: Option<usize>,
    pub(crate) cursor_masked: bool,
    pub(crate) cached_wrap_width: Option<u32>,
    pub(crate) cached_wrapped: Option<Vec<String>>,
    pub(crate) segments: Option<Vec<(String, Style)>>,
    /// Display width of each entry of `segments`, measured at build (#494)
    /// so render can chain unwrapped segments without re-measuring them.
    /// Recycled with the box, so it costs no steady-state allocation.
    pub(crate) segment_widths: Vec<u32>,
    pub(crate) cached_wrapped_segments: Option<Vec<Vec<(String, Style)>>>,
    /// OSC 8 hyperlink target of a `link` text node. Text-only, so it lives
    /// here rather than on every `LayoutNode` (#488).
    pub(crate) link_url: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct LayoutNode {
    pub(crate) kind: NodeKind,
    /// Text-only payload. `Some` only for `NodeKind::Text` nodes; always
    /// `None` for `Spacer`, `Container`, and `RawDraw`. See
    /// [`TextNodeData`] for the rationale behind boxing.
    pub(crate) text_data: Option<Box<TextNodeData>>,
    pub(crate) style: Style,
    pub(crate) grow: u16,
    /// Opt-in flex-shrink flag. Default `false`.
    ///
    /// Set by `build_children` when a [`Command::ShrinkMarker`] precedes the
    /// node's `Begin*` command. Read by [`super::flexbox::layout_row`] /
    /// `layout_column` to scale this child's contribution proportionally
    /// when the parent overflows. Children without the flag keep their
    /// historic overflow-by-design width / height. Closes #161.
    pub(crate) shrink: bool,
    pub(crate) align: Align,
    pub(crate) align_self: Option<Align>,
    pub(crate) justify: Justify,
    pub(crate) wrap: bool,
    /// Opt-in container-level flex-wrap flag. Default `false`.
    ///
    /// Set by `build_children` when a [`Command::WrapMarker`] precedes the
    /// node's `Begin*` command. Read by [`super::flexbox::layout_row`]: when
    /// set, row children that overflow the available width flow onto
    /// subsequent lines (multi-line row) rather than overflowing past the
    /// right edge. Applies to `Direction::Row` only; a no-op for columns.
    /// Distinct from [`LayoutNode::wrap`], which is text line-wrapping.
    /// Closes #258.
    pub(crate) wrap_children: bool,
    pub(crate) truncate: bool,
    /// Inter-child gap on the main axis, in cells.
    ///
    /// Signed: a negative value (set via
    /// [`ContainerBuilder::gap_overlap`](crate::ContainerBuilder::gap_overlap))
    /// makes adjacent children overlap by `-gap` cells, e.g. so two bordered
    /// panels share a border column/row. Positive values space children apart
    /// as usual. Same 4-byte size as the previous `u32` — no layout-node
    /// budget impact (#222).
    pub(crate) gap: i32,
    /// Cross-axis (between-line) gap for a wrapping row, in cells.
    ///
    /// Only meaningful when [`LayoutNode::wrap_children`] is set on a
    /// `Direction::Row` container. Resolves to `row_gap` when set, else the
    /// main-axis `gap`. Within-line spacing continues to use
    /// [`LayoutNode::gap`]. Closes #258.
    pub(crate) cross_gap: i32,
    /// Optional flex-basis: the initial main-axis size (in cells) that
    /// `grow` grows from and `shrink` (#161) shrinks from.
    ///
    /// Stored as a `u32` with [`LayoutNode::NO_BASIS`] (`u32::MAX`) meaning
    /// "unset" — a sentinel rather than `Option<u32>` so the field is 4 bytes,
    /// keeping `LayoutNode` within its size budget (the niche of
    /// `Option<u32>` would cost 8). Unset falls back to
    /// [`LayoutNode::min_width`] (the historic base size), so unflagged
    /// children keep their current sizing. Read via [`LayoutNode::flex_basis`].
    /// Set by `build_children` from a [`Command::BasisMarker`]. Closes #258.
    pub(crate) flex_basis_raw: u32,
    pub(crate) border: Option<Border>,
    pub(crate) border_sides: BorderSides,
    pub(crate) border_style: Style,
    pub(crate) bg_color: Option<Color>,
    pub(crate) padding: Padding,
    pub(crate) margin: Margin,
    pub(crate) constraints: Constraints,
    pub(crate) title: Option<(String, Style)>,
    pub(crate) children: Vec<LayoutNode>,
    pub(crate) pos: (u32, u32),
    pub(crate) size: (u32, u32),
    pub(crate) is_scrollable: bool,
    pub(crate) scroll_follow_focus: Option<bool>,
    pub(crate) scroll_state_id: u64,
    pub(crate) scroll_offset: u32,
    pub(crate) content_height: u32,
    /// Horizontal scroll offset in cells (#247).
    ///
    /// The x-axis mirror of [`LayoutNode::scroll_offset`]. Non-zero only for a
    /// scrollable `Direction::Row` container; render and collect subtract it
    /// from child x-positions exactly as `scroll_offset` is subtracted on the
    /// y-axis.
    pub(crate) scroll_offset_x: u32,
    /// Total content width in cells for a scrollable row (#247).
    ///
    /// The x-axis mirror of [`LayoutNode::content_height`]. Set by
    /// `flexbox::compute` to the natural width of the children when a
    /// scrollable row overflows its viewport; `0` for every non-scrollable
    /// container and for scrollable columns.
    pub(crate) content_width: u32,
    /// [`LayoutArena`] slot whose recycled `children` vector this container
    /// borrowed (#488), or [`LayoutArena::NO_SLOT`] for nodes built outside
    /// an arena. Returned to the same slot at frame end, so a stable tree
    /// gets every vector back at the capacity it needed last frame.
    pub(crate) arena_slot: u32,
    pub(crate) focus_id: Option<usize>,
    pub(crate) interaction_id: Option<usize>,
    /// Group name for hover/focus registration.
    ///
    /// Stored as `Arc<str>` so the collect-side handoff into
    /// `FrameData.group_rects` / `FrameData.focus_groups` is a pointer bump
    /// rather than a fresh `String` → `Arc<str>` allocation per group node.
    pub(crate) group_name: Option<std::sync::Arc<str>>,
    pub(crate) overlays: Vec<OverlayLayer>,
    /// Per-frame memo for [`LayoutNode::min_width`]; [`NO_MEMO`] until the
    /// first query. The value is width-independent, so it never needs a key.
    /// The tree is rebuilt every frame, so a new node starts empty.
    pub(crate) cached_min_width: std::cell::Cell<u32>,
    /// Per-frame memo for [`LayoutNode::min_height`]. Same width-independent
    /// semantics as [`LayoutNode::cached_min_width`].
    pub(crate) cached_min_height: std::cell::Cell<u32>,
    /// Per-frame memo for [`LayoutNode::min_height_for_width`] as
    /// `(available_width, result)`; empty while the width is [`NO_MEMO`]. A
    /// query at a different width recomputes and overwrites (the flex
    /// pipeline queries each node at one settled width, so this is
    /// effectively a one-slot cache).
    pub(crate) cached_min_height_for_width: std::cell::Cell<(u32, u32)>,
}

#[derive(Debug, Clone)]
pub(crate) struct ContainerConfig {
    /// See [`LayoutNode::gap`] — signed inter-child gap (negative = overlap).
    pub(crate) gap: i32,
    pub(crate) align: Align,
    pub(crate) align_self: Option<Align>,
    pub(crate) justify: Justify,
    pub(crate) border: Option<Border>,
    pub(crate) border_sides: BorderSides,
    pub(crate) border_style: Style,
    pub(crate) bg_color: Option<Color>,
    pub(crate) padding: Padding,
    pub(crate) margin: Margin,
    pub(crate) constraints: Constraints,
    pub(crate) title: Option<(String, Style)>,
    pub(crate) grow: u16,
}

/// Display width of a text node's content, saturated into `u32`.
#[inline]
fn text_width(text: &str) -> u32 {
    u32::try_from(crate::cell::display_width(text)).unwrap_or(u32::MAX)
}

impl LayoutNode {
    /// Sentinel value for [`LayoutNode::flex_basis_raw`] meaning "no basis
    /// set" (fall back to `min_width`). Chosen as `u32::MAX` because a real
    /// flex-basis that large is never a meaningful main-axis size. Closes #258.
    pub(crate) const NO_BASIS: u32 = u32::MAX;

    /// The resolved flex-basis, or `None` when unset (`NO_BASIS`).
    ///
    /// Reads the [`LayoutNode::flex_basis_raw`] sentinel field. Closes #258.
    #[inline]
    pub(crate) fn flex_basis(&self) -> Option<u32> {
        if self.flex_basis_raw == Self::NO_BASIS {
            None
        } else {
            Some(self.flex_basis_raw)
        }
    }

    /// Get a shared reference to the text-only payload.
    ///
    /// Returns `None` for non-text variants. Use this everywhere the
    /// caller only reads text fields (e.g. `render_inner`).
    #[inline]
    pub(crate) fn text_data(&self) -> Option<&TextNodeData> {
        self.text_data.as_deref()
    }

    /// Get a mutable reference to the text-only payload.
    ///
    /// Panics if the node is not a `NodeKind::Text` node — callers are
    /// expected to check `kind` first or operate on a node they know to
    /// be text-shaped (e.g. inside `ensure_wrapped_for_width`).
    #[inline]
    pub(crate) fn text_data_mut(&mut self) -> &mut TextNodeData {
        self.text_data
            .as_deref_mut()
            .expect("text_data_mut called on non-text node")
    }

    /// A node of `kind` with every layout property at its default.
    ///
    /// The single field list every constructor starts from, so adding a field
    /// to `LayoutNode` only touches this function.
    fn blank(kind: NodeKind) -> Self {
        Self {
            kind,
            text_data: None,
            style: Style::new(),
            grow: 0,
            shrink: false,
            align: Align::Start,
            align_self: None,
            justify: Justify::Start,
            wrap: false,
            wrap_children: false,
            truncate: false,
            gap: 0,
            cross_gap: 0,
            flex_basis_raw: Self::NO_BASIS,
            border: None,
            border_sides: BorderSides::all(),
            border_style: Style::new(),
            bg_color: None,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            children: Vec::new(),
            pos: (0, 0),
            size: (0, 0),
            is_scrollable: false,
            scroll_follow_focus: None,
            scroll_state_id: 0,
            scroll_offset: 0,
            content_height: 0,
            scroll_offset_x: 0,
            content_width: 0,
            arena_slot: LayoutArena::NO_SLOT,
            focus_id: None,
            interaction_id: None,
            group_name: None,
            overlays: Vec::new(),
            cached_min_width: std::cell::Cell::new(NO_MEMO),
            cached_min_height: std::cell::Cell::new(NO_MEMO),
            cached_min_height_for_width: std::cell::Cell::new((NO_MEMO, 0)),
        }
    }

    /// A text node with fresh payload storage (tests build trees by hand).
    #[cfg(test)]
    pub(crate) fn text(
        content: String,
        style: Style,
        grow: u16,
        align: Align,
        text_meta: (Option<usize>, bool, bool),
        margin: Margin,
        constraints: Constraints,
    ) -> Self {
        Self::text_in(
            Box::default(),
            content,
            style,
            grow,
            align,
            text_meta,
            margin,
            constraints,
        )
    }

    /// [`LayoutNode::text`] reusing a recycled (reset) payload box (#488).
    #[expect(
        clippy::too_many_arguments,
        reason = "mirrors LayoutNode::text plus the recycled payload box"
    )]
    fn text_in(
        mut data: Box<TextNodeData>,
        content: String,
        style: Style,
        grow: u16,
        align: Align,
        text_meta: (Option<usize>, bool, bool),
        margin: Margin,
        constraints: Constraints,
    ) -> Self {
        let (cursor_offset, wrap, truncate) = text_meta;
        let width = text_width(&content);
        data.content = Some(content);
        data.width = width;
        data.cursor_offset = cursor_offset;
        let mut node = Self::blank(NodeKind::Text);
        node.text_data = Some(data);
        node.style = style;
        node.grow = grow;
        node.align = align;
        node.wrap = wrap;
        node.truncate = truncate;
        node.margin = margin;
        node.constraints = constraints;
        node.size = (width, 1);
        node
    }

    /// A rich-text node whose payload reuses a recycled (reset) box (#488).
    fn rich_text_in(
        mut data: Box<TextNodeData>,
        segments: Vec<(String, Style)>,
        wrap: bool,
        align: Align,
        margin: Margin,
        constraints: Constraints,
    ) -> Self {
        data.segment_widths.clear();
        data.segment_widths
            .extend(segments.iter().map(|(text, _)| text_width(text)));
        let width = data
            .segment_widths
            .iter()
            .fold(0u32, |total, &w| total.saturating_add(w));
        data.segments = Some(segments);
        data.width = width;
        let mut node = Self::blank(NodeKind::Text);
        node.text_data = Some(data);
        node.align = align;
        node.wrap = wrap;
        node.margin = margin;
        node.constraints = constraints;
        node.size = (width, 1);
        node
    }

    pub(crate) fn container(direction: Direction, config: ContainerConfig) -> Self {
        let mut node = Self::blank(NodeKind::Container(direction));
        node.grow = config.grow;
        node.align = config.align;
        node.align_self = config.align_self;
        node.justify = config.justify;
        node.gap = config.gap;
        node.border = config.border;
        node.border_sides = config.border_sides;
        node.border_style = config.border_style;
        node.bg_color = config.bg_color;
        node.padding = config.padding;
        node.margin = config.margin;
        node.constraints = config.constraints;
        node.title = config.title;
        node
    }

    /// Construct a `RawDraw` leaf node.
    ///
    /// The initial `size` is seeded from the constraints' minimum so the
    /// parent's `min_height_for_width` / `min_width` queries report the same
    /// values the original inline literal produced (#156).
    pub(crate) fn raw_draw(
        draw_id: usize,
        constraints: Constraints,
        grow: u16,
        margin: Margin,
        focus_id: Option<usize>,
        interaction_id: Option<usize>,
    ) -> Self {
        let mut node = Self::blank(NodeKind::RawDraw(draw_id));
        node.grow = grow;
        node.margin = margin;
        node.constraints = constraints;
        node.size = (
            constraints.min_width().unwrap_or(0),
            constraints.min_height().unwrap_or(0),
        );
        node.focus_id = focus_id;
        node.interaction_id = interaction_id;
        node
    }

    pub(crate) fn spacer(grow: u16) -> Self {
        let mut node = Self::blank(NodeKind::Spacer);
        node.grow = grow;
        node
    }

    pub(crate) fn border_inset(&self) -> u32 {
        if self.border.is_some() { 1 } else { 0 }
    }

    pub(crate) fn border_left_inset(&self) -> u32 {
        if self.border.is_some() && self.border_sides.left {
            1
        } else {
            0
        }
    }

    pub(crate) fn border_right_inset(&self) -> u32 {
        if self.border.is_some() && self.border_sides.right {
            1
        } else {
            0
        }
    }

    pub(crate) fn border_top_inset(&self) -> u32 {
        if self.border.is_some() && self.border_sides.top {
            1
        } else {
            0
        }
    }

    pub(crate) fn border_bottom_inset(&self) -> u32 {
        if self.border.is_some() && self.border_sides.bottom {
            1
        } else {
            0
        }
    }

    pub(crate) fn frame_horizontal(&self) -> u32 {
        self.padding
            .left
            .saturating_add(self.padding.right)
            .saturating_add(self.border_left_inset())
            .saturating_add(self.border_right_inset())
    }

    pub(crate) fn frame_vertical(&self) -> u32 {
        self.padding
            .top
            .saturating_add(self.padding.bottom)
            .saturating_add(self.border_top_inset())
            .saturating_add(self.border_bottom_inset())
    }

    #[inline]
    pub(crate) fn margin_horizontal(&self) -> u32 {
        self.margin.left.saturating_add(self.margin.right)
    }

    #[inline]
    pub(crate) fn margin_vertical(&self) -> u32 {
        self.margin.top.saturating_add(self.margin.bottom)
    }

    /// Width-independent intrinsic minimum width, memoized for the frame.
    ///
    /// The result is a pure function of this node's (post-build, immutable)
    /// subtree and its own constraints. The flexbox call sites query the same
    /// node's `min_width` repeatedly (base widths, cross-align, overflow sums),
    /// and `min_width` itself recurses over the whole subtree, so without the
    /// memo the cost is O(nodes x depth). The cache is cleared whenever a
    /// parent resolves this node's `Pct`/`Ratio` constraints into `Fixed`
    /// (see [`LayoutNode::invalidate_size_cache`]) — the only mutation that can
    /// change the result mid-frame — so the memoized value stays byte-identical
    /// to the uncached computation.
    pub(crate) fn min_width(&self) -> u32 {
        let cached = self.cached_min_width.get();
        if cached != NO_MEMO {
            return cached;
        }
        let width = self.min_width_uncached();
        self.cached_min_width.set(width);
        width
    }

    pub(crate) fn min_width_for_parent(&self, parent_width: u32) -> u32 {
        let width = match self.constraints.width {
            WidthSpec::Pct(percent) => u64::from(parent_width) * u64::from(percent.min(100)) / 100,
            WidthSpec::Ratio(_, 0) => u64::from(parent_width),
            WidthSpec::Ratio(num, den) => u64::from(parent_width) * u64::from(num) / u64::from(den),
            _ => return self.min_width(),
        };
        u32::try_from(width)
            .unwrap_or(u32::MAX)
            .saturating_add(self.margin_horizontal())
    }

    pub(crate) fn column_outer_width(&self, parent_width: u32, align: Align) -> u32 {
        match self.constraints.width {
            WidthSpec::Pct(_) | WidthSpec::Ratio(_, _) => self.min_width_for_parent(parent_width),
            _ if self.align_self.unwrap_or(align) == Align::Start => parent_width,
            _ => self.min_width().min(parent_width),
        }
    }

    fn min_width_uncached(&self) -> u32 {
        let width = match self.kind {
            NodeKind::Text => self.size.0,
            NodeKind::Spacer | NodeKind::RawDraw(_) => 0,
            NodeKind::Container(Direction::Row) => {
                let gaps: i64 = if self.children.is_empty() {
                    0
                } else {
                    (self.children.len() as i64 - 1) * self.gap as i64
                };
                let children_width = self.children.iter().fold(0u64, |total, child| {
                    total.saturating_add(u64::from(child.min_width()))
                });
                // `gaps` may be negative for overlap (#222); clamp the total at 0
                // so a small intrinsic width never wraps the `u32` subtraction.
                (i128::from(children_width) + i128::from(gaps))
                    .clamp(0, i128::from(u32::MAX))
                    .try_into()
                    .unwrap_or(u32::MAX)
                    .saturating_add(self.frame_horizontal())
            }
            NodeKind::Container(Direction::Column) => self
                .children
                .iter()
                .map(|c| c.min_width())
                .max()
                .unwrap_or(0)
                .saturating_add(self.frame_horizontal()),
        };

        let width = width.max(self.constraints.min_width().unwrap_or(0));
        let width = match self.constraints.max_width() {
            Some(max_w) => width.min(max_w),
            None => width,
        };
        width.saturating_add(self.margin_horizontal())
    }

    /// Clear this node's per-frame intrinsic-size memos.
    ///
    /// Called by the flexbox layout loops at the exact point a parent resolves
    /// a child's `Pct`/`Ratio` constraints into concrete `Fixed` values
    /// (`flexbox::resolve_axis_specs`). That resolution is the only mid-frame
    /// mutation that can change `min_width` / `min_height` /
    /// `min_height_for_width`, so discarding any value cached during an
    /// ancestor's measurement pass (when the constraint was still unresolved)
    /// keeps the post-resolution query byte-identical to the uncached path.
    /// For the common `Auto` / `Fixed` / `MinMax` case the resolution is a
    /// no-op and the clear is cheap (three `Cell::set(None)`).
    #[inline]
    pub(crate) fn invalidate_size_cache(&self) {
        self.cached_min_width.set(NO_MEMO);
        self.cached_min_height.set(NO_MEMO);
        self.cached_min_height_for_width.set((NO_MEMO, 0));
    }

    /// Width-independent intrinsic minimum height, memoized for the frame.
    ///
    /// Same caching contract as [`LayoutNode::min_width`].
    pub(crate) fn min_height(&self) -> u32 {
        let cached = self.cached_min_height.get();
        if cached != NO_MEMO {
            return cached;
        }
        let height = self.min_height_uncached();
        self.cached_min_height.set(height);
        height
    }

    fn min_height_uncached(&self) -> u32 {
        let height = match self.kind {
            NodeKind::Text => 1,
            NodeKind::Spacer | NodeKind::RawDraw(_) => 0,
            NodeKind::Container(Direction::Row) => self
                .children
                .iter()
                .map(|c| c.min_height())
                .max()
                .unwrap_or(0)
                .saturating_add(self.frame_vertical()),
            NodeKind::Container(Direction::Column) => {
                let gaps: i64 = if self.children.is_empty() {
                    0
                } else {
                    (self.children.len() as i64 - 1) * self.gap as i64
                };
                let children_height = self.children.iter().fold(0u64, |total, child| {
                    total.saturating_add(u64::from(child.min_height()))
                });
                // `gaps` may be negative for overlap (#222); clamp at 0.
                (i128::from(children_height) + i128::from(gaps))
                    .clamp(0, i128::from(u32::MAX))
                    .try_into()
                    .unwrap_or(u32::MAX)
                    .saturating_add(self.frame_vertical())
            }
        };

        self.constrain_outer_height(height)
    }

    fn constrain_outer_height(&self, height: u32) -> u32 {
        height
            .clamp(
                self.constraints.min_height().unwrap_or(0),
                self.constraints.max_height().unwrap_or(u32::MAX),
            )
            .saturating_add(self.margin_vertical())
    }

    pub(crate) fn ensure_wrapped_for_width(&mut self, available_width: u32) -> u32 {
        // `ensure_wrapped_for_width` is only called for `NodeKind::Text` nodes
        // (gated by `compute_body` and `min_height_for_width`), so `text_data`
        // is guaranteed to be `Some`. Unwrap once at the top to avoid threading
        // mutable borrows across multiple field reads/writes below.
        let td = self.text_data_mut();
        if td.cached_wrap_width == Some(available_width) {
            if let Some(ref segs) = td.cached_wrapped_segments {
                return segs.len().max(1) as u32;
            }
            if let Some(ref lines) = td.cached_wrapped {
                return lines.len().max(1) as u32;
            }
        }

        if let Some(ref segs) = td.segments {
            let wrapped = wrap_segments(segs, available_width);
            let line_count = wrapped.len().max(1) as u32;
            td.cached_wrap_width = Some(available_width);
            td.cached_wrapped_segments = Some(wrapped);
            td.cached_wrapped = None;
            line_count
        } else {
            let text = td.content.as_deref().unwrap_or("");
            let lines = wrap_lines(text, available_width);
            let line_count = lines.len().max(1) as u32;
            td.cached_wrap_width = Some(available_width);
            td.cached_wrapped = Some(lines);
            td.cached_wrapped_segments = None;
            line_count
        }
    }

    pub(crate) fn min_height_for_width(&mut self, available_width: u32) -> u32 {
        let width = available_width
            .saturating_sub(self.margin_horizontal())
            .clamp(
                self.constraints.min_width().unwrap_or(0),
                self.constraints.max_width().unwrap_or(u32::MAX),
            );
        if matches!(self.kind, NodeKind::Text) && self.wrap {
            let lines = self.ensure_wrapped_for_width(width);
            return self.constrain_outer_height(lines);
        }
        let (memo_width, memo_height) = self.cached_min_height_for_width.get();
        if memo_width != NO_MEMO && memo_width == available_width {
            return memo_height;
        }
        let inner_width = width.saturating_sub(self.frame_horizontal());
        let content_height = match self.kind {
            NodeKind::Container(Direction::Column) => {
                let mut total = 0u64;
                for child in &mut self.children {
                    let width = child.column_outer_width(inner_width, self.align);
                    total = total.saturating_add(u64::from(child.min_height_for_width(width)));
                }
                let gaps = self.children.len().saturating_sub(1) as i128 * i128::from(self.gap);
                (i128::from(total) + gaps).clamp(0, i128::from(u32::MAX)) as u32
            }
            NodeKind::Container(Direction::Row) if self.wrap_children => {
                let ranges = super::flexbox::row_ranges(&self.children, inner_width, self.gap);
                let mut total = 0u64;
                for &(start, end) in &ranges {
                    total = total.saturating_add(u64::from(super::flexbox::row_line_height(
                        &mut self.children[start..end],
                        inner_width,
                        self.gap,
                    )));
                }
                let gaps = ranges.len().saturating_sub(1) as i128 * i128::from(self.cross_gap);
                (i128::from(total) + gaps).clamp(0, i128::from(u32::MAX)) as u32
            }
            NodeKind::Container(Direction::Row) => {
                let width = if self.is_scrollable {
                    self.min_width()
                        .saturating_sub(self.margin_horizontal())
                        .saturating_sub(self.frame_horizontal())
                        .max(inner_width)
                } else {
                    inner_width
                };
                super::flexbox::row_line_height(&mut self.children, width, self.gap)
            }
            _ => return self.min_height(),
        };
        let height =
            self.constrain_outer_height(content_height.saturating_add(self.frame_vertical()));
        self.cached_min_height_for_width
            .set((available_width, height));
        height
    }
}

pub(crate) fn wrap_lines(text: &str, max_width: u32) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    if max_width == 0 {
        // No width budget: honor hard breaks only, and never let a control
        // char reach a cell. `split('\n')` (not `str::lines()`) keeps a trailing
        // empty line so every '\n' opens a fresh line.
        return text
            .split('\n')
            .map(|p| p.strip_suffix('\r').unwrap_or(p).to_string())
            .collect();
    }

    // Words and chunks are referred to by byte ranges `(start, end)` into `text`,
    // avoiding any intermediate `String` allocations per word. The final `String`
    // for each line is built exactly once at line-flush time.
    //
    // Fills `chunk_buf` with `((start, end), width)` pairs covering the word at
    // `word_start..word_end` when that word is wider than `max_width`.
    fn split_long_word(
        text: &str,
        word_start: usize,
        word_end: usize,
        max_width: u32,
        out: &mut Vec<((usize, usize), u32)>,
    ) {
        out.clear();
        let slice = &text[word_start..word_end];
        let mut chunk_start = word_start;
        let mut chunk_end = word_start;
        let mut chunk_width: u32 = 0;

        // Chunk at grapheme-cluster boundaries: a cluster (ZWJ flag, family
        // emoji, Indic / Thai syllable) is never sliced. A cluster wider than
        // `max_width` is emitted whole on its own chunk, mirroring the
        // single-wide-char behavior.
        for (rel_i, g) in slice.grapheme_indices(true) {
            let abs_i = word_start + rel_i;
            let ch_width = crate::cell::display_width(g) as u32;
            let ch_len = g.len();

            if chunk_end == chunk_start {
                if ch_width > max_width {
                    out.push(((abs_i, abs_i + ch_len), ch_width));
                    chunk_start = abs_i + ch_len;
                    chunk_end = abs_i + ch_len;
                    chunk_width = 0;
                } else {
                    chunk_start = abs_i;
                    chunk_end = abs_i + ch_len;
                    chunk_width = ch_width;
                }
                continue;
            }

            if chunk_width.saturating_add(ch_width) > max_width {
                out.push(((chunk_start, chunk_end), chunk_width));
                if ch_width > max_width {
                    out.push(((abs_i, abs_i + ch_len), ch_width));
                    chunk_start = abs_i + ch_len;
                    chunk_end = abs_i + ch_len;
                    chunk_width = 0;
                } else {
                    chunk_start = abs_i;
                    chunk_end = abs_i + ch_len;
                    chunk_width = ch_width;
                }
            } else {
                chunk_end = abs_i + ch_len;
                chunk_width = chunk_width.saturating_add(ch_width);
            }
        }

        if chunk_end > chunk_start {
            out.push(((chunk_start, chunk_end), chunk_width));
        }
    }

    // Materialize the current line's word ranges into a single `String`,
    // allocated once at the right capacity, then push it to `lines`.
    fn flush_line(
        text: &str,
        lines: &mut Vec<String>,
        current_line_words: &mut Vec<(usize, usize)>,
    ) {
        if current_line_words.is_empty() {
            return;
        }
        let n = current_line_words.len();
        let mut total_bytes = n - 1; // single-space separators
        for &(start, end) in current_line_words.iter() {
            total_bytes += end - start;
        }
        let mut s = String::with_capacity(total_bytes);
        for (i, &(start, end)) in current_line_words.iter().enumerate() {
            if i > 0 {
                s.push(' ');
            }
            s.push_str(&text[start..end]);
        }
        lines.push(s);
        current_line_words.clear();
    }

    // Append a word that is known to fit within `max_width` to the current line,
    // flushing the line first if the word would overflow.
    #[expect(
        clippy::too_many_arguments,
        reason = "wrap kernel threads line buffers and running widths without allocation"
    )]
    fn append_fitting_word(
        text: &str,
        lines: &mut Vec<String>,
        current_line_words: &mut Vec<(usize, usize)>,
        current_width: &mut u32,
        word_start: usize,
        word_end: usize,
        word_width: u32,
        max_width: u32,
    ) {
        if current_line_words.is_empty() {
            current_line_words.push((word_start, word_end));
            *current_width = word_width;
        } else if current_width.saturating_add(1).saturating_add(word_width) <= max_width {
            current_line_words.push((word_start, word_end));
            *current_width = current_width.saturating_add(1).saturating_add(word_width);
        } else {
            flush_line(text, lines, current_line_words);
            current_line_words.push((word_start, word_end));
            *current_width = word_width;
        }
    }

    // Handle a completed word (skipping empty ranges). Words wider than
    // `max_width` are split into sub-chunks via `split_long_word`.
    #[expect(
        clippy::too_many_arguments,
        reason = "wrap kernel threads line buffers and running widths without allocation"
    )]
    fn push_word(
        text: &str,
        lines: &mut Vec<String>,
        current_line_words: &mut Vec<(usize, usize)>,
        current_width: &mut u32,
        chunk_buf: &mut Vec<((usize, usize), u32)>,
        word_start: usize,
        word_end: usize,
        word_width: u32,
        max_width: u32,
    ) {
        if word_start == word_end {
            return;
        }
        if word_width > max_width {
            split_long_word(text, word_start, word_end, max_width, chunk_buf);
            // Copy each chunk descriptor out before calling into the appender so
            // nothing aliases `chunk_buf` across calls.
            for &((cs, ce), cw) in chunk_buf.iter() {
                append_fitting_word(
                    text,
                    lines,
                    current_line_words,
                    current_width,
                    cs,
                    ce,
                    cw,
                    max_width,
                );
            }
            return;
        }
        append_fitting_word(
            text,
            lines,
            current_line_words,
            current_width,
            word_start,
            word_end,
            word_width,
            max_width,
        );
    }

    let mut lines: Vec<String> = Vec::new();
    let mut chunk_buf: Vec<((usize, usize), u32)> = Vec::new();

    // Resolve hard line breaks first: split on '\n' (CRLF-normalized), then
    // soft-wrap each paragraph independently. `split('\n')` (not `str::lines()`)
    // is deliberate so a trailing '\n' yields a trailing empty line — every
    // '\n' opens a fresh line.
    for raw_paragraph in text.split('\n') {
        let paragraph = raw_paragraph.strip_suffix('\r').unwrap_or(raw_paragraph);
        let lines_before = lines.len();

        let mut current_line_words: Vec<(usize, usize)> = Vec::new();
        let mut current_width: u32 = 0;
        let mut word_start: usize = 0;
        let mut word_width: u32 = 0;

        // Iterate by grapheme cluster so word width accumulates per
        // user-perceived character; a greedy break can therefore never land
        // inside a cluster (ZWJ flag, family emoji, Indic / Thai syllable).
        // Space detection compares the cluster to a single-space string — a
        // bare ASCII space is its own cluster.
        for (i, g) in paragraph.grapheme_indices(true) {
            if g == " " {
                push_word(
                    paragraph,
                    &mut lines,
                    &mut current_line_words,
                    &mut current_width,
                    &mut chunk_buf,
                    word_start,
                    i,
                    word_width,
                    max_width,
                );
                word_start = i + 1; // ASCII space is 1 byte
                word_width = 0;
                continue;
            }
            word_width = word_width
                .saturating_add(u32::try_from(crate::cell::display_width(g)).unwrap_or(u32::MAX));
        }

        push_word(
            paragraph,
            &mut lines,
            &mut current_line_words,
            &mut current_width,
            &mut chunk_buf,
            word_start,
            paragraph.len(),
            word_width,
            max_width,
        );

        flush_line(paragraph, &mut lines, &mut current_line_words);

        // An empty paragraph (consecutive / leading / trailing '\n', or an
        // all-whitespace run that trims to nothing) contributes one blank line.
        if lines.len() == lines_before {
            lines.push(String::new());
        }
    }

    lines
}

/// Split a styled-segment run into paragraph groups on hard line breaks
/// (`'\n'`), normalizing `"\r\n"` to a single break. Each returned group is a
/// segment list with no embedded newlines.
fn split_segments_on_newline(segments: &[(String, Style)]) -> Vec<Vec<(String, Style)>> {
    let mut groups: Vec<Vec<(String, Style)>> = Vec::new();
    let mut cur: Vec<(String, Style)> = Vec::new();
    for (text, style) in segments {
        let pieces: Vec<&str> = text.split('\n').collect();
        let last_idx = pieces.len() - 1;
        for (idx, piece) in pieces.iter().enumerate() {
            let piece = piece.strip_suffix('\r').unwrap_or(piece);
            if !piece.is_empty() {
                cur.push((piece.to_string(), *style));
            }
            if idx < last_idx {
                // A '\n' followed this piece — close the current paragraph.
                groups.push(std::mem::take(&mut cur));
            }
        }
    }
    groups.push(cur);
    groups
}

/// Wrap styled segments to `max_width`, honoring embedded hard line breaks
/// (`'\n'`, with `"\r\n"` normalized) in addition to soft word wrapping. Hard
/// breaks are resolved first by splitting into paragraphs; each paragraph is
/// word-wrapped independently and the results are concatenated.
pub(crate) fn wrap_segments(
    segments: &[(String, Style)],
    max_width: u32,
) -> Vec<Vec<(String, Style)>> {
    if max_width == 0 || segments.is_empty() {
        return vec![vec![]];
    }
    if !segments.iter().any(|(seg_text, _)| !seg_text.is_empty()) {
        return vec![vec![]];
    }

    // Fast path: with no hard break anywhere, behave exactly like the single
    // paragraph kernel (and skip the split allocation) so output stays
    // byte-identical for the common no-newline case.
    if !segments.iter().any(|(seg_text, _)| seg_text.contains('\n')) {
        return wrap_segments_paragraph(segments, max_width);
    }

    let mut lines: Vec<Vec<(String, Style)>> = Vec::new();
    for group in split_segments_on_newline(segments) {
        // An empty / all-empty group yields `[[]]` (one blank line) from the
        // paragraph kernel — exactly what a bare '\n' should produce — so
        // unconditionally extend.
        lines.extend(wrap_segments_paragraph(&group, max_width));
    }
    if lines.is_empty() {
        vec![vec![]]
    } else {
        lines
    }
}

/// Word-wrap a single newline-free styled-segment paragraph to `max_width`.
fn wrap_segments_paragraph(
    segments: &[(String, Style)],
    max_width: u32,
) -> Vec<Vec<(String, Style)>> {
    if max_width == 0 || segments.is_empty() {
        return vec![vec![]];
    }

    // Fast bail-out: if every segment is empty there's no content to wrap.
    if !segments.iter().any(|(seg_text, _)| !seg_text.is_empty()) {
        return vec![vec![]];
    }

    // Advance the cursor past any fully-consumed / empty segments.
    fn advance_past_empty(segments: &[(String, Style)], cur_seg: &mut usize, cur_off: &mut usize) {
        while *cur_seg < segments.len() && *cur_off >= segments[*cur_seg].0.len() {
            *cur_seg += 1;
            *cur_off = 0;
        }
    }

    let mut lines: Vec<Vec<(String, Style)>> = Vec::new();

    // Iterator state into `segments`: (segment index, byte offset within segment).
    let mut cur_seg: usize = 0;
    let mut cur_off: usize = 0;
    advance_past_empty(segments, &mut cur_seg, &mut cur_off);

    // Issue #157: hoist the per-line scratch out of the outer loop so the
    // capacity hint is paid once per call instead of once per output line.
    // Each completed line is moved into `lines` via `mem::replace`, leaving
    // `line_segs` as a fresh `Vec::with_capacity(scratch_hint)` — the hint is
    // re-applied so the first push on the next line still skips the
    // grow-from-zero path. Most lines hold a small handful of style runs, so
    // the 16-cap clamp keeps over-allocation bounded when the input has a
    // long segment list that wraps into many short lines.
    let scratch_hint = segments.len().min(16);
    let mut line_segs: Vec<(String, Style)> = Vec::with_capacity(scratch_hint);

    while cur_seg < segments.len() {
        // For non-first lines, skip any leading spaces (matching the original).
        if !lines.is_empty() {
            loop {
                advance_past_empty(segments, &mut cur_seg, &mut cur_off);
                if cur_seg >= segments.len() {
                    break;
                }
                let s = segments[cur_seg].0.as_str();
                let g = s[cur_off..]
                    .graphemes(true)
                    .next()
                    .expect("advance_past_empty guarantees cur_off < s.len() with a valid cluster");
                if g == " " {
                    cur_off += 1; // ASCII space is 1 byte
                    continue;
                }
                break;
            }
            if cur_seg >= segments.len() {
                break;
            }
        }

        // `line_segs` is reused across iterations: at this point it is either
        // the fresh `Vec::with_capacity(scratch_hint)` allocated above (first
        // iteration) or the empty buffer left behind by the previous
        // iteration's `mem::replace`. Either way, len == 0 and the capacity
        // hint matches the issue #157 contract.
        debug_assert!(line_segs.is_empty());
        let mut line_width: u32 = 0;
        // Snapshot of the most recent space boundary on the current line:
        // (line_segs.len(), last seg's byte-length, line_width, space_seg_idx, space_byte_off).
        let mut last_space_break: Option<(usize, usize, u32, usize, usize)> = None;

        loop {
            advance_past_empty(segments, &mut cur_seg, &mut cur_off);
            if cur_seg >= segments.len() {
                break;
            }
            let s = segments[cur_seg].0.as_str();
            let style = segments[cur_seg].1;
            // Advance by grapheme cluster within the current segment so a
            // cluster (ZWJ emoji, combining sequence) never spans a wrap
            // break. Width is measured on the whole cluster.
            let g = s[cur_off..]
                .graphemes(true)
                .next()
                .expect("advance_past_empty guarantees cur_off < s.len() with a valid cluster");
            let ch_len = g.len();
            let ch_width = crate::cell::display_width(g) as u32;

            if line_width + ch_width > max_width && line_width > 0 {
                if let Some((segs_len, last_byte_len, _w, sp_seg, sp_off)) = last_space_break {
                    line_segs.truncate(segs_len);
                    if let Some(last) = line_segs.last_mut() {
                        last.0.truncate(last_byte_len);
                    }
                    // `line_width` is not read after this break — it is reset at the top of the outer loop.
                    cur_seg = sp_seg;
                    cur_off = sp_off + 1; // skip the space itself
                }
                break;
            }

            // Snapshot BEFORE pushing the space so we can roll back to a pre-space state.
            if g == " " {
                let segs_len = line_segs.len();
                let last_byte_len = line_segs.last().map(|(text, _)| text.len()).unwrap_or(0);
                last_space_break = Some((segs_len, last_byte_len, line_width, cur_seg, cur_off));
            }

            // Extend the last run if the style matches, otherwise start a new run.
            //
            // Issue #205: pre-size new style-run `String`s with
            // `with_capacity` so the first `push` does not realloc. We use
            // the byte count remaining in the source segment (`cur_off..len`)
            // capped at `max_width * 4` (worst-case UTF-8 bytes for a single
            // wrap-width line) to avoid over-allocation when one
            // same-style segment spans many wrap widths. The clamp is in
            // bytes — `String::with_capacity` is bytes — and the `.max(1)`
            // guarantees we never request a zero-capacity `String` (which
            // would re-trigger the very alloc we are eliminating).
            let segment_remaining = segments[cur_seg].0.len().saturating_sub(cur_off);
            let cap = segment_remaining
                .min((max_width as usize).saturating_mul(4))
                .max(1);
            if let Some(last) = line_segs.last_mut() {
                if last.1 == style {
                    last.0.push_str(g);
                } else {
                    let mut nw = String::with_capacity(cap);
                    nw.push_str(g);
                    line_segs.push((nw, style));
                }
            } else {
                let mut nw = String::with_capacity(cap);
                nw.push_str(g);
                line_segs.push((nw, style));
            }
            line_width = line_width.saturating_add(ch_width);
            cur_off += ch_len;
        }

        // End-of-line trim: match the original's single-level cascading trim.
        let cascade = if let Some(last) = line_segs.last_mut() {
            let trimmed_len = last.0.trim_end().len();
            if trimmed_len == 0 {
                true
            } else {
                last.0.truncate(trimmed_len);
                false
            }
        } else {
            false
        };
        if cascade {
            line_segs.pop();
            if let Some(last) = line_segs.last_mut() {
                let trimmed_len = last.0.trim_end().len();
                if trimmed_len == 0 {
                    line_segs.pop();
                } else {
                    last.0.truncate(trimmed_len);
                }
            }
        }

        // Move the finished line into `lines`. `mem::replace` hands `lines` a
        // ready-to-own `Vec` (no clone, no per-element copy) and leaves
        // `line_segs` empty with the capacity hint applied for the next
        // iteration's first push (issue #157).
        let line = std::mem::replace(&mut line_segs, Vec::with_capacity(scratch_hint));
        lines.push(line);
    }

    if lines.is_empty() {
        vec![vec![]]
    } else {
        lines
    }
}

/// Hard upper bound on layout-tree recursion depth.
///
/// Reached only by pathological input (e.g. a code generator emitting an
/// unbounded chain of `BeginContainer` commands or a recursive widget). A 2 MB
/// task stack overflows around depth ~5000 with no diagnostic message; an
/// explicit panic with a message at 512 is far more actionable than a silent
/// SIGSEGV. Normal TUI trees reach depth 5–15.
pub(crate) const MAX_LAYOUT_DEPTH: usize = 512;

/// Frame-to-frame storage for the layout tree (issue #488).
///
/// The tree is rebuilt from the command stream every frame. Without reuse,
/// every container allocates (and regrows) its `children` vector and every
/// text node boxes a fresh [`TextNodeData`]; that was more than half of a
/// static dashboard's per-frame allocations. The arena lives in
/// `FrameState` across frames: [`build_tree_in`] borrows vectors and boxes
/// from it, and [`LayoutArena::recycle`] returns them cleared (capacity
/// kept) once the frame no longer needs the tree.
///
/// Containers take `children` vectors by *slot*, numbered in command order,
/// and each node records its slot so recycling returns the vector to the
/// same slot. A tree with a stable shape therefore gets back exactly the
/// capacities it grew to last frame and allocates nothing in steady state;
/// a shape change only costs regrowth on the containers that moved. Node
/// order, overlay order and focus geometry are unchanged: the tree is the
/// same nested `LayoutNode` structure as before, only its storage is reused.
///
/// Retained memory is bounded: after [`LayoutArena::TRIM_AFTER_FRAMES`]
/// consecutive frames that use much less than the arena holds, the next
/// recycle drops unused slots and boxes and shrinks oversized vectors.
#[derive(Debug, Default)]
pub(crate) struct LayoutArena {
    /// Recycled `children` vectors, indexed by slot (empty while lent out).
    child_vecs: Vec<Vec<LayoutNode>>,
    /// Slots handed out by the current build.
    used_slots: usize,
    /// Reset text payload boxes ready for reuse. The boxes themselves are
    /// the recycled allocations (a node holds `Box<TextNodeData>`), so the
    /// pool must keep them boxed.
    #[expect(
        clippy::vec_box,
        reason = "the boxes are the recycled allocations a LayoutNode holds"
    )]
    text_pool: Vec<Box<TextNodeData>>,
    /// Text payloads handed out by the current build.
    used_text: usize,
    /// Recycled root overlay list.
    overlays: Vec<OverlayLayer>,
    /// Cleared text buffers of recycled trees, lent to the next frame's
    /// `Context` for widgets to format into (see [`StringPool`]).
    strings: StringPool,
    /// Consecutive recycles whose tree used much less than the arena holds.
    oversized_frames: u32,
}

impl LayoutArena {
    /// [`LayoutNode::arena_slot`] of a node whose `children` vector does not
    /// come from an arena.
    pub(crate) const NO_SLOT: u32 = u32::MAX;

    /// Consecutive oversized frames before the arena trims itself.
    pub(crate) const TRIM_AFTER_FRAMES: u32 = 120;

    /// Spare entries always kept, so small trees never trim or regrow.
    const SLACK: usize = 64;

    /// Lend `node` the recycled `children` vector of the next slot.
    fn attach_children(&mut self, node: &mut LayoutNode) {
        let slot = self.used_slots;
        let Ok(slot_id) = u32::try_from(slot) else {
            return;
        };
        if slot_id == Self::NO_SLOT {
            return;
        }
        self.used_slots += 1;
        node.arena_slot = slot_id;
        match self.child_vecs.get_mut(slot) {
            Some(children) => node.children = std::mem::take(children),
            None => self.child_vecs.push(Vec::new()),
        }
    }

    /// A reset text payload box, recycled when one is available.
    /// Lend the pooled text buffers to a frame's `Context`; hand them back
    /// with [`restore_strings`](Self::restore_strings) before
    /// [`recycle`](Self::recycle).
    pub(crate) fn take_strings(&mut self) -> StringPool {
        std::mem::take(&mut self.strings)
    }

    /// Return the buffers lent by [`take_strings`](Self::take_strings).
    pub(crate) fn restore_strings(&mut self, strings: StringPool) {
        self.strings = strings;
    }

    fn take_text(&mut self) -> Box<TextNodeData> {
        self.used_text += 1;
        self.text_pool.pop().unwrap_or_default()
    }

    /// Return the storage of a finished frame's tree to the arena.
    ///
    /// Every `children` vector goes back to its slot cleared but with its
    /// capacity, and every text payload is reset and pooled. Dropping the
    /// tree instead is always correct; it only forfeits the reuse.
    pub(crate) fn recycle(&mut self, mut root: LayoutNode) {
        let trim = self.oversized_frames >= Self::TRIM_AFTER_FRAMES;
        let mut overlays = std::mem::take(&mut root.overlays);
        let mut used_nodes = overlays.len();
        for layer in &mut overlays {
            used_nodes = used_nodes.saturating_add(self.harvest(&mut layer.node, trim));
        }
        overlays.clear();
        self.overlays = overlays;
        used_nodes = used_nodes.saturating_add(self.harvest(&mut root, trim));

        if trim {
            self.child_vecs.truncate(self.used_slots);
            self.child_vecs.shrink_to(self.used_slots.saturating_mul(2));
            self.text_pool.truncate(self.used_text);
            self.text_pool.shrink_to(self.used_text.saturating_mul(2));
            self.oversized_frames = 0;
        } else {
            let retained_nodes = self.child_vecs.iter().fold(0usize, |total, children| {
                total.saturating_add(children.capacity())
            });
            let oversized = self.child_vecs.len() > self.used_slots.saturating_mul(2) + Self::SLACK
                || self.text_pool.len() > self.used_text.saturating_mul(2) + Self::SLACK
                || retained_nodes > used_nodes.saturating_mul(4) + Self::SLACK;
            self.oversized_frames = if oversized {
                self.oversized_frames.saturating_add(1)
            } else {
                0
            };
        }
        self.used_slots = 0;
        self.used_text = 0;
    }

    /// Retained storage as `(slots, pooled text boxes, child capacity)`.
    #[cfg(test)]
    pub(crate) fn retained(&self) -> (usize, usize, usize) {
        let capacity = self
            .child_vecs
            .iter()
            .map(|children| children.capacity())
            .sum();
        (self.child_vecs.len(), self.text_pool.len(), capacity)
    }

    /// Recycle `node`'s subtree storage; returns the subtree's child count.
    fn harvest(&mut self, node: &mut LayoutNode, trim: bool) -> usize {
        if let Some((title, _)) = node.title.take() {
            self.strings.put(title);
        }
        if let Some(mut data) = node.text_data.take() {
            if let Some(content) = data.content.take() {
                self.strings.put(content);
            }
            if let Some(url) = data.link_url.take() {
                self.strings.put(url);
            }
            if let Some(segments) = data.segments.take() {
                for (text, _) in segments {
                    self.strings.put(text);
                }
            }
            let mut segment_widths = std::mem::take(&mut data.segment_widths);
            segment_widths.clear();
            if trim && segment_widths.capacity() > Self::SLACK {
                segment_widths = Vec::new();
            }
            *data = TextNodeData::default();
            data.segment_widths = segment_widths;
            self.text_pool.push(data);
        }
        let mut count = node.children.len();
        for child in &mut node.children {
            count = count.saturating_add(self.harvest(child, trim));
        }
        let slot = node.arena_slot as usize;
        if let Some(entry) = self.child_vecs.get_mut(slot)
            && entry.capacity() == 0
        {
            let len = node.children.len();
            let mut children = std::mem::take(&mut node.children);
            children.clear();
            if trim && children.capacity() > len.saturating_mul(4).max(Self::SLACK) {
                children.shrink_to(len.saturating_mul(2));
            }
            *entry = children;
        }
        count
    }
}

/// Build the layout tree from a recorded command stream.
///
/// Takes `&mut Vec<Command>` and consumes the contents via `drain(..)` so the
/// caller retains ownership of the allocation; after this returns,
/// `commands.len() == 0` but `commands.capacity()` is preserved. Callers that
/// route through [`crate::FrameState::commands_buf`] reclaim that capacity at
/// frame end (issue #150) so the per-frame `Vec::new` allocation churn is
/// amortized to one allocation across the session.
///
/// Uses fresh storage; the frame kernel calls [`build_tree_in`] with the
/// session's [`LayoutArena`] instead (#488).
pub(crate) fn build_tree(commands: &mut Vec<Command>) -> LayoutNode {
    build_tree_in(commands, &mut LayoutArena::default())
}

/// [`build_tree`] borrowing node storage from `arena` (#488). Hand the tree
/// back with [`LayoutArena::recycle`] when the frame is done with it.
pub(crate) fn build_tree_in(commands: &mut Vec<Command>, arena: &mut LayoutArena) -> LayoutNode {
    arena.used_slots = 0;
    arena.used_text = 0;
    let mut root = LayoutNode::container(Direction::Column, default_container_config());
    arena.attach_children(&mut root);
    let mut overlays = std::mem::take(&mut arena.overlays);
    overlays.clear();
    let mut iter = commands.drain(..);
    build_children(&mut root, &mut iter, &mut overlays, arena, false, 0);
    root.overlays = overlays;
    root
}

pub(crate) fn default_container_config() -> ContainerConfig {
    ContainerConfig {
        gap: 0,
        align: Align::Start,
        align_self: None,
        justify: Justify::Start,
        border: None,
        border_sides: BorderSides::all(),
        border_style: Style::new(),
        bg_color: None,
        padding: Padding::default(),
        margin: Margin::default(),
        constraints: Constraints::default(),
        title: None,
        grow: 0,
    }
}

fn build_children(
    parent: &mut LayoutNode,
    commands: &mut std::vec::Drain<'_, Command>,
    overlays: &mut Vec<OverlayLayer>,
    arena: &mut LayoutArena,
    stop_on_end_overlay: bool,
    depth: usize,
) {
    if depth > MAX_LAYOUT_DEPTH {
        panic!(
            "layout tree depth exceeds {MAX_LAYOUT_DEPTH}: \
             check for recursive container nesting"
        );
    }
    let mut pending_focus_id: Option<usize> = None;
    let mut pending_interaction_id: Option<usize> = None;
    // ShrinkMarker is buffered into `pending_shrink` and consumed by the next
    // container / scrollable node. Closes #161.
    let mut pending_shrink: bool = false;
    // WrapMarker / BasisMarker mirror the shrink-marker pattern: buffered here
    // and applied to the next container / scrollable node. `pending_wrap`
    // holds the cross-axis (between-line) gap carried by the marker. Closes #258.
    let mut pending_wrap: Option<i32> = None;
    let mut pending_basis: Option<u32> = None;
    while let Some(command) = commands.next() {
        match command {
            Command::FocusMarker(id) => pending_focus_id = Some(id),
            Command::InteractionMarker(id) => pending_interaction_id = Some(id),
            Command::ShrinkMarker => pending_shrink = true,
            Command::WrapMarker(cross_gap) => pending_wrap = Some(cross_gap),
            Command::BasisMarker(basis) => pending_basis = Some(basis),
            Command::Text {
                content,
                cursor_offset,
                cursor_masked,
                style,
                grow,
                align,
                wrap,
                truncate,
                margin,
                constraints,
            } => {
                let mut node = LayoutNode::text_in(
                    arena.take_text(),
                    content,
                    style,
                    grow,
                    align,
                    (cursor_offset, wrap, truncate),
                    margin,
                    constraints,
                );
                node.text_data_mut().cursor_masked = cursor_masked;
                node.focus_id = pending_focus_id.take();
                node.interaction_id = pending_interaction_id.take();
                parent.children.push(node);
            }
            Command::RichText {
                segments,
                wrap,
                align,
                margin,
                constraints,
            } => {
                let mut node = LayoutNode::rich_text_in(
                    arena.take_text(),
                    segments,
                    wrap,
                    align,
                    margin,
                    constraints,
                );
                node.focus_id = pending_focus_id.take();
                node.interaction_id = pending_interaction_id.take();
                parent.children.push(node);
            }
            Command::Link {
                text,
                url,
                style,
                wrap,
                margin,
                constraints,
            } => {
                let mut node = LayoutNode::text_in(
                    arena.take_text(),
                    text,
                    style,
                    0,
                    Align::Start,
                    (None, wrap, false),
                    margin,
                    constraints,
                );
                node.text_data_mut().link_url = Some(url);
                node.focus_id = pending_focus_id.take();
                node.interaction_id = pending_interaction_id.take();
                parent.children.push(node);
            }
            Command::BeginContainer(args) => {
                let BeginContainerArgs {
                    direction,
                    gap,
                    align,
                    align_self,
                    justify,
                    border,
                    border_sides,
                    border_style,
                    bg_color,
                    padding,
                    margin,
                    constraints,
                    title,
                    grow,
                    group_name,
                } = args;
                let mut node = LayoutNode::container(
                    direction,
                    ContainerConfig {
                        gap,
                        align,
                        align_self,
                        justify,
                        border,
                        border_sides,
                        border_style,
                        bg_color,
                        padding,
                        margin,
                        constraints,
                        title: title.map(|title| *title),
                        grow,
                    },
                );
                node.focus_id = pending_focus_id.take();
                node.interaction_id = pending_interaction_id.take();
                node.group_name = group_name;
                if pending_shrink {
                    node.shrink = true;
                    pending_shrink = false;
                }
                if let Some(cross_gap) = pending_wrap.take() {
                    node.wrap_children = true;
                    node.cross_gap = cross_gap;
                }
                if let Some(basis) = pending_basis.take() {
                    node.flex_basis_raw = basis;
                }
                arena.attach_children(&mut node);
                build_children(&mut node, commands, overlays, arena, false, depth + 1);
                parent.children.push(node);
            }
            Command::BeginScrollable(args) => {
                let BeginScrollableArgs {
                    grow,
                    direction,
                    border,
                    border_sides,
                    border_style,
                    bg_color,
                    align,
                    align_self,
                    justify,
                    gap,
                    padding,
                    margin,
                    constraints,
                    title,
                    scroll_offset,
                    scroll_offset_x,
                    scroll_follow_focus,
                    scroll_state_id,
                    group_name,
                } = args;
                // #247: honor the caller's `.row()` / `.col()` direction instead
                // of hardcoding `Direction::Column`. A `Row` scrollable scrolls
                // horizontally; a `Column` scrollable scrolls vertically. The
                // offset that applies depends on the axis — the cross-axis offset
                // is always 0 for a single-axis scroller.
                let mut node = LayoutNode::container(
                    direction,
                    ContainerConfig {
                        gap,
                        align,
                        align_self,
                        justify,
                        border,
                        border_sides,
                        border_style,
                        bg_color,
                        padding,
                        margin,
                        constraints,
                        title: title.map(|title| *title),
                        grow,
                    },
                );
                node.is_scrollable = true;
                node.scroll_follow_focus = scroll_follow_focus;
                node.scroll_state_id = scroll_state_id;
                match direction {
                    Direction::Column => node.scroll_offset = scroll_offset,
                    Direction::Row => node.scroll_offset_x = scroll_offset_x,
                }
                node.focus_id = pending_focus_id.take();
                node.interaction_id = pending_interaction_id.take();
                node.group_name = group_name;
                if pending_shrink {
                    node.shrink = true;
                    pending_shrink = false;
                }
                // Consume any pending wrap/basis markers so they don't leak to a
                // later sibling. Wrap is a no-op on a column scrollable; the
                // cross-axis gap is recorded for completeness; basis is recorded
                // but only consumed by row resolution.
                if let Some(cross_gap) = pending_wrap.take() {
                    node.wrap_children = true;
                    node.cross_gap = cross_gap;
                }
                if let Some(basis) = pending_basis.take() {
                    node.flex_basis_raw = basis;
                }
                arena.attach_children(&mut node);
                build_children(&mut node, commands, overlays, arena, false, depth + 1);
                parent.children.push(node);
            }
            Command::BeginOverlay { modal } => {
                let mut overlay_node =
                    LayoutNode::container(Direction::Column, default_container_config());
                arena.attach_children(&mut overlay_node);
                overlay_node.interaction_id = pending_interaction_id.take();
                // #459: reserve this layer's paint slot before its children,
                // so an overlay opened inside it (a select dropdown or menu in
                // a modal) paints and hit-tests above it, not below.
                let slot = overlays.len();
                overlays.push(OverlayLayer {
                    node: LayoutNode::container(Direction::Column, default_container_config()),
                    modal,
                });
                build_children(
                    &mut overlay_node,
                    commands,
                    overlays,
                    arena,
                    true,
                    depth + 1,
                );
                overlays[slot].node = overlay_node;
            }
            Command::Spacer { grow } => parent.children.push(LayoutNode::spacer(grow)),
            Command::RawDraw {
                draw_id,
                constraints,
                grow,
                margin,
            } => {
                let node = LayoutNode::raw_draw(
                    draw_id,
                    constraints,
                    grow,
                    margin,
                    pending_focus_id.take(),
                    pending_interaction_id.take(),
                );
                parent.children.push(node);
            }
            Command::EndContainer => return,
            Command::EndOverlay => {
                if stop_on_end_overlay {
                    return;
                }
            }
        }
    }
}
