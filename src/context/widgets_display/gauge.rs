// Gauge / line_gauge widgets — block-fill and single-line progress indicators
// with optional inline labels.
//
// Introduced in v0.20.0 (#224). Complements the unlabeled
// `Context::progress_bar` / `Context::progress` (`textarea_progress.rs`).
//
// Callers use a chainable builder pattern
// (`ui.gauge(0.5).label("CPU").width(48)`). Auto-renders on `Drop`; call
// `.show()` to get a [`GaugeResponse`] back. Ratios are `f64` to match
// `animate_value`, chart APIs, and `progress_bar` — no more `f32` outliers.

use super::*;

/// Default width for `gauge` and `line_gauge` when no explicit width is set.
const DEFAULT_GAUGE_WIDTH: u32 = 20;

impl Context {
    /// Begin building a block-fill progress bar with optional centered label.
    ///
    /// `ratio` is clamped to `0.0..=1.0`. The returned [`Gauge`] auto-renders
    /// when dropped, so a bare `ui.gauge(0.5);` produces a default-width bar.
    /// Chain `.label(...)`, `.width(...)`, or `.color(...)` to customize.
    /// Call `.show()` (instead of dropping) to capture a [`GaugeResponse`].
    ///
    /// Color tiers follow theme colors: `success` below 50%, `warning` 50–80%,
    /// `error` at or above 80%. Override per-call with `.color(...)`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.gauge(0.6).label("60%");
    /// let r = ui.gauge(0.42).label("CPU").width(48).show();
    /// if r.hovered { /* attach tooltip */ }
    /// # });
    /// ```
    ///
    /// # Family
    ///
    /// The gauge family covers ratio-based progress indicators:
    ///
    /// - [`gauge`](Self::gauge) — block-fill bar with a centered label (this method).
    /// - [`line_gauge`](Self::line_gauge) — single-line bar with a trailing label
    ///   and configurable fill/empty chars.
    /// - [`progress_bar`](Self::progress_bar) / [`progress`](Self::progress) —
    ///   unlabeled progress bars.
    pub fn gauge(&mut self, ratio: f64) -> Gauge<'_> {
        Gauge::new(self, ratio)
    }

    /// Begin building a single-line gauge with configurable fill/empty chars.
    ///
    /// `ratio` is clamped to `0.0..=1.0`. Chain `.label(...)`, `.width(...)`,
    /// `.filled(...)`, `.empty(...)` to customize. Auto-renders on `Drop`;
    /// call `.show()` to capture a [`GaugeResponse`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.line_gauge(0.6).label("60%").width(24);
    /// ui.line_gauge(0.78).label("Memory").width(48).filled('━');
    /// # });
    /// ```
    ///
    /// # Family
    ///
    /// The gauge family covers ratio-based progress indicators:
    ///
    /// - [`line_gauge`](Self::line_gauge) — single-line bar with a trailing
    ///   label (this method).
    /// - [`gauge`](Self::gauge) — block-fill bar with a centered label.
    /// - [`progress_bar`](Self::progress_bar) / [`progress`](Self::progress) —
    ///   unlabeled progress bars.
    pub fn line_gauge(&mut self, ratio: f64) -> LineGauge<'_> {
        LineGauge::new(self, ratio)
    }
}

/// Block-fill gauge builder. Auto-renders on `Drop`.
///
/// Constructed via [`Context::gauge`]. Chainable `.label`, `.width`, `.color`
/// methods configure the gauge before it renders. Drop the value to render
/// without capturing a response, or call [`Self::show`] to render and obtain
/// a [`GaugeResponse`].
///
/// `Drop` is intentional: `ui.gauge(0.5).label("CPU");` is the idiomatic form
/// when the response isn't needed, mirroring egui's `ui.add(...)`. Use
/// [`Self::show`] when you need the response.
pub struct Gauge<'a> {
    ctx: Option<&'a mut Context>,
    ratio: f64,
    label: Option<String>,
    width: Option<u32>,
    color: Option<Color>,
}

impl<'a> Gauge<'a> {
    fn new(ctx: &'a mut Context, ratio: f64) -> Self {
        Self {
            ctx: Some(ctx),
            ratio,
            label: None,
            width: None,
            color: None,
        }
    }

    /// Set the centered inline label. Empty string is treated as "no label".
    ///
    /// Accepts both `&str` and owned `String` via `impl Into<String>` so
    /// callers with already-owned strings (e.g. `format!(...)`) don't pay a
    /// redundant clone.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        let label = label.into();
        if label.is_empty() {
            self.label = None;
        } else {
            self.label = Some(label);
        }
        self
    }

    /// Set the bar width in terminal cells (default: 20).
    pub fn width(mut self, w: u32) -> Self {
        self.width = Some(w);
        self
    }

    /// Override the auto-tiered color with a fixed color.
    pub fn color(mut self, c: Color) -> Self {
        self.color = Some(c);
        self
    }

    /// Render now and return the [`GaugeResponse`].
    pub fn show(mut self) -> GaugeResponse {
        // SAFETY: ctx is Some until Drop runs; show consumes self before Drop.
        let ctx = self.ctx.take().expect("Gauge::show called twice");
        render_gauge(
            ctx,
            self.ratio,
            self.width.unwrap_or(DEFAULT_GAUGE_WIDTH),
            self.label.as_deref().unwrap_or(""),
            self.color,
        )
    }
}

