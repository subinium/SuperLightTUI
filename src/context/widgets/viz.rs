//! Visualization family — bar charts, sparklines, line / area / scatter
//! charts, histograms, candlesticks, heatmaps, treemaps, canvases and QR
//! codes.
//!
//! Layer 3 widgets that render dense numeric or geometric data. Most
//! draw via the chart kernel ([`crate::chart`]) or the buffer-level
//! drawing primitives in `super::container`. Bar data types live in
//! `context/bars.rs`; [`TreemapItem`] lives in `viz/heat_tree.rs`.

use super::*;

mod bars;
mod canvas;
mod charts;
mod heat_tree;
mod line_chart;
mod qr;

pub use heat_tree::TreemapItem;
pub use line_chart::LineChart;

#[cfg(test)]
mod tests;

fn finite_or_zero(value: f64) -> f64 {
    if value.is_finite() { value } else { 0.0 }
}

fn positive_finite_max(values: impl Iterator<Item = f64>) -> f64 {
    values
        .filter(|value| value.is_finite() && *value > 0.0)
        .fold(1.0, f64::max)
}

fn bounded_positive_sum(values: impl Iterator<Item = f64>) -> f64 {
    const MAX_SAFE_SUM: f64 = f64::MAX / 16.0;
    values
        .filter(|value| value.is_finite() && *value > 0.0)
        .fold(0.0, |sum, value| {
            if sum >= MAX_SAFE_SUM - value.min(MAX_SAFE_SUM) {
                MAX_SAFE_SUM
            } else {
                sum + value
            }
        })
}

fn unit_ratio(value: f64, min: f64, max: f64) -> f64 {
    crate::chart::finite_ratio(value, min, max).unwrap_or(0.0)
}

fn finite_lerp(a: f64, b: f64, t: f64) -> f64 {
    match (a.is_finite(), b.is_finite()) {
        (true, true) => {
            let scale = a.abs().max(b.abs()).max(1.0);
            ((a / scale) * (1.0 - t) + (b / scale) * t) * scale
        }
        (true, false) => a,
        (false, true) => b,
        (false, false) => f64::NAN,
    }
}

impl Context {
    fn begin_viz_root(&mut self) -> Response {
        let response = self.interaction();
        self.commands
            .push(Command::BeginContainer(BeginContainerArgs {
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
            }));
        response
    }

    fn begin_sized_viz_root(&mut self, width: u32, height: u32) -> Response {
        let response = self.begin_viz_root();
        if let Some(Command::BeginContainer(args)) = self.commands.last_mut() {
            args.constraints = Constraints::default().w(width).h(height);
        }
        response
    }

    fn end_viz_root(&mut self) {
        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;
    }
}
