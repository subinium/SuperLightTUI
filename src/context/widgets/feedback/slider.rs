//! `Slider<'_>`: the builder returned by [`Context::slider`] (#460).
//!
//! The keyboard step changes how input is applied, so unlike the restyling
//! builders the slider cannot be recorded before its options are known. It
//! is recorded the first time its [`Response`] is read (a field access such
//! as `.changed`, [`Slider::show`]) or, when nothing reads it, at the end of
//! the statement. The builder holds the exclusive `&mut Context` borrow the
//! whole time, so nothing else can be recorded first: the slider still lands
//! exactly where it is written.

use std::cell::{Cell, OnceCell};
use std::ops::RangeInclusive;

use super::*;

impl Context {
    /// Horizontal slider for numeric values.
    ///
    /// Left/Right (or `h`/`l`) move `value` by the step, which defaults to
    /// `span / 20`; chain [`step`](Slider::step) for an explicit one (integer
    /// counters want `1.0`, fine controls `0.1`). Read the [`Response`]
    /// straight off the call or keep it with [`show`](Slider::show).
    ///
    /// ```
    /// # use slt::*;
    /// # TestBackend::new(80, 4).render(|ui| {
    /// let mut volume = 75.0_f64;
    /// if ui.slider("Volume", &mut volume, 0.0..=100.0).changed { /* adjusted */ }
    ///
    /// let mut count = 3.0_f64;
    /// ui.slider("Count", &mut count, 0.0..=10.0).step(1.0);
    /// let r = ui.slider("Fine", &mut count, 0.0..=10.0).step(0.1).show();
    /// # let _ = r;
    /// # });
    /// ```
    pub fn slider<'a>(
        &'a mut self,
        label: &'a str,
        value: &'a mut f64,
        range: RangeInclusive<f64>,
    ) -> Slider<'a> {
        Slider {
            parts: Cell::new(Some((self, value))),
            label,
            range,
            step: None,
            response: OnceCell::new(),
        }
    }

    /// Horizontal slider with options for label, range, and explicit step.
    #[deprecated(
        since = "0.26.0",
        note = "use `ui.slider(label, &mut value, range).step(step)`"
    )]
    pub fn slider_with(&mut self, value: &mut f64, opts: crate::widgets::SliderOpts) -> Response {
        let step = slider_step(&opts.range, opts.step);
        self.slider_inner(&opts.label, value, opts.range, step)
    }

    /// Horizontal slider with an explicit step size.
    #[deprecated(
        since = "0.23.0",
        note = "use `ui.slider(label, &mut value, range).step(step)`"
    )]
    pub fn slider_with_step(
        &mut self,
        label: &str,
        value: &mut f64,
        range: RangeInclusive<f64>,
        step: f64,
    ) -> Response {
        self.slider(label, value, range).step(step).show()
    }
}

/// Explicit step, or one twentieth of the normalized span.
fn slider_step(range: &RangeInclusive<f64>, step: Option<f64>) -> f64 {
    let (start, end) = crate::widgets::normalize_numeric_range(*range.start(), *range.end());
    let span = end - start;
    step.unwrap_or(if span > 0.0 { span / 20.0 } else { 0.0 })
}

/// Builder for one slider, returned by [`Context::slider`].
///
/// Recorded when its [`Response`] is first read (it dereferences to it),
/// on [`show`](Self::show), or at the end of the statement, always before
/// anything else can be recorded. Set options with [`step`](Self::step)
/// before reading the response.
pub struct Slider<'a> {
    /// Taken exactly once, when the slider is recorded.
    parts: Cell<Option<(&'a mut Context, &'a mut f64)>>,
    label: &'a str,
    range: RangeInclusive<f64>,
    step: Option<f64>,
    response: OnceCell<Response>,
}

impl Slider<'_> {
    /// Keyboard step: each Left/Right moves the value by `step`. A
    /// non-finite or non-positive step disables keyboard stepping.
    #[inline]
    pub fn step(mut self, step: f64) -> Self {
        self.step = Some(crate::widgets::normalize_numeric_step(step));
        self
    }

    /// Record the slider (if it is not yet) and return its [`Response`].
    #[inline]
    pub fn show(self) -> Response {
        *self.record()
    }

    fn record(&self) -> &Response {
        self.response.get_or_init(|| match self.parts.take() {
            Some((ctx, value)) => {
                let step = slider_step(&self.range, self.step);
                ctx.slider_inner(self.label, value, self.range.clone(), step)
            }
            None => Response::none(),
        })
    }
}

impl std::ops::Deref for Slider<'_> {
    type Target = Response;

    #[inline]
    fn deref(&self) -> &Response {
        self.record()
    }
}

impl Drop for Slider<'_> {
    fn drop(&mut self) {
        self.record();
    }
}
