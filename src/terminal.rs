use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{self, BufWriter, IsTerminal, Stdout, Write};
use std::time::{Duration, Instant};

use crossterm::event::{
    DisableBracketedPaste, DisableFocusChange, EnableBracketedPaste, EnableFocusChange,
};
// #444: other platforms write the DEC mouse modes for the chosen level.
#[cfg(windows)]
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::style::{Attribute, Print, ResetColor, SetAttribute};
use crossterm::terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate};
use crossterm::{cursor, execute, queue, terminal};

use unicode_width::UnicodeWidthStr;

use crate::buffer::{Buffer, KittyPlacement};
use crate::rect::Rect;
use crate::style::{Color, ColorDepth, Modifiers, Style, UnderlineStyle};

/// Saturating cast from `u32` to `u16` — clamps to `u16::MAX` instead of truncating.
#[inline]
fn sat_u16(v: u32) -> u16 {
    v.min(u16::MAX as u32) as u16
}
mod bench_hooks;
mod capabilities;
mod cell_pixel;
mod effects;
mod flush;
mod kitty;
mod osc;
mod query;
mod session;
mod sprixel;

pub use bench_hooks::{
    __BenchKittyFixture, __BenchSprixelFixture, __bench_flush_buffer_diff,
    __bench_flush_buffer_diff_mut, __bench_flush_buffer_diff_mut_with_buf, __bench_flush_kitty,
    __bench_flush_sprixels, __bench_new_kitty_fixture, __bench_new_sprixel_fixture,
};
pub use capabilities::{Blitter, BlitterSupport, Capabilities, capabilities};
pub use osc::{ColorScheme, detect_color_scheme, read_clipboard};

use capabilities::*;
pub(crate) use cell_pixel::*;
pub(crate) use effects::*;
use flush::*;
pub(crate) use kitty::*;
pub(crate) use osc::*;
#[cfg(unix)]
pub(crate) use query::read_pending_input;
use query::*;
pub(crate) use session::*;
pub(crate) use sprixel::*;

fn buffer_error(error: crate::buffer::BufferError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, error)
}

fn try_buffer_pair(area: Rect) -> io::Result<(Buffer, Buffer)> {
    Buffer::validate_area(area).map_err(buffer_error)?;
    let current = Buffer::try_empty(area).map_err(buffer_error)?;
    let previous = Buffer::try_empty(area).map_err(buffer_error)?;
    Ok((current, previous))
}

fn reset_buffer_pair(current: &mut Buffer, previous: &mut Buffer, area: Rect) -> io::Result<()> {
    if current.area == area && previous.area == area {
        for buffer in [current, previous] {
            // Session redraw discards cell payloads, unlike ordinary frame
            // reset which preserves their allocations. Reinitialize in the
            // existing vector so cells take the same fast path as a new pair.
            let len = buffer.content.len();
            buffer.content.clear();
            buffer.reset();
            buffer.content.resize(len, crate::Cell::default());
        }
    } else {
        // Allocate both before replacing either: a failed resize is atomic.
        let pair = try_buffer_pair(area)?;
        *current = pair.0;
        *previous = pair.1;
    }
    Ok(())
}

#[cfg(any(test, feature = "pty-test"))]
fn buffer_pair(area: Rect) -> (Buffer, Buffer) {
    try_buffer_pair(area)
        .unwrap_or_else(|error| panic!("terminal buffer pair for {area:?} failed: {error}"))
}

/// Output sink for a [`Terminal`] / [`InlineTerminal`] flush pipeline.
///
/// The production path is always [`Sink::Stdout`], a `BufWriter<Stdout>` — its
/// byte stream and buffering are byte-for-byte identical to the pre-seam code
/// (the [`Write`] impl below is a thin delegation, so the hot path is
/// unchanged). When the `pty-test` dev feature (or `cfg(test)`) is enabled, a
/// second [`Sink::Capture`] variant lets the PTY test harness drive the *real*
/// flush emitters into an in-process `Vec<u8>` instead of a terminal, so the
/// emitted escape / image-protocol bytes can be asserted end-to-end. The
/// capture variant never exists in a default build.
pub(crate) enum Sink {
    /// Production sink: buffered stdout.
    Stdout(BufWriter<Stdout>),
    /// Test sink: in-process byte capture, used only by the PTY harness.
    #[cfg(any(test, feature = "pty-test"))]
    Capture(Vec<u8>),
}

