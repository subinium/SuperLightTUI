//! Feedback family state — toasts, alert levels, spinners and slider
//! options, plus the gauge response.
//!
//! Rendered by `context/widgets/feedback.rs`.

use super::*;

/// State for toast notification display.
///
/// Add messages with [`ToastState::info`], [`ToastState::success`],
/// [`ToastState::warning`], or [`ToastState::error`], then pass the state to
/// `Context::toast` each frame. Expired messages are removed automatically.
#[derive(Debug, Clone)]
pub struct ToastState {
    /// Active toast messages, ordered oldest-first.
    pub messages: Vec<ToastMessage>,
}

/// A single toast notification message.
#[derive(Debug, Clone)]
pub struct ToastMessage {
    /// The text content of the notification.
    pub text: String,
    /// Severity level, used to choose the display color.
    pub level: ToastLevel,
    /// The tick at which this message was created.
    pub created_tick: u64,
    /// How many ticks the message remains visible.
    pub duration_ticks: u64,
}

impl Default for ToastMessage {
    fn default() -> Self {
        Self {
            text: String::new(),
            level: ToastLevel::Info,
            created_tick: 0,
            duration_ticks: 30,
        }
    }
}

/// Severity level for a [`ToastMessage`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastLevel {
    /// Informational message (primary color).
    Info,
    /// Success message (success color).
    Success,
    /// Warning message (warning color).
    Warning,
    /// Error message (error color).
    Error,
}

/// Severity level for alert widgets.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertLevel {
    /// Informational alert.
    Info,
    /// Success alert.
    Success,
    /// Warning alert.
    Warning,
    /// Error alert.
    Error,
}

impl ToastState {
    /// Create an empty toast state with no messages.
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    /// Push an informational toast visible for 30 ticks.
    pub fn info(&mut self, text: impl Into<String>, tick: u64) {
        self.push(text, ToastLevel::Info, tick, 30);
    }

    /// Push a success toast visible for 30 ticks.
    pub fn success(&mut self, text: impl Into<String>, tick: u64) {
        self.push(text, ToastLevel::Success, tick, 30);
    }

    /// Push a warning toast visible for 50 ticks.
    pub fn warning(&mut self, text: impl Into<String>, tick: u64) {
        self.push(text, ToastLevel::Warning, tick, 50);
    }

    /// Push an error toast visible for 80 ticks.
    pub fn error(&mut self, text: impl Into<String>, tick: u64) {
        self.push(text, ToastLevel::Error, tick, 80);
    }

    /// Push a toast with a custom level and duration.
    pub fn push(
        &mut self,
        text: impl Into<String>,
        level: ToastLevel,
        tick: u64,
        duration_ticks: u64,
    ) {
        self.messages.push(ToastMessage {
            text: text.into(),
            level,
            created_tick: tick,
            duration_ticks,
        });
    }

    /// Remove all messages whose display duration has elapsed.
    ///
    /// Called automatically by `Context::toast` before rendering.
    pub fn cleanup(&mut self, current_tick: u64) {
        self.messages.retain(|message| {
            current_tick < message.created_tick.saturating_add(message.duration_ticks)
        });
    }
}

impl Default for ToastState {
    fn default() -> Self {
        Self::new()
    }
}

/// Named throbber preset for [`SpinnerState`].
///
/// Each variant maps to a fixed frame sequence (parity with the common
/// `cli-spinners` / `ratatui-throbber` sets). Construct a spinner from a preset
/// with [`SpinnerState::preset`], or use the matching named constructor such as
/// [`SpinnerState::moon`].
///
/// # Example
///
/// ```
/// # use slt::widgets::{SpinnerState, SpinnerPreset};
/// let s = SpinnerState::preset(SpinnerPreset::Arrow);
/// assert_eq!(s, SpinnerState::arrow());
/// ```
///
/// Available since `0.21.1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpinnerPreset {
    /// Braille dots: `⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏`.
    Dots,
    /// ASCII line: `| / - \`.
    Line,
    /// Moon phases: `🌑 🌒 🌓 🌔 🌕 🌖 🌗 🌘`.
    Moon,
    /// Bouncing bar between brackets: `(●    )` … `(    ●)` and back.
    Bounce,
    /// Quarter-circle arc: `◜ ◠ ◝ ◞ ◡ ◟`.
    Circle,
    /// Travelling braille dot: `⠁ ⠂ ⠄ ⡀ ⢀ ⠠ ⠐ ⠈`.
    Points,
    /// Half-circle arc: `◜ ◠ ◝ ◞ ◡ ◟`.
    Arc,
    /// Toggle pulse: `⊶ ⊷`.
    Toggle,
    /// Clockwise arrow: `← ↖ ↑ ↗ → ↘ ↓ ↙`.
    Arrow,
}

