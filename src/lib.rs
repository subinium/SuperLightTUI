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
#[cfg(feature = "crossterm")]
mod iterm;
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
use std::time::Duration;

mod clock;
use clock::Instant;

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
    FilePickerScanOperation, FilePickerScanStatus, FilePickerState, FormField, FormState,
    GaugeResponse, GridColumn, GutterResponse, HighlightRange, ListResponse, ListState, ModeState,
    MultiSelectState, NumberInputState, PaginatorState, PaginatorStyle, PaletteCommand, PickerMode,
    RadioState, RichLogEntry, RichLogState, SchedulerState, ScreenState, ScrollState, SelectState,
    SliderOpts, SpinnerPreset, SpinnerState, SplitPaneResponse, SplitPaneState, StaticOutput,
    StreamingMarkdownState, StreamingTextState, TableColumn, TableState, TabsState, TextInputState,
    TextareaState, ToastLevel, ToastMessage, ToastState, ToolApprovalState, TreeNode, TreeState,
    Trend, ValidateTrigger, Validator,
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

#[cfg(feature = "crossterm")]
type PanicHook = Box<dyn Fn(&std::panic::PanicHookInfo<'_>) + Send + Sync + 'static>;

std::thread_local! {
    static RECOVERABLE_PANICS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static TERMINAL_PANIC_OWNER: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

struct PanicScopeGuard(&'static std::thread::LocalKey<std::cell::Cell<usize>>);

impl PanicScopeGuard {
    fn enter(slot: &'static std::thread::LocalKey<std::cell::Cell<usize>>) -> Self {
        slot.with(|depth| depth.set(depth.get().saturating_add(1)));
        Self(slot)
    }
}

impl Drop for PanicScopeGuard {
    fn drop(&mut self) {
        self.0
            .with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

pub(crate) fn with_recoverable_panics<T>(f: impl FnOnce() -> T) -> T {
    let _scope = PanicScopeGuard::enter(&RECOVERABLE_PANICS);
    f()
}

pub(crate) fn catch_recoverable_unwind<T>(f: impl FnOnce() -> T) -> std::thread::Result<T> {
    with_recoverable_panics(|| std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)))
}

#[cfg(feature = "crossterm")]
struct PanicHookState {
    active_sessions: usize,
    installed: bool,
    previous: Option<PanicHook>,
}

#[cfg(feature = "crossterm")]
static PANIC_HOOK_STATE: std::sync::Mutex<PanicHookState> = std::sync::Mutex::new(PanicHookState {
    active_sessions: 0,
    installed: false,
    previous: None,
});

#[cfg(feature = "crossterm")]
struct PanicHookGuard;

#[cfg(feature = "crossterm")]
impl Drop for PanicHookGuard {
    fn drop(&mut self) {
        let mut state = PANIC_HOOK_STATE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.active_sessions = state.active_sessions.saturating_sub(1);

        // `take_hook` panics while the current thread is unwinding. In that
        // case the dispatcher remains installed but inactive and forwards
        // directly to the previous hook. A later normal session restores it.
        if state.active_sessions == 0 && state.installed && !std::thread::panicking() {
            let dispatcher = std::panic::take_hook();
            drop(dispatcher);
            if let Some(previous) = state.previous.take() {
                std::panic::set_hook(previous);
            }
            state.installed = false;
        }
    }
}

#[allow(clippy::print_stderr)]
#[cfg(feature = "crossterm")]
fn slt_panic_hook(panic_info: &std::panic::PanicHookInfo<'_>) {
    if cfg!(panic = "unwind") && RECOVERABLE_PANICS.with(|depth| depth.get() > 0) {
        return;
    }
    let active = PANIC_HOOK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .active_sessions
        > 0;
    let restored = active
        && (cfg!(panic = "abort") || TERMINAL_PANIC_OWNER.with(|depth| depth.get() > 0))
        && terminal::cleanup_after_panic();

    if restored {
        eprintln!("\n\x1b[1;31m━━━ SLT Panic ━━━\x1b[0m\n");
        if let Some(location) = panic_info.location() {
            eprintln!(
                "\x1b[90m{}:{}:{}\x1b[0m",
                location.file(),
                location.line(),
                location.column()
            );
        }
        if let Some(msg) = panic_info.payload().downcast_ref::<&str>() {
            eprintln!("\x1b[1m{msg}\x1b[0m");
        } else if let Some(msg) = panic_info.payload().downcast_ref::<String>() {
            eprintln!("\x1b[1m{msg}\x1b[0m");
        }
        eprintln!(
            "\n\x1b[90mTerminal state restored. Report bugs at https://github.com/subinium/SuperLightTUI/issues\x1b[0m\n"
        );
    }

    let state = PANIC_HOOK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(previous) = state.previous.as_ref() {
        previous(panic_info);
    }
}

#[cfg(feature = "crossterm")]
fn install_panic_hook() -> PanicHookGuard {
    let mut state = PANIC_HOOK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if !state.installed {
        state.previous = Some(std::panic::take_hook());
        std::panic::set_hook(Box::new(slt_panic_hook));
        state.installed = true;
    }
    state.active_sessions = state.active_sessions.saturating_add(1);
    PanicHookGuard
}

#[cfg(feature = "crossterm")]
fn with_session_panic_hook<T>(f: impl FnOnce() -> T) -> T {
    let guard = install_panic_hook();
    let owner = PanicScopeGuard::enter(&TERMINAL_PANIC_OWNER);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    drop(owner);
    drop(guard);
    match result {
        Ok(value) => value,
        Err(payload) => std::panic::resume_unwind(payload),
    }
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

#[cfg(all(feature = "crossterm", unix))]
fn suspend_current_session(snapshot: terminal::SessionSnapshot) -> io::Result<()> {
    terminal::suspend_to_shell(&snapshot);
    let result = signal_hook::low_level::emulate_default_handler(signal_hook::consts::SIGTSTP);
    terminal::resume_from_shell(&snapshot);
    result?;
    Ok(())
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
    /// Maximum number of external async messages delivered to one frame.
    ///
    /// Messages beyond this count remain queued in FIFO order for later
    /// frames. Defaults to the async channel capacity (100).
    pub async_message_budget: usize,
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
    pub focus_name_map_buf: std::collections::HashMap<String, usize>,
    /// Issue #217: a name passed to `focus_by_name(...)` that has not yet
    /// been resolved. Consumed once the matching registration is found in
    /// `focus_name_map_prev`.
    pub pending_focus_name: Option<String>,
}

/// v0.21.1: maximum gap between two same-cell left clicks for them to count as
/// a double-click. Tuned to the common desktop default (~400ms).
pub(crate) const DOUBLE_CLICK_WINDOW: std::time::Duration = std::time::Duration::from_millis(400);

#[derive(Default)]
pub(crate) struct LayoutFeedbackState {
    pub size: Option<(u32, u32)>,
    #[cfg(feature = "crossterm")]
    pub origin_row: Option<u32>,
    /// `(content_extent, viewport_extent, is_horizontal)` per scrollable last
    /// frame (#247). `is_horizontal` selects which `ScrollState` axis the
    /// `scrollable` binding updates.
    pub prev_scroll_infos: Vec<(u32, u32, bool)>,
    pub prev_scroll_rects: Vec<rect::Rect>,
    pub prev_hit_map: Vec<rect::Rect>,
    pub prev_allocated_areas: Vec<rect::Rect>,
    pub prev_group_rects: Vec<(std::sync::Arc<str>, rect::Rect)>,
    pub prev_content_map: Vec<(rect::Rect, rect::Rect)>,
    pub prev_focus_rects: Vec<(usize, rect::Rect)>,
    pub prev_focus_groups: Vec<Option<std::sync::Arc<str>>>,
    pub last_mouse_pos: Option<(u32, u32)>,
    /// v0.21.1: wall-clock time of the previous left-click `Down`, used to
    /// detect a double-click (a second click on the same cell within
    /// `DOUBLE_CLICK_WINDOW`, ~400ms). `None` after a double-click fires (so a
    /// triple click is not double-counted) or when no click has occurred.
    pub last_click_at: Option<Instant>,
    /// v0.21.1: cell position of the previous left-click `Down`, paired with
    /// `last_click_at` for same-cell double-click detection.
    pub last_click_pos: Option<(u32, u32)>,
}

#[derive(Default)]
pub(crate) struct DiagnosticsState {
    pub tick: u64,
    pub notification_queue: Vec<(String, ToastLevel, u64)>,
    pub debug_mode: bool,
    pub debug_layer: DebugLayer,
    /// Issue #268: whether the devtools inspector panel (Ctrl+F12) is active.
    /// Independent of `debug_mode`/`debug_layer`. Round-trips through
    /// `Context::inspector_mode` like `debug_layer` so `set_inspector` persists.
    pub inspector_mode: bool,
    pub fps_ema: f32,
    pub frame_started: Option<Instant>,
    pub clock_override: Option<Instant>,
    pub frame_interval: Duration,
    pub render_duration: Duration,
    pub flush_duration: Duration,
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
    pub consumed_buf: Vec<bool>,
    pub events_buf: Vec<Event>,
    pub geometry_stack_buf: Vec<(usize, Option<usize>)>,
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
    /// Issue #248: persistent frame-clock timer table. Round-tripped through
    /// `Context` exactly like `named_states` — moved out at frame start, moved
    /// back at frame end where untouched slots are garbage-collected.
    pub scheduler: widgets::SchedulerState,
    /// Issue #234: persistent async task registry backing `Context::spawn` /
    /// `Context::poll`. Round-tripped through `Context` exactly like
    /// `scheduler` — moved out at frame start, moved back at frame end. Gated
    /// behind `async`; absent (zero overhead) when the feature is off.
    #[cfg(feature = "async")]
    pub async_tasks: context::AsyncTasks,
    pub screen_hook_map:
        std::collections::HashMap<u64, std::collections::HashMap<String, (usize, usize)>>,
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
    /// Issue #273: per-call-site version keys recorded by
    /// [`ContainerBuilder::cached`](crate::ContainerBuilder::cached) on the
    /// previous frame, indexed by the order `cached` regions were declared.
    /// Compared against this frame's keys to classify each cached region as a
    /// hit (key unchanged) or miss (key changed / new slot / first frame).
    /// Cleared on resize by [`clear_frame_layout_cache`] so every cached
    /// region misses after a geometry change. Round-trips through `Context`
    /// exactly like `commands_buf` (moved out at frame start, moved back at
    /// frame end). Empty (zero overhead) for apps that never call `cached`.
    pub region_versions: Vec<u64>,
    /// Issue #273: recycled scratch Vec for the CURRENT frame's `cached`
    /// region keys (same alloc-reuse discipline as `commands_buf`). Cleared
    /// before reuse; swapped into `region_versions` at frame end so the keys
    /// recorded this frame become next frame's comparison baseline.
    pub region_versions_buf: Vec<u64>,
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
#[cfg_attr(docsrs, doc(cfg(feature = "crossterm")))]
pub fn run(f: impl FnMut(&mut Context)) -> io::Result<()> {
    run_with(RunConfig::default(), f)
}

#[cfg(feature = "crossterm")]
fn validate_terminal_endpoints(
    stdin_is_terminal: bool,
    stdout_is_terminal: bool,
) -> io::Result<()> {
    if stdin_is_terminal && stdout_is_terminal {
        return Ok(());
    }

    let unavailable = match (stdin_is_terminal, stdout_is_terminal) {
        (false, false) => "stdin and stdout are not terminals",
        (false, true) => "stdin is not a terminal",
        (true, false) => "stdout is not a terminal",
        (true, true) => unreachable!("validated above"),
    };
    Err(io::Error::new(
        io::ErrorKind::NotConnected,
        format!("interactive SLT runtime unavailable: {unavailable}"),
    ))
}

#[cfg(feature = "crossterm")]
fn ensure_interactive_terminal() -> io::Result<()> {
    validate_terminal_endpoints(io::stdin().is_terminal(), io::stdout().is_terminal())
}

#[cfg(feature = "crossterm")]
fn validate_inline_height(height: u32) -> io::Result<()> {
    if height == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "inline terminal height must be greater than zero",
        ));
    }
    Ok(())
}

#[cfg(feature = "crossterm")]
fn set_terminal_title(title: &Option<String>) {
    if let Some(title) = title {
        use std::io::Write;
        let title = sanitize_terminal_text(title);
        let mut stdout = io::stdout();
        let _ = write!(stdout, "\x1b]2;{title}\x07");
        let _ = stdout.flush();
    }
}

#[cfg(feature = "crossterm")]
fn sanitize_terminal_text(input: &str) -> String {
    input
        .chars()
        .map(|ch| {
            if ch.is_control() || ('\u{80}'..='\u{9f}').contains(&ch) {
                '?'
            } else {
                ch
            }
        })
        .collect()
}

/// Run the TUI loop with custom configuration.
///
/// Like [`run`], but accepts a [`RunConfig`] to control tick rate, mouse
/// support, and theming.
///
/// Returns [`io::ErrorKind::NotConnected`] when stdin or stdout is not a
/// terminal. Headless and remote renderers should drive [`frame`] with a custom
/// [`Backend`] instead.
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
#[cfg_attr(docsrs, doc(cfg(feature = "crossterm")))]
pub fn run_with(config: RunConfig, f: impl FnMut(&mut Context)) -> io::Result<()> {
    ensure_interactive_terminal()?;
    with_session_panic_hook(|| run_with_inner(config, f))
}

#[cfg(feature = "crossterm")]
fn run_with_inner(config: RunConfig, mut f: impl FnMut(&mut Context)) -> io::Result<()> {
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
        if w > 0 && h > 0 {
            if !run_frame(
                &mut term,
                &mut state,
                &config,
                std::mem::take(&mut events),
                true,
                &mut f,
            )? {
                break;
            }
            // Issue #233: full-screen mode has no scrollback channel — warn and
            // drop any `ui.static_log(...)` lines so they do not leak into the
            // next frame's named_states.
            discard_static_log(&mut state, "full-screen run()");
            events = std::mem::take(&mut state.events_buf);
        }

        #[cfg(unix)]
        let suspend_snapshot = term.session_snapshot();
        #[cfg(unix)]
        let mut on_suspend = || suspend_current_session(suspend_snapshot);
        #[cfg(not(unix))]
        let mut on_suspend = || Ok(());

        if !poll_events(
            &mut events,
            &mut state,
            config.tick_rate,
            &mut || term.handle_resize(),
            config.handle_ctrl_c,
            config.handle_suspend,
            &mut on_suspend,
        )? {
            break;
        }

        sleep_for_fps_cap(config.max_fps, frame_start.elapsed());
    }

    Ok(())
}

/// Error returned when an asynchronous run loop is joined.
#[cfg(all(feature = "crossterm", feature = "async"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "crossterm", feature = "async"))))]
#[derive(Debug)]
pub enum AsyncRunError {
    /// The render loop returned an I/O error.
    Io(io::Error),
    /// Tokio reported task cancellation or a panic from the render loop.
    Join(tokio::task::JoinError),
}

#[cfg(all(feature = "crossterm", feature = "async"))]
impl std::fmt::Display for AsyncRunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "async render loop failed: {err}"),
            Self::Join(err) => write!(f, "async render task failed: {err}"),
        }
    }
}

