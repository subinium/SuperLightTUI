//! `Button<'_>`: the scoped builder returned by [`Context::button`] (#460).
//!
//! The button is recorded when the builder is created, exactly like the
//! scoped [`Text`](crate::Text) builder (#457): focus, hit-testing and
//! activation run at creation, so the [`Response`] is final before the first
//! chained call. `.variant(..)` and `.colors(..)` only change the look, so
//! they restyle the already-recorded commands in place. The builder holds the
//! exclusive `&mut Context` borrow, so nothing can be recorded between the
//! button and its options: a chain renders exactly where it is written.

use super::*;

/// Look of one button: what the recorded container and label carry.
struct ButtonLook {
    /// `"[ label ]"` when `true`, `" label "` otherwise.
    bracketed: bool,
    label_style: Style,
    bg_color: Option<Color>,
    border: Option<Border>,
    border_style: Style,
    centered: bool,
}

/// Compute the look for `variant` (or the plain button when `None`).
///
/// `colors` overrides the theme per role: `fg` is the label color, `bg` the
/// fill (`Primary` / `Danger`) or the background (plain, `Default`,
/// `Outline`), `accent` the hover/focus color and `border` the `Outline`
/// border. Unset fields fall back to the theme, which reproduces the 0.25
/// `button` / `button_with` looks exactly.
fn button_look(
    theme: &Theme,
    variant: Option<ButtonVariant>,
    colors: &WidgetColors,
    focused: bool,
    hovered: bool,
) -> ButtonLook {
    let Some(variant) = variant else {
        let base_fg = colors.fg.unwrap_or(theme.text);
        let accent = colors.accent.unwrap_or(theme.accent);
        let base_bg = colors.bg.unwrap_or(theme.surface_hover);
        let label_style = if focused {
            Style::new().fg(accent).bold()
        } else if hovered {
            Style::new().fg(accent)
        } else {
            Style::new().fg(base_fg)
        };
        return ButtonLook {
            bracketed: true,
            label_style,
            bg_color: (colors.bg.is_some() || hovered || focused).then_some(base_bg),
            border: None,
            border_style: Style::new().fg(colors.border.unwrap_or(theme.border)),
            centered: false,
        };
    };

    let active = hovered || focused;
    let surface_bg = if colors.bg.is_some() {
        colors.bg
    } else {
        active.then_some(theme.surface_hover)
    };
    let hover_bg = active.then_some(theme.surface_hover);
    let focus_fg = colors.accent.unwrap_or(theme.primary);
    let hover_fg = colors.accent.unwrap_or(theme.accent);
    let plain = |base: Color| {
        if focused {
            Style::new().fg(focus_fg).bold()
        } else if hovered {
            Style::new().fg(hover_fg)
        } else {
            Style::new().fg(base)
        }
    };
    let filled = |fill: Color, hover_fill: Color| {
        let text = colors.fg.unwrap_or(theme.bg);
        if focused {
            Style::new().fg(text).bg(fill).bold()
        } else if hovered {
            Style::new().fg(text).bg(hover_fill)
        } else {
            Style::new().fg(text).bg(fill)
        }
    };
    let look = |bracketed, label_style, bg_color| ButtonLook {
        bracketed,
        label_style,
        bg_color,
        border: None,
        border_style: Style::new(),
        centered: true,
    };
    match variant {
        ButtonVariant::Primary => look(
            false,
            filled(
                colors.bg.unwrap_or(theme.primary),
                colors.accent.unwrap_or(theme.accent),
            ),
            hover_bg,
        ),
        ButtonVariant::Danger => look(
            false,
            filled(
                colors.bg.unwrap_or(theme.error),
                colors.accent.unwrap_or(theme.warning),
            ),
            hover_bg,
        ),
        ButtonVariant::Outline => {
            let border_color = if focused {
                focus_fg
            } else if hovered {
                hover_fg
            } else {
                colors.border.unwrap_or(theme.border)
            };
            ButtonLook {
                border: Some(Border::Rounded),
                border_style: Style::new().fg(border_color),
                ..look(false, plain(colors.fg.unwrap_or(theme.text)), surface_bg)
            }
        }
        // `Default` and any future variant.
        _ => look(true, plain(colors.fg.unwrap_or(theme.text)), surface_bg),
    }
}

/// Write `label` framed as `"[ label ]"` or `" label "` into `out`.
fn frame_label(out: &mut String, label: &str, bracketed: bool) {
    out.clear();
    if bracketed {
        out.push_str("[ ");
        out.push_str(label);
        out.push_str(" ]");
    } else {
        out.push(' ');
        out.push_str(label);
        out.push(' ');
    }
}

