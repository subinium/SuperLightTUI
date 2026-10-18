//! `RunConfig`: run-loop configuration and its builder methods.

use std::time::Duration;

use crate::{ColorDepth, Theme, style};

/// How much pointer activity the terminal reports to the app.
///
/// Each mode asks the terminal for a different DEC mouse-tracking level, so
/// apps only pay for the pointer traffic they use. SGR encoding (`?1006`) is
/// always requested; the legacy urxvt encoding (`?1015`) is never used.
///
/// | Mode | DEC modes | Delivers |
/// |---|---|---|
/// | `Off` | none | nothing; the terminal's native selection keeps working |
/// | `Click` | `?1000h ?1006h` | presses, releases and the wheel |
/// | `Drag` | `?1000h ?1002h ?1006h` | plus motion while a button is held |
/// | `Hover` | `?1000h ?1003h ?1006h` | plus all motion (hover, tooltips) |
///
/// The default ([`RunConfig::default`]) is `Drag` locally and `Click` in an
/// SSH or mosh session; see [`RunConfig::mouse_mode`]. While any mode other
/// than `Off` is active the terminal's own text selection is replaced by
/// SLT's built-in selection; most emulators bypass mouse capture while Shift
/// is held (Shift-drag selects natively).
///
/// # Example
///
/// ```no_run
/// use slt::{MouseMode, RunConfig};
/// // Hover styling and tooltips need every pointer move.
/// let config = RunConfig::default().mouse_mode(MouseMode::Hover);
/// assert_eq!(config.effective_mouse_mode(), MouseMode::Hover);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseMode {
    /// No mouse reporting.
    Off,
    /// Button presses, releases and wheel events.
    Click,
    /// `Click` plus motion while a button is held: drags, split handles,
    /// scrollbar thumbs and the built-in text selection.
    Drag,
    /// `Drag` plus every pointer move: hover styling and tooltips. Every
    /// move produces an event and a frame, which costs the most over SSH.
    Hover,
}

impl From<bool> for MouseMode {
    /// `true` maps to [`MouseMode::Hover`] (the pre-0.26 `mouse(true)`
    /// behavior), `false` to [`MouseMode::Off`].
    fn from(enabled: bool) -> Self {
        if enabled { Self::Hover } else { Self::Off }
    }
}

impl MouseMode {
    /// Whether this mode delivers `event`. Used as an input filter on
    /// platforms where the terminal cannot be asked for a reporting level.
    pub(crate) fn admits(self, event: &crate::Event) -> bool {
        use crate::event::MouseKind;
        let crate::Event::Mouse(mouse) = event else {
            return true;
        };
        match self {
            Self::Off => false,
            Self::Click => !matches!(mouse.kind, MouseKind::Moved | MouseKind::Drag(_)),
            Self::Drag => !matches!(mouse.kind, MouseKind::Moved),
            Self::Hover => true,
        }
    }
}

/// Resolve the reporting mode: an explicit mode always wins; the default is
/// `Drag` locally and `Click` in a remote (SSH/mosh) session.
pub(crate) fn resolve_mouse_mode(explicit: Option<MouseMode>, remote: bool) -> MouseMode {
    match explicit {
        Some(mode) => mode,
        None if remote => MouseMode::Click,
        None => MouseMode::Drag,
    }
}

