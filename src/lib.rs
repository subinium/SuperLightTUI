//! SuperLightTUI — an immediate-mode flexbox-layout terminal UI library.
//!
//! Build a TUI as easily as a web page: write a closure, SLT calls it
//! every frame. State lives in your code; layout is described every
//! frame; styling uses Tailwind-inspired shorthand; focus and events are
//! threaded through a single [`Context`] parameter.
//!
//! See `docs/QUICK_START.md` for a 5-minute introduction and
//! `docs/DESIGN_PRINCIPLES.md` for the principles every public API
//! follows.
//!
//! # Example
//!
//! ```no_run
//! fn main() -> std::io::Result<()> {
//!     slt::run(|ui| {
//!         ui.text("hello, world");
//!     })
//! }
//! ```

// Safety
#![forbid(unsafe_code)]
// Documentation
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(rustdoc::broken_intra_doc_links)]
#![warn(missing_docs)]
#![warn(rustdoc::private_intra_doc_links)]
// Correctness
#![deny(clippy::unwrap_in_result)]
#![warn(clippy::unwrap_used)]
// Library hygiene — a library must not write to stdout/stderr
#![warn(clippy::dbg_macro)]
#![warn(clippy::print_stdout)]
#![warn(clippy::print_stderr)]

//! # SLT — Super Light TUI
//!
//! Immediate-mode terminal UI for Rust. Small core. Zero `unsafe`.
//!
//! SLT gives you an egui-style API for terminals: your closure runs each frame,
//! you describe your UI, and SLT handles layout, diffing, and rendering.
//!
//! ## Quick Start
//!
//! ```no_run
//! fn main() -> std::io::Result<()> {
//!     slt::run(|ui| {
//!         ui.text("hello, world");
//!     })
//! }
//! ```
//!
//! ## Features
//!
//! - **Flexbox layout** — `row()`, `col()`, `gap()`, `grow()`
//! - **50+ built-in widgets** — input, textarea, table, list, tabs, button, checkbox, toggle, spinner, progress, toast, slider, separator, help bar, scrollable, chart, bar chart, stacked bar chart, sparkline, histogram, heatmap, treemap, candlestick, canvas, grid, select, radio, multi-select, tree, virtual list, command palette, markdown, alert, badge, stat, breadcrumb, accordion, code block, big text, image, modal, tooltip, form, calendar, file picker, qr code
//! - **Styling** — bold, italic, dim, underline, 256 colors, RGB
//! - **Mouse** — click, hover, drag-to-scroll
//! - **Focus** — automatic Tab/Shift+Tab cycling
//! - **Theming** — 10 presets, semantic tokens (`ThemeColor`), spacing scale, contrast helpers
//! - **Animation** — tween and spring primitives with 9 easing functions
//! - **Inline mode** — render below your prompt, no alternate screen
//! - **Async** — optional tokio integration via `async` feature
//! - **Layout debugger** — F12 to visualize container bounds
//!
//! ## Feature Flags
//!
//! | Flag | Description |
//! |------|-------------|
//! | `crossterm` | Built-in terminal runtime (`run`, `run_inline`, clipboard query helpers). Enabled by default. |
//! | `bidi` | Reorder right-to-left text (Hebrew, Arabic, …) to visual order per UAX #9 before rendering. Enabled by default; pure-LTR text takes a zero-cost fast path. Since 0.21.0. |
//! | `async` | Enable `run_async()` with tokio channel-based message passing |
//! | `serde` | Enable Serialize/Deserialize for Style, Color, Theme, and layout types |
//! | `image` | Enable image-loading helpers for terminal image widgets |
//! | `qrcode` | Enable `ui.qr_code(...)` |
//! | `syntax` / `syntax-*` | Enable tree-sitter syntax highlighting |
//!
//! ## Learn More
//!
//! - Guides index: <https://github.com/subinium/SuperLightTUI/blob/main/docs/README.md>
//! - Quick start: <https://github.com/subinium/SuperLightTUI/blob/main/docs/QUICK_START.md>
//! - Backends and run loops: <https://github.com/subinium/SuperLightTUI/blob/main/docs/BACKENDS.md>
//! - Testing: <https://github.com/subinium/SuperLightTUI/blob/main/docs/TESTING.md>
//! - Debugging: <https://github.com/subinium/SuperLightTUI/blob/main/docs/DEBUGGING.md>

/// Animation primitives: tween, spring, keyframes, sequence, stagger.
pub mod anim;
/// Double-buffered cell grid with clip stack and diff tracking.
pub mod buffer;
/// Terminal cell representation.
pub mod cell;
/// Chart and data visualization widgets.
pub mod chart;
/// UI context, container builder, and widget rendering.
pub mod context;
/// Input events (keyboard, mouse, resize, paste).
pub mod event;
/// Half-block image rendering.
pub mod halfblock;
/// Keyboard shortcut mapping.
pub mod keymap;
/// Flexbox layout engine and command tree.
pub mod layout;
/// Color palettes (Tailwind-style).
pub mod palette;
/// Rectangular region type used throughout SLT layout.
pub mod rect;
#[cfg(feature = "crossterm")]
mod sixel;
/// Styling: colors, borders, padding, margins, themes, constraints.
pub mod style;
/// Tree-sitter syntax highlighting integration.
pub mod syntax;
#[cfg(feature = "crossterm")]
mod terminal;
/// Headless test utilities for unit-testing TUI closures.
pub mod test_utils;
/// Widget state types (list, table, input, select, etc.).
pub mod widgets;

use std::io;
#[cfg(feature = "crossterm")]
use std::io::IsTerminal;
#[cfg(feature = "crossterm")]
use std::io::Write;
#[cfg(feature = "crossterm")]
use std::sync::Once;
use std::time::{Duration, Instant};

#[doc(hidden)]
pub use layout::__bench_dim_buffer_around;
#[doc(hidden)]
pub use layout::__bench_wrap_segments;
#[cfg(feature = "crossterm")]
#[doc(hidden)]
pub use terminal::__bench_flush_buffer_diff;
#[cfg(feature = "crossterm")]
#[doc(hidden)]
pub use terminal::__bench_flush_buffer_diff_mut;
#[cfg(feature = "crossterm")]
#[doc(hidden)]
pub use terminal::{__BenchKittyFixture, __bench_new_kitty_fixture};
#[cfg(feature = "crossterm")]
pub use terminal::{detect_color_scheme, read_clipboard, ColorScheme};
#[cfg(feature = "crossterm")]
use terminal::{InlineTerminal, Terminal};

pub use crate::test_utils::{EventBuilder, FrameRecord, TestBackend, TestSequence};
// Animation primitives (builder types) are re-exported at crate root for
// ergonomic `use slt::{Tween, Spring, ...}`. The easing functions and `lerp`
// live under `slt::anim::*` — they are rarely imported in isolation and
// keeping them out of the root shrinks the top-level surface.
pub use anim::{Keyframes, LoopMode, Sequence, Spring, Stagger, Tween};
pub use buffer::Buffer;
pub use cell::Cell;
// Chart user-facing types at crate root; internals (`ChartRenderer`,
// `RenderedLine`, `ColorSpan`, `DatasetEntry`, `HistogramBuilder`,
// `GraphType`, `Axis`) live under `slt::chart::*`.
pub use chart::{Candle, ChartBuilder, ChartConfig, Dataset, LegendPosition, Marker};
pub use context::{
    Anchor, Bar, BarChartConfig, BarDirection, BarGroup, Breadcrumb, CanvasContext,
    ContainerBuilder, Context, Gauge, GutterOpts, LineGauge, Response, State, TreemapItem, Widget,
};
pub use event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, ModifierKey, MouseButton, MouseEvent,
    MouseKind,
};
pub use halfblock::HalfBlockImage;
pub use keymap::{Binding, KeyMap, PublishedKeymap, WidgetKeyHelp};
pub use layout::Direction;
pub use palette::Palette;
pub use rect::Rect;
pub use style::{
    Align, Border, BorderSides, Breakpoint, Color, ColorDepth, Constraints, ContainerStyle,
    HeightSpec, Justify, Margin, Modifiers, Padding, Spacing, Style, Theme, ThemeBuilder,
    ThemeColor, WidgetColors, WidgetTheme, WidthSpec,
};
pub use widgets::{
    AlertLevel, ApprovalAction, BreadcrumbResponse, ButtonVariant, CalDate, CalendarSelect,
    CalendarState, ChordState, ColorPickerState, CommandPaletteState, ContextItem,
    DirectoryTreeState, FileEntry, FilePickerState, FormField, FormState, GaugeResponse,
    GridColumn, GutterResponse, HighlightRange, ListState, ModeState, MultiSelectState,
    PaginatorState, PaginatorStyle, PaletteCommand, PickerMode, RadioState, RichLogEntry,
    RichLogState, ScreenState, ScrollState, SelectState, SpinnerState, SplitPaneResponse,
    SplitPaneState, StaticOutput, StreamingMarkdownState, StreamingTextState, TableColumn,
    TableState, TabsState, TextInputState, TextareaState, ToastLevel, ToastMessage, ToastState,
    ToolApprovalState, TreeNode, TreeState, Trend, DEFAULT_CHORD_TIMEOUT_TICKS,
};

