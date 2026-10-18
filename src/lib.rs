//! SuperLightTUI (SLT) — an immediate-mode, flexbox-layout terminal UI
//! library. Small core. Zero `unsafe`.
//!
//! Build a TUI as easily as a web page: write a closure, SLT calls it
//! every frame. State lives in your code; layout is described every
//! frame with `row()` / `col()` / `container()`; styling uses
//! Tailwind-inspired method chaining; focus and events are threaded
//! through a single [`Context`] parameter. SLT handles layout, diffing,
//! and rendering.
//!
//! # Quick Start
//!
//! ```no_run
//! fn main() -> std::io::Result<()> {
//!     slt::run(|ui| {
//!         ui.text("hello, world");
//!     })
//! }
//! ```
//!
//! `slt::run` uses [`RunConfig::default()`], which leaves mouse capture **off**
//! and exits on Ctrl+C. Keyboard focus (Tab / Shift+Tab) and Enter/Space
//! activation work out of the box; for click, hover, wheel scroll, and
//! drag, opt in with `slt::run_with(RunConfig::default().mouse(true), |ui| { ... })`.
//!
//! # Features
//!
//! - **Flexbox layout** — `row()`, `col()`, `gap()`, `grow()`
//! - **50+ built-in widgets** — input, textarea, table, list, tabs, button, checkbox, toggle, spinner, progress, toast, slider, separator, help bar, scrollable, chart, bar chart, stacked bar chart, sparkline, histogram, heatmap, treemap, candlestick, canvas, grid, select, radio, multi-select, tree, virtual list, command palette, markdown, alert, badge, stat, breadcrumb, accordion, code block, big text, image, modal, tooltip, form, calendar, file picker, qr code
//! - **Styling** — bold, italic, dim, underline, 256 colors, RGB
//! - **Mouse** — click, hover, drag-to-scroll (opt-in via [`RunConfig::mouse()`])
//! - **Focus** — automatic Tab/Shift+Tab cycling
//! - **Theming** — 11 color presets, semantic tokens (`ThemeColor`), spacing scale, contrast helpers
//! - **Animation** — tween and spring primitives with 9 easing functions
//! - **Inline mode** — render below your prompt, no alternate screen
//! - **Async** — optional tokio integration via `async` feature
//! - **Layout debugger** — F12 to visualize container bounds
//!
//! # Feature Flags
//!
//! | Flag | Description |
//! |------|-------------|
//! | `crossterm` | Built-in terminal runtime (`run`, `run_inline`, clipboard query helpers). Enabled by default. |
//! | `bidi` | Reorder right-to-left text (Hebrew, Arabic, …) to visual order per UAX #9 before rendering. Enabled by default; pure-LTR text takes a zero-cost fast path. Since 0.21.0. |
//! | `async` | Enable `run_async()` with tokio channel-based message passing (implies `crossterm`) |
//! | `serde` | Enable Serialize/Deserialize for Style, Color, Theme, and layout types |
//! | `theme-watch` | TOML theme hot reload via `ThemeWatcher` (implies `serde`, adds a filesystem watcher) |
//! | `image` | Enable image-loading helpers for terminal image widgets |
//! | `qrcode` | Enable `ui.qr_code(...)` |
//! | `kitty-compress` | zlib-compress Kitty graphics protocol payloads |
//! | `syntax` / `syntax-*` | Enable tree-sitter syntax highlighting (all bundled grammars, or one language) |
//! | `full` | `crossterm`, `async`, `serde`, `theme-watch`, `image`, `qrcode`, `kitty-compress`, `bidi`. Does **not** include `syntax` or `pty-test`. |
//! | `ratatui` | Render ratatui widgets inside SLT containers via `ContainerBuilder::ratatui` / `ratatui_widget` and `Color`/`Modifiers`/`Style` conversions (adds `ratatui-core` 0.1, no ratatui backend). Not part of `full`. Since 0.26.0. |
//! | `pty-test` | Dev-only test harness: exposes `PtyBackend` / `PtyFrame`, which drive the real terminal flush pipeline into an in-process byte sink for escape-code assertions. Not for application use. |
//!
//! # Learn More
//!
//! - Guides index: <https://github.com/subinium/SuperLightTUI/blob/main/docs/README.md>
//! - Quick start: <https://github.com/subinium/SuperLightTUI/blob/main/docs/QUICK_START.md>
//! - Design principles: <https://github.com/subinium/SuperLightTUI/blob/main/docs/DESIGN_PRINCIPLES.md>
//! - Backends and run loops: <https://github.com/subinium/SuperLightTUI/blob/main/docs/BACKENDS.md>
//! - Testing: <https://github.com/subinium/SuperLightTUI/blob/main/docs/TESTING.md>
//! - Debugging: <https://github.com/subinium/SuperLightTUI/blob/main/docs/DEBUGGING.md>

