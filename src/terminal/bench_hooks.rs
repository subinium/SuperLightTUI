//! `#[doc(hidden)]` benchmark fixtures re-exported from the crate root for
//! `benches/`.

use super::*;

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
/// Kept for signature stability: the per-row digests it used to refresh
/// were replaced by written-span tracking (#486), which needs no pre-pass,
/// so it now runs exactly the same kernel as the immutable variant.
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
/// the buffer across frames.
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
/// re-blits nothing, so the untouched-row shortcut in [`sprixel_needs_reblit`]
/// is exercised.
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
            seq: "<SIXEL>".into(),
            x: 0,
            y: i as u32 * 3,
            cols: 4,
            rows: 2,
            cells: vec![SprixelCell::Opaque; 8],
        };
        current.sprixels.push(placement.clone());
        previous.sprixels.push(placement);
    }

    __BenchSprixelFixture { current, previous }
}

// Reachable from `benches/` through the crate-root re-export; the methods stay
// ungated (not `cfg(test)`) so the external bench crate can call them.
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
