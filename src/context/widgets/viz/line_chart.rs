//! `LineChart<'_>`: the builder returned by [`Context::line_chart`] and
//! [`Context::area_chart`] (#460).
//!
//! The chart rasterizes its series with the line color, so it is recorded
//! when its [`Response`] is first read, on [`LineChart::show`], or at the end
//! of the statement. The builder holds the exclusive `&mut Context` borrow
//! until then, so nothing else can be recorded first.

use std::cell::{Cell, OnceCell};

use super::*;

impl Context {
    /// Render a multi-row line chart using braille characters.
    ///
    /// `width` and `height` are terminal cell dimensions. Internally this uses
    /// braille dot resolution (`width*2` x `height*4`) for smoother plotting.
    /// The line uses the theme's primary color; chain
    /// [`color`](LineChart::color) for another.
    ///
    /// ```
    /// # use slt::*;
    /// # TestBackend::new(40, 10).render(|ui| {
    /// let data = [1.0, 3.0, 2.0, 5.0, 4.0, 6.0, 3.0, 7.0];
    /// ui.line_chart(&data, 40, 4);
    /// let r = ui.line_chart(&data, 40, 4).color(Color::Cyan).show();
    /// # let _ = r;
    /// # });
    /// ```
    pub fn line_chart<'a>(&'a mut self, data: &'a [f64], width: u32, height: u32) -> LineChart<'a> {
        LineChart::new(self, data, width, height, false)
    }

    /// Render a multi-row area chart (the line chart filled down to its
    /// baseline) using the primary theme color; chain
    /// [`color`](LineChart::color) for another.
    ///
    /// ```
    /// # use slt::*;
    /// # TestBackend::new(40, 6).render(|ui| {
    /// ui.area_chart(&[1.0, 4.0, 2.0, 6.0], 40, 4).color(Color::Green);
    /// # });
    /// ```
    pub fn area_chart<'a>(&'a mut self, data: &'a [f64], width: u32, height: u32) -> LineChart<'a> {
        LineChart::new(self, data, width, height, true)
    }

    /// Render a multi-row line chart using a custom color.
    #[deprecated(
        since = "0.26.0",
        note = "use `ui.line_chart(data, width, height).color(color)`"
    )]
    pub fn line_chart_colored(
        &mut self,
        data: &[f64],
        width: u32,
        height: u32,
        color: Color,
    ) -> Response {
        self.render_line_chart_internal(data, width, height, color, false)
    }

    /// Render a multi-row area chart using a custom color.
    #[deprecated(
        since = "0.26.0",
        note = "use `ui.area_chart(data, width, height).color(color)`"
    )]
    pub fn area_chart_colored(
        &mut self,
        data: &[f64],
        width: u32,
        height: u32,
        color: Color,
    ) -> Response {
        self.render_line_chart_internal(data, width, height, color, true)
    }
}

/// Builder for one line or area chart, returned by [`Context::line_chart`]
/// and [`Context::area_chart`].
///
/// Recorded when its [`Response`] is first read (it dereferences to it), on
/// [`show`](Self::show), or at the end of the statement, always before
/// anything else can be recorded. Set [`color`](Self::color) before reading
/// the response.
pub struct LineChart<'a> {
    /// Taken exactly once, when the chart is recorded.
    ctx: Cell<Option<&'a mut Context>>,
    data: &'a [f64],
    width: u32,
    height: u32,
    fill: bool,
    color: Option<Color>,
    response: OnceCell<Response>,
}

impl<'a> LineChart<'a> {
    fn new(ctx: &'a mut Context, data: &'a [f64], width: u32, height: u32, fill: bool) -> Self {
        Self {
            ctx: Cell::new(Some(ctx)),
            data,
            width,
            height,
            fill,
            color: None,
            response: OnceCell::new(),
        }
    }

    /// Line (and fill) color; defaults to the theme's primary color.
    #[inline]
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Record the chart (if it is not yet) and return its [`Response`].
    #[inline]
    pub fn show(self) -> Response {
        *self.record()
    }

    fn record(&self) -> &Response {
        self.response.get_or_init(|| match self.ctx.take() {
            Some(ctx) => {
                let color = self.color.unwrap_or(ctx.theme.primary);
                ctx.render_line_chart_internal(self.data, self.width, self.height, color, self.fill)
            }
            None => Response::none(),
        })
    }
}

impl std::ops::Deref for LineChart<'_> {
    type Target = Response;

    #[inline]
    fn deref(&self) -> &Response {
        self.record()
    }
}

impl Drop for LineChart<'_> {
    fn drop(&mut self) {
        self.record();
    }
}