/// Re-frame an already framed label in place (no allocation when shrinking).
fn reframe(content: &mut String, bracketed: bool) {
    let was_bracketed = content.starts_with('[');
    if was_bracketed == bracketed {
        return;
    }
    if bracketed {
        // " label " -> "[ label ]"
        content.replace_range(..1, "[ ");
        let end = content.len() - 1;
        content.replace_range(end.., " ]");
    } else {
        // "[ label ]" -> " label "
        content.replace_range(..2, " ");
        let end = content.len() - 2;
        content.replace_range(end.., " ");
    }
}

impl Context {
    /// Render a clickable button. Activation fires via Enter, Space, or mouse click.
    ///
    /// Returns a [`Button`] builder. The button is recorded immediately and
    /// its [`Response`] is final, so read fields straight off the call
    /// (`ui.button("Save").clicked`), chain `.on_click(ui, ..)` /
    /// `.on_hover(ui, ..)`, or set the look with
    /// [`variant`](Button::variant) and [`colors`](Button::colors), which
    /// compose. [`show`](Button::show) returns the `Response` by value.
    ///
    /// The plain button is styled with the theme's accent color when focused
    /// or hovered. Mouse clicks work with the default mouse mode; hover
    /// styling needs [`MouseMode::Hover`](crate::MouseMode::Hover); keyboard
    /// activation works without it.
    ///
    /// ```
    /// # use slt::*;
    /// # TestBackend::new(40, 4).render(|ui| {
    /// if ui.button("Save").clicked { /* save */ }
    ///
    /// let danger = WidgetColors::new().bg(Color::Rgb(170, 20, 40));
    /// let r = ui
    ///     .button("Delete")
    ///     .variant(ButtonVariant::Danger)
    ///     .colors(&danger)
    ///     .show();
    /// if r.clicked { /* delete */ }
    /// # });
    /// ```
    pub fn button(&mut self, label: impl Into<String>) -> Button<'_> {
        let colors = self.widget_theme.button;
        // Non-generic body (#491): one copy instead of one per label type.
        self.button_begin(label.into(), colors)
    }

    /// Render a clickable button with custom widget colors.
    #[deprecated(since = "0.26.0", note = "use `ui.button(label).colors(&colors)`")]
    #[must_use = "a button's only output is its Response: check `.clicked` or chain `.on_click(ui, ..)`"]
    pub fn button_colored(&mut self, label: impl Into<String>, colors: &WidgetColors) -> Response {
        self.button_begin(label.into(), *colors).show()
    }

    /// Render a styled button variant.
    ///
    /// Use [`ButtonVariant::Primary`] for call-to-action, [`ButtonVariant::Danger`]
    /// for destructive actions, or [`ButtonVariant::Outline`] for secondary actions.
    #[deprecated(since = "0.26.0", note = "use `ui.button(label).variant(variant)`")]
    #[must_use = "a button's only output is its Response: check `.clicked` or chain `.on_click(ui, ..)`"]
    pub fn button_with(&mut self, label: impl Into<String>, variant: ButtonVariant) -> Response {
        self.button(label).variant(variant).show()
    }

    fn button_begin(&mut self, label: String, colors: WidgetColors) -> Button<'_> {
        let focused = self.register_focusable();
        let mut a11y = self.a11y_begin(focused);
        let (_interaction_id, mut response) = self.begin_widget_interaction(focused);

        let activated = response.clicked
            || self.consume_activation_keys(focused)
            || A11yBegin::take_activate(a11y.as_mut());

        let look = button_look(&self.theme, None, &colors, focused, response.hovered);
        let begin = self.commands.len();
        self.commands
            .push(Command::BeginContainer(BeginContainerArgs {
                direction: Direction::Row,
                gap: 0,
                align: Align::Start,
                align_self: None,
                justify: Justify::Start,
                border: look.border,
                border_sides: BorderSides::all(),
                border_style: look.border_style,
                bg_color: look.bg_color,
                padding: Padding::default(),
                margin: Margin::default(),
                constraints: Constraints::default(),
                title: None,
                grow: 0,
                group_name: None,
            }));
        let mut label_text = String::with_capacity(label.len() + 4);
        frame_label(&mut label_text, &label, look.bracketed);
        self.styled(label_text, look.label_style);
        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;
        if let Some(begin) = a11y {
            self.a11y_end_labelled(begin, A11yRole::Button, &label, None);
        }

        response.clicked = activated;
        Button {
            ctx: self,
            response,
            begin,
            variant: None,
            colors,
            focused,
        }
    }
}

