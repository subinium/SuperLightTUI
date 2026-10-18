use super::*;
use crate::KeyMap;

/// Scoped builder for one text element.
///
/// Returned by [`Context::text`], [`Context::styled`], [`Context::link`] and
/// [`Context::timer_display`]. The element is recorded when the builder is
/// created and every chained method edits that one element in place, so a
/// chain renders exactly where it is written, in call order, and the builder
/// can be dropped at any point:
///
/// ```
/// # use slt::*;
/// # TestBackend::new(40, 6).render(|ui| {
/// ui.text("hello").bold().fg(Color::Cyan);
/// ui.text("a longer body that wraps").wrap().w(16).mt(1);
/// ui.text("Status").with_if(true, |t| {
///     t.fg(Color::Red);
/// });
/// if ui.text("hover me").show().hovered {
///     // show a hint
/// }
/// # });
/// ```
///
/// Only text options follow `text()`, so a modifier always styles the element
/// it visibly follows. Chaining another widget does not compile:
///
/// ```compile_fail
/// # slt::TestBackend::new(40, 4).render(|ui| {
/// let _ = ui.text("Name:").bold().button("Save");
/// # });
/// ```
///
/// The builder owns no heap data: `ui.text(s)` costs the same as before
/// v0.26, when it returned `&mut Context`. The `Context`-level modifiers
/// (`ui.bold()`, `ui.fg(..)`, ...) that styled "the last text" are deprecated
/// in its favour; see `docs/MIGRATION.md`.
pub struct Text<'a> {
    ctx: &'a mut Context,
    /// Response computed when the element was recorded, for widgets that
    /// already reserved their hit slot (`link`). Plain text reserves one
    /// lazily in [`Text::show`].
    response: Option<Response>,
}

impl<'a> Text<'a> {
    /// Wrap the text element that `ctx` recorded last.
    #[inline]
    fn new(ctx: &'a mut Context, response: Option<Response>) -> Self {
        Self { ctx, response }
    }

    /// Index of this builder's element. Holding `&mut Context` exclusively
    /// guarantees nothing is recorded after it while the builder lives.
    #[inline]
    fn idx(&self) -> usize {
        self.ctx.commands.len().saturating_sub(1)
    }

    #[inline]
    fn style(self, f: impl FnMut(&mut Style)) -> Self {
        let idx = self.idx();
        self.ctx.style_text_at(idx, f);
        self
    }

    /// Bold.
    #[inline]
    pub fn bold(self) -> Self {
        self.style(|s| s.modifiers |= Modifiers::BOLD)
    }

    /// Dim. Also switches an inherited foreground to the theme's `text_dim`
    /// color; an explicit [`fg`](Self::fg) is kept.
    #[inline]
    pub fn dim(self) -> Self {
        let idx = self.idx();
        self.ctx.dim_text_at(idx);
        self
    }

    /// Italic.
    #[inline]
    pub fn italic(self) -> Self {
        self.style(|s| s.modifiers |= Modifiers::ITALIC)
    }

    /// Underline.
    #[inline]
    pub fn underline(self) -> Self {
        self.style(|s| s.modifiers |= Modifiers::UNDERLINE)
    }

    /// Reverse video.
    #[inline]
    pub fn reversed(self) -> Self {
        self.style(|s| s.modifiers |= Modifiers::REVERSED)
    }

    /// Strikethrough.
    #[inline]
    pub fn strikethrough(self) -> Self {
        self.style(|s| s.modifiers |= Modifiers::STRIKETHROUGH)
    }

    /// Foreground color.
    #[inline]
    pub fn fg(self, color: Color) -> Self {
        self.style(|s| s.fg = Some(color))
    }

    /// Background color.
    #[inline]
    pub fn bg(self, color: Color) -> Self {
        self.style(|s| s.bg = Some(color))
    }

    /// Per-character foreground gradient from `from` to `to`.
    #[inline]
    pub fn gradient(self, from: Color, to: Color) -> Self {
        let idx = self.idx();
        self.ctx
            .apply_char_gradient(idx, false, |t| to.blend_f64(from, t));
        self
    }

    /// Per-character multi-stop foreground gradient.
    ///
    /// `stops` is a slice of `(position, color)` pairs where `position` lies
    /// in `0.0..=1.0`. Stops do not need to be pre-sorted. An empty slice is a
    /// no-op; a single stop produces a solid color.
    ///
    /// ```
    /// # use slt::*;
    /// # TestBackend::new(20, 2).render(|ui| {
    /// ui.text("rainbow").gradient_stops_f64(&[
    ///     (0.0, Color::Red),
    ///     (0.5, Color::Yellow),
    ///     (1.0, Color::Green),
    /// ]);
    /// # });
    /// ```
    #[inline]
    pub fn gradient_stops_f64(self, stops: &[(f64, Color)]) -> Self {
        let idx = self.idx();
        self.ctx.gradient_stops_at(idx, false, stops);
        self
    }

    /// Per-character background gradient from `from` to `to`.
    #[inline]
    pub fn bg_gradient(self, from: Color, to: Color) -> Self {
        let idx = self.idx();
        self.ctx
            .apply_char_gradient(idx, true, |t| to.blend_f64(from, t));
        self
    }

    /// Per-character multi-stop background gradient; stop handling matches
    /// [`gradient_stops_f64`](Self::gradient_stops_f64).
    #[inline]
    pub fn bg_gradient_stops_f64(self, stops: &[(f64, Color)]) -> Self {
        let idx = self.idx();
        self.ctx.gradient_stops_at(idx, true, stops);
        self
    }

