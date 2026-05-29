use super::*;

/// Regression guard for the size of [`LayoutNode`] (issue #153).
///
/// A frame may build hundreds of layout nodes, and `LayoutNode` is moved /
/// recursed over throughout the layout pipeline. The text-only fields
/// (`content`, `cursor_offset`, `cached_*`, `segments`) are split into
/// [`TextNodeData`] behind a `Box`, so non-text variants (`Spacer`,
/// `Container`, `RawDraw`) — which are the vast majority of nodes — pay
/// only the 8-byte `Option<Box<TextNodeData>>` rather than ~120 bytes of
/// always-`None` fields inline. Pre-split the struct measured 432 bytes;
/// post-split it should be substantially smaller. If a future field
/// addition pushes this past the bound, either box the new field or audit
/// whether the addition needs to live on `LayoutNode` at all.
///
/// Bumped 320 → 328 for the flex-wrap / flex-basis fields (#258):
/// `cross_gap: i32` (4 bytes) and `flex_basis_raw: u32` (4 bytes), the two
/// new scalar layout properties read by `flexbox::layout_row`. The
/// `wrap_children: bool` flag packs into existing bool padding and adds
/// nothing. Both are scalars (no heap, no niche), so this is the minimum
/// footprint for the feature; boxing two 4-byte fields would cost a pointer
/// (8 bytes) plus an allocation per wrapping container, a net loss.
///
/// Bumped 328 → 336 for the horizontal-scroll fields (#247):
/// `scroll_offset_x: u32` (4 bytes) and `content_width: u32` (4 bytes), the
/// x-axis mirror of `scroll_offset` / `content_height`. Same scalar rationale
/// as #258 — boxing 4-byte fields would cost more than it saves.
const _ASSERT_LAYOUT_NODE_SIZE: () = assert!(std::mem::size_of::<LayoutNode>() <= 336);

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
/// All six fields are unused by `Spacer`, `Container`, and `RawDraw`
/// nodes, so we hide them behind a `Box` on `LayoutNode` to keep the
/// hot non-text paths small. Boxing is cheap because text nodes already
/// own at least one heap allocation (`content` or `segments`), so the
/// extra indirection costs one more allocation per text node in exchange
/// for ~120 bytes saved on every non-text node — a clear win when most
/// nodes are containers.
#[derive(Debug, Clone, Default)]
pub(crate) struct TextNodeData {
    pub(crate) content: Option<String>,
    pub(crate) cursor_offset: Option<usize>,
    pub(crate) cached_wrap_width: Option<u32>,
    pub(crate) cached_wrapped: Option<Vec<String>>,
    pub(crate) segments: Option<Vec<(String, Style)>>,
    pub(crate) cached_wrapped_segments: Option<Vec<Vec<(String, Style)>>>,
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
    pub(crate) focus_id: Option<usize>,
    pub(crate) interaction_id: Option<usize>,
    pub(crate) link_url: Option<String>,
    /// Group name for hover/focus registration.
    ///
    /// Stored as `Arc<str>` so the collect-side handoff into
    /// `FrameData.group_rects` / `FrameData.focus_groups` is a pointer bump
    /// rather than a fresh `String` → `Arc<str>` allocation per group node.
    pub(crate) group_name: Option<std::sync::Arc<str>>,
    pub(crate) overlays: Vec<OverlayLayer>,
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

    pub(crate) fn text(
        content: String,
        style: Style,
        grow: u16,
        align: Align,
        text_meta: (Option<usize>, bool, bool),
        margin: Margin,
        constraints: Constraints,
    ) -> Self {
        let (cursor_offset, wrap, truncate) = text_meta;
        let width = UnicodeWidthStr::width(content.as_str()) as u32;
        Self {
            kind: NodeKind::Text,
            text_data: Some(Box::new(TextNodeData {
                content: Some(content),
                cursor_offset,
                ..Default::default()
            })),
            style,
            grow,
            shrink: false,
            align,
            align_self: None,
            justify: Justify::Start,
            wrap,
            wrap_children: false,
            flex_basis_raw: Self::NO_BASIS,
            cross_gap: 0,
            truncate,
            gap: 0,
            border: None,
            border_sides: BorderSides::all(),
            border_style: Style::new(),
            bg_color: None,
            padding: Padding::default(),
            margin,
            constraints,
            title: None,
            children: Vec::new(),
            pos: (0, 0),
            size: (width, 1),
            is_scrollable: false,
            scroll_offset: 0,
            content_height: 0,
            scroll_offset_x: 0,
            content_width: 0,
            focus_id: None,
            interaction_id: None,
            link_url: None,
            group_name: None,
            overlays: Vec::new(),
        }
    }