/// Configuration for a TUI run loop.
///
/// Pass to [`run_with`](crate::run_with) or [`run_inline_with`](crate::run_inline_with) to customize behavior.
/// Use [`Default::default()`] for sensible defaults (16ms tick / 60fps, mouse
/// clicks and drags on, dark theme).
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
///     .mouse_mode(slt::MouseMode::Hover)
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
    /// Mouse reporting mode, or `None` for the automatic default.
    ///
    /// `None` (the default) resolves to [`MouseMode::Drag`] locally and to
    /// [`MouseMode::Click`] when an SSH or mosh session is detected, so
    /// clicks, the wheel, drag handles and text selection work out of the box
    /// without flooding a remote link with motion events. Set an explicit mode
    /// with [`mouse_mode`](Self::mouse_mode) (or the [`mouse`](Self::mouse)
    /// shorthand); an explicit mode is never overridden. Hover styling and
    /// tooltips need [`MouseMode::Hover`]. Since 0.26.0 (replaces the
    /// `mouse: bool` field).
    pub mouse_mode: Option<MouseMode>,
    /// Whether to enable the Kitty keyboard protocol for enhanced input.
    ///
    /// When `true`, enables disambiguated key events, key release events,
    /// and modifier-only key reporting on supporting terminals (kitty, Ghostty, WezTerm).
    ///
    /// On a directly attached terminal that answers startup queries, SLT asks
    /// for support with `CSI ? u` first and pushes the protocol only when the
    /// terminal confirms it ([`Capabilities::kitty_keyboard`](crate::Capabilities::kitty_keyboard)).
    /// Where startup queries are skipped (Windows, zellij, unidentified PTYs)
    /// the push follows the multiplexer policy alone; tmux and GNU screen never
    /// receive it. `SLT_FORCE_KITTY_KEYBOARD=1` pushes regardless of the probe
    /// and `SLT_DISABLE_KITTY_KEYBOARD=1` never pushes.
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
    /// Whether SLT's built-in text selection is active while the mouse is
    /// captured.
    ///
    /// When `true` (the default), dragging across text highlights it and the
    /// release copies it to the clipboard over OSC 52. Drags that a widget
    /// handles (split handles, scrollbar thumbs, anything that consumes the
    /// press) never start a selection. Set to `false` for apps with their own
    /// drag or selection model; nothing is highlighted or copied then. The
    /// terminal's native selection is still reachable with the emulator's
    /// bypass modifier (usually Shift-drag). Since 0.26.0.
    ///
    /// Any rendered text is selectable, including text outside containers. A
    /// selection that starts inside a [`scrollable`](crate::Context::scrollable)
    /// stays attached to its text while the region scrolls, and holding the
    /// pointer on or beyond the region's top or bottom row scrolls it one step
    /// per frame. Only the part of the selection that is visible when the
    /// button is released is copied; rows scrolled out of view are not
    /// retained.
    pub text_selection: bool,
    /// How long the pointer must rest on a widget before its hover tooltip
    /// ([`Context::tooltip`](crate::Context::tooltip),
    /// [`Response::on_hover`](crate::Response::on_hover)) appears.
    ///
    /// Defaults to 500 ms, like desktop and browser tooltips; moving to
    /// another widget restarts the wait. `Duration::ZERO` shows tooltips on
    /// the first hovered frame (the pre-0.26 behavior). The tooltip appears on
    /// the first frame after the delay, without further input. Since 0.26.0.
    pub tooltip_delay: Duration,
    /// Clear the hover state after the pointer has not moved for this long.
    ///
    /// Terminals report no "pointer left the window" event, so the last
    /// hovered widget would otherwise stay highlighted (and its tooltip open)
    /// until the next mouse event, focus loss, or resize. `None` (the default)
    /// keeps hover until then. Since 0.26.0.
    pub hover_timeout: Option<Duration>,
    /// Whether [`ContainerBuilder::pointer_shape`](crate::ContainerBuilder::pointer_shape)
    /// requests are written as OSC 22 pointer-shape escapes.
    ///
    /// Defaults to `false` because terminals without OSC 22 support may print
    /// the payload. Enable it for terminals known to support it (kitty, foot,
    /// Ghostty, recent xterm). The default pointer is restored on exit, in the
    /// panic hook and on suspend. Since 0.26.0.
    pub pointer_shapes: bool,
    /// Wrap `OSC 52` clipboard writes for tmux / GNU screen passthrough.
    ///
    /// See [`clipboard_passthrough`](Self::clipboard_passthrough). Defaults to
    /// `false`.
    pub clipboard_passthrough: bool,
    /// Largest UTF-8 payload, in bytes, sent through `OSC 52`.
    ///
    /// See [`clipboard_max_bytes`](Self::clipboard_max_bytes). Defaults to
    /// [`RunConfig::DEFAULT_CLIPBOARD_MAX_BYTES`].
    pub clipboard_max_bytes: usize,
    /// Treat East Asian Ambiguous characters (`→`, `±`, `①`) as two columns.
    ///
    /// See [`ambiguous_wide`](Self::ambiguous_wide). Defaults to `false`.
    pub ambiguous_wide: bool,
    /// Width-conservative flushing for emoji clusters terminals measure
    /// differently. `None` (the default) enables it automatically under
    /// tmux, GNU screen and zellij, or after a width probe mismatch.
    ///
    /// See [`width_conservative`](Self::width_conservative).
    pub width_conservative: Option<bool>,
    /// Measure representative emoji clusters with a startup cursor-position
    /// probe. See [`width_probe`](Self::width_probe). Defaults to `false`.
    pub width_probe: bool,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            tick_rate: Duration::from_millis(16),
            mouse_mode: None,
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
            text_selection: true,
            tooltip_delay: Duration::from_millis(500),
            hover_timeout: None,
            pointer_shapes: false,
            clipboard_passthrough: false,
            clipboard_max_bytes: Self::DEFAULT_CLIPBOARD_MAX_BYTES,
            ambiguous_wide: false,
            width_conservative: None,
            width_probe: false,
        }
    }
}

