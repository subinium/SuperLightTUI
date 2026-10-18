//! Scoped builders for the colorable status widgets (#460): [`Badge`],
//! [`Stat`] and [`Help`].
//!
//! Like [`Text`](crate::Text), each widget is recorded when its builder is
//! created and its [`Response`] is final; `.color(..)` / `.key_color(..)` /
//! `.text_color(..)` restyle the recorded text in place. The builders replace
//! the 0.25 `badge_colored` / `stat_colored` / `help_colored` variants and
//! dereference to their `Response`, so `ui.badge("x").hovered` and
//! `ui.badge("x").on_hover(ui, ..)` keep working.

use super::*;

impl Context {
    /// Render a help bar showing keybinding hints.
    ///
    /// `bindings` is a slice of `(key, action)` pairs. Keys are rendered bold
    /// in the theme's primary color; actions and the `·` separators in the
    /// dim text color. Chain [`key_color`](Help::key_color) /
    /// [`text_color`](Help::text_color) to override them.
    ///
    /// ```
    /// # use slt::*;
    /// # TestBackend::new(40, 2).render(|ui| {
    /// ui.help(&[("q", "quit"), ("?", "help")]);
    /// ui.help(&[("q", "quit")]).key_color(Color::Yellow).text_color(Color::White);
    /// # });
    /// ```
    pub fn help(&mut self, bindings: &[(&str, &str)]) -> Help<'_> {
        if bindings.is_empty() {
            return Help {
                ctx: self,
                response: Response::none(),
                first: 0,
                len: 0,
            };
        }

        self.skip_interaction_slot();
        let help_gap = self.theme.spacing.sm();
        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                direction: Direction::Row,
                gap: help_gap as i32,
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
        let first = self.commands.len();
        let key_style = Style::new().bold().fg(self.theme.primary);
        let text_style = Style::new().fg(self.theme.text_dim);
        for (idx, (key, action)) in bindings.iter().enumerate() {
            if idx > 0 {
                self.styled("·", text_style);
            }
            self.styled(*key, key_style);
            self.styled(*action, text_style);
        }
        let len = self.commands.len() - first;
        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;

        Help {
            ctx: self,
            response: Response::none(),
            first,
            len,
        }
    }

    /// Render a help bar with custom key/description colors.
    #[deprecated(
        since = "0.26.0",
        note = "use `ui.help(bindings).key_color(key).text_color(text)`"
    )]
    pub fn help_colored(
        &mut self,
        bindings: &[(&str, &str)],
        key_color: Color,
        text_color: Color,
    ) -> Response {
        self.help(bindings)
            .key_color(key_color)
            .text_color(text_color)
            .show()
    }

    /// Render a small badge (pill) with the theme's primary background.
    ///
    /// The foreground is auto-selected for contrast via
    /// [`Color::contrast_fg`]. Chain [`color`](Badge::color) for another
    /// background. The [`Response`] carries real `hovered` / `right_clicked`
    /// state for the badge's rect, so callers can attach `.on_hover(...)`
    /// tooltips.
    ///
    /// ```
    /// # use slt::*;
    /// # TestBackend::new(40, 2).render(|ui| {
    /// ui.badge("NEW");
    /// let r = ui.badge("ALPHA").color(Color::Magenta).show();
    /// if r.hovered { /* attach a tooltip */ }
    /// # });
    /// ```
    pub fn badge(&mut self, label: &str) -> Badge<'_> {
        let color = self.theme.primary;
        let mut label_text = String::with_capacity(label.len() + 2);
        label_text.push(' ');
        label_text.push_str(label);
        label_text.push(' ');
        // Reserve the interaction slot *before* the text so the marker
        // attaches to the badge's rect (same pattern as `spinner` / `gauge`).
        let response = self.interaction();
        self.text(label_text)
            .fg(Color::contrast_fg(color))
            .bg(color);
        let idx = self.commands.len() - 1;
        Badge {
            ctx: self,
            response,
            idx,
        }
    }

    /// Render a badge with a custom background color.
    #[deprecated(since = "0.26.0", note = "use `ui.badge(label).color(color)`")]
    pub fn badge_colored(&mut self, label: &str, color: Color) -> Response {
        self.badge(label).color(color).show()
    }

    /// Render a label-value stat pair.
    ///
    /// Renders as a column: a dim label above a bold value. Chain
    /// [`color`](Stat::color) to color the value. Pair multiple stats in a
    /// [`row`](Self::row) for a compact dashboard strip. The [`Response`]
    /// carries real `hovered` / `clicked` / `right_clicked` state for the
    /// stat's column rect.
    ///
    /// ```
    /// # use slt::*;
    /// # TestBackend::new(40, 4).render(|ui| {
    /// ui.row(|ui| {
    ///     let r = ui.stat("Users", "1.2k").show();
    ///     if r.hovered { /* attach a tooltip */ }
    ///     ui.stat("Errors", "0").color(Color::Green);
    /// });
    /// # });
    /// ```
    pub fn stat(&mut self, label: &str, value: &str) -> Stat<'_> {
        let mut value_idx = 0;
        let response = self.col(|ui| {
            ui.text(label).dim();
            ui.text(value).bold();
            value_idx = ui.commands.len() - 1;
        });
        Stat {
            ctx: self,
            response,
            value_idx,
        }
    }

    /// Render a stat pair with a custom value color.
    #[deprecated(since = "0.26.0", note = "use `ui.stat(label, value).color(color)`")]
    pub fn stat_colored(&mut self, label: &str, value: &str, color: Color) -> Response {
        self.stat(label, value).color(color).show()
    }
}