impl Write for Sink {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Sink::Stdout(w) => w.write(buf),
            #[cfg(any(test, feature = "pty-test"))]
            Sink::Capture(v) => v.write(buf),
        }
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        match self {
            Sink::Stdout(w) => w.flush(),
            #[cfg(any(test, feature = "pty-test"))]
            Sink::Capture(v) => v.flush(),
        }
    }
}

/// Fullscreen crossterm terminal backend: owns raw mode + the alternate
/// screen, double-buffers cells, and flushes only the diff each frame.
///
/// Exposed (issue #278) so external integrations can drive SLT's rendering
/// with their own event loop instead of reimplementing the backend. Pair with
/// [`crate::event::from_crossterm`] to translate input. The built-in
/// [`crate::run`] entry point uses this same type internally.
pub struct Terminal {
    stdout: Sink,
    current: Buffer,
    previous: Buffer,
    cursor_visible: bool,
    session: TerminalSessionGuard,
    color_depth: ColorDepth,
    synchronized_output: bool,
    pub(crate) theme_bg: Option<Color>,
    kitty_mgr: KittyImageManager,
    graphics_support: GraphicsEmissionSupport,
    /// Reused run-coalescing scratch for `flush_buffer_diff` (issue #269). Its
    /// capacity persists across frames so the hot flush loop never allocates a
    /// fresh `String` per call.
    run_buf: String,
    /// Title / progress / status / cursor-shape emitter, clipboard delivery
    /// and cluster-width policy (issues #446, #449, #468).
    effects: TerminalEffects,
}

/// Inline crossterm terminal backend: renders into a fixed-height region
/// below the cursor instead of taking over the whole screen.
///
/// Like [`Terminal`], exposed (issue #278) for custom integrations. Backs the
/// [`crate::run_inline`] entry point.
pub struct InlineTerminal {
    stdout: Sink,
    current: Buffer,
    previous: Buffer,
    cursor_visible: bool,
    session: TerminalSessionGuard,
    height: u32,
    anchor_row: u16,
    start_row: u16,
    viewport_rows: u16,
    reserved: bool,
    color_depth: ColorDepth,
    synchronized_output: bool,
    pub(crate) theme_bg: Option<Color>,
    kitty_mgr: KittyImageManager,
    graphics_support: GraphicsEmissionSupport,
    /// Reused run-coalescing scratch for `flush_buffer_diff` (issue #269).
    run_buf: String,
    /// See [`Terminal`]'s field of the same name.
    effects: TerminalEffects,
}

/// Initial capacity for the reused per-frame run-coalescing buffer. Sized to
/// comfortably hold a full wide terminal row of multi-byte graphemes so the
/// allocation is paid once at construction, never per frame.
const RUN_BUF_INITIAL_CAPACITY: usize = 4096;

impl Terminal {
    /// Construct a fullscreen terminal backend; enters raw mode and the
    /// alternate screen and optionally enables mouse capture and the
    /// kitty keyboard protocol. When `report_all_keys` is set (and
    /// `kitty_keyboard` is too), bare modifier presses are reported.
    ///
    /// `mouse` takes a [`MouseMode`](crate::MouseMode) or a `bool` (`true` is
    /// `MouseMode::Hover`, `false` is `MouseMode::Off`).
    pub fn new(
        mouse: impl Into<crate::MouseMode>,
        kitty_keyboard: bool,
        report_all_keys: bool,
        color_depth: ColorDepth,
    ) -> io::Result<Self> {
        let (cols, rows) = terminal::size()?;
        let area = Rect::new(0, 0, cols as u32, rows as u32);
        let (current, previous) = try_buffer_pair(area)?;

        let mut raw = io::stdout();
        let session = TerminalSessionGuard::enter(
            TerminalSessionMode::Fullscreen,
            &mut raw,
            mouse.into(),
            kitty_keyboard,
            report_all_keys,
        )?;
        let graphics_support = GraphicsEmissionSupport::detect(capabilities());
        let synchronized_output = should_emit_synchronized_update();

        // The input wake source is now installed. Remeasure after startup
        // probes so a resize before listener registration cannot be missed.
        let (cols, rows) = terminal::size()?;
        let actual = Rect::new(0, 0, u32::from(cols), u32::from(rows));
        let (current, previous) = if actual == area {
            (current, previous)
        } else {
            try_buffer_pair(actual)?
        };

        Ok(Self {
            stdout: Sink::Stdout(BufWriter::with_capacity(65536, raw)),
            current,
            previous,
            cursor_visible: false,
            session,
            color_depth,
            synchronized_output,
            theme_bg: None,
            kitty_mgr: KittyImageManager::new(),
            graphics_support,
            run_buf: String::with_capacity(RUN_BUF_INITIAL_CAPACITY),
            effects: TerminalEffects::detect(),
        })
    }