/// State for an animated spinner widget.
///
/// Create with a named constructor such as [`SpinnerState::dots`] or
/// [`SpinnerState::line`] (or from a [`SpinnerPreset`] via
/// [`SpinnerState::preset`]), then pass to `Context::spinner` each frame. The
/// frame advances automatically with the tick counter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpinnerState {
    chars: &'static [char],
}

static DOTS_CHARS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

static LINE_CHARS: &[char] = &['|', '/', '-', '\\'];

static MOON_CHARS: &[char] = &['🌑', '🌒', '🌓', '🌔', '🌕', '🌖', '🌗', '🌘'];

static BOUNCE_CHARS: &[char] = &['⠁', '⠂', '⠄', '⠂'];

static CIRCLE_CHARS: &[char] = &['◜', '◠', '◝', '◞', '◡', '◟'];

static POINTS_CHARS: &[char] = &['⠁', '⠂', '⠄', '⡀', '⢀', '⠠', '⠐', '⠈'];

static ARC_CHARS: &[char] = &['◜', '◠', '◝', '◞', '◡', '◟'];

static TOGGLE_CHARS: &[char] = &['⊶', '⊷'];

static ARROW_CHARS: &[char] = &['←', '↖', '↑', '↗', '→', '↘', '↓', '↙'];

impl SpinnerState {
    /// Create a dots-style spinner using braille characters.
    ///
    /// Cycles through: `⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏`
    pub fn dots() -> Self {
        Self { chars: DOTS_CHARS }
    }

    /// Create a line-style spinner using ASCII characters.
    ///
    /// Cycles through: `| / - \`
    pub fn line() -> Self {
        Self { chars: LINE_CHARS }
    }

    /// Create a moon-phase spinner.
    ///
    /// Cycles through: `🌑 🌒 🌓 🌔 🌕 🌖 🌗 🌘`
    ///
    /// Available since `0.21.1`.
    pub fn moon() -> Self {
        Self { chars: MOON_CHARS }
    }

    /// Create a bouncing single-dot spinner.
    ///
    /// Cycles through `⠁ ⠂ ⠄ ⠂`, giving a dot that rises and falls in place.
    ///
    /// Available since `0.21.1`.
    pub fn bounce() -> Self {
        Self {
            chars: BOUNCE_CHARS,
        }
    }

    /// Create a quarter-circle arc spinner.
    ///
    /// Cycles through: `◜ ◠ ◝ ◞ ◡ ◟`
    ///
    /// Available since `0.21.1`.
    pub fn circle() -> Self {
        Self {
            chars: CIRCLE_CHARS,
        }
    }

    /// Create a travelling braille-dot ("points") spinner.
    ///
    /// Cycles through: `⠁ ⠂ ⠄ ⡀ ⢀ ⠠ ⠐ ⠈`
    ///
    /// Available since `0.21.1`.
    pub fn points() -> Self {
        Self {
            chars: POINTS_CHARS,
        }
    }

    /// Create a half-circle arc spinner.
    ///
    /// Cycles through: `◜ ◠ ◝ ◞ ◡ ◟`
    ///
    /// Available since `0.21.1`.
    pub fn arc() -> Self {
        Self { chars: ARC_CHARS }
    }

    /// Create a two-frame toggle/pulse spinner.
    ///
    /// Cycles through: `⊶ ⊷`
    ///
    /// Available since `0.21.1`.
    pub fn toggle() -> Self {
        Self {
            chars: TOGGLE_CHARS,
        }
    }