impl RunConfig {
    /// Default [`clipboard_max_bytes`](Self::clipboard_max_bytes): 74,994
    /// bytes, which keeps the base64 payload under 100,000 bytes, the limit
    /// hterm and common OSC 52 helpers enforce.
    pub const DEFAULT_CLIPBOARD_MAX_BYTES: usize = 74_994;

    /// Set the tick rate (input polling interval).
    pub fn tick_rate(mut self, rate: Duration) -> Self {
        self.tick_rate = rate;
        self
    }

    /// Shorthand for [`mouse_mode`](Self::mouse_mode): `true` selects
    /// [`MouseMode::Hover`] (every pointer move, as before 0.26) and `false`
    /// selects [`MouseMode::Off`].
    ///
    /// Mouse reporting is on by default since 0.26 ([`MouseMode::Drag`]
    /// locally), so `mouse(true)` is only needed for hover, and `mouse(false)`
    /// restores the terminal's native selection.
    pub fn mouse(mut self, enabled: bool) -> Self {
        self.mouse_mode = Some(MouseMode::from(enabled));
        self
    }

    /// Choose the mouse reporting mode explicitly. See [`MouseMode`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::{MouseMode, RunConfig};
    /// // Clicks and the wheel only, the cheapest mode over SSH.
    /// let config = RunConfig::default().mouse_mode(MouseMode::Click);
    /// assert_eq!(config.mouse_mode, Some(MouseMode::Click));
    /// ```
    pub fn mouse_mode(mut self, mode: MouseMode) -> Self {
        self.mouse_mode = Some(mode);
        self
    }

    /// The mode a run loop will request: the explicit
    /// [`mouse_mode`](Self::mouse_mode), else [`MouseMode::Drag`] locally and
    /// [`MouseMode::Click`] in an SSH or mosh session.
    pub fn effective_mouse_mode(&self) -> MouseMode {
        // The run loops ask on every input poll; the environment is read once.
        static REMOTE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        resolve_mouse_mode(
            self.mouse_mode,
            *REMOTE.get_or_init(crate::context::session_is_remote),
        )
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

    /// Enable or disable SLT's built-in text selection and copy-on-release.
    ///
    /// See [`RunConfig::text_selection`]. Since 0.26.0.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::RunConfig;
    /// let cfg = RunConfig::default().text_selection(false);
    /// assert!(!cfg.text_selection);
    /// ```
    pub fn text_selection(mut self, enabled: bool) -> Self {
        self.text_selection = enabled;
        self
    }

    /// Set how long the pointer must rest before a hover tooltip appears.
    ///
    /// See [`RunConfig::tooltip_delay`]; `Duration::ZERO` shows tooltips
    /// immediately. Since 0.26.0.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::RunConfig;
    /// use std::time::Duration;
    /// let cfg = RunConfig::default().tooltip_delay(Duration::from_millis(250));
    /// assert_eq!(cfg.tooltip_delay, Duration::from_millis(250));
    /// ```
    pub fn tooltip_delay(mut self, delay: Duration) -> Self {
        self.tooltip_delay = delay;
        self
    }

    /// Clear the hover state after `timeout` without pointer motion, or never
    /// with `None`. See [`RunConfig::hover_timeout`]. Since 0.26.0.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::RunConfig;
    /// use std::time::Duration;
    /// let cfg = RunConfig::default().hover_timeout(Some(Duration::from_secs(3)));
    /// assert_eq!(cfg.hover_timeout, Some(Duration::from_secs(3)));
    /// ```
    pub fn hover_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.hover_timeout = timeout;
        self
    }

