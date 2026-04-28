// Gauge / line_gauge widgets — block-fill and single-line progress indicators
// with optional inline labels.
//
// Introduced in v0.20.0 (#224). Complements the unlabeled
// `Context::progress_bar` / `Context::progress` (`textarea_progress.rs`).

use super::*;

/// Default width for `gauge` and `line_gauge` when no explicit width is set.
const DEFAULT_GAUGE_WIDTH: u32 = 20;

impl Context {
    /// Render a block-fill progress bar with a centered inline label.
    ///
    /// `ratio` is clamped to `0.0..=1.0`. The label is rendered centered in
    /// the bar; pass `""` for no label. Width defaults to 20 cells; use
    /// [`Self::gauge_w`] for an explicit size.
    ///
    /// Color tiers follow theme colors: `success` below 50%, `warning` 50–80%,
    /// `error` above 80%. Override per-call via [`Self::gauge_colored`] when
    /// you need a single fixed color regardless of progress.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// let r = ui.gauge(0.6, "60%");
    /// if r.hovered { /* attach tooltip */ }
    /// # });
    /// ```
    pub fn gauge(&mut self, ratio: f32, label: &str) -> GaugeResponse {
        self.gauge_w(ratio, label, DEFAULT_GAUGE_WIDTH)
    }

    /// Render a block-fill gauge with an inline label at a fixed width.
    ///
    /// See [`Self::gauge`] for details on label centering and color tiers.
    pub fn gauge_w(&mut self, ratio: f32, label: &str, width: u32) -> GaugeResponse {
        let clamped = ratio.clamp(0.0, 1.0);
        let color = gauge_color_for(self, clamped);
        self.gauge_colored(clamped, label, width, color)
    }

    /// Render a block-fill gauge with a fixed color (no automatic tiering).
    pub fn gauge_colored(
        &mut self,
        ratio: f32,
        label: &str,
        width: u32,
        color: Color,
    ) -> GaugeResponse {
        let response = self.interaction();
        let clamped = ratio.clamp(0.0, 1.0);
        let width = width.max(1);
        let bar = compose_block_bar(clamped, width, label);
        self.styled(bar, Style::new().fg(color));
        GaugeResponse {
            response,
            ratio: clamped,
        }
    }

    /// Render a single-line gauge with configurable fill/empty characters.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::LineGaugeOpts;
    /// # slt::run(|ui: &mut slt::Context| {
    /// let _ = ui.line_gauge(0.6, LineGaugeOpts::default().label("60%"));
    /// # });
    /// ```
    pub fn line_gauge(&mut self, ratio: f32, opts: LineGaugeOpts) -> GaugeResponse {
        let response = self.interaction();
        let clamped = ratio.clamp(0.0, 1.0);
        let width = opts.width.unwrap_or(DEFAULT_GAUGE_WIDTH).max(1);
        let bar = compose_line_bar(
            clamped,
            width,
            opts.filled,
            opts.empty,
            opts.label.as_deref(),
        );
        let color = gauge_color_for(self, clamped);
        self.styled(bar, Style::new().fg(color));
        GaugeResponse {
            response,
            ratio: clamped,
        }
    }
}

/// Pick a color from the theme based on the current ratio.
///
/// `success` < 50%, `warning` 50–80%, `error` > 80%.
fn gauge_color_for(ctx: &Context, ratio: f32) -> Color {
    if ratio >= 0.80 {
        ctx.theme.error
    } else if ratio >= 0.50 {
        ctx.theme.warning
    } else {
        ctx.theme.success
    }
}

/// Build a block-style bar (`█` filled, `░` empty) of `width` cells with an
/// optional centered `label`. The label is omitted (not truncated) when the
/// bar is too narrow to fit it.
fn compose_block_bar(ratio: f32, width: u32, label: &str) -> String {
    let width_usize = width as usize;
    let filled = (ratio * width as f32).round() as u32;
    let filled = filled.min(width);

    if !label.is_empty() {
        let label_w = UnicodeWidthStr::width(label);
        if label_w + 2 <= width_usize {
            // Center the label and overlay it on the bar.
            let mut cells: Vec<char> = Vec::with_capacity(width_usize);
            for i in 0..width {
                if i < filled {
                    cells.push('█');
                } else {
                    cells.push('░');
                }
            }
            let label_start = (width_usize.saturating_sub(label_w)) / 2;
            let label_end = label_start + label_w;
            let mut out = String::with_capacity(width_usize * 4 + label.len());
            // Push leading bar cells.
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

    // No label or label doesn't fit — emit plain bar.
    let mut out = String::with_capacity(width_usize * 3);
    for _ in 0..filled {
        out.push('█');
    }
    for _ in 0..width.saturating_sub(filled) {
        out.push('░');
    }
    out
}

/// Build a single-line bar with configurable fill/empty chars and optional
/// label appended after the bar (not centered inside).
fn compose_line_bar(
    ratio: f32,
    width: u32,
    filled: char,
    empty: char,
    label: Option<&str>,
) -> String {
    let filled_count = (ratio * width as f32).round() as u32;
    let filled_count = filled_count.min(width);
    let empty_count = width.saturating_sub(filled_count);
    let mut out = String::with_capacity(width as usize + label.map_or(0, |s| s.len() + 1));
    for _ in 0..filled_count {
        out.push(filled);
    }
    for _ in 0..empty_count {
        out.push(empty);
    }
    if let Some(lbl) = label {
        if !lbl.is_empty() {
            out.push(' ');
            out.push_str(lbl);
        }
    }
    out
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
}