    pub(crate) fn rich_text(
        segments: Vec<(String, Style)>,
        wrap: bool,
        align: Align,
        margin: Margin,
        constraints: Constraints,
    ) -> Self {
        let width: u32 = segments
            .iter()
            .map(|(s, _)| UnicodeWidthStr::width(s.as_str()) as u32)
            .sum();
        Self {
            kind: NodeKind::Text,
            text_data: Some(Box::new(TextNodeData {
                segments: Some(segments),
                ..Default::default()
            })),
            style: Style::new(),
            grow: 0,
            shrink: false,
            align,
            align_self: None,
            justify: Justify::Start,
            wrap,
            wrap_children: false,
            flex_basis_raw: Self::NO_BASIS,
            cross_gap: 0,
            truncate: false,
            gap: 0,
            border: None,
            border_sides: BorderSides::all(),
            border_style: Style::new(),
            bg_color: None,
            padding: Padding::default(),
            margin,
            constraints,
            title: None,
            children: Vec::new(),
            pos: (0, 0),
            size: (width, 1),
            is_scrollable: false,
            scroll_offset: 0,
            content_height: 0,
            scroll_offset_x: 0,
            content_width: 0,
            focus_id: None,
            interaction_id: None,
            link_url: None,
            group_name: None,
            overlays: Vec::new(),
        }
    }

    pub(crate) fn container(direction: Direction, config: ContainerConfig) -> Self {
        Self {
            kind: NodeKind::Container(direction),
            text_data: None,
            style: Style::new(),
            grow: config.grow,
            shrink: false,
            align: config.align,
            align_self: config.align_self,
            justify: config.justify,
            wrap: false,
            wrap_children: false,
            flex_basis_raw: Self::NO_BASIS,
            cross_gap: 0,
            truncate: false,
            gap: config.gap,
            border: config.border,
            border_sides: config.border_sides,
            border_style: config.border_style,
            bg_color: config.bg_color,
            padding: config.padding,
            margin: config.margin,
            constraints: config.constraints,
            title: config.title,
            children: Vec::new(),
            pos: (0, 0),
            size: (0, 0),
            is_scrollable: false,
            scroll_offset: 0,
            content_height: 0,
            scroll_offset_x: 0,
            content_width: 0,
            focus_id: None,
            interaction_id: None,
            link_url: None,
            group_name: None,
            overlays: Vec::new(),
        }
    }

    /// Construct a `RawDraw` leaf node.
    ///
    /// Mirrors the `text` / `rich_text` / `container` / `spacer` constructor
    /// pattern so that adding a field to `LayoutNode` only requires editing
    /// the constructors, not every call site in `build_children`. The initial
    /// `size` is seeded from the constraints' minimum so the parent's
    /// `min_height_for_width` / `min_width` queries report the same values
    /// the previous inline literal produced.
    pub(crate) fn raw_draw(
        draw_id: usize,
        constraints: Constraints,
        grow: u16,
        margin: Margin,
        focus_id: Option<usize>,
        interaction_id: Option<usize>,
    ) -> Self {
        Self {
            kind: NodeKind::RawDraw(draw_id),
            text_data: None,
            style: Style::new(),
            grow,
            shrink: false,
            align: Align::Start,
            align_self: None,
            justify: Justify::Start,
            wrap: false,
            wrap_children: false,
            flex_basis_raw: Self::NO_BASIS,
            cross_gap: 0,
            truncate: false,
            gap: 0,
            border: None,
            border_sides: BorderSides::all(),
            border_style: Style::new(),
            bg_color: None,
            padding: Padding::default(),
            margin,
            constraints,
            title: None,
            children: Vec::new(),
            pos: (0, 0),
            size: (
                constraints.min_width().unwrap_or(0),
                constraints.min_height().unwrap_or(0),
            ),
            is_scrollable: false,
            scroll_offset: 0,
            content_height: 0,
            scroll_offset_x: 0,
            content_width: 0,
            focus_id,
            interaction_id,
            link_url: None,
            group_name: None,
            overlays: Vec::new(),
        }
    }

    pub(crate) fn spacer(grow: u16) -> Self {
        Self {
            kind: NodeKind::Spacer,
            text_data: None,
            style: Style::new(),
            grow,
            shrink: false,
            align: Align::Start,
            align_self: None,
            justify: Justify::Start,
            wrap: false,
            wrap_children: false,
            flex_basis_raw: Self::NO_BASIS,
            cross_gap: 0,
            truncate: false,
            gap: 0,
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
            scroll_offset: 0,
            content_height: 0,
            scroll_offset_x: 0,
            content_width: 0,
            focus_id: None,
            interaction_id: None,
            link_url: None,
            group_name: None,
            overlays: Vec::new(),
        }
    }