    /// Return the fullscreen terminal's current `(cols, rows)`.
    pub fn size(&self) -> (u32, u32) {
        (self.current.area.width, self.current.area.height)
    }

    /// Mutable access to the back buffer used by the next render pass.
    pub fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.current
    }

    /// Diff the back buffer against the front buffer, write the changed
    /// cells to stdout under a synchronized-output guard, then swap
    /// front and back buffers.
    pub fn flush(&mut self) -> io::Result<()> {
        if self.current.area.width < self.previous.area.width {
            execute!(self.stdout, terminal::Clear(terminal::ClearType::All))?;
        }

        // Synchronized output (BSU/ESU) is gated on the DECRQM ?2026 probe
        // (v0.21.1): emit unless the terminal definitively reported the mode
        // unrecognized. A silent / headless probe keeps emitting as before.
        let sync_guard = self.synchronized_output;
        if sync_guard {
            queue!(self.stdout, BeginSynchronizedUpdate)?;
        }
        // Issue #171: refresh both buffers' per-row digests so the per-row
        // skip inside `flush_buffer_diff` can short-circuit unchanged rows.
        // `previous` only needs a recompute when the prior frame mutated
        // it (e.g. after a swap); cheap when nothing's dirty.
        self.current.recompute_line_hashes();
        self.previous.recompute_line_hashes();
        let redrawn_sprixel_rows =
            previous_only_sprixel_rows(&self.current, &self.previous, |placement| {
                self.graphics_support
                    .should_emit_sprixel(sprixel_protocol(&placement.seq))
            });
        flush_buffer_diff_rows(
            &mut self.stdout,
            &self.current,
            &self.previous,
            self.color_depth,
            0,
            &mut self.run_buf,
            &redrawn_sprixel_rows,
        )?;

        // Kitty graphics: structured image management with IDs and compression.
        // Full-screen mode has no row offset (issue #206).
        if self.graphics_support.should_emit_kitty() {
            self.kitty_mgr
                .flush(&mut self.stdout, &self.current.kitty_placements, 0)?;
        }

        // Generic raw passthrough sequences (non-sprixel) — simple diff.
        flush_raw_sequences(&mut self.stdout, &self.current, &self.previous, 0)?;

        // Sprixels (sixel / iTerm2) — per-cell damage-tracked re-blit (#265).
        flush_sprixels_checked_with_rows(
            &mut self.stdout,
            &self.current,
            &self.previous,
            0,
            self.graphics_support,
            &redrawn_sprixel_rows,
        )?;

        if sync_guard {
            queue!(self.stdout, EndSynchronizedUpdate)?;
        }
        flush_cursor(
            &mut self.stdout,
            &mut self.cursor_visible,
            self.current.cursor_pos(),
            0,
            None,
        )?;
        let staged = self.effects.emit_meta(&mut self.stdout)?;

        self.stdout.flush()?;
        if let Some(staged) = staged {
            self.effects.deliver_clipboard(&mut self.stdout, &staged);
        }

        std::mem::swap(&mut self.current, &mut self.previous);
        if let Some(bg) = self.theme_bg {
            self.current.reset_with_bg(bg);
        } else {
            self.current.reset();
        }
        Ok(())
    }

    /// Apply run-loop configuration that depends on the startup probe:
    /// color depth resolution (issue #447) and the cluster-width policy,
    /// including the optional width probe (issue #449).
    pub(crate) fn apply_run_config(&mut self, config: &crate::RunConfig) {
        self.color_depth = crate::style::effective_color_depth(config.color_depth, || {
            ColorDepth::resolve(&capabilities())
        });
        self.effects.configure_width(config, true);
    }

    /// Cluster-width policy for the diff flush (issue #449).
    #[allow(dead_code)]
    pub(crate) fn width_policy(&self) -> WidthPolicy {
        self.effects.width
    }

    /// Re-query the terminal size and resize the front and back buffers
    /// to match. Called from the SIGWINCH handler.
    pub fn handle_resize(&mut self) -> io::Result<()> {
        let (cols, rows) = terminal::size()?;
        let area = Rect::new(0, 0, cols as u32, rows as u32);
        reset_buffer_pair(&mut self.current, &mut self.previous, area)?;
        invalidate_cell_pixel_size();
        if self.graphics_support.should_emit_kitty() {
            self.kitty_mgr.delete_all(&mut self.stdout)?;
        }
        execute!(
            self.stdout,
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(0, 0)
        )?;
        Ok(())
    }
}