#[cfg(all(feature = "crossterm", feature = "async"))]
impl std::error::Error for AsyncRunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Join(err) => Some(err),
        }
    }
}

#[cfg(all(feature = "crossterm", feature = "async"))]
#[derive(Default)]
struct AsyncWake {
    generation: std::sync::atomic::AtomicU64,
    notify: std::sync::Arc<tokio::sync::Notify>,
}

#[cfg(all(feature = "crossterm", feature = "async"))]
impl AsyncWake {
    fn notify(&self) {
        self.generation
            .fetch_add(1, std::sync::atomic::Ordering::Release);
        self.notify.notify_one();
    }

    fn generation(&self) -> u64 {
        self.generation.load(std::sync::atomic::Ordering::Acquire)
    }
}

/// Bounded async message sender that wakes its associated render loop.
#[cfg(all(feature = "crossterm", feature = "async"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "crossterm", feature = "async"))))]
pub struct AsyncSender<M> {
    inner: tokio::sync::mpsc::Sender<M>,
    wake: std::sync::Arc<AsyncWake>,
}

#[cfg(all(feature = "crossterm", feature = "async"))]
impl<M> Clone for AsyncSender<M> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            wake: std::sync::Arc::clone(&self.wake),
        }
    }
}

#[cfg(all(feature = "crossterm", feature = "async"))]
impl<M> Drop for AsyncSender<M> {
    fn drop(&mut self) {
        self.wake.notify();
    }
}

#[cfg(all(feature = "crossterm", feature = "async"))]
impl<M> AsyncSender<M> {
    /// Send one message, waiting for bounded-channel capacity.
    pub async fn send(&self, message: M) -> Result<(), tokio::sync::mpsc::error::SendError<M>> {
        self.inner.send(message).await?;
        self.wake.notify();
        Ok(())
    }

    /// Try to send one message without waiting for capacity.
    pub fn try_send(&self, message: M) -> Result<(), tokio::sync::mpsc::error::TrySendError<M>> {
        self.inner.try_send(message)?;
        self.wake.notify();
        Ok(())
    }