    /// Create a rotating-arrow spinner.
    ///
    /// Cycles clockwise through: `← ↖ ↑ ↗ → ↘ ↓ ↙`
    ///
    /// Available since `0.21.1`.
    pub fn arrow() -> Self {
        Self { chars: ARROW_CHARS }
    }

    /// Create a spinner from a named [`SpinnerPreset`].
    ///
    /// Equivalent to calling the matching named constructor.
    ///
    /// # Example
    ///
    /// ```
    /// # use slt::widgets::{SpinnerState, SpinnerPreset};
    /// let s = SpinnerState::preset(SpinnerPreset::Moon);
    /// assert_eq!(s, SpinnerState::moon());
    /// ```
    ///
    /// Available since `0.21.1`.
    pub fn preset(preset: SpinnerPreset) -> Self {
        match preset {
            SpinnerPreset::Dots => Self::dots(),
            SpinnerPreset::Line => Self::line(),
            SpinnerPreset::Moon => Self::moon(),
            SpinnerPreset::Bounce => Self::bounce(),
            SpinnerPreset::Circle => Self::circle(),
            SpinnerPreset::Points => Self::points(),
            SpinnerPreset::Arc => Self::arc(),
            SpinnerPreset::Toggle => Self::toggle(),
            SpinnerPreset::Arrow => Self::arrow(),
        }
    }

    /// Number of distinct frames in this spinner's cycle.
    ///
    /// Useful for tests and for detecting wrap-around.
    ///
    /// # Example
    ///
    /// ```
    /// # use slt::widgets::SpinnerState;
    /// assert_eq!(SpinnerState::line().frame_count(), 4);
    /// ```
    ///
    /// Available since `0.21.1`.
    pub fn frame_count(&self) -> usize {
        self.chars.len()
    }

    /// Return the spinner character for the given tick.
    pub fn frame(&self, tick: u64) -> char {
        if self.chars.is_empty() {
            return ' ';
        }
        self.chars[tick as usize % self.chars.len()]
    }
}

impl Default for SpinnerState {
    fn default() -> Self {
        Self::dots()
    }
}

/// Optional configuration for [`Context::slider_with`](crate::Context::slider_with).
#[derive(Debug, Clone)]
pub struct SliderOpts {
    pub(crate) label: String,
    pub(crate) range: std::ops::RangeInclusive<f64>,
    pub(crate) step: Option<f64>,
}

impl SliderOpts {
    /// Create slider options with an automatic step of one twentieth of the span.
    pub fn new(label: impl Into<String>, range: std::ops::RangeInclusive<f64>) -> Self {
        Self {
            label: label.into(),
            range,
            step: None,
        }
    }

    /// Set an explicit finite positive keyboard step.
    pub fn step(mut self, step: f64) -> Self {
        self.step = Some(normalize_numeric_step(step));
        self
    }
}

#[cfg(test)]
mod spinner_tests {
    use super::{SpinnerPreset, SpinnerState};

    /// Collect one full cycle of frames for a spinner.
    fn cycle(s: &SpinnerState) -> Vec<char> {
        (0..s.frame_count() as u64).map(|t| s.frame(t)).collect()
    }

    #[test]
    fn existing_presets_unchanged() {
        // dots() and line() must keep their historic sequences.
        assert_eq!(
            cycle(&SpinnerState::dots()),
            vec!['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏']
        );
        assert_eq!(cycle(&SpinnerState::line()), vec!['|', '/', '-', '\\']);
        // Default stays dots().
        assert_eq!(SpinnerState::default(), SpinnerState::dots());
    }

    #[test]
    fn new_presets_have_expected_lengths() {
        assert_eq!(SpinnerState::dots().frame_count(), 10);
        assert_eq!(SpinnerState::line().frame_count(), 4);
        assert_eq!(SpinnerState::moon().frame_count(), 8);
        assert_eq!(SpinnerState::bounce().frame_count(), 4);
        assert_eq!(SpinnerState::circle().frame_count(), 6);
        assert_eq!(SpinnerState::points().frame_count(), 8);
        assert_eq!(SpinnerState::arc().frame_count(), 6);
        assert_eq!(SpinnerState::toggle().frame_count(), 2);
        assert_eq!(SpinnerState::arrow().frame_count(), 8);
    }

