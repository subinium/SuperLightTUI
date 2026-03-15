use super::*;

impl Context {
    // ── text ──────────────────────────────────────────────────────────

    /// Render a text element. Returns `&mut Self` for style chaining.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// use slt::Color;
    /// ui.text("hello").bold().fg(Color::Cyan);
    /// # });
    /// ```
    pub fn text(&mut self, s: impl Into<String>) -> &mut Self {
        let content = s.into();
        self.commands.push(Command::Text {
            content,
            style: Style::new().fg(self.theme.text),
            grow: 0,
            align: Align::Start,
            wrap: false,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.last_text_idx = Some(self.commands.len() - 1);
        self
    }

    /// Render a clickable hyperlink.
    ///
    /// The link is interactive: clicking it (or pressing Enter/Space when
    /// focused) opens the URL in the system browser. OSC 8 is also emitted
    /// for terminals that support native hyperlinks.
    pub fn link(&mut self, text: impl Into<String>, url: impl Into<String>) -> &mut Self {
        let url_str = url.into();
        let focused = self.register_focusable();
        let interaction_id = self.interaction_count;
        self.interaction_count += 1;
        let response = self.response_for(interaction_id);

        let mut activated = response.clicked;
        if focused {
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                        activated = true;
                        self.consumed[i] = true;
                    }
                }
            }
        }

        if activated {
            let _ = open_url(&url_str);
        }

        let style = if focused {
            Style::new()
                .fg(self.theme.primary)
                .bg(self.theme.surface_hover)
                .underline()
                .bold()
        } else if response.hovered {
            Style::new()
                .fg(self.theme.accent)
                .bg(self.theme.surface_hover)
                .underline()
        } else {
            Style::new().fg(self.theme.primary).underline()
        };

