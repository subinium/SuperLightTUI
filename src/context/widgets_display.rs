use super::*;
use crate::KeyMap;

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
        let default_fg = self
            .text_color_stack
            .iter()
            .rev()
            .find_map(|c| *c)
            .unwrap_or(self.theme.text);
        self.commands.push(Command::Text {
            content,
            style: Style::new().fg(default_fg),
            grow: 0,
            align: Align::Start,
            wrap: false,
            truncate: false,
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
    #[allow(clippy::print_stderr)]
    pub fn link(&mut self, text: impl Into<String>, url: impl Into<String>) -> &mut Self {
        let url_str = url.into();
        let focused = self.register_focusable();
        let interaction_id = self.next_interaction_id();
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
            if let Err(e) = open_url(&url_str) {
                eprintln!("[slt] failed to open URL: {e}");
            }
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
        let default_fg = self
            .text_color_stack
            .iter()
            .rev()
            .find_map(|c| *c)
            .unwrap_or(self.theme.text);
        self.commands.push(Command::Text {
            content,
            style: Style::new().fg(default_fg),
            grow: 0,
            align: Align::Start,
            wrap: true,
            truncate: false,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.last_text_idx = Some(self.commands.len() - 1);
        self
    }

    /// Render an elapsed time display.
    ///
    /// Formats as `HH:MM:SS.CC` when hours are non-zero, otherwise `MM:SS.CC`.
    pub fn timer_display(&mut self, elapsed: std::time::Duration) -> &mut Self {
        let total_centis = elapsed.as_millis() / 10;
        let centis = total_centis % 100;
        let total_seconds = total_centis / 100;
        let seconds = total_seconds % 60;
        let minutes = (total_seconds / 60) % 60;
        let hours = total_seconds / 3600;

        let content = if hours > 0 {
            format!("{hours:02}:{minutes:02}:{seconds:02}.{centis:02}")
        } else {
            format!("{minutes:02}:{seconds:02}.{centis:02}")
        };

        self.commands.push(Command::Text {
            content,
            style: Style::new().fg(self.theme.text),
            grow: 0,
            align: Align::Start,
            wrap: false,
            truncate: false,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.last_text_idx = Some(self.commands.len() - 1);
        self
    }

    /// Render help bar from a KeyMap. Shows visible bindings as key-description pairs.
    pub fn help_from_keymap(&mut self, keymap: &KeyMap) -> Response {
        let pairs: Vec<(&str, &str)> = keymap
            .visible_bindings()
            .map(|binding| (binding.display.as_str(), binding.description.as_str()))
            .collect();
        self.help(&pairs)
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

    /// Apply a per-character foreground gradient to the last rendered text.
    pub fn gradient(&mut self, from: Color, to: Color) -> &mut Self {
        if let Some(idx) = self.last_text_idx {
            let replacement = match &self.commands[idx] {
                Command::Text {
                    content,
                    style,
                    wrap,
                    align,
                    margin,
                    constraints,
                    ..
                } => {
                    let chars: Vec<char> = content.chars().collect();
                    let len = chars.len();
                    let denom = len.saturating_sub(1).max(1) as f32;
                    let segments = chars
                        .into_iter()
                        .enumerate()
                        .map(|(i, ch)| {
                            let mut seg_style = *style;
                            seg_style.fg = Some(from.blend(to, i as f32 / denom));
                            (ch.to_string(), seg_style)
                        })
                        .collect();

                    Some(Command::RichText {
                        segments,
                        wrap: *wrap,
                        align: *align,
                        margin: *margin,
                        constraints: *constraints,
                    })
                }
                _ => None,
            };

            if let Some(command) = replacement {
                self.commands[idx] = command;
            }
        }

        self
    }

    /// Set foreground color when the current group is hovered or focused.
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

    /// Set background color when the current group is hovered or focused.
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
            truncate: false,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.last_text_idx = Some(self.commands.len() - 1);
        self
    }

    /// Render 8x8 bitmap text as half-block pixels (4 terminal rows tall).
    pub fn big_text(&mut self, s: impl Into<String>) -> Response {
        let text = s.into();
        let glyphs: Vec<[u8; 8]> = text.chars().map(glyph_8x8).collect();
        let total_width = (glyphs.len() as u32).saturating_mul(8);
        let on_color = self.theme.primary;

        self.container().w(total_width).h(4).draw(move |buf, rect| {
            if rect.width == 0 || rect.height == 0 {
                return;
            }

            for (glyph_idx, glyph) in glyphs.iter().enumerate() {
                let base_x = rect.x + (glyph_idx as u32) * 8;
                if base_x >= rect.right() {
                    break;
                }

                for pair in 0..4usize {
                    let y = rect.y + pair as u32;
                    if y >= rect.bottom() {
                        continue;
                    }

                    let upper = glyph[pair * 2];
                    let lower = glyph[pair * 2 + 1];

                    for bit in 0..8u32 {
                        let x = base_x + bit;
                        if x >= rect.right() {
                            break;
                        }

                        let mask = 1u8 << (bit as u8);
                        let upper_on = (upper & mask) != 0;
                        let lower_on = (lower & mask) != 0;
                        let (ch, fg, bg) = match (upper_on, lower_on) {
                            (true, true) => ('█', on_color, on_color),
                            (true, false) => ('▀', on_color, Color::Reset),
                            (false, true) => ('▄', on_color, Color::Reset),
                            (false, false) => (' ', Color::Reset, Color::Reset),
                        };
                        buf.set_char(x, y, ch, Style::new().fg(fg).bg(bg));
                    }
                }
            }
        });

        Response::none()
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
    pub fn image(&mut self, img: &HalfBlockImage) -> Response {
        let width = img.width;
        let height = img.height;

        let _ = self.container().w(width).h(height).gap(0).col(|ui| {
            for row in 0..height {
                let _ = ui.container().gap(0).row(|ui| {
                    for col in 0..width {
                        let idx = (row * width + col) as usize;
                        if let Some(&(upper, lower)) = img.pixels.get(idx) {
                            ui.styled("▀", Style::new().fg(upper).bg(lower));
                        }
                    }
                });
            }
        });

        Response::none()
    }

    /// Render a pixel-perfect image using the Kitty graphics protocol.
    ///
    /// The image data must be raw RGBA bytes (4 bytes per pixel).
    /// The widget allocates `cols` x `rows` cells and renders the image
    /// at full pixel resolution within that space.
    ///
    /// Requires a Kitty-compatible terminal (Kitty, Ghostty, WezTerm).
    /// On unsupported terminals, the area will be blank.
    ///
    /// # Arguments
    /// * `rgba` - Raw RGBA pixel data
    /// * `pixel_width` - Image width in pixels
    /// * `pixel_height` - Image height in pixels
    /// * `cols` - Terminal cell columns to occupy
    /// * `rows` - Terminal cell rows to occupy
    pub fn kitty_image(
        &mut self,
        rgba: &[u8],
        pixel_width: u32,
        pixel_height: u32,
        cols: u32,
        rows: u32,
    ) -> Response {
        let rgba = normalize_rgba(rgba, pixel_width, pixel_height);
        let encoded = base64_encode(&rgba);
        let pw = pixel_width;
        let ph = pixel_height;
        let c = cols;
        let r = rows;

        self.container().w(cols).h(rows).draw(move |buf, rect| {
            let chunks = split_base64(&encoded, 4096);
            let mut all_sequences = String::new();

            for (i, chunk) in chunks.iter().enumerate() {
                let more = if i < chunks.len() - 1 { 1 } else { 0 };
                if i == 0 {
                    all_sequences.push_str(&format!(
                        "\x1b_Ga=T,f=32,s={},v={},c={},r={},C=1,q=2,m={};{}\x1b\\",
                        pw, ph, c, r, more, chunk
                    ));
                } else {
                    all_sequences.push_str(&format!("\x1b_Gm={};{}\x1b\\", more, chunk));
                }
            }

            buf.raw_sequence(rect.x, rect.y, all_sequences);
        });
        Response::none()
    }

    /// Render a pixel-perfect image that preserves aspect ratio.
    ///
    /// Sends the original RGBA data to the terminal and lets the Kitty
    /// protocol handle scaling. The container width is `cols` cells;
    /// height is calculated automatically from the image aspect ratio
    /// (assuming 8px wide, 16px tall per cell).
    ///
    /// Requires a Kitty-compatible terminal (Kitty, Ghostty, WezTerm).
    pub fn kitty_image_fit(
        &mut self,
        rgba: &[u8],
        src_width: u32,
        src_height: u32,
        cols: u32,
    ) -> Response {
        let rows = if src_width == 0 {
            1
        } else {
            ((cols as f64 * src_height as f64 * 8.0) / (src_width as f64 * 16.0))
                .ceil()
                .max(1.0) as u32
        };
        let rgba = normalize_rgba(rgba, src_width, src_height);
        let sw = src_width;
        let sh = src_height;
        let c = cols;
        let r = rows;

        self.container().w(cols).h(rows).draw(move |buf, rect| {
            if rect.width == 0 || rect.height == 0 {
                return;
            }
            let encoded = base64_encode(&rgba);
            let chunks = split_base64(&encoded, 4096);
            let mut seq = String::new();
            for (i, chunk) in chunks.iter().enumerate() {
                let more = if i < chunks.len() - 1 { 1 } else { 0 };
                if i == 0 {
                    seq.push_str(&format!(
                        "\x1b_Ga=T,f=32,s={},v={},c={},r={},C=1,q=2,m={};{}\x1b\\",
                        sw, sh, c, r, more, chunk
                    ));
                } else {
                    seq.push_str(&format!("\x1b_Gm={};{}\x1b\\", more, chunk));
                }
            }
            buf.raw_sequence(rect.x, rect.y, seq);
        });
        Response::none()
    }

    /// Render an image using the Sixel protocol.
    #[cfg(feature = "crossterm")]
    pub fn sixel_image(
        &mut self,
        rgba: &[u8],
        pixel_w: u32,
        pixel_h: u32,
        cols: u32,
        rows: u32,
    ) -> Response {
        let sixel_supported = self.is_real_terminal && terminal_supports_sixel();
        if !sixel_supported {
            self.container().w(cols).h(rows).draw(|buf, rect| {
                if rect.width == 0 || rect.height == 0 {
                    return;
                }
                buf.set_string(rect.x, rect.y, "[sixel unsupported]", Style::new());
            });
            return Response::none();
        }

        let rgba = normalize_rgba(rgba, pixel_w, pixel_h);
        let encoded = crate::sixel::encode_sixel(&rgba, pixel_w, pixel_h, 256);

        if encoded.is_empty() {
            self.container().w(cols).h(rows).draw(|buf, rect| {
                if rect.width == 0 || rect.height == 0 {
                    return;
                }
                buf.set_string(rect.x, rect.y, "[sixel empty]", Style::new());
            });
            return Response::none();
        }

        self.container().w(cols).h(rows).draw(move |buf, rect| {
            if rect.width == 0 || rect.height == 0 {
                return;
            }
            buf.raw_sequence(rect.x, rect.y, encoded);
        });
        Response::none()
    }

    /// Render an image using the Sixel protocol.
    #[cfg(not(feature = "crossterm"))]
    pub fn sixel_image(
        &mut self,
        _rgba: &[u8],
        _pixel_w: u32,
        _pixel_h: u32,
        cols: u32,
        rows: u32,
    ) -> Response {
        self.container().w(cols).h(rows).draw(|buf, rect| {
            if rect.width == 0 || rect.height == 0 {
                return;
            }
            buf.set_string(rect.x, rect.y, "[sixel unsupported]", Style::new());
        });
        Response::none()
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
    pub fn streaming_text(&mut self, state: &mut StreamingTextState) -> Response {
        if state.streaming {
            state.cursor_tick = state.cursor_tick.wrapping_add(1);
            state.cursor_visible = (state.cursor_tick / 8) % 2 == 0;
        }

        if state.content.is_empty() && state.streaming {
            let cursor = if state.cursor_visible { "▌" } else { " " };
            let primary = self.theme.primary;
            self.text(cursor).fg(primary);
            return Response::none();
        }

        if !state.content.is_empty() {
            if state.streaming && state.cursor_visible {
                self.text_wrap(format!("{}▌", state.content));
            } else {
                self.text_wrap(&state.content);
            }
        }

        Response::none()
    }

    /// Render streaming markdown with a typing cursor indicator.
    ///
    /// Parses accumulated markdown content line-by-line while streaming.
    /// Supports headings, lists, inline formatting, horizontal rules, and
    /// fenced code blocks with open/close tracking across stream chunks.
    ///
    /// ```no_run
    /// # use slt::widgets::StreamingMarkdownState;
    /// # slt::run(|ui: &mut slt::Context| {
    /// let mut stream = StreamingMarkdownState::new();
    /// stream.start();
    /// stream.push("# Hello\n");
    /// stream.push("- **streaming** markdown\n");
    /// stream.push("```rust\nlet x = 1;\n");
    /// ui.streaming_markdown(&mut stream);
    /// # });
    /// ```
    pub fn streaming_markdown(
        &mut self,
        state: &mut crate::widgets::StreamingMarkdownState,
    ) -> Response {
        if state.streaming {
            state.cursor_tick = state.cursor_tick.wrapping_add(1);
            state.cursor_visible = (state.cursor_tick / 8) % 2 == 0;
        }

        if state.content.is_empty() && state.streaming {
            let cursor = if state.cursor_visible { "▌" } else { " " };
            let primary = self.theme.primary;
            self.text(cursor).fg(primary);
            return Response::none();
        }

        let show_cursor = state.streaming && state.cursor_visible;
        let trailing_newline = state.content.ends_with('\n');
        let lines: Vec<&str> = state.content.lines().collect();
        let last_line_index = lines.len().saturating_sub(1);

        self.commands.push(Command::BeginContainer {
            direction: Direction::Column,
            gap: 0,
            align: Align::Start,
            align_self: None,
            justify: Justify::Start,
            border: None,
            border_sides: BorderSides::all(),
            border_style: Style::new().fg(self.theme.border),
            bg_color: None,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
            group_name: None,
        });
        self.interaction_count += 1;

        let text_style = Style::new().fg(self.theme.text);
        let bold_style = Style::new().fg(self.theme.text).bold();
        let code_style = Style::new().fg(self.theme.accent);
        let border_style = Style::new().fg(self.theme.border).dim();

        let mut in_code_block = false;
        let mut code_block_lang = String::new();

        for (idx, line) in lines.iter().enumerate() {
            let line = *line;
            let trimmed = line.trim();
            let append_cursor = show_cursor && !trailing_newline && idx == last_line_index;
            let cursor = if append_cursor { "▌" } else { "" };

            if in_code_block {
                if trimmed.starts_with("```") {
                    in_code_block = false;
                    code_block_lang.clear();
                    let mut line = String::from("  └────");
                    line.push_str(cursor);
                    self.styled(line, border_style);
                } else {
                    self.line(|ui| {
                        ui.text("  ");
                        render_highlighted_line(ui, line);
                        if !cursor.is_empty() {
                            ui.styled(cursor, Style::new().fg(ui.theme.primary));
                        }
                    });
                }
                continue;
            }

            if trimmed.is_empty() {
                if append_cursor {
                    self.styled("▌", Style::new().fg(self.theme.primary));
                } else {
                    self.text(" ");
                }
                continue;
            }

            if trimmed == "---" || trimmed == "***" || trimmed == "___" {
                let mut line = "─".repeat(40);
                line.push_str(cursor);
                self.styled(line, border_style);
                continue;
            }

            if let Some(heading) = trimmed.strip_prefix("### ") {
                let mut line = String::with_capacity(heading.len() + cursor.len());
                line.push_str(heading);
                line.push_str(cursor);
                self.styled(line, Style::new().bold().fg(self.theme.accent));
                continue;
            }

            if let Some(heading) = trimmed.strip_prefix("## ") {
                let mut line = String::with_capacity(heading.len() + cursor.len());
                line.push_str(heading);
                line.push_str(cursor);
                self.styled(line, Style::new().bold().fg(self.theme.secondary));
                continue;
            }

            if let Some(heading) = trimmed.strip_prefix("# ") {
                let mut line = String::with_capacity(heading.len() + cursor.len());
                line.push_str(heading);
                line.push_str(cursor);
                self.styled(line, Style::new().bold().fg(self.theme.primary));
                continue;
            }

            if let Some(code) = trimmed.strip_prefix("```") {
                in_code_block = true;
                code_block_lang = code.trim().to_string();
                let label = if code_block_lang.is_empty() {
                    "code".to_string()
                } else {
                    let mut label = String::from("code:");
                    label.push_str(&code_block_lang);
                    label
                };
                let mut line = String::with_capacity(5 + label.len() + cursor.len());
                line.push_str("  ┌─");
                line.push_str(&label);
                line.push('─');
                line.push_str(cursor);
                self.styled(line, border_style);
                continue;
            }

            if let Some(item) = trimmed
                .strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* "))
            {
                let segs = Self::parse_inline_segments(item, text_style, bold_style, code_style);
                if segs.len() <= 1 {
                    let mut line = String::with_capacity(4 + item.len() + cursor.len());
                    line.push_str("  • ");
                    line.push_str(item);
                    line.push_str(cursor);
                    self.styled(line, text_style);
                } else {
                    self.line(|ui| {
                        ui.styled("  • ", text_style);
                        for (s, st) in segs {
                            ui.styled(s, st);
                        }
                        if append_cursor {
                            ui.styled("▌", Style::new().fg(ui.theme.primary));
                        }
                    });
                }
                continue;
            }

            if trimmed.starts_with(|c: char| c.is_ascii_digit()) && trimmed.contains(". ") {
                let parts: Vec<&str> = trimmed.splitn(2, ". ").collect();
                if parts.len() == 2 {
                    let segs =
                        Self::parse_inline_segments(parts[1], text_style, bold_style, code_style);
                    if segs.len() <= 1 {
                        let mut line = String::with_capacity(
                            4 + parts[0].len() + parts[1].len() + cursor.len(),
                        );
                        line.push_str("  ");
                        line.push_str(parts[0]);
                        line.push_str(". ");
                        line.push_str(parts[1]);
                        line.push_str(cursor);
                        self.styled(line, text_style);
                    } else {
                        self.line(|ui| {
                            let mut prefix = String::with_capacity(4 + parts[0].len());
                            prefix.push_str("  ");
                            prefix.push_str(parts[0]);
                            prefix.push_str(". ");
                            ui.styled(prefix, text_style);
                            for (s, st) in segs {
                                ui.styled(s, st);
                            }
                            if append_cursor {
                                ui.styled("▌", Style::new().fg(ui.theme.primary));
                            }
                        });
                    }
                } else {
                    let mut line = String::with_capacity(trimmed.len() + cursor.len());
                    line.push_str(trimmed);
                    line.push_str(cursor);
                    self.styled(line, text_style);
                }
                continue;
            }

            let segs = Self::parse_inline_segments(trimmed, text_style, bold_style, code_style);
            if segs.len() <= 1 {
                let mut line = String::with_capacity(trimmed.len() + cursor.len());
                line.push_str(trimmed);
                line.push_str(cursor);
                self.styled(line, text_style);
            } else {
                self.line(|ui| {
                    for (s, st) in segs {
                        ui.styled(s, st);
                    }
                    if append_cursor {
                        ui.styled("▌", Style::new().fg(ui.theme.primary));
                    }
                });
            }
        }

        if show_cursor && trailing_newline {
            if in_code_block {
                self.styled("  ▌", code_style);
            } else {
                self.styled("▌", Style::new().fg(self.theme.primary));
            }
        }

        state.in_code_block = in_code_block;
        state.code_block_lang = code_block_lang;

        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;
        Response::none()
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
    pub fn tool_approval(&mut self, state: &mut ToolApprovalState) -> Response {
        let old_action = state.action;
        let theme = self.theme;
        let _ = self.bordered(Border::Rounded).col(|ui| {
            let _ = ui.row(|ui| {
                ui.text("⚡").fg(theme.warning);
                ui.text(&state.tool_name).bold().fg(theme.primary);
            });
            ui.text(&state.description).dim();

            if state.action == ApprovalAction::Pending {
                let _ = ui.row(|ui| {
                    if ui.button("✓ Approve").clicked {
                        state.action = ApprovalAction::Approved;
                    }
                    if ui.button("✗ Reject").clicked {
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

        Response {
            changed: state.action != old_action,
            ..Response::none()
        }
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
    pub fn context_bar(&mut self, items: &[ContextItem]) -> Response {
        if items.is_empty() {
            return Response::none();
        }

        let theme = self.theme;
        let total: usize = items.iter().map(|item| item.tokens).sum();

        let _ = self.container().row(|ui| {
            ui.text("📎").dim();
            for item in items {
                let token_count = format_token_count(item.tokens);
                let mut line = String::with_capacity(item.label.len() + token_count.len() + 3);
                line.push_str(&item.label);
                line.push_str(" (");
                line.push_str(&token_count);
                line.push(')');
                ui.text(line).fg(theme.secondary);
            }
            ui.spacer();
            let total_text = format_token_count(total);
            let mut line = String::with_capacity(2 + total_text.len());
            line.push_str("Σ ");
            line.push_str(&total_text);
            ui.text(line).dim();
        });

        Response::none()
    }

    /// Render an alert banner with icon and level-based coloring.
    pub fn alert(&mut self, message: &str, level: crate::widgets::AlertLevel) -> Response {
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

        let mut response = self.container().col(|ui| {
            ui.line(|ui| {
                let mut icon_text = String::with_capacity(icon.len() + 2);
                icon_text.push(' ');
                icon_text.push_str(icon);
                icon_text.push(' ');
                ui.text(icon_text).fg(color).bold();
                ui.text(message).grow(1);
                ui.text(" [×] ").dim();
            });
        });
        response.focused = focused;
        if key_dismiss {
            response.clicked = true;
        }

        response
    }

    /// Yes/No confirmation dialog. Returns Response with .clicked=true when answered.
    ///
    /// `result` is set to true for Yes, false for No.
    ///
    /// # Examples
    /// ```
    /// # use slt::*;
    /// # TestBackend::new(80, 24).render(|ui| {
    /// let mut answer = false;
    /// let r = ui.confirm("Delete this file?", &mut answer);
    /// if r.clicked && answer { /* user confirmed */ }
    /// # });
    /// ```
    pub fn confirm(&mut self, question: &str, result: &mut bool) -> Response {
        let focused = self.register_focusable();
        let mut is_yes = *result;
        let mut clicked = false;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }

                    match key.code {
                        KeyCode::Char('y') => {
                            is_yes = true;
                            *result = true;
                            clicked = true;
                            consumed_indices.push(i);
                        }
                        KeyCode::Char('n') => {
                            is_yes = false;
                            *result = false;
                            clicked = true;
                            consumed_indices.push(i);
                        }
                        KeyCode::Tab | KeyCode::BackTab | KeyCode::Left | KeyCode::Right => {
                            is_yes = !is_yes;
                            *result = is_yes;
                            consumed_indices.push(i);
                        }
                        KeyCode::Enter => {
                            *result = is_yes;
                            clicked = true;
                            consumed_indices.push(i);
                        }
                        _ => {}
                    }
                }
            }

            for idx in consumed_indices {
                self.consumed[idx] = true;
            }
        }

        let yes_style = if is_yes {
            if focused {
                Style::new().fg(self.theme.bg).bg(self.theme.success).bold()
            } else {
                Style::new().fg(self.theme.success).bold()
            }
        } else {
            Style::new().fg(self.theme.text_dim)
        };
        let no_style = if !is_yes {
            if focused {
                Style::new().fg(self.theme.bg).bg(self.theme.error).bold()
            } else {
                Style::new().fg(self.theme.error).bold()
            }
        } else {
            Style::new().fg(self.theme.text_dim)
        };

        let q_width = UnicodeWidthStr::width(question) as u32;
        let mut response = self.row(|ui| {
            ui.text(question);
            ui.text(" ");
            ui.styled("[Yes]", yes_style);
            ui.text(" ");
            ui.styled("[No]", no_style);
        });

        if !clicked && response.clicked {
            if let Some((mx, _)) = self.click_pos {
                let yes_start = response.rect.x + q_width + 1;
                let yes_end = yes_start + 5;
                let no_start = yes_end + 1;
                if mx >= yes_start && mx < yes_end {
                    is_yes = true;
                    *result = true;
                    clicked = true;
                } else if mx >= no_start {
                    is_yes = false;
                    *result = false;
                    clicked = true;
                }
            }
        }

        response.focused = focused;
        response.clicked = clicked;
        response.changed = clicked;
        let _ = is_yes;
        response
    }

    /// Render a breadcrumb navigation bar. Returns the clicked segment index.
    pub fn breadcrumb(&mut self, segments: &[&str]) -> Option<usize> {
        self.breadcrumb_with(segments, " › ")
    }

    /// Render a breadcrumb with a custom separator string.
    pub fn breadcrumb_with(&mut self, segments: &[&str], separator: &str) -> Option<usize> {
        let theme = self.theme;
        let last_idx = segments.len().saturating_sub(1);
        let mut clicked_idx: Option<usize> = None;

        let _ = self.row(|ui| {
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

    /// Collapsible section that toggles on click or Enter.
    pub fn accordion(
        &mut self,
        title: &str,
        open: &mut bool,
        f: impl FnOnce(&mut Context),
    ) -> Response {
        let theme = self.theme;
        let focused = self.register_focusable();
        let old_open = *open;

        if focused && self.key_code(KeyCode::Enter) {
            *open = !*open;
        }

        let icon = if *open { "▾" } else { "▸" };
        let title_color = if focused { theme.primary } else { theme.text };

        let mut response = self.container().col(|ui| {
            ui.line(|ui| {
                ui.text(icon).fg(title_color);
                let mut title_text = String::with_capacity(1 + title.len());
                title_text.push(' ');
                title_text.push_str(title);
                ui.text(title_text).bold().fg(title_color);
            });
        });

        if response.clicked {
            *open = !*open;
        }

        if *open {
            let _ = self.container().pl(2).col(f);
        }

        response.focused = focused;
        response.changed = *open != old_open;
        response
    }

    /// Render a key-value definition list with aligned columns.
    pub fn definition_list(&mut self, items: &[(&str, &str)]) -> Response {
        let max_key_width = items
            .iter()
            .map(|(k, _)| unicode_width::UnicodeWidthStr::width(*k))
            .max()
            .unwrap_or(0);

        let _ = self.col(|ui| {
            for (key, value) in items {
                ui.line(|ui| {
                    let padded = format!("{:>width$}", key, width = max_key_width);
                    ui.text(padded).dim();
                    ui.text("  ");
                    ui.text(*value);
                });
            }
        });

        Response::none()
    }

    /// Render a horizontal divider with a centered text label.
    pub fn divider_text(&mut self, label: &str) -> Response {
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
            let mut label_text = String::with_capacity(label.len() + 2);
            label_text.push(' ');
            label_text.push_str(label);
            label_text.push(' ');
            ui.text(label_text).fg(theme.text);
            ui.text(&right).fg(theme.border);
        });

        Response::none()
    }

    /// Render a badge with the theme's primary color.
    pub fn badge(&mut self, label: &str) -> Response {
        let theme = self.theme;
        self.badge_colored(label, theme.primary)
    }

    /// Render a badge with a custom background color.
    pub fn badge_colored(&mut self, label: &str, color: Color) -> Response {
        let fg = Color::contrast_fg(color);
        let mut label_text = String::with_capacity(label.len() + 2);
        label_text.push(' ');
        label_text.push_str(label);
        label_text.push(' ');
        self.text(label_text).fg(fg).bg(color);

        Response::none()
    }

    /// Render a keyboard shortcut hint with reversed styling.
    pub fn key_hint(&mut self, key: &str) -> Response {
        let theme = self.theme;
        let mut key_text = String::with_capacity(key.len() + 2);
        key_text.push(' ');
        key_text.push_str(key);
        key_text.push(' ');
        self.text(key_text).reversed().fg(theme.text_dim);

        Response::none()
    }

    /// Render a label-value stat pair.
    pub fn stat(&mut self, label: &str, value: &str) -> Response {
        let _ = self.col(|ui| {
            ui.text(label).dim();
            ui.text(value).bold();
        });

        Response::none()
    }

    /// Render a stat pair with a custom value color.
    pub fn stat_colored(&mut self, label: &str, value: &str, color: Color) -> Response {
        let _ = self.col(|ui| {
            ui.text(label).dim();
            ui.text(value).bold().fg(color);
        });

        Response::none()
    }

    /// Render a stat pair with an up/down trend arrow.
    pub fn stat_trend(
        &mut self,
        label: &str,
        value: &str,
        trend: crate::widgets::Trend,
    ) -> Response {
        let theme = self.theme;
        let (arrow, color) = match trend {
            crate::widgets::Trend::Up => ("↑", theme.success),
            crate::widgets::Trend::Down => ("↓", theme.error),
        };
        let _ = self.col(|ui| {
            ui.text(label).dim();
            ui.line(|ui| {
                ui.text(value).bold();
                let mut arrow_text = String::with_capacity(1 + arrow.len());
                arrow_text.push(' ');
                arrow_text.push_str(arrow);
                ui.text(arrow_text).fg(color);
            });
        });

        Response::none()
    }

    /// Render a centered empty-state placeholder.
    pub fn empty_state(&mut self, title: &str, description: &str) -> Response {
        let _ = self.container().center().col(|ui| {
            ui.text(title).align(Align::Center);
            ui.text(description).dim().align(Align::Center);
        });

        Response::none()
    }

    /// Render a centered empty-state placeholder with an action button.
    pub fn empty_state_action(
        &mut self,
        title: &str,
        description: &str,
        action_label: &str,
    ) -> Response {
        let mut clicked = false;
        let _ = self.container().center().col(|ui| {
            ui.text(title).align(Align::Center);
            ui.text(description).dim().align(Align::Center);
            if ui.button(action_label).clicked {
                clicked = true;
            }
        });

        Response {
            clicked,
            changed: clicked,
            ..Response::none()
        }
    }

    /// Render a code block with keyword-based syntax highlighting.
    pub fn code_block(&mut self, code: &str) -> Response {
        self.code_block_lang(code, "")
    }

    /// Render a code block with language-aware syntax highlighting.
    pub fn code_block_lang(&mut self, code: &str, lang: &str) -> Response {
        let theme = self.theme;
        let highlighted: Option<Vec<Vec<(String, Style)>>> =
            crate::syntax::highlight_code(code, lang, &theme);
        let _ = self
            .bordered(Border::Rounded)
            .bg(theme.surface)
            .pad(1)
            .col(|ui| {
                if let Some(ref lines) = highlighted {
                    render_tree_sitter_lines(ui, lines);
                } else {
                    for line in code.lines() {
                        render_highlighted_line(ui, line);
                    }
                }
            });

        Response::none()
    }

    /// Render a code block with line numbers and keyword highlighting.
    pub fn code_block_numbered(&mut self, code: &str) -> Response {
        self.code_block_numbered_lang(code, "")
    }

    /// Render a code block with line numbers and language-aware highlighting.
    pub fn code_block_numbered_lang(&mut self, code: &str, lang: &str) -> Response {
        let lines: Vec<&str> = code.lines().collect();
        let gutter_w = format!("{}", lines.len()).len();
        let theme = self.theme;
        let highlighted: Option<Vec<Vec<(String, Style)>>> =
            crate::syntax::highlight_code(code, lang, &theme);
        let _ = self
            .bordered(Border::Rounded)
            .bg(theme.surface)
            .pad(1)
            .col(|ui| {
                if let Some(ref hl_lines) = highlighted {
                    for (i, segs) in hl_lines.iter().enumerate() {
                        ui.line(|ui| {
                            ui.text(format!("{:>gutter_w$} │ ", i + 1))
                                .fg(theme.text_dim);
                            for (text, style) in segs {
                                ui.styled(text, *style);
                            }
                        });
                    }
                } else {
                    for (i, line) in lines.iter().enumerate() {
                        ui.line(|ui| {
                            ui.text(format!("{:>gutter_w$} │ ", i + 1))
                                .fg(theme.text_dim);
                            render_highlighted_line(ui, line);
                        });
                    }
                }
            });

        Response::none()
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

    /// Truncate the last rendered text with `…` when it exceeds its allocated width.
    /// Use with `.w()` to set a fixed width, or let the parent container constrain it.
    pub fn truncate(&mut self) -> &mut Self {
        if let Some(idx) = self.last_text_idx {
            if let Command::Text { truncate, .. } = &mut self.commands[idx] {
                *truncate = true;
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

    fn modify_last_constraints(&mut self, f: impl FnOnce(&mut Constraints)) {
        if let Some(idx) = self.last_text_idx {
            match &mut self.commands[idx] {
                Command::Text { constraints, .. } | Command::Link { constraints, .. } => {
                    f(constraints)
                }
                _ => {}
            }
        }
    }

    fn modify_last_margin(&mut self, f: impl FnOnce(&mut Margin)) {
        if let Some(idx) = self.last_text_idx {
            match &mut self.commands[idx] {
                Command::Text { margin, .. } | Command::Link { margin, .. } => f(margin),
                _ => {}
            }
        }
    }

    // ── containers ───────────────────────────────────────────────────

    /// Conditionally render content when the named screen is active.
    pub fn screen(&mut self, name: &str, screens: &ScreenState, f: impl FnOnce(&mut Context)) {
        if screens.current() == name {
            f(self);
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
    pub fn modal(&mut self, f: impl FnOnce(&mut Context)) -> Response {
        let interaction_id = self.next_interaction_id();
        self.commands.push(Command::BeginOverlay { modal: true });
        self.overlay_depth += 1;
        self.modal_active = true;
        self.modal_focus_start = self.focus_count;
        f(self);
        self.modal_focus_count = self.focus_count.saturating_sub(self.modal_focus_start);
        self.overlay_depth = self.overlay_depth.saturating_sub(1);
        self.commands.push(Command::EndOverlay);
        self.last_text_idx = None;
        self.response_for(interaction_id)
    }

    /// Render floating content without dimming the background.
    pub fn overlay(&mut self, f: impl FnOnce(&mut Context)) -> Response {
        let interaction_id = self.next_interaction_id();
        self.commands.push(Command::BeginOverlay { modal: false });
        self.overlay_depth += 1;
        f(self);
        self.overlay_depth = self.overlay_depth.saturating_sub(1);
        self.commands.push(Command::EndOverlay);
        self.last_text_idx = None;
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
        let last_interaction_id = self.interaction_count.saturating_sub(1);
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

        let _ = self.container().w(1).h(track_height).col(|ui| {
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
        let interaction_id = self.next_interaction_id();
        let border = self.theme.border;

        self.commands.push(Command::BeginContainer {
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
        });
        self.text_color_stack.push(None);
        f(self);
        self.text_color_stack.pop();
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self.response_for(interaction_id)
    }

    pub(super) fn response_for(&self, interaction_id: usize) -> Response {
        if (self.modal_active || self.prev_modal_active) && self.overlay_depth == 0 {
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

    /// Center-align the last rendered text element horizontally.
    /// Shorthand for `.align(Align::Center)`. Requires the text to have
    /// a width constraint (via `.w()` or parent container) to be visible.
    pub fn text_center(&mut self) -> &mut Self {
        self.align(Align::Center)
    }

    /// Right-align the last rendered text element horizontally.
    /// Shorthand for `.align(Align::End)`.
    pub fn text_right(&mut self) -> &mut Self {
        self.align(Align::End)
    }

    // ── size constraints on last text/link ──────────────────────────

    /// Set a fixed width on the last rendered text or link element.
    ///
    /// Sets both `min_width` and `max_width` to `value`, making the element
    /// occupy exactly that many columns (padded with spaces or truncated).
    pub fn w(&mut self, value: u32) -> &mut Self {
        self.modify_last_constraints(|c| {
            c.min_width = Some(value);
            c.max_width = Some(value);
        });
        self
    }

    /// Set a fixed height on the last rendered text or link element.
    ///
    /// Sets both `min_height` and `max_height` to `value`.
    pub fn h(&mut self, value: u32) -> &mut Self {
        self.modify_last_constraints(|c| {
            c.min_height = Some(value);
            c.max_height = Some(value);
        });
        self
    }

    /// Set the minimum width on the last rendered text or link element.
    pub fn min_w(&mut self, value: u32) -> &mut Self {
        self.modify_last_constraints(|c| c.min_width = Some(value));
        self
    }

    /// Set the maximum width on the last rendered text or link element.
    pub fn max_w(&mut self, value: u32) -> &mut Self {
        self.modify_last_constraints(|c| c.max_width = Some(value));
        self
    }

    /// Set the minimum height on the last rendered text or link element.
    pub fn min_h(&mut self, value: u32) -> &mut Self {
        self.modify_last_constraints(|c| c.min_height = Some(value));
        self
    }

    /// Set the maximum height on the last rendered text or link element.
    pub fn max_h(&mut self, value: u32) -> &mut Self {
        self.modify_last_constraints(|c| c.max_height = Some(value));
        self
    }

    // ── margin on last text/link ────────────────────────────────────

    /// Set uniform margin on all sides of the last rendered text or link element.
    pub fn m(&mut self, value: u32) -> &mut Self {
        self.modify_last_margin(|m| *m = Margin::all(value));
        self
    }

    /// Set horizontal margin (left + right) on the last rendered text or link.
    pub fn mx(&mut self, value: u32) -> &mut Self {
        self.modify_last_margin(|m| {
            m.left = value;
            m.right = value;
        });
        self
    }

    /// Set vertical margin (top + bottom) on the last rendered text or link.
    pub fn my(&mut self, value: u32) -> &mut Self {
        self.modify_last_margin(|m| {
            m.top = value;
            m.bottom = value;
        });
        self
    }

    /// Set top margin on the last rendered text or link element.
    pub fn mt(&mut self, value: u32) -> &mut Self {
        self.modify_last_margin(|m| m.top = value);
        self
    }

    /// Set right margin on the last rendered text or link element.
    pub fn mr(&mut self, value: u32) -> &mut Self {
        self.modify_last_margin(|m| m.right = value);
        self
    }

    /// Set bottom margin on the last rendered text or link element.
    pub fn mb(&mut self, value: u32) -> &mut Self {
        self.modify_last_margin(|m| m.bottom = value);
        self
    }

    /// Set left margin on the last rendered text or link element.
    pub fn ml(&mut self, value: u32) -> &mut Self {
        self.modify_last_margin(|m| m.left = value);
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
            ui.styled(field.label.clone(), Style::new().bold().fg(ui.theme.text));
            let _ = ui.text_input(&mut field.input);
            if let Some(error) = field.error.as_deref() {
                ui.styled(error.to_string(), Style::new().dim().fg(ui.theme.error));
            }
        });
        self
    }

    /// Render a submit button.
    ///
    /// Returns `true` when the button is clicked or activated.
    pub fn form_submit(&mut self, label: impl Into<String>) -> Response {
        self.button(label)
    }
}

fn wrap_tooltip_text(text: &str, max_width: usize) -> Vec<String> {
    let max_width = max_width.max(1);
    let mut lines = Vec::new();

    for paragraph in text.lines() {
        if paragraph.trim().is_empty() {
            lines.push(String::new());
            continue;
        }

        let mut current = String::new();
        let mut current_width = 0usize;

        for word in paragraph.split_whitespace() {
            for chunk in split_word_for_width(word, max_width) {
                let chunk_width = UnicodeWidthStr::width(chunk.as_str());

                if current.is_empty() {
                    current = chunk;
                    current_width = chunk_width;
                    continue;
                }

                if current_width + 1 + chunk_width <= max_width {
                    current.push(' ');
                    current.push_str(&chunk);
                    current_width += 1 + chunk_width;
                } else {
                    lines.push(std::mem::take(&mut current));
                    current = chunk;
                    current_width = chunk_width;
                }
            }
        }

        if !current.is_empty() {
            lines.push(current);
        }
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

fn split_word_for_width(word: &str, max_width: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;

    for ch in word.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if !current.is_empty() && current_width + ch_width > max_width {
            chunks.push(std::mem::take(&mut current));
            current_width = 0;
        }
        current.push(ch);
        current_width += ch_width;

        if current_width >= max_width {
            chunks.push(std::mem::take(&mut current));
            current_width = 0;
        }
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    if chunks.is_empty() {
        chunks.push(String::new());
    }

    chunks
}

fn glyph_8x8(ch: char) -> [u8; 8] {
    if ch.is_ascii() {
        let code = ch as u8;
        if (32..=126).contains(&code) {
            return FONT_8X8_PRINTABLE[(code - 32) as usize];
        }
    }

    FONT_8X8_PRINTABLE[(b'?' - 32) as usize]
}

const FONT_8X8_PRINTABLE: [[u8; 8]; 95] = [
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x18, 0x3C, 0x3C, 0x18, 0x18, 0x00, 0x18, 0x00],
    [0x36, 0x36, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x36, 0x36, 0x7F, 0x36, 0x7F, 0x36, 0x36, 0x00],
    [0x0C, 0x3E, 0x03, 0x1E, 0x30, 0x1F, 0x0C, 0x00],
    [0x00, 0x63, 0x33, 0x18, 0x0C, 0x66, 0x63, 0x00],
    [0x1C, 0x36, 0x1C, 0x6E, 0x3B, 0x33, 0x6E, 0x00],
    [0x06, 0x06, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x18, 0x0C, 0x06, 0x06, 0x06, 0x0C, 0x18, 0x00],
    [0x06, 0x0C, 0x18, 0x18, 0x18, 0x0C, 0x06, 0x00],
    [0x00, 0x66, 0x3C, 0xFF, 0x3C, 0x66, 0x00, 0x00],
    [0x00, 0x0C, 0x0C, 0x3F, 0x0C, 0x0C, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C, 0x06],
    [0x00, 0x00, 0x00, 0x3F, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C, 0x00],
    [0x60, 0x30, 0x18, 0x0C, 0x06, 0x03, 0x01, 0x00],
    [0x3E, 0x63, 0x73, 0x7B, 0x6F, 0x67, 0x3E, 0x00],
    [0x0C, 0x0E, 0x0C, 0x0C, 0x0C, 0x0C, 0x3F, 0x00],
    [0x1E, 0x33, 0x30, 0x1C, 0x06, 0x33, 0x3F, 0x00],
    [0x1E, 0x33, 0x30, 0x1C, 0x30, 0x33, 0x1E, 0x00],
    [0x38, 0x3C, 0x36, 0x33, 0x7F, 0x30, 0x78, 0x00],
    [0x3F, 0x03, 0x1F, 0x30, 0x30, 0x33, 0x1E, 0x00],
    [0x1C, 0x06, 0x03, 0x1F, 0x33, 0x33, 0x1E, 0x00],
    [0x3F, 0x33, 0x30, 0x18, 0x0C, 0x0C, 0x0C, 0x00],
    [0x1E, 0x33, 0x33, 0x1E, 0x33, 0x33, 0x1E, 0x00],
    [0x1E, 0x33, 0x33, 0x3E, 0x30, 0x18, 0x0E, 0x00],
    [0x00, 0x0C, 0x0C, 0x00, 0x00, 0x0C, 0x0C, 0x00],
    [0x00, 0x0C, 0x0C, 0x00, 0x00, 0x0C, 0x0C, 0x06],
    [0x18, 0x0C, 0x06, 0x03, 0x06, 0x0C, 0x18, 0x00],
    [0x00, 0x00, 0x3F, 0x00, 0x00, 0x3F, 0x00, 0x00],
    [0x06, 0x0C, 0x18, 0x30, 0x18, 0x0C, 0x06, 0x00],
    [0x1E, 0x33, 0x30, 0x18, 0x0C, 0x00, 0x0C, 0x00],
    [0x3E, 0x63, 0x7B, 0x7B, 0x7B, 0x03, 0x1E, 0x00],
    [0x0C, 0x1E, 0x33, 0x33, 0x3F, 0x33, 0x33, 0x00],
    [0x3F, 0x66, 0x66, 0x3E, 0x66, 0x66, 0x3F, 0x00],
    [0x3C, 0x66, 0x03, 0x03, 0x03, 0x66, 0x3C, 0x00],
    [0x1F, 0x36, 0x66, 0x66, 0x66, 0x36, 0x1F, 0x00],
    [0x7F, 0x46, 0x16, 0x1E, 0x16, 0x46, 0x7F, 0x00],
    [0x7F, 0x46, 0x16, 0x1E, 0x16, 0x06, 0x0F, 0x00],
    [0x3C, 0x66, 0x03, 0x03, 0x73, 0x66, 0x7C, 0x00],
    [0x33, 0x33, 0x33, 0x3F, 0x33, 0x33, 0x33, 0x00],
    [0x1E, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x1E, 0x00],
    [0x78, 0x30, 0x30, 0x30, 0x33, 0x33, 0x1E, 0x00],
    [0x67, 0x66, 0x36, 0x1E, 0x36, 0x66, 0x67, 0x00],
    [0x0F, 0x06, 0x06, 0x06, 0x46, 0x66, 0x7F, 0x00],
    [0x63, 0x77, 0x7F, 0x7F, 0x6B, 0x63, 0x63, 0x00],
    [0x63, 0x67, 0x6F, 0x7B, 0x73, 0x63, 0x63, 0x00],
    [0x1C, 0x36, 0x63, 0x63, 0x63, 0x36, 0x1C, 0x00],
    [0x3F, 0x66, 0x66, 0x3E, 0x06, 0x06, 0x0F, 0x00],
    [0x1E, 0x33, 0x33, 0x33, 0x3B, 0x1E, 0x38, 0x00],
    [0x3F, 0x66, 0x66, 0x3E, 0x36, 0x66, 0x67, 0x00],
    [0x1E, 0x33, 0x07, 0x0E, 0x38, 0x33, 0x1E, 0x00],
    [0x3F, 0x2D, 0x0C, 0x0C, 0x0C, 0x0C, 0x1E, 0x00],
    [0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x3F, 0x00],
    [0x33, 0x33, 0x33, 0x33, 0x33, 0x1E, 0x0C, 0x00],
    [0x63, 0x63, 0x63, 0x6B, 0x7F, 0x77, 0x63, 0x00],
    [0x63, 0x63, 0x36, 0x1C, 0x1C, 0x36, 0x63, 0x00],
    [0x33, 0x33, 0x33, 0x1E, 0x0C, 0x0C, 0x1E, 0x00],
    [0x7F, 0x63, 0x31, 0x18, 0x4C, 0x66, 0x7F, 0x00],
    [0x1E, 0x06, 0x06, 0x06, 0x06, 0x06, 0x1E, 0x00],
    [0x03, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x40, 0x00],
    [0x1E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x1E, 0x00],
    [0x08, 0x1C, 0x36, 0x63, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF],
    [0x0C, 0x0C, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x1E, 0x30, 0x3E, 0x33, 0x6E, 0x00],
    [0x07, 0x06, 0x06, 0x3E, 0x66, 0x66, 0x3B, 0x00],
    [0x00, 0x00, 0x1E, 0x33, 0x03, 0x33, 0x1E, 0x00],
    [0x38, 0x30, 0x30, 0x3E, 0x33, 0x33, 0x6E, 0x00],
    [0x00, 0x00, 0x1E, 0x33, 0x3F, 0x03, 0x1E, 0x00],
    [0x1C, 0x36, 0x06, 0x0F, 0x06, 0x06, 0x0F, 0x00],
    [0x00, 0x00, 0x6E, 0x33, 0x33, 0x3E, 0x30, 0x1F],
    [0x07, 0x06, 0x36, 0x6E, 0x66, 0x66, 0x67, 0x00],
    [0x0C, 0x00, 0x0E, 0x0C, 0x0C, 0x0C, 0x1E, 0x00],
    [0x30, 0x00, 0x30, 0x30, 0x30, 0x33, 0x33, 0x1E],
    [0x07, 0x06, 0x66, 0x36, 0x1E, 0x36, 0x67, 0x00],
    [0x0E, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x1E, 0x00],
    [0x00, 0x00, 0x33, 0x7F, 0x7F, 0x6B, 0x63, 0x00],
    [0x00, 0x00, 0x1F, 0x33, 0x33, 0x33, 0x33, 0x00],
    [0x00, 0x00, 0x1E, 0x33, 0x33, 0x33, 0x1E, 0x00],
    [0x00, 0x00, 0x3B, 0x66, 0x66, 0x3E, 0x06, 0x0F],
    [0x00, 0x00, 0x6E, 0x33, 0x33, 0x3E, 0x30, 0x78],
    [0x00, 0x00, 0x3B, 0x6E, 0x66, 0x06, 0x0F, 0x00],
    [0x00, 0x00, 0x3E, 0x03, 0x1E, 0x30, 0x1F, 0x00],
    [0x08, 0x0C, 0x3E, 0x0C, 0x0C, 0x2C, 0x18, 0x00],
    [0x00, 0x00, 0x33, 0x33, 0x33, 0x33, 0x6E, 0x00],
    [0x00, 0x00, 0x33, 0x33, 0x33, 0x1E, 0x0C, 0x00],
    [0x00, 0x00, 0x63, 0x6B, 0x7F, 0x7F, 0x36, 0x00],
    [0x00, 0x00, 0x63, 0x36, 0x1C, 0x36, 0x63, 0x00],
    [0x00, 0x00, 0x33, 0x33, 0x33, 0x3E, 0x30, 0x1F],
    [0x00, 0x00, 0x3F, 0x19, 0x0C, 0x26, 0x3F, 0x00],
    [0x38, 0x0C, 0x0C, 0x07, 0x0C, 0x0C, 0x38, 0x00],
    [0x18, 0x18, 0x18, 0x00, 0x18, 0x18, 0x18, 0x00],
    [0x07, 0x0C, 0x0C, 0x38, 0x0C, 0x0C, 0x07, 0x00],
    [0x6E, 0x3B, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
];

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

fn render_tree_sitter_lines(ui: &mut Context, lines: &[Vec<(String, crate::style::Style)>]) {
    for segs in lines {
        if segs.is_empty() {
            ui.text(" ");
        } else {
            ui.line(|ui| {
                for (text, style) in segs {
                    ui.styled(text, *style);
                }
            });
        }
    }
}

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

fn normalize_rgba(data: &[u8], width: u32, height: u32) -> Vec<u8> {
    let expected = (width as usize) * (height as usize) * 4;
    if data.len() >= expected {
        return data[..expected].to_vec();
    }
    let mut buf = Vec::with_capacity(expected);
    buf.extend_from_slice(data);
    buf.resize(expected, 0);
    buf
}

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn split_base64(encoded: &str, chunk_size: usize) -> Vec<&str> {
    let mut chunks = Vec::new();
    let bytes = encoded.as_bytes();
    let mut offset = 0;
    while offset < bytes.len() {
        let end = (offset + chunk_size).min(bytes.len());
        chunks.push(&encoded[offset..end]);
        offset = end;
    }
    if chunks.is_empty() {
        chunks.push("");
    }
    chunks
}

#[cfg(feature = "crossterm")]
fn terminal_supports_sixel() -> bool {
    let force = std::env::var("SLT_FORCE_SIXEL")
        .ok()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_default();
    if matches!(force.as_str(), "1" | "true" | "yes" | "on") {
        return true;
    }

    let term = std::env::var("TERM")
        .ok()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_default();
    let term_program = std::env::var("TERM_PROGRAM")
        .ok()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_default();

    term.contains("sixel")
        || term.contains("mlterm")
        || term.contains("xterm")
        || term.contains("foot")
        || term_program.contains("foot")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TestBackend;
    use std::time::Duration;

    #[test]
    fn gradient_text_renders_content() {
        let mut backend = TestBackend::new(20, 4);
        backend.render(|ui| {
            ui.text("ABCD").gradient(Color::Red, Color::Blue);
        });

        backend.assert_contains("ABCD");
    }

    #[test]
    fn big_text_renders_half_block_grid() {
        let mut backend = TestBackend::new(16, 4);
        backend.render(|ui| {
            let _ = ui.big_text("A");
        });

        let output = backend.to_string();
        // Should contain half-block characters (▀, ▄, or █)
        assert!(
            output.contains('▀') || output.contains('▄') || output.contains('█'),
            "output should contain half-block glyphs: {output:?}"
        );
    }

    #[test]
    fn timer_display_formats_minutes_seconds_centis() {
        let mut backend = TestBackend::new(20, 4);
        backend.render(|ui| {
            ui.timer_display(Duration::from_secs(83) + Duration::from_millis(450));
        });

        backend.assert_contains("01:23.45");
    }
}
