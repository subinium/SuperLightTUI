use super::*;
use std::sync::OnceLock;

static SEP_LINE: OnceLock<String> = OnceLock::new();

fn sep_line() -> &'static str {
    SEP_LINE.get_or_init(|| "─".repeat(200))
}

impl Context {
    /// Render a horizontal divider line.
    ///
    /// The line is drawn with the theme's border color and expands to fill the
    /// container width.
    pub fn separator(&mut self) -> &mut Self {
        // The cached `sep_line()` is much wider than any reasonable terminal,
        // so the cross-axis (column-direction) clip in `Buffer::set_string`
        // truncates the trailing chars. Keeping `grow = 0` means a column
        // layout doesn't stretch the separator vertically, and `truncate =
        // false` avoids the ellipsis fallback which would otherwise replace
        // the last cell with `…`.
        self.commands.push(Command::Text {
            content: sep_line().to_owned(),
            cursor_offset: None,
            style: Style::new().fg(self.theme.border).dim(),
            grow: 0,
            align: Align::Start,
            wrap: false,
            truncate: false,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.rollback.last_text_idx = Some(self.commands.len() - 1);
        self
    }

    /// Render a horizontal separator line with a custom color.
    pub fn separator_colored(&mut self, color: Color) -> &mut Self {
        self.commands.push(Command::Text {
            content: sep_line().to_owned(),
            cursor_offset: None,
            style: Style::new().fg(color),
            grow: 0,
            align: Align::Start,
            wrap: false,
            truncate: false,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.rollback.last_text_idx = Some(self.commands.len() - 1);
        self
    }

    /// Conditionally render content when the named screen is active.
    ///
    /// Each screen gets an isolated hook segment — `use_state` / `use_memo`
    /// calls inside one screen do not interfere with another screen's hooks,
    /// even when you switch between screens across frames.
    ///
    /// Focus state is saved and restored per screen automatically.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # let mut screens = slt::ScreenState::new("main");
    /// # slt::run(|ui| {
    /// ui.screen("main", &mut screens, |ui| {
    ///     ui.text("Main screen");
    /// });
    /// # });
    /// ```
    pub fn screen(&mut self, name: &str, screens: &mut ScreenState, f: impl FnOnce(&mut Context)) {
        // Look up (or create) this screen's reserved hook segment.
        //
        // Cache-hit path is the steady state — every frame after the first.
        // Avoid the unconditional `name.to_string()` `entry()` allocation by
        // checking first via `&str` lookup. Only the first frame for a
        // given screen pays the `to_string()` cost. Closes #134 (Option B).
        let (seg_start, seg_count) = if let Some(&v) = self.screen_hook_map.get(name) {
            v
        } else {
            let v = (self.hook_states.len(), 0);
            self.screen_hook_map.insert(name.to_string(), v);
            v
        };

        let is_active = screens.current() == name;

        if is_active {
            // Save outer focus, restore this screen's focus
            let outer_focus_index = self.focus_index;
            let (saved_focus_idx, _saved_focus_count) = screens.restore_focus(name);
            self.focus_index = saved_focus_idx;

            // Set hook cursor to this screen's segment start
            self.rollback.hook_cursor = seg_start;
            let focus_count_before = self.rollback.focus_count;

            // Execute the screen's closure
            f(self);

            // Record the hook count for this screen.
            //
            // The first-frame path above already inserted an owned `String`
            // key for this screen; subsequent frames reuse it. Locate that
            // existing slot via `&str` and overwrite the value in place,
            // avoiding a second `to_string()` allocation per active frame.
            let hooks_used = self.rollback.hook_cursor - seg_start;
            if let Some(slot) = self.screen_hook_map.get_mut(name) {
                *slot = (seg_start, hooks_used);
            } else {
                self.screen_hook_map
                    .insert(name.to_string(), (seg_start, hooks_used));
            }

            // Save this screen's focus state
            let screen_focus_count = self.rollback.focus_count - focus_count_before;
            screens.save_focus(name, self.focus_index, screen_focus_count);

            // Restore outer focus
            self.focus_index = outer_focus_index;
        } else {
            // Skip: advance hook cursor past the reserved segment
            if seg_count > 0 && seg_start >= self.rollback.hook_cursor {
                self.rollback.hook_cursor = seg_start + seg_count;
            }
        }
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
            self.commands.insert(
                start,
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
            return self;
        }

        let mut segments: Vec<(String, Style)> = Vec::new();
        for cmd in self.commands.drain(start..) {
            match cmd {
                Command::Text { content, style, .. } => {
                    segments.push((content, style));
                }
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
        self
    }

    /// Render content in a modal overlay with dimmed background.
    ///
    /// ```no_run
    /// # let mut show = true;
    /// # slt::run(|ui: &mut slt::Context| {
    /// if show {
    ///     ui.modal(|ui| {
    ///         ui.text("Are you sure?");
    ///         if ui.button("OK").clicked { show = false; }
    ///     });
    /// }
    /// # });
    /// ```
    pub fn modal(&mut self, f: impl FnOnce(&mut Context)) -> Response {
        let interaction_id = self.next_interaction_id();
        self.commands.push(Command::BeginOverlay { modal: true });
        self.rollback.overlay_depth += 1;
        self.rollback.modal_active = true;
        self.rollback.modal_focus_start = self.rollback.focus_count;
        f(self);
        self.rollback.modal_focus_count = self
            .rollback
            .focus_count
            .saturating_sub(self.rollback.modal_focus_start);
        self.rollback.overlay_depth = self.rollback.overlay_depth.saturating_sub(1);
        self.commands.push(Command::EndOverlay);
        self.rollback.last_text_idx = None;
        self.response_for(interaction_id)
    }

    /// Render floating content without dimming the background.
    pub fn overlay(&mut self, f: impl FnOnce(&mut Context)) -> Response {
        let interaction_id = self.next_interaction_id();
        self.commands.push(Command::BeginOverlay { modal: false });
        self.rollback.overlay_depth += 1;
        f(self);
        self.rollback.overlay_depth = self.rollback.overlay_depth.saturating_sub(1);
        self.commands.push(Command::EndOverlay);
        self.rollback.last_text_idx = None;
        self.response_for(interaction_id)
    }

    /// Render a hover tooltip for the previously rendered interactive widget.
    ///
    /// Call this right after a widget or container response:
    /// ```ignore
    /// if ui.button("Save").clicked { save(); }
    /// ui.tooltip("Save the current document to disk");
    /// ```
    pub fn tooltip(&mut self, text: impl Into<String>) {
        let tooltip_text = text.into();
        if tooltip_text.is_empty() {
            return;
        }
        let last_interaction_id = self.rollback.interaction_count.saturating_sub(1);
        let last_response = self.response_for(last_interaction_id);
        if !last_response.hovered || last_response.rect.width == 0 || last_response.rect.height == 0
        {
            return;
        }
        let lines = wrap_tooltip_text(&tooltip_text, 38);
        self.pending_tooltips.push(PendingTooltip {
            anchor_rect: last_response.rect,
            lines,
        });
    }

    pub(crate) fn emit_pending_tooltips(&mut self) {
        let tooltips = std::mem::take(&mut self.pending_tooltips);
        if tooltips.is_empty() {
            return;
        }
        let area_w = self.area_width;
        let area_h = self.area_height;
        let surface = self.theme.surface;
        let border_color = self.theme.border;
        let text_color = self.theme.surface_text;

        for tooltip in tooltips {
            let content_w = tooltip
                .lines
                .iter()
                .map(|l| UnicodeWidthStr::width(l.as_str()) as u32)
                .max()
                .unwrap_or(0);
            let box_w = content_w.saturating_add(4).min(area_w);
            let box_h = (tooltip.lines.len() as u32).saturating_add(4).min(area_h);

            let tooltip_x = tooltip.anchor_rect.x.min(area_w.saturating_sub(box_w));
            let below_y = tooltip.anchor_rect.bottom();
            let tooltip_y = if below_y.saturating_add(box_h) <= area_h {
                below_y
            } else {
                tooltip.anchor_rect.y.saturating_sub(box_h)
            };

            let lines = tooltip.lines;
            let _ = self.overlay(|ui| {
                let _ = ui.container().w(area_w).h(area_h).col(|ui| {
                    let _ = ui
                        .container()
                        .ml(tooltip_x)
                        .mt(tooltip_y)
                        .max_w(box_w)
                        .border(Border::Rounded)
                        .border_fg(border_color)
                        .bg(surface)
                        .p(1)
                        .col(|ui| {
                            for line in &lines {
                                ui.text(line.as_str()).fg(text_color);
                            }
                        });
                });
            });
        }
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
            scroll_offset: None,
        }
    }

    /// Create a scrollable container. Handles wheel scroll and drag-to-scroll automatically.
    ///
    /// Pass a [`ScrollState`] to persist scroll position across frames. The state
    /// is updated in-place with the current scroll offset and bounds.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::widgets::ScrollState;
    /// # slt::run(|ui: &mut slt::Context| {
    /// let mut scroll = ScrollState::new();
    /// ui.scrollable(&mut scroll).col(|ui| {
    ///     for i in 0..100 {
    ///         ui.text(format!("Line {i}"));
    ///     }
    /// });
    /// # });
    /// ```
    pub fn scrollable(&mut self, state: &mut ScrollState) -> ContainerBuilder<'_> {
        let index = self.rollback.scroll_count;
        self.rollback.scroll_count += 1;
        if let Some(&(ch, vh)) = self.prev_scroll_infos.get(index) {
            state.set_bounds(ch, vh);
            let max = ch.saturating_sub(vh) as usize;
            state.offset = state.offset.min(max);
        }

        let next_id = self.rollback.interaction_count;
        if let Some(rect) = self.prev_hit_map.get(next_id).copied() {
            let inner_rects: Vec<Rect> = self
                .prev_scroll_rects
                .iter()
                .enumerate()
                .filter(|&(j, sr)| {
                    j != index
                        && sr.width > 0
                        && sr.height > 0
                        && sr.x >= rect.x
                        && sr.right() <= rect.right()
                        && sr.y >= rect.y
                        && sr.bottom() <= rect.bottom()
                })
                .map(|(_, sr)| *sr)
                .collect();
            self.auto_scroll_nested(&rect, state, &inner_rects);
        }

        self.container().scroll_offset(state.offset as u32)
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
    /// # use slt::widgets::ScrollState;
    /// # slt::run(|ui: &mut slt::Context| {
    /// let mut scroll = ScrollState::new();
    /// ui.scroll_col(&mut scroll, |ui| {
    ///     for i in 0..100 {
    ///         ui.text(format!("Line {i}"));
    ///     }
    /// });
    /// # });
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
    /// Useful for horizontally-scrolling timelines, kanban boards, and
    /// similar wide layouts.
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
    /// # use slt::widgets::ScrollState;
    /// # slt::run(|ui: &mut slt::Context| {
    /// let mut scroll = ScrollState::new();
    /// ui.row(|ui| {
    ///     ui.scrollable(&mut scroll).grow(1).col(|ui| {
    ///         for i in 0..100 { ui.text(format!("Line {i}")); }
    ///     });
    ///     ui.scrollbar(&scroll);
    /// });
    /// # });
    /// ```
    pub fn scrollbar(&mut self, state: &ScrollState) {
        let vh = state.viewport_height();
        let ch = state.content_height();
        if vh == 0 || ch <= vh {
            return;
        }

        let track_height = vh;
        let thumb_height = ((vh as f64 * vh as f64 / ch as f64).ceil() as u32).max(1);
        let max_offset = ch.saturating_sub(vh);
        let thumb_pos = if max_offset == 0 {
            0
        } else {
            ((state.offset as f64 / max_offset as f64) * (track_height - thumb_height) as f64)
                .round() as u32
        };

        let theme = self.theme;
        const THUMB: &str = "█";
        const TRACK: &str = "│";

        let _ = self.container().w(1).h(track_height).col(|ui| {
            for i in 0..track_height {
                if i >= thumb_pos && i < thumb_pos + thumb_height {
                    ui.styled(THUMB, Style::new().fg(theme.primary));
                } else {
                    ui.styled(TRACK, Style::new().fg(theme.text_dim).dim());
                }
            }
        });
    }

    fn auto_scroll_nested(
        &mut self,
        rect: &Rect,
        state: &mut ScrollState,
        inner_scroll_rects: &[Rect],
    ) {
        let mut to_consume = Vec::new();
        for (i, mouse) in self.mouse_events_in_rect(*rect) {
            let in_inner = inner_scroll_rects.iter().any(|sr| {
                mouse.x >= sr.x && mouse.x < sr.right() && mouse.y >= sr.y && mouse.y < sr.bottom()
            });
            if in_inner {
                continue;
            }

            let delta = self.scroll_lines_per_event as usize;
            match mouse.kind {
                MouseKind::ScrollUp => {
                    state.scroll_up(delta);
                    to_consume.push(i);
                }
                MouseKind::ScrollDown => {
                    state.scroll_down(delta);
                    to_consume.push(i);
                }
                MouseKind::Drag(MouseButton::Left) => {}
                _ => {}
            }
        }
        self.consume_indices(to_consume);
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
                gap,
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
        if (self.rollback.modal_active || self.prev_modal_active)
            && self.rollback.overlay_depth == 0
        {
            return Response::none();
        }
        if let Some(rect) = self.prev_hit_map.get(interaction_id) {
            let clicked = self
                .click_pos
                .map(|(mx, my)| {
                    mx >= rect.x && mx < rect.right() && my >= rect.y && my < rect.bottom()
                })
                .unwrap_or(false);
            let hovered = self
                .mouse_pos
                .map(|(mx, my)| {
                    mx >= rect.x && mx < rect.right() && my >= rect.y && my < rect.bottom()
                })
                .unwrap_or(false);
            Response {
                clicked,
                hovered,
                changed: false,
                focused: false,
                rect: *rect,
            }
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
    pub fn is_group_hovered(&self, name: &str) -> bool {
        if self.mouse_pos.is_none() {
            return false;
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

    /// Render a form that groups input fields vertically.
    ///
    /// Wraps the fields in a column container and forwards the form state
    /// to the closure. Use [`Context::form_field`] inside the closure to
    /// render each field with label + input + error display.
    ///
    /// Submission is driven by [`Context::form_submit`]; validation is
    /// triggered explicitly via [`FormState::validate`].
    pub fn form(
        &mut self,
        state: &mut FormState,
        f: impl FnOnce(&mut Context, &mut FormState),
    ) -> &mut Self {
        let _ = self.col(|ui| {
            f(ui, state);
        });
        self
    }

    /// Render a single form field with label and input.
    ///
    /// Shows a validation error below the input when present.
    pub fn form_field(&mut self, field: &mut FormField) -> &mut Self {
        let _ = self.col(|ui| {
            ui.styled(field.label.as_str(), Style::new().bold().fg(ui.theme.text));
            let _ = ui.text_input(&mut field.input);
            if let Some(error) = field.error.as_deref() {
                ui.styled(error, Style::new().dim().fg(ui.theme.error));
            }
        });
        self
    }

    /// Render a primary-styled submit button.
    ///
    /// Distinguishes the submit affordance from incidental buttons in the
    /// same form by rendering in the theme's primary color (via
    /// [`ButtonVariant::Primary`]). Returns `true` in `.clicked` when the
    /// user clicks it, presses Enter while focused, or activates it with
    /// Space. Pair with [`FormState::validate`] to gate submission on
    /// all fields being valid.
    pub fn form_submit(&mut self, label: impl Into<String>) -> Response {
        self.button_with(label, ButtonVariant::Primary)
    }
}