    /// Foreground color applied while the enclosing
    /// [`group`](Context::group) is hovered or focused.
    #[inline]
    pub fn group_hover_fg(self, color: Color) -> Self {
        if self.ctx.group_style_active() {
            self.fg(color)
        } else {
            self
        }
    }

    /// Background color applied while the enclosing
    /// [`group`](Context::group) is hovered or focused.
    #[inline]
    pub fn group_hover_bg(self, color: Color) -> Self {
        if self.ctx.group_style_active() {
            self.bg(color)
        } else {
            self
        }
    }

    /// Wrap at word boundaries instead of overflowing.
    #[inline]
    pub fn wrap(self) -> Self {
        let idx = self.idx();
        self.ctx.wrap_text_at(idx);
        self
    }

    /// Truncate with `…` when the text exceeds its allocated width. Use with
    /// [`w`](Self::w), or let the parent container constrain it.
    #[inline]
    pub fn truncate(self) -> Self {
        let idx = self.idx();
        self.ctx.truncate_text_at(idx);
        self
    }

    /// Flex-grow factor; `1` fills the remaining main-axis space.
    #[inline]
    pub fn grow(self, value: u16) -> Self {
        let idx = self.idx();
        self.ctx.grow_text_at(idx, value);
        self
    }

    /// Horizontal alignment within the element's width.
    #[inline]
    pub fn align(self, align: Align) -> Self {
        let idx = self.idx();
        self.ctx.align_text_at(idx, align);
        self
    }

    /// Center horizontally; shorthand for `.align(Align::Center)`. Needs a
    /// width (via [`w`](Self::w) or the parent) to be visible.
    #[inline]
    pub fn text_center(self) -> Self {
        self.align(Align::Center)
    }

    /// Right-align; shorthand for `.align(Align::End)`.
    #[inline]
    pub fn text_right(self) -> Self {
        self.align(Align::End)
    }

    #[inline]
    fn constraints(self, f: impl FnOnce(&mut Constraints)) -> Self {
        let idx = self.idx();
        self.ctx.constraints_text_at(idx, f);
        self
    }

    #[inline]
    fn margin(self, f: impl FnOnce(&mut Margin)) -> Self {
        let idx = self.idx();
        self.ctx.margin_text_at(idx, f);
        self
    }

    /// Fixed width in columns (padded or truncated).
    #[inline]
    pub fn w(self, value: u32) -> Self {
        self.constraints(|c| *c = c.w(value))
    }

    /// Fixed height in rows.
    #[inline]
    pub fn h(self, value: u32) -> Self {
        self.constraints(|c| *c = c.h(value))
    }

    /// Minimum width in columns.
    #[inline]
    pub fn min_w(self, value: u32) -> Self {
        self.constraints(|c| c.set_min_width(Some(value)))
    }

    /// Maximum width in columns.
    #[inline]
    pub fn max_w(self, value: u32) -> Self {
        self.constraints(|c| c.set_max_width(Some(value)))
    }

    /// Minimum height in rows.
    #[inline]
    pub fn min_h(self, value: u32) -> Self {
        self.constraints(|c| c.set_min_height(Some(value)))
    }

    /// Maximum height in rows.
    #[inline]
    pub fn max_h(self, value: u32) -> Self {
        self.constraints(|c| c.set_max_height(Some(value)))
    }

    /// Uniform margin on all sides.
    #[inline]
    pub fn m(self, value: u32) -> Self {
        self.margin(|m| *m = Margin::all(value))
    }

    /// Horizontal margin (left and right).
    #[inline]
    pub fn mx(self, value: u32) -> Self {
        self.margin(|m| {
            m.left = value;
            m.right = value;
        })
    }

    /// Vertical margin (top and bottom).
    #[inline]
    pub fn my(self, value: u32) -> Self {
        self.margin(|m| {
            m.top = value;
            m.bottom = value;
        })
    }

    /// Top margin.
    #[inline]
    pub fn mt(self, value: u32) -> Self {
        self.margin(|m| m.top = value)
    }

    /// Right margin.
    #[inline]
    pub fn mr(self, value: u32) -> Self {
        self.margin(|m| m.right = value)
    }

    /// Bottom margin.
    #[inline]
    pub fn mb(self, value: u32) -> Self {
        self.margin(|m| m.bottom = value)
    }

    /// Left margin.
    #[inline]
    pub fn ml(self, value: u32) -> Self {
        self.margin(|m| m.left = value)
    }