// Safety: the shipping library is 100% safe. Unit tests are excused only
// because edition 2024 made `std::env::set_var`/`remove_var` `unsafe`, and a
// few `#[cfg(test)]` terminal-detection helpers must mutate process env (they
// serialize via a mutex). `forbid` stays on for every non-test build.
#![cfg_attr(not(test), forbid(unsafe_code))]
#![cfg_attr(test, deny(unsafe_code))]
// Cross-target lints (rustdoc links, rust-2018-idioms) are configured
// centrally in [workspace.lints] and applied via `[lints] workspace = true` in
// Cargo.toml. The lints below stay here as lib-only inner attributes on
// purpose: `[lints]` is package-scoped and would otherwise fire on the
// package's example binaries and integration tests, which legitimately expose
// undocumented `pub` helpers, print to stdout, and unwrap. The cfg-conditional
// unsafe_code policy above likewise can't live in workspace.lints.
#![warn(missing_docs)]
#![warn(unreachable_pub)]
#![deny(clippy::unwrap_in_result)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::dbg_macro)]
#![warn(clippy::print_stdout)]
#![warn(clippy::print_stderr)]
#![cfg_attr(docsrs, feature(doc_cfg))]

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
#[cfg(feature = "crossterm")]
mod iterm;
/// Keyboard shortcut mapping.
pub mod keymap;
/// Flexbox layout engine and command tree.
pub mod layout;
/// Color palettes (Tailwind-style).
pub mod palette;
#[cfg(feature = "ratatui")]
#[cfg_attr(docsrs, doc(cfg(feature = "ratatui")))]
pub mod ratatui_compat;
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
mod width;

use std::io;
use std::time::Duration;

mod clock;
mod runtime;

#[cfg(all(feature = "crossterm", feature = "async"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "crossterm", feature = "async"))))]
pub use runtime::{AsyncRunError, AsyncRunHandle, AsyncSender, run_async, run_async_with};
pub use runtime::{DebugLayer, RunConfig};
#[cfg(feature = "crossterm")]
#[cfg_attr(docsrs, doc(cfg(feature = "crossterm")))]
pub use runtime::{run, run_inline, run_inline_with, run_static, run_static_with, run_with};

// Crate-internal run-loop items that the rest of the crate reaches through
// the crate root (they were defined here before the run loop moved).
#[cfg(all(test, feature = "crossterm"))]
pub(crate) use runtime::localize_inline_events;
#[cfg(feature = "crossterm")]
pub(crate) use runtime::sanitize_terminal_text;
#[cfg(feature = "async")]
pub(crate) use runtime::with_recoverable_panics;
pub(crate) use runtime::{
    DOUBLE_CLICK_WINDOW, FrameState, KEYMAP_REGISTRY_NAMED_STATE_KEY, LayoutFeedbackState,
    STATIC_LOG_NAMED_STATE_KEY, catch_recoverable_unwind, run_frame, run_frame_kernel,
};
#[cfg(test)]
pub(crate) use runtime::{clear_frame_layout_cache, input_frame};

/// Re-export of the [`crossterm`] crate (issue #278) so callers can name the
/// input type accepted by [`event::from_crossterm`] without depending on — and
/// risking a version mismatch against — crossterm directly.
#[cfg(feature = "crossterm")]
#[cfg_attr(docsrs, doc(cfg(feature = "crossterm")))]
pub use crossterm;

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
pub use terminal::__bench_flush_buffer_diff_mut_with_buf;
#[cfg(feature = "crossterm")]
#[doc(hidden)]
pub use terminal::__bench_flush_kitty;
#[cfg(feature = "crossterm")]
#[doc(hidden)]
pub use terminal::{__BenchKittyFixture, __bench_new_kitty_fixture};
#[cfg(feature = "crossterm")]
#[doc(hidden)]
pub use terminal::{__BenchSprixelFixture, __bench_flush_sprixels, __bench_new_sprixel_fixture};
/// Runtime terminal capability probe (issue #264): read-only [`Capabilities`]
/// snapshot plus the [`Blitter`] ladder it drives. Diagnostics-only — image
/// rendering routes through the ladder automatically.
#[cfg(feature = "crossterm")]
#[cfg_attr(docsrs, doc(cfg(feature = "crossterm")))]
pub use terminal::{Blitter, BlitterSupport, Capabilities, capabilities};
#[cfg(feature = "crossterm")]
#[cfg_attr(docsrs, doc(cfg(feature = "crossterm")))]
pub use terminal::{ColorScheme, detect_color_scheme, read_clipboard};
/// Concrete crossterm terminal backends, exposed (issue #278) so external
/// integrations can drive SLT's render pipeline with their own event loop —
/// pair with [`event::from_crossterm`]. Most apps should use [`run`] /
/// [`run_inline`], which build and drive these internally.
#[cfg(feature = "crossterm")]
#[cfg_attr(docsrs, doc(cfg(feature = "crossterm")))]
pub use terminal::{InlineTerminal, Terminal};