/// Scoped builder for one help bar, returned by [`Context::help`].
///
/// Already recorded when created; the color methods restyle it in place.
/// Dereferences to its [`Response`]; [`show`](Self::show) returns it.
pub struct Help<'a> {
    ctx: &'a mut Context,
    response: Response,
    /// Index of the first text command and how many follow it.
    first: usize,
    len: usize,
}

impl Help<'_> {
    /// Color of the key labels (kept bold).
    #[inline]
    pub fn key_color(self, color: Color) -> Self {
        self.restyle(true, color)
    }

    /// Color of the action descriptions and the `·` separators.
    #[inline]
    pub fn text_color(self, color: Color) -> Self {
        self.restyle(false, color)
    }

    /// Return the help bar's [`Response`].
    #[inline]
    pub fn show(self) -> Response {
        self.response
    }

    fn restyle(self, keys: bool, color: Color) -> Self {
        let end = self.first + self.len;
        for (pos, idx) in (self.first..end).enumerate() {
            // Sequence: key, action, then (·, key, action) per extra binding.
            let is_key = pos == 0 || (pos >= 2 && (pos - 2) % 3 == 1);
            if is_key == keys
                && let Some(Command::Text { style, .. }) = self.ctx.commands.get_mut(idx)
            {
                style.fg = Some(color);
            }
        }
        self
    }
}

impl std::ops::Deref for Help<'_> {
    type Target = Response;

    #[inline]
    fn deref(&self) -> &Response {
        &self.response
    }
}

/// Scoped builder for one badge, returned by [`Context::badge`].
///
/// Already recorded when created; [`color`](Self::color) restyles it in
/// place. Dereferences to its [`Response`]; [`show`](Self::show) returns it.
pub struct Badge<'a> {
    ctx: &'a mut Context,
    response: Response,
    idx: usize,
}

impl Badge<'_> {
    /// Background color; the foreground follows [`Color::contrast_fg`].
    #[inline]
    pub fn color(self, color: Color) -> Self {
        if let Some(Command::Text { style, .. }) = self.ctx.commands.get_mut(self.idx) {
            style.fg = Some(Color::contrast_fg(color));
            style.bg = Some(color);
        }
        self
    }

    /// Return the badge's [`Response`].
    #[inline]
    pub fn show(self) -> Response {
        self.response
    }
}

impl std::ops::Deref for Badge<'_> {
    type Target = Response;

    #[inline]
    fn deref(&self) -> &Response {
        &self.response
    }
}

/// Scoped builder for one stat pair, returned by [`Context::stat`].
///
/// Already recorded when created; [`color`](Self::color) restyles the value
/// in place. Dereferences to its [`Response`]; [`show`](Self::show) returns
/// it.
pub struct Stat<'a> {
    ctx: &'a mut Context,
    response: Response,
    value_idx: usize,
}

impl Stat<'_> {
    /// Color of the value line (kept bold).
    #[inline]
    pub fn color(self, color: Color) -> Self {
        if let Some(Command::Text { style, .. }) = self.ctx.commands.get_mut(self.value_idx) {
            style.fg = Some(color);
        }
        self
    }

    /// Return the stat's [`Response`].
    #[inline]
    pub fn show(self) -> Response {
        self.response
    }
}

impl std::ops::Deref for Stat<'_> {
    type Target = Response;

    #[inline]
    fn deref(&self) -> &Response {
        &self.response
    }
}
