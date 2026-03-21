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
//! Immediate-mode terminal UI for Rust. Two dependencies. Zero `unsafe`.
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
//! - **30+ built-in widgets** — input, textarea, table, list, tabs, button, checkbox, toggle, spinner, progress, toast, separator, help bar, scrollable, chart, bar chart, sparkline, histogram, canvas, grid, select, radio, multi-select, tree, virtual list, command palette, markdown
//! - **Styling** — bold, italic, dim, underline, 256 colors, RGB
//! - **Mouse** — click, hover, drag-to-scroll
//! - **Focus** — automatic Tab/Shift+Tab cycling
//! - **Theming** — dark/light presets or custom
//! - **Animation** — tween and spring primitives with 9 easing functions
//! - **Inline mode** — render below your prompt, no alternate screen
//! - **Async** — optional tokio integration via `async` feature
//! - **Layout debugger** — F12 to visualize container bounds
//!
//! ## Feature Flags
//!
//! | Flag | Description |
//! |------|-------------|
//! | `async` | Enable `run_async()` with tokio channel-based message passing |
//! | `serde` | Enable Serialize/Deserialize for Style, Color, Theme, and layout types |

pub mod anim;
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
pub mod rect;
#[cfg(feature = "crossterm")]
mod sixel;
pub mod style;
pub mod syntax;
#[cfg(feature = "crossterm")]
mod terminal;
pub mod test_utils;
pub mod widgets;

use std::io;
#[cfg(feature = "crossterm")]
use std::io::IsTerminal;
#[cfg(feature = "crossterm")]
use std::io::Write;
#[cfg(feature = "crossterm")]
use std::sync::Once;
use std::time::{Duration, Instant};

#[cfg(feature = "crossterm")]
pub use terminal::{detect_color_scheme, read_clipboard, ColorScheme};
#[cfg(feature = "crossterm")]
use terminal::{InlineTerminal, Terminal};

pub use crate::test_utils::{EventBuilder, TestBackend};
pub use anim::{
    ease_in_cubic, ease_in_out_cubic, ease_in_out_quad, ease_in_quad, ease_linear, ease_out_bounce,
    ease_out_cubic, ease_out_elastic, ease_out_quad, lerp, Keyframes, LoopMode, Sequence, Spring,
    Stagger, Tween,
};
pub use buffer::Buffer;
pub use cell::Cell;
pub use chart::{
    Axis, Candle, ChartBuilder, ChartConfig, ChartRenderer, Dataset, DatasetEntry, GraphType,
    HistogramBuilder, LegendPosition, Marker,
};
pub use context::{
    Bar, BarChartConfig, BarDirection, BarGroup, CanvasContext, ContainerBuilder, Context,
    Response, State, Widget,
};
pub use event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseKind,
};
pub use halfblock::HalfBlockImage;
pub use keymap::{Binding, KeyMap};
pub use layout::Direction;
pub use palette::Palette;
pub use rect::Rect;
pub use style::{
    Align, Border, BorderSides, Breakpoint, Color, ColorDepth, Constraints, ContainerStyle,
    Justify, Margin, Modifiers, Padding, Style, Theme, ThemeBuilder, WidgetColors,
};
pub use widgets::{
    AlertLevel, ApprovalAction, ButtonVariant, CalendarState, CommandPaletteState, ContextItem,
    DirectoryTreeState, FileEntry, FilePickerState, FormField, FormState, ListState,
    MultiSelectState, PaletteCommand, RadioState, RichLogEntry, RichLogState, ScreenState,
    ScrollState, SelectState, SpinnerState, StaticOutput, StreamingMarkdownState,
    StreamingTextState, TableState, TabsState, TextInputState, TextareaState, ToastLevel,
    ToastMessage, ToastState, ToolApprovalState, TreeNode, TreeState, Trend,
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
        self.inner.tick
    }

    /// Returns the smoothed FPS estimate (exponential moving average).
    pub fn fps(&self) -> f32 {
        self.inner.fps_ema
    }

    /// Toggle the debug overlay (same as pressing F12).
    pub fn set_debug(&mut self, enabled: bool) {
        self.inner.debug_mode = enabled;
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

/// Configuration for a TUI run loop.
///
/// Pass to [`run_with`] or [`run_inline_with`] to customize behavior.
/// Use [`Default::default()`] for sensible defaults (16ms tick / 60fps, no mouse, dark theme).
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
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            tick_rate: Duration::from_millis(16),
            mouse: false,
            kitty_keyboard: false,
            theme: Theme::dark(),
            color_depth: None,
            max_fps: Some(60),
            scroll_speed: 1,
            title: None,
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
}