/// Rendering backend for SLT.
///
/// Implement this trait to render SLT UIs to custom targets — alternative
/// terminals, GUI embeds, test harnesses, WASM canvas, etc.
///
/// The built-in terminal backend ([`run()`], [`run_with()`]) handles setup,
/// teardown, and event polling automatically. For custom backends, pair this
/// trait with [`AppState`] and [`frame()`] to drive the render loop yourself.
///
/// # Example
///
/// ```ignore
/// use slt::{Backend, AppState, Buffer, Rect, RunConfig, Context, Event};
///
/// struct MyBackend {
///     buffer: Buffer,
/// }
///
/// impl Backend for MyBackend {
///     fn size(&self) -> (u32, u32) {
///         (self.buffer.area.width, self.buffer.area.height)
///     }
///     fn buffer_mut(&mut self) -> &mut Buffer {
///         &mut self.buffer
///     }
///     fn flush(&mut self) -> std::io::Result<()> {
///         // Render self.buffer to your target
///         Ok(())
///     }
/// }
///
/// fn main() -> std::io::Result<()> {
///     let mut backend = MyBackend {
///         buffer: Buffer::empty(Rect::new(0, 0, 80, 24)),
///     };
///     let mut state = AppState::new();
///     let config = RunConfig::default();
///
///     loop {
///         let events: Vec<Event> = vec![]; // Collect your own events
///         if !slt::frame(&mut backend, &mut state, &config, &events, &mut |ui| {
///             ui.text("Hello from custom backend!");
///         })? {
///             break;
///         }
///     }
///     Ok(())
/// }
/// ```
pub trait Backend {
    /// Returns the current display size as `(width, height)` in cells.
    fn size(&self) -> (u32, u32);

    /// Returns a mutable reference to the display buffer.
    ///
    /// SLT writes the UI into this buffer each frame. After [`frame()`]
    /// returns, call [`flush()`](Backend::flush) to present the result.
    fn buffer_mut(&mut self) -> &mut Buffer;

    /// Flush the buffer contents to the display.
    ///
    /// Called automatically at the end of each [`frame()`] call. Implementations
    /// should present the current buffer to the user — by writing ANSI escapes,
    /// drawing to a canvas, updating a texture, etc.
    fn flush(&mut self) -> io::Result<()>;
}

/// Opaque per-session state that persists between frames.
///
/// Tracks focus, scroll positions, hook state, and other frame-to-frame data.
/// Create with [`AppState::new()`] and pass to [`frame()`] each iteration.
///
/// # Example
///
/// ```ignore
/// let mut state = slt::AppState::new();
/// // state is passed to slt::frame() in your render loop
/// ```
pub struct AppState {
    pub(crate) inner: FrameState,
}

impl AppState {
    /// Create a new empty application state.
    pub fn new() -> Self {
        Self {
            inner: FrameState::default(),
        }
    }

    /// Returns the current frame tick count (increments each frame).
    pub fn tick(&self) -> u64 {
        self.inner.diagnostics.tick
    }

    /// Returns the smoothed FPS estimate (exponential moving average).
    pub fn fps(&self) -> f32 {
        self.inner.diagnostics.fps_ema
    }

    /// Toggle the debug overlay (same as pressing F12).
    pub fn set_debug(&mut self, enabled: bool) {
        self.inner.diagnostics.debug_mode = enabled;
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Process a single UI frame with a custom [`Backend`].
///
/// This is the low-level entry point for custom backends. For standard terminal
/// usage, prefer [`run()`] or [`run_with()`] which handle the event loop,
/// terminal setup, and teardown automatically.
///
/// Returns `Ok(true)` to continue, `Ok(false)` when [`Context::quit()`] was
/// called.
///
/// # Arguments
///
/// * `backend` — Your [`Backend`] implementation
/// * `state` — Persistent [`AppState`] (reuse across frames)
/// * `config` — [`RunConfig`] (theme, tick rate, etc.)
/// * `events` — Input events for this frame (keyboard, mouse, resize)
/// * `f` — Your UI closure, called once per frame
///
/// Build a fresh event slice each frame in your outer loop, then pass it here.
/// `frame()` reads from that slice but does not own your event source.
/// Reuse the same [`AppState`] for the lifetime of the session.
///
/// # Example
///
/// ```ignore
/// let keep_going = slt::frame(
///     &mut my_backend,
///     &mut state,
///     &config,
///     &events,
///     &mut |ui| { ui.text("hello"); },
/// )?;
/// ```
pub fn frame(
    backend: &mut impl Backend,
    state: &mut AppState,
    config: &RunConfig,
    events: &[Event],
    f: &mut impl FnMut(&mut Context),
) -> io::Result<bool> {
    frame_owned(backend, state, config, events.to_vec(), f)
}

/// Process a single UI frame, taking ownership of the events `Vec` (zero-copy).
///
/// Like [`frame`], but accepts an owned `Vec<Event>` to avoid the `to_vec()`
/// copy `frame` performs internally. Prefer this in high-frequency custom
/// render loops where you already own the event buffer.
///
/// # Example
///
/// ```ignore
/// let events: Vec<slt::Event> = collect_events();
/// let keep_going = slt::frame_owned(
///     &mut my_backend,
///     &mut state,
///     &config,
///     events,
///     &mut |ui| { ui.text("hello"); },
/// )?;
/// ```
pub fn frame_owned(
    backend: &mut impl Backend,
    state: &mut AppState,
    config: &RunConfig,
    events: Vec<Event>,
    f: &mut impl FnMut(&mut Context),
) -> io::Result<bool> {
    run_frame(backend, &mut state.inner, config, events, f)
}

#[cfg(feature = "crossterm")]
static PANIC_HOOK_ONCE: Once = Once::new();

#[allow(clippy::print_stderr)]
#[cfg(feature = "crossterm")]
fn install_panic_hook() {
    PANIC_HOOK_ONCE.call_once(|| {
        let original = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            let _ = crossterm::terminal::disable_raw_mode();
            let mut stdout = io::stdout();
            let _ = crossterm::execute!(
                stdout,
                crossterm::terminal::LeaveAlternateScreen,
                crossterm::cursor::Show,
                crossterm::event::DisableMouseCapture,
                crossterm::event::DisableBracketedPaste,
                crossterm::style::ResetColor,
                crossterm::style::SetAttribute(crossterm::style::Attribute::Reset)
            );

            // Print friendly panic header
            eprintln!("\n\x1b[1;31m━━━ SLT Panic ━━━\x1b[0m\n");

            // Print location if available
            if let Some(location) = panic_info.location() {
                eprintln!(
                    "\x1b[90m{}:{}:{}\x1b[0m",
                    location.file(),
                    location.line(),
                    location.column()
                );
            }

            // Print message
            if let Some(msg) = panic_info.payload().downcast_ref::<&str>() {
                eprintln!("\x1b[1m{}\x1b[0m", msg);
            } else if let Some(msg) = panic_info.payload().downcast_ref::<String>() {
                eprintln!("\x1b[1m{}\x1b[0m", msg);
            }

            eprintln!(
                "\n\x1b[90mTerminal state restored. Report bugs at https://github.com/subinium/SuperLightTUI/issues\x1b[0m\n"
            );

            original(panic_info);
        }));
    });
}

/// RAII guard owning the unix suspend/resume (`SIGTSTP`/`SIGCONT`) handler
/// thread for the duration of a run loop (issue #263).
///
/// Dropping the guard closes the `signal-hook` registration so the background
/// thread breaks out of `Signals::forever()` and is joined, leaving no signal
/// handlers installed after the loop exits.
#[cfg(all(feature = "crossterm", unix))]
struct SuspendGuard {
    handle: signal_hook::iterator::Handle,
    thread: Option<std::thread::JoinHandle<()>>,
}

