//! Layout family — rows, columns, lines, separators, groups, containers,
//! scrollable regions and scrollbars, bordered shortcuts, grids, split panes
//! and gutters.
//!
//! State types ([`crate::widgets::ScrollState`],
//! [`crate::widgets::SplitPaneState`], [`crate::widgets::GridColumn`], ...)
//! live in `src/widgets/layout.rs`.

use super::*;
use std::sync::LazyLock;

mod grid;
mod gutter;
mod split;

pub use gutter::GutterOpts;

static SEP_LINE: LazyLock<String> = LazyLock::new(|| "─".repeat(200));

fn sep_line() -> &'static str {
    &SEP_LINE
}

impl Context {
    /// Render a horizontal divider line.
    ///
    /// The line is drawn with the theme's border color and expands to fill the
    /// container width.
    ///
    /// Returns a [`Response`] so the divider's hit-test rect is available for
    /// hover detection. Prior to v0.21.0 this returned `&mut Self`, but the
    /// chained style mutators (`.bold()`, `.fg()`) were a no-op — the cached
    /// separator string is already finalized — so the chain was dropped.
    /// Statement-form callers (`ui.separator();`) compile unchanged.
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.separator();
    /// # });
    /// ```
    pub fn separator(&mut self) -> Response {
        let response = self.interaction();
        // The cached `sep_line()` is much wider than any reasonable terminal,
        // so the cross-axis (column-direction) clip in `Buffer::set_string`
        // truncates the trailing chars. Keeping `grow = 0` means a column
        // layout doesn't stretch the separator vertically, and `truncate =
        // false` avoids the ellipsis fallback which would otherwise replace
        // the last cell with `…`.
        self.commands.push(Command::Text {
            content: sep_line().to_owned(),
            cursor_offset: None,
            cursor_masked: false,
            style: Style::new().fg(self.theme.border).dim(),
            grow: 0,
            align: Align::Start,
            wrap: false,
            truncate: false,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.rollback.last_text_idx = Some(self.commands.len() - 1);
        response
    }

    /// Render a horizontal separator line with a custom color.
    ///
    /// Returns a [`Response`] for hover detection; see [`Context::separator`]
    /// for the v0.21.0 return-shape change. Statement-form callers compile
    /// unchanged.
    ///
    /// ```no_run
    /// # use slt::Color;
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.separator_colored(Color::Cyan);
    /// # });
    /// ```
    pub fn separator_colored(&mut self, color: Color) -> Response {
        let response = self.interaction();
        self.commands.push(Command::Text {
            content: sep_line().to_owned(),
            cursor_offset: None,
            cursor_masked: false,
            style: Style::new().fg(color),
            grow: 0,
            align: Align::Start,
            wrap: false,
            truncate: false,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.rollback.last_text_idx = Some(self.commands.len() - 1);
        response
    }

    /// Create a vertical (column) container.
    ///
    /// Children are stacked top-to-bottom. Returns a [`Response`] with
    /// click/hover state for the container area.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.col(|ui| {
    ///     ui.text("line one");
    ///     ui.text("line two");
    /// });
    /// # });
    /// ```
    pub fn col(&mut self, f: impl FnOnce(&mut Context)) -> Response {
        self.push_container(Direction::Column, 0, f)
    }

    /// Create a vertical (column) container with a gap between children.
    ///
    /// `gap` is the number of blank rows inserted between each child.
    ///
    /// **Deprecated since 0.20.1**: the name collides with
    /// [`ContainerBuilder::col_gap`], which sets the *row-finalize* main-axis
    /// gap (Tailwind `gap-x` axis convention) and so means the opposite thing.
    /// Use `ui.container().gap(n).col(f)` instead — same output, no collision.
    #[deprecated(
        since = "0.20.1",
        note = "Use `ui.container().gap(n).col(f)` instead — same output, no name collision with `ContainerBuilder::col_gap`."
    )]
    pub fn col_gap(&mut self, gap: u32, f: impl FnOnce(&mut Context)) -> Response {
        self.push_container(Direction::Column, gap, f)
    }

    /// Create a horizontal (row) container.
    ///
    /// Children are placed left-to-right. Returns a [`Response`] with
    /// click/hover state for the container area.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.row(|ui| {
    ///     ui.text("left");
    ///     ui.spacer();
    ///     ui.text("right");
    /// });
    /// # });
    /// ```
    pub fn row(&mut self, f: impl FnOnce(&mut Context)) -> Response {
        self.push_container(Direction::Row, 0, f)
    }

    /// Create a horizontal (row) container with a gap between children.
    ///
    /// `gap` is the number of blank columns inserted between each child.
    ///
    /// **Deprecated since 0.20.1**: the name collides with
    /// [`ContainerBuilder::row_gap`], which sets the *column-finalize*
    /// main-axis gap (Tailwind `gap-y` axis convention) and so means the
    /// opposite thing. Use `ui.container().gap(n).row(f)` instead — same
    /// output, no collision.
    #[deprecated(
        since = "0.20.1",
        note = "Use `ui.container().gap(n).row(f)` instead — same output, no name collision with `ContainerBuilder::row_gap`."
    )]
    pub fn row_gap(&mut self, gap: u32, f: impl FnOnce(&mut Context)) -> Response {
        self.push_container(Direction::Row, gap, f)
    }

