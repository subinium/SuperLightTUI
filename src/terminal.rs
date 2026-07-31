use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{self, BufWriter, IsTerminal, Read, Stdout, Write};
use std::time::{Duration, Instant};

use crossterm::event::{
    DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
    EnableFocusChange, EnableMouseCapture,
};
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
    /// Reused dedup scratch for already-deleted image IDs in `flush`. Typical
    /// placement counts are 0–8 (well below where a `HashSet` beats a linear /
    /// sorted scan), so a `SmallVec` stays on the stack and carries its
    /// capacity across frames — no per-frame heap allocation, no SipHash.
    scratch_ids: smallvec::SmallVec<[u32; 8]>,
    /// Reused scratch for content hashes still referenced this frame, used to
    /// prune stale uploads. Sorted in place for `binary_search` membership.
    scratch_hashes: smallvec::SmallVec<[u64; 8]>,
}

impl KittyImageManager {
    /// Construct a new image manager with no uploaded images.
    pub(crate) fn new() -> Self {
        Self {
            next_id: 1,
            uploaded: HashMap::new(),
            prev_placements: Vec::new(),
            scratch_ids: smallvec::SmallVec::new(),
            scratch_hashes: smallvec::SmallVec::new(),
        }
    }

    /// Flush Kitty image placements: upload new images, manage placements.
    ///
    /// `row_offset` shifts `current[i].y` for both terminal output and the
    /// diff comparison against `prev_placements`. Stored placements always
    /// include the offset (the displayed `y`) so re-emit detection works
    /// across resize even when the offset itself changes (issue #206).
    pub(crate) fn flush(
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

        // Delete all previous placements (keep uploaded image data for reuse).
        // Dedup via a reused `SmallVec` instead of a per-frame `HashSet`: at the
        // 0–8 image counts this path actually sees, a linear membership scan
        // beats hashing, and the scratch keeps its capacity across frames. The
        // emit order (first-seen) is unchanged, so the byte stream is identical.
        if !self.prev_placements.is_empty() {
            self.scratch_ids.clear();
            for p in &self.prev_placements {
                if let Some(&img_id) = self.uploaded.get(&p.content_hash)
                    && !self.scratch_ids.contains(&img_id)
                {
                    self.scratch_ids.push(img_id);
                    // Delete all placements of this image (but keep image data)
                    queue!(stdout, Print(format!("\x1b_Ga=d,d=i,i={img_id},q=2\x1b\\")))?;
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

        // Clean up images no longer used by any placement. Build the
        // still-referenced hash set into a reused `SmallVec`, sort it, and test
        // membership with `binary_search` instead of a per-frame `HashSet`.
        // (The set of stale uploads is the same regardless of scan order; the
        // delete emission was already unordered via `HashMap` key iteration.)
        self.scratch_hashes.clear();
        self.scratch_hashes
            .extend(current.iter().map(|p| p.content_hash));
        self.scratch_hashes.sort_unstable();
        let scratch_hashes = &self.scratch_hashes;
        let stale: smallvec::SmallVec<[u64; 8]> = self
            .uploaded
            .keys()
            .filter(|h| scratch_hashes.binary_search(h).is_err())
            .copied()
            .collect();
        for hash in stale {
            if let Some(id) = self.uploaded.remove(&hash) {
                // Delete image data from terminal memory
                queue!(stdout, Print(format!("\x1b_Ga=d,d=I,i={id},q=2\x1b\\")))?;
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
                        "\x1b_Ga=t,i={id},f=32,{compression}s={},v={},q=2,m={more};{chunk}\x1b\\",
                        p.src_width, p.src_height
                    ))
                )?;
            } else {
                queue!(stdout, Print(format!("\x1b_Gm={more};{chunk}\x1b\\")))?;
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
    pub(crate) fn delete_all(&self, stdout: &mut impl Write) -> io::Result<()> {
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
///
/// The payload is returned as a [`Cow`] so the no-compression path (the
/// `kitty-compress` feature off, or compression that fails to save space)
/// **borrows** the caller's slice instead of cloning the full RGBA buffer into
/// a throwaway `Vec` on every `upload_image` call. The compressed path still
/// returns an owned `Vec`. The downstream `base64_encode(&payload)` call sees
/// `&[u8]` via `Deref` in both cases, so no signature change ripples out.
fn compress_rgba(data: &[u8]) -> (Cow<'_, [u8]>, &'static str) {
    #[cfg(feature = "kitty-compress")]
    {
        use flate2::Compression;
        use flate2::write::ZlibEncoder;
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
        if encoder.write_all(data).is_ok()
            && let Ok(compressed) = encoder.finish()
        {
            // Only use compression if it actually saves space
            if compressed.len() < data.len() {
                return (Cow::Owned(compressed), "o=z,");
            }
        }
    }
    (Cow::Borrowed(data), "")
}

/// Query the terminal for the actual cell pixel dimensions via CSI 16 t.
///
/// Returns `(cell_width, cell_height)` in pixels. Falls back to `(8, 16)` if
/// detection fails. Used by `kitty_image_fit` for accurate aspect ratio.
///
/// Cached after first successful detection.
pub(crate) fn cell_pixel_size() -> (u32, u32) {
    use std::sync::OnceLock;
    static CACHED: OnceLock<(u32, u32)> = OnceLock::new();
    if let Some(size) = CACHED.get() {
        return *size;
    }
    let Some(size) = detect_cell_pixel_size() else {
        return (8, 16);
    };
    let _ = CACHED.set(size);
    size
}

fn detect_cell_pixel_size() -> Option<(u32, u32)> {
    if !automatic_terminal_queries_allowed() {
        return None;
    }

    // CSI 16 t → reports cell size as CSI 6 ; height ; width t
    let mut stdout = io::stdout();
    write!(stdout, "\x1b[16t").ok()?;
    stdout.flush().ok()?;

    let response = read_osc_response(Duration::from_millis(100))?;

    // Parse: ESC [ 6 ; <height> ; <width> t
    // Locate the reply anywhere in the buffer rather than anchoring to its
    // start/end: interleaved control bytes — e.g. a pump-retirement nudge
    // answer (`CSI 0 n`) from a previous reply session — may surround it.
    let bytes = response.as_bytes();
    let start = bytes
        .windows(4)
        .position(|w| w == b"\x1b[6;")
        .map(|pos| pos + 4)
        .or_else(|| {
            // CSI can also start with 0x9B (single-byte CSI).
            bytes
                .windows(3)
                .position(|w| w == [0x9b, b'6', b';'])
                .map(|pos| pos + 3)
        })?;
    let tail = response.get(start..)?;
    let body = &tail[..tail.find('t')?];
    let mut parts = body.split(';');
    let ch: u32 = parts.next()?.parse().ok()?;
    let cw: u32 = parts.next()?.parse().ok()?;
    if cw > 0 && ch > 0 {
        Some((cw, ch))
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Runtime terminal capability probe (issue #264)
// ---------------------------------------------------------------------------
//
// Historically SLT decided whether a terminal could render images / accept the
// Kitty keyboard protocol / do truecolor *purely from environment-variable
// allowlists*, which silently degraded capable modern terminals (WezTerm,
// Ghostty) to an error string. This block adds a one-shot DA1/DA2/XTGETTCAP
// probe at session enter, parses the replies into a read-only [`Capabilities`]
// snapshot, and drives an automatic blitter ladder so app code never has to
// branch on terminal identity. The data types are always compiled (so the
// `Context` field exists on every build); only the runtime probe is
// `crossterm`-gated.

/// Image-rendering primitives the terminal can drive, used to build the
/// automatic blitter ladder. Each flag is conservative: when the runtime probe
/// returns no answer the defaults assume only the universally available
/// primitives (half-block + quadrants).
///
/// App code is **not** required to inspect this; it exists for diagnostics and
/// to feed [`Capabilities::best_blitter`].
///
/// # Example
///
/// ```no_run
/// # slt::run(|ui: &mut slt::Context| {
/// let blitters = ui.capabilities().blitters;
/// // Half-block is available on any ANSI terminal.
/// assert!(blitters.half);
/// # });
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlitterSupport {
    /// `▀` upper-half block — available on any ANSI terminal.
    pub half: bool,
    /// `▖▗▘▝` quadrant blocks — available on any Unicode-capable terminal.
    pub quad: bool,
    /// `🬀`..`🬻` sextants (Unicode 13+) — off by default until a renderer
    /// confirms support. This issue wires the capability slot; a sextant
    /// renderer is a separate feature.
    pub sextant: bool,
}

impl Default for BlitterSupport {
    fn default() -> Self {
        Self {
            half: true,
            quad: true,
            sextant: false,
        }
    }
}

/// Read-only snapshot of negotiated terminal capabilities, populated once at
/// session enter via DA1/DA2/XTGETTCAP.
///
/// App code **must not** be required to branch on this — it exists for
/// diagnostics and to drive the automatic blitter ladder (see
/// [`Capabilities::best_blitter`]). On a headless backend (TestBackend / piped
/// stdout) or when the probe gets no reply, every field falls back to a
/// conservative default.
///
/// Available since `0.21.0`.
///
/// # Example
///
/// ```no_run
/// # slt::run(|ui: &mut slt::Context| {
/// let caps = ui.capabilities();
/// if caps.sixel {
///     // Diagnostics only — image rendering already routes through the ladder.
/// }
/// # });
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Capabilities {
    /// 24-bit color confirmed (XTGETTCAP `Tc`/`RGB` or `COLORTERM`).
    pub truecolor: bool,
    /// Sixel graphics confirmed (DA1 attribute `4`).
    pub sixel: bool,
    /// iTerm2 OSC 1337 inline-image protocol confirmed (env identity for
    /// iTerm2 / WezTerm / Tabby / mintty; issue #265).
    pub iterm2: bool,
    /// Kitty graphics protocol confirmed (DA2 terminal-ID heuristic).
    pub kitty_graphics: bool,
    /// Kitty keyboard protocol confirmed.
    pub kitty_keyboard: bool,
    /// Synchronized output (DECSET 2026) confirmed.
    pub sync_output: bool,
    /// Set of cell-art blitters the terminal can drive.
    pub blitters: BlitterSupport,
}

/// Descending image-render preference. The first capability that is available
/// wins; app code never selects a [`Blitter`] directly.
///
/// Ladder order: [`Kitty`](Blitter::Kitty) > [`Sixel`](Blitter::Sixel) >
/// [`Iterm2`](Blitter::Iterm2) > [`Sextant`](Blitter::Sextant) >
/// [`HalfBlock`](Blitter::HalfBlock).
///
/// Available since `0.21.0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blitter {
    /// Kitty graphics protocol (highest fidelity).
    Kitty,
    /// Sixel graphics protocol.
    Sixel,
    /// iTerm2 OSC 1337 inline-image protocol (issue #265). Pixel-accurate on
    /// Tabby, older iTerm2, and WezTerm's iTerm2-compat mode.
    Iterm2,
    /// Unicode sextant cell art.
    Sextant,
    /// Half-block cell art (universal fallback).
    HalfBlock,
}

impl Capabilities {
    /// Resolve the best available image blitter for this terminal.
    ///
    /// Returns the first supported rung of the ladder
    /// (Kitty > Sixel > iTerm2 > Sextant > HalfBlock). This is total: it always
    /// returns a [`Blitter`], falling through to [`Blitter::HalfBlock`] which
    /// every terminal supports.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// let _ = ui.capabilities().best_blitter();
    /// # });
    /// ```
    pub fn best_blitter(&self) -> Blitter {
        if self.kitty_graphics {
            Blitter::Kitty
        } else if self.sixel {
            Blitter::Sixel
        } else if self.iterm2 {
            Blitter::Iterm2
        } else if self.blitters.sextant {
            Blitter::Sextant
        } else {
            Blitter::HalfBlock
        }
    }
}

/// Return the process-global negotiated [`Capabilities`], probing the terminal
/// exactly once on first call and caching the result.
///
/// On an identified direct terminal, the probe issues DA1 (`CSI c`), DA2
/// (`CSI > c`), and XTGETTCAP for the truecolor capname, reading replies
/// through the existing OSC round-trip infrastructure with a bounded total
/// timeout (≤180ms). Generic PTY wrappers, `TERM=dumb`, and tmux/screen skip
/// automatic queries to avoid leaking control bytes or racing user input;
/// environment-based fallbacks remain available. Set
/// `SLT_FORCE_TERMINAL_QUERIES=1` to opt in or
/// `SLT_DISABLE_TERMINAL_QUERIES=1` to disable all terminal queries. Repeated
/// calls are free.
#[cfg(feature = "crossterm")]
#[cfg_attr(docsrs, doc(cfg(feature = "crossterm")))]
pub fn capabilities() -> Capabilities {
    use std::sync::OnceLock;
    if !io::stdout().is_terminal() {
        return Capabilities::default();
    }
    static CACHED: OnceLock<Capabilities> = OnceLock::new();
    *CACHED.get_or_init(probe_capabilities)
}

/// Send DA1/DA2/XTGETTCAP and parse the replies into a [`Capabilities`].
///
/// Conservative on failure: any unread / unparsable reply leaves the
/// corresponding flag at its default. The total stdin wait is bounded to keep
/// startup latency within the same budget as the existing OSC 11 query.
#[cfg(feature = "crossterm")]
fn probe_capabilities() -> Capabilities {
    let mut caps = Capabilities::default();
    if automatic_terminal_queries_allowed() {
        // Total stdin wait is bounded to ≤180ms (90 + 30 + 30 + 30) so a
        // silent terminal cannot stall startup beyond a small multiple of the
        // existing OSC-11 budget. A responsive terminal replies in well under
        // 10ms per query, so the common path adds negligible latency.
        let mut out = io::stdout();
        // DA1 then DA2 in one write — both terminate with `c`, so a single
        // DA-aware read drains both replies (in order) when supported.
        if write!(out, "\x1b[c\x1b[>c").is_ok()
            && out.flush().is_ok()
            && let Some(resp) = read_da_response(Duration::from_millis(90))
        {
            parse_da1(&resp, &mut caps);
            parse_da2(&resp, &mut caps);
        }

        // Kitty graphics query: APC G a=q (query) with a 1×1 RGB direct
        // payload. Base64 of three zero bytes = "AAAA".
        if write!(out, "\x1b_Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\").is_ok()
            && out.flush().is_ok()
            && let Some(resp) = read_osc_response(Duration::from_millis(30))
        {
            parse_kitty_graphics_ack(&resp, &mut caps);
        }

        // XTGETTCAP for the `Tc` (truecolor) capname: `Tc` -> hex "5463".
        if write!(out, "\x1bP+q5463\x1b\\").is_ok()
            && out.flush().is_ok()
            && let Some(resp) = read_osc_response(Duration::from_millis(30))
        {
            parse_xtgettcap_truecolor(&resp, &mut caps);
        }

        // DECRQM for synchronized output (mode ?2026): CSI ? 2026 $ p.
        if write!(out, "\x1b[?2026$p").is_ok()
            && out.flush().is_ok()
            && let Some(resp) = read_decrpm_response(Duration::from_millis(30))
        {
            match parse_decrpm_sync_output(&resp) {
                Some(true) => {
                    caps.sync_output = true;
                    let _ = SYNC_OUTPUT_RESOLUTION.set(SyncOutputResolution::Supported);
                }
                Some(false) => {
                    let _ = SYNC_OUTPUT_RESOLUTION.set(SyncOutputResolution::Unsupported);
                }
                None => {}
            }
        }
    }

    // Env precedence chain stays authoritative for truecolor: a positive
    // COLORTERM/TERM signal confirms it even when the probe is silent.
    if matches!(ColorDepth::detect(), ColorDepth::TrueColor) {
        caps.truecolor = true;
    }

    if !caps.sixel && term_is_sixel_host() {
        caps.sixel = true;
    }

    // Env-fallback: when the runtime queries are silent (no reply within the
    // timeout), trust the terminal identity for the Kitty-graphics family so a
    // known-capable host (Kitty, Ghostty, WezTerm) still climbs the top rung.
    // The query above wins when it answers; this only fills an unknown.
    if !caps.kitty_graphics && term_is_kitty_graphics_host() {
        caps.kitty_graphics = true;
    }

    // iTerm2 OSC 1337 has no DA1/DA2 signal (issue #265): the protocol is
    // identified purely by terminal identity. Fill the capability slot from the
    // env so the blitter ladder can offer it below Kitty/Sixel.
    if term_is_iterm_host() {
        caps.iterm2 = true;
    }

    caps
}

/// Heuristic env-detection for iTerm2 OSC 1337 inline-image hosts (issue #265).
///
/// The protocol carries no DA reply, so detection is by `TERM_PROGRAM` identity
/// only: iTerm2, WezTerm (iTerm2-compat), Tabby, and mintty.
#[cfg(feature = "crossterm")]
fn term_is_iterm_host() -> bool {
    let term_program = std::env::var("TERM_PROGRAM")
        .unwrap_or_default()
        .to_ascii_lowercase();
    term_is_iterm_host_env(
        &term_program,
        terminal_is_multiplexed(),
        force_env_enabled("SLT_FORCE_ITERM"),
    )
}

#[cfg(feature = "crossterm")]
fn term_is_iterm_host_env(term_program: &str, multiplexed: bool, forced: bool) -> bool {
    if forced {
        return true;
    }
    if multiplexed {
        return false;
    }
    matches!(term_program, "iterm.app" | "wezterm" | "tabby" | "mintty")
}

#[cfg(feature = "crossterm")]
fn term_is_sixel_host() -> bool {
    let term = std::env::var("TERM")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let term_program = std::env::var("TERM_PROGRAM")
        .unwrap_or_default()
        .to_ascii_lowercase();
    term_is_sixel_host_env(
        &term,
        &term_program,
        terminal_is_multiplexed(),
        force_env_enabled("SLT_FORCE_SIXEL"),
    )
}

#[cfg(feature = "crossterm")]
fn term_is_sixel_host_env(term: &str, term_program: &str, multiplexed: bool, forced: bool) -> bool {
    if forced {
        return true;
    }
    if multiplexed {
        return false;
    }
    const KNOWN_SIXEL_TERMS: &[&str] = &["mlterm", "foot", "yaft", "xterm-256color-sixel"];
    const KNOWN_SIXEL_TERM_PROGRAMS: &[&str] = &["foot", "mlterm", "wezterm", "ghostty"];
    KNOWN_SIXEL_TERMS.contains(&term)
        || term.contains("sixel")
        || KNOWN_SIXEL_TERM_PROGRAMS.contains(&term_program)
}

/// Heuristic env-fallback for Kitty-graphics hosts, consulted only when the
/// runtime Kitty graphics query returned no reply. Matches the documented
/// `TERM` / `TERM_PROGRAM` identities of terminals that implement the Kitty
/// graphics protocol.
#[cfg(feature = "crossterm")]
fn term_is_kitty_graphics_host() -> bool {
    let term = std::env::var("TERM")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let term_program = std::env::var("TERM_PROGRAM")
        .unwrap_or_default()
        .to_ascii_lowercase();
    term_is_kitty_graphics_host_env(
        &term,
        &term_program,
        terminal_is_multiplexed(),
        force_env_enabled("SLT_FORCE_KITTY"),
    )
}

#[cfg(feature = "crossterm")]
fn term_is_kitty_graphics_host_env(
    term: &str,
    term_program: &str,
    multiplexed: bool,
    forced: bool,
) -> bool {
    if forced {
        return true;
    }
    if multiplexed {
        return false;
    }
    // Kitty sets `TERM=xterm-kitty`; Ghostty/WezTerm advertise via TERM_PROGRAM.
    term.contains("kitty") || matches!(term_program, "ghostty" | "wezterm" | "kitty")
}

#[cfg(feature = "crossterm")]
fn terminal_is_multiplexed() -> bool {
    if std::env::var_os("TMUX").is_some() || std::env::var_os("STY").is_some() {
        return true;
    }
    let term = std::env::var("TERM")
        .unwrap_or_default()
        .to_ascii_lowercase();
    terminal_is_multiplexed_env(&term, false, false)
}

#[cfg(feature = "crossterm")]
fn terminal_is_multiplexed_env(term: &str, has_tmux: bool, has_sty: bool) -> bool {
    let term = term.to_ascii_lowercase();
    has_tmux || has_sty || term.starts_with("tmux") || term.starts_with("screen")
}

#[cfg(feature = "crossterm")]
fn force_env_enabled(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .is_some_and(|value| truthy_env_value(&value))
}

#[cfg(feature = "crossterm")]
fn truthy_env_value(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

#[cfg(feature = "crossterm")]
fn terminal_queries_allowed() -> bool {
    let term = std::env::var("TERM").unwrap_or_default();
    terminal_query_allowed(
        io::stdout().is_terminal(),
        io::stdin().is_terminal(),
        &term,
        terminal_is_multiplexed(),
        force_env_enabled("SLT_FORCE_TERMINAL_QUERIES"),
        force_env_enabled("SLT_DISABLE_TERMINAL_QUERIES"),
    )
}

#[cfg(feature = "crossterm")]
fn automatic_terminal_queries_allowed() -> bool {
    if !terminal_queries_allowed() {
        return false;
    }
    force_env_enabled("SLT_FORCE_TERMINAL_QUERIES") || terminal_query_host_is_identified()
}

#[cfg(feature = "crossterm")]
fn terminal_query_host_is_identified() -> bool {
    const IDENTITY_VARS: &[&str] = &[
        "TERM_PROGRAM",
        "WT_SESSION",
        "VTE_VERSION",
        "KONSOLE_VERSION",
        "KITTY_WINDOW_ID",
    ];
    if IDENTITY_VARS
        .iter()
        .any(|name| std::env::var_os(name).is_some_and(|value| !value.is_empty()))
    {
        return true;
    }

    let term = std::env::var("TERM")
        .unwrap_or_default()
        .to_ascii_lowercase();
    terminal_query_host_is_identified_env(&term, false)
}

fn terminal_query_host_is_identified_env(term: &str, has_identity_var: bool) -> bool {
    has_identity_var
        || matches!(
            term.to_ascii_lowercase().as_str(),
            "alacritty" | "foot" | "foot-extra" | "mlterm" | "wezterm" | "xterm-kitty"
        )
}

fn terminal_query_allowed(
    stdout_is_terminal: bool,
    stdin_is_terminal: bool,
    term: &str,
    multiplexed: bool,
    forced: bool,
    disabled: bool,
) -> bool {
    if !stdout_is_terminal || !stdin_is_terminal || disabled {
        return false;
    }
    if forced {
        return true;
    }
    !multiplexed && !term.is_empty() && !term.eq_ignore_ascii_case("dumb")
}

/// Process-wide pump that owns the only blocking `stdin` read used for
/// terminal-reply probing. See [`read_stdin_reply`] for why it exists.
#[cfg(feature = "crossterm")]
struct ReplyPump {
    rx: std::sync::mpsc::Receiver<u8>,
    /// `true` while a reader session wants bytes. The pump thread re-checks it
    /// after every successful read and exits once it is cleared.
    serve: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Set by the pump thread on exit, distinguishing "parked inside a
    /// blocking `read()`" (reusable) from "gone" (must respawn).
    exited: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

#[cfg(feature = "crossterm")]
static REPLY_PUMP: std::sync::Mutex<Option<ReplyPump>> = std::sync::Mutex::new(None);

/// Read one terminal reply from raw stdin, hard-bounded by `timeout`, stopping
/// early once `is_complete` recognizes a full reply (or at the 4096-byte cap).
///
/// Why a pump thread: the previous readers gated a blocking
/// `io::stdin().read()` behind `crossterm::event::poll()`. Those two observe
/// different things — `poll()` answers "does crossterm's *internal event
/// queue* have something?", while the raw `read()` waits for bytes on the
/// stdin descriptor — and crossterm's poller consumes bytes from that same
/// descriptor into its own parser. On a host that never answers probe queries
/// (a detached tmux pane, `script`-style PTY wrappers, CI runners), `poll()`
/// could return `true` for a queued non-byte event while raw stdin stayed
/// empty, so the one-byte `read()` blocked forever *inside* the deadline loop
/// and the application hung on a blank alternate screen before its first
/// frame; later keystrokes were swallowed by crossterm's queue instead of
/// unblocking it. Moving the only blocking `read()` onto a dedicated thread
/// and waiting on a channel with `recv_timeout` makes every reply read
/// genuinely bounded by its budget no matter what the host does.
///
/// The pump is a process-wide singleton so back-to-back probes share one byte
/// stream instead of racing two readers for the same reply. After each
/// session the thread is retired: `serve` is cleared and a DSR status query
/// (`CSI 5 n`) nudges the terminal — an answering host replies `CSI 0 n`,
/// which wakes the parked `read()`, the thread observes `serve == false` and
/// exits, and the nudge bytes stay in the channel where the next session's
/// drain discards them (they never reach the application's input stream). A
/// host that answers nothing leaves the thread parked; it is reused by the
/// next session, and at worst it swallows one byte of typeahead on a host
/// class where, before this fix, startup deadlocked outright.
#[cfg(feature = "crossterm")]
fn read_stdin_reply(
    timeout: Duration,
    mut is_complete: impl FnMut(&[u8]) -> bool,
) -> Option<String> {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, mpsc};

    let deadline = Instant::now() + timeout;

    let Ok(mut slot) = REPLY_PUMP.lock() else {
        // Poisoned: a prior session panicked mid-read. Skip probing entirely
        // rather than risk a second fault; every caller treats `None` as "the
        // terminal stayed silent".
        return None;
    };

    let pump = match slot.take().filter(|p| !p.exited.load(Ordering::Acquire)) {
        Some(pump) => {
            // A parked pump from an earlier session: its thread is still
            // blocked in `read()` on a silent host. Reusing it (instead of
            // spawning a second thread) is what prevents two readers from
            // racing each other for the same reply bytes.
            pump.serve.store(true, Ordering::Release);
            pump
        }
        None => {
            let (tx, rx) = mpsc::channel::<u8>();
            let serve = Arc::new(AtomicBool::new(true));
            let exited = Arc::new(AtomicBool::new(false));
            let thread_serve = Arc::clone(&serve);
            let thread_exited = Arc::clone(&exited);
            let spawned = std::thread::Builder::new()
                .name("slt-reply-pump".into())
                .spawn(move || {
                    let mut stdin = io::stdin();
                    // One byte per read on purpose: a parked thread that wakes
                    // on real key input forwards at most this single byte
                    // before observing `serve == false` and exiting, so the
                    // worst-case typeahead loss on a silent host is exactly
                    // one byte (replies are short; the syscall-per-byte cost
                    // is irrelevant for one-shot probes).
                    let mut buf = [0u8; 1];
                    loop {
                        match stdin.read(&mut buf) {
                            Ok(0) | Err(_) => break,
                            Ok(_) => {
                                if tx.send(buf[0]).is_err() {
                                    thread_exited.store(true, Ordering::Release);
                                    return;
                                }
                            }
                        }
                        if !thread_serve.load(Ordering::Acquire) {
                            break;
                        }
                    }
                    thread_exited.store(true, Ordering::Release);
                });
            if spawned.is_err() {
                return None;
            }
            ReplyPump { rx, serve, exited }
        }
    };

    // Discard bytes left over from a previous session: a reply that arrived
    // after its deadline, or the retirement nudge's `CSI 0 n` answer.
    while pump.rx.try_recv().is_ok() {}

    let bytes = collect_reply(&pump.rx, deadline, &mut is_complete);

    // Retire the thread so it does not sit on a pending `read()` competing
    // with crossterm's event loop for real key input once the session ends.
    // The nudge fires only under raw mode (the `run()` / session-enter probe
    // paths): in cooked mode — e.g. a standalone `detect_color_scheme()`
    // call — the terminal would *echo* its `CSI 0 n` answer into the user's
    // scrollback as visible garbage, so there the parked thread is simply
    // left for the next session to reuse.
    pump.serve.store(false, Ordering::Release);
    if crossterm::terminal::is_raw_mode_enabled().unwrap_or(false) {
        let mut out = io::stdout();
        let _ = write!(out, "\x1b[5n");
        let _ = out.flush();
    }
    *slot = Some(pump);
    drop(slot);

    if bytes.is_empty() {
        return None;
    }
    String::from_utf8(bytes).ok()
}

/// Deadline-bounded accumulation loop shared by every reply reader: pull bytes
/// off the pump channel until `is_complete` fires, the 4096-byte cap is hit,
/// the deadline passes, or the pump disconnects (stdin EOF). Returns whatever
/// arrived — callers map an empty buffer to "no reply" and a partial buffer to
/// a best-effort parse, matching the pre-pump readers exactly.
#[cfg(feature = "crossterm")]
fn collect_reply(
    rx: &std::sync::mpsc::Receiver<u8>,
    deadline: Instant,
    is_complete: &mut dyn FnMut(&[u8]) -> bool,
) -> Vec<u8> {
    let mut bytes = Vec::new();
    loop {
        let now = Instant::now();
        if now >= deadline {
            break;
        }
        match rx.recv_timeout(deadline - now) {
            Ok(byte) => {
                bytes.push(byte);
                if is_complete(&bytes) || bytes.len() >= 4096 {
                    break;
                }
            }
            // Timed out, or the pump thread is gone (stdin EOF / error).
            Err(_) => break,
        }
    }
    bytes
}

/// Completion predicate for OSC / DCS / CSI-`t` style replies, which terminate
/// with BEL (`\x07`) or ST (`ESC \`).
#[cfg(feature = "crossterm")]
fn osc_reply_complete(bytes: &[u8]) -> bool {
    let len = bytes.len();
    bytes[len - 1] == b'\x07' || (len >= 2 && bytes[len - 2] == 0x1B && bytes[len - 1] == b'\\')
}

/// Completion predicate builder for Device-Attributes replies: `c` is the
/// final byte of each DA reply, and a combined `CSI c CSI > c` query yields
/// two of them, so completion fires on the second `c`.
#[cfg(feature = "crossterm")]
fn da_reply_complete() -> impl FnMut(&[u8]) -> bool {
    let mut terminators = 0usize;
    move |bytes: &[u8]| {
        if bytes[bytes.len() - 1] == b'c' {
            terminators += 1;
        }
        terminators >= 2
    }
}

/// Completion predicate for DECRPM replies (`CSI ? <mode> ; <Ps> $ y`).
#[cfg(feature = "crossterm")]
fn decrpm_reply_complete(bytes: &[u8]) -> bool {
    bytes[bytes.len() - 1] == b'y'
}

/// Read a Device-Attributes reply, which (unlike OSC) terminates with the byte
/// `c` rather than BEL / ST. Drains up to two `c`-terminated CSI replies
/// (DA1 + DA2) within the timeout so a combined `CSI c CSI > c` query yields
/// both answers in one string.
#[cfg(feature = "crossterm")]
fn read_da_response(timeout: Duration) -> Option<String> {
    read_stdin_reply(timeout, da_reply_complete())
}

/// Parse a DA1 reply (`CSI ? <attrs> c`). Attribute `4` indicates Sixel
/// support. Only the DA1 segment is consulted; a trailing DA2 segment in the
/// same string is ignored here.
#[cfg(feature = "crossterm")]
fn parse_da1(response: &str, caps: &mut Capabilities) {
    // DA1 reply: ESC [ ? <n> ; <n> ; ... c  (no `>` after `[`).
    let mut search = response;
    while let Some(pos) = search.find("\x1b[?") {
        let body = &search[pos + 3..];
        let Some(end) = body.find('c') else { break };
        let attrs = &body[..end];
        for attr in attrs.split(';') {
            if attr.trim() == "4" {
                caps.sixel = true;
            }
        }
        search = &body[end + 1..];
    }
}

/// Parsed DA2 (secondary device attributes) terminal identity:
/// `(primary_id, firmware_version)` from `CSI > <id> ; <ver> ; <sub> c`.
///
/// Returns `None` if the string contains no DA2 reply. Kept separate from the
/// `Capabilities` mutation so it is independently testable and so callers that
/// want the raw identity (e.g. future per-terminal quirks) are not forced
/// through capability inference.
#[cfg(feature = "crossterm")]
fn parse_da2(response: &str, caps: &mut Capabilities) {
    let Some((id, _ver)) = parse_da2_identity(response) else {
        return;
    };
    // DA2 primary id `41` is the documented Kitty graphics terminal id (Kitty
    // reports `\x1b[>41;<ver>;<sub>c`). This is the one unambiguous DA2 graphics
    // signal; every other host is resolved by the Kitty graphics query above or
    // the env-fallback, so we deliberately do not maintain a wider id registry.
    const KITTY_GRAPHICS_DA2_ID: u32 = 41;
    if id == KITTY_GRAPHICS_DA2_ID {
        caps.kitty_graphics = true;
    }
}

/// Extract `(primary_id, version)` from a DA2 reply, or `None` if absent.
#[cfg(feature = "crossterm")]
fn parse_da2_identity(response: &str) -> Option<(u32, u32)> {
    let pos = response.find("\x1b[>")?;
    let body = &response[pos + 3..];
    let end = body.find('c')?;
    let mut parts = body[..end].split(';');
    let id = parts.next()?.trim().parse::<u32>().ok()?;
    let ver = parts.next().and_then(|s| s.trim().parse::<u32>().ok());
    Some((id, ver.unwrap_or(0)))
}

/// Parse a Kitty graphics protocol query ack (`APC G i=31;OK ST`). A terminal
/// that supports the protocol echoes the image id with an `OK` status; anything
/// else (silence, error status) leaves the flag untouched.
#[cfg(feature = "crossterm")]
fn parse_kitty_graphics_ack(response: &str, caps: &mut Capabilities) {
    // Ack form: ESC _ G <key=val>;OK ESC \  — we sent i=31, so look for that id
    // paired with an OK status.
    if let Some(pos) = response.find("\x1b_G") {
        let body = &response[pos + 3..];
        let end = body.find("\x1b\\").unwrap_or(body.len());
        let payload = &body[..end];
        if payload.contains("i=31") && payload.contains("OK") {
            caps.kitty_graphics = true;
        }
    }
}

/// Parse an XTGETTCAP reply for the `Tc` (truecolor) capname. A valid reply is
/// `DCS 1 + r <hex(capname)>[=<hex(value)>] ST`; a leading `1` means the
/// capability is present.
#[cfg(feature = "crossterm")]
fn parse_xtgettcap_truecolor(response: &str, caps: &mut Capabilities) {
    // Valid reply prefix: ESC P 1 + r  (DCS 1 + r ...). `Tc` -> hex 5463.
    if let Some(pos) = response.find("\x1bP1+r") {
        let body = &response[pos + 5..];
        if body
            .to_ascii_lowercase()
            .split([';', '\x1b'])
            .any(|seg| seg.starts_with("5463"))
        {
            caps.truecolor = true;
        }
    }
}

/// Tri-state outcome of the DECRQM ?2026 (synchronized output) probe.
///
/// The synchronized-output BSU/ESU emission is gated on this rather than on the
/// public [`Capabilities::sync_output`] bool alone, because the public flag is
/// only ever set on *positive* support evidence. Gating emission on that flag
/// directly would flip the historic always-emit behavior to never-emit on every
/// headless / non-answering host (a regression). This tri-state lets the gate
/// suppress BSU/ESU **only** when the terminal definitively reported the mode
/// unrecognized, and keep emitting in the `Unknown` (silent / headless) case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SyncOutputResolution {
    /// DECRQM confirmed mode ?2026 is recognized (set or reset).
    Supported,
    /// DECRQM explicitly reported mode ?2026 as not recognized (Ps = 0).
    Unsupported,
}

/// Process-global resolution of the synchronized-output probe, populated at most
/// once by [`probe_capabilities`]. Absent (`Unknown`) until the probe answers.
static SYNC_OUTPUT_RESOLUTION: std::sync::OnceLock<SyncOutputResolution> =
    std::sync::OnceLock::new();

/// Whether the flush pipeline should wrap a frame in synchronized-output
/// BSU/ESU guards.
///
/// Returns `true` (emit) unless the DECRQM ?2026 probe *definitively* reported
/// the mode as unrecognized. A silent / headless / never-run probe leaves the
/// resolution `Unknown`, in which case this keeps emitting exactly as the
/// pre-gate code always did. This is the behavior-preserving half of the
/// capability gate: positive support and the unknown default both emit; only a
/// confirmed-unsupported terminal suppresses.
fn should_emit_synchronized_update() -> bool {
    !matches!(
        SYNC_OUTPUT_RESOLUTION.get(),
        Some(SyncOutputResolution::Unsupported)
    )
}

/// Read a DECRPM reply, which terminates with the byte `y` rather than BEL / ST
/// (used for the DECRQM ?2026 synchronized-output probe). Bounded by `timeout`
/// so a terminal that ignores the query cannot stall startup.
#[cfg(feature = "crossterm")]
fn read_decrpm_response(timeout: Duration) -> Option<String> {
    read_stdin_reply(timeout, decrpm_reply_complete)
}

/// Parse a DECRPM reply for synchronized output (mode `2026`):
/// `CSI ? 2026 ; <Ps> $ y`.
///
/// Returns:
///   * `Some(true)`  — mode recognized (`Ps` ∈ {1, 2, 3, 4}: set / reset /
///     permanently-set / permanently-reset all mean *supported*),
///   * `Some(false)` — mode not recognized (`Ps` = 0),
///   * `None`        — no DECRPM reply for mode 2026 in the string.
#[cfg(feature = "crossterm")]
fn parse_decrpm_sync_output(response: &str) -> Option<bool> {
    // Reply body: ESC [ ? 2026 ; <Ps> $ y
    let pos = response.find("\x1b[?2026;")?;
    let body = &response[pos + "\x1b[?2026;".len()..];
    let end = body.find("$y")?;
    let ps = body[..end].trim().parse::<u32>().ok()?;
    // Ps = 0 → not recognized; any other reported state means the mode exists.
    Some(ps != 0)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GraphicsEmissionSupport {
    real_terminal: bool,
    capabilities: Capabilities,
    force_kitty: bool,
    force_sixel: bool,
    force_iterm: bool,
}

impl GraphicsEmissionSupport {
    fn detect(capabilities: Capabilities) -> Self {
        Self {
            real_terminal: true,
            capabilities,
            force_kitty: force_env_enabled("SLT_FORCE_KITTY"),
            force_sixel: force_env_enabled("SLT_FORCE_SIXEL"),
            force_iterm: force_env_enabled("SLT_FORCE_ITERM"),
        }
    }

    #[cfg(any(test, feature = "pty-test"))]
    fn capture() -> Self {
        Self {
            real_terminal: true,
            capabilities: Capabilities::default(),
            force_kitty: force_env_enabled("SLT_FORCE_KITTY"),
            force_sixel: force_env_enabled("SLT_FORCE_SIXEL"),
            force_iterm: force_env_enabled("SLT_FORCE_ITERM"),
        }
    }

    fn should_emit_kitty(self) -> bool {
        self.real_terminal && (self.capabilities.kitty_graphics || self.force_kitty)
    }

    fn should_emit_sprixel(self, protocol: SprixelProtocol) -> bool {
        if !self.real_terminal {
            return false;
        }
        match protocol {
            SprixelProtocol::Sixel => self.capabilities.sixel || self.force_sixel,
            SprixelProtocol::Iterm2 => self.capabilities.iterm2 || self.force_iterm,
            SprixelProtocol::Unknown => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SprixelProtocol {
    Sixel,
    Iterm2,
    Unknown,
}

fn sprixel_protocol(seq: &str) -> SprixelProtocol {
    if seq.starts_with("\x1bPq") {
        SprixelProtocol::Sixel
    } else if seq.starts_with("\x1b]1337;File=") {
        SprixelProtocol::Iterm2
    } else {
        SprixelProtocol::Unknown
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
    pub(crate) theme_bg: Option<Color>,
    kitty_mgr: KittyImageManager,
    graphics_support: GraphicsEmissionSupport,
    /// Reused run-coalescing scratch for `flush_buffer_diff` (issue #269). Its
    /// capacity persists across frames so the hot flush loop never allocates a
    /// fresh `String` per call.
    run_buf: String,
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
    start_row: u16,
    reserved: bool,
    color_depth: ColorDepth,
    pub(crate) theme_bg: Option<Color>,
    kitty_mgr: KittyImageManager,
    graphics_support: GraphicsEmissionSupport,
    /// Reused run-coalescing scratch for `flush_buffer_diff` (issue #269).
    run_buf: String,
}

/// Initial capacity for the reused per-frame run-coalescing buffer. Sized to
/// comfortably hold a full wide terminal row of multi-byte graphemes so the
/// allocation is paid once at construction, never per frame.
const RUN_BUF_INITIAL_CAPACITY: usize = 4096;

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
    /// When `true`, the guard never touched real raw-mode / terminal state
    /// (PTY test harness path). `restore` then becomes a no-op so dropping a
    /// captured-sink `Terminal` does not call `disable_raw_mode` or emit
    /// teardown escapes into the byte capture. Always `false` on the
    /// production `enter` path.
    harness: bool,
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
            harness: false,
        };

        terminal::enable_raw_mode()?;
        if let Err(err) = write_session_enter(stdout, &guard) {
            guard.restore(stdout, false);
            return Err(err);
        }

        // Issue #264: run the one-shot DA1/DA2/XTGETTCAP capability probe at
        // session enter, while raw mode is active so the replies are readable.
        // `capabilities()` caches in a `OnceLock`, so the resume re-enter path
        // never re-probes. Never runs on the PTY-harness path (`harness` is
        // always `false` here, but resume/harness re-entries go through
        // `write_session_enter` directly, not `enter`).
        let _ = capabilities();

        Ok(guard)
    }

    fn restore(&self, stdout: &mut impl Write, inline_reserved: bool) {
        // PTY harness guard: nothing was ever entered, so nothing to restore.
        if self.harness {
            return;
        }
        let _ = write_session_exit(
            stdout,
            self.mode,
            inline_reserved,
            self.mouse_enabled,
            self.kitty_keyboard,
        );
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
        let graphics_support = GraphicsEmissionSupport::detect(capabilities());

        Ok(Self {
            stdout: Sink::Stdout(BufWriter::with_capacity(65536, raw)),
            current: Buffer::empty(area),
            previous: Buffer::empty(area),
            cursor_visible: false,
            session,
            color_depth,
            theme_bg: None,
            kitty_mgr: KittyImageManager::new(),
            graphics_support,
            run_buf: String::with_capacity(RUN_BUF_INITIAL_CAPACITY),
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
        let sync_guard = should_emit_synchronized_update();
        if sync_guard {
            queue!(self.stdout, BeginSynchronizedUpdate)?;
        }
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
            &mut self.run_buf,
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
        flush_sprixels_checked(
            &mut self.stdout,
            &self.current,
            &self.previous,
            0,
            self.graphics_support,
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
        Self {
            stdout: Sink::Capture(Vec::new()),
            current: Buffer::empty(area),
            previous: Buffer::empty(area),
            cursor_visible: false,
            session: TerminalSessionGuard {
                mode: TerminalSessionMode::Fullscreen,
                mouse_enabled: false,
                kitty_keyboard: false,
                report_all_keys: false,
                harness: true,
            },
            color_depth,
            theme_bg: None,
            kitty_mgr: KittyImageManager::new(),
            graphics_support: GraphicsEmissionSupport::capture(),
            run_buf: String::with_capacity(RUN_BUF_INITIAL_CAPACITY),
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
        let graphics_support = GraphicsEmissionSupport::detect(capabilities());

        let (_, cursor_row) = match cursor::position() {
            Ok(pos) => pos,
            Err(err) => {
                session.restore(&mut raw, false);
                return Err(err);
            }
        };
        Ok(Self {
            stdout: Sink::Stdout(BufWriter::with_capacity(65536, raw)),
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
            graphics_support,
            run_buf: String::with_capacity(RUN_BUF_INITIAL_CAPACITY),
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

        // Synchronized output (BSU/ESU) is gated on the DECRQM ?2026 probe
        // (v0.21.1); see `Terminal::flush`. Silent / headless keeps emitting.
        let sync_guard = should_emit_synchronized_update();
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
            &mut self.run_buf,
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
        flush_sprixels_checked(
            &mut self.stdout,
            &self.current,
            &self.previous,
            row_offset,
            self.graphics_support,
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
pub(crate) use selection::{SelectionState, apply_selection_overlay, extract_selection_text};
#[cfg(test)]
pub(crate) use selection::{find_innermost_rect, normalize_selection};

/// Detected terminal color scheme from OSC 11.
#[non_exhaustive]
#[cfg(feature = "crossterm")]
#[cfg_attr(docsrs, doc(cfg(feature = "crossterm")))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorScheme {
    /// Dark background detected.
    Dark,
    /// Light background detected.
    Light,
    /// Could not determine the scheme.
    Unknown,
}

/// Read an OSC-style reply (BEL- or ST-terminated), hard-bounded by `timeout`.
#[cfg(feature = "crossterm")]
fn read_osc_response(timeout: Duration) -> Option<String> {
    read_stdin_reply(timeout, osc_reply_complete)
}

/// Query the terminal's background color via OSC 11 and return the detected scheme.
#[cfg(feature = "crossterm")]
#[cfg_attr(docsrs, doc(cfg(feature = "crossterm")))]
pub fn detect_color_scheme() -> ColorScheme {
    if !terminal_queries_allowed() {
        return ColorScheme::Unknown;
    }

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

pub(crate) fn base64_encode(input: &[u8]) -> String {
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

/// Read clipboard contents via an OSC 52 terminal query.
///
/// Writes the OSC 52 read request (`ESC ] 52 ; c ; ? BEL`) to stdout, then
/// blocks reading the terminal's reply from stdin for up to ~200 ms. Returns
/// the decoded clipboard text, or `None` if the terminal does not answer, the
/// reply is empty, or it cannot be decoded. Many terminals disable OSC 52 reads
/// by default for security, in which case this always returns `None`.
///
/// # Note
///
/// This call reads the **same stdin** the [`run`](crate::run) event loop polls,
/// **synchronously and outside** the loop's own event dispatch. That creates a
/// typeahead-swallow hazard: during the blocking read window, any bytes the user
/// types — and any other terminal report in flight (mouse, focus, paste, a
/// different OSC reply) — land in this function's byte reader instead of the
/// event queue. Keystrokes consumed here are silently lost, and a foreign report
/// interleaved with the OSC 52 reply can corrupt parsing so the read returns
/// `None`. There is no locking between this reader and the run loop's poll, so
/// calling it concurrently from another thread while the loop is running races
/// on stdin.
///
/// Recommended usage:
///   * Call it from the main thread, **not** from a spawned thread, and never
///     concurrently with a running [`run`](crate::run) loop on another thread.
///   * Trigger it only in direct response to an explicit user action (e.g. a
///     paste keybinding) and keep the window brief, so the typeahead lost to the
///     blocking read is bounded to that moment.
///   * Prefer the OS clipboard via a dedicated crate when reliable, race-free
///     clipboard reads are required; reserve this for the no-dependency,
///     terminal-only fallback.
///   * For *writing* the clipboard there is no such hazard — that path only
///     emits bytes and never reads stdin.
#[cfg(feature = "crossterm")]
#[cfg_attr(docsrs, doc(cfg(feature = "crossterm")))]
pub fn read_clipboard() -> Option<String> {
    if !terminal_queries_allowed() {
        return None;
    }

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
    run_buf: &mut String,
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
    // `run_buf` is hoisted to a caller-owned, reused buffer (issue #269): its
    // backing allocation persists across frames so the hot flush loop performs
    // no per-frame `String` allocation. Start clean but keep capacity.
    run_buf.clear();
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

                let need_move = last_cursor.is_none_or(|(lx, ly)| lx != x || ly != abs_y);
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
                        // Emit the OSC 8 open in three borrowed `Print`s instead
                        // of `format!`ing a throwaway `String` per link-state
                        // change (issue #269). The byte stream is identical to
                        // `"\x1b]8;;{url}\x07"`.
                        queue!(stdout, Print("\x1b]8;;"))?;
                        queue!(stdout, Print(url))?;
                        queue!(stdout, Print("\x07"))?;
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
    // Own a local run buffer to keep the public bench signature stable
    // (issue #269); the real backends pass a reused field instead.
    let mut run_buf = String::with_capacity(RUN_BUF_INITIAL_CAPACITY);
    flush_buffer_diff(w, current, previous, color_depth, 0, &mut run_buf)
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
    // Own a local run buffer to keep the public bench signature stable
    // (issue #269). Use `__bench_flush_buffer_diff_mut_with_buf` to exercise
    // cross-frame buffer reuse explicitly.
    let mut run_buf = String::with_capacity(RUN_BUF_INITIAL_CAPACITY);
    __bench_flush_buffer_diff_mut_with_buf(w, current, previous, color_depth, &mut run_buf)
}

/// Reuse-aware variant of [`__bench_flush_buffer_diff_mut`] that threads a
/// caller-owned `run_buf` (issue #269), mirroring how the real backends carry
/// the buffer across frames. Refreshes per-row digests before the diff.
///
/// Not part of the stable API.
///
/// ```no_run
/// # use slt::{Buffer, Rect, ColorDepth, Style};
/// let area = Rect::new(0, 0, 8, 2);
/// let mut current = Buffer::empty(area);
/// let mut previous = Buffer::empty(area);
/// current.set_string(0, 0, "hi", Style::new());
/// let mut sink: Vec<u8> = Vec::new();
/// // The same `run_buf` can be passed across frames — its capacity persists.
/// let mut run_buf = String::with_capacity(4096);
/// slt::__bench_flush_buffer_diff_mut_with_buf(
///     &mut sink,
///     &mut current,
///     &mut previous,
///     ColorDepth::TrueColor,
///     &mut run_buf,
/// )
/// .unwrap();
/// ```
#[doc(hidden)]
pub fn __bench_flush_buffer_diff_mut_with_buf<W: Write>(
    w: &mut W,
    current: &mut Buffer,
    previous: &mut Buffer,
    color_depth: ColorDepth,
    run_buf: &mut String,
) -> io::Result<()> {
    current.recompute_line_hashes();
    previous.recompute_line_hashes();
    flush_buffer_diff(w, current, previous, color_depth, 0, run_buf)
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

/// Benchmark-only entry point for the Kitty image flush path.
///
/// Builds an `n`-image fixture and runs [`KittyImageManager::flush`] once into
/// the supplied sink at `row_offset`, mirroring the [`__bench_flush_buffer_diff`]
/// free-function style. `KittyPlacement` / `KittyImageManager` are `pub(crate)`,
/// so an external bench crate cannot construct them directly — this wrapper owns
/// the construction and only the `Write` sink crosses the crate boundary.
///
/// Not part of the stable API.
#[doc(hidden)]
pub fn __bench_flush_kitty<W: Write>(sink: &mut W, n: usize, row_offset: u32) -> io::Result<()> {
    let mut fixture = __bench_new_kitty_fixture(n);
    fixture.flush_inline(sink, row_offset)
}

/// Opaque test/bench fixture wrapping two `Buffer`s populated with structurally
/// identical sprixel placements, used to drive the [`flush_sprixels`] re-blit
/// path. `SprixelPlacement` is `pub(crate)`, so this fixture owns construction
/// and exposes only `Write`-based flush entry points across the crate boundary.
///
/// Returned by [`__bench_new_sprixel_fixture`].
#[doc(hidden)]
pub struct __BenchSprixelFixture {
    current: Buffer,
    previous: Buffer,
}

/// Build a self-contained sprixel-reblit fixture for the perf suite (v0.21.1).
///
/// Creates `n` opaque sprixel placements laid out down the buffer and mirrors
/// them into both the current and previous frame so the steady-state flush
/// re-blits nothing. Per-row digests are refreshed (as the real `flush` does)
/// so the per-row clean+hash shortcut in [`sprixel_needs_reblit`] is exercised.
///
/// Not part of the stable API.
#[doc(hidden)]
pub fn __bench_new_sprixel_fixture(n: usize) -> __BenchSprixelFixture {
    use crate::buffer::{SprixelCell, SprixelPlacement};

    // A buffer tall enough to stack `n` 2-row sprixels with a 1-row gap.
    let height = (n as u32 * 3).max(1);
    let area = Rect::new(0, 0, 8, height);
    let mut current = Buffer::empty(area);
    let mut previous = Buffer::empty(area);

    for i in 0..n {
        let placement = SprixelPlacement {
            content_hash: 0x5000 + i as u64,
            seq: "<SIXEL>".to_string(),
            x: 0,
            y: i as u32 * 3,
            cols: 4,
            rows: 2,
            cells: vec![SprixelCell::Opaque; 8],
        };
        current.sprixels.push(placement.clone());
        previous.sprixels.push(placement);
    }

    // Refresh digests so the per-row shortcut can fire, matching the real
    // `Terminal::flush` ordering (recompute happens before `flush_sprixels`).
    current.recompute_line_hashes();
    previous.recompute_line_hashes();

    __BenchSprixelFixture { current, previous }
}

// The bench fixture's inherent methods are reachable only once the crate root
// re-exports `__BenchSprixelFixture` (an integrator step listed in the release
// notes); until then the lib-target dead-code lint flags them, exactly as it
// would the already-shipped `__BenchKittyFixture` methods without their
// `lib.rs` re-export. They are also exercised by the in-crate tests below.
// Suppress the lint on the impl rather than gating the items behind `cfg(test)`,
// which would make them invisible to the external `benches/` crate they exist
// to serve.
#[allow(dead_code)]
impl __BenchSprixelFixture {
    /// Run [`flush_sprixels`] once, writing any re-blitted graphics into `sink`.
    /// A steady-state fixture emits nothing; this measures the no-damage scan
    /// cost (hash-set build + per-row shortcut) on the hot path.
    #[doc(hidden)]
    pub fn flush<W: Write>(&self, sink: &mut W, row_offset: u32) -> io::Result<()> {
        flush_sprixels(sink, &self.current, &self.previous, row_offset)
    }

    /// Number of sprixel placements in this fixture.
    #[doc(hidden)]
    pub fn len(&self) -> usize {
        self.current.sprixels.len()
    }

    /// Whether this fixture has zero placements.
    #[doc(hidden)]
    pub fn is_empty(&self) -> bool {
        self.current.sprixels.is_empty()
    }
}

/// Benchmark-only entry point for the optimized sprixel re-blit scan (v0.21.1).
///
/// Builds an `n`-placement steady-state fixture and runs [`flush_sprixels`] once
/// into `sink` at `row_offset`, mirroring the [`__bench_flush_buffer_diff`]
/// free-function style. A steady frame re-blits nothing, so this measures the
/// no-damage scan cost (hashed-key build + per-row clean/hash shortcut). When
/// the fixture is empty the early-out fires and no work is done.
///
/// Not part of the stable API.
#[doc(hidden)]
pub fn __bench_flush_sprixels<W: Write>(sink: &mut W, n: usize, row_offset: u32) -> io::Result<()> {
    let fixture = __bench_new_sprixel_fixture(n);
    if fixture.is_empty() {
        return Ok(());
    }
    debug_assert_eq!(fixture.len(), n);
    fixture.flush(sink, row_offset)
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

/// Structural identity key for a [`crate::buffer::SprixelPlacement`], matching
/// its [`PartialEq`] contract (`content_hash`/`x`/`y`/`cols`/`rows`, damage
/// matrix excluded). Hashing this lets [`flush_sprixels`] answer "did an equal
/// placement exist last frame?" in O(1) instead of an O(n·m) linear scan.
type SprixelKey = (u64, u32, u32, u32, u32);

/// Build the structural identity key for a placement.
#[inline]
fn sprixel_key(p: &crate::buffer::SprixelPlacement) -> SprixelKey {
    (p.content_hash, p.x, p.y, p.cols, p.rows)
}

/// Decide whether a sprixel placement must be re-blitted this frame, applying
/// the per-cell damage matrix (issue #265).
///
/// Returns `true` when:
///   * the placement is new or its `(x, y, content_hash, cols, rows)` changed
///     (its key is absent from `prev_keys`, the precomputed set of last frame's
///     placement keys), OR
///   * a text cell inside the footprint was overwritten this frame *and* the
///     footprint marks that cell as covering graphic ink
///     ([`SprixelCell::Opaque`] / [`SprixelCell::Mixed`]) — i.e. the cell is
///     [`SprixelCell::Annihilated`].
///
/// A pure text edit landing on a [`SprixelCell::Transparent`] cell never marks
/// damage, so the graphic is not re-emitted.
///
/// The footprint scan short-circuits an entire footprint row when that row was
/// untouched this frame *and* hashes identically to the previous frame
/// (`current.row_clean(y) && current.row_hash(y) == previous.row_hash(y)`):
/// no cell in such a row can have changed, so no ink can have been annihilated.
/// On the headless / direct-call path (where `recompute_line_hashes` was not
/// run) every row reports dirty, so the shortcut never fires and the per-cell
/// scan runs exactly as before — preserving correctness.
fn sprixel_needs_reblit(
    placement: &crate::buffer::SprixelPlacement,
    current: &Buffer,
    previous: &Buffer,
    prev_keys: &std::collections::HashSet<SprixelKey>,
) -> bool {
    use crate::buffer::SprixelCell;

    // Position / content change: re-blit if no equal placement existed last
    // frame. The key mirrors `SprixelPlacement: PartialEq` (content_hash/x/y/
    // cols/rows; damage matrix excluded), so a moved or recolored image
    // re-blits. O(1) lookup vs the former O(n·m) `iter().any(..)` scan.
    if !prev_keys.contains(&sprixel_key(placement)) {
        return true;
    }

    // Annihilation scan: a covered text cell that changed since last frame and
    // now shows ink forces a re-blit. `Transparent` cells are skipped so free
    // text edits in graphic gaps emit zero sprixel bytes.
    for row in 0..placement.rows {
        let y = placement.y + row;
        // Per-row shortcut: a row that was not touched this frame and whose
        // cached digest matches the previous frame's cannot contain a changed
        // cell, so the whole footprint row is skipped without per-cell work.
        if current.row_clean(y) && current.row_hash(y) == previous.row_hash(y) {
            continue;
        }
        for col in 0..placement.cols {
            let idx = (row * placement.cols + col) as usize;
            match placement.cells.get(idx) {
                Some(SprixelCell::Opaque) | Some(SprixelCell::Mixed) => {}
                // Transparent / Annihilated / out-of-range: not ink-covering,
                // so a text write here does not damage the graphic.
                _ => continue,
            }
            let x = placement.x + col;
            // A footprint can extend past the buffer edge (a clipped placement,
            // or `iterm_image_fit` reserving rows beyond the viewport). Use
            // `try_get` so an out-of-bounds footprint cell is simply skipped
            // rather than panicking — there is no text there to annihilate it.
            let (Some(cell), Some(prev)) = (current.try_get(x, y), previous.try_get(x, y)) else {
                continue;
            };
            // Mirror `flush_buffer_diff`'s write predicate exactly: a cell is
            // emitted (and thus overwrites graphic ink) iff it changed since
            // last frame and carries a non-empty symbol. Matching the predicate
            // keeps the damage matrix in lockstep with what the cell diff
            // actually paints over the graphic.
            if cell != prev && !cell.symbol.is_empty() {
                return true;
            }
        }
    }

    false
}

/// Flush the sprixel (Sixel / iTerm2) layer with per-cell damage tracking.
///
/// Unlike [`flush_raw_sequences`]' all-or-nothing guard, this re-emits each
/// pixel graphic **only** when [`sprixel_needs_reblit`] reports damage, so a
/// text edit in a transparent region of a Sixel emits zero passthrough bytes
/// (issue #265).
///
/// The previous frame's placement keys are hashed once up front so the
/// position/content change check is O(1) per placement (vs the former O(n·m)
/// linear scan), and the per-row clean+hash shortcut inside
/// [`sprixel_needs_reblit`] skips untouched footprint rows entirely.
fn flush_sprixels(
    stdout: &mut impl Write,
    current: &Buffer,
    previous: &Buffer,
    row_offset: u32,
) -> io::Result<()> {
    flush_sprixels_inner(stdout, current, previous, row_offset, |_| true)
}

fn flush_sprixels_checked(
    stdout: &mut impl Write,
    current: &Buffer,
    previous: &Buffer,
    row_offset: u32,
    graphics_support: GraphicsEmissionSupport,
) -> io::Result<()> {
    flush_sprixels_inner(stdout, current, previous, row_offset, |placement| {
        graphics_support.should_emit_sprixel(sprixel_protocol(&placement.seq))
    })
}

fn flush_sprixels_inner(
    stdout: &mut impl Write,
    current: &Buffer,
    previous: &Buffer,
    row_offset: u32,
    mut should_emit: impl FnMut(&crate::buffer::SprixelPlacement) -> bool,
) -> io::Result<()> {
    // Early out: no graphics to emit. Avoids building the key set on the
    // common text-only frame.
    if current.sprixels.is_empty() {
        return Ok(());
    }

    let prev_keys: std::collections::HashSet<SprixelKey> =
        previous.sprixels.iter().map(sprixel_key).collect();

    for placement in &current.sprixels {
        if should_emit(placement) && sprixel_needs_reblit(placement, current, previous, &prev_keys)
        {
            queue!(
                stdout,
                cursor::MoveTo(sat_u16(placement.x), sat_u16(row_offset + placement.y)),
                Print(&placement.seq)
            )?;
        }
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
            Some(fg) => emit_fg_color(w, fg, depth)?,
            None => write!(w, "\x1b[39m")?,
        }
    }
    if old.bg != new.bg {
        match new.bg {
            Some(bg) => emit_bg_color(w, bg, depth)?,
            None => write!(w, "\x1b[49m")?,
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
    if removed.contains(Modifiers::BLINK) {
        queue!(w, SetAttribute(Attribute::NoBlink))?;
    }
    if added.contains(Modifiers::BLINK) {
        queue!(w, SetAttribute(Attribute::SlowBlink))?;
    }
    if removed.contains(Modifiers::OVERLINE) {
        queue!(w, SetAttribute(Attribute::NotOverLined))?;
    }
    if added.contains(Modifiers::OVERLINE) {
        queue!(w, SetAttribute(Attribute::OverLined))?;
    }
    // Underline style and color use raw escapes: crossterm 0.28 cannot
    // express the `CSI 4:Nm` subparameters or the `SGR 58`/`59` underline
    // color reliably (its discriminants collide on these terminals).
    if old.underline_style != new.underline_style {
        write!(w, "\x1b[4:{}m", underline_style_param(new.underline_style))?;
    }
    if old.underline_color != new.underline_color {
        emit_underline_color(w, new.underline_color, depth)?;
    }
    Ok(())
}

/// Map an [`UnderlineStyle`] to its `CSI 4:Nm` subparameter value.
fn underline_style_param(style: UnderlineStyle) -> u8 {
    match style {
        UnderlineStyle::Straight => 1,
        UnderlineStyle::Double => 2,
        UnderlineStyle::Curly => 3,
        UnderlineStyle::Dotted => 4,
        UnderlineStyle::Dashed => 5,
    }
}

/// Emit the raw `SGR 58` underline-color sequence (or `SGR 59` to reset).
///
/// `None` resets the underline color to the foreground (`\x1b[59m`). Otherwise
/// the color is downsampled to the terminal's depth: true-color emits
/// `\x1b[58:2::r:g:bm`, while indexed/named colors emit `\x1b[58:5:im`.
fn emit_underline_color(
    w: &mut impl Write,
    color: Option<Color>,
    depth: ColorDepth,
) -> io::Result<()> {
    match color {
        None => write!(w, "\x1b[59m"),
        Some(c) => match c.downsampled(depth) {
            Color::Reset => write!(w, "\x1b[59m"),
            Color::Rgb(r, g, b) => write!(w, "\x1b[58:2::{r}:{g}:{b}m"),
            Color::Indexed(i) => write!(w, "\x1b[58:5:{i}m"),
            // Named colors have no direct SGR-58 form; resolve them to their
            // RGB equivalent and emit a true-color underline sequence.
            named => {
                let (r, g, b) = named.to_rgb();
                write!(w, "\x1b[58:2::{r}:{g}:{b}m")
            }
        },
    }
}

fn apply_style(w: &mut impl Write, style: &Style, depth: ColorDepth) -> io::Result<()> {
    if let Some(fg) = style.fg {
        emit_fg_color(w, fg, depth)?;
    }
    if let Some(bg) = style.bg {
        emit_bg_color(w, bg, depth)?;
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
    if m.contains(Modifiers::BLINK) {
        queue!(w, SetAttribute(Attribute::SlowBlink))?;
    }
    if m.contains(Modifiers::OVERLINE) {
        queue!(w, SetAttribute(Attribute::OverLined))?;
    }
    if style.underline_style != UnderlineStyle::Straight {
        write!(
            w,
            "\x1b[4:{}m",
            underline_style_param(style.underline_style)
        )?;
    }
    if style.underline_color.is_some() {
        emit_underline_color(w, style.underline_color, depth)?;
    }
    Ok(())
}

fn emit_fg_color(w: &mut impl Write, color: Color, depth: ColorDepth) -> io::Result<()> {
    emit_sgr_color(w, color, depth, true)
}

fn emit_bg_color(w: &mut impl Write, color: Color, depth: ColorDepth) -> io::Result<()> {
    emit_sgr_color(w, color, depth, false)
}

fn emit_sgr_color(
    w: &mut impl Write,
    color: Color,
    depth: ColorDepth,
    foreground: bool,
) -> io::Result<()> {
    match color.downsampled(depth) {
        Color::Reset => {
            let reset = if foreground { 39 } else { 49 };
            write!(w, "\x1b[{reset}m")
        }
        Color::Rgb(r, g, b) => {
            let channel = if foreground { 38 } else { 48 };
            write!(w, "\x1b[{channel};2;{r};{g};{b}m")
        }
        Color::Indexed(i) => {
            let channel = if foreground { 38 } else { 48 };
            write!(w, "\x1b[{channel};5;{i}m")
        }
        named => {
            let code = named_sgr_code(named, foreground);
            write!(w, "\x1b[{code}m")
        }
    }
}

fn named_sgr_code(color: Color, foreground: bool) -> u8 {
    let dark_base = if foreground { 30 } else { 40 };
    let bright_base = if foreground { 90 } else { 100 };
    match color {
        Color::Black => dark_base,
        Color::Red => dark_base + 1,
        Color::Green => dark_base + 2,
        Color::Yellow => dark_base + 3,
        Color::Blue => dark_base + 4,
        Color::Magenta => dark_base + 5,
        Color::Cyan => dark_base + 6,
        Color::White => dark_base + 7,
        Color::DarkGray => bright_base,
        Color::LightRed => bright_base + 1,
        Color::LightGreen => bright_base + 2,
        Color::LightYellow => bright_base + 3,
        Color::LightBlue => bright_base + 4,
        Color::LightMagenta => bright_base + 5,
        Color::LightCyan => bright_base + 6,
        Color::LightWhite => bright_base + 7,
        Color::Reset | Color::Rgb(..) | Color::Indexed(_) => unreachable!(),
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
        let _ = write_kitty_keyboard_push(stdout, session.report_all_keys);
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

/// Write Kitty keyboard protocol commands directly instead of routing them
/// through crossterm's legacy Windows API, where these commands are reported
/// as unsupported even when the terminal host accepts ANSI sequences.
fn write_kitty_keyboard_push(stdout: &mut impl Write, report_all_keys: bool) -> io::Result<()> {
    write!(stdout, "\x1b[>{}u", kitty_flags(report_all_keys).bits())
}

fn write_kitty_keyboard_pop(stdout: &mut impl Write) -> io::Result<()> {
    stdout.write_all(b"\x1b[<1u")
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

fn write_session_exit(
    stdout: &mut impl Write,
    mode: TerminalSessionMode,
    inline_reserved: bool,
    mouse_enabled: bool,
    kitty_keyboard: bool,
) -> io::Result<()> {
    if kitty_keyboard {
        write_kitty_keyboard_pop(stdout)?;
    }
    if mouse_enabled {
        // On Windows this uses crossterm's process-global console mode state,
        // which may not have been initialized when panic cleanup runs. Mouse
        // restoration is best-effort so it cannot prevent the core cursor,
        // paste, and screen cleanup below.
        let _ = execute!(stdout, DisableMouseCapture);
    }
    let _ = execute!(stdout, DisableFocusChange);
    write_session_cleanup(stdout, mode, inline_reserved)
}

/// Best-effort terminal restoration used by the panic hook.
///
/// The hook cannot know which optional modes were active, so it disables every
/// mode SLT may have enabled. Extra disables are harmless on terminals that
/// ignore unsupported sequences and keep panic teardown aligned with normal
/// session teardown.
#[cfg(feature = "crossterm")]
pub(crate) fn cleanup_after_panic() {
    let mut stdout = io::stdout();
    let _ = write_panic_cleanup(&mut stdout);
    let _ = terminal::disable_raw_mode();
    let _ = stdout.flush();
}

fn write_panic_cleanup(stdout: &mut impl Write) -> io::Result<()> {
    write_session_exit(stdout, TerminalSessionMode::Fullscreen, false, true, true)
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
    write_session_exit(
        stdout,
        snapshot.mode,
        false,
        snapshot.mouse_enabled,
        snapshot.kitty_keyboard,
    )
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
    let _ = resume_from_shell_with_writer(&mut out, snapshot);
}

#[cfg(unix)]
fn resume_from_shell_with_writer(
    out: &mut impl Write,
    snapshot: &SessionSnapshot,
) -> io::Result<()> {
    let guard = TerminalSessionGuard {
        mode: snapshot.mode,
        mouse_enabled: snapshot.mouse_enabled,
        kitty_keyboard: snapshot.kitty_keyboard,
        report_all_keys: snapshot.report_all_keys,
        harness: false,
    };
    write_session_enter(out, &guard)?;
    out.flush()?;
    NEEDS_FULL_REDRAW.store(true, std::sync::atomic::Ordering::SeqCst);
    Ok(())
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

    /// Feed `bytes` to a channel from a helper thread after `delay`, then run
    /// [`collect_reply`] against it with the given budget and predicate.
    fn collect_with_feed(
        bytes: &'static [u8],
        delay: Duration,
        budget: Duration,
        is_complete: &mut dyn FnMut(&[u8]) -> bool,
    ) -> (Vec<u8>, Duration) {
        let (tx, rx) = std::sync::mpsc::channel::<u8>();
        std::thread::spawn(move || {
            std::thread::sleep(delay);
            for &b in bytes {
                if tx.send(b).is_err() {
                    return;
                }
            }
            // Keep the sender alive past the collector's budget: the real
            // pump thread only drops its sender on stdin EOF, so dropping it
            // here right after the payload would disconnect the channel and
            // end the wait early, masking deadline behavior.
            std::thread::sleep(Duration::from_secs(3));
        });
        let start = Instant::now();
        let out = collect_reply(&rx, start + budget, is_complete);
        (out, start.elapsed())
    }

    #[test]
    fn collect_reply_osc_bel_terminator_completes_early() {
        let reply = b"\x1b]11;rgb:0000/0000/0000\x07";
        let (out, elapsed) = collect_with_feed(
            reply,
            Duration::ZERO,
            Duration::from_secs(2),
            &mut osc_reply_complete,
        );
        assert_eq!(out, reply);
        assert!(
            elapsed < Duration::from_secs(1),
            "should not wait out the budget"
        );
    }

    #[test]
    fn collect_reply_osc_st_terminator_completes_early() {
        let reply = b"\x1bP>|tmux 3.5a\x1b\\";
        let (out, elapsed) = collect_with_feed(
            reply,
            Duration::ZERO,
            Duration::from_secs(2),
            &mut osc_reply_complete,
        );
        assert_eq!(out, reply);
        assert!(elapsed < Duration::from_secs(1));
    }

    #[test]
    fn collect_reply_silence_returns_empty_at_deadline() {
        // The silent-host case that used to deadlock startup: no bytes ever
        // arrive. The collector must give up at the deadline, not block.
        let budget = Duration::from_millis(150);
        let (out, elapsed) =
            collect_with_feed(b"", Duration::from_secs(5), budget, &mut osc_reply_complete);
        assert!(out.is_empty());
        assert!(elapsed >= budget);
        assert!(
            elapsed < Duration::from_secs(2),
            "must not block past the budget"
        );
    }

    #[test]
    fn collect_reply_da_drains_two_replies() {
        let reply = b"\x1b[?62;4c\x1b[>1;10;0c";
        let (out, elapsed) = collect_with_feed(
            reply,
            Duration::ZERO,
            Duration::from_secs(2),
            &mut da_reply_complete(),
        );
        assert_eq!(out, reply);
        assert!(elapsed < Duration::from_secs(1));
    }

    #[test]
    fn collect_reply_da_lone_reply_returns_partial_at_deadline() {
        // A terminal that answers DA1 but ignores DA2: the collector waits out
        // the budget, then hands back the partial reply for best-effort parse
        // (pre-pump behavior, preserved).
        let budget = Duration::from_millis(150);
        let (out, elapsed) = collect_with_feed(
            b"\x1b[?62;4c",
            Duration::ZERO,
            budget,
            &mut da_reply_complete(),
        );
        assert_eq!(out, b"\x1b[?62;4c");
        assert!(elapsed >= budget);
    }

    #[test]
    fn collect_reply_unterminated_caps_at_4096_bytes() {
        static BIG: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
        let big = BIG.get_or_init(|| vec![b'x'; 5000]).as_slice();
        let (tx, rx) = std::sync::mpsc::channel::<u8>();
        for &b in big {
            tx.send(b).unwrap();
        }
        let out = collect_reply(
            &rx,
            Instant::now() + Duration::from_secs(2),
            &mut osc_reply_complete,
        );
        assert_eq!(out.len(), 4096);
    }

    #[test]
    fn decrpm_predicate_terminates_on_y() {
        let reply = b"\x1b[?2026;1$y";
        let (out, _) = collect_with_feed(
            reply,
            Duration::ZERO,
            Duration::from_secs(2),
            &mut decrpm_reply_complete,
        );
        assert_eq!(out, reply);
    }

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
            harness: false,
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
            harness: false,
        };
        let mut out = Vec::new();
        write_session_enter(&mut out, &session).unwrap();
        let output = String::from_utf8(out).unwrap();
        assert!(!output.contains("\u{1b}[?1049h"));
        assert!(output.contains("\u{1b}[?25l"));
        assert!(output.contains("\u{1b}[?2004h"));
    }

    #[test]
    fn session_enter_writes_kitty_keyboard_flags_portably() {
        let session = TerminalSessionGuard {
            mode: TerminalSessionMode::Fullscreen,
            mouse_enabled: false,
            kitty_keyboard: true,
            report_all_keys: true,
            harness: false,
        };
        let mut out = Vec::new();
        write_session_enter(&mut out, &session).unwrap();
        let output = String::from_utf8(out).unwrap();
        let expected = format!("\u{1b}[>{}u", kitty_flags(true).bits());
        assert!(output.contains(&expected));
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

    #[test]
    fn session_exit_disables_focus_mouse_and_kitty_keyboard() {
        let mut out = Vec::new();
        write_session_exit(&mut out, TerminalSessionMode::Fullscreen, false, true, true).unwrap();
        let output = String::from_utf8(out).unwrap();
        // Crossterm manages mouse/focus through the Windows console API rather
        // than writing those escape sequences to the supplied writer.
        #[cfg(not(windows))]
        assert!(output.contains("\u{1b}[?1004l"), "disables focus reporting");
        #[cfg(not(windows))]
        assert!(output.contains("\u{1b}[?1006l"), "disables SGR mouse mode");
        assert!(output.contains("\u{1b}[<1u"), "pops Kitty keyboard flags");
        assert!(output.contains("\u{1b}[?1049l"), "leaves alt screen");
    }

    #[test]
    fn panic_cleanup_uses_full_session_exit_path() {
        let mut out = Vec::new();
        write_panic_cleanup(&mut out).unwrap();
        let output = String::from_utf8(out).unwrap();
        #[cfg(not(windows))]
        assert!(output.contains("\u{1b}[?1004l"), "disables focus reporting");
        assert!(output.contains("\u{1b}[<1u"), "pops Kitty keyboard flags");
        assert!(output.contains("\u{1b}[?1049l"), "leaves alt screen");
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
            report_all_keys: snapshot.report_all_keys,
            harness: false,
        };
        let mut enter_bytes = Vec::new();
        write_session_enter(&mut enter_bytes, &guard).unwrap();
        let enter = String::from_utf8(enter_bytes).unwrap();
        assert!(enter.contains("\u{1b}[?1049h"));
        assert!(enter.contains("\u{1b}[?25l"));
        assert!(enter.contains("\u{1b}[?2004h"));

        // Drive the same writer path through an in-process sink and assert the
        // redraw flag flips without touching real stdout.
        NEEDS_FULL_REDRAW.store(false, std::sync::atomic::Ordering::SeqCst);
        let mut resume_bytes = Vec::new();
        resume_from_shell_with_writer(&mut resume_bytes, &snapshot).unwrap();
        assert_eq!(String::from_utf8(resume_bytes).unwrap(), enter);
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

    static ENV_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[allow(unsafe_code)]
    fn with_terminal_env<F: FnOnce()>(
        term: Option<&str>,
        term_program: Option<&str>,
        tmux: Option<&str>,
        sty: Option<&str>,
        f: F,
    ) {
        let _guard = ENV_GUARD.lock().unwrap_or_else(|err| err.into_inner());
        let prev_term = std::env::var("TERM").ok();
        let prev_program = std::env::var("TERM_PROGRAM").ok();
        let prev_tmux = std::env::var("TMUX").ok();
        let prev_sty = std::env::var("STY").ok();

        unsafe {
            match term {
                Some(value) => std::env::set_var("TERM", value),
                None => std::env::remove_var("TERM"),
            }
            match term_program {
                Some(value) => std::env::set_var("TERM_PROGRAM", value),
                None => std::env::remove_var("TERM_PROGRAM"),
            }
            match tmux {
                Some(value) => std::env::set_var("TMUX", value),
                None => std::env::remove_var("TMUX"),
            }
            match sty {
                Some(value) => std::env::set_var("STY", value),
                None => std::env::remove_var("STY"),
            }
        }

        f();

        unsafe {
            match prev_term {
                Some(value) => std::env::set_var("TERM", value),
                None => std::env::remove_var("TERM"),
            }
            match prev_program {
                Some(value) => std::env::set_var("TERM_PROGRAM", value),
                None => std::env::remove_var("TERM_PROGRAM"),
            }
            match prev_tmux {
                Some(value) => std::env::set_var("TMUX", value),
                None => std::env::remove_var("TMUX"),
            }
            match prev_sty {
                Some(value) => std::env::set_var("STY", value),
                None => std::env::remove_var("STY"),
            }
        }
    }

    #[test]
    fn multiplexers_disable_graphics_env_fallbacks() {
        with_terminal_env(
            Some("tmux-256color"),
            Some("WezTerm"),
            Some("/tmp/tmux"),
            None,
            || {
                assert!(terminal_is_multiplexed());
                assert!(!term_is_kitty_graphics_host());
                assert!(!term_is_sixel_host());
                assert!(!term_is_iterm_host());
            },
        );
        with_terminal_env(
            Some("screen-256color"),
            Some("iTerm.app"),
            None,
            Some("1234.pts"),
            || {
                assert!(terminal_is_multiplexed());
                assert!(!term_is_kitty_graphics_host());
                assert!(!term_is_sixel_host());
                assert!(!term_is_iterm_host());
            },
        );
    }

    #[test]
    fn direct_hosts_keep_graphics_env_fallbacks() {
        with_terminal_env(Some("xterm-kitty"), None, None, None, || {
            assert!(!terminal_is_multiplexed());
            assert!(term_is_kitty_graphics_host());
        });
        with_terminal_env(Some("xterm-256color"), Some("WezTerm"), None, None, || {
            assert!(!terminal_is_multiplexed());
            assert!(term_is_sixel_host());
        });
        with_terminal_env(
            Some("xterm-256color"),
            Some("iTerm.app"),
            None,
            None,
            || {
                assert!(!terminal_is_multiplexed());
                assert!(term_is_iterm_host());
            },
        );
    }

    #[test]
    fn graphics_emission_requires_protocol_support() {
        let unsupported = GraphicsEmissionSupport {
            real_terminal: true,
            capabilities: Capabilities::default(),
            force_kitty: false,
            force_sixel: false,
            force_iterm: false,
        };
        assert!(!unsupported.should_emit_kitty());
        assert!(!unsupported.should_emit_sprixel(SprixelProtocol::Sixel));
        assert!(!unsupported.should_emit_sprixel(SprixelProtocol::Iterm2));
        assert!(!unsupported.should_emit_sprixel(SprixelProtocol::Unknown));

        let sixel = GraphicsEmissionSupport {
            capabilities: Capabilities {
                sixel: true,
                ..Capabilities::default()
            },
            ..unsupported
        };
        assert!(sixel.should_emit_sprixel(SprixelProtocol::Sixel));

        let forced = GraphicsEmissionSupport {
            force_kitty: true,
            force_iterm: true,
            ..unsupported
        };
        assert!(forced.should_emit_kitty());
        assert!(forced.should_emit_sprixel(SprixelProtocol::Iterm2));
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

    // ---- Capability probe / blitter ladder (issue #264) ----

    #[test]
    fn blitter_support_default_is_conservative() {
        let b = BlitterSupport::default();
        assert!(b.half);
        assert!(b.quad);
        assert!(!b.sextant);
    }

    #[test]
    fn capabilities_default_is_all_false_but_half_block() {
        let c = Capabilities::default();
        assert!(!c.truecolor);
        assert!(!c.sixel);
        assert!(!c.iterm2);
        assert!(!c.kitty_graphics);
        assert!(!c.kitty_keyboard);
        assert!(!c.sync_output);
        // With nothing negotiated the ladder must still resolve to half-block.
        assert_eq!(c.best_blitter(), Blitter::HalfBlock);
    }

    #[test]
    fn best_blitter_ladder_table() {
        let kitty = Capabilities {
            kitty_graphics: true,
            ..Default::default()
        };
        assert_eq!(kitty.best_blitter(), Blitter::Kitty);

        let sixel = Capabilities {
            sixel: true,
            ..Default::default()
        };
        assert_eq!(sixel.best_blitter(), Blitter::Sixel);

        let iterm2 = Capabilities {
            iterm2: true,
            ..Default::default()
        };
        assert_eq!(iterm2.best_blitter(), Blitter::Iterm2);

        // iTerm2 sits below Sixel: a host advertising both prefers Sixel.
        let sixel_and_iterm2 = Capabilities {
            sixel: true,
            iterm2: true,
            ..Default::default()
        };
        assert_eq!(sixel_and_iterm2.best_blitter(), Blitter::Sixel);

        let sextant = Capabilities {
            blitters: BlitterSupport {
                sextant: true,
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(sextant.best_blitter(), Blitter::Sextant);

        assert_eq!(Capabilities::default().best_blitter(), Blitter::HalfBlock);
    }

    #[test]
    fn best_blitter_precedence_kitty_over_everything() {
        let all = Capabilities {
            kitty_graphics: true,
            sixel: true,
            blitters: BlitterSupport {
                sextant: true,
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(all.best_blitter(), Blitter::Kitty);

        let sixel_and_sextant = Capabilities {
            sixel: true,
            blitters: BlitterSupport {
                sextant: true,
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(sixel_and_sextant.best_blitter(), Blitter::Sixel);
    }

    #[test]
    fn best_blitter_never_picks_unsupported_protocol() {
        // Exhaustive sweep over field combinations: the resolver must never
        // return Kitty without kitty_graphics, nor Sixel without sixel, etc.
        for kitty in [false, true] {
            for sixel in [false, true] {
                for iterm2 in [false, true] {
                    for sextant in [false, true] {
                        let caps = Capabilities {
                            kitty_graphics: kitty,
                            sixel,
                            iterm2,
                            blitters: BlitterSupport {
                                sextant,
                                ..Default::default()
                            },
                            ..Default::default()
                        };
                        match caps.best_blitter() {
                            Blitter::Kitty => assert!(kitty),
                            Blitter::Sixel => assert!(sixel && !kitty),
                            Blitter::Iterm2 => assert!(iterm2 && !sixel && !kitty),
                            Blitter::Sextant => {
                                assert!(sextant && !iterm2 && !sixel && !kitty)
                            }
                            Blitter::HalfBlock => {
                                assert!(!kitty && !sixel && !iterm2 && !sextant)
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(feature = "crossterm")]
    #[test]
    fn parse_da1_attribute_4_sets_sixel() {
        let mut caps = Capabilities::default();
        parse_da1("\x1b[?62;4;6c", &mut caps);
        assert!(caps.sixel);
    }

    #[cfg(feature = "crossterm")]
    #[test]
    fn parse_da1_without_4_leaves_sixel_false() {
        let mut caps = Capabilities::default();
        parse_da1("\x1b[?62;1;6c", &mut caps);
        assert!(!caps.sixel);
    }

    #[cfg(feature = "crossterm")]
    #[test]
    fn parse_da1_ignores_da2_segment_in_same_string() {
        // DA1 (no `4`) followed by DA2 — DA2 must not be mistaken for DA1.
        let mut caps = Capabilities::default();
        parse_da1("\x1b[?62;1c\x1b[>0;276;0c", &mut caps);
        assert!(!caps.sixel);
    }

    #[cfg(feature = "crossterm")]
    #[test]
    fn parse_da2_no_panic_on_garbage() {
        let mut caps = Capabilities::default();
        // Must not panic and must not set kitty_graphics on an unknown id.
        parse_da2("\x1b[>99;1;0c", &mut caps);
        assert!(!caps.kitty_graphics);
        parse_da2("not a da2 reply", &mut caps);
        assert!(!caps.kitty_graphics);
    }

    #[cfg(feature = "crossterm")]
    #[test]
    fn parse_da2_kitty_id_sets_kitty_graphics() {
        let mut caps = Capabilities::default();
        // Kitty reports DA2 primary id 41.
        parse_da2("\x1b[>41;4000;0c", &mut caps);
        assert!(caps.kitty_graphics);
    }

    #[cfg(feature = "crossterm")]
    #[test]
    fn parse_da2_identity_extracts_id_and_version() {
        assert_eq!(parse_da2_identity("\x1b[>0;276;0c"), Some((0, 276)));
        assert_eq!(parse_da2_identity("\x1b[>41;4000;0c"), Some((41, 4000)));
        assert_eq!(parse_da2_identity("no reply here"), None);
    }

    #[cfg(feature = "crossterm")]
    #[test]
    fn parse_kitty_graphics_ack_ok_sets_flag() {
        let mut caps = Capabilities::default();
        parse_kitty_graphics_ack("\x1b_Gi=31;OK\x1b\\", &mut caps);
        assert!(caps.kitty_graphics);
    }

    #[cfg(feature = "crossterm")]
    #[test]
    fn parse_kitty_graphics_ack_error_or_wrong_id_leaves_flag() {
        let mut caps = Capabilities::default();
        // Error status must not flag support.
        parse_kitty_graphics_ack("\x1b_Gi=31;ENOENT:bad\x1b\\", &mut caps);
        assert!(!caps.kitty_graphics);
        // A different image id is not our query.
        parse_kitty_graphics_ack("\x1b_Gi=99;OK\x1b\\", &mut caps);
        assert!(!caps.kitty_graphics);
        // No APC at all.
        parse_kitty_graphics_ack("garbage", &mut caps);
        assert!(!caps.kitty_graphics);
    }

    #[cfg(feature = "crossterm")]
    #[test]
    fn parse_decrpm_sync_output_recognized_states_are_supported() {
        // Ps = 1 (set), 2 (reset), 3 (perm set), 4 (perm reset) all mean the
        // mode is recognized → supported.
        assert_eq!(parse_decrpm_sync_output("\x1b[?2026;1$y"), Some(true));
        assert_eq!(parse_decrpm_sync_output("\x1b[?2026;2$y"), Some(true));
        assert_eq!(parse_decrpm_sync_output("\x1b[?2026;3$y"), Some(true));
        assert_eq!(parse_decrpm_sync_output("\x1b[?2026;4$y"), Some(true));
    }

    #[cfg(feature = "crossterm")]
    #[test]
    fn parse_decrpm_sync_output_ps0_is_unsupported() {
        // Ps = 0 → mode not recognized.
        assert_eq!(parse_decrpm_sync_output("\x1b[?2026;0$y"), Some(false));
    }

    #[cfg(feature = "crossterm")]
    #[test]
    fn parse_decrpm_sync_output_garbage_is_none() {
        // No DECRPM reply for mode 2026 in the string → inconclusive.
        assert_eq!(parse_decrpm_sync_output("not a decrpm reply"), None);
        // A reply for a *different* mode must not match.
        assert_eq!(parse_decrpm_sync_output("\x1b[?2004;1$y"), None);
        // Truncated reply (missing `$y` terminator) → None, not a panic.
        assert_eq!(parse_decrpm_sync_output("\x1b[?2026;1"), None);
        // Non-numeric Ps → None.
        assert_eq!(parse_decrpm_sync_output("\x1b[?2026;x$y"), None);
    }

    #[test]
    fn sync_output_gate_defaults_to_emit() {
        // With the probe never having run (the unit-test process never enters a
        // real terminal session), the resolution stays `Unknown`, so the gate
        // must keep emitting BSU/ESU — preserving the historic always-emit
        // behavior on headless / non-answering hosts.
        assert!(should_emit_synchronized_update());
    }

    #[test]
    fn terminal_query_guard_rejects_unsafe_hosts() {
        assert!(terminal_query_allowed(
            true,
            true,
            "xterm-256color",
            false,
            false,
            false
        ));
        assert!(!terminal_query_allowed(
            true,
            false,
            "xterm-256color",
            false,
            false,
            false
        ));
        assert!(!terminal_query_allowed(
            false,
            true,
            "xterm-256color",
            false,
            false,
            false
        ));
        assert!(!terminal_query_allowed(
            true, true, "dumb", false, false, false
        ));
        assert!(!terminal_query_allowed(
            true,
            true,
            "screen-256color",
            true,
            false,
            false
        ));
        assert!(!terminal_query_allowed(true, true, "", false, false, false));
    }

    #[test]
    fn terminal_query_guard_honors_force_and_disable_precedence() {
        assert!(terminal_query_allowed(
            true, true, "dumb", true, true, false
        ));
        assert!(!terminal_query_allowed(
            true,
            true,
            "xterm-kitty",
            false,
            true,
            true
        ));
    }

    #[test]
    fn automatic_query_hosts_require_a_real_terminal_identity() {
        assert!(!terminal_query_host_is_identified_env(
            "xterm-256color",
            false
        ));
        assert!(terminal_query_host_is_identified_env(
            "xterm-256color",
            true
        ));
        assert!(terminal_query_host_is_identified_env("xterm-kitty", false));
        assert!(terminal_query_host_is_identified_env("foot", false));
    }

    #[test]
    fn terminal_multiplexer_detection_is_conservative() {
        assert!(terminal_is_multiplexed_env("tmux-256color", false, false));
        assert!(terminal_is_multiplexed_env("screen-256color", false, false));
        assert!(terminal_is_multiplexed_env("xterm-256color", true, false));
        assert!(terminal_is_multiplexed_env("xterm-256color", false, true));
        assert!(!terminal_is_multiplexed_env("xterm-kitty", false, false));
    }

    #[test]
    fn kitty_env_fallback_is_blocked_inside_multiplexer_unless_forced() {
        assert!(term_is_kitty_graphics_host_env(
            "xterm-kitty",
            "",
            false,
            false
        ));
        assert!(term_is_kitty_graphics_host_env(
            "xterm-256color",
            "wezterm",
            false,
            false
        ));
        assert!(!term_is_kitty_graphics_host_env(
            "xterm-kitty",
            "wezterm",
            true,
            false
        ));
        assert!(term_is_kitty_graphics_host_env(
            "xterm-256color",
            "",
            true,
            true
        ));
    }

    #[test]
    fn iterm_env_fallback_is_blocked_inside_multiplexer_unless_forced() {
        assert!(term_is_iterm_host_env("iterm.app", false, false));
        assert!(term_is_iterm_host_env("wezterm", false, false));
        assert!(!term_is_iterm_host_env("wezterm", true, false));
        assert!(term_is_iterm_host_env("xterm", true, true));
    }

    #[test]
    fn graphics_support_blocks_kitty_without_ack_or_force() {
        let support = GraphicsEmissionSupport {
            real_terminal: true,
            capabilities: Capabilities::default(),
            force_kitty: false,
            force_sixel: false,
            force_iterm: false,
        };
        assert!(!support.should_emit_kitty());

        let acked = GraphicsEmissionSupport {
            capabilities: Capabilities {
                kitty_graphics: true,
                ..Default::default()
            },
            ..support
        };
        assert!(acked.should_emit_kitty());

        let forced = GraphicsEmissionSupport {
            force_kitty: true,
            ..support
        };
        assert!(forced.should_emit_kitty());

        let captured = GraphicsEmissionSupport {
            real_terminal: false,
            force_kitty: true,
            ..support
        };
        assert!(!captured.should_emit_kitty());
    }

    #[test]
    fn graphics_support_blocks_sprixels_without_ack_or_force() {
        let support = GraphicsEmissionSupport {
            real_terminal: true,
            capabilities: Capabilities::default(),
            force_kitty: false,
            force_sixel: false,
            force_iterm: false,
        };
        assert!(!support.should_emit_sprixel(SprixelProtocol::Sixel));
        assert!(!support.should_emit_sprixel(SprixelProtocol::Iterm2));
        assert!(!support.should_emit_sprixel(SprixelProtocol::Unknown));

        let sixel_acked = GraphicsEmissionSupport {
            capabilities: Capabilities {
                sixel: true,
                ..Default::default()
            },
            ..support
        };
        assert!(sixel_acked.should_emit_sprixel(SprixelProtocol::Sixel));

        let iterm_forced = GraphicsEmissionSupport {
            force_iterm: true,
            ..support
        };
        assert!(iterm_forced.should_emit_sprixel(SprixelProtocol::Iterm2));
    }

    #[test]
    fn sprixel_protocol_detects_sixel_and_iterm() {
        assert_eq!(
            sprixel_protocol("\x1bPqpayload\x1b\\"),
            SprixelProtocol::Sixel
        );
        assert_eq!(
            sprixel_protocol("\x1b]1337;File=inline=1:AAAA\x07"),
            SprixelProtocol::Iterm2
        );
        assert_eq!(sprixel_protocol("plain"), SprixelProtocol::Unknown);
    }

    #[cfg(feature = "crossterm")]
    #[test]
    fn parse_xtgettcap_tc_sets_truecolor() {
        let mut caps = Capabilities::default();
        // DCS 1 + r 5463 (=Tc) ST → truecolor present.
        parse_xtgettcap_truecolor("\x1bP1+r5463=\x1b\\", &mut caps);
        assert!(caps.truecolor);
    }

    #[cfg(feature = "crossterm")]
    #[test]
    fn parse_xtgettcap_invalid_leaves_truecolor_false() {
        let mut caps = Capabilities::default();
        // DCS 0 + r (capability NOT present) must not set the flag.
        parse_xtgettcap_truecolor("\x1bP0+r5463\x1b\\", &mut caps);
        assert!(!caps.truecolor);
        // Wrong capname hex must not match.
        parse_xtgettcap_truecolor("\x1bP1+r1234=\x1b\\", &mut caps);
        assert!(!caps.truecolor);
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
        flush_buffer_diff(
            &mut out,
            &current,
            &previous,
            ColorDepth::TrueColor,
            0,
            &mut String::new(),
        )
        .unwrap();
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
        flush_buffer_diff(
            &mut out,
            &current,
            &previous,
            ColorDepth::TrueColor,
            0,
            &mut String::new(),
        )
        .unwrap();
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
        flush_buffer_diff(
            &mut out,
            &current,
            &previous,
            ColorDepth::TrueColor,
            0,
            &mut String::new(),
        )
        .unwrap();
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
        flush_buffer_diff(
            &mut direct,
            &current,
            &previous,
            ColorDepth::TrueColor,
            0,
            &mut String::new(),
        )
        .unwrap();

        let mut buffered: BufWriter<Vec<u8>> = BufWriter::with_capacity(65536, Vec::new());
        flush_buffer_diff(
            &mut buffered,
            &current,
            &previous,
            ColorDepth::TrueColor,
            0,
            &mut String::new(),
        )
        .unwrap();
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
        flush_buffer_diff(
            &mut bw,
            &current,
            &previous,
            ColorDepth::TrueColor,
            0,
            &mut String::new(),
        )
        .unwrap();
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
        flush_buffer_diff(
            &mut out,
            &current,
            &previous,
            ColorDepth::TrueColor,
            0,
            &mut String::new(),
        )
        .unwrap();
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
        flush_buffer_diff(
            &mut out,
            &current,
            &previous,
            ColorDepth::TrueColor,
            0,
            &mut String::new(),
        )
        .unwrap();
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

    fn delta_bytes(old: &Style, new: &Style) -> Vec<u8> {
        let mut out = Vec::new();
        apply_style_delta(&mut out, old, new, ColorDepth::TrueColor).unwrap();
        out
    }

    fn contains_seq(haystack: &[u8], needle: &[u8]) -> bool {
        haystack.windows(needle.len()).any(|w| w == needle)
    }

    #[test]
    fn apply_style_delta_emits_blink_set_and_reset() {
        let on = delta_bytes(&Style::new(), &Style::new().blink());
        // SGR 5 = SlowBlink.
        assert!(contains_seq(&on, b"\x1b[5m"), "blink set: {on:?}");
        let off = delta_bytes(&Style::new().blink(), &Style::new());
        // SGR 25 = NoBlink.
        assert!(contains_seq(&off, b"\x1b[25m"), "blink reset: {off:?}");
    }

    #[test]
    fn apply_style_delta_emits_overline_set_and_reset() {
        let on = delta_bytes(&Style::new(), &Style::new().overline());
        // SGR 53 = OverLined.
        assert!(contains_seq(&on, b"\x1b[53m"), "overline set: {on:?}");
        let off = delta_bytes(&Style::new().overline(), &Style::new());
        // SGR 55 = NotOverLined.
        assert!(contains_seq(&off, b"\x1b[55m"), "overline reset: {off:?}");
    }

    #[test]
    fn apply_style_delta_emits_curly_underline_subparameter() {
        let out = delta_bytes(
            &Style::new(),
            &Style::new().underline_style(UnderlineStyle::Curly),
        );
        assert!(contains_seq(&out, b"\x1b[4:3m"), "curly underline: {out:?}");
    }

    #[test]
    fn apply_style_delta_emits_underline_color_and_reset() {
        let set = delta_bytes(
            &Style::new(),
            &Style::new().underline_color(Color::Rgb(255, 0, 0)),
        );
        assert!(
            contains_seq(&set, b"\x1b[58:2::255:0:0m"),
            "underline color set: {set:?}"
        );
        let clear = delta_bytes(
            &Style::new().underline_color(Color::Rgb(255, 0, 0)),
            &Style::new(),
        );
        assert!(
            contains_seq(&clear, b"\x1b[59m"),
            "underline color reset: {clear:?}"
        );
    }

    #[test]
    fn apply_style_delta_underline_color_indexed_uses_sgr_58_5() {
        let out = delta_bytes(
            &Style::new(),
            &Style::new().underline_color(Color::Indexed(42)),
        );
        assert!(
            contains_seq(&out, b"\x1b[58:5:42m"),
            "indexed underline: {out:?}"
        );
    }

    #[test]
    fn apply_style_full_emits_blink_overline_and_underline() {
        let mut out = Vec::new();
        let style = Style::new()
            .blink()
            .overline()
            .underline_style(UnderlineStyle::Dotted)
            .underline_color(Color::Rgb(0, 0, 255));
        apply_style(&mut out, &style, ColorDepth::TrueColor).unwrap();
        assert!(contains_seq(&out, b"\x1b[5m"), "blink: {out:?}");
        assert!(contains_seq(&out, b"\x1b[53m"), "overline: {out:?}");
        assert!(
            contains_seq(&out, b"\x1b[4:4m"),
            "dotted underline: {out:?}"
        );
        assert!(
            contains_seq(&out, b"\x1b[58:2::0:0:255m"),
            "underline color: {out:?}"
        );
    }
    /// Issue #274: a captured-sink `Terminal` routes a styled cell through the
    /// real flush pipeline into the in-process byte sink, and dropping it does
    /// not emit teardown escapes (no raw mode was entered).
    #[test]
    fn with_sink_captures_flush_bytes_and_drops_clean() {
        let mut term = Terminal::with_sink(10, 1, ColorDepth::TrueColor);
        term.buffer_mut()
            .set_string(0, 0, "Z", Style::new().fg(Color::Rgb(200, 50, 50)));
        term.flush().unwrap();
        let bytes = term.take_sink_bytes();
        let s = String::from_utf8_lossy(&bytes);
        // Real terminal control bytes + the printed glyph went to the sink.
        assert!(s.contains("\u{1b}["), "missing CSI: {s:?}");
        assert!(s.contains('Z'), "missing glyph: {s:?}");
        // A second take after no flush yields nothing (capture was drained).
        assert!(term.take_sink_bytes().is_empty());
        // Dropping the harness terminal must not panic or emit teardown.
        drop(term);
    }

    /// Issue #269: hoisting `run_buf` to a reused, caller-owned buffer must not
    /// change the emitted bytes. Re-running the diff twice through the *same*
    /// `run_buf` (which `clear()`s but keeps capacity at the top of each call)
    /// produces the same output as a single fresh-buffer run.
    #[test]
    fn reused_run_buf_byte_identical_across_frames() {
        let area = Rect::new(0, 0, 12, 2);
        // `Buffer` is not `Clone`, so rebuild the frame pair on demand.
        let make_frame = || {
            let mut current = Buffer::empty(area);
            let previous = Buffer::empty(area);
            current.set_string(0, 0, "hello world", Style::new().fg(Color::Rgb(1, 2, 3)));
            current.set_string(0, 1, "second line", Style::new().fg(Color::Rgb(4, 5, 6)));
            (current, previous)
        };

        // Baseline: a fresh run_buf per call.
        let mut baseline: Vec<u8> = Vec::new();
        {
            let (mut a, mut b) = make_frame();
            __bench_flush_buffer_diff_mut_with_buf(
                &mut baseline,
                &mut a,
                &mut b,
                ColorDepth::TrueColor,
                &mut String::with_capacity(RUN_BUF_INITIAL_CAPACITY),
            )
            .unwrap();
        }

        // Reuse: run a throwaway frame first, then the real frame through the
        // SAME run_buf (now carrying leftover capacity, freshly cleared).
        let mut shared = String::with_capacity(RUN_BUF_INITIAL_CAPACITY);
        {
            let mut warm: Vec<u8> = Vec::new();
            let (mut a, mut b) = make_frame();
            __bench_flush_buffer_diff_mut_with_buf(
                &mut warm,
                &mut a,
                &mut b,
                ColorDepth::TrueColor,
                &mut shared,
            )
            .unwrap();
        }
        let cap_after_warm = shared.capacity();

        let mut reused: Vec<u8> = Vec::new();
        let (mut current, mut previous) = make_frame();
        __bench_flush_buffer_diff_mut_with_buf(
            &mut reused,
            &mut current,
            &mut previous,
            ColorDepth::TrueColor,
            &mut shared,
        )
        .unwrap();

        assert_eq!(
            baseline, reused,
            "reused run_buf must emit byte-identical output"
        );
        // The reuse path keeps capacity across frames (never re-grows below the
        // initial reservation) — the whole point of the hoist.
        assert!(
            shared.capacity() >= cap_after_warm,
            "run_buf capacity must persist across frames"
        );
    }

    /// Issue #269: the OSC 8 hyperlink open, rewritten from `format!` to three
    /// borrowed `Print`s, must still emit the exact `\x1b]8;;<url>\x07 ...
    /// \x1b]8;;\x07` sequence.
    #[test]
    fn osc8_hyperlink_emitted_verbatim_after_write_rewrite() {
        let area = Rect::new(0, 0, 8, 1);
        let mut current = Buffer::empty(area);
        let previous = Buffer::empty(area);
        let url = "https://example.com/x";
        // `set_string_linked` sanitizes + attaches the hyperlink to each cell.
        current.set_string_linked(0, 0, "link", Style::new(), url);

        let mut out: Vec<u8> = Vec::new();
        flush_buffer_diff(
            &mut out,
            &current,
            &previous,
            ColorDepth::TrueColor,
            0,
            &mut String::new(),
        )
        .unwrap();

        let open = format!("\x1b]8;;{url}\x07");
        assert!(
            contains_seq(&out, open.as_bytes()),
            "OSC 8 open must appear verbatim: {:?}",
            String::from_utf8_lossy(&out)
        );
        assert!(
            contains_seq(&out, b"\x1b]8;;\x07"),
            "OSC 8 close must appear: {:?}",
            String::from_utf8_lossy(&out)
        );
    }

    /// Build `n` distinct 8x8 RGBA placements for kitty-flush golden tests.
    fn kitty_placements(n: usize) -> Vec<KittyPlacement> {
        (0..n)
            .map(|i| {
                let mut rgba = vec![0u8; 256];
                rgba[0] = i as u8;
                let content_hash = crate::buffer::hash_rgba(&rgba);
                KittyPlacement {
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
                }
            })
            .collect()
    }

    #[test]
    fn captured_sink_suppresses_kitty_graphics_bytes() {
        let mut term = Terminal::with_sink(8, 4, ColorDepth::TrueColor);
        term.graphics_support = GraphicsEmissionSupport {
            real_terminal: true,
            capabilities: Capabilities::default(),
            force_kitty: false,
            force_sixel: false,
            force_iterm: false,
        };
        for placement in kitty_placements(1) {
            term.buffer_mut().kitty_place(placement);
        }
        term.flush().unwrap();
        let bytes = term.take_sink_bytes();
        assert!(
            !contains_seq(&bytes, b"\x1b_G"),
            "captured sink must not emit Kitty APC bytes: {:?}",
            String::from_utf8_lossy(&bytes)
        );
    }

    /// Issue #269: replacing the two per-frame `HashSet`s in
    /// `KittyImageManager::flush` with reused `SmallVec` dedup scratch must not
    /// change the emitted escape stream for the small placement counts (0, 1, 5)
    /// the path actually sees. We assert structural invariants of the byte
    /// stream rather than an opaque golden blob so the test documents intent.
    #[test]
    fn kitty_flush_smallvec_dedup_matches_for_small_n() {
        for n in [0usize, 1, 5] {
            let placements = kitty_placements(n);
            let mut mgr = KittyImageManager::new();

            // Frame 1: nothing previously placed → upload + place each image.
            let mut frame1: Vec<u8> = Vec::new();
            mgr.flush(&mut frame1, &placements, 0).unwrap();
            let s1 = String::from_utf8_lossy(&frame1);
            // One transmit (`a=t`) and one placement (`a=p`) per image.
            assert_eq!(
                s1.matches("a=t,").count(),
                n,
                "n={n}: expected {n} uploads in frame 1: {s1:?}"
            );
            assert_eq!(
                s1.matches("a=p,").count(),
                n,
                "n={n}: expected {n} placements in frame 1: {s1:?}"
            );

            // Frame 2: identical placements → fast path, zero output.
            let mut frame2: Vec<u8> = Vec::new();
            mgr.flush(&mut frame2, &placements, 0).unwrap();
            assert!(
                frame2.is_empty(),
                "n={n}: identical frame must hit the kitty fast path, got {} bytes",
                frame2.len()
            );

            // Frame 3: clear all placements → one delete (`a=d,d=i`) per image,
            // deduped by the reused SmallVec, plus image-data cleanup
            // (`a=d,d=I`) for every now-unused upload.
            let mut frame3: Vec<u8> = Vec::new();
            mgr.flush(&mut frame3, &[], 0).unwrap();
            let s3 = String::from_utf8_lossy(&frame3);
            assert_eq!(
                s3.matches("a=d,d=i,").count(),
                n,
                "n={n}: expected {n} placement deletes in frame 3: {s3:?}"
            );
            assert_eq!(
                s3.matches("a=d,d=I,").count(),
                n,
                "n={n}: expected {n} image-data deletes in frame 3: {s3:?}"
            );
        }
    }

    // ---- #265 sprixel damage matrix ----------------------------------------

    use crate::buffer::{SprixelCell, SprixelPlacement};

    /// Build a 2×2-cell sprixel at (1, 1) with the given footprint states.
    fn make_sprixel(cells: Vec<SprixelCell>) -> SprixelPlacement {
        SprixelPlacement {
            content_hash: 0xABCD,
            seq: "<SIXEL>".to_string(),
            x: 1,
            y: 1,
            cols: 2,
            rows: 2,
            cells,
        }
    }

    #[test]
    fn checked_sprixel_flush_suppresses_mux_without_ack_or_force() {
        let area = Rect::new(0, 0, 10, 5);
        let mut placement = make_sprixel(vec![SprixelCell::Opaque; 4]);
        placement.seq = "\x1bPqpayload\x1b\\".to_string();

        let mut current = Buffer::empty(area);
        current.sprixels.push(placement);
        let previous = Buffer::empty(area);
        let support = GraphicsEmissionSupport {
            real_terminal: true,
            capabilities: Capabilities::default(),
            force_kitty: false,
            force_sixel: false,
            force_iterm: false,
        };

        let mut out = Vec::new();
        flush_sprixels_checked(&mut out, &current, &previous, 0, support).unwrap();
        assert!(
            out.is_empty(),
            "tmux/screen without ack must not emit Sixel"
        );
    }

    #[test]
    fn checked_sprixel_flush_allows_mux_with_probe_ack() {
        let area = Rect::new(0, 0, 10, 5);
        let mut placement = make_sprixel(vec![SprixelCell::Opaque; 4]);
        placement.seq = "\x1bPqpayload\x1b\\".to_string();

        let mut current = Buffer::empty(area);
        current.sprixels.push(placement);
        let previous = Buffer::empty(area);
        let support = GraphicsEmissionSupport {
            real_terminal: true,
            capabilities: Capabilities {
                sixel: true,
                ..Default::default()
            },
            force_kitty: false,
            force_sixel: false,
            force_iterm: false,
        };

        let mut out = Vec::new();
        flush_sprixels_checked(&mut out, &current, &previous, 0, support).unwrap();
        assert!(
            contains_seq(&out, b"\x1bPqpayload\x1b\\"),
            "probe-acked Sixel should emit: {:?}",
            String::from_utf8_lossy(&out)
        );
    }

    #[test]
    fn sprixel_no_text_change_emits_zero_bytes() {
        // A frame identical to the previous one must emit no sprixel bytes.
        let area = Rect::new(0, 0, 10, 5);
        let placement = make_sprixel(vec![SprixelCell::Opaque; 4]);

        let mut current = Buffer::empty(area);
        current.sprixels.push(placement.clone());
        let mut previous = Buffer::empty(area);
        previous.sprixels.push(placement);

        let mut out: Vec<u8> = Vec::new();
        flush_sprixels(&mut out, &current, &previous, 0).unwrap();
        assert!(out.is_empty(), "stable frame should emit no sprixel bytes");
    }

    #[test]
    fn sprixel_first_frame_blits_once() {
        // No previous placement -> the graphic must be emitted exactly once.
        let area = Rect::new(0, 0, 10, 5);
        let mut current = Buffer::empty(area);
        current
            .sprixels
            .push(make_sprixel(vec![SprixelCell::Opaque; 4]));
        let previous = Buffer::empty(area);

        let mut out: Vec<u8> = Vec::new();
        flush_sprixels(&mut out, &current, &previous, 0).unwrap();
        let s = String::from_utf8(out).unwrap();
        assert_eq!(s.matches("<SIXEL>").count(), 1);
    }

    #[test]
    fn sprixel_text_in_opaque_cell_reblits_once() {
        // A text write over an opaque footprint cell annihilates the graphic.
        let area = Rect::new(0, 0, 10, 5);
        let placement = make_sprixel(vec![SprixelCell::Opaque; 4]);

        let mut current = Buffer::empty(area);
        current.sprixels.push(placement.clone());
        // Write a glyph over the top-left footprint cell (1, 1).
        current.set_char(1, 1, 'X', Style::new());

        let mut previous = Buffer::empty(area);
        previous.sprixels.push(placement);

        let mut out: Vec<u8> = Vec::new();
        flush_sprixels(&mut out, &current, &previous, 0).unwrap();
        let s = String::from_utf8(out).unwrap();
        assert_eq!(
            s.matches("<SIXEL>").count(),
            1,
            "opaque-cell text write must re-blit the graphic exactly once"
        );
    }

    #[test]
    fn sprixel_text_in_transparent_cell_does_not_reblit() {
        // The footprint marks (1, 1) transparent; a text write there must NOT
        // re-blit the graphic (the core #265 win).
        let area = Rect::new(0, 0, 10, 5);
        let cells = vec![
            SprixelCell::Transparent, // (1, 1)
            SprixelCell::Opaque,      // (2, 1)
            SprixelCell::Opaque,      // (1, 2)
            SprixelCell::Opaque,      // (2, 2)
        ];
        let placement = make_sprixel(cells);

        let mut current = Buffer::empty(area);
        current.sprixels.push(placement.clone());
        current.set_char(1, 1, 'X', Style::new());

        let mut previous = Buffer::empty(area);
        previous.sprixels.push(placement);

        let mut out: Vec<u8> = Vec::new();
        flush_sprixels(&mut out, &current, &previous, 0).unwrap();
        assert!(
            out.is_empty(),
            "text in a transparent footprint cell must emit zero sprixel bytes"
        );
    }

    #[test]
    fn sprixel_text_outside_footprint_does_not_reblit() {
        // A text write adjacent to (but outside) the footprint is free.
        let area = Rect::new(0, 0, 10, 5);
        let placement = make_sprixel(vec![SprixelCell::Opaque; 4]);

        let mut current = Buffer::empty(area);
        current.sprixels.push(placement.clone());
        // (5, 0) is well outside the (1,1)-(2,2) footprint.
        current.set_char(5, 0, 'Z', Style::new());

        let mut previous = Buffer::empty(area);
        previous.sprixels.push(placement);

        let mut out: Vec<u8> = Vec::new();
        flush_sprixels(&mut out, &current, &previous, 0).unwrap();
        assert!(
            out.is_empty(),
            "text outside the footprint must not re-blit the graphic"
        );
    }

    #[test]
    fn sprixel_position_change_reblits() {
        // Moving the graphic (same content, new x/y) must re-blit.
        let area = Rect::new(0, 0, 10, 5);
        let mut moved = make_sprixel(vec![SprixelCell::Opaque; 4]);
        let original = moved.clone();
        moved.x = 4;

        let mut current = Buffer::empty(area);
        current.sprixels.push(moved);
        let mut previous = Buffer::empty(area);
        previous.sprixels.push(original);

        let mut out: Vec<u8> = Vec::new();
        flush_sprixels(&mut out, &current, &previous, 0).unwrap();
        let s = String::from_utf8(out).unwrap();
        assert_eq!(s.matches("<SIXEL>").count(), 1);
    }

    #[test]
    fn sprixel_content_change_reblits() {
        // Same position, different content hash -> re-blit.
        let area = Rect::new(0, 0, 10, 5);
        let mut recolored = make_sprixel(vec![SprixelCell::Opaque; 4]);
        let original = recolored.clone();
        recolored.content_hash = 0x1234;
        recolored.seq = "<SIXEL2>".to_string();

        let mut current = Buffer::empty(area);
        current.sprixels.push(recolored);
        let mut previous = Buffer::empty(area);
        previous.sprixels.push(original);

        let mut out: Vec<u8> = Vec::new();
        flush_sprixels(&mut out, &current, &previous, 0).unwrap();
        let s = String::from_utf8(out).unwrap();
        assert_eq!(s.matches("<SIXEL2>").count(), 1);
    }

    #[test]
    fn sprixel_reblit_count_invariant_over_single_cell_writes() {
        // Invariant (issue #265 proptest spirit, exhaustive here): for a write
        // to a single footprint cell, the number of re-emitted sprixels is 0
        // iff that cell is Transparent, else 1.
        let area = Rect::new(0, 0, 10, 5);
        for (idx, (col, row)) in [(0u32, 0u32), (1, 0), (0, 1), (1, 1)]
            .into_iter()
            .enumerate()
        {
            for state in [
                SprixelCell::Opaque,
                SprixelCell::Mixed,
                SprixelCell::Transparent,
            ] {
                let mut cells = vec![SprixelCell::Opaque; 4];
                cells[idx] = state;
                let placement = make_sprixel(cells);

                let mut current = Buffer::empty(area);
                current.sprixels.push(placement.clone());
                current.set_char(1 + col, 1 + row, 'A', Style::new());

                let mut previous = Buffer::empty(area);
                previous.sprixels.push(placement);

                let mut out: Vec<u8> = Vec::new();
                flush_sprixels(&mut out, &current, &previous, 0).unwrap();
                let count = String::from_utf8(out).unwrap().matches("<SIXEL>").count();
                let expected = if matches!(state, SprixelCell::Transparent) {
                    0
                } else {
                    1
                };
                assert_eq!(
                    count, expected,
                    "cell ({col},{row}) state {state:?}: expected {expected} re-blits"
                );
            }
        }
    }

    // ---- v0.21.1 sprixel reblit-scan optimization regression ---------------
    //
    // These drive the hashed-key position lookup and the per-row clean+hash
    // shortcut with `recompute_line_hashes` engaged (the real `flush` ordering),
    // proving the optimization preserves the exact #265 re-blit semantics.

    #[test]
    fn sprixel_unchanged_with_hashes_engaged_emits_zero_bytes() {
        // Regression: a steady frame (identical to previous) with per-row
        // digests refreshed must NOT re-blit. This exercises the per-row
        // clean+hash shortcut: every footprint row is clean and hash-matched, so
        // the per-cell scan is skipped and nothing is emitted.
        let area = Rect::new(0, 0, 10, 5);
        let placement = make_sprixel(vec![SprixelCell::Opaque; 4]);

        let mut current = Buffer::empty(area);
        current.sprixels.push(placement.clone());
        let mut previous = Buffer::empty(area);
        previous.sprixels.push(placement);

        // Match `Terminal::flush`: refresh digests before the sprixel pass.
        current.recompute_line_hashes();
        previous.recompute_line_hashes();
        // Sanity: the footprint rows are clean and hash-identical, so the
        // shortcut is the path actually taken.
        assert!(current.row_clean(1) && current.row_clean(2));
        assert_eq!(current.row_hash(1), previous.row_hash(1));

        let mut out: Vec<u8> = Vec::new();
        flush_sprixels(&mut out, &current, &previous, 0).unwrap();
        assert!(
            out.is_empty(),
            "unchanged sprixel must not be re-blitted (per-row shortcut)"
        );
    }

    #[test]
    fn sprixel_changed_text_with_hashes_engaged_reblits_once() {
        // Regression: a text write over an opaque footprint cell must still
        // re-blit exactly once even with digests refreshed. The touched row is
        // dirty (or hash-mismatched), so the shortcut correctly does NOT skip it
        // and the per-cell annihilation scan fires.
        let area = Rect::new(0, 0, 10, 5);
        let placement = make_sprixel(vec![SprixelCell::Opaque; 4]);

        let mut current = Buffer::empty(area);
        current.sprixels.push(placement.clone());
        current.set_char(1, 1, 'X', Style::new());
        let mut previous = Buffer::empty(area);
        previous.sprixels.push(placement);

        current.recompute_line_hashes();
        previous.recompute_line_hashes();
        // The footprint's top row differs from the previous frame.
        assert_ne!(current.row_hash(1), previous.row_hash(1));

        let mut out: Vec<u8> = Vec::new();
        flush_sprixels(&mut out, &current, &previous, 0).unwrap();
        let s = String::from_utf8(out).unwrap();
        assert_eq!(
            s.matches("<SIXEL>").count(),
            1,
            "annihilating text write must re-blit exactly once"
        );
    }

    #[test]
    fn sprixel_changed_text_in_transparent_cell_with_hashes_does_not_reblit() {
        // Regression edge case: even though the touched row is dirty/hash-mismatched
        // (so the per-row shortcut does NOT skip it), a write landing only on a
        // Transparent footprint cell must still emit zero bytes — the per-cell
        // damage matrix governs, exactly as in the unoptimized path.
        let area = Rect::new(0, 0, 10, 5);
        let cells = vec![
            SprixelCell::Transparent, // (1, 1)
            SprixelCell::Opaque,      // (2, 1)
            SprixelCell::Opaque,      // (1, 2)
            SprixelCell::Opaque,      // (2, 2)
        ];
        let placement = make_sprixel(cells);

        let mut current = Buffer::empty(area);
        current.sprixels.push(placement.clone());
        current.set_char(1, 1, 'X', Style::new());
        let mut previous = Buffer::empty(area);
        previous.sprixels.push(placement);

        current.recompute_line_hashes();
        previous.recompute_line_hashes();

        let mut out: Vec<u8> = Vec::new();
        flush_sprixels(&mut out, &current, &previous, 0).unwrap();
        assert!(
            out.is_empty(),
            "transparent-cell text write must not re-blit even with hashes engaged"
        );
    }

    #[test]
    fn sprixel_key_matches_partial_eq_contract() {
        // The hashed identity key must agree with `SprixelPlacement: PartialEq`:
        // equal placements share a key; any field the PartialEq compares
        // produces a distinct key.
        let base = make_sprixel(vec![SprixelCell::Opaque; 4]);
        assert_eq!(sprixel_key(&base), sprixel_key(&base.clone()));

        let mut moved = base.clone();
        moved.x = 7;
        assert_ne!(sprixel_key(&base), sprixel_key(&moved));

        let mut recolored = base.clone();
        recolored.content_hash = 0x9999;
        assert_ne!(sprixel_key(&base), sprixel_key(&recolored));

        // The damage matrix is excluded from both PartialEq and the key.
        let mut annihilated = base.clone();
        annihilated.cells = vec![SprixelCell::Annihilated; 4];
        assert_eq!(sprixel_key(&base), sprixel_key(&annihilated));
        assert_eq!(base, annihilated);
    }

    #[test]
    fn sprixel_multi_placement_only_changed_one_reblits() {
        // With several stacked sprixels, moving one must re-blit only that one;
        // the others (clean, hash-matched) stay silent. Exercises the hash-set
        // position lookup across multiple placements.
        let area = Rect::new(0, 0, 10, 9);
        let mut current = Buffer::empty(area);
        let mut previous = Buffer::empty(area);
        for i in 0..3u32 {
            let p = SprixelPlacement {
                content_hash: 0x100 + i as u64,
                seq: format!("<S{i}>"),
                x: 0,
                y: i * 3,
                cols: 2,
                rows: 2,
                cells: vec![SprixelCell::Opaque; 4],
            };
            current.sprixels.push(p.clone());
            previous.sprixels.push(p);
        }
        // Move only the middle sprixel.
        current.sprixels[1].x = 5;

        current.recompute_line_hashes();
        previous.recompute_line_hashes();

        let mut out: Vec<u8> = Vec::new();
        flush_sprixels(&mut out, &current, &previous, 0).unwrap();
        let s = String::from_utf8(out).unwrap();
        assert_eq!(s.matches("<S0>").count(), 0);
        assert_eq!(
            s.matches("<S1>").count(),
            1,
            "only the moved sprixel reblits"
        );
        assert_eq!(s.matches("<S2>").count(), 0);
    }

    #[test]
    fn bench_sprixel_fixture_steady_state_emits_nothing() {
        // The bench fixture must represent a steady frame (no re-blit) so it
        // measures the no-damage scan cost. Guards against the wrapper silently
        // emitting work.
        let fixture = __bench_new_sprixel_fixture(4);
        assert_eq!(fixture.len(), 4);
        assert!(!fixture.is_empty());
        let mut out: Vec<u8> = Vec::new();
        fixture.flush(&mut out, 0).unwrap();
        assert!(
            out.is_empty(),
            "steady-state bench fixture re-blits nothing"
        );
    }
}
