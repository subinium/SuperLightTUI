//! `ProgressBar<'_>`: the scoped builder returned by [`Context::progress`]
//! and [`Context::progress_bar`] (#460). Recorded at creation;
//! [`ProgressBar::color`] restyles it in place.

use super::*;

impl Context {
    /// Render a progress bar (20 chars wide). `ratio` is clamped to `0.0..=1.0`.
    ///
    /// Uses block characters (`█` filled, `░` empty) in the theme's primary
    /// color. For a custom width use [`Context::progress_bar`]; for an inline
    /// label use [`Context::gauge`]. Chain [`color`](ProgressBar::color) for
    /// another fill color. The builder dereferences to the bar's
    /// [`Response`], so callers can detect hover, attach a tooltip, or build
    /// click-to-set scrubbers.
    ///
    /// ```
    /// # use slt::*;
    /// # TestBackend::new(30, 3).render(|ui| {
    /// ui.progress(0.5);
    /// ui.progress(0.8).color(Color::Yellow);
    /// let r = ui.progress(0.3).show();
    /// # let _ = r;
    /// # });
    /// ```
    pub fn progress(&mut self, ratio: f64) -> ProgressBar<'_> {
        self.progress_bar(ratio, 20)
    }

    /// Render a progress bar with a custom character width.
    ///
    /// `ratio` is clamped to `0.0..=1.0`. `width` is the total number of
    /// characters rendered. Chain [`color`](ProgressBar::color) for another
    /// fill color.
    ///
    /// ```
    /// # use slt::*;
    /// # TestBackend::new(40, 2).render(|ui| {
    /// ui.progress_bar(0.42, 32).color(Color::Green);
    /// # });
    /// ```
    pub fn progress_bar(&mut self, ratio: f64, width: u32) -> ProgressBar<'_> {
        let response = self.interaction();
        let clamped = ratio.clamp(0.0, 1.0);
        let filled = (clamped * width as f64).round() as u32;
        let empty = width.saturating_sub(filled);
        let mut bar = self.pooled_string(width as usize * 3);
        for _ in 0..filled {
            bar.push('█');
        }
        for _ in 0..empty {
            bar.push('░');
        }
        let color = self.theme.primary;
        self.styled(bar, Style::new().fg(color));
        let idx = self.commands.len() - 1;
        ProgressBar {
            ctx: self,
            response,
            idx,
        }
    }

    /// Render a progress bar with a custom fill color.
    #[deprecated(
        since = "0.26.0",
        note = "use `ui.progress_bar(ratio, width).color(color)`"
    )]
    pub fn progress_bar_colored(&mut self, ratio: f64, width: u32, color: Color) -> Response {
        self.progress_bar(ratio, width).color(color).show()
    }
}

/// Scoped builder for one progress bar, returned by [`Context::progress`] and
/// [`Context::progress_bar`].
///
/// Already recorded when created; [`color`](Self::color) restyles it in
/// place. Dereferences to its [`Response`]; [`show`](Self::show) returns it.
pub struct ProgressBar<'a> {
    ctx: &'a mut Context,
    response: Response,
    idx: usize,
}

#[expect(
    clippy::return_self_not_must_use,
    reason = "the widget call already recorded the element (or renders on drop); modifiers edit it in place, so a statement-terminated chain is the intended use"
)]
impl ProgressBar<'_> {
    /// Fill color of the bar.
    #[inline]
    pub fn color(self, color: Color) -> Self {
        if let Some(Command::Text { style, .. }) = self.ctx.commands.get_mut(self.idx) {
            style.fg = Some(color);
        }
        self
    }

    /// Return the bar's [`Response`].
    #[inline]
    pub fn show(self) -> Response {
        self.response
    }
}

impl std::ops::Deref for ProgressBar<'_> {
    type Target = Response;

    #[inline]
    fn deref(&self) -> &Response {
        &self.response
    }
}