#[cfg(all(feature = "crossterm", unix))]
impl Drop for SuspendGuard {
    fn drop(&mut self) {
        // Closing the handle wakes `Signals::forever()` so the thread returns.
        self.handle.close();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Install the unix job-control suspend/resume handler for one run loop.
///
/// Spawns a `signal-hook` background thread that, on `SIGTSTP`, restores the
/// terminal and re-raises the default-disposition stop, and on `SIGCONT`
/// re-enters the session and flags a full redraw. Uses only signal-hook's safe
/// API, preserving `#![forbid(unsafe_code)]`. Returns the guard that owns the
/// thread; dropping it uninstalls the handler.
#[cfg(all(feature = "crossterm", unix))]
fn install_suspend_handler(snapshot: terminal::SessionSnapshot) -> io::Result<SuspendGuard> {
    use signal_hook::consts::{SIGCONT, SIGTSTP};
    use signal_hook::iterator::Signals;

    let mut signals = Signals::new([SIGTSTP, SIGCONT])?;
    let handle = signals.handle();
    let thread = std::thread::Builder::new()
        .name("slt-suspend".to_string())
        .spawn(move || {
            // `has_terminal` tracks whether the TUI session is currently
            // entered, so a stray SIGCONT (no prior SIGTSTP) or a repeated
            // SIGTSTP cannot double-leave / double-enter (idempotency).
            let mut has_terminal = true;
            for signal in &mut signals {
                match signal {
                    SIGTSTP if has_terminal => {
                        terminal::suspend_to_shell(&snapshot);
                        has_terminal = false;
                        // Genuinely stop the process now that the terminal is
                        // restored; control returns to the shell.
                        let _ = signal_hook::low_level::emulate_default_handler(SIGTSTP);
                    }
                    SIGCONT if !has_terminal => {
                        terminal::resume_from_shell(&snapshot);
                        has_terminal = true;
                    }
                    // Repeated SIGTSTP/SIGCONT or out-of-order delivery is a
                    // no-op — the `has_terminal` guard keeps enter/leave
                    // balanced (idempotency, issue #263).
                    _ => {}
                }
            }
        })?;

    Ok(SuspendGuard {
        handle,
        thread: Some(thread),
    })
}

/// Consume the pending full-redraw request raised by a `SIGCONT` resume and, if
/// set, clear + repaint the whole frame (issue #263).
///
/// Called at the top of each run-loop iteration. No-op on non-unix builds.
#[cfg(all(feature = "crossterm", unix))]
fn drain_resume_redraw(handle_resize: &mut impl FnMut() -> io::Result<()>) -> io::Result<()> {
    use std::sync::atomic::Ordering;
    if terminal::NEEDS_FULL_REDRAW.swap(false, Ordering::SeqCst) {
        handle_resize()?;
    }
    Ok(())
}

/// Configuration for a TUI run loop.
///
/// Pass to [`run_with`] or [`run_inline_with`] to customize behavior.
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
    /// arrive as [`KeyCode::Modifier`] events; other terminals never emit them.
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
    /// through to the frame closure as a regular [`Event::Key`], matching
    /// RataTUI's raw-mode semantics. The user is then responsible for
    /// deciding whether to call [`Context::quit`] or treat it as any other
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
    /// and releases arrive as [`KeyCode::Modifier`] events. Defaults to
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
    /// receive Ctrl+C as a regular [`Event::Key`] inside the frame closure
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
}

#[derive(Default)]
pub(crate) struct FocusState {
    pub focus_index: usize,
    pub prev_focus_count: usize,
    pub prev_modal_active: bool,
    pub prev_modal_focus_start: usize,
    pub prev_modal_focus_count: usize,
    /// Issue #208: focus index at the end of the previous frame. `None` on
    /// the first frame so widgets do not falsely report `gained_focus`.
    pub prev_focus_index: Option<usize>,
    /// Issue #217: persisted `name → focus_index` map from the most recent
    /// completed frame. Used at frame start to resolve a pending
    /// `focus_by_name(...)` against the previous render's registrations.
    pub focus_name_map_prev: std::collections::HashMap<String, usize>,
    /// Issue #217: a name passed to `focus_by_name(...)` that has not yet
    /// been resolved. Consumed once the matching registration is found in
    /// `focus_name_map_prev`.
    pub pending_focus_name: Option<String>,
}

#[derive(Default)]
pub(crate) struct LayoutFeedbackState {
    pub prev_scroll_infos: Vec<(u32, u32)>,
    pub prev_scroll_rects: Vec<rect::Rect>,
    pub prev_hit_map: Vec<rect::Rect>,
    pub prev_group_rects: Vec<(std::sync::Arc<str>, rect::Rect)>,
    pub prev_content_map: Vec<(rect::Rect, rect::Rect)>,
    pub prev_focus_rects: Vec<(usize, rect::Rect)>,
    pub prev_focus_groups: Vec<Option<std::sync::Arc<str>>>,
    pub last_mouse_pos: Option<(u32, u32)>,
}

#[derive(Default)]
pub(crate) struct DiagnosticsState {
    pub tick: u64,
    pub notification_queue: Vec<(String, ToastLevel, u64)>,
    pub debug_mode: bool,
    pub debug_layer: DebugLayer,
    pub fps_ema: f32,
}

/// Which layers the F12 debug overlay should outline (issue #201).
///
/// `All` (the default) outlines both the base layer and any active
/// overlays/modals — matching the user's expectation for "show everything
/// the renderer is producing this frame." `TopMost` only outlines the
/// topmost overlay (or the base if no overlay is active), and `BaseOnly`
/// keeps the legacy pre-fix behavior of skipping overlays entirely.
///
/// At runtime, **Shift+F12** cycles `All → TopMost → BaseOnly → All` so a
/// developer debugging a stacked modal can shrink the visible outlines to
/// just the layer they care about without leaving the keyboard. Plain
/// **F12** independently toggles the overlay on/off.
///
/// # Example
///
/// ```no_run
/// use slt::{Context, DebugLayer};
///
/// slt::run(|ui: &mut Context| {
///     // Match on the current layer to drive bespoke debug UI.
///     let label = match ui.debug_layer() {
///         DebugLayer::All => "showing base + overlays",
///         DebugLayer::TopMost => "showing topmost overlay only",
///         DebugLayer::BaseOnly => "showing base layer only",
///     };
///     ui.text(label);
/// })
/// .unwrap();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DebugLayer {
    /// Outline both the base tree and every active overlay/modal.
    ///
    /// Default. Matches the reporter expectation that F12 reflects
    /// everything the renderer is producing this frame. Each layer family
    /// gets its own hue so a glance distinguishes base, overlay, and modal
    /// containers.
    #[default]
    All,
    /// Outline only the topmost overlay (or the base if no overlay is
    /// active).
    ///
    /// Useful when modals or popovers stack and you only care about the
    /// active dialog — base-tree outlines become noise underneath an open
    /// modal.
    TopMost,
    /// Outline only the base layer (legacy v0.19.x behavior).
    ///
    /// Skips overlays and modals entirely. Use when an overlay is
    /// confirmed correct and you want to inspect the base layout
    /// underneath it.
    BaseOnly,
}

/// Type alias matching `context::core::RawDrawCallback` (private over there);
/// used inside `FrameState` for the recycled-Vec field for issue #204. Kept
/// in lib.rs to avoid leaking a public type alias.
pub(crate) type FrameDeferredDrawSlot =
    Option<Box<dyn FnOnce(&mut crate::buffer::Buffer, crate::rect::Rect)>>;

#[derive(Default)]
pub(crate) struct FrameState {
    pub hook_states: Vec<Box<dyn std::any::Any>>,
    pub named_states: std::collections::HashMap<&'static str, Box<dyn std::any::Any>>,
    /// Issue #215: runtime-string-keyed parallel of `named_states`. Persisted
    /// across frames; survives panics inside `error_boundary` (matching the
    /// `named_states` policy).
    pub keyed_states: std::collections::HashMap<String, Box<dyn std::any::Any>>,
    /// Issue #262: cross-frame partial-chord buffer for [`Context::key_chord`].
    /// Round-trips across frames using the same `std::mem::take` out/in policy
    /// as `keyed_states` (moved out in `Context::new`, restored at frame end in
    /// `run_frame_kernel`).
    pub chord_states: widgets::ChordState,
    pub screen_hook_map: std::collections::HashMap<String, (usize, usize)>,
    pub focus: FocusState,
    pub layout_feedback: LayoutFeedbackState,
    pub diagnostics: DiagnosticsState,
    /// Recycled command Vec (issue #150). `Context::new` swaps this into the
    /// new context (capacity preserved, len reset to 0). After `build_tree`
    /// drains the commands, the now-empty Vec is reclaimed back here.
    pub commands_buf: Vec<crate::layout::Command>,
    /// Recycled per-frame layout collection scratch (issue #155). Same
    /// pattern as `commands_buf`: clear before use, restore after.
    pub frame_data: crate::layout::FrameData,
    /// Recycled `Context::context_stack` Vec (issue #204). Empty/cleared at
    /// frame end (same pattern as `commands_buf`).
    pub context_stack_buf: Vec<Box<dyn std::any::Any>>,
    /// Recycled `Context::deferred_draws` Vec (issue #204). Slots are emptied
    /// (set to `None`) when callbacks fire; we clear before reuse.
    pub deferred_draws_buf: Vec<FrameDeferredDrawSlot>,
    /// Recycled `rollback.group_stack` Vec (issue #204). Asserted empty at
    /// frame end before reclamation.
    pub group_stack_buf: Vec<std::sync::Arc<str>>,
    /// Recycled `rollback.text_color_stack` Vec (issue #204). Asserted empty
    /// at frame end before reclamation.
    pub text_color_stack_buf: Vec<Option<crate::style::Color>>,
    /// Recycled `Context::pending_tooltips` Vec (issue #204). Asserted empty
    /// at frame end before reclamation.
    pub pending_tooltips_buf: Vec<context::PendingTooltip>,
    /// Recycled `Context::hovered_groups` set (issue #204). Cleared at the
    /// start of each frame by `build_hovered_groups`.
    pub hovered_groups_buf: std::collections::HashSet<std::sync::Arc<str>>,
    #[cfg(feature = "crossterm")]
    pub selection: terminal::SelectionState,
}

