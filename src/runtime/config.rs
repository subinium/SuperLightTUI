//! `RunConfig`: run-loop configuration and its builder methods.

use std::time::Duration;

use crate::{ColorDepth, Theme, style};

/// Configuration for a TUI run loop.
///
/// Pass to [`run_with`](crate::run_with) or [`run_inline_with`](crate::run_inline_with) to customize behavior.
/// Use [`Default::default()`] for sensible defaults (16ms tick / 60fps, no mouse, dark theme).
/// This type is `#[non_exhaustive]`, so prefer builder methods instead of struct literals.
///
/// # Example
///
/// ```no_run
/// use slt::{RunConfig, Theme};
/// use std::time::Duration;
///
/// let config = RunConfig::default()
///     .tick_rate(Duration::from_millis(50))
///     .mouse(true)
///     .theme(Theme::light())
///     .max_fps(60);
/// ```
#[non_exhaustive]
#[must_use = "configure loop behavior before passing to run_with or run_inline_with"]
pub struct RunConfig {
    /// How long to wait for input before triggering a tick with no events.
    ///
    /// Lower values give smoother animations at the cost of more CPU usage.
    /// Defaults to 16ms (60fps).
    pub tick_rate: Duration,
    /// Whether to enable mouse event reporting.
    ///
    /// When `true`, the terminal captures mouse clicks, scrolls, and movement.
    /// Defaults to `false`.
    pub mouse: bool,
    /// Whether to enable the Kitty keyboard protocol for enhanced input.
    ///
    /// When `true`, enables disambiguated key events, key release events,
    /// and modifier-only key reporting on supporting terminals (kitty, Ghostty, WezTerm).
    /// Terminals that don't support it silently ignore the request.
    /// Defaults to `false`.
    pub kitty_keyboard: bool,
    /// Whether to request modifier-only key events (bare Ctrl/Shift/Alt/Super
    /// presses and releases, with no accompanying character).
    ///
    /// Has **no effect** unless [`kitty_keyboard`](Self::kitty_keyboard) is also
    /// `true`: it OR-es the Kitty `REPORT_ALL_KEYS_AS_ESCAPE_CODES`
    /// progressive-enhancement flag into the pushed flag set. On supporting
    /// terminals (kitty, Ghostty, WezTerm) this makes bare modifier presses
    /// arrive as [`KeyCode::Modifier`](crate::KeyCode::Modifier) events; other terminals never emit them.
    ///
    /// Kept opt-in to avoid flooding apps with modifier events they don't want.
    /// Defaults to `false`.
    ///
    /// Since 0.21.0.
    pub report_all_keys: bool,
    /// The color theme applied to all widgets automatically.
    ///
    /// Defaults to [`Theme::dark()`].
    pub theme: Theme,
    /// Color depth override.
    ///
    /// `None` means auto-detect from `$COLORTERM` and `$TERM` environment
    /// variables. Set explicitly to force a specific color depth regardless
    /// of terminal capabilities.
    pub color_depth: Option<ColorDepth>,
    /// Optional maximum frame rate.
    ///
    /// `None` means unlimited frame rate. `Some(fps)` sleeps at the end of each
    /// loop iteration to target that frame time.
    pub max_fps: Option<u32>,
    /// Lines scrolled per mouse scroll event. Defaults to 1.
    pub scroll_speed: u32,
    /// Optional terminal window title (set via OSC 2).
    pub title: Option<String>,
    /// Default colors applied to all instances of each widget type.
    ///
    /// Per-callsite `_colored()` overrides still take precedence.
    /// Defaults to all-`None` (use theme colors).
    pub widget_theme: style::WidgetTheme,
    /// Whether the runtime intercepts Ctrl+C and exits the loop cleanly.
    ///
    /// When `true` (the default), Ctrl+C is treated as a quit signal —
    /// matching the v0.19 behavior. When `false`, the Ctrl+C key event flows
    /// through to the frame closure as a regular [`Event::Key`](crate::Event::Key), matching
    /// RataTUI's raw-mode semantics. The user is then responsible for
    /// deciding whether to call [`Context::quit`](crate::Context::quit) or treat it as any other
    /// shortcut (e.g. clear input, cancel current operation).
    ///
    /// Set this to `false` when migrating code from RataTUI that already
    /// handles Ctrl+C explicitly, or when implementing a graceful-shutdown
    /// prompt (e.g. "save unsaved changes?").
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::{KeyCode, KeyModifiers, RunConfig};
    /// slt::run_with(RunConfig::default().handle_ctrl_c(false), |ui| {
    ///     // Ctrl+C now reaches your closure as a normal key event.
    ///     if ui.key_mod('c', KeyModifiers::CONTROL) {
    ///         // Decide what to do — clear input, prompt to save, quit, etc.
    ///         ui.quit();
    ///     }
    /// }).unwrap();
    /// ```
    pub handle_ctrl_c: bool,
    /// Whether the runtime restores the terminal on Ctrl+Z (`SIGTSTP`) and
    /// re-enters it on resume (`SIGCONT`).
    ///
    /// When `true` (the default) on Unix, pressing Ctrl+Z runs the full
    /// session teardown — leave the alternate screen (fullscreen only), show
    /// the cursor, disable raw mode / bracketed paste / focus / mouse / kitty
    /// — *before* the process is suspended, so the shell prompt returns to a
    /// clean terminal. Resuming with `fg` re-enters the same session and forces
    /// a full redraw. This matches helix/zellij/bubbletea job-control behavior.
    ///
    /// When `false`, no signal handler is installed and Ctrl+Z falls through to
    /// crossterm as a regular key event in raw mode (the pre-0.21 behavior).
    ///
    /// Unix only; ignored on Windows, WASM, and non-`crossterm` builds where
    /// there is no `SIGTSTP`. Defaults to `true`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::RunConfig;
    /// // Opt out: let Ctrl+Z reach the frame closure as a key event.
    /// let cfg = RunConfig::default().handle_suspend(false);
    /// assert!(!cfg.handle_suspend);
    /// ```
    pub handle_suspend: bool,
    /// Maximum number of external async messages delivered to one frame.
    ///
    /// Messages beyond this count remain queued in FIFO order for later
    /// frames. Defaults to the async channel capacity (100).
    pub async_message_budget: usize,
    /// Whether [`Response::clicked`](crate::Response::clicked) fires on the
    /// mouse release instead of the press.
    ///
    /// `false` (the default) activates on the press, matching SLT before
    /// v0.26. `true` activates on the frame the left button is released inside
    /// the same slot that was pressed, so dragging off a button before letting
    /// go cancels the click, like the browser `click` event. Keyboard
    /// activation is unaffected. Since 0.26.0.
    pub click_on_release: bool,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            tick_rate: Duration::from_millis(16),
            mouse: false,
            kitty_keyboard: false,
            report_all_keys: false,
            theme: Theme::dark(),
            color_depth: None,
            max_fps: Some(60),
            scroll_speed: 1,
            title: None,
            widget_theme: style::WidgetTheme::new(),
            handle_ctrl_c: true,
            handle_suspend: true,
            async_message_budget: 100,
            click_on_release: false,
        }
    }
}

