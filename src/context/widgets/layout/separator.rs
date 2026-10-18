//! `Separator<'_>`: the scoped builder returned by [`Context::separator`]
//! (#460). Recorded at creation; [`Separator::color`] restyles it in place.

use super::*;

impl Context {
    /// Render a horizontal divider line.
    ///
    /// The line is drawn dim in the theme's border color and expands to fill
    /// the container width. Chain [`color`](Separator::color) for a custom
    /// color. The builder dereferences to the divider's [`Response`] (its
    /// hit-test rect for hover detection); [`show`](Separator::show) returns
    /// it by value.
    ///
    /// ```
    /// # use slt::*;
    /// # TestBackend::new(20, 3).render(|ui| {
    /// ui.separator();
    /// let r = ui.separator().color(Color::Cyan).show();
    /// # let _ = r;
    /// # });
    /// ```
    pub fn separator(&mut self) -> Separator<'_> {
        let response = self.interaction();
        // The cached `sep_line()` is much wider than any reasonable terminal,
        // so the cross-axis (column-direction) clip in `Buffer::set_string`
        // truncates the trailing chars. Keeping `grow = 0` means a column
        // layout doesn't stretch the separator vertically, and `truncate =
        // false` avoids the ellipsis fallback which would otherwise replace
        // the last cell with `…`.
        let line = sep_line();
        let mut content = self.pooled_string(line.len());
        content.push_str(line);
        self.commands.push(Command::Text {
            content,
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
        let idx = self.commands.len() - 1;
        self.rollback.last_text_idx = Some(idx);
        Separator {
            ctx: self,
            response,
            idx,
        }
    }

    /// Render a horizontal separator line with a custom color.
    #[deprecated(since = "0.26.0", note = "use `ui.separator().color(color)`")]
    pub fn separator_colored(&mut self, color: Color) -> Response {
        self.separator().color(color).show()
    }
}

/// Scoped builder for one divider, returned by [`Context::separator`].
///
/// Already recorded when created; [`color`](Self::color) restyles it in
/// place. Dereferences to its [`Response`]; [`show`](Self::show) returns it.
pub struct Separator<'a> {
    ctx: &'a mut Context,
    response: Response,
    idx: usize,
}

#[expect(
    clippy::return_self_not_must_use,
    reason = "the widget call already recorded the element (or renders on drop); modifiers edit it in place, so a statement-terminated chain is the intended use"
)]
impl Separator<'_> {
    /// Draw the line in `color` at full intensity (drops the default dim).
    #[inline]
    pub fn color(self, color: Color) -> Self {
        if let Some(Command::Text { style, .. }) = self.ctx.commands.get_mut(self.idx) {
            *style = Style::new().fg(color);
        }
        self
    }

    /// Return the divider's [`Response`].
    #[inline]
    pub fn show(self) -> Response {
        self.response
    }
}

impl std::ops::Deref for Separator<'_> {
    type Target = Response;

    #[inline]
    fn deref(&self) -> &Response {
        &self.response
    }
}