/// Run the TUI loop with default configuration.
///
/// Enters alternate screen mode, runs `f` each frame, and exits cleanly on
/// Ctrl+C or when [`Context::quit`] is called.
///
/// # Raw mode is handled for you
///
/// SLT enters raw mode automatically inside [`run`] / [`run_with`] /
/// [`run_inline`] / [`run_async`]. Wrapping these with manual
/// `crossterm::terminal::enable_raw_mode()` and `disable_raw_mode()` is
/// **redundant** — the calls are idempotent so no harm comes of it, but it
/// suggests a misunderstood lifecycle. Drop the wrapper calls:
///
/// ```no_run
/// // Don't do this — it's already handled internally:
/// // crossterm::terminal::enable_raw_mode()?;
/// slt::run(|ui| { ui.text("hi"); })?;
/// // crossterm::terminal::disable_raw_mode()?;
/// # Ok::<_, std::io::Error>(())
/// ```
///
/// # Ctrl+C opt-out (issue #238)
///
/// By default, Ctrl+C exits the loop cleanly — matching the v0.19 contract
/// and the convention most TUIs follow. To match RataTUI's raw-mode
/// semantics (Ctrl+C delivered as a regular `Event::Key`), set
/// [`RunConfig::handle_ctrl_c(false)`](RunConfig::handle_ctrl_c) and decide
/// inside the frame closure whether to call [`Context::quit`]:
///
/// ```no_run
/// use slt::{KeyModifiers, RunConfig};
///
/// slt::run_with(RunConfig::default().handle_ctrl_c(false), |ui| {
///     if ui.key_mod('c', KeyModifiers::CONTROL) {
///         // e.g. clear input, prompt to save, then quit:
///         ui.quit();
///     }
/// })?;
/// # Ok::<_, std::io::Error>(())
/// ```
///
/// # Example
///
/// ```no_run
/// fn main() -> std::io::Result<()> {
///     slt::run(|ui| {
///         ui.text("Press Ctrl+C to exit");
///     })
/// }
/// ```
#[cfg(feature = "crossterm")]
pub fn run(f: impl FnMut(&mut Context)) -> io::Result<()> {
    run_with(RunConfig::default(), f)
}

#[cfg(feature = "crossterm")]
fn set_terminal_title(title: &Option<String>) {
    if let Some(title) = title {
        use std::io::Write;
        let mut stdout = io::stdout();
        let _ = write!(stdout, "\x1b]2;{title}\x07");
        let _ = stdout.flush();
    }
}

/// Run the TUI loop with custom configuration.
///
/// Like [`run`], but accepts a [`RunConfig`] to control tick rate, mouse
/// support, and theming.
///
/// # Example
///
/// ```no_run
/// use slt::{RunConfig, Theme};
///
/// fn main() -> std::io::Result<()> {
///     slt::run_with(
///         RunConfig::default().theme(Theme::light()),
///         |ui| {
///             ui.text("Light theme!");
///         },
///     )
/// }
/// ```
#[cfg(feature = "crossterm")]
pub fn run_with(config: RunConfig, mut f: impl FnMut(&mut Context)) -> io::Result<()> {
    if !io::stdout().is_terminal() {
        return Ok(());
    }

    install_panic_hook();
    let color_depth = config.color_depth.unwrap_or_else(ColorDepth::detect);
    let mut term = Terminal::new(
        config.mouse,
        config.kitty_keyboard,
        config.report_all_keys,
        color_depth,
    )?;
    set_terminal_title(&config.title);
    if config.theme.bg != Color::Reset {
        term.theme_bg = Some(config.theme.bg);
    }
    // Issue #263: install the unix Ctrl+Z / `fg` suspend handler for the loop.
    #[cfg(unix)]
    let _suspend_guard = if config.handle_suspend {
        Some(install_suspend_handler(term.session_snapshot())?)
    } else {
        None
    };
    let mut events: Vec<Event> = Vec::new();
    let mut state = FrameState::default();

    loop {
        let frame_start = Instant::now();
        // Issue #263: after a SIGCONT resume, repaint the whole frame.
        #[cfg(unix)]
        drain_resume_redraw(&mut || term.handle_resize())?;
        let (w, h) = term.size();
        if w == 0 || h == 0 {
            sleep_for_fps_cap(config.max_fps, frame_start.elapsed());
            continue;
        }

        if !run_frame(
            &mut term,
            &mut state,
            &config,
            std::mem::take(&mut events),
            &mut f,
        )? {
            break;
        }
        // Issue #233: full-screen mode has no scrollback channel — warn and
        // drop any `ui.static_log(...)` lines so they do not leak into the
        // next frame's named_states.
        discard_static_log(&mut state, "full-screen run()");
        let render_elapsed = frame_start.elapsed();

        if !poll_events(
            &mut events,
            &mut state,
            config.tick_rate,
            &mut || term.handle_resize(),
            config.handle_ctrl_c,
        )? {
            break;
        }

        sleep_for_fps_cap(config.max_fps, render_elapsed);
    }

    Ok(())
}

/// Run the TUI loop asynchronously with default configuration.
///
/// Requires the `async` feature. Spawns the render loop in a blocking thread
/// and returns a [`tokio::sync::mpsc::Sender`] you can use to push messages
/// from async tasks into the UI closure.
///
/// # Example
///
/// ```no_run
/// # #[cfg(feature = "async")]
/// # async fn example() -> std::io::Result<()> {
/// let tx = slt::run_async::<String>(|ui, messages| {
///     for msg in messages.drain(..) {
///         ui.text(msg);
///     }
/// })?;
/// tx.send("hello from async".to_string()).await.ok();
/// # Ok(())
/// # }
/// ```
#[cfg(all(feature = "crossterm", feature = "async"))]
pub fn run_async<M: Send + 'static>(
    f: impl FnMut(&mut Context, &mut Vec<M>) + Send + 'static,
) -> io::Result<tokio::sync::mpsc::Sender<M>> {
    run_async_with(RunConfig::default(), f)
}

/// Run the TUI loop asynchronously with custom configuration.
///
/// Requires the `async` feature. Like [`run_async`], but accepts a
/// [`RunConfig`] to control tick rate, mouse support, and theming.
///
/// Returns a [`tokio::sync::mpsc::Sender`] for pushing messages into the UI.
#[cfg(all(feature = "crossterm", feature = "async"))]
pub fn run_async_with<M: Send + 'static>(
    config: RunConfig,
    f: impl FnMut(&mut Context, &mut Vec<M>) + Send + 'static,
) -> io::Result<tokio::sync::mpsc::Sender<M>> {
    let (tx, rx) = tokio::sync::mpsc::channel(100);
    let handle =
        tokio::runtime::Handle::try_current().map_err(|err| io::Error::other(err.to_string()))?;

    handle.spawn_blocking(move || {
        let _ = run_async_loop(config, f, rx);
    });

    Ok(tx)
}

#[cfg(all(feature = "crossterm", feature = "async"))]
fn run_async_loop<M: Send + 'static>(
    config: RunConfig,
    mut f: impl FnMut(&mut Context, &mut Vec<M>) + Send,
    mut rx: tokio::sync::mpsc::Receiver<M>,
) -> io::Result<()> {
    if !io::stdout().is_terminal() {
        return Ok(());
    }

    install_panic_hook();
    let color_depth = config.color_depth.unwrap_or_else(ColorDepth::detect);
    let mut term = Terminal::new(
        config.mouse,
        config.kitty_keyboard,
        config.report_all_keys,
        color_depth,
    )?;
    set_terminal_title(&config.title);
    if config.theme.bg != Color::Reset {
        term.theme_bg = Some(config.theme.bg);
    }
    // Issue #263: install the unix Ctrl+Z / `fg` suspend handler for the loop.
    #[cfg(unix)]
    let _suspend_guard = if config.handle_suspend {
        Some(install_suspend_handler(term.session_snapshot())?)
    } else {
        None
    };
    let mut events: Vec<Event> = Vec::new();
    let mut messages: Vec<M> = Vec::new();
    let mut state = FrameState::default();

    loop {
        let frame_start = Instant::now();
        // Issue #263: after a SIGCONT resume, repaint the whole frame.
        #[cfg(unix)]
        drain_resume_redraw(&mut || term.handle_resize())?;
        messages.clear();
        while let Ok(message) = rx.try_recv() {
            messages.push(message);
        }

        let (w, h) = term.size();
        if w == 0 || h == 0 {
            sleep_for_fps_cap(config.max_fps, frame_start.elapsed());
            continue;
        }

        let mut render = |ctx: &mut Context| {
            f(ctx, &mut messages);
        };
        if !run_frame(
            &mut term,
            &mut state,
            &config,
            std::mem::take(&mut events),
            &mut render,
        )? {
            break;
        }
        // Issue #233: full-screen async mode has no scrollback channel — warn
        // and drop any pending static_log lines.
        discard_static_log(&mut state, "run_async()");
        let render_elapsed = frame_start.elapsed();

        if !poll_events(
            &mut events,
            &mut state,
            config.tick_rate,
            &mut || term.handle_resize(),
            config.handle_ctrl_c,
        )? {
            break;
        }

        sleep_for_fps_cap(config.max_fps, render_elapsed);
    }

    Ok(())
}

