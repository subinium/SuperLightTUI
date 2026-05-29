use std::collections::HashMap;
use std::io::{self, BufWriter, Read, Stdout, Write};
use std::time::{Duration, Instant};

use crossterm::event::{
    DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
    EnableFocusChange, EnableMouseCapture,
};
use crossterm::style::{
    Attribute, Color as CtColor, Print, ResetColor, SetAttribute, SetBackgroundColor,
    SetForegroundColor,
};
use crossterm::terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate};
use crossterm::{cursor, execute, queue, terminal};

use unicode_width::UnicodeWidthStr;

use crate::buffer::{Buffer, KittyPlacement};
use crate::rect::Rect;
use crate::style::{Color, ColorDepth, Modifiers, Style};

/// Saturating cast from `u32` to `u16` — clamps to `u16::MAX` instead of truncating.
#[inline]
fn sat_u16(v: u32) -> u16 {
    v.min(u16::MAX as u32) as u16
}

// ---------------------------------------------------------------------------
// Kitty graphics protocol image manager
// ---------------------------------------------------------------------------

/// Manages Kitty graphics protocol image IDs, uploads, and placements.
///
/// Images are deduplicated by content hash — identical RGBA data is uploaded
/// only once. Each frame, placements are diffed against the previous frame
/// to minimize terminal I/O.
pub(crate) struct KittyImageManager {
    next_id: u32,
    /// content_hash → kitty image ID for uploaded images.
    uploaded: HashMap<u64, u32>,
    /// Previous frame's placements (for diff).
    prev_placements: Vec<KittyPlacement>,
}

impl KittyImageManager {
    /// Construct a new image manager with no uploaded images.
    pub fn new() -> Self {
        Self {
            next_id: 1,
            uploaded: HashMap::new(),
            prev_placements: Vec::new(),
        }
    }

    /// Flush Kitty image placements: upload new images, manage placements.
    ///
    /// `row_offset` shifts `current[i].y` for both terminal output and the
    /// diff comparison against `prev_placements`. Stored placements always
    /// include the offset (the displayed `y`) so re-emit detection works
    /// across resize even when the offset itself changes (issue #206).
    pub fn flush(
        &mut self,
        stdout: &mut impl Write,
        current: &[KittyPlacement],
        row_offset: u32,
    ) -> io::Result<()> {
        // Fast path: nothing changed (compare against post-offset y values
        // stored in `prev_placements`). This avoids materializing a translated
        // `Vec<KittyPlacement>` in the caller (issue #206).
        if current.len() == self.prev_placements.len()
            && current
                .iter()
                .zip(self.prev_placements.iter())
                .all(|(c, p)| placement_eq_with_offset(c, row_offset, p))
        {
            return Ok(());
        }

        // Delete all previous placements (keep uploaded image data for reuse)
        if !self.prev_placements.is_empty() {
            // Delete all visible placements by ID
            let mut deleted_ids = std::collections::HashSet::new();
            for p in &self.prev_placements {
                if let Some(&img_id) = self.uploaded.get(&p.content_hash) {
                    if deleted_ids.insert(img_id) {
                        // Delete all placements of this image (but keep image data)
                        queue!(
                            stdout,
                            Print(format!("\x1b_Ga=d,d=i,i={},q=2\x1b\\", img_id))
                        )?;
                    }
                }
            }
        }

        // Upload new images and create placements
        for (idx, p) in current.iter().enumerate() {
            let img_id = if let Some(&existing_id) = self.uploaded.get(&p.content_hash) {
                existing_id
            } else {
                // Upload new image with zlib compression if available
                let id = self.next_id;
                self.next_id += 1;
                self.upload_image(stdout, id, p)?;
                self.uploaded.insert(p.content_hash, id);
                id
            };

            // Place the image (with row_offset applied to y at point of use).
            let pid = idx as u32 + 1;
            self.place_image_offset(stdout, img_id, pid, p, row_offset)?;
        }

        // Clean up images no longer used by any placement
        let used_hashes: std::collections::HashSet<u64> =
            current.iter().map(|p| p.content_hash).collect();
        let stale: Vec<u64> = self
            .uploaded
            .keys()
            .filter(|h| !used_hashes.contains(h))
            .copied()
            .collect();
        for hash in stale {
            if let Some(id) = self.uploaded.remove(&hash) {
                // Delete image data from terminal memory
                queue!(stdout, Print(format!("\x1b_Ga=d,d=I,i={},q=2\x1b\\", id)))?;
            }
        }

        // Persist post-offset placements for the next frame's diff. We still
        // write `current.len()` items but rebuild the Vec in place — capacity
        // is preserved across frames so this is at most an `Arc::clone` per
        // image (the `Vec<u8>` is shared via `Arc`, no pixel copy). This
        // remains the only `Arc::clone` cost; the per-frame `Vec` allocation
        // in the caller (`InlineTerminal::flush`) is what #206 eliminates.
        self.prev_placements.clear();
        self.prev_placements.reserve(current.len());
        for p in current {
            let mut copy = p.clone();
            copy.y = copy.y.saturating_add(row_offset);
            self.prev_placements.push(copy);
        }
        Ok(())
    }

    /// Upload image data to the terminal with `a=t` (transmit only, no display).
    fn upload_image(&self, stdout: &mut impl Write, id: u32, p: &KittyPlacement) -> io::Result<()> {
        let (payload, compression) = compress_rgba(&p.rgba);
        let encoded = base64_encode(&payload);
        let chunks = split_base64(&encoded, 4096);

        for (i, chunk) in chunks.iter().enumerate() {
            let more = if i < chunks.len() - 1 { 1 } else { 0 };
            if i == 0 {
                queue!(
                    stdout,
                    Print(format!(
                        "\x1b_Ga=t,i={},f=32,{}s={},v={},q=2,m={};{}\x1b\\",
                        id, compression, p.src_width, p.src_height, more, chunk
                    ))
                )?;
            } else {
                queue!(stdout, Print(format!("\x1b_Gm={};{}\x1b\\", more, chunk)))?;
            }
        }
        Ok(())
    }

    /// Place an already-uploaded image at a screen position with optional crop.
    ///
    /// `row_offset` is added to `p.y` at output time so callers (notably
    /// `InlineTerminal::flush`) can avoid materializing a translated copy of
    /// the placements list per frame (issue #206).
    fn place_image_offset(
        &self,
        stdout: &mut impl Write,
        img_id: u32,
        placement_id: u32,
        p: &KittyPlacement,
        row_offset: u32,
    ) -> io::Result<()> {
        let display_y = p.y.saturating_add(row_offset);
        queue!(stdout, cursor::MoveTo(sat_u16(p.x), sat_u16(display_y)))?;

        let mut cmd = format!(
            "\x1b_Ga=p,i={},p={},c={},r={},C=1,q=2",
            img_id, placement_id, p.cols, p.rows
        );

        // Add crop parameters for scroll clipping
        if p.crop_y > 0 || p.crop_h > 0 {
            cmd.push_str(&format!(",y={}", p.crop_y));
            if p.crop_h > 0 {
                cmd.push_str(&format!(",h={}", p.crop_h));
            }
        }

        cmd.push_str("\x1b\\");
        queue!(stdout, Print(cmd))?;
        Ok(())
    }

    /// Delete all images from the terminal (used on drop/cleanup).
    pub fn delete_all(&self, stdout: &mut impl Write) -> io::Result<()> {
        queue!(stdout, Print("\x1b_Ga=d,d=A,q=2\x1b\\"))
    }
}

/// Compare a fresh placement (`current`, in pre-offset coordinates) against a
/// stored placement (`prev`, already includes any prior `row_offset`).
///
/// Equivalent to `*current == *prev` after virtually applying `row_offset` to
/// `current.y`, without materializing the translated copy. Used by
/// `KittyImageManager::flush` to keep the diff fast-path even when the inline
/// terminal applies a non-zero offset (issue #206).
#[inline]
fn placement_eq_with_offset(
    current: &KittyPlacement,
    row_offset: u32,
    prev: &KittyPlacement,
) -> bool {
    current.content_hash == prev.content_hash
        && current.x == prev.x
        && current.y.saturating_add(row_offset) == prev.y
        && current.cols == prev.cols
        && current.rows == prev.rows
        && current.crop_y == prev.crop_y
        && current.crop_h == prev.crop_h
}

/// Compress RGBA data with zlib if available, returning (payload, format_string).
fn compress_rgba(data: &[u8]) -> (Vec<u8>, &'static str) {
    #[cfg(feature = "kitty-compress")]
    {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
        if encoder.write_all(data).is_ok() {
            if let Ok(compressed) = encoder.finish() {
                // Only use compression if it actually saves space
                if compressed.len() < data.len() {
                    return (compressed, "o=z,");
                }
            }
        }
    }
    (data.to_vec(), "")
}

/// Query the terminal for the actual cell pixel dimensions via CSI 16 t.
///
/// Returns `(cell_width, cell_height)` in pixels. Falls back to `(8, 16)` if
/// detection fails. Used by `kitty_image_fit` for accurate aspect ratio.
///
/// Cached after first successful detection.
pub fn cell_pixel_size() -> (u32, u32) {
    use std::sync::OnceLock;
    static CACHED: OnceLock<(u32, u32)> = OnceLock::new();
    *CACHED.get_or_init(|| detect_cell_pixel_size().unwrap_or((8, 16)))
}

fn detect_cell_pixel_size() -> Option<(u32, u32)> {
    // CSI 16 t → reports cell size as CSI 6 ; height ; width t
    let mut stdout = io::stdout();
    write!(stdout, "\x1b[16t").ok()?;
    stdout.flush().ok()?;

    let response = read_osc_response(Duration::from_millis(100))?;

    // Parse: ESC [ 6 ; <height> ; <width> t
    let body = response.strip_prefix("\x1b[6;").or_else(|| {
        // CSI can also start with 0x9B (single-byte CSI)
        let bytes = response.as_bytes();
        if bytes.len() > 3 && bytes[0] == 0x9b && bytes[1] == b'6' && bytes[2] == b';' {
            Some(&response[3..])
        } else {
            None
        }
    })?;
    let body = body
        .strip_suffix('t')
        .or_else(|| body.strip_suffix("t\x1b"))?;
    let mut parts = body.split(';');
    let ch: u32 = parts.next()?.parse().ok()?;
    let cw: u32 = parts.next()?.parse().ok()?;
    if cw > 0 && ch > 0 {
        Some((cw, ch))
    } else {
        None
    }
}