    #[test]
    fn new_presets_yield_expected_sequences() {
        assert_eq!(
            cycle(&SpinnerState::moon()),
            vec!['🌑', '🌒', '🌓', '🌔', '🌕', '🌖', '🌗', '🌘']
        );
        assert_eq!(cycle(&SpinnerState::bounce()), vec!['⠁', '⠂', '⠄', '⠂']);
        assert_eq!(
            cycle(&SpinnerState::circle()),
            vec!['◜', '◠', '◝', '◞', '◡', '◟']
        );
        assert_eq!(
            cycle(&SpinnerState::points()),
            vec!['⠁', '⠂', '⠄', '⡀', '⢀', '⠠', '⠐', '⠈']
        );
        assert_eq!(
            cycle(&SpinnerState::arc()),
            vec!['◜', '◠', '◝', '◞', '◡', '◟']
        );
        assert_eq!(cycle(&SpinnerState::toggle()), vec!['⊶', '⊷']);
        assert_eq!(
            cycle(&SpinnerState::arrow()),
            vec!['←', '↖', '↑', '↗', '→', '↘', '↓', '↙']
        );
    }

    #[test]
    fn frame_cycles_modulo_length() {
        let s = SpinnerState::arrow();
        let n = s.frame_count() as u64;
        // Tick 0 and one full revolution later yield the same frame.
        assert_eq!(s.frame(0), s.frame(n));
        assert_eq!(s.frame(1), s.frame(n + 1));
        // Wrap-around at the boundary.
        assert_eq!(s.frame(n - 1), '↙');
        assert_eq!(s.frame(n), '←');
    }

    #[test]
    fn frame_advances_through_sequence() {
        let s = SpinnerState::toggle();
        assert_eq!(s.frame(0), '⊶');
        assert_eq!(s.frame(1), '⊷');
        assert_eq!(s.frame(2), '⊶');
        assert_eq!(s.frame(3), '⊷');
    }

    #[test]
    fn preset_matches_named_constructor() {
        let cases = [
            (SpinnerPreset::Dots, SpinnerState::dots()),
            (SpinnerPreset::Line, SpinnerState::line()),
            (SpinnerPreset::Moon, SpinnerState::moon()),
            (SpinnerPreset::Bounce, SpinnerState::bounce()),
            (SpinnerPreset::Circle, SpinnerState::circle()),
            (SpinnerPreset::Points, SpinnerState::points()),
            (SpinnerPreset::Arc, SpinnerState::arc()),
            (SpinnerPreset::Toggle, SpinnerState::toggle()),
            (SpinnerPreset::Arrow, SpinnerState::arrow()),
        ];
        for (preset, expected) in cases {
            assert_eq!(SpinnerState::preset(preset), expected);
        }
    }

    #[test]
    fn frame_handles_large_tick_without_panicking() {
        // Edge case: very large tick must wrap, not overflow/panic.
        let s = SpinnerState::moon();
        let n = s.frame_count() as u64;
        assert_eq!(s.frame(u64::MAX), s.frame(u64::MAX % n));
    }
}

/// Response from [`Context::gauge`](crate::Context::gauge) and
/// [`Context::line_gauge`](crate::Context::line_gauge).
///
/// Wraps the row-level [`Response`] plus the rendered ratio so callers can
/// confirm the displayed value (clamped to `0.0..=1.0`). Implements
/// `Deref<Target = Response>` so `r.hovered` etc. work directly.
///
/// Note: `ratio` was widened from `f32` to `f64` in v0.20.0 so the gauge
/// family aligns with `animate_value`, chart APIs, and `progress_bar`.
#[derive(Debug, Clone, Default)]
#[must_use = "GaugeResponse contains interaction state — check .hovered or .ratio"]
pub struct GaugeResponse {
    /// The row-level interaction response.
    pub response: Response,
    /// The clamped ratio that was rendered (always `0.0..=1.0`).
    pub ratio: f64,
}

impl std::ops::Deref for GaugeResponse {
    type Target = Response;
    fn deref(&self) -> &Response {
        &self.response
    }
}