    /// Render inline text with mixed styles on a single line.
    ///
    /// Unlike [`row`](Context::row), `line()` is designed for rich text —
    /// children are rendered as continuous inline text without gaps.
    ///
    /// It intentionally returns `&mut Self` instead of [`Response`] so you can
    /// keep chaining display-oriented modifiers after composing the inline run.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::Color;
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.line(|ui| {
    ///     ui.text("Status: ");
    ///     ui.text("Online").bold().fg(Color::Green);
    /// });
    /// # });
    /// ```
    pub fn line(&mut self, f: impl FnOnce(&mut Context)) -> &mut Self {
        let _ = self.push_container(Direction::Row, 0, f);
        self
    }

    /// Render inline text with mixed styles, wrapping at word boundaries.
    ///
    /// Like [`line`](Context::line), but when the combined text exceeds
    /// the container width it wraps across multiple lines while
    /// preserving per-segment styles.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::{Color, Style};
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.line_wrap(|ui| {
    ///     ui.text("This is a long ");
    ///     ui.text("important").bold().fg(Color::Red);
    ///     ui.text(" message that wraps across lines");
    /// });
    /// # });
    /// ```
    pub fn line_wrap(&mut self, f: impl FnOnce(&mut Context)) -> &mut Self {
        let start = self.commands.len();
        f(self);
        let has_link = self.commands[start..]
            .iter()
            .any(|cmd| matches!(cmd, Command::Link { .. }));

        if has_link {
            for command in &mut self.commands[start..] {
                match command {
                    Command::Text { wrap, .. }
                    | Command::Link { wrap, .. }
                    | Command::RichText { wrap, .. } => *wrap = true,
                    _ => {}
                }
            }
            self.commands.insert(start, Command::WrapMarker(0));
            self.commands.insert(
                start + 1,
                Command::BeginContainer(Box::new(BeginContainerArgs {
                    direction: Direction::Row,
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
                    group_name: None,
                })),
            );
            self.commands.push(Command::EndContainer);
            self.rollback.last_text_idx = None;
            self.invalidate_geometry_cache_from(start);
            return self;
        }

        let mut segments: Vec<(String, Style)> = Vec::new();
        for cmd in self.commands.drain(start..) {
            match cmd {
                Command::Text { content, style, .. } => {
                    segments.push((content, style));
                }
                Command::RichText { segments: rich, .. } => segments.extend(rich),
                Command::Link { text, style, .. } => {
                    // Preserve link text with underline styling (URL lost in RichText,
                    // but text is visible and wraps correctly)
                    segments.push((text, style));
                }
                _ => {}
            }
        }
        self.commands.push(Command::RichText {
            segments,
            wrap: true,
            align: Align::Start,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.rollback.last_text_idx = None;
        self.invalidate_geometry_cache_from(start);
        self
    }

    /// Create a named group container for shared hover/focus styling.
    ///
    /// ```ignore
    /// ui.group("card").border(Border::Rounded)
    ///     .group_hover_bg(Color::Indexed(238))
    ///     .col(|ui| { ui.text("Hover anywhere"); });
    /// ```
    pub fn group(&mut self, name: &str) -> ContainerBuilder<'_> {
        // Materialize the name once; subsequent uses are cheap `Arc::clone`
        // pointer bumps. Closes #145 (double `to_string` allocation) and
        // completes the `Arc<str>` migration tracked by #139.
        self.rollback.group_count = self.rollback.group_count.saturating_add(1);
        let name_arc: std::sync::Arc<str> = std::sync::Arc::from(name);
        self.rollback
            .group_stack
            .push(std::sync::Arc::clone(&name_arc));
        self.container().group_name_arc(name_arc)
    }

    /// Create a container with a fluent builder.
    ///
    /// Use this for borders, padding, grow, constraints, and titles. Chain
    /// configuration methods on the returned [`ContainerBuilder`], then call
    /// `.col()` or `.row()` to finalize.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// use slt::Border;
    /// ui.container()
    ///     .border(Border::Rounded)
    ///     .p(1)
    ///     .title("My Panel")
    ///     .col(|ui| {
    ///         ui.text("content");
    ///     });
    /// # });
    /// ```
    pub fn container(&mut self) -> ContainerBuilder<'_> {
        let border = self.theme.border;
        ContainerBuilder {
            ctx: self,
            gap: 0,
            row_gap: None,
            col_gap: None,
            align: Align::Start,
            align_self_value: None,
            justify: Justify::Start,
            border: None,
            border_sides: BorderSides::all(),
            border_style: Style::new().fg(border),
            bg: None,
            text_color: None,
            dark_bg: None,
            dark_border_style: None,
            group_hover_bg: None,
            group_hover_border_style: None,
            group_name: None,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
            shrink_flag: false,
            wrap_flag: false,
            basis: None,
            scroll_offset: None,
            scroll_offset_x: None,
            scroll_follow_focus: None,
            scroll_state_id: 0,
            theme_override: None,
            pointer_flags: 0,
            pointer_shape: None,
            state_styles: Default::default(),
        }
    }

    pub(crate) fn invalidate_geometry_cache(&mut self) {
        self.geometry_cursor = 0;
        self.geometry_stack.clear();
        self.geometry_pending_interaction = None;
    }

    pub(crate) fn invalidate_geometry_cache_from(&mut self, changed_start: usize) {
        // An unscanned suffix can be rewritten without discarding the prefix.
        if self.geometry_cursor > changed_start {
            self.invalidate_geometry_cache();
        }
    }

    fn sync_geometry_cache(&mut self) -> usize {
        if self.geometry_cursor > self.commands.len() {
            self.invalidate_geometry_cache();
        }
        let scanned = self.commands.len() - self.geometry_cursor;
        for index in self.geometry_cursor..self.commands.len() {
            match &self.commands[index] {
                Command::InteractionMarker(id) => self.geometry_pending_interaction = Some(*id),
                Command::BeginContainer(_)
                | Command::BeginScrollable(_)
                | Command::BeginOverlay { .. } => self
                    .geometry_stack
                    .push((index, self.geometry_pending_interaction.take())),
                Command::EndContainer | Command::EndOverlay => {
                    self.geometry_stack.pop();
                }
                Command::Text { .. }
                | Command::RichText { .. }
                | Command::Link { .. }
                | Command::RawDraw { .. }
                | Command::Spacer { .. } => self.geometry_pending_interaction = None,
                _ => {}
            }
        }
        self.geometry_cursor = self.commands.len();
        scanned
    }

    /// Available parent content capacity, not the last widget's intrinsic size.
    /// Flex allocations use previous-frame logical geometry; explicit
    /// constraints are resolved immediately, including the first frame.
    /// Appended commands are scanned once; each lookup visits only active parents.
    pub(crate) fn available_content_size(&mut self) -> (u32, u32) {
        use crate::style::{HeightSpec, WidthSpec};

        self.sync_geometry_cache();
        let mut width = self.area_width;
        let mut height = self.area_height;
        let mut parent_direction = Direction::Column;
        let mut content_origin = (0, 0);
        for &(index, id) in &self.geometry_stack {
            let (direction, constraints, padding, margin, border, sides, grow) =
                match &self.commands[index] {
                    Command::BeginContainer(args) => (
                        args.direction,
                        args.constraints,
                        args.padding,
                        args.margin,
                        args.border,
                        args.border_sides,
                        args.grow,
                    ),
                    Command::BeginScrollable(args) => (
                        args.direction,
                        args.constraints,
                        args.padding,
                        args.margin,
                        args.border,
                        args.border_sides,
                        args.grow,
                    ),
                    Command::BeginOverlay { .. } => (
                        Direction::Column,
                        Constraints::default(),
                        Padding::default(),
                        Margin::default(),
                        None,
                        BorderSides::all(),
                        0,
                    ),
                    _ => unreachable!("geometry stack contains only open containers"),
                };
            let previous = id.and_then(|id| self.prev_allocated_areas.get(id)).copied();
            let allocated_width = if parent_direction == Direction::Row && grow > 0 {
                previous
                    .filter(|rect| rect.width > 0)
                    .map_or(width.saturating_sub(margin.horizontal()), |rect| rect.width)
            } else {
                width.saturating_sub(margin.horizontal())
            };
            let allocated_height = if parent_direction == Direction::Column && grow > 0 {
                previous
                    .filter(|rect| rect.height > 0)
                    .map_or(height.saturating_sub(margin.vertical()), |rect| rect.height)
            } else {
                height.saturating_sub(margin.vertical())
            };
            width = match constraints.width {
                WidthSpec::Fixed(value) => value,
                WidthSpec::Pct(pct) => (u64::from(width) * u64::from(pct.min(100)) / 100) as u32,
                WidthSpec::Ratio(num, den) if den > 0 => {
                    (u64::from(width) * u64::from(num) / u64::from(den)).min(u64::from(u32::MAX))
                        as u32
                }
                WidthSpec::MinMax { min, max } => allocated_width.max(min).min(max),
                _ => allocated_width,
            };
            height = match constraints.height {
                HeightSpec::Fixed(value) => value,
                HeightSpec::Pct(pct) => (u64::from(height) * u64::from(pct.min(100)) / 100) as u32,
                HeightSpec::Ratio(num, den) if den > 0 => {
                    (u64::from(height) * u64::from(num) / u64::from(den)).min(u64::from(u32::MAX))
                        as u32
                }
                HeightSpec::MinMax { min, max } => allocated_height.max(min).min(max),
                _ => allocated_height,
            };
            let left = padding
                .left
                .saturating_add(u32::from(border.is_some() && sides.left));
            let top = padding
                .top
                .saturating_add(u32::from(border.is_some() && sides.top));
            width = width.saturating_sub(
                left.saturating_add(padding.right)
                    .saturating_add(u32::from(border.is_some() && sides.right)),
            );
            height = height.saturating_sub(
                top.saturating_add(padding.bottom)
                    .saturating_add(u32::from(border.is_some() && sides.bottom)),
            );
            content_origin = previous.map_or(content_origin, |rect| {
                (rect.x.saturating_add(left), rect.y.saturating_add(top))
            });
            parent_direction = direction;
        }
        // Previous positions account for preceding siblings, but their old
        // content-dependent width/height never becomes the next capacity.
        if let Some(rect) = self
            .geometry_pending_interaction
            .and_then(|id| self.prev_allocated_areas.get(id))
        {
            if parent_direction == Direction::Column {
                height = height.saturating_sub(rect.y.saturating_sub(content_origin.1));
            } else {
                width = width.saturating_sub(rect.x.saturating_sub(content_origin.0));
            }
        }
        (width, height)
    }

    /// Create a scrollable container. Handles wheel scroll and drag-to-scroll automatically.
    ///
    /// Pass a [`ScrollState`] to persist scroll position across frames. The state
    /// is updated in-place with the current scroll offset and bounds.
    /// Wheel events go to the innermost region under the pointer that can
    /// still move on the requested axis. When a nested scrollable reaches its
    /// end, further wheel motion chains to the enclosing scrollable (the
    /// browser default). Call [`ContainerBuilder::overscroll_contain`] on a
    /// region that should keep the wheel at its limit instead, such as a popup
    /// list.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::widgets::ScrollState;
    ///
    /// // Create state once, outside the frame closure, so the offset persists.
    /// let mut scroll = ScrollState::new();
    /// slt::run(|ui: &mut slt::Context| {
    ///     ui.scrollable(&mut scroll).col(|ui| {
    ///         for i in 0..100 {
    ///             ui.text(format!("Line {i}"));
    ///         }
    ///     });
    /// })?;
    /// # Ok::<_, std::io::Error>(())
    /// ```
    pub fn scrollable(&mut self, state: &mut ScrollState) -> ContainerBuilder<'_> {
        self.rollback.scroll_count += 1;
        // State identity survives insertion/reordering of unrelated widgets,
        // overlays, and raw offset containers between completed frames.
        let id = state.id();
        let index = self.prev_geometry.scroll_by_id.get(&id).copied();
        if let Some(adjustment) = self.pending_scroll.get(&id).copied() {
            let offset = if adjustment.horizontal {
                &mut state.offset_x
            } else {
                &mut state.offset
            };
            if u32::try_from(*offset).unwrap_or(u32::MAX) == adjustment.from {
                *offset = adjustment.to as usize;
            }
        }
        // #247: the previous frame recorded the scroll axis (`is_horizontal`)
        // because this binding runs before `.row()` / `.col()` is known. Bind
        // the matching axis so a horizontal scrollable updates `offset_x` while
        // a vertical one keeps the byte-identical `offset` path.
        let mut is_horizontal = false;
        if let Some(&(content, viewport, horizontal)) =
            index.and_then(|index| self.prev_scroll_infos.get(index))
        {
            is_horizontal = horizontal;
            let max = index
                .and_then(|index| self.prev_geometry.scrolls.get(index))
                .map_or(content.saturating_sub(viewport), |scroll| {
                    scroll.max_offset()
                });
            state.set_clipped_bounds(content, viewport, max, horizontal);
            if horizontal {
                state.offset_x = state.offset_x.min(max as usize);
            } else {
                state.offset = state.offset.min(max as usize);
            }
        }

        if let Some(index) = index {
            self.auto_scroll_nested(index, state, is_horizontal);
        }
        // #445: a text-selection drag held on this region's top or bottom row
        // scrolls it one step per frame so the selection can extend.
        if !is_horizontal
            && let Some((region, direction)) = self.pointer.selection_autoscroll
            && region == id
        {
            let step = self.scroll_lines_per_event as usize;
            if direction < 0 {
                state.scroll_up(step);
            } else {
                state.scroll_down(step);
            }
        }
        // #443: hand this binding's anchoring policy to the layout pass.
        self.pointer
            .scroll_policies
            .push(crate::layout::ScrollPolicy {
                state_id: id,
                anchor: state.anchor,
                stick_to_bottom: state.stick_to_bottom,
                dragging: state.dragging,
            });

        // Carry both axis offsets; the tree builder applies the one matching
        // the finalizing `.row()` / `.col()` direction (#247).
        let mut builder = self
            .container()
            .scroll_offset(u32::try_from(state.offset).unwrap_or(u32::MAX));
        builder.scroll_offset_x = Some(u32::try_from(state.offset_x).unwrap_or(u32::MAX));
        builder.scroll_follow_focus = Some(state.follow_focus);
        builder.scroll_state_id = id;
        builder
    }

    /// Scrollable column container — shortcut for
    /// `scrollable(state).grow(1).col(f)`.
    ///
    /// This is the form used by nearly every scrollable view: a vertical
    /// list that fills its parent and wheels through its own content. Use
    /// the explicit [`Context::scrollable`] builder when you need custom
    /// `grow`, borders, padding, or a scrollbar alongside.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::widgets::ScrollState;
    ///
    /// // Create state once, outside the frame closure, so the offset persists.
    /// let mut scroll = ScrollState::new();
    /// slt::run(|ui: &mut slt::Context| {
    ///     ui.scroll_col(&mut scroll, |ui| {
    ///         for i in 0..100 {
    ///             ui.text(format!("Line {i}"));
    ///         }
    ///     });
    /// })?;
    /// # Ok::<_, std::io::Error>(())
    /// ```
    pub fn scroll_col(
        &mut self,
        state: &mut ScrollState,
        f: impl FnOnce(&mut Context),
    ) -> Response {
        self.scrollable(state).grow(1).col(f)
    }

    /// Scrollable row container — shortcut for
    /// `scrollable(state).grow(1).row(f)`.
    ///
    /// Lays children out left-to-right and scrolls **horizontally** when their
    /// combined width exceeds the viewport: useful for timelines, kanban
    /// boards, wide tables, Gantt strips, and long single-line log entries
    /// (#247). The horizontal axis is driven by
    /// [`ScrollState::scroll_left`] / [`ScrollState::scroll_right`], native
    /// horizontal mouse wheel, and shift+wheel. Nest a `scroll_row` inside a
    /// [`scroll_col`](Self::scroll_col) to scroll both axes.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::widgets::ScrollState;
    ///
    /// // Create state once, outside the frame closure, so the offset persists.
    /// let mut scroll = ScrollState::new();
    /// slt::run(|ui: &mut slt::Context| {
    ///     ui.scroll_row(&mut scroll, |ui| {
    ///         for i in 0..40 {
    ///             ui.text(format!("col-{i:02}  "));
    ///         }
    ///     });
    /// })?;
    /// # Ok::<_, std::io::Error>(())
    /// ```
    pub fn scroll_row(
        &mut self,
        state: &mut ScrollState,
        f: impl FnOnce(&mut Context),
    ) -> Response {
        self.scrollable(state).grow(1).row(f)
    }

    /// Render a scrollbar track for a [`ScrollState`].
    ///
    /// Displays a track (`│`) with a proportional thumb (`█`). The thumb size
    /// and position are calculated from the scroll state's content height,
    /// viewport height, and current offset.
    ///
    /// Typically placed beside a `scrollable()` container in a `row()`:
    /// ```no_run
    /// use slt::widgets::ScrollState;
    ///
    /// // Create state once, outside the frame closure, so the offset persists.
    /// let mut scroll = ScrollState::new();
    /// slt::run(|ui: &mut slt::Context| {
    ///     ui.row(|ui| {
    ///         ui.scrollable(&mut scroll).grow(1).col(|ui| {
    ///             for i in 0..100 { ui.text(format!("Line {i}")); }
    ///         });
    ///         ui.scrollbar(&mut scroll);
    ///     });
    /// })?;
    /// # Ok::<_, std::io::Error>(())
    /// ```
    ///
    /// # Interaction (since 0.21.0)
    ///
    /// The bar is a real input surface, mirroring `split_pane`'s drag handle:
    ///
    /// - **Click-to-jump on the track:** a left mouse-down inside the track but
    ///   outside the thumb jumps `state.offset` so the clicked row maps
    ///   proportionally to the content (top cell → offset 0, bottom cell →
    ///   `max_offset`).
    /// - **Drag-to-scroll on the thumb:** a left mouse-down on the thumb sets
    ///   [`ScrollState::dragging`]; subsequent drag events scroll proportionally
    ///   to the cursor's y within the track (even when the cursor leaves the
    ///   track on the x-axis); mouse-up clears `dragging`.
    ///
    /// Only the mouse events the bar acts on are consumed, so wheel scrolling
    /// over a sibling [`scrollable`](Self::scrollable) keeps working unchanged.
    /// Like every mouse handler the bar is inert while a modal is active and
    /// the bar is not inside it.
    ///
    /// # Returns
    ///
    /// A [`Response`] whose hit-test rect covers the scrollbar track — it is
    /// the track container's own interaction response, so `.clicked`,
    /// `.hovered`, and `.rect` are populated for the track region. `.changed`
    /// is `true` on a frame where a scrollbar interaction moved the offset.
    /// When the content fits the viewport nothing is rendered and
    /// [`Response::none()`] is returned. Prior to v0.21.0 the receiver was
    /// `&ScrollState`; pass `&mut scroll` instead.
    pub fn scrollbar(&mut self, state: &mut ScrollState) -> Response {
        let vh = state.viewport_height();
        let ch = state.content_height();
        if vh == 0 || ch <= vh {
            // No overflow: render nothing, consume nothing, leave drag state
            // untouched. Matches the pre-interaction behavior exactly.
            return Response::none();
        }

        let track_height = vh;
        let thumb_height = ((vh as f64 * vh as f64 / ch as f64).ceil() as u32).max(1);
        let max_offset = state.max_offset() as u32;

        // The upcoming `self.container()…col()` allocates the next interaction
        // slot, so its id is the current `interaction_count`. We hit-test
        // against THAT slot's rect from the previous frame, exactly as
        // `scrollable()` and `consume_split_pane_drag` do.
        let track_id = self.rollback.interaction_count;
        let thumb_pos =
            Self::scrollbar_thumb_pos(state.offset, max_offset, track_height, thumb_height);
        let changed = if let Some(rect) = self.prev_hit_map.get(track_id).copied() {
            self.handle_scrollbar_drag(rect, track_id, state, thumb_pos, thumb_height, max_offset)
        } else {
            false
        };

        // Recompute the thumb position AFTER handling so the same frame's draw
        // reflects an offset moved by a click/drag this frame.
        let thumb_pos =
            Self::scrollbar_thumb_pos(state.offset, max_offset, track_height, thumb_height);

        let theme = self.theme;
        const THUMB: &str = "█";
        const TRACK: &str = "│";

        // The track container carries its own interaction slot (every
        // `col`/`row` reserves one), so its `Response` is the hit-test rect
        // for click-to-jump — no separate `interaction()` call is needed.
        let mut response = self.container().w(1).h(track_height).col(|ui| {
            for i in 0..track_height {
                if i >= thumb_pos && i < thumb_pos + thumb_height {
                    ui.styled(THUMB, Style::new().fg(theme.primary));
                } else {
                    ui.styled(TRACK, Style::new().fg(theme.text_dim).dim());
                }
            }
        });
        response.changed = changed;
        response
    }

    /// Map a scroll `offset` to the thumb's top row within the track.
    ///
    /// Pure helper shared by the render path and the interaction path so both
    /// agree on where the thumb sits.
    fn scrollbar_thumb_pos(
        offset: usize,
        max_offset: u32,
        track_height: u32,
        thumb_height: u32,
    ) -> u32 {
        if max_offset == 0 {
            0
        } else {
            let travel = track_height.saturating_sub(thumb_height);
            ((offset as f64 / max_offset as f64) * travel as f64).round() as u32
        }
    }

    /// Map a cursor row `y` (absolute) to a clamped scroll offset for the
    /// track rect at `track_y` with height `track_h`.
    ///
    /// The thumb is centered on the cursor: the cursor row relative to the
    /// track maps to the thumb top (minus half the thumb), which then maps
    /// linearly onto `[0, max_offset]`. The result is always in
    /// `[0, max_offset]` and monotonically non-decreasing in `y`. Extracted
    /// as an associated function so it is `proptest`-able without driving a
    /// full frame.
    pub(crate) fn scrollbar_offset_for_y(
        y: u32,
        track_y: u32,
        track_h: u32,
        thumb_height: u32,
        max_offset: u32,
    ) -> usize {
        let travel = track_h.saturating_sub(thumb_height);
        if travel == 0 {
            return 0;
        }
        let rel = y.saturating_sub(track_y).min(track_h.saturating_sub(1));
        let thumb_top = rel.saturating_sub(thumb_height / 2).min(travel);
        ((thumb_top as f64 / travel as f64) * max_offset as f64).round() as usize
    }

    /// Hit-test the previous-frame track `rect` against this frame's mouse
    /// events and apply click-to-jump / thumb-drag to `state`.
    ///
    /// Returns `true` if the offset moved. Mirrors `consume_split_pane_drag`:
    /// snapshots the unconsumed mouse events, mutates `state`, then consumes
    /// only the events it acted on so wheel scroll on a sibling container is
    /// never double-counted.
    fn handle_scrollbar_drag(
        &mut self,
        rect: Rect,
        track_id: usize,
        state: &mut ScrollState,
        thumb_pos: u32,
        thumb_height: u32,
        max_offset: u32,
    ) -> bool {
        // Modal suppression: while a modal is active and the bar is not inside
        // an overlay, the bar is inert — consistent with `mouse_down`'s guard.
        if !self.interaction_allowed() {
            state.dragging = false;
            return false;
        }
        if rect.width == 0 || rect.height == 0 {
            return false;
        }

        // Snapshot so `consume_indices` (mutable borrow) can run after the loop.
        // `MouseKind` is not `Copy`, so clone it (mirrors `consume_split_pane_drag`).
        // #442: focus loss and resize end a thumb drag (its release will
        // never arrive); they are replayed in order as `None` kinds.
        let events: Vec<(usize, Option<MouseKind>, u32, u32)> = self
            .events
            .iter()
            .enumerate()
            .filter_map(|(i, e)| match e {
                Event::Mouse(m) if !self.consumed[i] => Some((i, Some(m.kind.clone()), m.x, m.y)),
                Event::FocusLost | Event::Resize(_, _) => Some((i, None, 0, 0)),
                _ => None,
            })
            .collect();

        let track_y = rect.y;
        let track_h = rect.height;
        let thumb_top = track_y + thumb_pos;
        let thumb_bottom = thumb_top + thumb_height;

        let mut consumed: Vec<usize> = Vec::new();
        let mut changed = false;
        for (i, kind, mx, my) in events {
            let Some(kind) = kind else {
                state.dragging = false;
                continue;
            };
            let in_track = mx >= rect.x && mx < rect.right() && my >= track_y && my < rect.bottom();
            match kind {
                MouseKind::Down(MouseButton::Left)
                    if in_track && self.press_reaches_slot(i, track_id) =>
                {
                    let on_thumb = my >= thumb_top && my < thumb_bottom;
                    if on_thumb {
                        // Grab the thumb; offset only moves on subsequent drags.
                        state.dragging = true;
                    } else {
                        // Click-to-jump on the track.
                        let before = state.offset;
                        state.set_offset(Self::scrollbar_offset_for_y(
                            my,
                            track_y,
                            track_h,
                            thumb_height,
                            max_offset,
                        ));
                        changed |= state.offset != before;
                    }
                    consumed.push(i);
                }
                MouseKind::Drag(MouseButton::Left) if state.dragging => {
                    // Drag tracks the cursor's y even outside the track on x.
                    let before = state.offset;
                    state.set_offset(Self::scrollbar_offset_for_y(
                        my,
                        track_y,
                        track_h,
                        thumb_height,
                        max_offset,
                    ));
                    changed |= state.offset != before;
                    consumed.push(i);
                }
                MouseKind::Up(MouseButton::Left) if state.dragging => {
                    state.dragging = false;
                    consumed.push(i);
                }
                // A press outside the track, or motion without a held button,
                // means the drag's release was lost.
                MouseKind::Down(MouseButton::Left) | MouseKind::Moved if state.dragging => {
                    state.dragging = false;
                }
                _ => {}
            }
        }
        self.consume_indices(consumed);
        changed
    }

    /// Assign every unconsumed wheel event of this frame to one scroll region
    /// (#440): the innermost region under the pointer, on the event's axis,
    /// that can still move in the event's direction.
    ///
    /// `scrollable()` binds before its children run, so the chain is resolved
    /// up front from the previous frame's rendered offsets and limits. A
    /// simulated offset per region advances with each event, so a burst fills
    /// the inner region to its end and spills the rest to the enclosing one,
    /// like the browser default `overscroll-behavior: auto`. A region marked
    /// with [`ContainerBuilder::overscroll_contain`] stops the chain even at
    /// its limit. An event no region can absorb stays unconsumed.
    fn resolve_wheel_chain(&self) -> Vec<Option<usize>> {
        let delta = self.scroll_lines_per_event.max(1);
        let contained = |region: usize| {
            self.prev_geometry
                .scrolls
                .get(region)
                .is_some_and(|scroll| self.overscroll_contained(scroll.state_id))
        };
        // Simulated `(offset, max_offset)` per region; `None` when the
        // previous frame recorded no geometry for it (it then always absorbs,
        // the pre-chaining behavior).
        let mut simulated: Vec<Option<(u32, u32)>> = (0..self.prev_scroll_rects.len())
            .map(|region| {
                self.prev_geometry.scrolls.get(region).map(|scroll| {
                    let max = scroll.max_offset();
                    (scroll.offset.min(max), max)
                })
            })
            .collect();
        self.events
            .iter()
            .enumerate()
            .map(|(i, event)| {
                let Event::Mouse(mouse) = event else {
                    return None;
                };
                if self.consumed[i] {
                    return None;
                }
                let (horizontal, backwards) = match mouse.kind {
                    MouseKind::ScrollLeft => (true, true),
                    MouseKind::ScrollRight => (true, false),
                    MouseKind::ScrollUp => (mouse.modifiers.contains(KeyModifiers::SHIFT), true),
                    MouseKind::ScrollDown => (mouse.modifiers.contains(KeyModifiers::SHIFT), false),
                    _ => return None,
                };
                // Innermost first: later-collected regions are nested inside
                // or painted above earlier ones.
                for (region, (rect, &(content, viewport, axis))) in self
                    .prev_scroll_rects
                    .iter()
                    .zip(&self.prev_scroll_infos)
                    .enumerate()
                    .rev()
                {
                    if axis != horizontal || content <= viewport || !rect.contains(mouse.x, mouse.y)
                    {
                        continue;
                    }
                    match simulated.get_mut(region).and_then(Option::as_mut) {
                        None => return Some(region),
                        Some((offset, max)) => {
                            let can_move = if backwards {
                                *offset > 0
                            } else {
                                *offset < *max
                            };
                            if can_move {
                                *offset = if backwards {
                                    offset.saturating_sub(delta)
                                } else {
                                    offset.saturating_add(delta).min(*max)
                                };
                                return Some(region);
                            }
                            if contained(region) {
                                // `overscroll-behavior: contain`: swallow the
                                // event at the limit instead of chaining.
                                return Some(region);
                            }
                        }
                    }
                }
                None
            })
            .collect()
    }

    fn auto_scroll_nested(&mut self, index: usize, state: &mut ScrollState, is_horizontal: bool) {
        if !self.interaction_allowed() || self.events.is_empty() {
            return;
        }
        if self.scroll_wheel_targets.is_none() {
            // One O(events * regions) pass, never a region discovery on idle frames.
            self.scroll_wheel_targets = Some(self.resolve_wheel_chain());
        }
        let targets = self
            .scroll_wheel_targets
            .as_ref()
            .expect("wheel targets resolved");
        let delta = self.scroll_lines_per_event as usize;
        for (i, target) in targets.iter().enumerate() {
            if *target != Some(index) || self.consumed[i] {
                continue;
            }
            let Event::Mouse(mouse) = &self.events[i] else {
                continue;
            };
            let backwards = matches!(mouse.kind, MouseKind::ScrollUp | MouseKind::ScrollLeft);
            match (is_horizontal, backwards) {
                (true, true) => state.scroll_left(delta),
                (true, false) => state.scroll_right(delta),
                (false, true) => state.scroll_up(delta),
                (false, false) => state.scroll_down(delta),
            }
            self.consumed[i] = true;
        }
    }

    /// Shortcut for `container().border(border)`.
    ///
    /// Returns a [`ContainerBuilder`] pre-configured with the given border style.
    pub fn bordered(&mut self, border: Border) -> ContainerBuilder<'_> {
        self.container()
            .border(border)
            .border_sides(BorderSides::all())
    }

    fn push_container(
        &mut self,
        direction: Direction,
        gap: u32,
        f: impl FnOnce(&mut Context),
    ) -> Response {
        let interaction_id = self.next_interaction_id();
        let border = self.theme.border;

        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                direction,
                // `BeginContainerArgs::gap` is signed since #222; this helper's
                // public `u32` callers (`row`/`col_gap`/…) never overlap.
                gap: gap as i32,
                align: Align::Start,
                align_self: None,
                justify: Justify::Start,
                border: None,
                border_sides: BorderSides::all(),
                border_style: Style::new().fg(border),
                bg_color: None,
                padding: Padding::default(),
                margin: Margin::default(),
                constraints: Constraints::default(),
                title: None,
                grow: 0,
                group_name: None,
            })));
        self.rollback.text_color_stack.push(None);
        f(self);
        self.rollback.text_color_stack.pop();
        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;

        self.response_for(interaction_id)
    }

    pub(crate) fn response_for(&self, interaction_id: usize) -> Response {
        if self.rollback.disabled_depth > 0 {
            return self.disabled_response(interaction_id);
        }
        if !self.interaction_allowed()
            || (self.rollback.new_focus_slot
                && self
                    .rollback
                    .focus_interaction_origin
                    .is_some_and(|(_, start)| interaction_id >= start))
        {
            return Response::none();
        }
        if let Some(rect) = self.prev_hit_map.get(interaction_id).copied() {
            // Issue #439: each press activates only its topmost, innermost
            // slot; see `Context::pointer_response`.
            self.pointer_response(interaction_id, rect)
        } else {
            Response::none()
        }
    }

    /// Returns true if the named group is currently hovered by the mouse.
    ///
    /// Uses the per-frame `hovered_groups` `HashSet` populated by
    /// `Context::build_hovered_groups()`; turns the previous O(n) scan over
    /// `prev_group_rects` into an O(1) lookup. Closes the cache half of
    /// #136 / #139.
    ///
    /// Inside a group instance named `name` this answers for that instance
    /// (#458), so rows that share a group name are told apart; elsewhere it
    /// reports whether any group with that name is hovered.
    pub fn is_group_hovered(&self, name: &str) -> bool {
        if self.mouse_pos.is_none() {
            return false;
        }
        let rollback = &self.rollback;
        if let (Some(top), Some(&hovered)) = (
            rollback.group_stack.last(),
            rollback.group_hover_stack.last(),
        ) && rollback.group_hover_stack.len() == rollback.group_stack.len()
            && &**top == name
        {
            return hovered;
        }
        // `HashSet<Arc<str>>::contains` accepts `&str` via `Borrow<str>`, so
        // there is no allocation on the hot path.
        self.hovered_groups.contains(name)
    }

    /// Returns true if the named group contains the currently focused widget.
    pub fn is_group_focused(&self, name: &str) -> bool {
        if self.prev_focus_count == 0 {
            return false;
        }
        let focused_index = self.focus_index % self.prev_focus_count;
        self.prev_focus_groups
            .get(focused_index)
            .and_then(|group| group.as_deref())
            .map(|group| group == name)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod line_wrap_tests;

#[cfg(test)]
mod v024_geometry_cache_tests {
    use super::*;

    fn assert_fresh_matches(ui: &mut Context) {
        let cached = ui.available_content_size();
        ui.invalidate_geometry_cache();
        assert_eq!(cached, ui.available_content_size());
    }

    #[test]
    fn geometry_cache_scans_only_appended_commands_and_pops_parents() {
        let mut backend = crate::TestBackend::new(80, 24);
        backend.render(|ui| {
            let _ = ui.container().w(40).h(12).p(1).col(|ui| {
                assert_eq!(ui.available_content_size(), (38, 10));
                let cursor = ui.geometry_cursor;
                assert_eq!(ui.sync_geometry_cache(), 0);
                let _ = ui.container().w(10).h(4).col(|ui| {
                    assert_eq!(ui.sync_geometry_cache(), ui.commands.len() - cursor);
                    assert_eq!(ui.available_content_size(), (10, 4));
                    assert_eq!(ui.sync_geometry_cache(), 0);
                });
                assert_eq!(ui.sync_geometry_cache(), 1);
                assert_eq!(ui.available_content_size(), (38, 10));
                assert_eq!(ui.geometry_stack.len(), 1);
            });
            assert_eq!(ui.available_content_size(), (80, 24));
            assert!(ui.geometry_stack.is_empty());
        });
    }

    #[test]
    fn geometry_cache_resets_per_frame_and_reuses_stack_capacity() {
        let mut backend = crate::TestBackend::new(80, 24);
        let mut capacity = 0;
        backend.render(|ui| {
            let _ = ui.container().w(40).col(|ui| {
                let _ = ui.available_content_size();
                capacity = ui.geometry_stack.capacity();
            });
        });
        backend.render(|ui| {
            assert_eq!(ui.geometry_cursor, 0);
            assert!(ui.geometry_stack.is_empty());
            assert!(ui.geometry_pending_interaction.is_none());
            assert!(ui.geometry_stack.capacity() >= capacity);
            assert_eq!(ui.available_content_size(), (80, 24));
        });
    }

    #[test]
    fn geometry_cache_keeps_prefix_when_wrap_only_rewrites_unscanned_suffix() {
        let mut backend = crate::TestBackend::new(80, 24);
        backend.render(|ui| {
            let _ = ui.container().w(40).h(12).col(|ui| {
                assert_eq!(ui.available_content_size(), (40, 12));
                let cursor = ui.geometry_cursor;
                for _ in 0..10 {
                    ui.line_wrap(|ui| {
                        ui.text("plain");
                    });
                }
                assert_eq!(ui.geometry_cursor, cursor);
                assert_eq!(ui.geometry_stack.len(), 1);
                assert_eq!(ui.available_content_size(), (40, 12));
            });
        });
    }

    #[test]
    fn geometry_cache_defensively_rebuilds_after_command_length_shrinks() {
        let mut backend = crate::TestBackend::new(80, 24);
        backend.render(|ui| {
            let _ = ui.container().w(40).col(|ui| {
                assert_eq!(ui.available_content_size().0, 40);
            });
            ui.commands.clear();
            assert_eq!(ui.available_content_size(), (80, 24));
            assert_eq!(ui.geometry_cursor, 0);
            assert!(ui.geometry_stack.is_empty());
        });
    }

    #[test]
    fn geometry_cache_invalidates_after_wrap_drain_and_insert() {
        for linked in [false, true] {
            let mut backend = crate::TestBackend::new(80, 24);
            backend.render(|ui| {
                let _ = ui.container().w(40).h(12).col(|ui| {
                    ui.line_wrap(|ui| {
                        if linked {
                            ui.link("link", "https://example.com");
                        }
                        let _ = ui.container().w(7).h(3).col(|ui| {
                            ui.text("inside");
                            assert_eq!(ui.available_content_size(), (7, 3));
                        });
                    });
                    for _ in 0..20 {
                        ui.text("after");
                    }
                    assert_eq!(ui.available_content_size(), (40, 12));
                    assert_fresh_matches(ui);
                });
            });
        }
    }

    #[test]
    fn geometry_cache_invalidates_after_grid_rebuilds() {
        for specified in [false, true] {
            let mut backend = crate::TestBackend::new(80, 24);
            backend.render(|ui| {
                let _ = ui.container().w(40).h(12).col(|ui| {
                    let cells = |ui: &mut Context| {
                        let _ = ui.container().w(7).h(3).col(|ui| {
                            assert_eq!(ui.available_content_size(), (7, 3));
                        });
                    };
                    if specified {
                        let _ = ui.grid_with(&[GridColumn::Auto, GridColumn::Auto], cells);
                    } else {
                        let _ = ui.grid(2, cells);
                    }
                    for _ in 0..20 {
                        ui.text("after");
                    }
                    assert_eq!(ui.available_content_size(), (40, 12));
                    assert_fresh_matches(ui);
                });
            });
        }
    }

    #[test]
    fn geometry_cache_invalidates_after_checkpoint_restore() {
        let mut backend = crate::TestBackend::new(80, 24);
        backend.render(|ui| {
            let _ = ui.container().w(40).h(12).col(|ui| {
                ui.error_boundary_with(
                    |ui| {
                        let _ = ui.container().w(7).h(3).col(|ui| {
                            assert_eq!(ui.available_content_size(), (7, 3));
                            panic!("geometry rollback probe");
                        });
                    },
                    |ui, _| {
                        ui.text("fallback");
                    },
                );
                for _ in 0..20 {
                    ui.text("after");
                }
                assert_eq!(ui.available_content_size(), (40, 12));
                assert_fresh_matches(ui);
            });
        });
    }
}

#[cfg(test)]
mod scrollbar_tests {
    use super::*;

    // ── #249: scrollbar() pixel ↔ offset mapping (pure helpers) ──────────

    #[test]
    fn offset_for_y_top_cell_maps_to_zero() {
        // Track at y=0..20, thumb 4 tall → travel 16, max_offset 80.
        let off = Context::scrollbar_offset_for_y(0, 0, 20, 4, 80);
        assert_eq!(off, 0);
    }

    #[test]
    fn offset_for_y_bottom_cell_maps_to_max() {
        // Clicking the last track cell jumps to the bottom of the content.
        let off = Context::scrollbar_offset_for_y(19, 0, 20, 4, 80);
        assert_eq!(off, 80);
    }

    #[test]
    fn offset_for_y_middle_is_near_half_max() {
        // Vertical midpoint → ~max_offset / 2 (within a few rows of slop).
        let off = Context::scrollbar_offset_for_y(10, 0, 20, 4, 80) as i64;
        assert!((off - 40).abs() <= 5, "midpoint offset {off} not near 40");
    }

    #[test]
    fn offset_for_y_respects_track_origin() {
        // Track offset by track_y=3; the top cell of that track yields 0.
        let off = Context::scrollbar_offset_for_y(3, 3, 20, 4, 80);
        assert_eq!(off, 0);
    }

    #[test]
    fn offset_for_y_zero_travel_is_zero() {
        // Thumb fills the whole track → nowhere to move → always 0.
        let off = Context::scrollbar_offset_for_y(7, 0, 5, 5, 0);
        assert_eq!(off, 0);
    }

    #[test]
    fn thumb_pos_endpoints() {
        // offset 0 → thumb at top; offset == max → thumb at travel.
        assert_eq!(Context::scrollbar_thumb_pos(0, 80, 20, 4), 0);
        assert_eq!(Context::scrollbar_thumb_pos(80, 80, 20, 4), 16);
    }

    proptest::proptest! {
        /// `scrollbar_offset_for_y` is always in `[0, max_offset]` and
        /// monotonically non-decreasing in the cursor row.
        #[test]
        fn offset_for_y_is_clamped_and_monotonic(
            content_height in 2u32..500,
            viewport_height in 1u32..200,
            y in 0u32..600,
        ) {
            // Derive the same track / thumb geometry the widget uses.
            proptest::prop_assume!(content_height > viewport_height);
            let track_h = viewport_height;
            let thumb_height = ((viewport_height as f64 * viewport_height as f64
                / content_height as f64)
                .ceil() as u32)
                .max(1);
            let max_offset = content_height.saturating_sub(viewport_height);

            let off = Context::scrollbar_offset_for_y(y, 0, track_h, thumb_height, max_offset);
            proptest::prop_assert!(off <= max_offset as usize);

            // Monotonic: a strictly lower cursor row never yields a smaller offset.
            let off_lower = Context::scrollbar_offset_for_y(
                y.saturating_add(1),
                0,
                track_h,
                thumb_height,
                max_offset,
            );
            proptest::prop_assert!(off_lower >= off);
        }
    }
}