/// Run the TUI in inline mode with default configuration.
///
/// Renders `height` rows directly below the current cursor position without
/// entering alternate screen mode. Useful for CLI tools that want a small
/// interactive widget below the prompt.
///
/// `height` is the reserved inline render area in terminal rows.
/// The rest of the terminal stays in normal scrollback mode.
///
/// # Example
///
/// ```no_run
/// fn main() -> std::io::Result<()> {
///     slt::run_inline(3, |ui| {
///         ui.text("Inline TUI — no alternate screen");
///     })
/// }
/// ```
#[cfg(feature = "crossterm")]
pub fn run_inline(height: u32, f: impl FnMut(&mut Context)) -> io::Result<()> {
    run_inline_with(height, RunConfig::default(), f)
}

/// Run the TUI in inline mode with custom configuration.
///
/// Like [`run_inline`], but accepts a [`RunConfig`] to control tick rate,
/// mouse support, and theming.
#[cfg(feature = "crossterm")]
pub fn run_inline_with(
    height: u32,
    config: RunConfig,
    mut f: impl FnMut(&mut Context),
) -> io::Result<()> {
    if !io::stdout().is_terminal() {
        return Ok(());
    }

    install_panic_hook();
    let color_depth = config.color_depth.unwrap_or_else(ColorDepth::detect);
    let mut term = InlineTerminal::new(
        height,
        config.mouse,
        config.kitty_keyboard,
        config.report_all_keys,
        color_depth,
    )?;
    set_terminal_title(&config.title);
    if config.theme.bg != Color::Reset {
        term.theme_bg = Some(config.theme.bg);
    }
    // Issue #263: install the unix Ctrl+Z / `fg` suspend handler for the loop.
    #[cfg(unix)]
    let _suspend_guard = if config.handle_suspend {
        Some(install_suspend_handler(term.session_snapshot())?)
    } else {
        None
    };
    let mut events: Vec<Event> = Vec::new();
    let mut state = FrameState::default();

    loop {
        let frame_start = Instant::now();
        // Issue #263: after a SIGCONT resume, repaint the whole frame.
        #[cfg(unix)]
        drain_resume_redraw(&mut || term.handle_resize())?;
        let (w, h) = term.size();
        if w == 0 || h == 0 {
            sleep_for_fps_cap(config.max_fps, frame_start.elapsed());
            continue;
        }

        if !run_frame(
            &mut term,
            &mut state,
            &config,
            std::mem::take(&mut events),
            &mut f,
        )? {
            break;
        }
        // Issue #233: inline mode without `StaticOutput` has no scrollback
        // channel either — warn and drop any pending lines.
        discard_static_log(&mut state, "run_inline()");
        let render_elapsed = frame_start.elapsed();

        if !poll_events(
            &mut events,
            &mut state,
            config.tick_rate,
            &mut || term.handle_resize(),
            config.handle_ctrl_c,
        )? {
            break;
        }

        sleep_for_fps_cap(config.max_fps, render_elapsed);
    }

    Ok(())
}

/// Run the TUI in static-output mode.
///
/// Static lines written through [`StaticOutput`] are printed into terminal
/// scrollback, while the interactive UI stays rendered in a fixed-height inline
/// area at the bottom.
///
/// Use this when you want a log-style output stream above a live inline UI.
#[cfg(feature = "crossterm")]
pub fn run_static(
    output: &mut StaticOutput,
    dynamic_height: u32,
    f: impl FnMut(&mut Context),
) -> io::Result<()> {
    run_static_with(output, dynamic_height, RunConfig::default(), f)
}

/// Run the TUI in static-output mode with custom configuration.
///
/// Like [`run_static`] but accepts a [`RunConfig`] for theme, mouse, tick rate,
/// and other settings.
#[cfg(feature = "crossterm")]
pub fn run_static_with(
    output: &mut StaticOutput,
    dynamic_height: u32,
    config: RunConfig,
    mut f: impl FnMut(&mut Context),
) -> io::Result<()> {
    if !io::stdout().is_terminal() {
        return Ok(());
    }

    install_panic_hook();

    let initial_lines = output.drain_new();
    write_static_lines(&initial_lines)?;

    let color_depth = config.color_depth.unwrap_or_else(ColorDepth::detect);
    let mut term = InlineTerminal::new(
        dynamic_height,
        config.mouse,
        config.kitty_keyboard,
        config.report_all_keys,
        color_depth,
    )?;
    set_terminal_title(&config.title);
    if config.theme.bg != Color::Reset {
        term.theme_bg = Some(config.theme.bg);
    }
    // Issue #263: install the unix Ctrl+Z / `fg` suspend handler for the loop.
    #[cfg(unix)]
    let _suspend_guard = if config.handle_suspend {
        Some(install_suspend_handler(term.session_snapshot())?)
    } else {
        None
    };

    let mut events: Vec<Event> = Vec::new();
    let mut state = FrameState::default();

    loop {
        let frame_start = Instant::now();
        // Issue #263: after a SIGCONT resume, repaint the whole frame.
        #[cfg(unix)]
        drain_resume_redraw(&mut || term.handle_resize())?;
        let (w, h) = term.size();
        if w == 0 || h == 0 {
            sleep_for_fps_cap(config.max_fps, frame_start.elapsed());
            continue;
        }

        let new_lines = output.drain_new();
        write_static_lines(&new_lines)?;

        if !run_frame(
            &mut term,
            &mut state,
            &config,
            std::mem::take(&mut events),
            &mut f,
        )? {
            break;
        }
        // Issue #233: drain any `ui.static_log(...)` lines queued during the
        // frame closure into `output`; the next loop iteration flushes them
        // above the inline area via `write_static_lines`.
        for line in drain_static_log(&mut state) {
            output.println(line);
        }
        let render_elapsed = frame_start.elapsed();

        if !poll_events(
            &mut events,
            &mut state,
            config.tick_rate,
            &mut || term.handle_resize(),
            config.handle_ctrl_c,
        )? {
            break;
        }

        sleep_for_fps_cap(config.max_fps, render_elapsed);
    }

    Ok(())
}

#[cfg(feature = "crossterm")]
fn write_static_lines(lines: &[String]) -> io::Result<()> {
    if lines.is_empty() {
        return Ok(());
    }

    let mut stdout = io::stdout();
    for line in lines {
        stdout.write_all(line.as_bytes())?;
        stdout.write_all(b"\r\n")?;
    }
    stdout.flush()
}

/// Reserved sentinel key used by [`Context::static_log`] (issue #233).
/// Re-exported into `context::runtime` so reads/writes never drift.
pub(crate) const STATIC_LOG_NAMED_STATE_KEY: &str = "__slt_static_log_pending";

/// Reserved sentinel key used by [`Context::publish_keymap`] (issue #236).
/// Re-exported into `context::runtime` so reads/writes never drift.
pub(crate) const KEYMAP_REGISTRY_NAMED_STATE_KEY: &str = "__slt_keymap_registry";

/// Clear the per-frame keymap registry stored in [`FrameState::named_states`]
/// (issue #236). Called at the start of every kernel iteration so that
/// `Context::publish_keymap` always sees a fresh empty buffer. Capacity is
/// preserved by clearing the inner `Vec` rather than removing the entry.
pub(crate) fn clear_keymap_registry(state: &mut FrameState) {
    if let Some(boxed) = state.named_states.get_mut(KEYMAP_REGISTRY_NAMED_STATE_KEY) {
        if let Some(vec) = boxed.downcast_mut::<Vec<crate::keymap::PublishedKeymap>>() {
            vec.clear();
        }
    }
}

/// Drain any [`Context::static_log`] lines accumulated during the most recent
/// frame from the persisted [`FrameState`] (issue #233).
///
/// After [`run_frame_kernel`] returns, `state.named_states` owns the buffer.
/// This helper drains it back to a `Vec<String>` so the runtime can flush
/// the lines through whichever scrollback mechanism is appropriate
/// (`run_static_with` writes them above the inline region; other run modes
/// drop them with a debug warning).
#[cfg(feature = "crossterm")]
pub(crate) fn drain_static_log(state: &mut FrameState) -> Vec<String> {
    if let Some(boxed) = state.named_states.get_mut(STATIC_LOG_NAMED_STATE_KEY) {
        if let Some(buf) = boxed.downcast_mut::<Vec<String>>() {
            return std::mem::take(buf);
        }
    }
    Vec::new()
}