    /// Apply `f` only if `cond` is true, keeping the chain intact.
    ///
    /// The closure gets a builder for the same element; it is skipped
    /// entirely when `cond` is `false`.
    ///
    /// ```
    /// # use slt::*;
    /// # TestBackend::new(20, 2).render(|ui| {
    /// let is_error = true;
    /// ui.text("Status")
    ///     .with_if(is_error, |t| {
    ///         t.bold().fg(Color::Red);
    ///     })
    ///     .with_if(!is_error, |t| {
    ///         t.dim();
    ///     });
    /// # });
    /// ```
    #[inline]
    pub fn with_if(self, cond: bool, f: impl FnOnce(Text<'_>)) -> Self {
        if cond {
            f(Text::new(&mut *self.ctx, None));
        }
        self
    }

    /// Apply `f` unconditionally; useful for factoring out a block of
    /// modifiers.
    #[inline]
    pub fn with(self, f: impl FnOnce(Text<'_>)) -> Self {
        self.with_if(true, f)
    }

    /// Finish the chain and return the element's [`Response`].
    ///
    /// Plain text reserves its hover / click hit area only when `show()` is
    /// called, so call it on the same element every frame to keep the slot
    /// stable. For [`Context::link`] the response is the link's own, with
    /// `clicked` set when it was activated by mouse or keyboard.
    ///
    /// ```
    /// # use slt::*;
    /// # TestBackend::new(20, 2).render(|ui| {
    /// let r = ui.text("hover me").fg(Color::Cyan).show();
    /// if r.hovered {
    ///     // ...
    /// }
    /// # });
    /// ```
    #[must_use = "show() only returns the Response; drop the builder instead if you do not need it"]
    #[inline]
    pub fn show(self) -> Response {
        if let Some(response) = self.response {
            return response;
        }
        let ctx = self.ctx;
        let Some(command) = ctx.commands.pop() else {
            return Response::none();
        };
        // The hit slot marker must precede the element it describes.
        let idx = ctx.commands.len();
        ctx.invalidate_geometry_cache_from(idx);
        let response = ctx.interaction();
        ctx.commands.push(command);
        if ctx.rollback.last_text_idx == Some(idx) {
            ctx.rollback.last_text_idx = Some(ctx.commands.len() - 1);
        }
        response
    }
}

impl Context {
    /// Render a text element and return its [`Text`] builder.
    ///
    /// Chain style, size and margin options on the result; it renders where
    /// it is written whether or not anything is chained.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// use slt::Color;
    /// ui.text("hello").bold().fg(Color::Cyan);
    /// # });
    /// ```
    pub fn text(&mut self, s: impl Into<String>) -> Text<'_> {
        let content = s.into();
        let default_fg = self.inherited_text_fg();
        self.commands.push(Command::Text {
            content,
            cursor_offset: None,
            cursor_masked: false,
            style: Style::new().fg(default_fg),
            grow: 0,
            align: Align::Start,
            wrap: false,
            truncate: false,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.rollback.last_text_idx = Some(self.commands.len() - 1);
        Text::new(self, None)
    }

    /// Render a clickable hyperlink and return its [`Text`] builder.
    ///
    /// The link is interactive: clicking it (or pressing Enter/Space when
    /// focused) opens the URL with the system opener (`open` on macOS,
    /// `xdg-open` on Linux, `rundll32 url.dll,FileProtocolHandler` on
    /// Windows; never through a shell). OSC 8 is also emitted for terminals
    /// that support native hyperlinks. [`Text::show`] returns the link's
    /// [`Response`]; `clicked` is set when it was activated.
    ///
    /// # Safety of link targets
    ///
    /// Only `http`, `https` and `mailto` URLs (case-insensitive) are opened.
    /// URLs with any other scheme (`file:`, `javascript:`, `data:`, custom
    /// protocol handlers, ...), with whitespace, control characters or `"`,
    /// or without a scheme are still rendered, but activating them only shows
    /// a warning toast; no process is spawned.
    ///
    /// When running over SSH or mosh (`SSH_CONNECTION`, `SSH_TTY` or
    /// `MOSH_IP` set), SLT does not spawn an opener at all, since it would
    /// open on the remote host. The OSC 8 hyperlink still lets the local
    /// terminal open the link.
    pub fn link(&mut self, text: impl Into<String>, url: impl Into<String>) -> Text<'_> {
        let url_str = url.into();
        let focused = self.register_focusable();
        let (_interaction_id, mut response) = self.begin_widget_interaction(focused);

        let activated = response.clicked || self.consume_activation_keys(focused);
        response.clicked = activated;

        // Never print here: writing to stderr would corrupt the alternate
        // screen. Surface the failure as a toast instead.
        if activated && let Err(e) = open_url(&url_str) {
            self.notify(&format!("Could not open link: {e}"), ToastLevel::Warning);
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
            wrap: false,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.rollback.last_text_idx = Some(self.commands.len() - 1);
        Text::new(self, Some(response))
    }

    /// Render an elapsed time display and return its [`Text`] builder.
    ///
    /// Formats as `HH:MM:SS.CC` when hours are non-zero, otherwise `MM:SS.CC`.
    pub fn timer_display(&mut self, elapsed: std::time::Duration) -> Text<'_> {
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
            cursor_offset: None,
            cursor_masked: false,
            style: Style::new().fg(self.theme.text),
            grow: 0,
            align: Align::Start,
            wrap: false,
            truncate: false,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.rollback.last_text_idx = Some(self.commands.len() - 1);
        Text::new(self, None)
    }

    /// Render help bar from a KeyMap. Shows visible bindings as key-description pairs.
    pub fn help_from_keymap(&mut self, keymap: &KeyMap) -> Response {
        let pairs: Vec<(&str, &str)> = keymap
            .visible_bindings()
            .map(|binding| (binding.display.as_str(), binding.description.as_str()))
            .collect();
        self.help(&pairs)
    }

    /// Render a text element with an explicit [`Style`] and return its
    /// [`Text`] builder.
    ///
    /// Equivalent to `text(s)` followed by style-chain methods, but more
    /// concise when you already have a `Style` value.
    pub fn styled(&mut self, s: impl Into<String>, style: Style) -> Text<'_> {
        self.styled_with_cursor(s, style, None);
        Text::new(self, None)
    }

    pub(crate) fn styled_with_cursor(
        &mut self,
        s: impl Into<String>,
        style: Style,
        cursor_offset: Option<usize>,
    ) -> &mut Self {
        self.styled_with_cursor_privacy(s, style, cursor_offset, false)
    }

    pub(crate) fn styled_with_cursor_privacy(
        &mut self,
        s: impl Into<String>,
        style: Style,
        cursor_offset: Option<usize>,
        cursor_masked: bool,
    ) -> &mut Self {
        self.commands.push(Command::Text {
            content: s.into(),
            cursor_offset,
            cursor_masked,
            style,
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

    /// Render an invisible spacer that expands to fill available space.
    ///
    /// Useful for pushing siblings to opposite ends of a row or column.
    pub fn spacer(&mut self) -> &mut Self {
        self.commands.push(Command::Spacer { grow: 1 });
        self.rollback.last_text_idx = None;
        self
    }

    fn inherited_text_fg(&self) -> Color {
        self.rollback
            .text_color_stack
            .iter()
            .rev()
            .find_map(|color| *color)
            .unwrap_or(self.theme.text)
    }

    // ── element editors shared by `Text` and the deprecated modifiers ──

    fn style_text_at(&mut self, idx: usize, mut f: impl FnMut(&mut Style)) {
        match self.commands.get_mut(idx) {
            Some(Command::Text { style, .. } | Command::Link { style, .. }) => f(style),
            Some(Command::RichText { segments, .. }) => {
                for (_, style) in segments {
                    f(style);
                }
            }
            _ => {}
        }
    }

    fn dim_text_at(&mut self, idx: usize) {
        let text_dim = self.theme.text_dim;
        let inherited_fg = self.inherited_text_fg();
        match self.commands.get_mut(idx) {
            Some(Command::Text { style, .. }) => {
                style.modifiers |= Modifiers::DIM;
                if style.fg.is_none() || style.fg == Some(inherited_fg) {
                    style.fg = Some(text_dim);
                }
            }
            Some(Command::Link { style, .. }) => {
                style.modifiers |= Modifiers::DIM;
            }
            Some(Command::RichText { segments, .. }) => {
                let all_inherited = segments
                    .iter()
                    .all(|(_, style)| style.fg.is_none() || style.fg == Some(inherited_fg));
                for (_, style) in segments {
                    style.modifiers |= Modifiers::DIM;
                    if all_inherited {
                        style.fg = Some(text_dim);
                    }
                }
            }
            _ => {}
        }
    }

    fn wrap_text_at(&mut self, idx: usize) {
        if let Some(
            Command::Text { wrap, .. }
            | Command::Link { wrap, .. }
            | Command::RichText { wrap, .. },
        ) = self.commands.get_mut(idx)
        {
            *wrap = true;
        }
    }

    fn truncate_text_at(&mut self, idx: usize) {
        if let Some(Command::Text { truncate, .. }) = self.commands.get_mut(idx) {
            *truncate = true;
        }
    }

    fn grow_text_at(&mut self, idx: usize, value: u16) {
        if let Some(Command::Text { grow, .. }) = self.commands.get_mut(idx) {
            *grow = value;
        }
    }

    fn align_text_at(&mut self, idx: usize, align: Align) {
        if let Some(
            Command::Text {
                align: text_align, ..
            }
            | Command::RichText {
                align: text_align, ..
            },
        ) = self.commands.get_mut(idx)
        {
            *text_align = align;
        }
    }

    fn constraints_text_at(&mut self, idx: usize, f: impl FnOnce(&mut Constraints)) {
        if let Some(
            Command::Text { constraints, .. }
            | Command::Link { constraints, .. }
            | Command::RichText { constraints, .. },
        ) = self.commands.get_mut(idx)
        {
            f(constraints);
        }
    }

    fn margin_text_at(&mut self, idx: usize, f: impl FnOnce(&mut Margin)) {
        if let Some(
            Command::Text { margin, .. }
            | Command::Link { margin, .. }
            | Command::RichText { margin, .. },
        ) = self.commands.get_mut(idx)
        {
            f(margin);
        }
    }

    /// Whether the innermost [`group`](Context::group) is hovered or focused.
    fn group_style_active(&self) -> bool {
        self.rollback
            .group_stack
            .last()
            .map(|name| self.is_group_hovered(name) || self.is_group_focused(name))
            .unwrap_or(false)
    }

    fn gradient_stops_at(&mut self, idx: usize, is_bg: bool, stops: &[(f64, Color)]) {
        if stops.is_empty() {
            return;
        }
        let sorted = Self::sorted_gradient_stops(stops);
        self.apply_char_gradient(idx, is_bg, |t| Self::sample_gradient_stops(&sorted, t));
    }

    /// Return `stops` sorted ascending by clamped position. Positions are
    /// clamped into `0.0..=1.0` so out-of-range inputs degrade gracefully.
    fn sorted_gradient_stops(stops: &[(f64, Color)]) -> Vec<(f64, Color)> {
        let mut sorted: Vec<(f64, Color)> = stops
            .iter()
            .map(|(pos, color)| {
                let pos = if pos.is_finite() {
                    pos.clamp(0.0, 1.0)
                } else {
                    0.0
                };
                (pos, *color)
            })
            .collect();
        sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
        sorted
    }

    /// Sample the color at position `t` (in `0.0..=1.0`) from pre-sorted,
    /// non-empty `stops`, linearly interpolating between the bracketing stops.
    fn sample_gradient_stops(stops: &[(f64, Color)], t: f64) -> Color {
        let t = if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        };
        // Non-empty is guaranteed by callers; fall back defensively otherwise.
        let first = match stops.first() {
            Some(stop) => *stop,
            None => return Color::Rgb(0, 0, 0),
        };
        let last = *stops.last().unwrap_or(&first);
        if t <= first.0 {
            return first.1;
        }
        if t >= last.0 {
            return last.1;
        }
        for window in stops.windows(2) {
            let (p0, c0) = window[0];
            let (p1, c1) = window[1];
            if t >= p0 && t <= p1 {
                let span = p1 - p0;
                if span <= f64::EPSILON {
                    return c1;
                }
                let local = (t - p0) / span;
                return c1.blend_f64(c0, local);
            }
        }
        last.1
    }

    /// Replace the `Text` command at `idx` with a `RichText` gradient, mapping
    /// each grapheme's starting cell to a position in `0.0..=1.0`. `is_bg`
    /// selects background vs foreground.
    fn apply_char_gradient(&mut self, idx: usize, is_bg: bool, color_at: impl Fn(f64) -> Color) {
        let replacement = match self.commands.get(idx) {
            Some(Command::Text {
                content,
                style,
                wrap,
                align,
                margin,
                constraints,
                ..
            }) => {
                let graphemes: Vec<&str> = content.graphemes(true).collect();
                let last_start = graphemes
                    .iter()
                    .take(graphemes.len().saturating_sub(1))
                    .map(|grapheme| crate::width::str_width(grapheme))
                    .sum::<usize>();
                let denom = last_start.max(1) as f64;
                let mut cell = 0usize;
                let segments = graphemes
                    .into_iter()
                    .map(|grapheme| {
                        let mut seg_style = *style;
                        let color = color_at(cell as f64 / denom);
                        if is_bg {
                            seg_style.bg = Some(color);
                        } else {
                            seg_style.fg = Some(color);
                        }
                        cell = cell.saturating_add(crate::width::str_width(grapheme));
                        (grapheme.to_string(), seg_style)
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
}

/// Deprecated "modify the last text" modifiers (issue #457).
///
/// Each one keeps its v0.25 behaviour for one release: it edits the most
/// recent `text` / `styled` / `link` / `timer_display` / separator element,
/// and silently does nothing after any other widget. Chain the same method on
/// the [`Text`] builder instead.
impl Context {
    fn last_text(&self) -> Option<usize> {
        self.rollback.last_text_idx
    }

    /// Apply bold to the last rendered text element.
    ///
    /// Deprecated with the other "last text" modifiers; using one warns:
    ///
    /// ```compile_fail
    /// #![deny(deprecated)]
    /// # slt::TestBackend::new(20, 2).render(|ui| {
    /// ui.text("x");
    /// ui.bold();
    /// # });
    /// ```
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).bold()`"
    )]
    pub fn bold(&mut self) -> &mut Self {
        if let Some(idx) = self.last_text() {
            self.style_text_at(idx, |s| s.modifiers |= Modifiers::BOLD);
        }
        self
    }

    /// Apply dim styling to the last rendered text element.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).dim()`"
    )]
    pub fn dim(&mut self) -> &mut Self {
        if let Some(idx) = self.last_text() {
            self.dim_text_at(idx);
        }
        self
    }

    /// Apply italic to the last rendered text element.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).italic()`"
    )]
    pub fn italic(&mut self) -> &mut Self {
        if let Some(idx) = self.last_text() {
            self.style_text_at(idx, |s| s.modifiers |= Modifiers::ITALIC);
        }
        self
    }

    /// Apply underline to the last rendered text element.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).underline()`"
    )]
    pub fn underline(&mut self) -> &mut Self {
        if let Some(idx) = self.last_text() {
            self.style_text_at(idx, |s| s.modifiers |= Modifiers::UNDERLINE);
        }
        self
    }

    /// Apply reverse-video to the last rendered text element.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).reversed()`"
    )]
    pub fn reversed(&mut self) -> &mut Self {
        if let Some(idx) = self.last_text() {
            self.style_text_at(idx, |s| s.modifiers |= Modifiers::REVERSED);
        }
        self
    }

    /// Apply strikethrough to the last rendered text element.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).strikethrough()`"
    )]
    pub fn strikethrough(&mut self) -> &mut Self {
        if let Some(idx) = self.last_text() {
            self.style_text_at(idx, |s| s.modifiers |= Modifiers::STRIKETHROUGH);
        }
        self
    }

    /// Set the foreground color of the last rendered text element.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).fg(color)`"
    )]
    pub fn fg(&mut self, color: Color) -> &mut Self {
        if let Some(idx) = self.last_text() {
            self.style_text_at(idx, |s| s.fg = Some(color));
        }
        self
    }

    /// Set the background color of the last rendered text element.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).bg(color)`"
    )]
    pub fn bg(&mut self, color: Color) -> &mut Self {
        if let Some(idx) = self.last_text() {
            self.style_text_at(idx, |s| s.bg = Some(color));
        }
        self
    }

    /// Apply a per-character foreground gradient to the last rendered text.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).gradient(from, to)`"
    )]
    pub fn gradient(&mut self, from: Color, to: Color) -> &mut Self {
        if let Some(idx) = self.last_text() {
            self.apply_char_gradient(idx, false, |t| to.blend_f64(from, t));
        }
        self
    }

    /// Apply a per-character multi-stop foreground gradient to the last text.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).gradient_stops_f64(stops)`"
    )]
    pub fn gradient_stops_f64(&mut self, stops: &[(f64, Color)]) -> &mut Self {
        if let Some(idx) = self.last_text() {
            self.gradient_stops_at(idx, false, stops);
        }
        self
    }

    /// Deprecated `f32` alias of the multi-stop foreground gradient.
    #[deprecated(since = "0.22.2", note = "use `ui.text(..).gradient_stops_f64(stops)`")]
    pub fn gradient_stops(&mut self, stops: &[(f32, Color)]) -> &mut Self {
        let stops: Vec<(f64, Color)> = stops
            .iter()
            .map(|(pos, color)| (f64::from(*pos), *color))
            .collect();
        if let Some(idx) = self.last_text() {
            self.gradient_stops_at(idx, false, &stops);
        }
        self
    }

    /// Apply a per-character background gradient to the last rendered text.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).bg_gradient(from, to)`"
    )]
    pub fn bg_gradient(&mut self, from: Color, to: Color) -> &mut Self {
        if let Some(idx) = self.last_text() {
            self.apply_char_gradient(idx, true, |t| to.blend_f64(from, t));
        }
        self
    }

    /// Apply a per-character multi-stop background gradient to the last text.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).bg_gradient_stops_f64(stops)`"
    )]
    pub fn bg_gradient_stops_f64(&mut self, stops: &[(f64, Color)]) -> &mut Self {
        if let Some(idx) = self.last_text() {
            self.gradient_stops_at(idx, true, stops);
        }
        self
    }

    /// Deprecated `f32` alias of the multi-stop background gradient.
    #[deprecated(
        since = "0.22.2",
        note = "use `ui.text(..).bg_gradient_stops_f64(stops)`"
    )]
    pub fn bg_gradient_stops(&mut self, stops: &[(f32, Color)]) -> &mut Self {
        let stops: Vec<(f64, Color)> = stops
            .iter()
            .map(|(pos, color)| (f64::from(*pos), *color))
            .collect();
        if let Some(idx) = self.last_text() {
            self.gradient_stops_at(idx, true, &stops);
        }
        self
    }

    /// Set foreground color when the current group is hovered or focused.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).group_hover_fg(color)`"
    )]
    pub fn group_hover_fg(&mut self, color: Color) -> &mut Self {
        if self.group_style_active()
            && let Some(idx) = self.last_text()
        {
            self.style_text_at(idx, |s| s.fg = Some(color));
        }
        self
    }

    /// Set background color when the current group is hovered or focused.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).group_hover_bg(color)`"
    )]
    pub fn group_hover_bg(&mut self, color: Color) -> &mut Self {
        if self.group_style_active()
            && let Some(idx) = self.last_text()
        {
            self.style_text_at(idx, |s| s.bg = Some(color));
        }
        self
    }

    /// Enable word-boundary wrapping on the last rendered text element.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).wrap()`"
    )]
    pub fn wrap(&mut self) -> &mut Self {
        if let Some(idx) = self.last_text() {
            self.wrap_text_at(idx);
        }
        self
    }

    /// Truncate the last rendered text with `…` when it exceeds its width.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).truncate()`"
    )]
    pub fn truncate(&mut self) -> &mut Self {
        if let Some(idx) = self.last_text() {
            self.truncate_text_at(idx);
        }
        self
    }

    /// Set the flex-grow factor of the last rendered text element.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).grow(n)`"
    )]
    pub fn grow(&mut self, value: u16) -> &mut Self {
        if let Some(idx) = self.last_text() {
            self.grow_text_at(idx, value);
        }
        self
    }

    /// Set the text alignment of the last rendered text element.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).align(a)`"
    )]
    pub fn align(&mut self, align: Align) -> &mut Self {
        if let Some(idx) = self.last_text() {
            self.align_text_at(idx, align);
        }
        self
    }

    /// Center-align the last rendered text element horizontally.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).text_center()`"
    )]
    pub fn text_center(&mut self) -> &mut Self {
        if let Some(idx) = self.last_text() {
            self.align_text_at(idx, Align::Center);
        }
        self
    }

    /// Right-align the last rendered text element horizontally.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).text_right()`"
    )]
    pub fn text_right(&mut self) -> &mut Self {
        if let Some(idx) = self.last_text() {
            self.align_text_at(idx, Align::End);
        }
        self
    }

    fn last_constraints(&mut self, f: impl FnOnce(&mut Constraints)) -> &mut Self {
        if let Some(idx) = self.last_text() {
            self.constraints_text_at(idx, f);
        }
        self
    }

    fn last_margin(&mut self, f: impl FnOnce(&mut Margin)) -> &mut Self {
        if let Some(idx) = self.last_text() {
            self.margin_text_at(idx, f);
        }
        self
    }

    /// Set a fixed width on the last rendered text or link element.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).w(n)`"
    )]
    pub fn w(&mut self, value: u32) -> &mut Self {
        self.last_constraints(|c| *c = c.w(value))
    }

    /// Set a fixed height on the last rendered text or link element.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).h(n)`"
    )]
    pub fn h(&mut self, value: u32) -> &mut Self {
        self.last_constraints(|c| *c = c.h(value))
    }

    /// Set the minimum width on the last rendered text or link element.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).min_w(n)`"
    )]
    pub fn min_w(&mut self, value: u32) -> &mut Self {
        self.last_constraints(|c| c.set_min_width(Some(value)))
    }

    /// Set the maximum width on the last rendered text or link element.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).max_w(n)`"
    )]
    pub fn max_w(&mut self, value: u32) -> &mut Self {
        self.last_constraints(|c| c.set_max_width(Some(value)))
    }

    /// Set the minimum height on the last rendered text or link element.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).min_h(n)`"
    )]
    pub fn min_h(&mut self, value: u32) -> &mut Self {
        self.last_constraints(|c| c.set_min_height(Some(value)))
    }

    /// Set the maximum height on the last rendered text or link element.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).max_h(n)`"
    )]
    pub fn max_h(&mut self, value: u32) -> &mut Self {
        self.last_constraints(|c| c.set_max_height(Some(value)))
    }

    /// Set uniform margin on all sides of the last rendered text or link.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).m(n)`"
    )]
    pub fn m(&mut self, value: u32) -> &mut Self {
        self.last_margin(|m| *m = Margin::all(value))
    }

    /// Set horizontal margin (left + right) on the last rendered text or link.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).mx(n)`"
    )]
    pub fn mx(&mut self, value: u32) -> &mut Self {
        self.last_margin(|m| {
            m.left = value;
            m.right = value;
        })
    }

    /// Set vertical margin (top + bottom) on the last rendered text or link.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).my(n)`"
    )]
    pub fn my(&mut self, value: u32) -> &mut Self {
        self.last_margin(|m| {
            m.top = value;
            m.bottom = value;
        })
    }

    /// Set top margin on the last rendered text or link element.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).mt(n)`"
    )]
    pub fn mt(&mut self, value: u32) -> &mut Self {
        self.last_margin(|m| m.top = value)
    }

    /// Set right margin on the last rendered text or link element.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).mr(n)`"
    )]
    pub fn mr(&mut self, value: u32) -> &mut Self {
        self.last_margin(|m| m.right = value)
    }

    /// Set bottom margin on the last rendered text or link element.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).mb(n)`"
    )]
    pub fn mb(&mut self, value: u32) -> &mut Self {
        self.last_margin(|m| m.bottom = value)
    }

    /// Set left margin on the last rendered text or link element.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).ml(n)`"
    )]
    pub fn ml(&mut self, value: u32) -> &mut Self {
        self.last_margin(|m| m.left = value)
    }

    /// Apply `f` only if `cond` is true. Returns `self` so chaining continues.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).with_if(cond, |t| { t.bold(); })`"
    )]
    pub fn with_if(&mut self, cond: bool, f: impl FnOnce(&mut Self)) -> &mut Self {
        if cond {
            f(self);
        }
        self
    }

    /// Apply `f` unconditionally.
    #[deprecated(
        since = "0.26.0",
        note = "chain on the Text returned by text(): `ui.text(..).with(|t| { t.bold(); })`"
    )]
    pub fn with(&mut self, f: impl FnOnce(&mut Self)) -> &mut Self {
        f(self);
        self
    }
}

