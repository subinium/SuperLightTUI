use super::*;

impl Context {
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
        let pixels: Vec<(Color, Color)> = img.pixels.clone();
        let (w, h) = (img.width, img.height);
        self.container().w(w).h(h).draw(move |buf, rect| {
            for row in 0..h {
                for col in 0..w {
                    if let Some(&(fg, bg)) = pixels.get((row * w + col) as usize) {
                        buf.set_char(rect.x + col, rect.y + row, '▀', Style::new().fg(fg).bg(bg));
                    }
                }
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
        let rgba_data = normalize_rgba(rgba, pixel_width, pixel_height);
        let content_hash = crate::buffer::hash_rgba(&rgba_data);
        let rgba_arc = std::sync::Arc::new(rgba_data);
        let sw = pixel_width;
        let sh = pixel_height;

        self.container().w(cols).h(rows).draw(move |buf, rect| {
            if rect.width == 0 || rect.height == 0 {
                return;
            }
            buf.kitty_place(crate::buffer::KittyPlacement {
                content_hash,
                rgba: rgba_arc.clone(),
                src_width: sw,
                src_height: sh,
                x: rect.x,
                y: rect.y,
                cols: rect.width,
                rows: rect.height,
                crop_y: 0,
                crop_h: 0,
            });
        });
        Response::none()
    }

    /// Render a pixel-perfect image that preserves aspect ratio.
    ///
    /// Sends the original RGBA data to the terminal and lets the Kitty
    /// protocol handle scaling. The container width is `cols` cells;
    /// height is calculated automatically from the image aspect ratio
    /// using detected cell pixel dimensions (falls back to 8×16 if
    /// detection fails).
    ///
    /// Requires a Kitty-compatible terminal (Kitty, Ghostty, WezTerm).
    pub fn kitty_image_fit(
        &mut self,
        rgba: &[u8],
        src_width: u32,
        src_height: u32,
        cols: u32,
    ) -> Response {
        #[cfg(feature = "crossterm")]
        let (cell_w, cell_h) = crate::terminal::cell_pixel_size();
        #[cfg(not(feature = "crossterm"))]
        let (cell_w, cell_h) = (8u32, 16u32);

        let rows = if src_width == 0 {
            1
        } else {
            ((cols as f64 * src_height as f64 * cell_w as f64) / (src_width as f64 * cell_h as f64))
                .ceil()
                .max(1.0) as u32
        };
        let rgba_data = normalize_rgba(rgba, src_width, src_height);
        let content_hash = crate::buffer::hash_rgba(&rgba_data);
        let rgba_arc = std::sync::Arc::new(rgba_data);
        let sw = src_width;
        let sh = src_height;

        self.container().w(cols).h(rows).draw(move |buf, rect| {
            if rect.width == 0 || rect.height == 0 {
                return;
            }
            buf.kitty_place(crate::buffer::KittyPlacement {
                content_hash,
                rgba: rgba_arc.clone(),
                src_width: sw,
                src_height: sh,
                x: rect.x,
                y: rect.y,
                cols: rect.width,
                rows: rect.height,
                crop_y: 0,
                crop_h: 0,
            });
        });
        Response::none()
    }

    /// Render an image using the Sixel protocol.
    ///
    /// `rgba` is raw RGBA pixel data, `pixel_width`/`pixel_height` are pixel dimensions,
    /// and `cols`/`rows` are the terminal cell size to reserve for the image.
    ///
    /// Requires the `crossterm` feature (enabled by default). Falls back to
    /// `[sixel unsupported]` on terminals without Sixel support. Set the
    /// `SLT_FORCE_SIXEL=1` environment variable to skip terminal detection.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// // 2x2 red square (RGBA: 4 pixels × 4 bytes)
    /// let rgba = [255u8, 0, 0, 255].repeat(4);
    /// ui.sixel_image(&rgba, 2, 2, 20, 2);
    /// # });
    /// ```
    #[cfg(feature = "crossterm")]
    #[cfg_attr(docsrs, doc(cfg(feature = "crossterm")))]
    pub fn sixel_image(
        &mut self,
        rgba: &[u8],
        pixel_width: u32,
        pixel_height: u32,
        cols: u32,
        rows: u32,
    ) -> Response {
        // Issue #264: consult the negotiated capability snapshot first (the
        // DA1 probe is authoritative when it answered). Fall back to the env
        // allowlist (now including WezTerm/Ghostty) / `SLT_FORCE_SIXEL` when the
        // probe returned unknown. App code never selects a protocol — the
        // blitter ladder resolves it.
        let sixel_supported =
            self.is_real_terminal && (self.capabilities.sixel || terminal_supports_sixel());
        if !sixel_supported {
            self.container().w(cols).h(rows).draw(|buf, rect| {
                if rect.width == 0 || rect.height == 0 {
                    return;
                }
                buf.set_string(rect.x, rect.y, "[sixel unsupported]", Style::new());
            });
            return Response::none();
        }

        let rgba = normalize_rgba(rgba, pixel_width, pixel_height);
        let content_hash = crate::buffer::hash_rgba(&rgba);
        let encoded = crate::sixel::encode_sixel(&rgba, pixel_width, pixel_height, 256);

        if encoded.is_empty() {
            self.container().w(cols).h(rows).draw(|buf, rect| {
                if rect.width == 0 || rect.height == 0 {
                    return;
                }
                buf.set_string(rect.x, rect.y, "[sixel empty]", Style::new());
            });
            return Response::none();
        }

        // Issue #265: route through the sprixel damage matrix instead of a flat
        // `raw_sequence`, so a text edit adjacent to the image no longer forces
        // a full re-blit. The footprint is recorded as fully `Opaque`; the flush
        // layer flips cells to `Annihilated` only where text overwrites ink.
        self.container().w(cols).h(rows).draw(move |buf, rect| {
            if rect.width == 0 || rect.height == 0 {
                return;
            }
            let cells = (rect.width as usize).saturating_mul(rect.height as usize);
            buf.sprixel_place(crate::buffer::SprixelPlacement {
                content_hash,
                seq: encoded,
                x: rect.x,
                y: rect.y,
                cols: rect.width,
                rows: rect.height,
                cells: vec![crate::buffer::SprixelCell::Opaque; cells],
            });
        });
        Response::none()
    }

    /// Render an image via iTerm2's OSC 1337 inline-image protocol.
    ///
    /// Unlike [`Context::kitty_image`] (raw RGBA) or [`Context::sixel_image`]
    /// (raw RGBA, quantized), `data` is **encoded image-file bytes**
    /// (PNG/JPEG/GIF): the terminal decodes and scales the file itself. This is
    /// the pixel-accurate path on Tabby, older iTerm2 builds, and WezTerm's
    /// iTerm2-compat mode (issue #265).
    ///
    /// `cols`/`rows` reserve the cell box for the image. On a terminal without
    /// OSC 1337 support the area is reserved and `[iterm2 unsupported]` is
    /// drawn, mirroring the Sixel fallback. Set `SLT_FORCE_ITERM=1` to skip
    /// detection.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// // `png` holds encoded PNG bytes loaded from disk or memory.
    /// let png = [0x89u8, b'P', b'N', b'G'];
    /// ui.iterm_image(&png, 20, 4);
    /// # });
    /// ```
    #[cfg(feature = "crossterm")]
    #[cfg_attr(docsrs, doc(cfg(feature = "crossterm")))]
    pub fn iterm_image(&mut self, data: &[u8], cols: u32, rows: u32) -> Response {
        // Issue #264 ladder integration: consult the negotiated capability
        // snapshot first, then the env allowlist / `SLT_FORCE_ITERM`. App code
        // never selects a protocol directly.
        let supported =
            self.is_real_terminal && (self.capabilities.iterm2 || terminal_supports_iterm());
        if !supported {
            return self.iterm_placeholder(cols, rows);
        }

        let content_hash = crate::buffer::hash_rgba(data);
        let encoded = crate::iterm::encode_iterm_osc1337(data, cols, rows, false);
        if encoded.is_empty() {
            return self.iterm_placeholder(cols, rows);
        }

        self.container().w(cols).h(rows).draw(move |buf, rect| {
            if rect.width == 0 || rect.height == 0 {
                return;
            }
            let cells = (rect.width as usize).saturating_mul(rect.height as usize);
            buf.sprixel_place(crate::buffer::SprixelPlacement {
                content_hash,
                seq: encoded,
                x: rect.x,
                y: rect.y,
                cols: rect.width,
                rows: rect.height,
                cells: vec![crate::buffer::SprixelCell::Opaque; cells],
            });
        });
        Response::none()
    }

    /// Render an iTerm2 OSC 1337 inline image preserving aspect ratio.
    ///
    /// `data` is **encoded image-file bytes** (PNG/JPEG/GIF). The container is
    /// `cols` cells wide; height is reserved from the detected cell pixel
    /// dimensions (falling back to 8×16) and the OSC 1337 `height=auto` /
    /// `preserveAspectRatio=1` flags let the terminal scale to fit. Mirrors
    /// [`Context::kitty_image_fit`] (issue #265).
    ///
    /// Falls back to `[iterm2 unsupported]` on terminals without OSC 1337.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// let png = [0x89u8, b'P', b'N', b'G'];
    /// ui.iterm_image_fit(&png, 20);
    /// # });
    /// ```
    #[cfg(feature = "crossterm")]
    #[cfg_attr(docsrs, doc(cfg(feature = "crossterm")))]
    pub fn iterm_image_fit(&mut self, data: &[u8], cols: u32) -> Response {
        let supported =
            self.is_real_terminal && (self.capabilities.iterm2 || terminal_supports_iterm());

        // Reserve rows from the cell aspect using a 1:1 source assumption — the
        // terminal does the real aspect math from the decoded file; this only
        // sizes the cell box so layout below the image is not clobbered. Use a
        // square-ish default of `cols / 2` rows (cells are ~2:1 tall), min 1.
        let (cell_w, cell_h) = crate::terminal::cell_pixel_size();
        let rows = if cell_h == 0 {
            cols.max(1)
        } else {
            ((cols as f64 * cell_w as f64) / cell_h as f64)
                .ceil()
                .max(1.0) as u32
        };

        if !supported {
            return self.iterm_placeholder(cols, rows);
        }

        let content_hash = crate::buffer::hash_rgba(data);
        // `rows == 0` signals `height=auto`; the reserved cell box is `rows`.
        let encoded = crate::iterm::encode_iterm_osc1337(data, cols, 0, true);
        if encoded.is_empty() {
            return self.iterm_placeholder(cols, rows);
        }

        self.container().w(cols).h(rows).draw(move |buf, rect| {
            if rect.width == 0 || rect.height == 0 {
                return;
            }
            let cells = (rect.width as usize).saturating_mul(rect.height as usize);
            buf.sprixel_place(crate::buffer::SprixelPlacement {
                content_hash,
                seq: encoded,
                x: rect.x,
                y: rect.y,
                cols: rect.width,
                rows: rect.height,
                cells: vec![crate::buffer::SprixelCell::Opaque; cells],
            });
        });
        Response::none()
    }

    /// Reserve a `cols`×`rows` container and draw the unsupported placeholder,
    /// matching the Sixel fallback pattern.
    #[cfg(feature = "crossterm")]
    fn iterm_placeholder(&mut self, cols: u32, rows: u32) -> Response {
        self.container().w(cols).h(rows).draw(|buf, rect| {
            if rect.width == 0 || rect.height == 0 {
                return;
            }
            buf.set_string(rect.x, rect.y, "[iterm2 unsupported]", Style::new());
        });
        Response::none()
    }

    /// Render an image via iTerm2's OSC 1337 inline-image protocol.
    #[cfg(not(feature = "crossterm"))]
    pub fn iterm_image(&mut self, _data: &[u8], cols: u32, rows: u32) -> Response {
        self.container().w(cols).h(rows).draw(|buf, rect| {
            if rect.width == 0 || rect.height == 0 {
                return;
            }
            buf.set_string(rect.x, rect.y, "[iterm2 unsupported]", Style::new());
        });
        Response::none()
    }

    /// Render an iTerm2 OSC 1337 inline image preserving aspect ratio.
    #[cfg(not(feature = "crossterm"))]
    pub fn iterm_image_fit(&mut self, _data: &[u8], cols: u32) -> Response {
        // Without `crossterm` there is no cell-pixel probe; reserve a
        // square-ish box so layout below the image is stable.
        let rows = (cols / 2).max(1);
        self.container().w(cols).h(rows).draw(|buf, rect| {
            if rect.width == 0 || rect.height == 0 {
                return;
            }
            buf.set_string(rect.x, rect.y, "[iterm2 unsupported]", Style::new());
        });
        Response::none()
    }

    /// Render an image using the Sixel protocol.
    #[cfg(not(feature = "crossterm"))]
    pub fn sixel_image(
        &mut self,
        _rgba: &[u8],
        _pixel_width: u32,
        _pixel_height: u32,
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
            state.cursor_visible = (state.cursor_tick / 8).is_multiple_of(2);
        }

        if state.content.is_empty() && state.streaming {
            let cursor = if state.cursor_visible { "▌" } else { " " };
            let primary = self.theme.primary;
            self.text(cursor).fg(primary);
            return Response::none();
        }

        if !state.content.is_empty() {
            self.text(&state.content).wrap();
            if state.streaming && state.cursor_visible {
                let primary = self.theme.primary;
                self.styled("▌", Style::new().fg(primary));
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
            state.cursor_visible = (state.cursor_tick / 8).is_multiple_of(2);
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

        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
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
            })));
        self.skip_interaction_slot();

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

        if state.in_code_block != in_code_block {
            state.in_code_block = in_code_block;
        }
        if state.code_block_lang != code_block_lang {
            state.code_block_lang = code_block_lang;
        }

        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;
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
}