pub use crate::test_utils::{EventBuilder, FrameRecord, TestBackend, TestSequence};
/// PTY/sink test harness for end-to-end escape-byte assertions (issue #274).
/// Gated behind the dev-only `pty-test` feature; absent from default builds.
#[cfg(feature = "pty-test")]
#[cfg_attr(docsrs, doc(cfg(feature = "pty-test")))]
pub use crate::test_utils::{PtyBackend, PtyFrame};
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
    Anchor, Bar, BarChartConfig, BarDirection, BarGroup, Breadcrumb, CanvasContext, CanvasError,
    CodeBlock, ContainerBuilder, Context, Gauge, GutterOpts, LineGauge, Memo, Response, State,
    TreemapItem, Widget,
};
// Issues #446 / #468: terminal metadata and clipboard outcome types.
pub use context::{ClipboardStatus, CursorShape, ProgramState, ProgramStatus, Progress};
// Issue #234: opaque handle from `Context::spawn`, gated behind `async`.
#[cfg(feature = "async")]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
pub use context::{TaskHandle, TaskOutcome};
pub use event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, ModifierKey, MouseButton, MouseEvent,
    MouseKind,
};
pub use halfblock::HalfBlockImage;
pub use keymap::{Binding, KeyMap, PublishedKeymap, WidgetKeyHelp};
pub use layout::Direction;
pub use palette::Palette;
pub use rect::Rect;
#[cfg(feature = "theme-watch")]
#[cfg_attr(docsrs, doc(cfg(feature = "theme-watch")))]
pub use style::ThemeWatcher;
pub use style::{
    Align, Border, BorderSides, Breakpoint, Color, ColorDepth, ColorParseError, Constraints,
    ContainerStyle, HeightSpec, Justify, Margin, Modifiers, Padding, Spacing, Style, SyntaxPalette,
    Theme, ThemeBuilder, ThemeColor, UnderlineStyle, WidgetColors, WidgetTheme, WidthSpec,
};
#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
pub use style::{ThemeFile, ThemeLoadError};
#[cfg(feature = "async")]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
pub use widgets::AsyncValidation;
pub use widgets::validators;
pub use widgets::{
    AlertLevel, ApprovalAction, BreadcrumbResponse, ButtonVariant, CalDate, CalendarSelect,
    CalendarState, ChordState, ColorPickerState, CommandPaletteState, ContextItem,
    DEFAULT_CHORD_TIMEOUT_TICKS, DirectoryTreeState, FileEntry, FilePickerScanError,
    FilePickerScanOperation, FilePickerScanStatus, FilePickerState, FormField, FormFieldResponse,
    FormState, GaugeResponse, GridColumn, GutterResponse, HighlightRange, ListResponse, ListState,
    ModeState, MultiSelectState, NumberInputState, PaginatorState, PaginatorStyle, PaletteCommand,
    PickerMode, RadioState, RichLogEntry, RichLogState, SchedulerState, ScreenState, ScrollState,
    SelectState, SliderOpts, SpinnerPreset, SpinnerState, SplitPaneResponse, SplitPaneState,
    StaticOutput, StreamingMarkdownState, StreamingTextState, TableColumn, TableState, TabsState,
    TextInputState, TextareaState, ToastLevel, ToastMessage, ToastState, ToolApprovalState,
    TreeNode, TreeState, Trend, ValidateTrigger, Validator,
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

    /// Returns whether this backend owns a real terminal-style session.
    ///
    /// Custom backends should keep the default `false`, which prevents process
    /// stdout clipboard writes, capability probes, and terminal panic recovery.
    /// The built-in [`Terminal`] and [`InlineTerminal`] backends opt in.
    #[doc(hidden)]
    fn owns_terminal_session(&self) -> bool {
        false
    }
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

    /// Whether input after a focus-navigation boundary awaits the next frame.
    ///
    /// Custom loops should schedule another frame before blocking for new
    /// events. Built-in native and browser loops do this automatically.
    pub fn has_pending_input(&self) -> bool {
        !self.inner.pending_input.is_empty()
    }

    /// Returns the smoothed FPS estimate (exponential moving average).
    pub fn fps_f64(&self) -> f64 {
        f64::from(self.inner.diagnostics.fps_ema)
    }

    /// Wall-clock interval between the starts of the two latest frames.
    pub fn frame_interval(&self) -> Duration {
        self.inner.diagnostics.frame_interval
    }

    /// Time spent building, laying out and rendering the latest frame.
    pub fn render_duration(&self) -> Duration {
        self.inner.diagnostics.render_duration
    }

    /// Time spent presenting the latest frame through the backend.
    pub fn flush_duration(&self) -> Duration {
        self.inner.diagnostics.flush_duration
    }

    /// Deprecated `f32` alias for [`fps_f64`](Self::fps_f64).
    #[deprecated(
        since = "0.22.2",
        note = "use AppState::fps_f64() to keep public float APIs on f64"
    )]
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
    let terminal_side_effects = backend.owns_terminal_session();
    run_frame(
        backend,
        &mut state.inner,
        config,
        events,
        terminal_side_effects,
        f,
    )
}