        self.commands.push(Command::Link {
            text: text.into(),
            url: url_str,
            style,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.last_text_idx = Some(self.commands.len() - 1);
        self
    }

    /// Render a text element with word-boundary wrapping.
    ///
    /// Long lines are broken at word boundaries to fit the container width.
    /// Style chaining works the same as [`Context::text`].
    pub fn text_wrap(&mut self, s: impl Into<String>) -> &mut Self {
        let content = s.into();
        self.commands.push(Command::Text {
            content,
            style: Style::new().fg(self.theme.text),
            grow: 0,
            align: Align::Start,
            wrap: true,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.last_text_idx = Some(self.commands.len() - 1);
        self
    }

    // ── style chain (applies to last text) ───────────────────────────

    /// Apply bold to the last rendered text element.
    pub fn bold(&mut self) -> &mut Self {
        self.modify_last_style(|s| s.modifiers |= Modifiers::BOLD);
        self
    }

    /// Apply dim styling to the last rendered text element.
    ///
    /// Also sets the foreground color to the theme's `text_dim` color if no
    /// explicit foreground has been set.
    pub fn dim(&mut self) -> &mut Self {
        let text_dim = self.theme.text_dim;
        self.modify_last_style(|s| {
            s.modifiers |= Modifiers::DIM;
            if s.fg.is_none() {
                s.fg = Some(text_dim);
            }
        });
        self
    }

    /// Apply italic to the last rendered text element.
    pub fn italic(&mut self) -> &mut Self {
        self.modify_last_style(|s| s.modifiers |= Modifiers::ITALIC);
        self
    }

    /// Apply underline to the last rendered text element.
    pub fn underline(&mut self) -> &mut Self {
        self.modify_last_style(|s| s.modifiers |= Modifiers::UNDERLINE);
        self
    }

    /// Apply reverse-video to the last rendered text element.
    pub fn reversed(&mut self) -> &mut Self {
        self.modify_last_style(|s| s.modifiers |= Modifiers::REVERSED);
        self
    }

    /// Apply strikethrough to the last rendered text element.
    pub fn strikethrough(&mut self) -> &mut Self {
        self.modify_last_style(|s| s.modifiers |= Modifiers::STRIKETHROUGH);
        self
    }

    /// Set the foreground color of the last rendered text element.
    pub fn fg(&mut self, color: Color) -> &mut Self {
        self.modify_last_style(|s| s.fg = Some(color));
        self
    }

    /// Set the background color of the last rendered text element.
    pub fn bg(&mut self, color: Color) -> &mut Self {
        self.modify_last_style(|s| s.bg = Some(color));
        self
    }

    pub fn group_hover_fg(&mut self, color: Color) -> &mut Self {
        let apply_group_style = self
            .group_stack
            .last()
            .map(|name| self.is_group_hovered(name) || self.is_group_focused(name))
            .unwrap_or(false);
        if apply_group_style {
            self.modify_last_style(|s| s.fg = Some(color));
        }
        self
    }

    pub fn group_hover_bg(&mut self, color: Color) -> &mut Self {
        let apply_group_style = self
            .group_stack
            .last()
            .map(|name| self.is_group_hovered(name) || self.is_group_focused(name))
            .unwrap_or(false);
        if apply_group_style {
            self.modify_last_style(|s| s.bg = Some(color));
        }
        self
    }

    /// Render a text element with an explicit [`Style`] applied immediately.
    ///
    /// Equivalent to calling `text(s)` followed by style-chain methods, but
    /// more concise when you already have a `Style` value.
    pub fn styled(&mut self, s: impl Into<String>, style: Style) -> &mut Self {
        self.commands.push(Command::Text {
            content: s.into(),
            style,
            grow: 0,
            align: Align::Start,
            wrap: false,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.last_text_idx = Some(self.commands.len() - 1);
        self
    }

    /// Render a half-block image in the terminal.
    ///
    /// Each terminal cell displays two vertical pixels using the `▀` character
    /// with foreground (upper pixel) and background (lower pixel) colors.
    ///
    /// Create a [`HalfBlockImage`] from a file (requires `image` feature):
    /// ```ignore
    /// let img = image::open("photo.png").unwrap();
    /// let half = HalfBlockImage::from_dynamic(&img, 40, 20);
    /// ui.image(&half);
    /// ```
    ///
    /// Or from raw RGB data (no feature needed):
    /// ```no_run
    /// # use slt::{Context, HalfBlockImage};
    /// # slt::run(|ui: &mut Context| {
    /// let rgb = vec![255u8; 30 * 20 * 3];
    /// let half = HalfBlockImage::from_rgb(&rgb, 30, 10);
    /// ui.image(&half);
    /// # });
    /// ```
    pub fn image(&mut self, img: &HalfBlockImage) {
        let width = img.width;
        let height = img.height;

        self.container().w(width).h(height).gap(0).col(|ui| {
            for row in 0..height {
                ui.container().gap(0).row(|ui| {
                    for col in 0..width {
                        let idx = (row * width + col) as usize;
                        if let Some(&(upper, lower)) = img.pixels.get(idx) {
                            ui.styled("▀", Style::new().fg(upper).bg(lower));
                        }
                    }
                });
            }
        });
    }

    /// Render streaming text with a typing cursor indicator.
    ///
    /// Displays the accumulated text content. While `streaming` is true,
    /// shows a blinking cursor (`▌`) at the end.
    ///
    /// ```no_run
    /// # use slt::widgets::StreamingTextState;
    /// # slt::run(|ui: &mut slt::Context| {
    /// let mut stream = StreamingTextState::new();
    /// stream.start();
    /// stream.push("Hello from ");
    /// stream.push("the AI!");
    /// ui.streaming_text(&mut stream);
    /// # });
    /// ```
    pub fn streaming_text(&mut self, state: &mut StreamingTextState) {
        if state.streaming {
            state.cursor_tick = state.cursor_tick.wrapping_add(1);
            state.cursor_visible = (state.cursor_tick / 8) % 2 == 0;
        }

        if state.content.is_empty() && state.streaming {
            let cursor = if state.cursor_visible { "▌" } else { " " };
            let primary = self.theme.primary;
            self.text(cursor).fg(primary);
            return;
        }

        if !state.content.is_empty() {
            if state.streaming && state.cursor_visible {
                self.text_wrap(format!("{}▌", state.content));
            } else {
                self.text_wrap(&state.content);
            }
        }
    }

    /// Render a tool approval widget with approve/reject buttons.
    ///
    /// Shows the tool name, description, and two action buttons.
    /// Returns the updated [`ApprovalAction`] each frame.
    ///
    /// ```no_run
    /// # use slt::widgets::{ApprovalAction, ToolApprovalState};
    /// # slt::run(|ui: &mut slt::Context| {
    /// let mut tool = ToolApprovalState::new("read_file", "Read contents of config.toml");
    /// ui.tool_approval(&mut tool);
    /// if tool.action == ApprovalAction::Approved {
    /// }
    /// # });
    /// ```
    pub fn tool_approval(&mut self, state: &mut ToolApprovalState) {
        let theme = self.theme;
        self.bordered(Border::Rounded).col(|ui| {
            ui.row(|ui| {
                ui.text("⚡").fg(theme.warning);
                ui.text(&state.tool_name).bold().fg(theme.primary);
            });
            ui.text(&state.description).dim();

            if state.action == ApprovalAction::Pending {
                ui.row(|ui| {
                    if ui.button("✓ Approve") {
                        state.action = ApprovalAction::Approved;
                    }
                    if ui.button("✗ Reject") {
                        state.action = ApprovalAction::Rejected;
                    }
                });
            } else {
                let (label, color) = match state.action {
                    ApprovalAction::Approved => ("✓ Approved", theme.success),
                    ApprovalAction::Rejected => ("✗ Rejected", theme.error),
                    ApprovalAction::Pending => unreachable!(),
                };
                ui.text(label).fg(color).bold();
            }
        });
    }

    /// Render a context bar showing active context items with token counts.
    ///
    /// Displays a horizontal bar of context sources (files, URLs, etc.)
    /// with their token counts, useful for AI chat interfaces.
    ///
    /// ```no_run
    /// # use slt::widgets::ContextItem;
    /// # slt::run(|ui: &mut slt::Context| {
    /// let items = vec![ContextItem::new("main.rs", 1200), ContextItem::new("lib.rs", 800)];
    /// ui.context_bar(&items);
    /// # });
    /// ```
    pub fn context_bar(&mut self, items: &[ContextItem]) {
        if items.is_empty() {
            return;
        }

        let theme = self.theme;
        let total: usize = items.iter().map(|item| item.tokens).sum();

        self.container().row(|ui| {
            ui.text("📎").dim();
            for item in items {
                ui.text(format!(
                    "{} ({})",
                    item.label,
                    format_token_count(item.tokens)
                ))
                .fg(theme.secondary);
            }
            ui.spacer();
            ui.text(format!("Σ {}", format_token_count(total))).dim();
        });
    }

    pub fn alert(&mut self, message: &str, level: crate::widgets::AlertLevel) -> bool {
        use crate::widgets::AlertLevel;

        let theme = self.theme;
        let (icon, color) = match level {
            AlertLevel::Info => ("ℹ", theme.accent),
            AlertLevel::Success => ("✓", theme.success),
            AlertLevel::Warning => ("⚠", theme.warning),
            AlertLevel::Error => ("✕", theme.error),
        };

        let focused = self.register_focusable();
        let key_dismiss = focused && (self.key_code(KeyCode::Enter) || self.key('x'));

        let resp = self.container().col(|ui| {
            ui.line(|ui| {
                ui.text(format!(" {icon} ")).fg(color).bold();
                ui.text(message).grow(1);
                ui.text(" [×] ").dim();
            });
        });

        key_dismiss || resp.clicked
    }

    pub fn breadcrumb(&mut self, segments: &[&str]) -> Option<usize> {
        self.breadcrumb_with(segments, " › ")
    }

    pub fn breadcrumb_with(&mut self, segments: &[&str], separator: &str) -> Option<usize> {
        let theme = self.theme;
        let last_idx = segments.len().saturating_sub(1);
        let mut clicked_idx: Option<usize> = None;

        self.row(|ui| {
            for (i, segment) in segments.iter().enumerate() {
                let is_last = i == last_idx;
                if is_last {
                    ui.text(*segment).bold();
                } else {
                    let focused = ui.register_focusable();
                    let pressed = focused && (ui.key_code(KeyCode::Enter) || ui.key(' '));
                    let resp = ui.interaction();
                    let color = if resp.hovered || focused {
                        theme.accent
                    } else {
                        theme.primary
                    };
                    ui.text(*segment).fg(color).underline();
                    if resp.clicked || pressed {
                        clicked_idx = Some(i);
                    }
                    ui.text(separator).dim();
                }
            }
        });

        clicked_idx
    }

    pub fn accordion(&mut self, title: &str, open: &mut bool, f: impl FnOnce(&mut Context)) {
        let theme = self.theme;
        let focused = self.register_focusable();

        if focused && self.key_code(KeyCode::Enter) {
            *open = !*open;
        }

        let icon = if *open { "▾" } else { "▸" };
        let title_color = if focused { theme.primary } else { theme.text };

        let resp = self.container().col(|ui| {
            ui.line(|ui| {
                ui.text(icon).fg(title_color);
                ui.text(format!(" {title}")).bold().fg(title_color);
            });
        });

        if resp.clicked {
            *open = !*open;
        }

        if *open {
            self.container().pl(2).col(f);
        }
    }

    pub fn definition_list(&mut self, items: &[(&str, &str)]) {
        let max_key_width = items
            .iter()
            .map(|(k, _)| unicode_width::UnicodeWidthStr::width(*k))
            .max()
            .unwrap_or(0);

        self.col(|ui| {
            for (key, value) in items {
                ui.line(|ui| {
                    let padded = format!("{:>width$}", key, width = max_key_width);
                    ui.text(padded).dim();
                    ui.text("  ");
                    ui.text(*value);
                });
            }
        });
    }

    pub fn divider_text(&mut self, label: &str) {
        let w = self.width();
        let label_len = unicode_width::UnicodeWidthStr::width(label) as u32;
        let pad = 1u32;
        let left_len = 4u32;
        let right_len = w.saturating_sub(left_len + pad + label_len + pad);
        let left: String = "─".repeat(left_len as usize);
        let right: String = "─".repeat(right_len as usize);
        let theme = self.theme;
        self.line(|ui| {
            ui.text(&left).fg(theme.border);
            ui.text(format!(" {} ", label)).fg(theme.text);
            ui.text(&right).fg(theme.border);
        });
    }

    pub fn badge(&mut self, label: &str) {
        let theme = self.theme;
        self.badge_colored(label, theme.primary);
    }

    pub fn badge_colored(&mut self, label: &str, color: Color) {
        let fg = Color::contrast_fg(color);
        self.text(format!(" {} ", label)).fg(fg).bg(color);
    }

    pub fn key_hint(&mut self, key: &str) {
        let theme = self.theme;
        self.text(format!(" {} ", key))
            .reversed()
            .fg(theme.text_dim);
    }

    pub fn stat(&mut self, label: &str, value: &str) {
        self.col(|ui| {
            ui.text(label).dim();
            ui.text(value).bold();
        });
    }

    pub fn stat_colored(&mut self, label: &str, value: &str, color: Color) {
        self.col(|ui| {
            ui.text(label).dim();
            ui.text(value).bold().fg(color);
        });
    }

    pub fn stat_trend(&mut self, label: &str, value: &str, trend: crate::widgets::Trend) {
        let theme = self.theme;
        let (arrow, color) = match trend {
            crate::widgets::Trend::Up => ("↑", theme.success),
            crate::widgets::Trend::Down => ("↓", theme.error),
        };
        self.col(|ui| {
            ui.text(label).dim();
            ui.line(|ui| {
                ui.text(value).bold();
                ui.text(format!(" {arrow}")).fg(color);
            });
        });
    }

    pub fn empty_state(&mut self, title: &str, description: &str) {
        self.container().center().col(|ui| {
            ui.text(title).align(Align::Center);
            ui.text(description).dim().align(Align::Center);
        });
    }

    pub fn empty_state_action(
        &mut self,
        title: &str,
        description: &str,
        action_label: &str,
    ) -> bool {
        let mut clicked = false;
        self.container().center().col(|ui| {
            ui.text(title).align(Align::Center);
            ui.text(description).dim().align(Align::Center);
            if ui.button(action_label) {
                clicked = true;
            }
        });
        clicked
    }

    pub fn code_block(&mut self, code: &str) {
        let theme = self.theme;
        self.bordered(Border::Rounded)
            .bg(theme.surface)
            .pad(1)
            .col(|ui| {
                for line in code.lines() {
                    render_highlighted_line(ui, line);
                }
            });
    }

    pub fn code_block_numbered(&mut self, code: &str) {
        let lines: Vec<&str> = code.lines().collect();
        let gutter_w = format!("{}", lines.len()).len();
        let theme = self.theme;
        self.bordered(Border::Rounded)
            .bg(theme.surface)
            .pad(1)
            .col(|ui| {
                for (i, line) in lines.iter().enumerate() {
                    ui.line(|ui| {
                        ui.text(format!("{:>gutter_w$} │ ", i + 1))
                            .fg(theme.text_dim);
                        render_highlighted_line(ui, line);
                    });
                }
            });
    }

    /// Enable word-boundary wrapping on the last rendered text element.
    pub fn wrap(&mut self) -> &mut Self {
        if let Some(idx) = self.last_text_idx {
            if let Command::Text { wrap, .. } = &mut self.commands[idx] {
                *wrap = true;
            }
        }
        self
    }

    fn modify_last_style(&mut self, f: impl FnOnce(&mut Style)) {
        if let Some(idx) = self.last_text_idx {
            match &mut self.commands[idx] {
                Command::Text { style, .. } | Command::Link { style, .. } => f(style),
                _ => {}
            }
        }
    }

    // ── containers ───────────────────────────────────────────────────

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
        let mut segments: Vec<(String, Style)> = Vec::new();
        for cmd in self.commands.drain(start..) {
            if let Command::Text { content, style, .. } = cmd {
                segments.push((content, style));
            }
        }
        self.commands.push(Command::RichText {
            segments,
            wrap: true,
            align: Align::Start,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.last_text_idx = None;
        self
    }

    /// Render content in a modal overlay with dimmed background.
    ///
    /// ```ignore
    /// ui.modal(|ui| {
    ///     ui.text("Are you sure?");
    ///     if ui.button("OK") { show = false; }
    /// });
    /// ```
    pub fn modal(&mut self, f: impl FnOnce(&mut Context)) {
        self.commands.push(Command::BeginOverlay { modal: true });
        self.overlay_depth += 1;
        self.modal_active = true;
        f(self);
        self.overlay_depth = self.overlay_depth.saturating_sub(1);
        self.commands.push(Command::EndOverlay);
        self.last_text_idx = None;
    }

    /// Render floating content without dimming the background.
    pub fn overlay(&mut self, f: impl FnOnce(&mut Context)) {
        self.commands.push(Command::BeginOverlay { modal: false });
        self.overlay_depth += 1;
        f(self);
        self.overlay_depth = self.overlay_depth.saturating_sub(1);
        self.commands.push(Command::EndOverlay);
        self.last_text_idx = None;
    }

    /// Create a named group container for shared hover/focus styling.
    ///
    /// ```ignore
    /// ui.group("card").border(Border::Rounded)
    ///     .group_hover_bg(Color::Indexed(238))
    ///     .col(|ui| { ui.text("Hover anywhere"); });
    /// ```
    pub fn group(&mut self, name: &str) -> ContainerBuilder<'_> {
        self.group_count = self.group_count.saturating_add(1);
        self.group_stack.push(name.to_string());
        self.container().group_name(name.to_string())
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
    ///     .pad(1)
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
            align: Align::Start,
            justify: Justify::Start,
            border: None,
            border_sides: BorderSides::all(),
            border_style: Style::new().fg(border),
            bg: None,
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
        let index = self.scroll_count;
        self.scroll_count += 1;
        if let Some(&(ch, vh)) = self.prev_scroll_infos.get(index) {
            state.set_bounds(ch, vh);
            let max = ch.saturating_sub(vh) as usize;
            state.offset = state.offset.min(max);
        }

        let next_id = self.interaction_count;
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
        let track_char = '│';
        let thumb_char = '█';

        self.container().w(1).h(track_height).col(|ui| {
            for i in 0..track_height {
                if i >= thumb_pos && i < thumb_pos + thumb_height {
                    ui.styled(thumb_char.to_string(), Style::new().fg(theme.primary));
                } else {
                    ui.styled(
                        track_char.to_string(),
                        Style::new().fg(theme.text_dim).dim(),
                    );
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
        let mut to_consume: Vec<usize> = Vec::new();

        for (i, event) in self.events.iter().enumerate() {
            if self.consumed[i] {
                continue;
            }
            if let Event::Mouse(mouse) = event {
                let in_bounds = mouse.x >= rect.x
                    && mouse.x < rect.right()
                    && mouse.y >= rect.y
                    && mouse.y < rect.bottom();
                if !in_bounds {
                    continue;
                }
                let in_inner = inner_scroll_rects.iter().any(|sr| {
                    mouse.x >= sr.x
                        && mouse.x < sr.right()
                        && mouse.y >= sr.y
                        && mouse.y < sr.bottom()
                });
                if in_inner {
                    continue;
                }
                match mouse.kind {
                    MouseKind::ScrollUp => {
                        state.scroll_up(1);
                        to_consume.push(i);
                    }
                    MouseKind::ScrollDown => {
                        state.scroll_down(1);
                        to_consume.push(i);
                    }
                    MouseKind::Drag(MouseButton::Left) => {}
                    _ => {}
                }
            }
        }

        for i in to_consume {
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
        let interaction_id = self.interaction_count;
        self.interaction_count += 1;
        let border = self.theme.border;

        self.commands.push(Command::BeginContainer {
            direction,
            gap,
            align: Align::Start,
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
        });
        f(self);
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self.response_for(interaction_id)
    }

    pub(super) fn response_for(&self, interaction_id: usize) -> Response {
        if (self.modal_active || self.prev_modal_active) && self.overlay_depth == 0 {
            return Response::default();
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
            Response { clicked, hovered }
        } else {
            Response::default()
        }
    }

    /// Returns true if the named group is currently hovered by the mouse.
    pub fn is_group_hovered(&self, name: &str) -> bool {
        if let Some(pos) = self.mouse_pos {
            self.prev_group_rects.iter().any(|(n, rect)| {
                n == name
                    && pos.0 >= rect.x
                    && pos.0 < rect.x + rect.width
                    && pos.1 >= rect.y
                    && pos.1 < rect.y + rect.height
            })
        } else {
            false
        }
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

    /// Set the flex-grow factor of the last rendered text element.
    ///
    /// A value of `1` causes the element to expand and fill remaining space
    /// along the main axis.
    pub fn grow(&mut self, value: u16) -> &mut Self {
        if let Some(idx) = self.last_text_idx {
            if let Command::Text { grow, .. } = &mut self.commands[idx] {
                *grow = value;
            }
        }
        self
    }

    /// Set the text alignment of the last rendered text element.
    pub fn align(&mut self, align: Align) -> &mut Self {
        if let Some(idx) = self.last_text_idx {
            if let Command::Text {
                align: text_align, ..
            } = &mut self.commands[idx]
            {
                *text_align = align;
            }
        }
        self
    }

    /// Render an invisible spacer that expands to fill available space.
    ///
    /// Useful for pushing siblings to opposite ends of a row or column.
    pub fn spacer(&mut self) -> &mut Self {
        self.commands.push(Command::Spacer { grow: 1 });
        self.last_text_idx = None;
        self
    }

    /// Render a form that groups input fields vertically.
    ///
    /// Use [`Context::form_field`] inside the closure to render each field.
    pub fn form(
        &mut self,
        state: &mut FormState,
        f: impl FnOnce(&mut Context, &mut FormState),
    ) -> &mut Self {
        self.col(|ui| {
            f(ui, state);
        });
        self
    }

    /// Render a single form field with label and input.
    ///
    /// Shows a validation error below the input when present.
    pub fn form_field(&mut self, field: &mut FormField) -> &mut Self {
        self.col(|ui| {
            ui.styled(field.label.clone(), Style::new().bold().fg(ui.theme.text));
            ui.text_input(&mut field.input);
            if let Some(error) = field.error.as_deref() {
                ui.styled(error.to_string(), Style::new().dim().fg(ui.theme.error));
            }
        });
        self
    }

    /// Render a submit button.
    ///
    /// Returns `true` when the button is clicked or activated.
    pub fn form_submit(&mut self, label: impl Into<String>) -> bool {
        self.button(label)
    }
}

const KEYWORDS: &[&str] = &[
    "fn",
    "let",
    "mut",
    "pub",
    "use",
    "impl",
    "struct",
    "enum",
    "trait",
    "type",
    "const",
    "static",
    "if",
    "else",
    "match",
    "for",
    "while",
    "loop",
    "return",
    "break",
    "continue",
    "where",
    "self",
    "super",
    "crate",
    "mod",
    "async",
    "await",
    "move",
    "ref",
    "in",
    "as",
    "true",
    "false",
    "Some",
    "None",
    "Ok",
    "Err",
    "Self",
    "def",
    "class",
    "import",
    "from",
    "pass",
    "lambda",
    "yield",
    "with",
    "try",
    "except",
    "raise",
    "finally",
    "elif",
    "del",
    "global",
    "nonlocal",
    "assert",
    "is",
    "not",
    "and",
    "or",
    "function",
    "var",
    "const",
    "export",
    "default",
    "switch",
    "case",
    "throw",
    "catch",
    "typeof",
    "instanceof",
    "new",
    "delete",
    "void",
    "this",
    "null",
    "undefined",
    "func",
    "package",
    "defer",
    "go",
    "chan",
    "select",
    "range",
    "map",
    "interface",
    "fallthrough",
    "nil",
];

fn render_highlighted_line(ui: &mut Context, line: &str) {
    let theme = ui.theme;
    let is_light = matches!(
        theme.bg,
        Color::Reset | Color::White | Color::Rgb(255, 255, 255)
    );
    let keyword_color = if is_light {
        Color::Rgb(166, 38, 164)
    } else {
        Color::Rgb(198, 120, 221)
    };
    let string_color = if is_light {
        Color::Rgb(80, 161, 79)
    } else {
        Color::Rgb(152, 195, 121)
    };
    let comment_color = theme.text_dim;
    let number_color = if is_light {
        Color::Rgb(152, 104, 1)
    } else {
        Color::Rgb(209, 154, 102)
    };
    let fn_color = if is_light {
        Color::Rgb(64, 120, 242)
    } else {
        Color::Rgb(97, 175, 239)
    };
    let macro_color = if is_light {
        Color::Rgb(1, 132, 188)
    } else {
        Color::Rgb(86, 182, 194)
    };

    let trimmed = line.trim_start();
    let indent = &line[..line.len() - trimmed.len()];
    if !indent.is_empty() {
        ui.text(indent);
    }

    if trimmed.starts_with("//") {
        ui.text(trimmed).fg(comment_color).italic();
        return;
    }

    let mut pos = 0;

    while pos < trimmed.len() {
        let ch = trimmed.as_bytes()[pos];

        if ch == b'"' {
            if let Some(end) = trimmed[pos + 1..].find('"') {
                let s = &trimmed[pos..pos + end + 2];
                ui.text(s).fg(string_color);
                pos += end + 2;
                continue;
            }
        }

        if ch.is_ascii_digit() && (pos == 0 || !trimmed.as_bytes()[pos - 1].is_ascii_alphanumeric())
        {
            let end = trimmed[pos..]
                .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '_')
                .map_or(trimmed.len(), |e| pos + e);
            ui.text(&trimmed[pos..end]).fg(number_color);
            pos = end;
            continue;
        }

        if ch.is_ascii_alphabetic() || ch == b'_' {
            let end = trimmed[pos..]
                .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .map_or(trimmed.len(), |e| pos + e);
            let word = &trimmed[pos..end];

            if end < trimmed.len() && trimmed.as_bytes()[end] == b'!' {
                ui.text(&trimmed[pos..end + 1]).fg(macro_color);
                pos = end + 1;
            } else if end < trimmed.len()
                && trimmed.as_bytes()[end] == b'('
                && !KEYWORDS.contains(&word)
            {
                ui.text(word).fg(fn_color);
                pos = end;
            } else if KEYWORDS.contains(&word) {
                ui.text(word).fg(keyword_color);
                pos = end;
            } else {
                ui.text(word);
                pos = end;
            }
            continue;
        }

        let end = trimmed[pos..]
            .find(|c: char| c.is_ascii_alphanumeric() || c == '_' || c == '"')
            .map_or(trimmed.len(), |e| pos + e);
        ui.text(&trimmed[pos..end]);
        pos = end;
    }
}