#[cfg(any(test, feature = "pty-test"))]
impl Terminal {
    /// Construct a fullscreen [`Terminal`] whose flush pipeline targets an
    /// in-process byte capture instead of stdout.
    ///
    /// Used **only** by the PTY test harness ([`crate::PtyBackend`]): the
    /// production [`Terminal::new`] / [`crate::run`] path is unchanged and
    /// still binds `BufWriter<Stdout>`. No raw mode is entered and no session
    /// escapes are emitted, so this can run on a headless CI runner with no
    /// TTY. The emitted bytes — SGR runs, OSC 8, Sixel, Kitty graphics — flow
    /// through the exact same [`flush_buffer_diff`] / [`apply_style_delta`] /
    /// Sixel / Kitty emitters that a real terminal sees.
    ///
    /// `color_depth` selects the SGR encoding (truecolor vs 256-color etc.)
    /// exercised by the flush, mirroring [`Terminal::new`]'s argument.
    pub(crate) fn with_sink(width: u32, height: u32, color_depth: ColorDepth) -> Self {
        let area = Rect::new(0, 0, width, height);
        let (current, previous) = buffer_pair(area);
        Self {
            stdout: Sink::Capture(Vec::new()),
            current,
            previous,
            cursor_visible: false,
            session: TerminalSessionGuard {
                mode: TerminalSessionMode::Fullscreen,
                mouse_mode: crate::MouseMode::Off,
                kitty_keyboard: false,
                report_all_keys: false,
                raw_mode_owned: false,
                registry_id: None,
                restored: std::sync::atomic::AtomicBool::new(false),
                harness: true,
            },
            color_depth,
            synchronized_output: true,
            theme_bg: None,
            kitty_mgr: KittyImageManager::new(),
            graphics_support: GraphicsEmissionSupport::capture(),
            run_buf: String::with_capacity(RUN_BUF_INITIAL_CAPACITY),
            effects: TerminalEffects::capture(),
        }
    }

    /// Drain and return the bytes captured by a [`with_sink`](Terminal::with_sink)
    /// terminal since the last call, resetting the capture buffer.
    ///
    /// Panics if this terminal is not a captured-sink (harness) terminal.
    pub(crate) fn take_sink_bytes(&mut self) -> Vec<u8> {
        match &mut self.stdout {
            Sink::Capture(v) => std::mem::take(v),
            Sink::Stdout(_) => panic!("take_sink_bytes called on a non-capture Terminal"),
        }
    }
}

impl crate::Backend for Terminal {
    fn size(&self) -> (u32, u32) {
        Terminal::size(self)
    }

    fn buffer_mut(&mut self) -> &mut Buffer {
        Terminal::buffer_mut(self)
    }

    fn flush(&mut self) -> io::Result<()> {
        Terminal::flush(self)
    }

    fn owns_terminal_session(&self) -> bool {
        true
    }
}

impl InlineTerminal {
    /// Physical zero-based row at which the inline viewport starts.
    pub(crate) fn origin_row(&self) -> u32 {
        u32::from(self.start_row)
    }

    /// Translate physical coordinates; pixel metadata remains physical.
    pub(crate) fn localize_mouse(
        &self,
        mut event: crate::event::MouseEvent,
    ) -> Option<crate::event::MouseEvent> {
        let visible_rows = self
            .height
            .min(u32::from(self.viewport_rows).saturating_sub(self.origin_row()));
        event.y = event.y.checked_sub(self.origin_row())?;
        (event.x < self.current.area.width && event.y < visible_rows).then_some(event)
    }