#[cfg(test)]
mod gradient_tests {
    use super::*;
    use crate::TestBackend;

    #[test]
    fn gradient_stops_interpolates_fg_across_columns() {
        let red = Color::Rgb(255, 0, 0);
        let blue = Color::Rgb(0, 0, 255);
        let mut backend = TestBackend::new(20, 4);
        backend.render(|ui| {
            ui.text("ABC")
                .gradient_stops_f64(&[(0.0, red), (1.0, blue)]);
        });

        let buf = backend.buffer();
        // i=0 → t=0 → stop at 0.0 (red); i=2 → t=1 → stop at 1.0 (blue);
        // i=1 → t=0.5 → halfway blend.
        assert_eq!(
            buf.get(0, 0).style.fg,
            Some(red),
            "first column should be red"
        );
        assert_eq!(
            buf.get(1, 0).style.fg,
            Some(Color::Rgb(128, 0, 128)),
            "middle column should be the halfway blend"
        );
        assert_eq!(
            buf.get(2, 0).style.fg,
            Some(blue),
            "last column should be blue"
        );
    }

    #[test]
    fn two_stop_gradient_uses_documented_endpoint_order() {
        let red = Color::Rgb(255, 0, 0);
        let blue = Color::Rgb(0, 0, 255);
        let mut backend = TestBackend::new(20, 4);
        backend.render(|ui| {
            ui.text("ABC").gradient(red, blue);
        });

        let buf = backend.buffer();
        assert_eq!(buf.get(0, 0).style.fg, Some(red));
        assert_eq!(buf.get(1, 0).style.fg, Some(Color::Rgb(128, 0, 128)));
        assert_eq!(buf.get(2, 0).style.fg, Some(blue));
    }