pub(crate) struct FrameState {
    pub hook_states: Vec<Box<dyn std::any::Any>>,
    pub focus_index: usize,
    pub prev_focus_count: usize,
    pub prev_modal_focus_start: usize,
    pub prev_modal_focus_count: usize,
    pub tick: u64,
    pub prev_scroll_infos: Vec<(u32, u32)>,
    pub prev_scroll_rects: Vec<rect::Rect>,
    pub prev_hit_map: Vec<rect::Rect>,
    pub prev_group_rects: Vec<(String, rect::Rect)>,
    pub prev_content_map: Vec<(rect::Rect, rect::Rect)>,
    pub prev_focus_rects: Vec<(usize, rect::Rect)>,
    pub prev_focus_groups: Vec<Option<String>>,
    pub last_mouse_pos: Option<(u32, u32)>,
    pub prev_modal_active: bool,
    pub notification_queue: Vec<(String, ToastLevel, u64)>,
    pub debug_mode: bool,
    pub fps_ema: f32,
    #[cfg(feature = "crossterm")]
    pub selection: terminal::SelectionState,
}

impl Default for FrameState {
    fn default() -> Self {
        Self {
            hook_states: Vec::new(),
            focus_index: 0,
            prev_focus_count: 0,
            prev_modal_focus_start: 0,
            prev_modal_focus_count: 0,
            tick: 0,
            prev_scroll_infos: Vec::new(),
            prev_scroll_rects: Vec::new(),
            prev_hit_map: Vec::new(),
            prev_group_rects: Vec::new(),
            prev_content_map: Vec::new(),
            prev_focus_rects: Vec::new(),
            prev_focus_groups: Vec::new(),
            last_mouse_pos: None,
            prev_modal_active: false,
            notification_queue: Vec::new(),
            debug_mode: false,
            fps_ema: 0.0,
            #[cfg(feature = "crossterm")]
            selection: terminal::SelectionState::default(),
        }
    }
}