    /// Send one message from synchronous code, blocking for capacity.
    pub fn blocking_send(&self, message: M) -> Result<(), tokio::sync::mpsc::error::SendError<M>> {
        self.inner.blocking_send(message)?;
        self.wake.notify();
        Ok(())
    }

    /// Returns `true` when the render-loop receiver has closed.
    pub fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }

    /// Wait until the render-loop receiver closes.
    pub async fn closed(&self) {
        self.inner.closed().await;
    }

    /// Return the channel's remaining capacity.
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Return the channel's configured maximum capacity.
    pub fn max_capacity(&self) -> usize {
        self.inner.max_capacity()
    }
}

#[cfg(all(feature = "crossterm", feature = "async"))]
impl<M> std::ops::Deref for AsyncSender<M> {
    type Target = tokio::sync::mpsc::Sender<M>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// Owned lifetime handle for an asynchronous SLT render loop.
///
/// The handle dereferences to [`AsyncSender`], preserving the common
/// `handle.send(message).await` call shape. Dropping it requests cancellation;
/// call [`join`](Self::join) to observe normal completion, I/O failures, or a
/// render-task panic.
#[cfg(all(feature = "crossterm", feature = "async"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "crossterm", feature = "async"))))]
#[must_use = "dropping the handle cancels the render loop; join it to observe completion"]
pub struct AsyncRunHandle<M> {
    sender: Option<AsyncSender<M>>,
    join: Option<tokio::task::JoinHandle<io::Result<()>>>,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    wake: std::sync::Arc<AsyncWake>,
    cancel_on_drop: bool,
}

#[cfg(all(feature = "crossterm", feature = "async"))]
impl<M> AsyncRunHandle<M> {
    /// Clone the wake-aware sender while retaining ownership of the run loop.
    pub fn sender(&self) -> AsyncSender<M> {
        self.sender
            .as_ref()
            .expect("sender is available until join starts")
            .clone()
    }

    /// Request cooperative cancellation of the render loop.
    pub fn cancel(&self) {
        self.cancel
            .store(true, std::sync::atomic::Ordering::Release);
        self.wake.notify();
    }

    /// Returns `true` when the Tokio render task has completed.
    pub fn is_finished(&self) -> bool {
        self.join
            .as_ref()
            .is_none_or(tokio::task::JoinHandle::is_finished)
    }

    /// Wait for render-loop completion and preserve both I/O and join errors.
    ///
    /// Joining drops this handle's sender first. Sender clones must also be
    /// dropped, or the UI must quit, before a disconnect-driven loop can end.
    pub async fn join(mut self) -> Result<(), AsyncRunError> {
        self.sender.take();
        let join = self
            .join
            .take()
            .expect("join handle is consumed exactly once");
        join.await
            .map_err(AsyncRunError::Join)?
            .map_err(AsyncRunError::Io)
    }

    /// Request cancellation and wait for deterministic teardown.
    pub async fn cancel_and_join(mut self) -> Result<(), AsyncRunError> {
        self.cancel();
        self.sender.take();
        let join = self
            .join
            .take()
            .expect("join handle is consumed exactly once");
        join.await
            .map_err(AsyncRunError::Join)?
            .map_err(AsyncRunError::Io)
    }

    /// Detach the render task and return only a wake-aware sender.
    ///
    /// This compatibility path intentionally makes completion errors
    /// unobservable. Prefer retaining and joining the owned handle.
    pub fn detach(mut self) -> AsyncSender<M> {
        self.cancel_on_drop = false;
        self.join.take();
        self.sender
            .take()
            .expect("sender is available until detach")
    }
}

#[cfg(all(feature = "crossterm", feature = "async"))]
impl<M> std::ops::Deref for AsyncRunHandle<M> {
    type Target = AsyncSender<M>;

    fn deref(&self) -> &Self::Target {
        self.sender
            .as_ref()
            .expect("sender is available until join starts")
    }
}

#[cfg(all(feature = "crossterm", feature = "async"))]
impl<M> Drop for AsyncRunHandle<M> {
    fn drop(&mut self) {
        if self.cancel_on_drop {
            self.cancel
                .store(true, std::sync::atomic::Ordering::Release);
            self.wake.notify();
        }
    }
}

#[cfg(all(feature = "crossterm", feature = "async"))]
impl<M: Send + 'static> std::future::IntoFuture for AsyncRunHandle<M> {
    type Output = Result<(), AsyncRunError>;
    type IntoFuture =
        std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send + 'static>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.join())
    }
}

/// Run the TUI loop asynchronously with default configuration.
///
/// Requires the `async` feature. Spawns the render loop in a blocking thread
/// and returns an owned [`AsyncRunHandle`]. The handle sends messages directly,
/// can be cloned into an [`AsyncSender`], supports cooperative cancellation,
/// and exposes render-loop I/O or panic failures when joined.
///
/// Returns [`io::ErrorKind::NotConnected`] when stdin or stdout is not a
/// terminal. Use [`frame`] with a custom backend for headless rendering.
///
/// # Example
///
/// ```no_run
/// # #[cfg(feature = "async")]
/// # async fn example() -> std::io::Result<()> {
/// let run = slt::run_async::<String>(|ui, messages| {
///     for msg in messages.drain(..) {
///         ui.text(msg);
///     }
/// })?;
/// run.send("hello from async".to_string()).await.ok();
/// run.cancel_and_join().await.map_err(std::io::Error::other)?;
/// # Ok(())
/// # }
/// ```
#[cfg(all(feature = "crossterm", feature = "async"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "crossterm", feature = "async"))))]
pub fn run_async<M: Send + 'static>(
    f: impl FnMut(&mut Context, &mut Vec<M>) + Send + 'static,
) -> io::Result<AsyncRunHandle<M>> {
    run_async_with(RunConfig::default(), f)
}

/// Run the TUI loop asynchronously with custom configuration.
///
/// Requires the `async` feature. Like [`run_async`], but accepts a
/// [`RunConfig`] to control tick rate, mouse support, and theming.
///
/// Returns an owned [`AsyncRunHandle`] for sending, cancellation, and joining.
#[cfg(all(feature = "crossterm", feature = "async"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "crossterm", feature = "async"))))]
pub fn run_async_with<M: Send + 'static>(
    config: RunConfig,
    f: impl FnMut(&mut Context, &mut Vec<M>) + Send + 'static,
) -> io::Result<AsyncRunHandle<M>> {
    ensure_interactive_terminal()?;
    let (tx, rx) = tokio::sync::mpsc::channel(100);
    let handle =
        tokio::runtime::Handle::try_current().map_err(|err| io::Error::other(err.to_string()))?;
    let wake = std::sync::Arc::new(AsyncWake::default());
    let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    // Issue #234: clone the runtime handle into the render loop so
    // `Context::spawn` has a runtime to launch tasks onto. The render loop runs
    // on `spawn_blocking` (no ambient runtime), so the handle must be passed
    // explicitly rather than recovered via `Handle::try_current()` inside.
    let loop_handle = handle.clone();
    let loop_wake = std::sync::Arc::clone(&wake);
    let loop_cancel = std::sync::Arc::clone(&cancel);
    let join = handle
        .spawn_blocking(move || run_async_loop(config, f, rx, loop_handle, loop_wake, loop_cancel));

    Ok(AsyncRunHandle {
        sender: Some(AsyncSender {
            inner: tx,
            wake: std::sync::Arc::clone(&wake),
        }),
        join: Some(join),
        cancel,
        wake,
        cancel_on_drop: true,
    })
}

#[cfg(all(feature = "crossterm", feature = "async"))]
fn drain_async_messages<M>(
    rx: &mut tokio::sync::mpsc::Receiver<M>,
    messages: &mut Vec<M>,
    budget: usize,
) -> bool {
    let mut disconnected = false;
    for _ in 0..budget.max(1) {
        match rx.try_recv() {
            Ok(message) => messages.push(message),
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                disconnected = true;
                break;
            }
        }
    }
    disconnected || (rx.is_closed() && rx.is_empty())
}

#[cfg(all(feature = "crossterm", feature = "async"))]
fn run_async_loop<M: Send + 'static>(
    config: RunConfig,
    f: impl FnMut(&mut Context, &mut Vec<M>) + Send,
    rx: tokio::sync::mpsc::Receiver<M>,
    runtime: tokio::runtime::Handle,
    wake: std::sync::Arc<AsyncWake>,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> io::Result<()> {
    with_session_panic_hook(move || run_async_loop_inner(config, f, rx, runtime, wake, cancel))
}