    pub(crate) fn border_inset(&self) -> u32 {
        if self.border.is_some() {
            1
        } else {
            0
        }
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
        self.padding.horizontal() + self.border_left_inset() + self.border_right_inset()
    }

    pub(crate) fn frame_vertical(&self) -> u32 {
        self.padding.vertical() + self.border_top_inset() + self.border_bottom_inset()
    }

    pub(crate) fn min_width(&self) -> u32 {
        let width = match self.kind {
            NodeKind::Text => self.size.0,
            NodeKind::Spacer | NodeKind::RawDraw(_) => 0,
            NodeKind::Container(Direction::Row) => {
                let gaps: i64 = if self.children.is_empty() {
                    0
                } else {
                    (self.children.len() as i64 - 1) * self.gap as i64
                };
                let children_width: u32 = self.children.iter().map(|c| c.min_width()).sum();
                // `gaps` may be negative for overlap (#222); clamp the total at 0
                // so a small intrinsic width never wraps the `u32` subtraction.
                ((children_width as i64 + gaps).max(0) as u32) + self.frame_horizontal()
            }
            NodeKind::Container(Direction::Column) => {
                self.children
                    .iter()
                    .map(|c| c.min_width())
                    .max()
                    .unwrap_or(0)
                    + self.frame_horizontal()
            }
        };

        let width = width.max(self.constraints.min_width().unwrap_or(0));
        let width = match self.constraints.max_width() {
            Some(max_w) => width.min(max_w),
            None => width,
        };
        width.saturating_add(self.margin.horizontal())
    }

    pub(crate) fn min_height(&self) -> u32 {
        let height = match self.kind {
            NodeKind::Text => 1,
            NodeKind::Spacer | NodeKind::RawDraw(_) => 0,
            NodeKind::Container(Direction::Row) => {
                self.children
                    .iter()
                    .map(|c| c.min_height())
                    .max()
                    .unwrap_or(0)
                    + self.frame_vertical()
            }
            NodeKind::Container(Direction::Column) => {
                let gaps: i64 = if self.children.is_empty() {
                    0
                } else {
                    (self.children.len() as i64 - 1) * self.gap as i64
                };
                let children_height: u32 = self.children.iter().map(|c| c.min_height()).sum();
                // `gaps` may be negative for overlap (#222); clamp at 0.
                ((children_height as i64 + gaps).max(0) as u32) + self.frame_vertical()
            }
        };

        let height = height.max(self.constraints.min_height().unwrap_or(0));
        height.saturating_add(self.margin.vertical())
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
        match self.kind {
            NodeKind::Text if self.wrap => {
                let inner_width = available_width.saturating_sub(self.margin.horizontal());
                let lines = self.ensure_wrapped_for_width(inner_width);
                lines.saturating_add(self.margin.vertical())
            }
            // A wrapping row's height depends on how many lines its children
            // flow onto at `available_width`, so it cannot be derived from the
            // width-independent `min_height`. Partition the children greedily
            // (mirroring `flexbox::layout_row`'s wrap pass) and sum each line's
            // tallest child plus the between-line cross-axis gap. Closes #258.
            NodeKind::Container(Direction::Row) if self.wrap_children => {
                self.wrapped_min_height(available_width)
            }
            _ => self.min_height(),
        }
    }