/// Discard any [`Context::static_log`] lines that accumulated during the
/// most recent frame and emit a debug warning (issue #233).
///
/// Used by run modes that have no scrollback channel (full-screen,
/// inline-without-static, async). Release builds silently drop the buffer.
#[cfg(feature = "crossterm")]
fn discard_static_log(state: &mut FrameState, mode: &str) {
    let drained = drain_static_log(state);
    #[cfg(debug_assertions)]
    if !drained.is_empty() {
        #[allow(clippy::print_stderr)]
        {
            eprintln!(
                "[slt] {} static_log lines were dropped: {} runtime has no scrollback channel; use slt::run_static for streaming output",
                drained.len(),
                mode
            );
        }
    }
    #[cfg(not(debug_assertions))]
    {
        let _ = (drained, mode);
    }
}

/// Apply a single terminal event to `FrameState`, mutating tracked
/// diagnostics fields (debug overlay toggle, mouse position cache,
/// resize flag) accordingly.
///
/// Issue #201: handles **F12** (toggle overlay on/off) and **Shift+F12**
/// (cycle [`DebugLayer`] across `All → TopMost → BaseOnly`). The two
/// keybindings are independent — toggling the overlay does not change
/// the active layer.
///
/// Extracted from `poll_events` so the keybinding behavior can be
/// exercised by unit tests without standing up a real crossterm event
/// stream.
#[cfg(feature = "crossterm")]
pub(crate) fn process_run_loop_event(ev: &Event, state: &mut FrameState, has_resize: &mut bool) {
    match ev {
        Event::Mouse(m) => {
            state.layout_feedback.last_mouse_pos = Some((m.x, m.y));
        }
        Event::FocusLost => {
            state.layout_feedback.last_mouse_pos = None;
        }
        // Issue #201: Shift+F12 cycles the active `DebugLayer`. Match
        // before the plain-F12 arm so the modifier branch wins. Plain
        // F12 keeps its legacy on/off toggle when no modifiers are
        // held; we explicitly require `KeyModifiers::NONE` so the two
        // arms do not double-fire on the same press.
        Event::Key(event::KeyEvent {
            code: KeyCode::F(12),
            kind: event::KeyEventKind::Press,
            modifiers,
        }) if modifiers.contains(event::KeyModifiers::SHIFT) => {
            state.diagnostics.debug_layer = match state.diagnostics.debug_layer {
                DebugLayer::All => DebugLayer::TopMost,
                DebugLayer::TopMost => DebugLayer::BaseOnly,
                DebugLayer::BaseOnly => DebugLayer::All,
            };
        }
        Event::Key(event::KeyEvent {
            code: KeyCode::F(12),
            kind: event::KeyEventKind::Press,
            modifiers,
        }) if *modifiers == event::KeyModifiers::NONE => {
            state.diagnostics.debug_mode = !state.diagnostics.debug_mode;
        }
        Event::Resize(_, _) => {
            *has_resize = true;
        }
        _ => {}
    }
}

/// Poll for terminal events, handling resize, Ctrl-C, F12 debug toggle,
/// and layout cache invalidation. Returns `Ok(false)` when the loop should exit.
///
/// `handle_ctrl_c` controls whether Ctrl+C exits the loop (`true`, default
/// v0.19 behavior) or is delivered to the frame closure as a regular key
/// event (`false`, RataTUI parity, issue #238).
#[cfg(feature = "crossterm")]
fn poll_events(
    events: &mut Vec<Event>,
    state: &mut FrameState,
    tick_rate: Duration,
    on_resize: &mut impl FnMut() -> io::Result<()>,
    handle_ctrl_c: bool,
) -> io::Result<bool> {
    let mut has_resize = false;

    fn process_ev(ev: &Event, state: &mut FrameState, has_resize: &mut bool) {
        process_run_loop_event(ev, state, has_resize);
    }

    if crossterm::event::poll(tick_rate)? {
        let raw = crossterm::event::read()?;
        if let Some(ev) = event::from_crossterm(raw) {
            if handle_ctrl_c && is_ctrl_c(&ev) {
                return Ok(false);
            }
            if matches!(ev, Event::Resize(_, _)) {
                on_resize()?;
            }
            process_ev(&ev, state, &mut has_resize);
            events.push(ev);
        }

        while crossterm::event::poll(Duration::ZERO)? {
            let raw = crossterm::event::read()?;
            if let Some(ev) = event::from_crossterm(raw) {
                if handle_ctrl_c && is_ctrl_c(&ev) {
                    return Ok(false);
                }
                if matches!(ev, Event::Resize(_, _)) {
                    on_resize()?;
                }
                process_ev(&ev, state, &mut has_resize);
                events.push(ev);
            }
        }
    }

    // #90: clear cache first (which also resets last_mouse_pos to None),
    // then re-apply latest mouse pos so Resize+Mouse frames keep coords.
    if has_resize {
        clear_frame_layout_cache(state);
        // After clearing, re-walk events to restore the latest mouse pos
        // (process_ev already set it during collection, but
        // clear_frame_layout_cache wiped it).
        for ev in events.iter() {
            match ev {
                Event::Mouse(m) => {
                    state.layout_feedback.last_mouse_pos = Some((m.x, m.y));
                }
                Event::FocusLost => {
                    state.layout_feedback.last_mouse_pos = None;
                }
                _ => {}
            }
        }
    }

    Ok(true)
}

struct FrameKernelResult {
    should_quit: bool,
    #[cfg(feature = "crossterm")]
    clipboard_text: Option<String>,
    #[cfg(feature = "crossterm")]
    should_copy_selection: bool,
}