    /// Construct an inline terminal backend that renders `height` rows
    /// below the current cursor without entering the alternate screen.
    /// Optionally enables mouse capture and the kitty keyboard protocol.
    /// When `report_all_keys` is set (and `kitty_keyboard` is too), bare
    /// modifier presses are reported.
    ///
    /// `mouse` takes a [`MouseMode`](crate::MouseMode) or a `bool` (`true` is
    /// `MouseMode::Hover`, `false` is `MouseMode::Off`).
    pub fn new(
        height: u32,
        mouse: impl Into<crate::MouseMode>,
        kitty_keyboard: bool,
        report_all_keys: bool,
        color_depth: ColorDepth,
    ) -> io::Result<Self> {
        let (cols, _rows) = terminal::size()?;
        let area = Rect::new(0, 0, cols as u32, height);
        let (current, previous) = try_buffer_pair(area)?;

        let mut raw = io::stdout();
        let session = TerminalSessionGuard::enter(
            TerminalSessionMode::Inline,
            &mut raw,
            mouse.into(),
            kitty_keyboard,
            report_all_keys,
        )?;
        let graphics_support = GraphicsEmissionSupport::detect(capabilities());
        let synchronized_output = should_emit_synchronized_update();

        if graphics_support.should_emit_kitty()
            || graphics_support.should_emit_sprixel(SprixelProtocol::Sixel)
            || graphics_support.should_emit_sprixel(SprixelProtocol::Iterm2)
        {
            let _ = cell_pixel_size();
        }
        let (_, cursor_row) = match crate::event::reply::cursor_position() {
            Ok(pos) => pos,
            Err(err) => {
                session.restore(&mut raw, false);
                return Err(err);
            }
        };
        let (cols, rows) = terminal::size()?;
        let actual = Rect::new(0, 0, u32::from(cols), height);
        let (current, previous) = if actual == area {
            (current, previous)
        } else {
            try_buffer_pair(actual)?
        };
        Ok(Self {
            stdout: Sink::Stdout(BufWriter::with_capacity(65536, raw)),
            current,
            previous,
            cursor_visible: false,
            session,
            height,
            anchor_row: cursor_row,
            start_row: cursor_row,
            viewport_rows: rows,
            reserved: false,
            color_depth,
            synchronized_output,
            theme_bg: None,
            kitty_mgr: KittyImageManager::new(),
            graphics_support,
            run_buf: String::with_capacity(RUN_BUF_INITIAL_CAPACITY),
            effects: TerminalEffects::detect(),
        })
    }

    /// Apply probe-dependent run configuration; see
    /// [`Terminal::apply_run_config`]. Inline sessions never run the width
    /// probe (it would print on the user's scrollback row).
    pub(crate) fn apply_run_config(&mut self, config: &crate::RunConfig) {
        self.color_depth = crate::style::effective_color_depth(config.color_depth, || {
            ColorDepth::resolve(&capabilities())
        });
        self.effects.configure_width(config, false);
    }

    /// Cluster-width policy for the diff flush (issue #449).
    #[allow(dead_code)]
    pub(crate) fn width_policy(&self) -> WidthPolicy {
        self.effects.width
    }

    /// Return the inline terminal's current `(cols, rows)`.
    pub fn size(&self) -> (u32, u32) {
        (self.current.area.width, self.current.area.height)
    }