impl RunConfig {
    /// Set the tick rate (input polling interval).
    pub fn tick_rate(mut self, rate: Duration) -> Self {
        self.tick_rate = rate;
        self
    }

    /// Enable or disable mouse event reporting.
    pub fn mouse(mut self, enabled: bool) -> Self {
        self.mouse = enabled;
        self
    }

    /// Enable or disable Kitty keyboard protocol.
    pub fn kitty_keyboard(mut self, enabled: bool) -> Self {
        self.kitty_keyboard = enabled;
        self
    }

    /// Enable or disable modifier-only key reporting (Kitty
    /// `REPORT_ALL_KEYS_AS_ESCAPE_CODES`).
    ///
    /// Requires [`kitty_keyboard(true)`](Self::kitty_keyboard) to have any
    /// effect. When enabled on a supporting terminal, bare modifier presses
    /// and releases arrive as [`KeyCode::Modifier`](crate::KeyCode::Modifier) events. Defaults to
    /// `false`.
    ///
    /// Since 0.21.0.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::RunConfig;
    /// let cfg = RunConfig::default().kitty_keyboard(true).report_all_keys(true);
    /// assert!(cfg.report_all_keys);
    /// ```
    pub fn report_all_keys(mut self, enabled: bool) -> Self {
        self.report_all_keys = enabled;
        self
    }