pub(crate) fn run_frame_kernel(
    buffer: &mut Buffer,
    state: &mut FrameState,
    config: &RunConfig,
    size: (u32, u32),
    events: Vec<event::Event>,
    is_real_terminal: bool,
    f: &mut impl FnMut(&mut context::Context),
) -> FrameKernelResult {
    let frame_start = Instant::now();
    let (w, h) = size;
    // Issue #236: reset the per-frame keymap registry before constructing
    // `Context`. Widgets that call `publish_keymap` accumulate fresh
    // entries; entries from the previous frame must not leak through
    // `named_states` persistence.
    clear_keymap_registry(state);
    let mut ctx = Context::new(events, w, h, state, config.theme);
    ctx.is_real_terminal = is_real_terminal;
    ctx.set_scroll_speed(config.scroll_speed);
    ctx.widget_theme = config.widget_theme;

    f(&mut ctx);
    ctx.process_focus_keys();
    ctx.render_notifications();
    ctx.emit_pending_tooltips();

    debug_assert_eq!(
        ctx.rollback.overlay_depth, 0,
        "overlay depth must settle back to zero before layout"
    );
    debug_assert_eq!(
        ctx.rollback.group_count, 0,
        "group count must settle back to zero before layout"
    );
    debug_assert!(
        ctx.rollback.group_stack.is_empty(),
        "group stack must be empty before layout"
    );
    debug_assert!(
        ctx.rollback.text_color_stack.is_empty(),
        "text color stack must be empty before layout"
    );
    debug_assert!(
        ctx.pending_tooltips.is_empty(),
        "pending tooltips must be emitted before layout"
    );

    if ctx.should_quit {
        state.hook_states = ctx.hook_states;
        state.named_states = ctx.named_states;
        state.keyed_states = ctx.keyed_states;
        // Issue #262: persist the partial-chord buffer on quit too (TestBackend
        // reuses `FrameState` across `render()` calls — same rationale as the
        // keyed-state reclaim).
        state.chord_states = ctx.chord;
        state.screen_hook_map = ctx.screen_hook_map;
        state.diagnostics.notification_queue = ctx.rollback.notification_queue;
        state.diagnostics.debug_layer = ctx.debug_layer;
        // Issue #208 / #217: persist focus tracking state on quit so a later
        // resumed run starts in a sensible place. (Real TUI exits before
        // resuming, but tests reuse `FrameState` across calls.)
        state.focus.prev_focus_index = Some(ctx.focus_index);
        state.focus.focus_name_map_prev = ctx.focus_name_map;
        state.focus.pending_focus_name = ctx.pending_focus_name;
        // Issue #204: reclaim the 6 alloc-reuse buffers on the quit path
        // too. Real TUI exits ignore this, but TestBackend reuses the same
        // FrameState across `render()` calls — without the reclaim the next
        // frame's `Context::new` `mem::take`s an empty Vec and silently
        // reverts to v0.19 per-frame allocation.
        ctx.deferred_draws.clear();
        state.context_stack_buf = std::mem::take(&mut ctx.context_stack);
        state.deferred_draws_buf = std::mem::take(&mut ctx.deferred_draws);
        state.group_stack_buf = std::mem::take(&mut ctx.rollback.group_stack);
        state.text_color_stack_buf = std::mem::take(&mut ctx.rollback.text_color_stack);
        state.pending_tooltips_buf = std::mem::take(&mut ctx.pending_tooltips);
        state.hovered_groups_buf = std::mem::take(&mut ctx.hovered_groups);
        // Issue #150: reclaim `commands` on quit too (TestBackend reuses
        // `FrameState` across `render()` calls — same rationale as #204).
        // The Vec was never `build_tree`'d on the quit path so it may still
        // hold the recorded commands; clearing here drops them and keeps
        // capacity for the next frame.
        ctx.commands.clear();
        state.commands_buf = std::mem::take(&mut ctx.commands);
        #[cfg(feature = "crossterm")]
        let clipboard_text = ctx.clipboard_text.take();
        #[cfg(feature = "crossterm")]
        let should_copy_selection = false;
        return FrameKernelResult {
            should_quit: true,
            #[cfg(feature = "crossterm")]
            clipboard_text,
            #[cfg(feature = "crossterm")]
            should_copy_selection,
        };
    }
    state.focus.prev_modal_active = ctx.rollback.modal_active;
    state.focus.prev_modal_focus_start = ctx.rollback.modal_focus_start;
    state.focus.prev_modal_focus_count = ctx.rollback.modal_focus_count;
    #[cfg(feature = "crossterm")]
    let clipboard_text = ctx.clipboard_text.take();
    #[cfg(not(feature = "crossterm"))]
    let _clipboard_text = ctx.clipboard_text.take();

    #[cfg(feature = "crossterm")]
    let mut should_copy_selection = false;
    #[cfg(feature = "crossterm")]
    for ev in &ctx.events {
        if let Event::Mouse(mouse) = ev {
            match mouse.kind {
                event::MouseKind::Down(event::MouseButton::Left) => {
                    state.selection.mouse_down(
                        mouse.x,
                        mouse.y,
                        &state.layout_feedback.prev_content_map,
                    );
                }
                event::MouseKind::Drag(event::MouseButton::Left) => {
                    state.selection.mouse_drag(
                        mouse.x,
                        mouse.y,
                        &state.layout_feedback.prev_content_map,
                    );
                }
                event::MouseKind::Up(event::MouseButton::Left) => {
                    should_copy_selection = state.selection.active;
                }
                _ => {}
            }
        }
    }

    state.focus.focus_index = ctx.focus_index;
    state.focus.prev_focus_count = ctx.rollback.focus_count;

    // Issue #150: `state.commands_buf` is swapped into `ctx.commands` on
    // entry (see `Context::new`), so the per-frame `Vec::new()` allocation
    // for the command list is amortized to one allocation across the
    // session. `build_tree` now takes `&mut Vec<Command>` and `drain`s it,
    // leaving the Vec at `len == 0` with capacity preserved. We reclaim
    // that Vec into `state.commands_buf` after the frame so the next call
    // to `Context::new` can pick it up via `mem::take` (matches the #204
    // pattern for the other six recycled buffers).
    let mut tree = layout::build_tree(&mut ctx.commands);
    let area = crate::rect::Rect::new(0, 0, w, h);
    layout::compute(&mut tree, area);

    // Issue #155: reuse `state.frame_data` across frames. `collect_all` calls
    // `fd.clear()` first so the Vecs reset to len=0 with capacity preserved
    // from the prior frame, then refills them.
    let mut fd = std::mem::take(&mut state.frame_data);
    layout::collect_all(&tree, &mut fd);
    debug_assert_eq!(
        fd.scroll_infos.len(),
        fd.scroll_rects.len(),
        "scroll feedback vectors must stay aligned"
    );
    let raw_rects = std::mem::take(&mut fd.raw_draw_rects);
    state.layout_feedback.prev_scroll_infos = std::mem::take(&mut fd.scroll_infos);
    state.layout_feedback.prev_scroll_rects = std::mem::take(&mut fd.scroll_rects);
    state.layout_feedback.prev_hit_map = std::mem::take(&mut fd.hit_areas);
    state.layout_feedback.prev_group_rects = std::mem::take(&mut fd.group_rects);
    state.layout_feedback.prev_content_map = std::mem::take(&mut fd.content_areas);
    state.layout_feedback.prev_focus_rects = std::mem::take(&mut fd.focus_rects);
    state.layout_feedback.prev_focus_groups = std::mem::take(&mut fd.focus_groups);
    state.frame_data = fd;
    layout::render(&tree, buffer);
    // RAII guard ensuring the kitty clip frame is popped even if a raw-draw
    // callback panics — prevents stale scroll-clip state leaking into the
    // next region or subsequent frames.
    struct KittyClipGuard<'a>(&'a mut crate::buffer::Buffer);
    impl Drop for KittyClipGuard<'_> {
        fn drop(&mut self) {
            let _ = self.0.pop_kitty_clip();
        }
    }
    for rdr in raw_rects {
        if rdr.rect.width == 0 || rdr.rect.height == 0 {
            continue;
        }
        if let Some(cb) = ctx
            .deferred_draws
            .get_mut(rdr.draw_id)
            .and_then(|c| c.take())
        {
            buffer.push_clip(rdr.rect);
            buffer.push_kitty_clip(crate::buffer::KittyClipInfo {
                top_clip_rows: rdr.top_clip_rows,
                original_height: rdr.original_height,
            });
            {
                let guard = KittyClipGuard(buffer);
                // Explicit reborrow so the guard keeps ownership of the
                // outer `&mut Buffer` and pops on drop.
                cb(&mut *guard.0, rdr.rect);
                // Guard pops on drop at end of this scope.
            }
            buffer.pop_clip();
        }
    }
    debug_assert!(
        buffer.kitty_clip_info_stack.is_empty(),
        "kitty_clip_info_stack must be empty at end of frame"
    );
    state.hook_states = ctx.hook_states;
    state.named_states = ctx.named_states;
    // Issue #215: hand the keyed-state map back to FrameState so the next
    // frame can pick it up via `Context::new`. Mirrors the `named_states`
    // round-trip exactly.
    state.keyed_states = ctx.keyed_states;
    // Issue #262: hand the partial-chord buffer back so a chord spanning
    // multiple frames survives between them. Same round-trip as `keyed_states`.
    state.chord_states = ctx.chord;
    state.screen_hook_map = ctx.screen_hook_map;
    state.diagnostics.notification_queue = ctx.rollback.notification_queue;
    // Issue #201: persist any in-frame `set_debug_layer` change.
    state.diagnostics.debug_layer = ctx.debug_layer;
    // Issue #208: remember the focus index that finished this frame so the
    // next frame can compute `Response::gained_focus` / `lost_focus`.
    state.focus.prev_focus_index = Some(ctx.focus_index);
    // Issue #217: swap the freshly-built focus name map into the previous
    // slot for next-frame resolution; carry forward any unresolved pending
    // name (deferred until the named widget exists).
    state.focus.focus_name_map_prev = ctx.focus_name_map;
    state.focus.pending_focus_name = ctx.pending_focus_name;

    // Issue #204: reclaim the six per-frame `Vec`/`HashSet` allocations so the
    // next frame reuses the existing capacity instead of allocating fresh.
    // Frame-end invariants (asserted above at lines 1102–1121):
    //   - `rollback.group_stack` and `rollback.text_color_stack` are empty
    //   - `pending_tooltips` is empty
    // `context_stack` is asserted-empty by the consumers in `widgets_*`
    // modules (provider/use_context); on the rare panic-rollback path the
    // checkpoint truncates it back to the saved length, so we still
    // recover capacity.
    //
    // `deferred_draws`: most slots are emptied by the `take()` above, but
    // entries whose `RawDrawRect` had `width == 0 || height == 0` are
    // skipped at the loop guard and remain `Some(_)`. We explicitly
    // `clear()` to drop those callbacks here so they don't outlive the
    // frame; capacity is preserved. (Leaving them would not cause UB —
    // `Context::new` calls `.clear()` on the reclaimed Vec — but dropping
    // promptly matches user expectation that one-shot callbacks don't
    // survive past their frame.)
    //
    // `hovered_groups`: `clear()`-ed at the start of every frame inside
    // `build_hovered_groups`, so the existing entries are harmless to
    // reclaim with content; capacity is preserved.
    ctx.deferred_draws.clear();
    state.context_stack_buf = std::mem::take(&mut ctx.context_stack);
    state.deferred_draws_buf = std::mem::take(&mut ctx.deferred_draws);
    state.group_stack_buf = std::mem::take(&mut ctx.rollback.group_stack);
    state.text_color_stack_buf = std::mem::take(&mut ctx.rollback.text_color_stack);
    state.pending_tooltips_buf = std::mem::take(&mut ctx.pending_tooltips);
    state.hovered_groups_buf = std::mem::take(&mut ctx.hovered_groups);
    // Issue #150: reclaim the drained command Vec so the next `Context::new`
    // picks it up via `mem::take(&mut state.commands_buf)`. After
    // `build_tree(&mut ctx.commands)` the Vec is at `len == 0` with capacity
    // preserved; mirror the #204 reclamation pattern for the other six
    // per-frame buffers.
    state.commands_buf = std::mem::take(&mut ctx.commands);

    let frame_time = frame_start.elapsed();
    let frame_time_us = frame_time.as_micros().min(u128::from(u64::MAX)) as u64;
    let frame_secs = frame_time.as_secs_f32();
    let inst_fps = if frame_secs > 0.0 {
        1.0 / frame_secs
    } else {
        0.0
    };
    state.diagnostics.fps_ema = if state.diagnostics.fps_ema == 0.0 {
        inst_fps
    } else {
        (state.diagnostics.fps_ema * 0.9) + (inst_fps * 0.1)
    };
    if state.diagnostics.debug_mode {
        layout::render_debug_overlay(
            &tree,
            buffer,
            frame_time_us,
            state.diagnostics.fps_ema,
            state.diagnostics.debug_layer,
        );
    }

    FrameKernelResult {
        should_quit: false,
        #[cfg(feature = "crossterm")]
        clipboard_text,
        #[cfg(feature = "crossterm")]
        should_copy_selection,
    }
}