/// Run the TUI loop with default configuration.
///
/// Enters alternate screen mode, runs `f` each frame, and exits cleanly on
/// Ctrl+C or when [`Context::quit`] is called.
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
        let _ = write!(io::stdout(), "\x1b]2;{title}\x07");
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
    let mut term = Terminal::new(config.mouse, config.kitty_keyboard, color_depth)?;
    set_terminal_title(&config.title);
    if config.theme.bg != Color::Reset {
        term.theme_bg = Some(config.theme.bg);
    }
    let mut events: Vec<Event> = Vec::new();
    let mut state = FrameState::default();

    loop {
        let frame_start = Instant::now();
        let (w, h) = term.size();
        if w == 0 || h == 0 {
            sleep_for_fps_cap(config.max_fps, frame_start);
            continue;
        }

        if !run_frame(&mut term, &mut state, &config, &events, &mut f)? {
            break;
        }

        events.clear();
        if crossterm::event::poll(config.tick_rate)? {
            let raw = crossterm::event::read()?;
            if let Some(ev) = event::from_crossterm(raw) {
                if is_ctrl_c(&ev) {
                    break;
                }
                if let Event::Resize(_, _) = &ev {
                    term.handle_resize()?;
                }
                events.push(ev);
            }

            while crossterm::event::poll(Duration::ZERO)? {
                let raw = crossterm::event::read()?;
                if let Some(ev) = event::from_crossterm(raw) {
                    if is_ctrl_c(&ev) {
                        return Ok(());
                    }
                    if let Event::Resize(_, _) = &ev {
                        term.handle_resize()?;
                    }
                    events.push(ev);
                }
            }

            for ev in &events {
                if matches!(
                    ev,
                    Event::Key(event::KeyEvent {
                        code: KeyCode::F(12),
                        kind: event::KeyEventKind::Press,
                        ..
                    })
                ) {
                    state.debug_mode = !state.debug_mode;
                }
            }
        }

        update_last_mouse_pos(&mut state, &events);

        if events.iter().any(|e| matches!(e, Event::Resize(_, _))) {
            clear_frame_layout_cache(&mut state);
        }

        sleep_for_fps_cap(config.max_fps, frame_start);
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
    let mut term = Terminal::new(config.mouse, config.kitty_keyboard, color_depth)?;
    set_terminal_title(&config.title);
    if config.theme.bg != Color::Reset {
        term.theme_bg = Some(config.theme.bg);
    }
    let mut events: Vec<Event> = Vec::new();
    let mut state = FrameState::default();

    loop {
        let frame_start = Instant::now();
        let mut messages: Vec<M> = Vec::new();
        while let Ok(message) = rx.try_recv() {
            messages.push(message);
        }

        let (w, h) = term.size();
        if w == 0 || h == 0 {
            sleep_for_fps_cap(config.max_fps, frame_start);
            continue;
        }

        let mut render = |ctx: &mut Context| {
            f(ctx, &mut messages);
        };
        if !run_frame(&mut term, &mut state, &config, &events, &mut render)? {
            break;
        }

        events.clear();
        if crossterm::event::poll(config.tick_rate)? {
            let raw = crossterm::event::read()?;
            if let Some(ev) = event::from_crossterm(raw) {
                if is_ctrl_c(&ev) {
                    break;
                }
                if let Event::Resize(_, _) = &ev {
                    term.handle_resize()?;
                    clear_frame_layout_cache(&mut state);
                }
                events.push(ev);
            }

            while crossterm::event::poll(Duration::ZERO)? {
                let raw = crossterm::event::read()?;
                if let Some(ev) = event::from_crossterm(raw) {
                    if is_ctrl_c(&ev) {
                        return Ok(());
                    }
                    if let Event::Resize(_, _) = &ev {
                        term.handle_resize()?;
                        clear_frame_layout_cache(&mut state);
                    }
                    events.push(ev);
                }
            }
        }

        update_last_mouse_pos(&mut state, &events);

        sleep_for_fps_cap(config.max_fps, frame_start);
    }

    Ok(())
}

/// Run the TUI in inline mode with default configuration.
///
/// Renders `height` rows directly below the current cursor position without
/// entering alternate screen mode. Useful for CLI tools that want a small
/// interactive widget below the prompt.
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
    let mut term = InlineTerminal::new(height, config.mouse, color_depth)?;
    set_terminal_title(&config.title);
    if config.theme.bg != Color::Reset {
        term.theme_bg = Some(config.theme.bg);
    }
    let mut events: Vec<Event> = Vec::new();
    let mut state = FrameState::default();

    loop {
        let frame_start = Instant::now();
        let (w, h) = term.size();
        if w == 0 || h == 0 {
            sleep_for_fps_cap(config.max_fps, frame_start);
            continue;
        }

        if !run_frame(&mut term, &mut state, &config, &events, &mut f)? {
            break;
        }

        events.clear();
        if crossterm::event::poll(config.tick_rate)? {
            let raw = crossterm::event::read()?;
            if let Some(ev) = event::from_crossterm(raw) {
                if is_ctrl_c(&ev) {
                    break;
                }
                if let Event::Resize(_, _) = &ev {
                    term.handle_resize()?;
                }
                events.push(ev);
            }

            while crossterm::event::poll(Duration::ZERO)? {
                let raw = crossterm::event::read()?;
                if let Some(ev) = event::from_crossterm(raw) {
                    if is_ctrl_c(&ev) {
                        return Ok(());
                    }
                    if let Event::Resize(_, _) = &ev {
                        term.handle_resize()?;
                    }
                    events.push(ev);
                }
            }

            for ev in &events {
                if matches!(
                    ev,
                    Event::Key(event::KeyEvent {
                        code: KeyCode::F(12),
                        kind: event::KeyEventKind::Press,
                        ..
                    })
                ) {
                    state.debug_mode = !state.debug_mode;
                }
            }
        }

        update_last_mouse_pos(&mut state, &events);

        if events.iter().any(|e| matches!(e, Event::Resize(_, _))) {
            clear_frame_layout_cache(&mut state);
        }

        sleep_for_fps_cap(config.max_fps, frame_start);
    }

    Ok(())
}