    /// Intrinsic height of a wrapping row at a given available width.
    ///
    /// Greedily partitions the children into lines by accumulated main-axis
    /// width (`flex_basis` else `min_width`, plus the within-line gap), then
    /// sums each line's tallest child plus the cross-axis (between-line) gap.
    /// A child wider than the inner width occupies its own line. The result
    /// is clamped against the container's own `constraints` / `margin`, and
    /// the cross-axis gap total is clamped at 0 so an overlap gap never wraps
    /// the unsigned height. Closes #258.
    fn wrapped_min_height(&mut self, available_width: u32) -> u32 {
        let inner_width = available_width
            .saturating_sub(self.margin.horizontal())
            .saturating_sub(self.frame_horizontal());

        // Snapshot per-child base widths / heights (immutable borrow ends
        // before we touch `self.constraints` below).
        let gap = self.gap;
        let cross_gap = self.cross_gap;
        let mut line_count: u32 = 0;
        let mut total_lines_height: u32 = 0;
        let mut cur_width: i64 = 0;
        let mut cur_line_height: u32 = 0;
        let mut cur_has_child = false;

        for child in &self.children {
            let base = child.flex_basis().unwrap_or_else(|| child.min_width());
            let child_height = child.min_height();
            if cur_has_child {
                // Would adding this child (plus the within-line gap) overflow?
                let prospective = cur_width + gap as i64 + base as i64;
                if prospective > inner_width as i64 {
                    // Flush the current line and start a new one with this child.
                    line_count += 1;
                    total_lines_height = total_lines_height.saturating_add(cur_line_height);
                    cur_width = base as i64;
                    cur_line_height = child_height;
                } else {
                    cur_width = prospective;
                    cur_line_height = cur_line_height.max(child_height);
                }
            } else {
                cur_width = base as i64;
                cur_line_height = child_height;
                cur_has_child = true;
            }
        }
        if cur_has_child {
            line_count += 1;
            total_lines_height = total_lines_height.saturating_add(cur_line_height);
        }

        // Between-line cross-axis gaps: `(line_count - 1) * cross_gap`,
        // clamped at 0 for overlap gaps.
        let gap_total = if line_count > 1 {
            ((line_count as i64 - 1) * cross_gap as i64).max(0) as u32
        } else {
            0
        };
        let content_height = total_lines_height.saturating_add(gap_total);
        let height = content_height + self.frame_vertical();
        let height = height.max(self.constraints.min_height().unwrap_or(0));
        height.saturating_add(self.margin.vertical())
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
            let ch_width = UnicodeWidthStr::width(g) as u32;
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

            if chunk_width + ch_width > max_width {
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
                chunk_width += ch_width;
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
    #[allow(clippy::too_many_arguments)]
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
        } else if *current_width + 1 + word_width <= max_width {
            current_line_words.push((word_start, word_end));
            *current_width += 1 + word_width;
        } else {
            flush_line(text, lines, current_line_words);
            current_line_words.push((word_start, word_end));
            *current_width = word_width;
        }
    }

    // Handle a completed word (skipping empty ranges). Words wider than
    // `max_width` are split into sub-chunks via `split_long_word`.
    #[allow(clippy::too_many_arguments)]
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
            word_width += UnicodeWidthStr::width(g) as u32;
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
            let ch_width = UnicodeWidthStr::width(g) as u32;

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
            line_width += ch_width;
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

/// Build the layout tree from a recorded command stream.
///
/// Takes `&mut Vec<Command>` and consumes the contents via `drain(..)` so the
/// caller retains ownership of the allocation; after this returns,
/// `commands.len() == 0` but `commands.capacity()` is preserved. Callers that
/// route through [`crate::FrameState::commands_buf`] reclaim that capacity at
/// frame end (issue #150) so the per-frame `Vec::new` allocation churn is
/// amortized to one allocation across the session.
pub(crate) fn build_tree(commands: &mut Vec<Command>) -> LayoutNode {
    let mut root = LayoutNode::container(Direction::Column, default_container_config());
    let mut overlays: Vec<OverlayLayer> = Vec::new();
    let mut iter = commands.drain(..);
    build_children(&mut root, &mut iter, &mut overlays, false, 0);
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
                style,
                grow,
                align,
                wrap,
                truncate,
                margin,
                constraints,
            } => {
                let mut node = LayoutNode::text(
                    content,
                    style,
                    grow,
                    align,
                    (cursor_offset, wrap, truncate),
                    margin,
                    constraints,
                );
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
                let mut node = LayoutNode::rich_text(segments, wrap, align, margin, constraints);
                node.focus_id = pending_focus_id.take();
                node.interaction_id = pending_interaction_id.take();
                parent.children.push(node);
            }
            Command::Link {
                text,
                url,
                style,
                margin,
                constraints,
            } => {
                let mut node = LayoutNode::text(
                    text,
                    style,
                    0,
                    Align::Start,
                    (None, false, false),
                    margin,
                    constraints,
                );
                node.link_url = Some(url);
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
                } = *args;
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
                        title,
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
                build_children(&mut node, commands, overlays, false, depth + 1);
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
                    group_name,
                } = *args;
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
                        title,
                        grow,
                    },
                );
                node.is_scrollable = true;
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
                build_children(&mut node, commands, overlays, false, depth + 1);
                parent.children.push(node);
            }
            Command::BeginOverlay { modal } => {
                let mut overlay_node =
                    LayoutNode::container(Direction::Column, default_container_config());
                overlay_node.interaction_id = pending_interaction_id.take();
                build_children(&mut overlay_node, commands, overlays, true, depth + 1);
                overlays.push(OverlayLayer {
                    node: overlay_node,
                    modal,
                });
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