    /// Mutable access to the back buffer used by the next render pass.
    pub fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.current
    }

    /// Diff the back buffer against the front buffer, write changed
    /// cells to stdout under a synchronized-output guard at the
    /// inline rows reserved below the cursor, then swap buffers.
    pub fn flush(&mut self) -> io::Result<()> {
        if self.current.area.width < self.previous.area.width {
            execute!(self.stdout, terminal::Clear(terminal::ClearType::All))?;
        }

        // Synchronized output (BSU/ESU) is gated on the DECRQM ?2026 probe
        // (v0.21.1); see `Terminal::flush`. Silent / headless keeps emitting.
        let sync_guard = self.synchronized_output;
        if sync_guard {
            queue!(self.stdout, BeginSynchronizedUpdate)?;
        }

        if !self.reserved {
            queue!(self.stdout, cursor::MoveToColumn(0))?;
            for _ in 0..self.height {
                queue!(self.stdout, Print("\n"))?;
            }
            self.reserved = true;

            let (_, rows) = terminal::size()?;
            self.viewport_rows = rows;
            self.start_row = self
                .anchor_row
                .min(rows.saturating_sub(sat_u16(self.height)));
        }
        let row_offset = self.start_row as u32;
        // Issue #171: refresh per-row digests before the diff so the
        // unchanged-row skip can fire (same call shape as `Terminal::flush`).
        self.current.recompute_line_hashes();
        self.previous.recompute_line_hashes();
        let redrawn_sprixel_rows =
            previous_only_sprixel_rows(&self.current, &self.previous, |placement| {
                self.graphics_support
                    .should_emit_sprixel(sprixel_protocol(&placement.seq))
            });
        flush_buffer_diff_rows(
            &mut self.stdout,
            &self.current,
            &self.previous,
            self.color_depth,
            row_offset,
            &mut self.run_buf,
            &redrawn_sprixel_rows,
        )?;

        // Kitty graphics: structured image management with IDs and compression.
        // Issue #206: pass `row_offset` instead of materializing a translated
        // `Vec<KittyPlacement>` copy — `KittyImageManager::flush` applies the
        // offset arithmetically at point of use and stores post-offset y in
        // `prev_placements` for the next frame's diff.
        if self.graphics_support.should_emit_kitty() {
            self.kitty_mgr
                .flush(&mut self.stdout, &self.current.kitty_placements, row_offset)?;
        }

        // Generic raw passthrough sequences (non-sprixel) — simple diff.
        flush_raw_sequences(&mut self.stdout, &self.current, &self.previous, row_offset)?;

        // Sprixels (sixel / iTerm2) — per-cell damage-tracked re-blit (#265).
        flush_sprixels_checked_with_rows(
            &mut self.stdout,
            &self.current,
            &self.previous,
            row_offset,
            self.graphics_support,
            &redrawn_sprixel_rows,
        )?;

        if sync_guard {
            queue!(self.stdout, EndSynchronizedUpdate)?;
        }
        let fallback_row = row_offset + self.height.saturating_sub(1);
        flush_cursor(
            &mut self.stdout,
            &mut self.cursor_visible,
            self.current.cursor_pos(),
            row_offset,
            Some(fallback_row),
        )?;
        let staged = self.effects.emit_meta(&mut self.stdout)?;

        self.stdout.flush()?;
        if let Some(staged) = staged {
            self.effects.deliver_clipboard(&mut self.stdout, &staged);
        }

        std::mem::swap(&mut self.current, &mut self.previous);
        reset_current_buffer(&mut self.current, self.theme_bg);
        Ok(())
    }

    /// Write permanent lines above the inline region and invalidate its diff.
    pub(crate) fn write_scrollback(&mut self, lines: &[String]) -> io::Result<()> {
        if lines.is_empty() {
            return Ok(());
        }

        if self.graphics_support.should_emit_kitty() {
            self.kitty_mgr.delete_all(&mut self.stdout)?;
        }
        queue!(
            self.stdout,
            cursor::MoveTo(0, self.start_row),
            terminal::Clear(terminal::ClearType::FromCursorDown)
        )?;
        let width = self.current.area.width.max(1);
        let mut physical_rows = 0u32;
        for line in lines {
            let safe = crate::sanitize_terminal_text(line);
            for row in scrollback_rows(&safe, width) {
                queue!(self.stdout, Print(row), Print("\r\n"))?;
                physical_rows = physical_rows.saturating_add(1);
            }
        }
        // Reserve the dynamic rows after the permanent text, letting LF at
        // the bottom scroll the text into history before the next repaint.
        for _ in 0..self.height.saturating_sub(1) {
            queue!(self.stdout, Print("\r\n"))?;
        }
        self.stdout.flush()?;

        self.start_row = self
            .start_row
            .saturating_add(sat_u16(physical_rows))
            .min(self.viewport_rows.saturating_sub(sat_u16(self.height)));
        self.anchor_row = self.start_row;
        self.reserved = true;
        self.previous.reset();
        self.cursor_visible = false;
        Ok(())
    }

    /// Re-query both terminal dimensions and preserve the inline anchor.
    pub fn handle_resize(&mut self) -> io::Result<()> {
        let (cols, rows) = terminal::size()?;
        self.handle_resize_to(cols, rows)
    }

    fn handle_resize_to(&mut self, cols: u16, rows: u16) -> io::Result<()> {
        let previous_start_row = self.start_row;
        let start_row = self
            .anchor_row
            .min(rows.saturating_sub(sat_u16(self.height)));
        let area = Rect::new(0, 0, cols as u32, self.height);
        reset_buffer_pair(&mut self.current, &mut self.previous, area)?;
        invalidate_cell_pixel_size();
        self.viewport_rows = rows;
        self.start_row = start_row;
        if self.graphics_support.should_emit_kitty() {
            self.kitty_mgr.delete_all(&mut self.stdout)?;
        }
        self.cursor_visible = false;
        // Erase only rows owned by either placement, including a previously
        // bottom-clamped viewport that moves down again when the host grows.
        for row in 0..rows {
            if (row >= previous_start_row && u32::from(row - previous_start_row) < self.height)
                || (row >= self.start_row && u32::from(row - self.start_row) < self.height)
            {
                queue!(
                    self.stdout,
                    cursor::MoveTo(0, row),
                    terminal::Clear(terminal::ClearType::CurrentLine)
                )?;
            }
        }
        execute!(self.stdout, cursor::MoveTo(0, self.start_row))?;
        Ok(())
    }
}