/// Run the TUI in static-output mode.
///
/// Static lines written through [`StaticOutput`] are printed into terminal
/// scrollback, while the interactive UI stays rendered in a fixed-height inline
/// area at the bottom.
#[cfg(feature = "crossterm")]
pub fn run_static(
    output: &mut StaticOutput,
    dynamic_height: u32,
    mut f: impl FnMut(&mut Context),
) -> io::Result<()> {
    let config = RunConfig::default();
    if !io::stdout().is_terminal() {
        return Ok(());
    }

    install_panic_hook();

    let initial_lines = output.drain_new();
    write_static_lines(&initial_lines)?;

    let color_depth = config.color_depth.unwrap_or_else(ColorDepth::detect);
    let mut term = InlineTerminal::new(dynamic_height, config.mouse, color_depth)?;
    set_terminal_title(&config.title);
    if config.theme.bg != Color::Reset {
        term.theme_bg = Some(config.theme.bg);
    }

    let mut events: Vec<Event> = Vec::new();
    let mut state = FrameState::default();

    loop {
        let frame_start = Instant::now();
        let (w, h) = term.size();
        if w == 0 || h == 0 {
            sleep_for_fps_cap(config.max_fps, frame_start);
            continue;
        }

        let new_lines = output.drain_new();
        write_static_lines(&new_lines)?;

        if !run_frame(&mut term, &mut state, &config, &events, &mut f)? {
            break;
        }

        events.clear();
        if crossterm::event::poll(config.tick_rate)? {
            let raw = crossterm::event::read()?;
            if let Some(ev) = event::from_crossterm(raw) {
                if is_ctrl_c(&ev) {
                    break;
                }
                if let Event::Resize(_, _) = &ev {
                    term.handle_resize()?;
                }
                events.push(ev);
            }

            while crossterm::event::poll(Duration::ZERO)? {
                let raw = crossterm::event::read()?;
                if let Some(ev) = event::from_crossterm(raw) {
                    if is_ctrl_c(&ev) {
                        return Ok(());
                    }
                    if let Event::Resize(_, _) = &ev {
                        term.handle_resize()?;
                    }
                    events.push(ev);
                }
            }

            for ev in &events {
                if matches!(
                    ev,
                    Event::Key(event::KeyEvent {
                        code: KeyCode::F(12),
                        kind: event::KeyEventKind::Press,
                        ..
                    })
                ) {
                    state.debug_mode = !state.debug_mode;
                }
            }
        }

        update_last_mouse_pos(&mut state, &events);

        if events.iter().any(|e| matches!(e, Event::Resize(_, _))) {
            clear_frame_layout_cache(&mut state);
        }

        sleep_for_fps_cap(config.max_fps, frame_start);
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

fn run_frame(
    term: &mut impl Backend,
    state: &mut FrameState,
    config: &RunConfig,
    events: &[event::Event],
    f: &mut impl FnMut(&mut context::Context),
) -> io::Result<bool> {
    let frame_start = Instant::now();
    let (w, h) = term.size();
    let mut ctx = Context::new(events.to_vec(), w, h, state, config.theme);
    ctx.is_real_terminal = true;
    ctx.set_scroll_speed(config.scroll_speed);

    f(&mut ctx);
    ctx.process_focus_keys();
    ctx.render_notifications();
    ctx.emit_pending_tooltips();

    if ctx.should_quit {
        return Ok(false);
    }
    state.prev_modal_active = ctx.modal_active;
    state.prev_modal_focus_start = ctx.modal_focus_start;
    state.prev_modal_focus_count = ctx.modal_focus_count;
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
                    state
                        .selection
                        .mouse_down(mouse.x, mouse.y, &state.prev_content_map);
                }
                event::MouseKind::Drag(event::MouseButton::Left) => {
                    state
                        .selection
                        .mouse_drag(mouse.x, mouse.y, &state.prev_content_map);
                }
                event::MouseKind::Up(event::MouseButton::Left) => {
                    should_copy_selection = state.selection.active;
                }
                _ => {}
            }
        }
    }

    state.focus_index = ctx.focus_index;
    state.prev_focus_count = ctx.focus_count;

    let mut tree = layout::build_tree(std::mem::take(&mut ctx.commands));
    let area = crate::rect::Rect::new(0, 0, w, h);
    layout::compute(&mut tree, area);
    let fd = layout::collect_all(&tree);
    state.prev_scroll_infos = fd.scroll_infos;
    state.prev_scroll_rects = fd.scroll_rects;
    state.prev_hit_map = fd.hit_areas;
    state.prev_group_rects = fd.group_rects;
    state.prev_content_map = fd.content_areas;
    state.prev_focus_rects = fd.focus_rects;
    state.prev_focus_groups = fd.focus_groups;
    layout::render(&tree, term.buffer_mut());
    let raw_rects = fd.raw_draw_rects;
    for rdr in raw_rects {
        if rdr.rect.width == 0 || rdr.rect.height == 0 {
            continue;
        }
        if let Some(cb) = ctx
            .deferred_draws
            .get_mut(rdr.draw_id)
            .and_then(|c| c.take())
        {
            let buf = term.buffer_mut();
            buf.push_clip(rdr.rect);
            buf.kitty_clip_info = Some((rdr.top_clip_rows, rdr.original_height));
            cb(buf, rdr.rect);
            buf.kitty_clip_info = None;
            buf.pop_clip();
        }
    }
    state.hook_states = ctx.hook_states;
    state.notification_queue = ctx.notification_queue;

    let frame_time = frame_start.elapsed();
    let frame_time_us = frame_time.as_micros().min(u128::from(u64::MAX)) as u64;
    let frame_secs = frame_time.as_secs_f32();
    let inst_fps = if frame_secs > 0.0 {
        1.0 / frame_secs
    } else {
        0.0
    };
    state.fps_ema = if state.fps_ema == 0.0 {
        inst_fps
    } else {
        (state.fps_ema * 0.9) + (inst_fps * 0.1)
    };
    if state.debug_mode {
        layout::render_debug_overlay(&tree, term.buffer_mut(), frame_time_us, state.fps_ema);
    }

    #[cfg(feature = "crossterm")]
    if state.selection.active {
        terminal::apply_selection_overlay(
            term.buffer_mut(),
            &state.selection,
            &state.prev_content_map,
        );
    }
    #[cfg(feature = "crossterm")]
    if should_copy_selection {
        let text = terminal::extract_selection_text(
            term.buffer_mut(),
            &state.selection,
            &state.prev_content_map,
        );
        if !text.is_empty() {
            terminal::copy_to_clipboard(&mut io::stdout(), &text)?;
        }
        state.selection.clear();
    }

    term.flush()?;
    #[cfg(feature = "crossterm")]
    if let Some(text) = clipboard_text {
        #[allow(clippy::print_stderr)]
        if let Err(e) = terminal::copy_to_clipboard(&mut io::stdout(), &text) {
            eprintln!("[slt] failed to copy to clipboard: {e}");
        }
    }
    state.tick = state.tick.wrapping_add(1);

    Ok(true)
}

#[cfg(feature = "crossterm")]
fn update_last_mouse_pos(state: &mut FrameState, events: &[Event]) {
    for ev in events {
        match ev {
            Event::Mouse(mouse) => {
                state.last_mouse_pos = Some((mouse.x, mouse.y));
            }
            Event::FocusLost => {
                state.last_mouse_pos = None;
            }
            _ => {}
        }
    }
}

#[cfg(feature = "crossterm")]
fn clear_frame_layout_cache(state: &mut FrameState) {
    state.prev_hit_map.clear();
    state.prev_group_rects.clear();
    state.prev_content_map.clear();
    state.prev_focus_rects.clear();
    state.prev_focus_groups.clear();
    state.prev_scroll_infos.clear();
    state.prev_scroll_rects.clear();
    state.last_mouse_pos = None;
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
fn sleep_for_fps_cap(max_fps: Option<u32>, frame_start: Instant) {
    if let Some(fps) = max_fps.filter(|fps| *fps > 0) {
        let target = Duration::from_secs_f64(1.0 / fps as f64);
        let elapsed = frame_start.elapsed();
        if elapsed < target {
            std::thread::sleep(target - elapsed);
        }
    }
}