    #[test]
    fn gradient_stops_unsorted_input_is_sorted() {
        let red = Color::Rgb(255, 0, 0);
        let blue = Color::Rgb(0, 0, 255);
        let mut backend = TestBackend::new(20, 4);
        backend.render(|ui| {
            // Deliberately out of order — must behave identically to sorted.
            ui.text("ABC")
                .gradient_stops_f64(&[(1.0, blue), (0.0, red)]);
        });

        let buf = backend.buffer();
        assert_eq!(buf.get(0, 0).style.fg, Some(red));
        assert_eq!(buf.get(2, 0).style.fg, Some(blue));
    }

    #[test]
    fn gradient_stops_multi_stop_hits_middle_stop_exactly() {
        let red = Color::Rgb(255, 0, 0);
        let green = Color::Rgb(0, 255, 0);
        let blue = Color::Rgb(0, 0, 255);
        let mut backend = TestBackend::new(20, 4);
        backend.render(|ui| {
            // len=3, denom=2 → columns map to t = 0.0, 0.5, 1.0.
            ui.text("ABC")
                .gradient_stops_f64(&[(0.0, red), (0.5, green), (1.0, blue)]);
        });

        let buf = backend.buffer();
        assert_eq!(buf.get(0, 0).style.fg, Some(red), "t=0 → first stop");
        assert_eq!(
            buf.get(1, 0).style.fg,
            Some(green),
            "t=0.5 → middle stop exactly"
        );
        assert_eq!(buf.get(2, 0).style.fg, Some(blue), "t=1 → last stop");
    }