#[cfg(all(feature = "crossterm", feature = "async"))]
fn run_async_loop_inner<M: Send + 'static>(
    config: RunConfig,
    mut f: impl FnMut(&mut Context, &mut Vec<M>) + Send,
    mut rx: tokio::sync::mpsc::Receiver<M>,
    runtime: tokio::runtime::Handle,
    wake: std::sync::Arc<AsyncWake>,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> io::Result<()> {
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
    // Issue #234: inject the ambient runtime so `Context::spawn` works inside
    // the frame closure. Set once before the loop; round-tripped through
    // `Context` from here on (see `run_frame_kernel`).
    state.async_tasks.set_runtime(runtime.clone());
    state
        .async_tasks
        .set_waker(std::sync::Arc::clone(&wake.notify));

    'app: loop {
        if cancel.load(std::sync::atomic::Ordering::Acquire) {
            break;
        }
        let frame_start = Instant::now();
        // Issue #263: after a SIGCONT resume, repaint the whole frame.
        #[cfg(unix)]
        drain_resume_redraw(&mut || term.handle_resize())?;
        let observed_wake = wake.generation();
        let (w, h) = term.size();
        if w > 0 && h > 0 {
            messages.clear();
            let input_disconnected =
                drain_async_messages(&mut rx, &mut messages, config.async_message_budget);
            if input_disconnected && messages.is_empty() {
                break;
            }
            let mut render = |ctx: &mut Context| {
                f(ctx, &mut messages);
            };
            if !run_frame(
                &mut term,
                &mut state,
                &config,
                std::mem::take(&mut events),
                true,
                &mut render,
            )? {
                break;
            }
            // Issue #233: full-screen async mode has no scrollback channel — warn
            // and drop any pending static_log lines.
            discard_static_log(&mut state, "run_async()");
            events = std::mem::take(&mut state.events_buf);
            if input_disconnected {
                break;
            }
        } else if rx.is_closed() && rx.is_empty() {
            break;
        }

        #[cfg(unix)]
        let suspend_snapshot = term.session_snapshot();
        #[cfg(unix)]
        let mut on_suspend = || suspend_current_session(suspend_snapshot);
        #[cfg(not(unix))]
        let mut on_suspend = || Ok(());

        let wait_start = Instant::now();
        let mut notified = false;
        loop {
            if cancel.load(std::sync::atomic::Ordering::Acquire) {
                break 'app;
            }
            notified |= wake.generation() != observed_wake
                || runtime.block_on(async {
                    tokio::time::timeout(Duration::ZERO, wake.notify.notified())
                        .await
                        .is_ok()
                });
            let pending = async_work_pending(
                term.size(),
                !events.is_empty(),
                notified || !rx.is_empty() || rx.is_closed(),
            );
            let remaining = async_wait_remaining(
                config.tick_rate,
                wait_start.elapsed(),
                config.max_fps,
                frame_start.elapsed(),
                pending,
            );
            let wait = remaining.min(Duration::from_millis(4));
            let wait = if term.size().0 == 0 || term.size().1 == 0 {
                wait.max(Duration::from_millis(4))
            } else {
                wait
            };
            if !poll_events(
                &mut events,
                &mut state,
                wait,
                &mut || term.handle_resize(),
                config.handle_ctrl_c,
                config.handle_suspend,
                &mut on_suspend,
            )? {
                break 'app;
            }
            let pending = async_work_pending(
                term.size(),
                !events.is_empty(),
                notified || !rx.is_empty() || rx.is_closed(),
            );
            if async_wait_remaining(
                config.tick_rate,
                wait_start.elapsed(),
                config.max_fps,
                frame_start.elapsed(),
                pending,
            )
            .is_zero()
            {
                break;
            }
        }
    }

    Ok(())
}

#[cfg(all(feature = "crossterm", feature = "async"))]
fn async_work_pending(size: (u32, u32), events: bool, messages: bool) -> bool {
    size.0 > 0 && size.1 > 0 && (events || messages)
}

#[cfg(all(feature = "crossterm", feature = "async"))]
fn async_wait_remaining(
    tick_rate: Duration,
    idle_elapsed: Duration,
    max_fps: Option<u32>,
    loop_elapsed: Duration,
    pending: bool,
) -> Duration {
    let cap = fps_sleep_duration(max_fps, loop_elapsed).unwrap_or(Duration::ZERO);
    if pending {
        cap
    } else {
        cap.max(tick_rate.saturating_sub(idle_elapsed))
    }
}

#[cfg(feature = "crossterm")]
fn localize_inline_events(term: &InlineTerminal, state: &mut FrameState, events: &mut Vec<Event>) {
    let origin = term.origin_row();
    if let Some(previous) = state.layout_feedback.origin_row
        && previous != origin
    {
        state.layout_feedback.last_mouse_pos =
            state.layout_feedback.last_mouse_pos.and_then(|(x, y)| {
                y.checked_add(previous).and_then(|y| {
                    term.localize_mouse(event::MouseEvent::new(
                        event::MouseKind::Moved,
                        x,
                        y,
                        KeyModifiers::NONE,
                        None,
                        None,
                    ))
                    .map(|mouse| (mouse.x, mouse.y))
                })
            });
        state.layout_feedback.last_click_at = None;
        state.layout_feedback.last_click_pos = None;
        state.selection.clear();
    }
    state.layout_feedback.origin_row = Some(origin);
    events.retain_mut(|event| match event {
        Event::Mouse(mouse) => match term.localize_mouse(mouse.clone()) {
            Some(local) => {
                *mouse = local;
                true
            }
            None => false,
        },
        _ => true,
    });
}

/// Run the TUI in inline mode with default configuration.
///
/// Renders `height` rows directly below the current cursor position without
/// entering alternate screen mode. Useful for CLI tools that want a small
/// interactive widget below the prompt.
///
/// `height` is the reserved inline render area in terminal rows.
/// The rest of the terminal stays in normal scrollback mode.
/// A zero height returns [`io::ErrorKind::InvalidInput`]; non-terminal stdin or
/// stdout returns [`io::ErrorKind::NotConnected`].
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
#[cfg_attr(docsrs, doc(cfg(feature = "crossterm")))]
pub fn run_inline(height: u32, f: impl FnMut(&mut Context)) -> io::Result<()> {
    run_inline_with(height, RunConfig::default(), f)
}

/// Run the TUI in inline mode with custom configuration.
///
/// Like [`run_inline`], but accepts a [`RunConfig`] to control tick rate,
/// mouse support, and theming.
#[cfg(feature = "crossterm")]
#[cfg_attr(docsrs, doc(cfg(feature = "crossterm")))]
pub fn run_inline_with(
    height: u32,
    config: RunConfig,
    f: impl FnMut(&mut Context),
) -> io::Result<()> {
    validate_inline_height(height)?;
    ensure_interactive_terminal()?;
    with_session_panic_hook(|| run_inline_with_inner(height, config, f))
}