fn run_frame(
    term: &mut impl Backend,
    state: &mut FrameState,
    config: &RunConfig,
    events: Vec<event::Event>,
    f: &mut impl FnMut(&mut context::Context),
) -> io::Result<bool> {
    let size = term.size();
    let kernel = run_frame_kernel(term.buffer_mut(), state, config, size, events, true, f);
    if kernel.should_quit {
        return Ok(false);
    }

    #[cfg(feature = "crossterm")]
    if state.selection.active {
        terminal::apply_selection_overlay(
            term.buffer_mut(),
            &state.selection,
            &state.layout_feedback.prev_content_map,
        );
    }
    #[cfg(feature = "crossterm")]
    if kernel.should_copy_selection {
        let text = terminal::extract_selection_text(
            term.buffer_mut(),
            &state.selection,
            &state.layout_feedback.prev_content_map,
        );
        if !text.is_empty() {
            terminal::copy_to_clipboard(&mut io::stdout(), &text)?;
        }
        state.selection.clear();
    }

    term.flush()?;
    #[cfg(feature = "crossterm")]
    if let Some(text) = kernel.clipboard_text {
        #[allow(clippy::print_stderr)]
        if let Err(e) = terminal::copy_to_clipboard(&mut io::stdout(), &text) {
            eprintln!("[slt] failed to copy to clipboard: {e}");
        }
    }
    state.diagnostics.tick = state.diagnostics.tick.wrapping_add(1);

    Ok(true)
}

#[cfg(feature = "crossterm")]
fn clear_frame_layout_cache(state: &mut FrameState) {
    state.layout_feedback.prev_hit_map.clear();
    state.layout_feedback.prev_group_rects.clear();
    state.layout_feedback.prev_content_map.clear();
    state.layout_feedback.prev_focus_rects.clear();
    state.layout_feedback.prev_focus_groups.clear();
    state.layout_feedback.prev_scroll_infos.clear();
    state.layout_feedback.prev_scroll_rects.clear();
    state.layout_feedback.last_mouse_pos = None;
}

#[cfg(feature = "crossterm")]
fn is_ctrl_c(ev: &Event) -> bool {
    matches!(
        ev,
        Event::Key(event::KeyEvent {
            code: KeyCode::Char('c'),
            modifiers,
            kind: event::KeyEventKind::Press,
        }) if modifiers.contains(KeyModifiers::CONTROL)
    )
}

#[cfg(feature = "crossterm")]
fn sleep_for_fps_cap(max_fps: Option<u32>, render_elapsed: Duration) {
    if let Some(fps) = max_fps.filter(|fps| *fps > 0) {
        let target = Duration::from_secs_f64(1.0 / fps as f64);
        if render_elapsed < target {
            std::thread::sleep(target - render_elapsed);
        }
    }
}

#[cfg(all(test, feature = "crossterm"))]
mod run_loop_tests {
    //! Issue #201 regression tests for the run-loop F12 / Shift+F12
    //! keybinding handler. Exercises [`process_run_loop_event`] directly
    //! so we don't need a real crossterm event source.
    use super::*;

    fn key(modifiers: event::KeyModifiers) -> Event {
        Event::Key(event::KeyEvent {
            code: KeyCode::F(12),
            kind: event::KeyEventKind::Press,
            modifiers,
        })
    }

    #[test]
    fn plain_f12_toggles_debug_mode() {
        let mut state = FrameState::default();
        let mut has_resize = false;
        assert!(!state.diagnostics.debug_mode);
        process_run_loop_event(&key(event::KeyModifiers::NONE), &mut state, &mut has_resize);
        assert!(state.diagnostics.debug_mode);
        process_run_loop_event(&key(event::KeyModifiers::NONE), &mut state, &mut has_resize);
        assert!(!state.diagnostics.debug_mode);
    }

    #[test]
    fn shift_f12_cycles_debug_layer_without_toggling_overlay() {
        let mut state = FrameState::default();
        let mut has_resize = false;
        // Default layer is `All`; debug overlay starts off.
        assert_eq!(state.diagnostics.debug_layer, DebugLayer::All);
        assert!(!state.diagnostics.debug_mode);

        process_run_loop_event(
            &key(event::KeyModifiers::SHIFT),
            &mut state,
            &mut has_resize,
        );
        assert_eq!(state.diagnostics.debug_layer, DebugLayer::TopMost);
        // Cycling does not flip the on/off state.
        assert!(!state.diagnostics.debug_mode);

        process_run_loop_event(
            &key(event::KeyModifiers::SHIFT),
            &mut state,
            &mut has_resize,
        );
        assert_eq!(state.diagnostics.debug_layer, DebugLayer::BaseOnly);

        process_run_loop_event(
            &key(event::KeyModifiers::SHIFT),
            &mut state,
            &mut has_resize,
        );
        assert_eq!(state.diagnostics.debug_layer, DebugLayer::All);
    }

    #[test]
    fn shift_f12_does_not_also_toggle_overlay() {
        // Regression for the modifier disambiguation: pre-fix, the F12
        // arm matched `..` modifiers so Shift+F12 would both cycle the
        // layer AND toggle the overlay on the same press.
        let mut state = FrameState::default();
        let mut has_resize = false;
        let before = state.diagnostics.debug_mode;
        process_run_loop_event(
            &key(event::KeyModifiers::SHIFT),
            &mut state,
            &mut has_resize,
        );
        assert_eq!(
            state.diagnostics.debug_mode, before,
            "Shift+F12 must not flip the on/off toggle"
        );
    }

    #[test]
    fn plain_f12_does_not_cycle_layer() {
        // Symmetric guard: pressing plain F12 must not change the active
        // layer, only the on/off flag.
        let mut state = FrameState::default();
        let mut has_resize = false;
        let before = state.diagnostics.debug_layer;
        process_run_loop_event(&key(event::KeyModifiers::NONE), &mut state, &mut has_resize);
        assert_eq!(state.diagnostics.debug_layer, before);
    }

    // ── Issue #263: RunConfig::handle_suspend ────────────────────────────

    #[test]
    fn handle_suspend_defaults_to_true() {
        assert!(RunConfig::default().handle_suspend);
    }

    #[test]
    fn handle_suspend_builder_opts_out() {
        let cfg = RunConfig::default().handle_suspend(false);
        assert!(!cfg.handle_suspend);
    }

    #[test]
    fn handle_suspend_builder_is_independent_of_ctrl_c() {
        // Toggling suspend must not perturb the unrelated Ctrl+C toggle.
        let cfg = RunConfig::default()
            .handle_ctrl_c(false)
            .handle_suspend(false);
        assert!(!cfg.handle_ctrl_c);
        assert!(!cfg.handle_suspend);

        let cfg = RunConfig::default().handle_suspend(true);
        assert!(cfg.handle_suspend);
        assert!(cfg.handle_ctrl_c, "Ctrl+C default preserved");
    }

    /// End-to-end test of the real signal-delivery wiring: install the
    /// handler, deliver a real `SIGCONT` through signal-hook's registry +
    /// background thread, then drop the guard and confirm it closes the
    /// registration and joins the thread without hanging or panicking.
    ///
    /// `SIGCONT`'s default disposition is "continue", so it is safe to raise on
    /// the running test process — unlike `SIGTSTP`, which would stop the test
    /// runner. The suspend (`SIGTSTP`) sequence itself is covered hermetically
    /// by the `write_suspend_sequence` unit tests in `terminal`.
    #[cfg(unix)]
    #[test]
    fn suspend_handler_installs_delivers_and_tears_down() {
        // In constrained sandboxes signal registration can fail; if so the
        // wiring under test cannot be exercised, so skip rather than flake.
        let Ok(guard) = install_suspend_handler(terminal::test_session_snapshot()) else {
            return;
        };

        // Deliver a real SIGCONT; the background thread must drain it. With no
        // prior SIGTSTP the handler's `has_terminal` guard makes this a no-op
        // re-enter (idempotency), which is exactly what we want to verify does
        // not corrupt state or crash the thread.
        let _ = signal_hook::low_level::raise(signal_hook::consts::SIGCONT);
        std::thread::sleep(Duration::from_millis(50));

        // Dropping the guard closes the registration and joins the thread.
        // If `Handle::close` failed to wake `Signals::forever`, this hangs and
        // the test times out — a real regression signal.
        drop(guard);
    }
}