    #[test]
    fn gradient_stops_single_stop_is_solid() {
        let cyan = Color::Rgb(0, 200, 200);
        let mut backend = TestBackend::new(20, 4);
        backend.render(|ui| {
            ui.text("ABCD").gradient_stops_f64(&[(0.0, cyan)]);
        });

        let buf = backend.buffer();
        for x in 0..4 {
            assert_eq!(
                buf.get(x, 0).style.fg,
                Some(cyan),
                "every column should be the single solid stop"
            );
        }
    }

    #[test]
    fn gradient_stops_empty_is_noop() {
        let mut backend = TestBackend::new(20, 4);
        backend.render(|ui| {
            // Empty slice must not panic and must leave content intact.
            ui.text("HELLO").gradient_stops_f64(&[]);
        });

        backend.assert_contains("HELLO");
    }

    #[test]
    fn bg_gradient_applies_to_background() {
        let red = Color::Rgb(255, 0, 0);
        let blue = Color::Rgb(0, 0, 255);
        let mut backend = TestBackend::new(20, 4);
        backend.render(|ui| {
            ui.text("ABC").bg_gradient(red, blue);
        });

        let buf = backend.buffer();
        assert_eq!(buf.get(0, 0).style.bg, Some(red), "first column bg = from");
        assert_eq!(buf.get(2, 0).style.bg, Some(blue), "last column bg = to");
        assert_eq!(
            buf.get(1, 0).style.bg,
            Some(Color::Rgb(128, 0, 128)),
            "middle column bg = halfway blend"
        );
    }