    /// Set the color theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Override the color depth.
    pub fn color_depth(mut self, depth: ColorDepth) -> Self {
        self.color_depth = Some(depth);
        self
    }

    /// Set the maximum frame rate.
    pub fn max_fps(mut self, fps: u32) -> Self {
        self.max_fps = Some(fps);
        self
    }

    /// Disable the frame rate cap (unlimited FPS).
    ///
    /// By default, [`RunConfig`] caps rendering at 60 fps. Call this to remove
    /// the cap entirely — useful when controlling external sleep/vsync.
    ///
    /// # Example
    ///
    /// ```no_run
    /// slt::run_with(
    ///     slt::RunConfig::default().no_fps_cap(),
    ///     |ui| { ui.text("uncapped"); },
    /// ).unwrap();
    /// ```
    pub fn no_fps_cap(mut self) -> Self {
        self.max_fps = None;
        self
    }

    /// Set the maximum number of external async messages delivered per frame.
    ///
    /// Values below one are normalized to one so queued messages always make
    /// progress.
    pub fn async_message_budget(mut self, messages: usize) -> Self {
        self.async_message_budget = messages.max(1);
        self
    }

    /// Set the scroll speed (lines per scroll event).
    pub fn scroll_speed(mut self, lines: u32) -> Self {
        self.scroll_speed = lines.max(1);
        self
    }

    /// Set the terminal window title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set default widget colors for all widget types.
    pub fn widget_theme(mut self, widget_theme: style::WidgetTheme) -> Self {
        self.widget_theme = widget_theme;
        self
    }

    /// Configure whether the runtime auto-exits on Ctrl+C.
    ///
    /// Defaults to `true` (current v0.19 behavior). Set to `false` to
    /// receive Ctrl+C as a regular [`Event::Key`](crate::Event::Key) inside the frame closure
    /// — see [`RunConfig::handle_ctrl_c`] for the full migration story.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::RunConfig;
    /// let cfg = RunConfig::default().handle_ctrl_c(false);
    /// assert!(!cfg.handle_ctrl_c);
    /// ```
    pub fn handle_ctrl_c(mut self, enabled: bool) -> Self {
        self.handle_ctrl_c = enabled;
        self
    }

    /// Configure whether the runtime restores the terminal on Ctrl+Z
    /// (`SIGTSTP`) and re-enters it on resume (`SIGCONT`).
    ///
    /// Defaults to `true`. Set to `false` to disable the suspend handler so
    /// Ctrl+Z falls through to crossterm as a regular key event — see
    /// [`RunConfig::handle_suspend`] for the full behavior. Unix only; ignored
    /// elsewhere.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::RunConfig;
    /// let cfg = RunConfig::default().handle_suspend(false);
    /// assert!(!cfg.handle_suspend);
    /// ```
    pub fn handle_suspend(mut self, enabled: bool) -> Self {
        self.handle_suspend = enabled;
        self
    }

    /// Choose whether mouse clicks activate on release instead of press.
    ///
    /// See [`RunConfig::click_on_release`]. With `true`, a press on a button
    /// followed by a release outside it does not report `clicked`; a press and
    /// release inside reports `clicked` on the release frame. Since 0.26.0.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::RunConfig;
    /// let cfg = RunConfig::default().click_on_release(true);
    /// assert!(cfg.click_on_release);
    /// ```
    pub fn click_on_release(mut self, enabled: bool) -> Self {
        self.click_on_release = enabled;
        self
    }
}
