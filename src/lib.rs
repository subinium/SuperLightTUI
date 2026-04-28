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
    Anchor, Bar, BarChartConfig, BarDirection, BarGroup, CanvasContext, ContainerBuilder, Context,
    Response, State, TreemapItem, Widget,
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
    HeightSpec, Justify, Margin, Modifiers, Padding, Spacing, Style, Theme, ThemeBuilder,
    ThemeColor, WidgetColors, WidgetTheme, WidthSpec,
};
pub use widgets::{
    AlertLevel, ApprovalAction, BreadcrumbResponse, ButtonVariant, CalendarState,
    CommandPaletteState, ContextItem, DirectoryTreeState, FileEntry, FilePickerState, FormField,
    FormState, GaugeResponse, GridColumn, GutterResponse, HighlightRange, LineGaugeOpts, ListState,
    ModeState, MultiSelectState, PaletteCommand, RadioState, RichLogEntry, RichLogState,
    ScreenState, ScrollState, SelectState, SpinnerState, SplitPaneResponse, SplitPaneState,
    StaticOutput, StreamingMarkdownState, StreamingTextState, TableState, TabsState,
    TextInputState, TextareaState, ToastLevel, ToastMessage, ToastState, ToolApprovalState,
    TreeNode, TreeState, Trend,
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
            widget_theme: style::WidgetTheme::new(),
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DebugLayer {
    /// Outline base + all overlays (default — matches reporter expectation).
    #[default]
    All,
    /// Outline only the topmost overlay, or the base if none.
    TopMost,
    /// Outline only the base layer (legacy behavior).
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
        let render_elapsed = frame_start.elapsed();

        if !poll_events(&mut events, &mut state, config.tick_rate, &mut || {
            term.handle_resize()
        })? {
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
    let mut term = Terminal::new(config.mouse, config.kitty_keyboard, color_depth)?;
    set_terminal_title(&config.title);
    if config.theme.bg != Color::Reset {
        term.theme_bg = Some(config.theme.bg);
    }
    let mut events: Vec<Event> = Vec::new();
    let mut messages: Vec<M> = Vec::new();
    let mut state = FrameState::default();

    loop {
        let frame_start = Instant::now();
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
        let render_elapsed = frame_start.elapsed();

        if !poll_events(&mut events, &mut state, config.tick_rate, &mut || {
            term.handle_resize()
        })? {
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
    let mut term = InlineTerminal::new(height, config.mouse, config.kitty_keyboard, color_depth)?;
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
        let render_elapsed = frame_start.elapsed();

        if !poll_events(&mut events, &mut state, config.tick_rate, &mut || {
            term.handle_resize()
        })? {
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
        color_depth,
    )?;
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
        let render_elapsed = frame_start.elapsed();

        if !poll_events(&mut events, &mut state, config.tick_rate, &mut || {
            term.handle_resize()
        })? {
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

/// Poll for terminal events, handling resize, Ctrl-C, F12 debug toggle,
/// and layout cache invalidation. Returns `Ok(false)` when the loop should exit.
#[cfg(feature = "crossterm")]
fn poll_events(
    events: &mut Vec<Event>,
    state: &mut FrameState,
    tick_rate: Duration,
    on_resize: &mut impl FnMut() -> io::Result<()>,
) -> io::Result<bool> {
    let mut has_resize = false;

    fn process_ev(ev: &Event, state: &mut FrameState, has_resize: &mut bool) {
        match ev {
            Event::Mouse(m) => {
                state.layout_feedback.last_mouse_pos = Some((m.x, m.y));
            }
            Event::FocusLost => {
                state.layout_feedback.last_mouse_pos = None;
            }
            Event::Key(event::KeyEvent {
                code: KeyCode::F(12),
                kind: event::KeyEventKind::Press,
                ..
            }) => {
                state.diagnostics.debug_mode = !state.diagnostics.debug_mode;
            }
            Event::Resize(_, _) => {
                *has_resize = true;
            }
            _ => {}
        }
    }

    if crossterm::event::poll(tick_rate)? {
        let raw = crossterm::event::read()?;
        if let Some(ev) = event::from_crossterm(raw) {
            if is_ctrl_c(&ev) {
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
                if is_ctrl_c(&ev) {
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
        state.screen_hook_map = ctx.screen_hook_map;
        state.diagnostics.notification_queue = ctx.rollback.notification_queue;
        state.diagnostics.debug_layer = ctx.debug_layer;
        // Issue #208 / #217: persist focus tracking state on quit so a later
        // resumed run starts in a sensible place. (Real TUI exits before
        // resuming, but tests reuse `FrameState` across calls.)
        state.focus.prev_focus_index = Some(ctx.focus_index);
        state.focus.focus_name_map_prev = ctx.focus_name_map;
        state.focus.pending_focus_name = ctx.pending_focus_name;
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
    // session. Build_tree consumes the Vec by-value here — the empty placeholder
    // returns to `state.commands_buf` via the `Default` shell from `mem::take`,
    // and full capacity reclamation will land when build_tree's signature is
    // refactored to drain (tracked separately; tree.rs is owned by another agent).
    let commands = std::mem::take(&mut ctx.commands);
    let mut tree = layout::build_tree(commands);
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
