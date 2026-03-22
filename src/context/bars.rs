/// Direction for bar chart rendering.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarDirection {
    /// Bars grow horizontally (default, current behavior).
    Horizontal,
    /// Bars grow vertically from bottom to top.
    Vertical,
}

/// A single bar in a styled bar chart.
#[derive(Debug, Clone)]
pub struct Bar {
    /// Display label for this bar.
    pub label: String,
    /// Numeric value.
    pub value: f64,
    /// Bar color. If None, uses theme.primary.
    pub color: Option<Color>,
    /// Optional text label displayed on the bar.
    pub text_value: Option<String>,
    /// Optional style for the value text.
    pub value_style: Option<Style>,
}

impl Bar {
    /// Create a new bar with a label and value.
    pub fn new(label: impl Into<String>, value: f64) -> Self {
        Self {
            label: label.into(),
            value,
            color: None,
            text_value: None,
            value_style: None,
        }
    }

    /// Set the bar color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set the display text for this bar's value.
    pub fn text_value(mut self, text: impl Into<String>) -> Self {
        self.text_value = Some(text.into());
        self
    }

    /// Set the style for the value text.
    pub fn value_style(mut self, style: Style) -> Self {
        self.value_style = Some(style);
        self
    }
}

/// Configuration for bar chart rendering.
#[derive(Debug, Clone, Copy)]
pub struct BarChartConfig {
    /// Bar direction (horizontal or vertical).
    pub direction: BarDirection,
    /// Width of each bar in cells.
    pub bar_width: u16,
    /// Gap between bars in the same group.
    pub bar_gap: u16,
    /// Gap between bar groups.
    pub group_gap: u16,
    /// Optional maximum value for scaling.
    pub max_value: Option<f64>,
}

impl Default for BarChartConfig {
    fn default() -> Self {
        Self {
            direction: BarDirection::Horizontal,
            bar_width: 1,
            bar_gap: 0,
            group_gap: 2,
            max_value: None,
        }
    }
}

impl BarChartConfig {
    /// Set the bar direction.
    pub fn direction(&mut self, direction: BarDirection) -> &mut Self {
        self.direction = direction;
        self
    }

    /// Set the width of each bar in cells.
    pub fn bar_width(&mut self, bar_width: u16) -> &mut Self {
        self.bar_width = bar_width.max(1);
        self
    }

    /// Set the gap between bars.
    pub fn bar_gap(&mut self, bar_gap: u16) -> &mut Self {
        self.bar_gap = bar_gap;
        self
    }

    /// Set the gap between bar groups.
    pub fn group_gap(&mut self, group_gap: u16) -> &mut Self {
        self.group_gap = group_gap;
        self
    }

    /// Set the maximum value for bar scaling.
    pub fn max_value(&mut self, max_value: f64) -> &mut Self {
        self.max_value = Some(max_value);
        self
    }
}

/// A group of bars rendered together (for grouped bar charts).
#[derive(Debug, Clone)]
pub struct BarGroup {
    /// Group label displayed below the bars.
    pub label: String,
    /// Bars in this group.
    pub bars: Vec<Bar>,
}

impl BarGroup {
    /// Create a new bar group with a label and bars.
    pub fn new(label: impl Into<String>, bars: Vec<Bar>) -> Self {
        Self {
            label: label.into(),
            bars,
        }
    }
}