#[cfg(feature = "crossterm")]
fn run_inline_with_inner(
    height: u32,
    config: RunConfig,
    mut f: impl FnMut(&mut Context),
) -> io::Result<()> {
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
        if w > 0 && h > 0 {
            localize_inline_events(&term, &mut state, &mut events);
            if !run_frame(
                &mut term,
                &mut state,
                &config,
                std::mem::take(&mut events),
                true,
                &mut f,
            )? {
                break;
            }
            // Issue #233: inline mode without `StaticOutput` has no scrollback
            // channel either — warn and drop any pending lines.
            discard_static_log(&mut state, "run_inline()");
            events = std::mem::take(&mut state.events_buf);
        }

        #[cfg(unix)]
        let suspend_snapshot = term.session_snapshot();
        #[cfg(unix)]
        let mut on_suspend = || suspend_current_session(suspend_snapshot);
        #[cfg(not(unix))]
        let mut on_suspend = || Ok(());

        if !poll_events(
            &mut events,
            &mut state,
            config.tick_rate,
            &mut || term.handle_resize(),
            config.handle_ctrl_c,
            config.handle_suspend,
            &mut on_suspend,
        )? {
            break;
        }

        sleep_for_fps_cap(config.max_fps, frame_start.elapsed());
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
/// A zero dynamic height returns [`io::ErrorKind::InvalidInput`]; non-terminal
/// stdin or stdout returns [`io::ErrorKind::NotConnected`].
#[cfg(feature = "crossterm")]
#[cfg_attr(docsrs, doc(cfg(feature = "crossterm")))]
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
#[cfg_attr(docsrs, doc(cfg(feature = "crossterm")))]
pub fn run_static_with(
    output: &mut StaticOutput,
    dynamic_height: u32,
    config: RunConfig,
    f: impl FnMut(&mut Context),
) -> io::Result<()> {
    validate_inline_height(dynamic_height)?;
    ensure_interactive_terminal()?;
    with_session_panic_hook(|| run_static_with_inner(output, dynamic_height, config, f))
}

#[cfg(feature = "crossterm")]
fn run_static_with_inner(
    output: &mut StaticOutput,
    dynamic_height: u32,
    config: RunConfig,
    mut f: impl FnMut(&mut Context),
) -> io::Result<()> {
    let color_depth = config.color_depth.unwrap_or_else(ColorDepth::detect);
    let mut term = InlineTerminal::new(
        dynamic_height,
        config.mouse,
        config.kitty_keyboard,
        config.report_all_keys,
        color_depth,
    )?;
    term.write_scrollback(&output.drain_new())?;
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
        term.write_scrollback(&output.drain_new())?;
        if w > 0 && h > 0 {
            localize_inline_events(&term, &mut state, &mut events);
            let keep_running = run_frame(
                &mut term,
                &mut state,
                &config,
                std::mem::take(&mut events),
                true,
                &mut f,
            )?;
            // Issue #233: drain any `ui.static_log(...)` lines queued during the
            // frame closure into `output`, then flush them before any exit path.
            for line in drain_static_log(&mut state) {
                output.println(line);
            }
            term.write_scrollback(&output.drain_new())?;
            if !keep_running {
                break;
            }
            events = std::mem::take(&mut state.events_buf);
        }

        #[cfg(unix)]
        let suspend_snapshot = term.session_snapshot();
        #[cfg(unix)]
        let mut on_suspend = || suspend_current_session(suspend_snapshot);
        #[cfg(not(unix))]
        let mut on_suspend = || Ok(());

        if !poll_events(
            &mut events,
            &mut state,
            config.tick_rate,
            &mut || term.handle_resize(),
            config.handle_ctrl_c,
            config.handle_suspend,
            &mut on_suspend,
        )? {
            break;
        }

        sleep_for_fps_cap(config.max_fps, frame_start.elapsed());
    }

    Ok(())
}