impl Drop for Gauge<'_> {
    fn drop(&mut self) {
        if let Some(ctx) = self.ctx.take() {
            let _ = render_gauge(
                ctx,
                self.ratio,
                self.width.unwrap_or(DEFAULT_GAUGE_WIDTH),
                self.label.as_deref().unwrap_or(""),
                self.color,
            );
        }
    }
}

/// Single-line gauge builder. Auto-renders on `Drop`.
///
/// Constructed via [`Context::line_gauge`]. Chainable methods configure the
/// gauge before it renders. Drop to render without capturing a response, or
/// call [`Self::show`] to render and obtain a [`GaugeResponse`].
///
/// `Drop` is intentional: `ui.line_gauge(0.5).filled('━');` is the idiomatic
/// form when the response isn't needed.
pub struct LineGauge<'a> {
    ctx: Option<&'a mut Context>,
    ratio: f64,
    label: Option<String>,
    width: Option<u32>,
    filled: char,
    empty: char,
}

impl<'a> LineGauge<'a> {
    fn new(ctx: &'a mut Context, ratio: f64) -> Self {
        Self {
            ctx: Some(ctx),
            ratio,
            label: None,
            width: None,
            filled: '━',
            empty: '─',
        }
    }

    /// Set the trailing label, appended after the bar.
    ///
    /// Accepts both `&str` and owned `String` via `impl Into<String>` so
    /// callers with already-owned strings (e.g. `format!(...)`) don't pay a
    /// redundant clone.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        let label = label.into();
        if label.is_empty() {
            self.label = None;
        } else {
            self.label = Some(label);
        }
        self
    }

    /// Set the bar width in terminal cells (default: 20).
    pub fn width(mut self, w: u32) -> Self {
        self.width = Some(w);
        self
    }

    /// Set the filled character (default: `'━'`).
    pub fn filled(mut self, ch: char) -> Self {
        self.filled = ch;
        self
    }

    /// Set the empty character (default: `'─'`).
    pub fn empty(mut self, ch: char) -> Self {
        self.empty = ch;
        self
    }

    /// Render now and return the [`GaugeResponse`].
    pub fn show(mut self) -> GaugeResponse {
        let ctx = self.ctx.take().expect("LineGauge::show called twice");
        render_line_gauge(
            ctx,
            self.ratio,
            self.width.unwrap_or(DEFAULT_GAUGE_WIDTH),
            self.filled,
            self.empty,
            self.label.as_deref(),
        )
    }
}

impl Drop for LineGauge<'_> {
    fn drop(&mut self) {
        if let Some(ctx) = self.ctx.take() {
            let _ = render_line_gauge(
                ctx,
                self.ratio,
                self.width.unwrap_or(DEFAULT_GAUGE_WIDTH),
                self.filled,
                self.empty,
                self.label.as_deref(),
            );
        }
    }
}

/// Internal rendering for a block-fill gauge.
fn render_gauge(
    ctx: &mut Context,
    ratio: f64,
    width: u32,
    label: &str,
    color_override: Option<Color>,
) -> GaugeResponse {
    let response = ctx.interaction();
    let clamped = ratio.clamp(0.0, 1.0);
    let width = width.max(1);
    let bar = compose_block_bar(clamped, width, label);
    let color = color_override.unwrap_or_else(|| gauge_color_for(ctx, clamped));
    ctx.styled(bar, Style::new().fg(color));
    GaugeResponse {
        response,
        ratio: clamped,
    }
}

/// Internal rendering for a single-line gauge.
fn render_line_gauge(
    ctx: &mut Context,
    ratio: f64,
    width: u32,
    filled: char,
    empty: char,
    label: Option<&str>,
) -> GaugeResponse {
    let response = ctx.interaction();
    let clamped = ratio.clamp(0.0, 1.0);
    let width = width.max(1);
    let bar = compose_line_bar(clamped, width, filled, empty, label);
    let color = gauge_color_for(ctx, clamped);
    ctx.styled(bar, Style::new().fg(color));
    GaugeResponse {
        response,
        ratio: clamped,
    }
}

/// Pick a color from the theme based on the current ratio.
///
/// `success` < 50%, `warning` 50–80%, `error` >= 80%.
fn gauge_color_for(ctx: &Context, ratio: f64) -> Color {
    if ratio >= 0.80 {
        ctx.theme.error
    } else if ratio >= 0.50 {
        ctx.theme.warning
    } else {
        ctx.theme.success
    }
}