fn split_base64(encoded: &str, chunk_size: usize) -> Vec<&str> {
    let mut chunks = Vec::new();
    let bytes = encoded.as_bytes();
    let mut offset = 0;
    while offset < bytes.len() {
        let end = (offset + chunk_size).min(bytes.len());
        chunks.push(&encoded[offset..end]);
        offset = end;
    }
    if chunks.is_empty() {
        chunks.push("");
    }
    chunks
}

pub(crate) struct Terminal {
    stdout: BufWriter<Stdout>,
    current: Buffer,
    previous: Buffer,
    cursor_visible: bool,
    session: TerminalSessionGuard,
    color_depth: ColorDepth,
    pub(crate) theme_bg: Option<Color>,
    kitty_mgr: KittyImageManager,
}

pub(crate) struct InlineTerminal {
    stdout: BufWriter<Stdout>,
    current: Buffer,
    previous: Buffer,
    cursor_visible: bool,
    session: TerminalSessionGuard,
    height: u32,
    start_row: u16,
    reserved: bool,
    color_depth: ColorDepth,
    pub(crate) theme_bg: Option<Color>,
    kitty_mgr: KittyImageManager,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalSessionMode {
    Fullscreen,
    Inline,
}

#[derive(Debug, Clone, Copy)]
struct TerminalSessionGuard {
    mode: TerminalSessionMode,
    mouse_enabled: bool,
    kitty_keyboard: bool,
    report_all_keys: bool,
}

impl TerminalSessionGuard {
    fn enter(
        mode: TerminalSessionMode,
        stdout: &mut impl Write,
        mouse_enabled: bool,
        kitty_keyboard: bool,
        report_all_keys: bool,
    ) -> io::Result<Self> {
        let guard = Self {
            mode,
            mouse_enabled,
            kitty_keyboard,
            report_all_keys,
        };

        terminal::enable_raw_mode()?;
        if let Err(err) = write_session_enter(stdout, &guard) {
            guard.restore(stdout, false);
            return Err(err);
        }

        Ok(guard)
    }