    #[test]
    fn bg_gradient_stops_interpolates_background() {
        let red = Color::Rgb(255, 0, 0);
        let blue = Color::Rgb(0, 0, 255);
        let mut backend = TestBackend::new(20, 4);
        backend.render(|ui| {
            ui.text("ABC")
                .bg_gradient_stops_f64(&[(0.0, red), (1.0, blue)]);
        });

        let buf = backend.buffer();
        assert_eq!(buf.get(0, 0).style.bg, Some(red), "first column bg = red");
        assert_eq!(
            buf.get(1, 0).style.bg,
            Some(Color::Rgb(128, 0, 128)),
            "middle column bg = halfway blend"
        );
        assert_eq!(buf.get(2, 0).style.bg, Some(blue), "last column bg = blue");
    }

    #[test]
    fn bg_gradient_stops_empty_is_noop() {
        let mut backend = TestBackend::new(20, 4);
        backend.render(|ui| {
            ui.text("WORLD").bg_gradient_stops_f64(&[]);
        });

        backend.assert_contains("WORLD");
    }

    #[test]
    fn style_and_constraints_after_gradient_update_rich_text() {
        let mut backend = TestBackend::new(20, 4);
        backend.render(|ui| {
            ui.text("ABC")
                .gradient(Color::Red, Color::Blue)
                .bold()
                .fg(Color::Green)
                .bg(Color::Black)
                .w(8)
                .m(1)
                .align(Align::End);
        });

        let buf = backend.buffer();
        let (start_x, y) = backend.find_text("ABC").expect("rendered gradient text");
        for x in start_x..start_x + 3 {
            let cell = buf.get(x, y);
            assert_eq!(cell.style.fg, Some(Color::Green));
            assert_eq!(cell.style.bg, Some(Color::Black));
            assert!(cell.style.modifiers.contains(Modifiers::BOLD));
        }
    }

    #[test]
    fn dim_uses_theme_color_only_for_inherited_foreground() {
        let theme = Theme::dark();
        let mut backend = TestBackend::new(20, 4);
        backend.render(|ui| {
            ui.set_theme(theme);
            ui.text("inherited").dim();
            ui.text("explicit").fg(Color::Red).dim();
        });

        let buf = backend.buffer();
        assert_eq!(buf.get(0, 0).style.fg, Some(theme.text_dim));
        assert_eq!(buf.get(0, 1).style.fg, Some(Color::Red));
        assert!(buf.get(0, 0).style.modifiers.contains(Modifiers::DIM));
        assert!(buf.get(0, 1).style.modifiers.contains(Modifiers::DIM));
    }

    #[test]
    fn gradient_positions_follow_grapheme_cell_width() {
        let mut backend = TestBackend::new(20, 4);
        backend.render(|ui| {
            ui.text("界A").gradient(Color::Red, Color::Blue);
        });

        let buf = backend.buffer();
        assert_eq!(buf.get(0, 0).style.fg, Some(Color::Red));
        assert_eq!(buf.get(2, 0).style.fg, Some(Color::Blue));
    }
}