/// Scoped builder for one button, returned by [`Context::button`].
///
/// The button is already recorded and its [`Response`] is final when the
/// builder is created; `.variant(..)` and `.colors(..)` restyle it in place
/// and compose. The builder dereferences to its `Response`, so existing
/// `ui.button("x").clicked` and `ui.button("x").on_click(ui, ..)` code keeps
/// working; [`show`](Self::show) returns the `Response` by value.
///
/// ```
/// # use slt::*;
/// # TestBackend::new(40, 6).render(|ui| {
/// // builder form: renders where written, response read off the chain
/// if ui.button("Save").variant(ButtonVariant::Primary).clicked { /* save */ }
/// // `.show()` form: keep the response
/// let colors = WidgetColors::new().fg(Color::White).bg(Color::Blue);
/// let r: Response = ui.button("Open").colors(&colors).show();
/// # let _ = r;
/// # });
/// ```
///
/// Ignoring the builder warns, because a button's `clicked` flag is its only
/// output:
///
/// ```compile_fail
/// #![deny(unused_must_use)]
/// # slt::TestBackend::new(20, 3).render(|ui| {
/// ui.button("Save").variant(slt::ButtonVariant::Primary);
/// # });
/// ```
#[must_use = "a button's only output is its Response: check `.clicked`, chain `.on_click(ui, ..)` or call `.show()`"]
pub struct Button<'a> {
    ctx: &'a mut Context,
    response: Response,
    /// Index of the button's `BeginContainer`; the label follows it.
    begin: usize,
    variant: Option<ButtonVariant>,
    colors: WidgetColors,
    focused: bool,
}

impl Button<'_> {
    /// Use a [`ButtonVariant`] look: `Primary` (filled call to action),
    /// `Danger` (filled, destructive), `Outline` (bordered) or `Default`.
    ///
    /// Without a variant the button keeps the plain `[ label ]` look of
    /// [`Context::button`].
    #[inline]
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = Some(variant);
        self.restyle();
        self
    }

    /// Override the theme colors for this button.
    ///
    /// Replaces the [`RunConfig::widget_theme`](crate::RunConfig::widget_theme)
    /// button colors. `fg` colors the label, `bg` the fill of `Primary` /
    /// `Danger` (the background otherwise), `accent` the hover/focus color and
    /// `border` the `Outline` border. Combines with [`variant`](Self::variant)
    /// in either order.
    #[inline]
    pub fn colors(mut self, colors: &WidgetColors) -> Self {
        self.colors = *colors;
        self.restyle();
        self
    }

    /// Return the button's [`Response`].
    #[inline]
    pub fn show(self) -> Response {
        self.response
    }

    fn restyle(&mut self) {
        let look = button_look(
            &self.ctx.theme,
            self.variant,
            &self.colors,
            self.focused,
            self.response.hovered,
        );
        if let Some(Command::BeginContainer(args)) = self.ctx.commands.get_mut(self.begin) {
            args.border = look.border;
            args.border_style = look.border_style;
            args.bg_color = look.bg_color;
            let (align, justify) = if look.centered {
                (Align::Center, Justify::Center)
            } else {
                (Align::Start, Justify::Start)
            };
            args.align = align;
            args.justify = justify;
        }
        if let Some(Command::Text { content, style, .. }) =
            self.ctx.commands.get_mut(self.begin + 1)
        {
            reframe(content, look.bracketed);
            *style = look.label_style;
        }
    }
}

impl std::ops::Deref for Button<'_> {
    type Target = Response;

    #[inline]
    fn deref(&self) -> &Response {
        &self.response
    }
}

#[cfg(test)]
mod tests {
    use super::{frame_label, reframe};

    #[test]
    fn reframe_round_trips_both_frames() {
        let mut s = String::new();
        frame_label(&mut s, "Save", true);
        assert_eq!(s, "[ Save ]");
        reframe(&mut s, false);
        assert_eq!(s, " Save ");
        reframe(&mut s, true);
        assert_eq!(s, "[ Save ]");
        reframe(&mut s, true);
        assert_eq!(s, "[ Save ]");
        frame_label(&mut s, "[x]", false);
        assert_eq!(s, " [x] ");
        reframe(&mut s, true);
        assert_eq!(s, "[ [x] ]");
    }
}