    fn restore(&self, stdout: &mut impl Write, inline_reserved: bool) {
        if self.kitty_keyboard {
            use crossterm::event::PopKeyboardEnhancementFlags;
            let _ = execute!(stdout, PopKeyboardEnhancementFlags);
        }
        if self.mouse_enabled {
            let _ = execute!(stdout, DisableMouseCapture);
        }
        let _ = execute!(stdout, DisableFocusChange);
        let _ = write_session_cleanup(stdout, self.mode, inline_reserved);
        let _ = terminal::disable_raw_mode();
    }
}

impl Terminal {
    /// Construct a fullscreen terminal backend; enters raw mode and the
    /// alternate screen and optionally enables mouse capture and the
    /// kitty keyboard protocol. When `report_all_keys` is set (and
    /// `kitty_keyboard` is too), bare modifier presses are reported.
    pub fn new(
        mouse: bool,
        kitty_keyboard: bool,
        report_all_keys: bool,
        color_depth: ColorDepth,
    ) -> io::Result<Self> {
        let (cols, rows) = terminal::size()?;
        let area = Rect::new(0, 0, cols as u32, rows as u32);

        let mut raw = io::stdout();
        let session = TerminalSessionGuard::enter(
            TerminalSessionMode::Fullscreen,
            &mut raw,
            mouse,
            kitty_keyboard,
            report_all_keys,
        )?;

        Ok(Self {
            stdout: BufWriter::with_capacity(65536, raw),
            current: Buffer::empty(area),
            previous: Buffer::empty(area),
            cursor_visible: false,
            session,
            color_depth,
            theme_bg: None,
            kitty_mgr: KittyImageManager::new(),
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

        queue!(self.stdout, BeginSynchronizedUpdate)?;
        // Issue #171: refresh both buffers' per-row digests so the per-row
        // skip inside `flush_buffer_diff` can short-circuit unchanged rows.
        // `previous` only needs a recompute when the prior frame mutated
        // it (e.g. after a swap); cheap when nothing's dirty.
        self.current.recompute_line_hashes();
        self.previous.recompute_line_hashes();
        flush_buffer_diff(
            &mut self.stdout,
            &self.current,
            &self.previous,
            self.color_depth,
            0,
        )?;

        // Kitty graphics: structured image management with IDs and compression.
        // Full-screen mode has no row offset (issue #206).
        self.kitty_mgr
            .flush(&mut self.stdout, &self.current.kitty_placements, 0)?;

        // Raw sequences (sixel, other passthrough) — simple diff
        flush_raw_sequences(&mut self.stdout, &self.current, &self.previous, 0)?;

        queue!(self.stdout, EndSynchronizedUpdate)?;
        flush_cursor(
            &mut self.stdout,
            &mut self.cursor_visible,
            self.current.cursor_pos(),
            0,
            None,
        )?;

        self.stdout.flush()?;

        std::mem::swap(&mut self.current, &mut self.previous);
        if let Some(bg) = self.theme_bg {
            self.current.reset_with_bg(bg);
        } else {
            self.current.reset();
        }
        Ok(())
    }

    /// Re-query the terminal size and resize the front and back buffers
    /// to match. Called from the SIGWINCH handler.
    pub fn handle_resize(&mut self) -> io::Result<()> {
        let (cols, rows) = terminal::size()?;
        let area = Rect::new(0, 0, cols as u32, rows as u32);
        self.current.resize(area);
        self.previous.resize(area);
        execute!(
            self.stdout,
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(0, 0)
        )?;
        Ok(())
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
}

impl InlineTerminal {
    /// Construct an inline terminal backend that renders `height` rows
    /// below the current cursor without entering the alternate screen.
    /// Optionally enables mouse capture and the kitty keyboard protocol.
    /// When `report_all_keys` is set (and `kitty_keyboard` is too), bare
    /// modifier presses are reported.
    pub fn new(
        height: u32,
        mouse: bool,
        kitty_keyboard: bool,
        report_all_keys: bool,
        color_depth: ColorDepth,
    ) -> io::Result<Self> {
        let (cols, _) = terminal::size()?;
        let area = Rect::new(0, 0, cols as u32, height);

        let mut raw = io::stdout();
        let session = TerminalSessionGuard::enter(
            TerminalSessionMode::Inline,
            &mut raw,
            mouse,
            kitty_keyboard,
            report_all_keys,
        )?;

        let (_, cursor_row) = match cursor::position() {
            Ok(pos) => pos,
            Err(err) => {
                session.restore(&mut raw, false);
                return Err(err);
            }
        };
        Ok(Self {
            stdout: BufWriter::with_capacity(65536, raw),
            current: Buffer::empty(area),
            previous: Buffer::empty(area),
            cursor_visible: false,
            session,
            height,
            start_row: cursor_row,
            reserved: false,
            color_depth,
            theme_bg: None,
            kitty_mgr: KittyImageManager::new(),
        })
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

        queue!(self.stdout, BeginSynchronizedUpdate)?;

        if !self.reserved {
            queue!(self.stdout, cursor::MoveToColumn(0))?;
            for _ in 0..self.height {
                queue!(self.stdout, Print("\n"))?;
            }
            self.reserved = true;

            let (_, rows) = terminal::size()?;
            let bottom = self.start_row.saturating_add(sat_u16(self.height));
            if bottom > rows {
                self.start_row = rows.saturating_sub(sat_u16(self.height));
            }
        }
        let row_offset = self.start_row as u32;
        // Issue #171: refresh per-row digests before the diff so the
        // unchanged-row skip can fire (same call shape as `Terminal::flush`).
        self.current.recompute_line_hashes();
        self.previous.recompute_line_hashes();
        flush_buffer_diff(
            &mut self.stdout,
            &self.current,
            &self.previous,
            self.color_depth,
            row_offset,
        )?;

        // Kitty graphics: structured image management with IDs and compression.
        // Issue #206: pass `row_offset` instead of materializing a translated
        // `Vec<KittyPlacement>` copy — `KittyImageManager::flush` applies the
        // offset arithmetically at point of use and stores post-offset y in
        // `prev_placements` for the next frame's diff.
        self.kitty_mgr
            .flush(&mut self.stdout, &self.current.kitty_placements, row_offset)?;

        // Raw sequences (sixel, other passthrough) — simple diff
        flush_raw_sequences(&mut self.stdout, &self.current, &self.previous, row_offset)?;

        queue!(self.stdout, EndSynchronizedUpdate)?;
        let fallback_row = row_offset + self.height.saturating_sub(1);
        flush_cursor(
            &mut self.stdout,
            &mut self.cursor_visible,
            self.current.cursor_pos(),
            row_offset,
            Some(fallback_row),
        )?;

        self.stdout.flush()?;

        std::mem::swap(&mut self.current, &mut self.previous);
        reset_current_buffer(&mut self.current, self.theme_bg);
        Ok(())
    }

    /// Re-query the terminal size and resize the inline buffers to match
    /// the new column count, preserving the inline row height.
    pub fn handle_resize(&mut self) -> io::Result<()> {
        let (cols, _) = terminal::size()?;
        let area = Rect::new(0, 0, cols as u32, self.height);
        self.current.resize(area);
        self.previous.resize(area);
        execute!(
            self.stdout,
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(0, 0)
        )?;
        Ok(())
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
}

impl Drop for Terminal {
    fn drop(&mut self) {
        // Clean up Kitty images before leaving alternate screen
        let _ = self.kitty_mgr.delete_all(&mut self.stdout);
        let _ = self.stdout.flush();
        self.session.restore(&mut self.stdout, false);
    }
}

impl Drop for InlineTerminal {
    fn drop(&mut self) {
        let _ = self.kitty_mgr.delete_all(&mut self.stdout);
        let _ = self.stdout.flush();
        self.session.restore(&mut self.stdout, self.reserved);
    }
}

mod selection;
pub(crate) use selection::{apply_selection_overlay, extract_selection_text, SelectionState};
#[cfg(test)]
pub(crate) use selection::{find_innermost_rect, normalize_selection};

/// Detected terminal color scheme from OSC 11.
#[non_exhaustive]
#[cfg(feature = "crossterm")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorScheme {
    /// Dark background detected.
    Dark,
    /// Light background detected.
    Light,
    /// Could not determine the scheme.
    Unknown,
}

#[cfg(feature = "crossterm")]
fn read_osc_response(timeout: Duration) -> Option<String> {
    let deadline = Instant::now() + timeout;
    let mut stdin = io::stdin();
    let mut bytes = Vec::new();
    let mut buf = [0u8; 1];

    while Instant::now() < deadline {
        if !crossterm::event::poll(Duration::from_millis(10)).ok()? {
            continue;
        }

        let read = stdin.read(&mut buf).ok()?;
        if read == 0 {
            continue;
        }

        bytes.push(buf[0]);

        if buf[0] == b'\x07' {
            break;
        }
        let len = bytes.len();
        if len >= 2 && bytes[len - 2] == 0x1B && bytes[len - 1] == b'\\' {
            break;
        }

        if bytes.len() >= 4096 {
            break;
        }
    }

    if bytes.is_empty() {
        return None;
    }

    String::from_utf8(bytes).ok()
}

/// Query the terminal's background color via OSC 11 and return the detected scheme.
#[cfg(feature = "crossterm")]
pub fn detect_color_scheme() -> ColorScheme {
    let mut stdout = io::stdout();
    if write!(stdout, "\x1b]11;?\x07").is_err() {
        return ColorScheme::Unknown;
    }
    if stdout.flush().is_err() {
        return ColorScheme::Unknown;
    }

    let Some(response) = read_osc_response(Duration::from_millis(100)) else {
        return ColorScheme::Unknown;
    };

    parse_osc11_response(&response)
}

#[cfg(feature = "crossterm")]
pub(crate) fn parse_osc11_response(response: &str) -> ColorScheme {
    let Some(rgb_pos) = response.find("rgb:") else {
        return ColorScheme::Unknown;
    };

    let payload = &response[rgb_pos + 4..];
    let end = payload
        .find(['\x07', '\x1b', '\r', '\n', ' ', '\t'])
        .unwrap_or(payload.len());
    let rgb = &payload[..end];

    let mut channels = rgb.split('/');
    let (Some(r), Some(g), Some(b), None) = (
        channels.next(),
        channels.next(),
        channels.next(),
        channels.next(),
    ) else {
        return ColorScheme::Unknown;
    };

    fn parse_channel(channel: &str) -> Option<f64> {
        if channel.is_empty() || channel.len() > 4 {
            return None;
        }
        let value = u16::from_str_radix(channel, 16).ok()? as f64;
        let max = ((1u32 << (channel.len() * 4)) - 1) as f64;
        if max <= 0.0 {
            return None;
        }
        Some((value / max).clamp(0.0, 1.0))
    }

    let (Some(r), Some(g), Some(b)) = (parse_channel(r), parse_channel(g), parse_channel(b)) else {
        return ColorScheme::Unknown;
    };

    let luminance = 0.299 * r + 0.587 * g + 0.114 * b;
    if luminance < 0.5 {
        ColorScheme::Dark
    } else {
        ColorScheme::Light
    }
}

fn base64_encode(input: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        out.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 {
            CHARS[((triple >> 6) & 0x3F) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            CHARS[(triple & 0x3F) as usize] as char
        } else {
            '='
        });
    }
    out
}

pub(crate) fn copy_to_clipboard(w: &mut impl Write, text: &str) -> io::Result<()> {
    let encoded = base64_encode(text.as_bytes());
    write!(w, "\x1b]52;c;{encoded}\x1b\\")?;
    w.flush()
}

#[cfg(feature = "crossterm")]
fn parse_osc52_response(response: &str) -> Option<String> {
    let osc_pos = response.find("]52;")?;
    let body = &response[osc_pos + 4..];
    let semicolon = body.find(';')?;
    let payload = &body[semicolon + 1..];

    let end = payload
        .find("\x1b\\")
        .or_else(|| payload.find('\x07'))
        .unwrap_or(payload.len());
    let encoded = payload[..end].trim();
    if encoded.is_empty() || encoded == "?" {
        return None;
    }

    base64_decode(encoded)
}

/// Read clipboard contents via OSC 52 terminal query.
#[cfg(feature = "crossterm")]
pub fn read_clipboard() -> Option<String> {
    let mut stdout = io::stdout();
    write!(stdout, "\x1b]52;c;?\x07").ok()?;
    stdout.flush().ok()?;

    let response = read_osc_response(Duration::from_millis(200))?;
    parse_osc52_response(&response)
}

#[cfg(feature = "crossterm")]
fn base64_decode(input: &str) -> Option<String> {
    let mut filtered: Vec<u8> = input
        .bytes()
        .filter(|b| !matches!(b, b' ' | b'\n' | b'\r' | b'\t'))
        .collect();

    match filtered.len() % 4 {
        0 => {}
        2 => filtered.extend_from_slice(b"=="),
        3 => filtered.push(b'='),
        _ => return None,
    }

    fn decode_val(b: u8) -> Option<u8> {
        match b {
            b'A'..=b'Z' => Some(b - b'A'),
            b'a'..=b'z' => Some(b - b'a' + 26),
            b'0'..=b'9' => Some(b - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }

    let mut out = Vec::with_capacity((filtered.len() / 4) * 3);
    for chunk in filtered.chunks_exact(4) {
        let p2 = chunk[2] == b'=';
        let p3 = chunk[3] == b'=';
        if p2 && !p3 {
            return None;
        }

        let v0 = decode_val(chunk[0])? as u32;
        let v1 = decode_val(chunk[1])? as u32;
        let v2 = if p2 { 0 } else { decode_val(chunk[2])? as u32 };
        let v3 = if p3 { 0 } else { decode_val(chunk[3])? as u32 };

        let triple = (v0 << 18) | (v1 << 12) | (v2 << 6) | v3;
        out.push(((triple >> 16) & 0xFF) as u8);
        if !p2 {
            out.push(((triple >> 8) & 0xFF) as u8);
        }
        if !p3 {
            out.push((triple & 0xFF) as u8);
        }
    }

    String::from_utf8(out).ok()
}

#[allow(clippy::too_many_arguments)]
#[allow(unused_assignments)]
fn flush_buffer_diff(
    stdout: &mut impl Write,
    current: &Buffer,
    previous: &Buffer,
    color_depth: ColorDepth,
    row_offset: u32,
) -> io::Result<()> {
    // Run-coalescing: consecutive changed cells in the same row that share
    // `Style` + `hyperlink` + contiguous x-coordinates are emitted as a single
    // `Print(run)` after one cursor move and one style delta. This cuts the
    // number of `queue!` calls on a full redraw from O(cells) to
    // O(style-change boundaries), which is the dominant stdout write cost.
    //
    // A run is broken whenever:
    //   * style, hyperlink, or row changes,
    //   * the next cell is not at the expected next column (gap from skipped
    //     cells — unchanged, empty wide-char trailer, or end of row),
    //   * end-of-row (always flushed before descending to the next row).
    let mut last_style = Style::new();
    let mut first_style = true;
    let mut active_link: Option<&str> = None;
    let mut has_updates = false;
    // Where we believe the cursor currently sits — lets us skip a redundant
    // `MoveTo` when a new run starts exactly where the previous one ended
    // (e.g. split only by a style change on otherwise contiguous columns).
    let mut last_cursor: Option<(u32, u32)> = None;

    // Active run state. `run_next_col` is the column the next cell must
    // occupy to extend the run; `run_open` guards the rest of the fields.
    let mut run_buf = String::new();
    let mut run_abs_y: u32 = 0;
    let mut run_style: Style = Style::new();
    let mut run_link: Option<&str> = None;
    let mut run_next_col: u32 = 0;
    let mut run_open = false;

    // Helper: flush the currently open run, if any. Emits a single `Print`
    // for the entire accumulated buffer; positioning, style, and OSC 8 were
    // already written when the run opened. Updates `last_cursor` to reflect
    // where the cursor ends up after the Print.
    macro_rules! flush_run {
        ($stdout:expr) => {
            if run_open {
                queue!($stdout, Print(&run_buf))?;
                last_cursor = Some((run_next_col, run_abs_y));
                run_buf.clear();
                run_open = false;
            }
        };
    }

    for y in current.area.y..current.area.bottom() {
        // Issue #171: skip the per-cell scan for rows that were not touched
        // since the last hash refresh AND match the previous frame's
        // digest. Both conditions must hold:
        //   * `row_clean` rules out rows that received writes this frame
        //     even if those writes happened to land on identical cells.
        //   * The hash equality is the actual unchanged-row signal.
        // Falling through to the per-cell loop on either failure preserves
        // legacy behavior; the skip is a pure short-circuit.
        if current.row_clean(y)
            && current.row_hash(y).is_some()
            && current.row_hash(y) == previous.row_hash(y)
        {
            continue;
        }
        for x in current.area.x..current.area.right() {
            let cell = current.get(x, y);
            let prev = previous.get(x, y);
            if cell == prev || cell.symbol.is_empty() {
                // Gap — any open run on this row must be flushed.
                flush_run!(stdout);
                continue;
            }

            let abs_y = row_offset + y;
            // Defense-in-depth: `Cell::hyperlink` is a public field that can
            // be written directly. `set_string_linked` pre-sanitizes, but a
            // direct write could still smuggle control bytes into the OSC 8
            // payload. Validate here before flushing to stdout.
            let cell_link = cell
                .hyperlink
                .as_deref()
                .filter(|u| crate::buffer::is_valid_osc8_url(u));

            // Decide whether this cell extends the open run or starts a new one.
            let extends = run_open
                && run_abs_y == abs_y
                && run_next_col == x
                && run_style == cell.style
                && run_link == cell_link;

            if !extends {
                flush_run!(stdout);

                // Begin a new run. Emit positioning + style + OSC 8 header now
                // (before the Print bytes) so the resulting stream is a valid
                // SGR sequence exactly matching the per-cell flush.
                has_updates = true;

                let need_move = last_cursor.map_or(true, |(lx, ly)| lx != x || ly != abs_y);
                if need_move {
                    queue!(stdout, cursor::MoveTo(sat_u16(x), sat_u16(abs_y)))?;
                }

                if cell.style != last_style {
                    if first_style {
                        queue!(stdout, ResetColor, SetAttribute(Attribute::Reset))?;
                        apply_style(stdout, &cell.style, color_depth)?;
                        first_style = false;
                    } else {
                        apply_style_delta(stdout, &last_style, &cell.style, color_depth)?;
                    }
                    last_style = cell.style;
                }

                if cell_link != active_link {
                    if let Some(url) = cell_link {
                        queue!(stdout, Print(format!("\x1b]8;;{url}\x07")))?;
                    } else {
                        queue!(stdout, Print("\x1b]8;;\x07"))?;
                    }
                    active_link = cell_link;
                }

                run_open = true;
                run_abs_y = abs_y;
                run_style = cell.style;
                run_link = cell_link;
            }

            // Append the cell's grapheme cluster (possibly multi-char when it
            // carries combining marks). Wide chars advance by their column
            // width so subsequent cells line up.
            run_buf.push_str(&cell.symbol);
            let char_width = UnicodeWidthStr::width(cell.symbol.as_str()).max(1) as u32;
            if char_width > 1 && cell.symbol.chars().any(|c| c == '\u{FE0F}') {
                // Emoji variation selector — terminal renders 2 cols but the
                // glyph often measures as 1; pad so the cursor ends up where
                // the next cell is drawn.
                run_buf.push(' ');
            }
            run_next_col = x + char_width;
        }

        // End of row: flush whatever is buffered before moving to the next row.
        flush_run!(stdout);
    }

    if has_updates {
        if active_link.is_some() {
            queue!(stdout, Print("\x1b]8;;\x07"))?;
        }
        queue!(stdout, ResetColor, SetAttribute(Attribute::Reset))?;
    }

    Ok(())
}

/// Benchmark-only entry point for the per-frame buffer flush.
///
/// Exposed so criterion benches under `benches/` (an external crate) can
/// measure the stdout-emit cost of the per-frame flush against a hermetic
/// `Vec<u8>` (or any `Write`) sink, without constructing a real terminal.
///
/// Not part of the stable API. Do not depend on this in application code —
/// prefer the real terminal backend ([`crate::run`]) or
/// [`TestBackend`](crate::TestBackend).
#[doc(hidden)]
pub fn __bench_flush_buffer_diff<W: Write>(
    w: &mut W,
    current: &Buffer,
    previous: &Buffer,
    color_depth: ColorDepth,
) -> io::Result<()> {
    flush_buffer_diff(w, current, previous, color_depth, 0)
}

/// Mutable-buffer variant of [`__bench_flush_buffer_diff`] (issue #171).
///
/// Refreshes per-row digests on both buffers before invoking
/// `flush_buffer_diff`, matching what the real `Terminal::flush` and
/// `InlineTerminal::flush` paths do. Benches that want to measure the
/// flush including the hash-refresh cost should use this entry point;
/// the immutable variant is preserved for backwards compatibility with
/// existing benches that own only `&Buffer`.
#[doc(hidden)]
pub fn __bench_flush_buffer_diff_mut<W: Write>(
    w: &mut W,
    current: &mut Buffer,
    previous: &mut Buffer,
    color_depth: ColorDepth,
) -> io::Result<()> {
    current.recompute_line_hashes();
    previous.recompute_line_hashes();
    flush_buffer_diff(w, current, previous, color_depth, 0)
}

/// Opaque test fixture wrapping `KittyImageManager` + a placements list.
///
/// Returned by [`__bench_new_kitty_fixture`]. Internal types stay
/// `pub(crate)` — only the opaque struct crosses the crate boundary.
#[doc(hidden)]
pub struct __BenchKittyFixture {
    mgr: KittyImageManager,
    placements: Vec<KittyPlacement>,
}

/// Build a self-contained kitty-flush fixture for the perf alloc suite
/// (issue #206). `n` is the number of distinct images.
#[doc(hidden)]
pub fn __bench_new_kitty_fixture(n: usize) -> __BenchKittyFixture {
    let mut placements = Vec::with_capacity(n);
    for i in 0..n {
        // 8x8 RGBA: 64 px * 4 bytes = 256 bytes.
        let mut rgba = vec![0u8; 256];
        // Vary contents per placement to give each a unique content_hash.
        rgba[0] = i as u8;
        let content_hash = crate::buffer::hash_rgba(&rgba);
        placements.push(KittyPlacement {
            content_hash,
            rgba: std::sync::Arc::new(rgba),
            src_width: 8,
            src_height: 8,
            x: (i as u32) * 4,
            y: (i as u32) * 2,
            cols: 4,
            rows: 2,
            crop_y: 0,
            crop_h: 0,
        });
    }
    __BenchKittyFixture {
        mgr: KittyImageManager::new(),
        placements,
    }
}

impl __BenchKittyFixture {
    /// Strong-count snapshot of the inner `Arc<Vec<u8>>` for each placement.
    /// Used by the alloc-budget tests to confirm no extra Arc clones leak
    /// past the manager's stored `prev_placements`.
    #[doc(hidden)]
    pub fn rgba_strong_counts(&self) -> Vec<usize> {
        self.placements
            .iter()
            .map(|p| std::sync::Arc::strong_count(&p.rgba))
            .collect()
    }

    /// Run the inline-mode flush path with the given row offset. Writes
    /// terminal escapes into `sink` and updates the internal manager state.
    #[doc(hidden)]
    pub fn flush_inline<W: Write>(&mut self, sink: &mut W, row_offset: u32) -> io::Result<()> {
        self.mgr.flush(sink, &self.placements, row_offset)
    }

    /// Number of placements in this fixture.
    #[doc(hidden)]
    pub fn len(&self) -> usize {
        self.placements.len()
    }

    /// Whether this fixture has zero placements.
    #[doc(hidden)]
    pub fn is_empty(&self) -> bool {
        self.placements.is_empty()
    }
}

fn flush_raw_sequences(
    stdout: &mut impl Write,
    current: &Buffer,
    previous: &Buffer,
    row_offset: u32,
) -> io::Result<()> {
    if current.raw_sequences == previous.raw_sequences {
        return Ok(());
    }

    for (x, y, seq) in &current.raw_sequences {
        queue!(
            stdout,
            cursor::MoveTo(sat_u16(*x), sat_u16(row_offset + *y)),
            Print(seq)
        )?;
    }

    Ok(())
}

fn flush_cursor(
    stdout: &mut impl Write,
    cursor_visible: &mut bool,
    cursor_pos: Option<(u32, u32)>,
    row_offset: u32,
    fallback_row: Option<u32>,
) -> io::Result<()> {
    match cursor_pos {
        Some((cx, cy)) => {
            if !*cursor_visible {
                queue!(stdout, cursor::Show)?;
                *cursor_visible = true;
            }
            queue!(
                stdout,
                cursor::MoveTo(sat_u16(cx), sat_u16(row_offset + cy))
            )?;
        }
        None => {
            if *cursor_visible {
                queue!(stdout, cursor::Hide)?;
                *cursor_visible = false;
            }
            if let Some(row) = fallback_row {
                queue!(stdout, cursor::MoveTo(0, sat_u16(row)))?;
            }
        }
    }

    Ok(())
}

fn apply_style_delta(
    w: &mut impl Write,
    old: &Style,
    new: &Style,
    depth: ColorDepth,
) -> io::Result<()> {
    if old.fg != new.fg {
        match new.fg {
            Some(fg) => queue!(w, SetForegroundColor(to_crossterm_color(fg, depth)))?,
            None => queue!(w, SetForegroundColor(CtColor::Reset))?,
        }
    }
    if old.bg != new.bg {
        match new.bg {
            Some(bg) => queue!(w, SetBackgroundColor(to_crossterm_color(bg, depth)))?,
            None => queue!(w, SetBackgroundColor(CtColor::Reset))?,
        }
    }
    let removed = Modifiers(old.modifiers.0 & !new.modifiers.0);
    let added = Modifiers(new.modifiers.0 & !old.modifiers.0);
    if removed.contains(Modifiers::BOLD) || removed.contains(Modifiers::DIM) {
        queue!(w, SetAttribute(Attribute::NormalIntensity))?;
        if new.modifiers.contains(Modifiers::BOLD) {
            queue!(w, SetAttribute(Attribute::Bold))?;
        }
        if new.modifiers.contains(Modifiers::DIM) {
            queue!(w, SetAttribute(Attribute::Dim))?;
        }
    } else {
        if added.contains(Modifiers::BOLD) {
            queue!(w, SetAttribute(Attribute::Bold))?;
        }
        if added.contains(Modifiers::DIM) {
            queue!(w, SetAttribute(Attribute::Dim))?;
        }
    }
    if removed.contains(Modifiers::ITALIC) {
        queue!(w, SetAttribute(Attribute::NoItalic))?;
    }
    if added.contains(Modifiers::ITALIC) {
        queue!(w, SetAttribute(Attribute::Italic))?;
    }
    if removed.contains(Modifiers::UNDERLINE) {
        queue!(w, SetAttribute(Attribute::NoUnderline))?;
    }
    if added.contains(Modifiers::UNDERLINE) {
        queue!(w, SetAttribute(Attribute::Underlined))?;
    }
    if removed.contains(Modifiers::REVERSED) {
        queue!(w, SetAttribute(Attribute::NoReverse))?;
    }
    if added.contains(Modifiers::REVERSED) {
        queue!(w, SetAttribute(Attribute::Reverse))?;
    }
    if removed.contains(Modifiers::STRIKETHROUGH) {
        queue!(w, SetAttribute(Attribute::NotCrossedOut))?;
    }
    if added.contains(Modifiers::STRIKETHROUGH) {
        queue!(w, SetAttribute(Attribute::CrossedOut))?;
    }
    Ok(())
}

fn apply_style(w: &mut impl Write, style: &Style, depth: ColorDepth) -> io::Result<()> {
    if let Some(fg) = style.fg {
        queue!(w, SetForegroundColor(to_crossterm_color(fg, depth)))?;
    }
    if let Some(bg) = style.bg {
        queue!(w, SetBackgroundColor(to_crossterm_color(bg, depth)))?;
    }
    let m = style.modifiers;
    if m.contains(Modifiers::BOLD) {
        queue!(w, SetAttribute(Attribute::Bold))?;
    }
    if m.contains(Modifiers::DIM) {
        queue!(w, SetAttribute(Attribute::Dim))?;
    }
    if m.contains(Modifiers::ITALIC) {
        queue!(w, SetAttribute(Attribute::Italic))?;
    }
    if m.contains(Modifiers::UNDERLINE) {
        queue!(w, SetAttribute(Attribute::Underlined))?;
    }
    if m.contains(Modifiers::REVERSED) {
        queue!(w, SetAttribute(Attribute::Reverse))?;
    }
    if m.contains(Modifiers::STRIKETHROUGH) {
        queue!(w, SetAttribute(Attribute::CrossedOut))?;
    }
    Ok(())
}

fn to_crossterm_color(color: Color, depth: ColorDepth) -> CtColor {
    let color = color.downsampled(depth);
    match color {
        Color::Reset => CtColor::Reset,
        Color::Black => CtColor::Black,
        Color::Red => CtColor::DarkRed,
        Color::Green => CtColor::DarkGreen,
        Color::Yellow => CtColor::DarkYellow,
        Color::Blue => CtColor::DarkBlue,
        Color::Magenta => CtColor::DarkMagenta,
        Color::Cyan => CtColor::DarkCyan,
        Color::White => CtColor::White,
        Color::DarkGray => CtColor::DarkGrey,
        Color::LightRed => CtColor::Red,
        Color::LightGreen => CtColor::Green,
        Color::LightYellow => CtColor::Yellow,
        Color::LightBlue => CtColor::Blue,
        Color::LightMagenta => CtColor::Magenta,
        Color::LightCyan => CtColor::Cyan,
        Color::LightWhite => CtColor::White,
        Color::Rgb(r, g, b) => CtColor::Rgb { r, g, b },
        Color::Indexed(i) => CtColor::AnsiValue(i),
    }
}

fn reset_current_buffer(buffer: &mut Buffer, theme_bg: Option<Color>) {
    if let Some(bg) = theme_bg {
        buffer.reset_with_bg(bg);
    } else {
        buffer.reset();
    }
}

fn write_session_enter(stdout: &mut impl Write, session: &TerminalSessionGuard) -> io::Result<()> {
    match session.mode {
        TerminalSessionMode::Fullscreen => {
            execute!(
                stdout,
                terminal::EnterAlternateScreen,
                cursor::Hide,
                EnableBracketedPaste
            )?;
        }
        TerminalSessionMode::Inline => {
            execute!(stdout, cursor::Hide, EnableBracketedPaste)?;
        }
    }

    // Focus-change reporting is independent of mouse capture — callers
    // routinely pause animations or clear hover state on focus loss even
    // without mouse support. Enabling it unconditionally matches modern
    // TUI conventions (zellij, helix, yazi) and the cost is one extra SGR
    // per session.
    execute!(stdout, EnableFocusChange)?;
    if session.mouse_enabled {
        execute!(stdout, EnableMouseCapture)?;
    }
    if session.kitty_keyboard {
        use crossterm::event::PushKeyboardEnhancementFlags;
        let _ = execute!(
            stdout,
            PushKeyboardEnhancementFlags(kitty_flags(session.report_all_keys))
        );
    }

    Ok(())
}

/// Assemble the Kitty keyboard enhancement flags to push.
///
/// Always sets `DISAMBIGUATE_ESCAPE_CODES | REPORT_EVENT_TYPES`. When
/// `report_all_keys` is `true`, also OR-es in
/// `REPORT_ALL_KEYS_AS_ESCAPE_CODES`, which is the only mechanism by which a
/// spec-compliant terminal emits a bare modifier as a key event.
///
/// This is a pure helper so the flag assembly can be unit-tested without
/// touching stdout.
fn kitty_flags(report_all_keys: bool) -> crossterm::event::KeyboardEnhancementFlags {
    use crossterm::event::KeyboardEnhancementFlags;
    let mut flags = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        | KeyboardEnhancementFlags::REPORT_EVENT_TYPES;
    if report_all_keys {
        flags |= KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES;
    }
    flags
}

fn write_session_cleanup(
    stdout: &mut impl Write,
    mode: TerminalSessionMode,
    inline_reserved: bool,
) -> io::Result<()> {
    execute!(
        stdout,
        ResetColor,
        SetAttribute(Attribute::Reset),
        cursor::Show,
        DisableBracketedPaste
    )?;

    match mode {
        TerminalSessionMode::Fullscreen => {
            execute!(stdout, terminal::LeaveAlternateScreen)?;
        }
        TerminalSessionMode::Inline => {
            if inline_reserved {
                execute!(
                    stdout,
                    cursor::MoveToColumn(0),
                    cursor::MoveDown(1),
                    cursor::MoveToColumn(0),
                    Print("\n")
                )?;
            } else {
                execute!(stdout, Print("\n"))?;
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Unix job-control suspend/resume (Ctrl+Z / `fg`) — issue #263
// ---------------------------------------------------------------------------
//
// On Unix, SIGTSTP stops the process in kernel space with no Rust code on the
// stack, so neither `Drop` nor the panic hook can restore the terminal. The
// run loops install a `signal-hook` background thread that, on SIGTSTP, runs
// the same teardown the session guard would (`disable_raw_mode`, leave alt
// screen, show cursor, disable paste/focus/mouse/kitty) and then re-raises
// SIGTSTP to genuinely stop; on SIGCONT it re-enters the session and flags a
// full redraw. The whole feature is `#[cfg(unix)]` and uses only signal-hook's
// safe API, preserving `#![forbid(unsafe_code)]`.

/// Immutable snapshot of the active terminal session used by the unix
/// suspend/resume handler to restore and re-enter the terminal across a
/// Ctrl+Z / `fg` cycle without owning the `Terminal`/`InlineTerminal`.
#[cfg(unix)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct SessionSnapshot {
    mode: TerminalSessionMode,
    mouse_enabled: bool,
    kitty_keyboard: bool,
    report_all_keys: bool,
}

/// Set by the SIGCONT handler and consumed once at the top of each run-loop
/// iteration to force a full clear + repaint after resuming from suspend.
#[cfg(unix)]
pub(crate) static NEEDS_FULL_REDRAW: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[cfg(unix)]
impl Terminal {
    /// Capture the session state the suspend/resume handler needs to restore
    /// and re-enter this fullscreen terminal across Ctrl+Z / `fg`.
    pub(crate) fn session_snapshot(&self) -> SessionSnapshot {
        SessionSnapshot {
            mode: self.session.mode,
            mouse_enabled: self.session.mouse_enabled,
            kitty_keyboard: self.session.kitty_keyboard,
            report_all_keys: self.session.report_all_keys,
        }
    }
}

#[cfg(unix)]
impl InlineTerminal {
    /// Capture the session state the suspend/resume handler needs to restore
    /// and re-enter this inline terminal across Ctrl+Z / `fg`.
    pub(crate) fn session_snapshot(&self) -> SessionSnapshot {
        SessionSnapshot {
            mode: self.session.mode,
            mouse_enabled: self.session.mouse_enabled,
            kitty_keyboard: self.session.kitty_keyboard,
            report_all_keys: self.session.report_all_keys,
        }
    }
}

/// Write the escape sequences that tear down the TUI session in preparation
/// for SIGTSTP (the inverse of [`write_session_enter`]).
///
/// `inline_reserved` is passed `false` to [`write_session_cleanup`] to avoid
/// emitting the inline trailing-newline dance mid-session; the reserved region
/// is repainted on resume via the forced full redraw. Pure byte output, no
/// raw-mode toggle — split out so it can be unit-tested against a `Vec<u8>`.
#[cfg(unix)]
fn write_suspend_sequence(stdout: &mut impl Write, snapshot: &SessionSnapshot) -> io::Result<()> {
    if snapshot.kitty_keyboard {
        use crossterm::event::PopKeyboardEnhancementFlags;
        execute!(stdout, PopKeyboardEnhancementFlags)?;
    }
    if snapshot.mouse_enabled {
        execute!(stdout, DisableMouseCapture)?;
    }
    execute!(stdout, DisableFocusChange)?;
    write_session_cleanup(stdout, snapshot.mode, false)
}

/// Restore the terminal to cooked/non-TUI state in preparation for the process
/// being stopped by SIGTSTP.
///
/// Mirrors [`TerminalSessionGuard::restore`] but writes directly to
/// `io::stdout()` (the handler runs on a background thread that does not own
/// the buffered terminal stdout).
#[cfg(unix)]
pub(crate) fn suspend_to_shell(snapshot: &SessionSnapshot) {
    let mut out = io::stdout();
    let _ = write_suspend_sequence(&mut out, snapshot);
    let _ = terminal::disable_raw_mode();
    let _ = out.flush();
}

/// Re-enter the TUI session after a SIGCONT (resume via `fg`), matching the
/// original [`SessionSnapshot`], and flag a full redraw for the next frame.
///
/// Mirrors [`TerminalSessionGuard::enter`] but writes directly to
/// `io::stdout()`. Sets [`NEEDS_FULL_REDRAW`] so the next loop iteration clears
/// the front buffer and repaints every cell.
#[cfg(unix)]
pub(crate) fn resume_from_shell(snapshot: &SessionSnapshot) {
    let mut out = io::stdout();
    let _ = terminal::enable_raw_mode();
    let guard = TerminalSessionGuard {
        mode: snapshot.mode,
        mouse_enabled: snapshot.mouse_enabled,
        kitty_keyboard: snapshot.kitty_keyboard,
        report_all_keys: snapshot.report_all_keys,
    };
    let _ = write_session_enter(&mut out, &guard);
    let _ = out.flush();
    NEEDS_FULL_REDRAW.store(true, std::sync::atomic::Ordering::SeqCst);
}

/// Construct a [`SessionSnapshot`] for tests without a live terminal.
#[cfg(all(unix, test))]
fn test_snapshot(mode: TerminalSessionMode, mouse: bool, kitty: bool) -> SessionSnapshot {
    SessionSnapshot {
        mode,
        mouse_enabled: mouse,
        kitty_keyboard: kitty,
        report_all_keys: false,
    }
}

/// Construct a fullscreen [`SessionSnapshot`] for crate-level tests that drive
/// the suspend handler without a live terminal (issue #263).
#[cfg(all(unix, test))]
pub(crate) fn test_session_snapshot() -> SessionSnapshot {
    SessionSnapshot {
        mode: TerminalSessionMode::Fullscreen,
        mouse_enabled: false,
        kitty_keyboard: false,
        report_all_keys: false,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn reset_current_buffer_applies_theme_background() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 2, 1));

        reset_current_buffer(&mut buffer, Some(Color::Rgb(10, 20, 30)));
        assert_eq!(buffer.get(0, 0).style.bg, Some(Color::Rgb(10, 20, 30)));

        reset_current_buffer(&mut buffer, None);
        assert_eq!(buffer.get(0, 0).style.bg, None);
    }

    #[test]
    fn fullscreen_session_enter_writes_alt_screen_sequence() {
        let session = TerminalSessionGuard {
            mode: TerminalSessionMode::Fullscreen,
            mouse_enabled: false,
            kitty_keyboard: false,
            report_all_keys: false,
        };
        let mut out = Vec::new();
        write_session_enter(&mut out, &session).unwrap();
        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("\u{1b}[?1049h"));
        assert!(output.contains("\u{1b}[?25l"));
        assert!(output.contains("\u{1b}[?2004h"));
    }

    #[test]
    fn inline_session_enter_skips_alt_screen_sequence() {
        let session = TerminalSessionGuard {
            mode: TerminalSessionMode::Inline,
            mouse_enabled: false,
            kitty_keyboard: false,
            report_all_keys: false,
        };
        let mut out = Vec::new();
        write_session_enter(&mut out, &session).unwrap();
        let output = String::from_utf8(out).unwrap();
        assert!(!output.contains("\u{1b}[?1049h"));
        assert!(output.contains("\u{1b}[?25l"));
        assert!(output.contains("\u{1b}[?2004h"));
    }

    #[test]
    fn fullscreen_session_cleanup_leaves_alt_screen() {
        let mut out = Vec::new();
        write_session_cleanup(&mut out, TerminalSessionMode::Fullscreen, false).unwrap();
        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("\u{1b}[?1049l"));
        assert!(output.contains("\u{1b}[?25h"));
        assert!(output.contains("\u{1b}[?2004l"));
    }

    #[test]
    fn inline_session_cleanup_keeps_normal_screen() {
        let mut out = Vec::new();
        write_session_cleanup(&mut out, TerminalSessionMode::Inline, false).unwrap();
        let output = String::from_utf8(out).unwrap();
        assert!(!output.contains("\u{1b}[?1049l"));
        assert!(output.ends_with('\n'));
        assert!(output.contains("\u{1b}[?25h"));
        assert!(output.contains("\u{1b}[?2004l"));
    }

    // ── Unix suspend/resume sequence tests (issue #263) ──────────────────

    #[cfg(unix)]
    #[test]
    fn suspend_sequence_fullscreen_leaves_alt_screen() {
        let snapshot = test_snapshot(TerminalSessionMode::Fullscreen, false, false);
        let mut out = Vec::new();
        write_suspend_sequence(&mut out, &snapshot).unwrap();
        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("\u{1b}[?1049l"), "leaves alt screen");
        assert!(output.contains("\u{1b}[?25h"), "shows cursor");
        assert!(output.contains("\u{1b}[?2004l"), "disables bracketed paste");
    }

    #[cfg(unix)]
    #[test]
    fn suspend_sequence_inline_keeps_normal_screen() {
        let snapshot = test_snapshot(TerminalSessionMode::Inline, false, false);
        let mut out = Vec::new();
        write_suspend_sequence(&mut out, &snapshot).unwrap();
        let output = String::from_utf8(out).unwrap();
        assert!(
            !output.contains("\u{1b}[?1049l"),
            "inline must not leave alt screen"
        );
        assert!(output.contains("\u{1b}[?25h"), "shows cursor");
        assert!(output.contains("\u{1b}[?2004l"), "disables bracketed paste");
    }

    #[cfg(unix)]
    #[test]
    fn suspend_sequence_disables_mouse_and_kitty_when_enabled() {
        let snapshot = test_snapshot(TerminalSessionMode::Fullscreen, true, true);
        let mut out = Vec::new();
        write_suspend_sequence(&mut out, &snapshot).unwrap();
        // DisableMouseCapture emits the SGR-mouse disable (?1006l) among others.
        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("\u{1b}[?1006l"), "disables SGR mouse mode");
    }

    #[cfg(unix)]
    #[test]
    fn resume_sequence_fullscreen_round_trips_enter_and_flags_redraw() {
        let snapshot = test_snapshot(TerminalSessionMode::Fullscreen, false, false);

        // The resume path re-enters the same byte state as the initial enter.
        let guard = TerminalSessionGuard {
            mode: snapshot.mode,
            mouse_enabled: snapshot.mouse_enabled,
            kitty_keyboard: snapshot.kitty_keyboard,
        };
        let mut enter_bytes = Vec::new();
        write_session_enter(&mut enter_bytes, &guard).unwrap();
        let enter = String::from_utf8(enter_bytes).unwrap();
        assert!(enter.contains("\u{1b}[?1049h"));
        assert!(enter.contains("\u{1b}[?25l"));
        assert!(enter.contains("\u{1b}[?2004h"));

        // Drive the public resume entry point and assert the redraw flag flips.
        NEEDS_FULL_REDRAW.store(false, std::sync::atomic::Ordering::SeqCst);
        resume_from_shell(&snapshot);
        assert!(
            NEEDS_FULL_REDRAW.swap(false, std::sync::atomic::Ordering::SeqCst),
            "resume must request a full redraw exactly once"
        );
        assert!(
            !NEEDS_FULL_REDRAW.swap(false, std::sync::atomic::Ordering::SeqCst),
            "the redraw flag is consumed by the first swap (idempotent)"
        );
    }

    #[cfg(unix)]
    #[test]
    fn needs_full_redraw_swaps_true_once() {
        NEEDS_FULL_REDRAW.store(true, std::sync::atomic::Ordering::SeqCst);
        assert!(NEEDS_FULL_REDRAW.swap(false, std::sync::atomic::Ordering::SeqCst));
        assert!(!NEEDS_FULL_REDRAW.swap(false, std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn kitty_flags_base_set_excludes_report_all_keys() {
        use crossterm::event::KeyboardEnhancementFlags;
        let flags = kitty_flags(false);
        assert!(flags.contains(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES));
        assert!(flags.contains(KeyboardEnhancementFlags::REPORT_EVENT_TYPES));
        assert!(!flags.contains(KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES));
    }

    #[test]
    fn kitty_flags_report_all_keys_sets_flag() {
        use crossterm::event::KeyboardEnhancementFlags;
        let flags = kitty_flags(true);
        assert!(flags.contains(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES));
        assert!(flags.contains(KeyboardEnhancementFlags::REPORT_EVENT_TYPES));
        assert!(flags.contains(KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES));
    }

    #[test]
    fn base64_encode_empty() {
        assert_eq!(base64_encode(b""), "");
    }

    #[test]
    fn base64_encode_hello() {
        assert_eq!(base64_encode(b"Hello"), "SGVsbG8=");
    }

    #[test]
    fn base64_encode_padding() {
        assert_eq!(base64_encode(b"a"), "YQ==");
        assert_eq!(base64_encode(b"ab"), "YWI=");
        assert_eq!(base64_encode(b"abc"), "YWJj");
    }

    #[test]
    fn base64_encode_unicode() {
        assert_eq!(base64_encode("한글".as_bytes()), "7ZWc6riA");
    }

    #[cfg(feature = "crossterm")]
    #[test]
    fn parse_osc11_response_dark_and_light() {
        assert_eq!(
            parse_osc11_response("\x1b]11;rgb:0000/0000/0000\x1b\\"),
            ColorScheme::Dark
        );
        assert_eq!(
            parse_osc11_response("\x1b]11;rgb:ffff/ffff/ffff\x07"),
            ColorScheme::Light
        );
    }

    #[cfg(feature = "crossterm")]
    #[test]
    fn base64_decode_round_trip_hello() {
        let encoded = base64_encode("hello".as_bytes());
        assert_eq!(base64_decode(&encoded), Some("hello".to_string()));
    }

    #[cfg(feature = "crossterm")]
    #[test]
    fn color_scheme_equality() {
        assert_eq!(ColorScheme::Dark, ColorScheme::Dark);
        assert_ne!(ColorScheme::Dark, ColorScheme::Light);
        assert_eq!(ColorScheme::Unknown, ColorScheme::Unknown);
    }

    fn pair(r: Rect) -> (Rect, Rect) {
        (r, r)
    }

    #[test]
    fn find_innermost_rect_picks_smallest() {
        let rects = vec![
            pair(Rect::new(0, 0, 80, 24)),
            pair(Rect::new(5, 2, 30, 10)),
            pair(Rect::new(10, 4, 10, 5)),
        ];
        let result = find_innermost_rect(&rects, 12, 5);
        assert_eq!(result, Some(Rect::new(10, 4, 10, 5)));
    }

    #[test]
    fn find_innermost_rect_no_match() {
        let rects = vec![pair(Rect::new(10, 10, 5, 5))];
        assert_eq!(find_innermost_rect(&rects, 0, 0), None);
    }

    #[test]
    fn find_innermost_rect_empty() {
        assert_eq!(find_innermost_rect(&[], 5, 5), None);
    }

    #[test]
    fn find_innermost_rect_returns_content_rect() {
        let rects = vec![
            (Rect::new(0, 0, 80, 24), Rect::new(1, 1, 78, 22)),
            (Rect::new(5, 2, 30, 10), Rect::new(6, 3, 28, 8)),
        ];
        let result = find_innermost_rect(&rects, 10, 5);
        assert_eq!(result, Some(Rect::new(6, 3, 28, 8)));
    }

    #[test]
    fn normalize_selection_already_ordered() {
        let (s, e) = normalize_selection((2, 1), (5, 3));
        assert_eq!(s, (2, 1));
        assert_eq!(e, (5, 3));
    }

    #[test]
    fn normalize_selection_reversed() {
        let (s, e) = normalize_selection((5, 3), (2, 1));
        assert_eq!(s, (2, 1));
        assert_eq!(e, (5, 3));
    }

    #[test]
    fn normalize_selection_same_row() {
        let (s, e) = normalize_selection((10, 5), (3, 5));
        assert_eq!(s, (3, 5));
        assert_eq!(e, (10, 5));
    }

    #[test]
    fn selection_state_mouse_down_finds_rect() {
        let hit_map = vec![pair(Rect::new(0, 0, 80, 24)), pair(Rect::new(5, 2, 20, 10))];
        let mut sel = SelectionState::default();
        sel.mouse_down(10, 5, &hit_map);
        assert_eq!(sel.anchor, Some((10, 5)));
        assert_eq!(sel.current, Some((10, 5)));
        assert_eq!(sel.widget_rect, Some(Rect::new(5, 2, 20, 10)));
        assert!(!sel.active);
    }

    #[test]
    fn selection_state_drag_activates() {
        let hit_map = vec![pair(Rect::new(0, 0, 80, 24))];
        let mut sel = SelectionState {
            anchor: Some((10, 5)),
            current: Some((10, 5)),
            widget_rect: Some(Rect::new(0, 0, 80, 24)),
            ..Default::default()
        };
        sel.mouse_drag(10, 5, &hit_map);
        assert!(!sel.active, "no movement = not active");
        sel.mouse_drag(11, 5, &hit_map);
        assert!(!sel.active, "1 cell horizontal = not active yet");
        sel.mouse_drag(13, 5, &hit_map);
        assert!(sel.active, ">1 cell horizontal = active");
    }

    #[test]
    fn selection_state_drag_vertical_activates() {
        let hit_map = vec![pair(Rect::new(0, 0, 80, 24))];
        let mut sel = SelectionState {
            anchor: Some((10, 5)),
            current: Some((10, 5)),
            widget_rect: Some(Rect::new(0, 0, 80, 24)),
            ..Default::default()
        };
        sel.mouse_drag(10, 6, &hit_map);
        assert!(sel.active, "any vertical movement = active");
    }

    #[test]
    fn selection_state_drag_expands_widget_rect() {
        let hit_map = vec![
            pair(Rect::new(0, 0, 80, 24)),
            pair(Rect::new(5, 2, 30, 10)),
            pair(Rect::new(5, 2, 30, 3)),
        ];
        let mut sel = SelectionState {
            anchor: Some((10, 3)),
            current: Some((10, 3)),
            widget_rect: Some(Rect::new(5, 2, 30, 3)),
            ..Default::default()
        };
        sel.mouse_drag(10, 6, &hit_map);
        assert_eq!(sel.widget_rect, Some(Rect::new(5, 2, 30, 10)));
    }

    #[test]
    fn selection_state_clear_resets() {
        let mut sel = SelectionState {
            anchor: Some((1, 2)),
            current: Some((3, 4)),
            widget_rect: Some(Rect::new(0, 0, 10, 10)),
            active: true,
        };
        sel.clear();
        assert_eq!(sel.anchor, None);
        assert_eq!(sel.current, None);
        assert_eq!(sel.widget_rect, None);
        assert!(!sel.active);
    }

    #[test]
    fn extract_selection_text_single_line() {
        let area = Rect::new(0, 0, 20, 5);
        let mut buf = Buffer::empty(area);
        buf.set_string(0, 0, "Hello World", Style::default());
        let sel = SelectionState {
            anchor: Some((0, 0)),
            current: Some((4, 0)),
            widget_rect: Some(area),
            active: true,
        };
        let text = extract_selection_text(&buf, &sel, &[]);
        assert_eq!(text, "Hello");
    }

    #[test]
    fn extract_selection_text_multi_line() {
        let area = Rect::new(0, 0, 20, 5);
        let mut buf = Buffer::empty(area);
        buf.set_string(0, 0, "Line one", Style::default());
        buf.set_string(0, 1, "Line two", Style::default());
        buf.set_string(0, 2, "Line three", Style::default());
        let sel = SelectionState {
            anchor: Some((5, 0)),
            current: Some((3, 2)),
            widget_rect: Some(area),
            active: true,
        };
        let text = extract_selection_text(&buf, &sel, &[]);
        assert_eq!(text, "one\nLine two\nLine");
    }

    #[test]
    fn extract_selection_text_clamped_to_widget() {
        let area = Rect::new(0, 0, 40, 10);
        let widget = Rect::new(5, 2, 10, 3);
        let mut buf = Buffer::empty(area);
        buf.set_string(5, 2, "ABCDEFGHIJ", Style::default());
        buf.set_string(5, 3, "KLMNOPQRST", Style::default());
        let sel = SelectionState {
            anchor: Some((3, 1)),
            current: Some((20, 5)),
            widget_rect: Some(widget),
            active: true,
        };
        let text = extract_selection_text(&buf, &sel, &[]);
        assert_eq!(text, "ABCDEFGHIJ\nKLMNOPQRST");
    }

    #[test]
    fn extract_selection_text_inactive_returns_empty() {
        let area = Rect::new(0, 0, 10, 5);
        let buf = Buffer::empty(area);
        let sel = SelectionState {
            anchor: Some((0, 0)),
            current: Some((5, 2)),
            widget_rect: Some(area),
            active: false,
        };
        assert_eq!(extract_selection_text(&buf, &sel, &[]), "");
    }

    #[test]
    fn apply_selection_overlay_reverses_cells() {
        let area = Rect::new(0, 0, 10, 3);
        let mut buf = Buffer::empty(area);
        buf.set_string(0, 0, "ABCDE", Style::default());
        let sel = SelectionState {
            anchor: Some((1, 0)),
            current: Some((3, 0)),
            widget_rect: Some(area),
            active: true,
        };
        apply_selection_overlay(&mut buf, &sel, &[]);
        assert!(!buf.get(0, 0).style.modifiers.contains(Modifiers::REVERSED));
        assert!(buf.get(1, 0).style.modifiers.contains(Modifiers::REVERSED));
        assert!(buf.get(2, 0).style.modifiers.contains(Modifiers::REVERSED));
        assert!(buf.get(3, 0).style.modifiers.contains(Modifiers::REVERSED));
        assert!(!buf.get(4, 0).style.modifiers.contains(Modifiers::REVERSED));
    }

    #[test]
    fn extract_selection_text_skips_border_cells() {
        // Simulate two bordered columns side by side:
        // Col1: full=(0,0,20,5) content=(1,1,18,3)
        // Col2: full=(20,0,20,5) content=(21,1,18,3)
        // Parent widget_rect covers both: (0,0,40,5)
        let area = Rect::new(0, 0, 40, 5);
        let mut buf = Buffer::empty(area);
        // Col1 border characters
        buf.set_string(0, 0, "╭", Style::default());
        buf.set_string(0, 1, "│", Style::default());
        buf.set_string(0, 2, "│", Style::default());
        buf.set_string(0, 3, "│", Style::default());
        buf.set_string(0, 4, "╰", Style::default());
        buf.set_string(19, 0, "╮", Style::default());
        buf.set_string(19, 1, "│", Style::default());
        buf.set_string(19, 2, "│", Style::default());
        buf.set_string(19, 3, "│", Style::default());
        buf.set_string(19, 4, "╯", Style::default());
        // Col2 border characters
        buf.set_string(20, 0, "╭", Style::default());
        buf.set_string(20, 1, "│", Style::default());
        buf.set_string(20, 2, "│", Style::default());
        buf.set_string(20, 3, "│", Style::default());
        buf.set_string(20, 4, "╰", Style::default());
        buf.set_string(39, 0, "╮", Style::default());
        buf.set_string(39, 1, "│", Style::default());
        buf.set_string(39, 2, "│", Style::default());
        buf.set_string(39, 3, "│", Style::default());
        buf.set_string(39, 4, "╯", Style::default());
        // Content inside Col1
        buf.set_string(1, 1, "Hello Col1", Style::default());
        buf.set_string(1, 2, "Line2 Col1", Style::default());
        // Content inside Col2
        buf.set_string(21, 1, "Hello Col2", Style::default());
        buf.set_string(21, 2, "Line2 Col2", Style::default());

        let content_map = vec![
            (Rect::new(0, 0, 20, 5), Rect::new(1, 1, 18, 3)),
            (Rect::new(20, 0, 20, 5), Rect::new(21, 1, 18, 3)),
        ];

        // Select across both columns, rows 1-2
        let sel = SelectionState {
            anchor: Some((0, 1)),
            current: Some((39, 2)),
            widget_rect: Some(area),
            active: true,
        };
        let text = extract_selection_text(&buf, &sel, &content_map);
        // Should NOT contain border characters (│, ╭, ╮, etc.)
        assert!(!text.contains('│'), "Border char │ found in: {text}");
        assert!(!text.contains('╭'), "Border char ╭ found in: {text}");
        assert!(!text.contains('╮'), "Border char ╮ found in: {text}");
        // Should contain actual content
        assert!(
            text.contains("Hello Col1"),
            "Missing Col1 content in: {text}"
        );
        assert!(
            text.contains("Hello Col2"),
            "Missing Col2 content in: {text}"
        );
        assert!(text.contains("Line2 Col1"), "Missing Col1 line2 in: {text}");
        assert!(text.contains("Line2 Col2"), "Missing Col2 line2 in: {text}");
    }

    #[test]
    fn apply_selection_overlay_skips_border_cells() {
        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);
        buf.set_string(0, 0, "│", Style::default());
        buf.set_string(1, 0, "ABC", Style::default());
        buf.set_string(19, 0, "│", Style::default());

        let content_map = vec![(Rect::new(0, 0, 20, 3), Rect::new(1, 0, 18, 3))];
        let sel = SelectionState {
            anchor: Some((0, 0)),
            current: Some((19, 0)),
            widget_rect: Some(area),
            active: true,
        };
        apply_selection_overlay(&mut buf, &sel, &content_map);
        // Border cells at x=0 and x=19 should NOT be reversed
        assert!(
            !buf.get(0, 0).style.modifiers.contains(Modifiers::REVERSED),
            "Left border cell should not be reversed"
        );
        assert!(
            !buf.get(19, 0).style.modifiers.contains(Modifiers::REVERSED),
            "Right border cell should not be reversed"
        );
        // Content cells should be reversed
        assert!(buf.get(1, 0).style.modifiers.contains(Modifiers::REVERSED));
        assert!(buf.get(2, 0).style.modifiers.contains(Modifiers::REVERSED));
        assert!(buf.get(3, 0).style.modifiers.contains(Modifiers::REVERSED));
    }

    #[test]
    fn copy_to_clipboard_writes_osc52() {
        let mut output: Vec<u8> = Vec::new();
        copy_to_clipboard(&mut output, "test").unwrap();
        let s = String::from_utf8(output).unwrap();
        assert!(s.starts_with("\x1b]52;c;"));
        assert!(s.ends_with("\x1b\\"));
        assert!(s.contains(&base64_encode(b"test")));
    }

    // Count occurrences of CSI cursor-move (`ESC [ ... H`) in flush output.
    fn count_move_tos(s: &str) -> usize {
        let bytes = s.as_bytes();
        let mut count = 0;
        let mut i = 0;
        while i + 1 < bytes.len() {
            if bytes[i] == 0x1b && bytes[i + 1] == b'[' {
                // Scan to the terminator — final byte in 0x40..=0x7e.
                let mut j = i + 2;
                while j < bytes.len() && !(0x40..=0x7e).contains(&bytes[j]) {
                    j += 1;
                }
                if j < bytes.len() && bytes[j] == b'H' {
                    count += 1;
                }
                i = j + 1;
            } else {
                i += 1;
            }
        }
        count
    }

    #[test]
    fn flush_coalesces_consecutive_same_style_cells_into_one_run() {
        // 10 cells, identical Style, contiguous columns -> 1 MoveTo + 1 Print.
        let area = Rect::new(0, 0, 20, 1);
        let mut current = Buffer::empty(area);
        let previous = Buffer::empty(area);
        let style = Style::new().fg(Color::Red);
        for x in 0..10u32 {
            let cell = current.get_mut(x, 0);
            cell.set_char('X');
            cell.set_style(style);
        }

        let mut out: Vec<u8> = Vec::new();
        flush_buffer_diff(&mut out, &current, &previous, ColorDepth::TrueColor, 0).unwrap();
        let s = String::from_utf8(out).unwrap();

        // Exactly one cursor move for the whole run.
        assert_eq!(
            count_move_tos(&s),
            1,
            "expected 1 MoveTo for a coalesced run, got {} in {:?}",
            count_move_tos(&s),
            s
        );
        // The 10 glyphs are emitted contiguously as a single run.
        assert!(
            s.contains("XXXXXXXXXX"),
            "expected contiguous run 'XXXXXXXXXX' in {:?}",
            s
        );
    }

    #[test]
    fn flush_breaks_run_on_style_change() {
        // 5 red cells + 5 blue cells in the same row -> 2 MoveTo calls not 10.
        let area = Rect::new(0, 0, 20, 1);
        let mut current = Buffer::empty(area);
        let previous = Buffer::empty(area);
        let red = Style::new().fg(Color::Red);
        let blue = Style::new().fg(Color::Blue);
        for x in 0..5u32 {
            let cell = current.get_mut(x, 0);
            cell.set_char('R');
            cell.set_style(red);
        }
        for x in 5..10u32 {
            let cell = current.get_mut(x, 0);
            cell.set_char('B');
            cell.set_style(blue);
        }

        let mut out: Vec<u8> = Vec::new();
        flush_buffer_diff(&mut out, &current, &previous, ColorDepth::TrueColor, 0).unwrap();
        let s = String::from_utf8(out).unwrap();

        // First run needs a MoveTo; the second run starts exactly where the
        // cursor already is, so `last_cursor` suppresses a redundant MoveTo.
        // Either way, we should see at most 2 MoveTos and far fewer than 10.
        let moves = count_move_tos(&s);
        assert!(
            moves <= 2,
            "expected at most 2 MoveTos across a style boundary, got {} in {:?}",
            moves,
            s
        );
        assert!(s.contains("RRRRR"), "missing 'RRRRR' run in {:?}", s);
        assert!(s.contains("BBBBB"), "missing 'BBBBB' run in {:?}", s);
    }

    #[test]
    fn flush_breaks_run_on_column_gap() {
        // Cells at x=0..3 and x=6..9; gap at x=3,4,5 must split runs.
        let area = Rect::new(0, 0, 20, 1);
        let mut current = Buffer::empty(area);
        let previous = Buffer::empty(area);
        let style = Style::new().fg(Color::Green);
        for x in 0..3u32 {
            current.get_mut(x, 0).set_char('A').set_style(style);
        }
        for x in 6..9u32 {
            current.get_mut(x, 0).set_char('B').set_style(style);
        }

        let mut out: Vec<u8> = Vec::new();
        flush_buffer_diff(&mut out, &current, &previous, ColorDepth::TrueColor, 0).unwrap();
        let s = String::from_utf8(out).unwrap();

        // Two separate runs means two MoveTo commands.
        assert_eq!(
            count_move_tos(&s),
            2,
            "expected 2 MoveTos across a column gap, got {} in {:?}",
            count_move_tos(&s),
            s
        );
        assert!(s.contains("AAA"), "missing 'AAA' run in {:?}", s);
        assert!(s.contains("BBB"), "missing 'BBB' run in {:?}", s);
    }

    /// Verifies that `flush_buffer_diff` produces identical ANSI output whether the
    /// destination is a plain `Vec<u8>` or a `BufWriter<Vec<u8>>`. This ensures the
    /// BufWriter wrapper introduced for stdout does not alter the byte stream.
    #[test]
    fn bufwriter_output_identical_to_direct_write() {
        let area = Rect::new(0, 0, 5, 1);
        let mut current = Buffer::empty(area);
        let previous = Buffer::empty(area);
        let style = Style::new().fg(Color::Rgb(255, 128, 0));
        for x in 0..5u32 {
            current.get_mut(x, 0).set_char('X').set_style(style);
        }

        let mut direct: Vec<u8> = Vec::new();
        flush_buffer_diff(&mut direct, &current, &previous, ColorDepth::TrueColor, 0).unwrap();

        let mut buffered: BufWriter<Vec<u8>> = BufWriter::with_capacity(65536, Vec::new());
        flush_buffer_diff(&mut buffered, &current, &previous, ColorDepth::TrueColor, 0).unwrap();
        buffered.flush().unwrap();
        let via_buf = buffered.into_inner().unwrap();

        assert_eq!(
            direct, via_buf,
            "BufWriter output must be byte-for-byte identical to direct write"
        );
    }

    /// Verifies that a `BufWriter<Vec<u8>>` sink accumulates all writes and only
    /// issues a single underlying `write` call to the inner sink when flushed.
    /// This is a proxy for the syscall-reduction guarantee on the real stdout.
    #[test]
    fn bufwriter_coalesces_writes_into_single_flush() {
        #[derive(Debug)]
        struct CountingWriter {
            buf: Vec<u8>,
            write_call_count: usize,
        }
        impl Write for CountingWriter {
            fn write(&mut self, data: &[u8]) -> io::Result<usize> {
                self.write_call_count += 1;
                self.buf.extend_from_slice(data);
                Ok(data.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let area = Rect::new(0, 0, 10, 1);
        let mut current = Buffer::empty(area);
        let previous = Buffer::empty(area);
        // Alternate styles on every cell to maximise queue! calls inside flush_buffer_diff.
        for x in 0..10u32 {
            let color = if x % 2 == 0 {
                Color::Rgb(255, 0, 0)
            } else {
                Color::Rgb(0, 255, 0)
            };
            current
                .get_mut(x, 0)
                .set_char('Z')
                .set_style(Style::new().fg(color));
        }

        let sink = CountingWriter {
            buf: Vec::new(),
            write_call_count: 0,
        };
        let mut bw = BufWriter::with_capacity(65536, sink);
        flush_buffer_diff(&mut bw, &current, &previous, ColorDepth::TrueColor, 0).unwrap();
        bw.flush().unwrap();
        let inner = bw.into_inner().unwrap();

        // BufWriter should have batched everything into 1 write call to the sink.
        assert_eq!(
            inner.write_call_count, 1,
            "expected 1 write syscall to sink, got {}",
            inner.write_call_count
        );
    }

    /// Issue #171 regression: identical buffers must produce no flush
    /// output once both have refreshed line hashes. Validates that the
    /// per-row skip path is correctness-preserving — a skipped row
    /// emits zero bytes, exactly like the per-cell path would for an
    /// unchanged row.
    #[test]
    fn flush_skips_unchanged_rows_when_hashes_match() {
        let area = Rect::new(0, 0, 20, 4);
        let mut current = Buffer::empty(area);
        let mut previous = Buffer::empty(area);
        // Populate both buffers with identical content.
        for y in 0..4u32 {
            current.set_string(0, y, "identical-row-content", Style::new());
            previous.set_string(0, y, "identical-row-content", Style::new());
        }
        current.recompute_line_hashes();
        previous.recompute_line_hashes();

        let mut out: Vec<u8> = Vec::new();
        flush_buffer_diff(&mut out, &current, &previous, ColorDepth::TrueColor, 0).unwrap();
        assert!(
            out.is_empty(),
            "identical buffers must emit zero flush bytes; got {} bytes: {:?}",
            out.len(),
            out
        );
    }

    /// Issue #171 regression: when only some rows match, only those rows
    /// are skipped. The differing row must still drive its full per-cell
    /// flush path so the terminal sees the correct glyphs.
    #[test]
    fn flush_skips_only_matching_rows_in_mixed_diff() {
        let area = Rect::new(0, 0, 6, 3);
        let mut current = Buffer::empty(area);
        let mut previous = Buffer::empty(area);
        current.set_string(0, 0, "abcdef", Style::new());
        previous.set_string(0, 0, "abcdef", Style::new());
        current.set_string(0, 1, "xxxxxx", Style::new());
        previous.set_string(0, 1, "yyyyyy", Style::new());
        current.set_string(0, 2, "zzzzzz", Style::new());
        previous.set_string(0, 2, "zzzzzz", Style::new());
        current.recompute_line_hashes();
        previous.recompute_line_hashes();

        let mut out: Vec<u8> = Vec::new();
        flush_buffer_diff(&mut out, &current, &previous, ColorDepth::TrueColor, 0).unwrap();
        let s = String::from_utf8_lossy(&out);
        // The mismatched row's new content must appear; matching rows'
        // glyphs must not (they share content with `previous`).
        assert!(s.contains("xxxxxx"), "differing row must flush: {s:?}");
        assert!(
            !s.contains("abcdef"),
            "matching row 0 must not flush: {s:?}"
        );
        assert!(
            !s.contains("zzzzzz"),
            "matching row 2 must not flush: {s:?}"
        );
    }
}