    /// Opt in to OSC 22 pointer-shape escapes. See
    /// [`RunConfig::pointer_shapes`]. Since 0.26.0.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::RunConfig;
    /// let cfg = RunConfig::default().pointer_shapes(true);
    /// assert!(cfg.pointer_shapes);
    /// ```
    pub fn pointer_shapes(mut self, enabled: bool) -> Self {
        self.pointer_shapes = enabled;
        self
    }

    /// Wrap `OSC 52` clipboard writes so they pass through a multiplexer.
    ///
    /// Inside tmux the sequence is sent as `ESC P tmux; … ESC \` with every
    /// inner `ESC` doubled, which requires `set -g allow-passthrough on`.
    /// Inside GNU screen it is split into DCS chunks. Elsewhere the bytes are
    /// unchanged. Off by default: passthrough bypasses the multiplexer's own
    /// clipboard handling and its security settings. Without it, tmux needs
    /// `set -g set-clipboard on` to forward copies to the outer terminal.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let cfg = slt::RunConfig::default().clipboard_passthrough(true);
    /// assert!(cfg.clipboard_passthrough);
    /// ```
    pub fn clipboard_passthrough(mut self, enabled: bool) -> Self {
        self.clipboard_passthrough = enabled;
        self
    }

    /// Set the largest UTF-8 payload, in bytes, sent through `OSC 52`.
    ///
    /// Larger copies are not written and surface as
    /// [`ClipboardStatus::TooLarge`](crate::ClipboardStatus::TooLarge) through
    /// [`Context::clipboard_status`](crate::Context::clipboard_status).
    ///
    /// # Example
    ///
    /// ```no_run
    /// let cfg = slt::RunConfig::default().clipboard_max_bytes(1 << 20);
    /// assert_eq!(cfg.clipboard_max_bytes, 1 << 20);
    /// ```
    pub fn clipboard_max_bytes(mut self, bytes: usize) -> Self {
        self.clipboard_max_bytes = bytes;
        self
    }

    /// Treat East Asian Ambiguous characters as two columns wide.
    ///
    /// Match this to a CJK-locale terminal configured with "ambiguous width =
    /// wide" so `→±①` lays out as six columns in the buffer, wrapping and
    /// truncation. Defaults to `false` (one column, the Unicode default).
    ///
    /// # Example
    ///
    /// ```no_run
    /// let cfg = slt::RunConfig::default().ambiguous_wide(true);
    /// assert!(cfg.ambiguous_wide);
    /// ```
    pub fn ambiguous_wide(mut self, enabled: bool) -> Self {
        self.ambiguous_wide = enabled;
        self
    }

    /// Force width-conservative flushing on or off.
    ///
    /// In conservative mode the flush positions the cursor explicitly after
    /// every width-uncertain cluster (ZWJ sequences, skin-tone modifiers,
    /// flags, VS16 and other non-BMP emoji) so a terminal that advances by a
    /// different amount cannot shift the rest of the row. By default it is on
    /// under tmux, GNU screen and zellij and after a
    /// [`width_probe`](Self::width_probe) mismatch, and off otherwise.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let cfg = slt::RunConfig::default().width_conservative(true);
    /// assert_eq!(cfg.width_conservative, Some(true));
    /// ```
    pub fn width_conservative(mut self, enabled: bool) -> Self {
        self.width_conservative = Some(enabled);
        self
    }

    /// Measure emoji cluster widths at startup.
    ///
    /// Fullscreen runs on a directly attached terminal that answers startup
    /// queries print a VS16, ZWJ, skin-tone and flag sample on the alternate
    /// screen, read the cursor position (`CSI 6 n`) after each, and erase the
    /// row. A class whose advance differs from SLT's measurement enables
    /// width-conservative flushing. Skipped on Windows, under multiplexers,
    /// in inline mode and when queries are disabled. Defaults to `false`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let cfg = slt::RunConfig::default().width_probe(true);
    /// assert!(cfg.width_probe);
    /// ```
    pub fn width_probe(mut self, enabled: bool) -> Self {
        self.width_probe = enabled;
        self
    }
}