fn scrollback_rows(text: &str, width: u32) -> Vec<String> {
    use unicode_segmentation::UnicodeSegmentation;
    let width = width.max(1) as usize;
    let mut rows = Vec::new();
    let mut row = String::new();
    let mut used = 0;
    for grapheme in text.graphemes(true) {
        let cells = UnicodeWidthStr::width(grapheme);
        if used > 0 && used + cells > width {
            rows.push(std::mem::take(&mut row));
            used = 0;
        }
        if cells > width {
            row.push('?');
            used += 1;
        } else {
            row.push_str(grapheme);
            used += cells;
        }
    }
    rows.push(row);
    rows
}

#[cfg(test)]
impl InlineTerminal {
    fn with_sink(width: u16, viewport_rows: u16, height: u32, anchor_row: u16) -> Self {
        let area = Rect::new(0, 0, width as u32, height);
        let (current, previous) = buffer_pair(area);
        let start_row = anchor_row.min(viewport_rows.saturating_sub(sat_u16(height)));
        Self {
            stdout: Sink::Capture(Vec::new()),
            current,
            previous,
            cursor_visible: false,
            session: TerminalSessionGuard {
                mode: TerminalSessionMode::Inline,
                mouse_mode: crate::MouseMode::Off,
                kitty_keyboard: false,
                report_all_keys: false,
                raw_mode_owned: false,
                registry_id: None,
                restored: std::sync::atomic::AtomicBool::new(false),
                harness: true,
            },
            height,
            anchor_row,
            start_row,
            viewport_rows,
            reserved: false,
            color_depth: ColorDepth::TrueColor,
            synchronized_output: true,
            theme_bg: None,
            kitty_mgr: KittyImageManager::new(),
            graphics_support: GraphicsEmissionSupport {
                real_terminal: false,
                capabilities: Capabilities::default(),
                force_kitty: false,
                force_sixel: false,
                force_iterm: false,
            },
            run_buf: String::with_capacity(RUN_BUF_INITIAL_CAPACITY),
            effects: TerminalEffects::capture(),
        }
    }

    fn take_sink_bytes(&mut self) -> Vec<u8> {
        match &mut self.stdout {
            Sink::Capture(bytes) => std::mem::take(bytes),
            Sink::Stdout(_) => unreachable!("test inline terminal always captures"),
        }
    }
}

impl crate::Backend for InlineTerminal {
    fn size(&self) -> (u32, u32) {
        InlineTerminal::size(self)
    }

    fn buffer_mut(&mut self) -> &mut Buffer {
        InlineTerminal::buffer_mut(self)
    }

    fn flush(&mut self) -> io::Result<()> {
        InlineTerminal::flush(self)
    }

    fn owns_terminal_session(&self) -> bool {
        true
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        // Clean up Kitty images before leaving alternate screen
        if self.graphics_support.should_emit_kitty() {
            let _ = self.kitty_mgr.delete_all(&mut self.stdout);
        }
        let _ = self.stdout.flush();
        self.session.restore(&mut self.stdout, false);
    }
}

impl Drop for InlineTerminal {
    fn drop(&mut self) {
        if self.graphics_support.should_emit_kitty() {
            let _ = self.kitty_mgr.delete_all(&mut self.stdout);
        }
        let _ = self.stdout.flush();
        self.session.restore(&mut self.stdout, self.reserved);
    }
}

mod selection;
pub(crate) use selection::{
    ScrollView, SelectionState, apply_selection_overlay_in, extract_selection_text_in,
    track_selection,
};
#[cfg(test)]
pub(crate) use selection::{apply_selection_overlay, extract_selection_text};
#[cfg(test)]
pub(crate) use selection::{find_innermost_rect, normalize_selection};

#[cfg(test)]
mod tests;