#[cfg(all(feature = "crossterm", test))]
fn write_static_lines_to(stdout: &mut impl io::Write, lines: &[String]) -> io::Result<()> {
    for line in lines {
        let safe = sanitize_terminal_text(line);
        stdout.write_all(safe.as_bytes())?;
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
    if let Some(boxed) = state.named_states.get_mut(KEYMAP_REGISTRY_NAMED_STATE_KEY)
        && let Some(vec) = boxed.downcast_mut::<Vec<crate::keymap::PublishedKeymap>>()
    {
        vec.clear();
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
    if let Some(boxed) = state.named_states.get_mut(STATIC_LOG_NAMED_STATE_KEY)
        && let Some(buf) = boxed.downcast_mut::<Vec<String>>()
    {
        return std::mem::take(buf);
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
pub(crate) fn process_run_loop_event(ev: &Event, state: &mut FrameState, has_resize: &mut bool) {
    match ev {
        Event::Mouse(m) => {
            state.layout_feedback.last_mouse_pos = Some((m.x, m.y));
        }
        Event::FocusLost => {
            state.layout_feedback.last_mouse_pos = None;
            state.layout_feedback.last_click_at = None;
            state.layout_feedback.last_click_pos = None;
        }
        // Issue #268: Ctrl+F12 toggles the devtools inspector panel
        // independently of the F12 outline overlay and the Shift+F12 layer
        // cycle. Match before the Shift/NONE arms so the Control branch wins.
        Event::Key(event::KeyEvent {
            code: KeyCode::F(12),
            kind: event::KeyEventKind::Press,
            modifiers,
        }) if modifiers.contains(event::KeyModifiers::CONTROL) => {
            state.diagnostics.inspector_mode = !state.diagnostics.inspector_mode;
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

/// Number of `on_resize` invocations a batch of events should trigger.
///
/// v0.21.1 resize coalescing: a single poll batch may deliver a burst of
/// `Event::Resize` events while a user drags the window edge. Each
/// [`Terminal::handle_resize`](crate::terminal::Terminal::handle_resize) does a
/// `terminal::size()` syscall, two buffer reallocations, and a `Clear(All)`, so
/// firing it per-event is pure waste — only the *final* geometry matters and
/// `handle_resize` always reads the live terminal size, not the per-event
/// payload. This helper returns `1` if the batch contains any resize and `0`
/// otherwise, so the caller can collapse the burst into one end-of-batch call.
///
/// Kept as a pure function (no I/O) so the coalescing rule is unit-testable
/// without a real crossterm event source.
#[cfg(feature = "crossterm")]
#[inline]
fn resize_invocations_for_batch(events: &[Event]) -> usize {
    usize::from(events.iter().any(|e| matches!(e, Event::Resize(_, _))))
}

/// Poll for terminal events, handling resize, Ctrl-C, F12 debug toggle,
/// and layout cache invalidation. Returns `Ok(false)` when the loop should exit.
///
/// `handle_ctrl_c` controls whether Ctrl+C exits the loop (`true`, default
/// v0.19 behavior) or is delivered to the frame closure as a regular key
/// event (`false`, RataTUI parity, issue #238).
///
/// v0.21.1: resize events within one poll batch are *coalesced* — `on_resize`
/// is invoked at most once, after the whole batch is drained, using the final
/// terminal size (`handle_resize` re-reads `terminal::size()`). Dragging a
/// window edge can emit dozens of `Event::Resize` per poll; firing the
/// `Clear(All)` + double realloc + `size()` syscall for each is wasted work
/// when only the last geometry survives. The SIGCONT/resume redraw path in
/// [`run_with`] is unaffected — it calls `handle_resize` directly, outside this
/// function.
#[cfg(feature = "crossterm")]
fn poll_events(
    events: &mut Vec<Event>,
    state: &mut FrameState,
    tick_rate: Duration,
    on_resize: &mut impl FnMut() -> io::Result<()>,
    handle_ctrl_c: bool,
    handle_suspend: bool,
    on_suspend: &mut impl FnMut() -> io::Result<()>,
) -> io::Result<bool> {
    let mut has_resize = false;
    let batch_start = events.len();

    fn process_ev(ev: &Event, has_resize: &mut bool) {
        *has_resize |= matches!(ev, Event::Resize(_, _));
    }

    if event::poll(tick_rate)? {
        let raw = event::read()?;
        let mut raw_events = 1;
        if let Some(ev) = event::from_crossterm(raw) {
            if handle_ctrl_c && is_ctrl_c(&ev) {
                return Ok(false);
            }
            if handle_suspend && is_ctrl_z(&ev) {
                on_suspend()?;
                return Ok(true);
            }
            // Resize is recorded (via `has_resize`) but not yet acted on — the
            // single `on_resize` call is deferred to end-of-batch so a burst
            // collapses into one geometry sync.
            process_ev(&ev, &mut has_resize);
            events.push(ev);
        }

        // Unmapped native events (for example media keys) also consume the
        // budget, otherwise a filtered-event flood can starve UI frames.
        while raw_events < 256 && events.len() < 256 && event::poll(Duration::ZERO)? {
            let raw = event::read()?;
            raw_events += 1;
            if let Some(ev) = event::from_crossterm(raw) {
                if handle_ctrl_c && is_ctrl_c(&ev) {
                    return Ok(false);
                }
                if handle_suspend && is_ctrl_z(&ev) {
                    on_suspend()?;
                    return Ok(true);
                }
                process_ev(&ev, &mut has_resize);
                events.push(ev);
            }
        }
    }

    // Coalesced resize: fire `on_resize` exactly once for the whole batch,
    // after every event has been read, so it picks up the final terminal size.
    // `has_resize` is the per-batch "saw a resize" flag set by `process_ev`.
    debug_assert_eq!(
        usize::from(has_resize),
        resize_invocations_for_batch(&events[batch_start..]),
        "has_resize must agree with the coalescing helper"
    );
    if has_resize {
        on_resize()?;
    }

    if has_resize {
        clear_frame_layout_cache(state);
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
    let now = state.diagnostics.clock_override.unwrap_or(frame_start);
    if let Some(previous) = state.diagnostics.frame_started {
        let interval = now.saturating_duration_since(previous);
        state.diagnostics.frame_interval = interval;
        if !interval.is_zero() {
            let fps = 1.0 / interval.as_secs_f32();
            state.diagnostics.fps_ema = if state.diagnostics.fps_ema == 0.0 {
                fps
            } else {
                state.diagnostics.fps_ema * 0.9 + fps * 0.1
            };
        }
    }
    state.diagnostics.frame_started = Some(now);
    let (w, h) = size;
    if state.layout_feedback.size != Some(size)
        || events
            .iter()
            .any(|event| matches!(event, Event::Resize(_, _)))
    {
        clear_frame_layout_cache(state);
    }
    state.layout_feedback.size = Some(size);
    let mut resized = false;
    for event in &events {
        process_run_loop_event(event, state, &mut resized);
    }
    // Issue #236: reset the per-frame keymap registry before constructing
    // `Context`. Widgets that call `publish_keymap` accumulate fresh
    // entries; entries from the previous frame must not leak through
    // `named_states` persistence.
    clear_keymap_registry(state);
    // Issue #273: invalidate every `cached` region's persisted version key on a
    // resize. The real run loop also clears region keys via
    // `clear_frame_layout_cache` (driven by its `has_resize` flag), but the
    // headless `TestBackend` / `frame_owned` paths feed the kernel directly
    // and never run that flag, so we detect the resize event here too. This
    // keeps the "resize forces a cache miss for all cached regions" invariant
    // path-independent: a geometry change cannot be silently treated as a hit.
    // Cheap when unused — `region_versions` is empty for apps without `cached`.
    if !state.region_versions.is_empty() && events.iter().any(|e| matches!(e, Event::Resize(_, _)))
    {
        state.region_versions.clear();
    }
    let mut ctx = Context::new(events, w, h, state, config.theme);
    ctx.is_real_terminal = is_real_terminal;
    // Issue #264: surface the negotiated capability snapshot read-only. The
    // probe ran once at session enter (cached in a `OnceLock`); on a headless
    // backend it never ran, so we keep the conservative default rather than
    // forcing a probe that would block on stdin.
    #[cfg(feature = "crossterm")]
    if is_real_terminal {
        ctx.capabilities = terminal::capabilities();
    }
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
        reclaim_feedback(&mut ctx, state);
        reclaim_event_scratch(&mut ctx, state);
        state.hook_states = ctx.hook_states;
        state.named_states = ctx.named_states;
        state.keyed_states = ctx.keyed_states;
        // Issue #262: persist the partial-chord buffer on quit too (TestBackend
        // reuses `FrameState` across `render()` calls — same rationale as the
        // keyed-state reclaim).
        state.chord_states = ctx.chord;
        // Issue #248: hand the scheduler table back and GC abandoned timers.
        let mut scheduler = ctx.scheduler;
        scheduler.gc_untouched();
        state.scheduler = scheduler;
        // Issue #234: hand the async task registry back so in-flight tasks and
        // pending results survive to the next frame (TestBackend reuses
        // `FrameState` across `render()` calls — same rationale as the
        // scheduler reclaim).
        #[cfg(feature = "async")]
        {
            // Pump the registry every frame so a handle dropped on a frame that
            // calls neither spawn nor poll still has its cancellation processed
            // (and completed results moved in) before the round-trip.
            ctx.async_tasks.maintain();
            state.async_tasks = ctx.async_tasks;
        }
        state.screen_hook_map = ctx.screen_hook_map;
        state.diagnostics.notification_queue = ctx.rollback.notification_queue;
        state.diagnostics.debug_layer = ctx.debug_layer;
        // Issue #268: persist any in-frame `set_inspector` change on quit too.
        state.diagnostics.inspector_mode = ctx.inspector_mode;
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
        // Issue #273: reclaim the region-cache key buffers on quit too
        // (TestBackend reuses `FrameState` across `render()` calls — same
        // rationale as #204). The quit path skips `build_tree`, but the keys
        // recorded by any `cached` regions before `quit()` are still valid as
        // next frame's baseline.
        state.region_versions = std::mem::take(&mut ctx.region_versions_cur);
        state.region_versions_buf = std::mem::take(&mut ctx.region_versions_prev);
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

    // Recover the previous feedback vectors as this frame's collection
    // scratch, then publish the newly collected vectors with a second swap.
    // This keeps both sides' capacities warm instead of `mem::take`-ing every
    // filled vector and leaving an empty FrameData behind.
    reclaim_feedback(&mut ctx, state);
    let mut fd = std::mem::take(&mut state.frame_data);
    fd.swap_feedback(&mut state.layout_feedback);
    layout::collect_all(&tree, &mut fd);
    debug_assert_eq!(
        fd.scroll_infos.len(),
        fd.scroll_rects.len(),
        "scroll feedback vectors must stay aligned"
    );
    fd.swap_feedback(&mut state.layout_feedback);
    let mut raw_rects = std::mem::take(&mut fd.raw_draw_rects);
    layout::render(&tree, buffer);
    let mut deferred_draw_panic = None;
    for rdr in raw_rects.drain(..) {
        if rdr.rect.width == 0 || rdr.rect.height == 0 {
            continue;
        }
        let Some(cb) = ctx
            .deferred_draws
            .get_mut(rdr.draw_id)
            .and_then(|c| c.take())
        else {
            continue;
        };
        if let Err(panic) = context::invoke_deferred_draw(
            buffer,
            rdr.rect,
            rdr.left_clip_cols,
            rdr.top_clip_rows,
            rdr.original_width,
            rdr.original_height,
            cb,
        ) {
            deferred_draw_panic = Some(panic);
            break;
        }
    }
    raw_rects.clear();
    fd.raw_draw_rects = raw_rects;
    state.frame_data = fd;
    debug_assert!(
        buffer.kitty_clip_info_stack.is_empty(),
        "kitty_clip_info_stack must be empty at end of frame"
    );
    debug_assert!(
        buffer.kitty_horizontal_clip_stack.is_empty(),
        "kitty_horizontal_clip_stack must be empty at end of frame"
    );
    reclaim_event_scratch(&mut ctx, state);
    state.hook_states = ctx.hook_states;
    state.named_states = ctx.named_states;
    // Issue #215: hand the keyed-state map back to FrameState so the next
    // frame can pick it up via `Context::new`. Mirrors the `named_states`
    // round-trip exactly.
    state.keyed_states = ctx.keyed_states;
    // Issue #262: hand the partial-chord buffer back so a chord spanning
    // multiple frames survives between them. Same round-trip as `keyed_states`.
    state.chord_states = ctx.chord;
    // Issue #248: hand the scheduler table back and GC any timer slot that was
    // not sampled this frame (mirrors the `named_states` round-trip lifecycle).
    let mut scheduler = ctx.scheduler;
    scheduler.gc_untouched();
    state.scheduler = scheduler;
    // Issue #234: hand the async task registry back so in-flight tasks and
    // pending results survive to the next frame (same round-trip lifecycle as
    // the scheduler table).
    #[cfg(feature = "async")]
    {
        // Pump the registry every frame (see the quit-path note): drains
        // completed results and honours handle-drop cancellations even on a
        // frame that called neither spawn nor poll.
        ctx.async_tasks.maintain();
        state.async_tasks = ctx.async_tasks;
    }
    state.screen_hook_map = ctx.screen_hook_map;
    state.diagnostics.notification_queue = ctx.rollback.notification_queue;
    // Issue #201: persist any in-frame `set_debug_layer` change.
    state.diagnostics.debug_layer = ctx.debug_layer;
    // Issue #268: persist any in-frame `set_inspector` change.
    state.diagnostics.inspector_mode = ctx.inspector_mode;
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
    // Issue #273: this frame's recorded `cached` keys become next frame's
    // comparison baseline; the (now-stale) previous keys are reclaimed as the
    // recycled scratch buffer. Same alloc-reuse discipline as `commands_buf`.
    state.region_versions = std::mem::take(&mut ctx.region_versions_cur);
    state.region_versions_buf = std::mem::take(&mut ctx.region_versions_prev);
    // Issue #150: reclaim the drained command Vec so the next `Context::new`
    // picks it up via `mem::take(&mut state.commands_buf)`. After
    // `build_tree(&mut ctx.commands)` the Vec is at `len == 0` with capacity
    // preserved; mirror the #204 reclamation pattern for the other six
    // per-frame buffers.
    state.commands_buf = std::mem::take(&mut ctx.commands);

    let frame_time = frame_start.elapsed();
    state.diagnostics.render_duration = frame_time;
    let frame_time_us = frame_time.as_micros().min(u128::from(u64::MAX)) as u64;
    if state.diagnostics.debug_mode {
        layout::render_debug_overlay(
            &tree,
            buffer,
            frame_time_us,
            state.diagnostics.fps_ema,
            state.diagnostics.debug_layer,
        );
    }
    // Issue #268: render the devtools inspector panel (Ctrl+F12) on top of the
    // frame. Reuses the already-built tree and the focus snapshot threaded in
    // from `FrameState` (no new traversal beyond one focused-node DFS). The
    // name map was already swapped into `focus_name_map_prev` above, so it
    // reflects this frame's registrations.
    if state.diagnostics.inspector_mode {
        let focus = layout::InspectorFocus {
            focus_index: state.focus.focus_index,
            focus_count: state.focus.prev_focus_count,
            names: &state.focus.focus_name_map_prev,
            theme: &config.theme,
        };
        layout::render_inspector(&tree, buffer, &focus);
    }

    // The callback executed after layout, so no Context fallback can safely
    // run here. Restore every persistent frame field first, then preserve the
    // original panic for the owning runtime or outer catch_unwind boundary.
    if let Some(panic) = deferred_draw_panic {
        std::panic::resume_unwind(panic);
    }

    FrameKernelResult {
        should_quit: false,
        #[cfg(feature = "crossterm")]
        clipboard_text,
        #[cfg(feature = "crossterm")]
        should_copy_selection,
    }
}

fn reclaim_feedback(ctx: &mut Context, state: &mut FrameState) {
    state.layout_feedback.prev_scroll_infos = std::mem::take(&mut ctx.prev_scroll_infos);
    state.layout_feedback.prev_scroll_rects = std::mem::take(&mut ctx.prev_scroll_rects);
    state.layout_feedback.prev_hit_map = std::mem::take(&mut ctx.prev_hit_map);
    state.layout_feedback.prev_allocated_areas = std::mem::take(&mut ctx.prev_allocated_areas);
    state.layout_feedback.prev_group_rects = std::mem::take(&mut ctx.prev_group_rects);
    state.layout_feedback.prev_focus_groups = std::mem::take(&mut ctx.prev_focus_groups);
}

fn reclaim_event_scratch(ctx: &mut Context, state: &mut FrameState) {
    ctx.events.clear();
    ctx.consumed.clear();
    state.events_buf = std::mem::take(&mut ctx.events);
    state.consumed_buf = std::mem::take(&mut ctx.consumed);
    state.focus.focus_name_map_buf = std::mem::take(&mut ctx.focus_name_map_prev);
    ctx.geometry_stack.clear();
    state.geometry_stack_buf = std::mem::take(&mut ctx.geometry_stack);
}

fn run_frame(
    term: &mut impl Backend,
    state: &mut FrameState,
    config: &RunConfig,
    events: Vec<event::Event>,
    terminal_side_effects: bool,
    f: &mut impl FnMut(&mut context::Context),
) -> io::Result<bool> {
    let size = term.size();
    let kernel = run_frame_kernel(
        term.buffer_mut(),
        state,
        config,
        size,
        events,
        terminal_side_effects,
        f,
    );
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
    if terminal_side_effects && kernel.should_copy_selection {
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

    let flush_start = Instant::now();
    term.flush()?;
    state.diagnostics.flush_duration = flush_start.elapsed();
    #[cfg(feature = "crossterm")]
    if terminal_side_effects && let Some(text) = kernel.clipboard_text {
        #[allow(clippy::print_stderr)]
        if let Err(e) = terminal::copy_to_clipboard(&mut io::stdout(), &text) {
            eprintln!("[slt] failed to copy to clipboard: {e}");
        }
    }
    state.diagnostics.tick = state.diagnostics.tick.wrapping_add(1);

    Ok(true)
}

fn clear_frame_layout_cache(state: &mut FrameState) {
    state.layout_feedback.prev_hit_map.clear();
    state.layout_feedback.prev_allocated_areas.clear();
    state.layout_feedback.prev_group_rects.clear();
    state.layout_feedback.prev_content_map.clear();
    state.layout_feedback.prev_focus_rects.clear();
    state.layout_feedback.prev_focus_groups.clear();
    state.layout_feedback.prev_scroll_infos.clear();
    state.layout_feedback.prev_scroll_rects.clear();
    state.layout_feedback.last_mouse_pos = None;
    state.layout_feedback.last_click_at = None;
    state.layout_feedback.last_click_pos = None;
    #[cfg(feature = "crossterm")]
    state.selection.clear();
    // Issue #273: a resize may change the geometry of every cached region, so
    // the previous frame's version keys are no longer a safe stability signal.
    // Dropping them forces a cache miss for all `cached` regions on the next
    // frame, matching the layout-feedback invalidation above.
    state.region_versions.clear();
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
fn is_ctrl_z(ev: &Event) -> bool {
    matches!(
        ev,
        Event::Key(event::KeyEvent {
            code: KeyCode::Char('z'),
            modifiers,
            kind: event::KeyEventKind::Press,
        }) if modifiers.contains(KeyModifiers::CONTROL)
    )
}

#[cfg(feature = "crossterm")]
fn sleep_for_fps_cap(max_fps: Option<u32>, loop_elapsed: Duration) {
    if let Some(remaining) = fps_sleep_duration(max_fps, loop_elapsed) {
        std::thread::sleep(remaining);
    }
}

#[cfg(feature = "crossterm")]
fn fps_sleep_duration(max_fps: Option<u32>, loop_elapsed: Duration) -> Option<Duration> {
    let fps = max_fps.filter(|fps| *fps > 0)?;
    let target = Duration::from_secs_f64(1.0 / fps as f64);
    (loop_elapsed < target).then(|| target - loop_elapsed)
}

#[cfg(all(test, feature = "crossterm"))]
mod run_loop_tests {
    //! Issue #201 regression tests for the run-loop F12 / Shift+F12
    //! keybinding handler. Exercises [`process_run_loop_event`] directly
    //! so we don't need a real crossterm event source.
    use super::*;

    static PANIC_HOOK_TEST_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[cfg(feature = "async")]
    #[test]
    fn zero_geometry_retained_events_do_not_schedule_immediate_frames() {
        assert!(!async_work_pending((0, 24), true, true));
        assert!(!async_work_pending((80, 0), true, true));
        assert!(async_work_pending((80, 24), true, false));
        assert!(async_work_pending((80, 24), false, true));
        assert!(!async_work_pending((80, 24), false, false));
        assert_eq!(
            async_wait_remaining(
                Duration::from_secs(1),
                Duration::ZERO,
                None,
                Duration::ZERO,
                false
            ),
            Duration::from_secs(1),
        );
        assert_eq!(
            async_wait_remaining(
                Duration::from_secs(1),
                Duration::ZERO,
                None,
                Duration::ZERO,
                true
            ),
            Duration::ZERO,
        );
    }

    fn key(modifiers: event::KeyModifiers) -> Event {
        Event::Key(event::KeyEvent {
            code: KeyCode::F(12),
            kind: event::KeyEventKind::Press,
            modifiers,
        })
    }

    fn char_key(ch: char, modifiers: event::KeyModifiers) -> Event {
        Event::Key(event::KeyEvent {
            code: KeyCode::Char(ch),
            kind: event::KeyEventKind::Press,
            modifiers,
        })
    }

    #[test]
    fn terminal_text_sanitizer_replaces_control_bytes() {
        assert_eq!(
            sanitize_terminal_text("safe\x1b]52;c;AAAA\x07text\u{9b}tail"),
            "safe?]52;c;AAAA?text?tail"
        );
    }

    #[test]
    fn fps_pacing_accounts_for_polling_and_rendering_together() {
        let remaining = fps_sleep_duration(Some(60), Duration::from_millis(16))
            .expect("a sub-frame remainder remains");
        assert!(remaining < Duration::from_millis(1));
        assert_eq!(
            fps_sleep_duration(Some(60), Duration::from_millis(20)),
            None,
            "a slow whole-loop iteration must not sleep again"
        );
        assert_eq!(fps_sleep_duration(None, Duration::ZERO), None);
    }

    #[test]
    fn terminal_endpoint_validation_reports_each_non_tty_case() {
        assert!(validate_terminal_endpoints(true, true).is_ok());
        for endpoints in [(false, true), (true, false), (false, false)] {
            let err = validate_terminal_endpoints(endpoints.0, endpoints.1).unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::NotConnected);
        }
    }

    #[test]
    fn panic_hook_tracks_nested_session_ownership() {
        let _serial = PANIC_HOOK_TEST_GUARD
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let outer = install_panic_hook();
        let inner = install_panic_hook();
        {
            let state = PANIC_HOOK_STATE
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            assert_eq!(state.active_sessions, 2);
            assert!(state.installed);
        }
        drop(inner);
        assert_eq!(
            PANIC_HOOK_STATE
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .active_sessions,
            1
        );
        drop(outer);
        let state = PANIC_HOOK_STATE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert_eq!(state.active_sessions, 0);
        assert!(!state.installed);
        assert!(state.previous.is_none());
        drop(state);

        let caught = std::panic::catch_unwind(|| {
            with_session_panic_hook(|| panic!("session panic"));
        });
        assert!(caught.is_err());
        let state = PANIC_HOOK_STATE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert_eq!(state.active_sessions, 0);
        assert!(!state.installed);
        assert!(state.previous.is_none());
    }

    #[test]
    fn static_lines_are_sanitized_before_scrollback_write() {
        let lines = vec!["ok\x1b[31mred\x07".to_string()];
        let mut out = Vec::new();
        write_static_lines_to(&mut out, &lines).unwrap();
        assert_eq!(out, b"ok?[31mred?\r\n");
    }

    #[test]
    fn quit_frame_keeps_final_static_log_available_for_drain() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 2));
        let mut state = FrameState::default();
        let result = run_frame_kernel(
            &mut buffer,
            &mut state,
            &RunConfig::default(),
            (20, 2),
            Vec::new(),
            false,
            &mut |ui| {
                ui.static_log("final line");
                ui.quit();
            },
        );

        assert!(result.should_quit);
        assert_eq!(drain_static_log(&mut state), vec!["final line"]);
    }

    #[test]
    fn ctrl_z_suspend_key_is_detected_separately_from_ctrl_c() {
        assert!(is_ctrl_z(&char_key('z', event::KeyModifiers::CONTROL)));
        assert!(!is_ctrl_z(&char_key('z', event::KeyModifiers::NONE)));
        assert!(is_ctrl_c(&char_key('c', event::KeyModifiers::CONTROL)));
        assert!(!is_ctrl_c(&char_key('z', event::KeyModifiers::CONTROL)));
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

    #[cfg(feature = "async")]
    #[test]
    fn async_message_drain_reports_disconnect_after_sender_drop() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(4);
        tx.try_send(1u8).expect("channel has capacity");
        tx.try_send(2u8).expect("channel has capacity");
        drop(tx);

        let mut messages = Vec::new();
        let disconnected = drain_async_messages(&mut rx, &mut messages, 4);

        assert!(disconnected);
        assert_eq!(messages, vec![1, 2]);
    }

    #[cfg(feature = "async")]
    #[test]
    fn async_message_drain_respects_per_frame_budget_and_fifo_order() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(8);
        for message in 0u8..6 {
            tx.try_send(message).expect("channel has capacity");
        }

        let mut first = Vec::new();
        assert!(!drain_async_messages(&mut rx, &mut first, 2));
        assert_eq!(first, vec![0, 1]);

        let mut second = Vec::new();
        assert!(!drain_async_messages(&mut rx, &mut second, 3));
        assert_eq!(second, vec![2, 3, 4]);

        drop(tx);
        let mut final_batch = Vec::new();
        assert!(drain_async_messages(&mut rx, &mut final_batch, 3));
        assert_eq!(final_batch, vec![5]);
    }

    #[test]
    fn async_message_budget_normalizes_zero_to_one() {
        assert_eq!(
            RunConfig::default()
                .async_message_budget(0)
                .async_message_budget,
            1
        );
    }

    #[cfg(feature = "async")]
    fn test_async_handle(
        task: impl FnOnce(std::sync::Arc<std::sync::atomic::AtomicBool>) -> io::Result<()>
        + Send
        + 'static,
    ) -> AsyncRunHandle<()> {
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        let wake = std::sync::Arc::new(AsyncWake::default());
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let task_cancel = std::sync::Arc::clone(&cancel);
        let join = tokio::task::spawn_blocking(move || task(task_cancel));
        AsyncRunHandle {
            sender: Some(AsyncSender {
                inner: tx,
                wake: std::sync::Arc::clone(&wake),
            }),
            join: Some(join),
            cancel,
            wake,
            cancel_on_drop: true,
        }
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn async_run_handle_observes_normal_and_io_completion() {
        test_async_handle(|_| Ok(())).await.unwrap();

        let error = test_async_handle(|_| {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "deterministic failure",
            ))
        })
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            AsyncRunError::Io(ref err) if err.kind() == io::ErrorKind::BrokenPipe
        ));
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn async_run_handle_observes_panics_and_cancels_cooperatively() {
        let panic_error = test_async_handle(|_| panic!("render panic"))
            .await
            .unwrap_err();
        assert!(matches!(
            panic_error,
            AsyncRunError::Join(ref err) if err.is_panic()
        ));

        let handle = test_async_handle(|cancel| {
            while !cancel.load(std::sync::atomic::Ordering::Acquire) {
                std::thread::sleep(Duration::from_millis(1));
            }
            Ok(())
        });
        handle.cancel_and_join().await.unwrap();
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn async_sender_notifies_on_send_and_last_clone_drop() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(2);
        let wake = std::sync::Arc::new(AsyncWake::default());
        let sender = AsyncSender {
            inner: tx,
            wake: std::sync::Arc::clone(&wake),
        };

        let before_send = wake.generation();
        sender.send(7u8).await.unwrap();
        assert_ne!(wake.generation(), before_send);
        assert_eq!(rx.recv().await, Some(7));

        let clone = sender.clone();
        let before_drop = wake.generation();
        drop(clone);
        assert_ne!(wake.generation(), before_drop);
    }

    // ── Issue #268: Ctrl+F12 devtools inspector toggle ───────────────────

    #[test]
    fn ctrl_f12_toggles_inspector_independently() {
        let mut state = FrameState::default();
        let mut has_resize = false;
        assert!(!state.diagnostics.inspector_mode);

        // Ctrl+F12 flips the inspector without touching debug overlay state.
        process_run_loop_event(
            &key(event::KeyModifiers::CONTROL),
            &mut state,
            &mut has_resize,
        );
        assert!(state.diagnostics.inspector_mode);
        assert!(
            !state.diagnostics.debug_mode,
            "Ctrl+F12 must not toggle the F12 outline overlay"
        );
        assert_eq!(
            state.diagnostics.debug_layer,
            DebugLayer::All,
            "Ctrl+F12 must not cycle the debug layer"
        );

        // A second Ctrl+F12 toggles it back off.
        process_run_loop_event(
            &key(event::KeyModifiers::CONTROL),
            &mut state,
            &mut has_resize,
        );
        assert!(!state.diagnostics.inspector_mode);
    }

    #[test]
    fn plain_and_shift_f12_do_not_touch_inspector() {
        let mut state = FrameState::default();
        let mut has_resize = false;
        // Plain F12 (overlay toggle) leaves the inspector alone.
        process_run_loop_event(&key(event::KeyModifiers::NONE), &mut state, &mut has_resize);
        assert!(state.diagnostics.debug_mode);
        assert!(!state.diagnostics.inspector_mode);
        // Shift+F12 (layer cycle) also leaves the inspector alone.
        process_run_loop_event(
            &key(event::KeyModifiers::SHIFT),
            &mut state,
            &mut has_resize,
        );
        assert!(!state.diagnostics.inspector_mode);
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

    // ── v0.21.1: resize debounce / coalesce ─────────────────────────────

    fn resize(w: u32, h: u32) -> Event {
        Event::Resize(w, h)
    }

    #[test]
    fn resize_batch_coalesces_to_single_invocation() {
        // Three resize events in one poll batch must collapse to exactly one
        // `on_resize` call (the helper that drives the single end-of-batch
        // call in `poll_events`). The final size is irrelevant to the count —
        // `handle_resize` re-reads `terminal::size()` — but we feed distinct
        // sizes to mirror a real drag burst.
        let batch = vec![resize(80, 24), resize(100, 30), resize(120, 40)];
        assert_eq!(
            resize_invocations_for_batch(&batch),
            1,
            "a burst of resizes must coalesce to one on_resize"
        );
    }

    #[test]
    fn resize_batch_without_resize_invokes_zero_times() {
        // A batch with no resize event must not trigger `on_resize` at all.
        let batch = vec![key(event::KeyModifiers::NONE)];
        assert_eq!(resize_invocations_for_batch(&batch), 0);
        // Empty batch is likewise a no-op.
        assert_eq!(resize_invocations_for_batch(&[]), 0);
    }

    #[test]
    fn resize_coalesce_uses_final_size_via_has_resize_flag() {
        // The single deferred `on_resize` is gated on `has_resize`, which
        // `process_run_loop_event` sets to `true` for any resize in the batch.
        // Feeding three resizes leaves the flag set once (idempotent), and the
        // coalescing helper agrees — this is exactly the `debug_assert_eq!`
        // invariant `poll_events` checks before its single `on_resize` call.
        let mut state = FrameState::default();
        let mut has_resize = false;
        let batch = vec![resize(80, 24), resize(100, 30), resize(120, 40)];
        for ev in &batch {
            process_run_loop_event(ev, &mut state, &mut has_resize);
        }
        assert!(has_resize, "any resize in the batch must set has_resize");
        assert_eq!(
            usize::from(has_resize),
            resize_invocations_for_batch(&batch)
        );
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