/// How a label is positioned relative to the bar cells.
enum LabelMode<'a> {
    /// Overlay the label on top of the bar, centered. If the bar is too narrow
    /// to fit `label_w + 2`, the label is omitted entirely (not truncated).
    Centered(&'a str),
    /// Append the label after the bar, separated by a single space. Empty or
    /// missing labels emit nothing.
    Trailing(Option<&'a str>),
}

/// Compute the filled-cell count for `ratio`, clamped to `[0, width]`.
///
/// Internal math runs in `f64` (the public ratio type) and only crosses to
/// `u32` at this boundary — keeps `compose_*_bar` precision-stable.
fn filled_cells(ratio: f64, width: u32) -> u32 {
    let count = (ratio * f64::from(width)).round() as u32;
    count.min(width)
}

/// Shared bar-composition core for `compose_block_bar` / `compose_line_bar`.
///
/// Builds `width` cells (filled or empty) and overlays/appends the label
/// according to `mode`. Unicode width is honored for centered overlays so
/// multi-byte labels (e.g. CJK) line up correctly.
fn compose_bar(
    ratio: f64,
    width: u32,
    fill_ch: char,
    empty_ch: char,
    mode: LabelMode<'_>,
) -> String {
    let width_usize = width as usize;
    let filled = filled_cells(ratio, width);

    if let LabelMode::Centered(label) = mode {
        if !label.is_empty() {
            let label_w = UnicodeWidthStr::width(label);
            if label_w + 2 <= width_usize {
                // Build the bar then overlay the centered label.
                let mut cells: Vec<char> = Vec::with_capacity(width_usize);
                for i in 0..width {
                    cells.push(if i < filled { fill_ch } else { empty_ch });
                }
                let label_start = (width_usize.saturating_sub(label_w)) / 2;
                let label_end = label_start + label_w;
                let mut out = String::with_capacity(width_usize * 4 + label.len());
                for ch in cells.iter().take(label_start) {
                    out.push(*ch);
                }
                out.push_str(label);
                for ch in cells.iter().take(width_usize).skip(label_end) {
                    out.push(*ch);
                }
                return out;
            }
        }
    }

    // Plain bar (no label, label too wide, or trailing mode).
    let trailing = match mode {
        LabelMode::Trailing(Some(lbl)) if !lbl.is_empty() => Some(lbl),
        _ => None,
    };
    let mut out = String::with_capacity(
        width_usize * fill_ch.len_utf8().max(empty_ch.len_utf8())
            + trailing.map_or(0, |s| s.len() + 1),
    );
    for _ in 0..filled {
        out.push(fill_ch);
    }
    for _ in 0..width.saturating_sub(filled) {
        out.push(empty_ch);
    }
    if let Some(lbl) = trailing {
        out.push(' ');
        out.push_str(lbl);
    }
    out
}

/// Build a block-style bar (`█` filled, `░` empty) of `width` cells with an
/// optional centered `label`. The label is omitted (not truncated) when the
/// bar is too narrow to fit it.
fn compose_block_bar(ratio: f64, width: u32, label: &str) -> String {
    compose_bar(ratio, width, '█', '░', LabelMode::Centered(label))
}

/// Build a single-line bar with configurable fill/empty chars and optional
/// label appended after the bar (not centered inside).
fn compose_line_bar(
    ratio: f64,
    width: u32,
    filled: char,
    empty: char,
    label: Option<&str>,
) -> String {
    compose_bar(ratio, width, filled, empty, LabelMode::Trailing(label))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_bar_no_label() {
        let bar = compose_block_bar(0.5, 10, "");
        assert_eq!(bar, "█████░░░░░");
    }

    #[test]
    fn block_bar_with_label() {
        let bar = compose_block_bar(0.5, 12, "50%");
        assert!(bar.contains("50%"), "label visible: {bar}");
        // The label sits on the bar — total cells unchanged.
        assert_eq!(UnicodeWidthStr::width(bar.as_str()), 12);
    }

    #[test]
    fn block_bar_omits_label_when_too_narrow() {
        // "12345" is 5 wide; bar of 6 has only 4 free cells (need label_w + 2).
        let bar = compose_block_bar(0.5, 6, "12345");
        assert!(!bar.contains("12345"));
        assert_eq!(UnicodeWidthStr::width(bar.as_str()), 6);
    }

    #[test]
    fn line_bar_default_chars() {
        let bar = compose_line_bar(0.5, 10, '━', '─', None);
        assert_eq!(bar, "━━━━━─────");
    }

    #[test]
    fn line_bar_appends_label() {
        let bar = compose_line_bar(1.0, 4, '#', '.', Some("done"));
        assert_eq!(bar, "#### done");
    }

    #[test]
    fn block_bar_f64_precision() {
        // Ratios that f32 rounds differently from f64 still produce stable
        // block counts — confirms internal math runs in f64.
        let bar = compose_block_bar(1.0 / 3.0, 30, "");
        let filled = bar.chars().filter(|&c| c == '█').count();
        // (1/3 * 30).round() == 10
        assert_eq!(filled, 10);
    }
}
