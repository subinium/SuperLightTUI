//! Sprixel (Sixel / iTerm2) graphics emission policy, raw-sequence flushing
//! and sprixel reblit scheduling.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GraphicsEmissionSupport {
    pub(super) real_terminal: bool,
    pub(super) capabilities: Capabilities,
    pub(super) force_kitty: bool,
    pub(super) force_sixel: bool,
    pub(super) force_iterm: bool,
}

impl GraphicsEmissionSupport {
    pub(crate) fn for_context(capabilities: Capabilities, real_terminal: bool) -> Self {
        Self {
            real_terminal,
            ..Self::detect(capabilities)
        }
    }

    pub(super) fn detect(capabilities: Capabilities) -> Self {
        let disable_kitty = force_env_enabled("SLT_DISABLE_KITTY");
        let disable_sixel = force_env_enabled("SLT_DISABLE_SIXEL");
        let disable_iterm = force_env_enabled("SLT_DISABLE_ITERM");
        Self {
            real_terminal: true,
            capabilities: Capabilities {
                kitty_graphics: (capabilities.kitty_graphics || term_is_kitty_graphics_host())
                    && !disable_kitty,
                sixel: (capabilities.sixel || term_is_sixel_host()) && !disable_sixel,
                iterm2: (capabilities.iterm2 || term_is_iterm_host()) && !disable_iterm,
                ..capabilities
            },
            force_kitty: force_env_enabled("SLT_FORCE_KITTY") && !disable_kitty,
            force_sixel: force_env_enabled("SLT_FORCE_SIXEL") && !disable_sixel,
            force_iterm: force_env_enabled("SLT_FORCE_ITERM") && !disable_iterm,
        }
    }

    #[cfg(any(test, feature = "pty-test"))]
    pub(super) fn capture() -> Self {
        Self::detect(probe_capabilities())
    }

    pub(crate) fn should_emit_kitty(self) -> bool {
        self.real_terminal && (self.capabilities.kitty_graphics || self.force_kitty)
    }

    pub(crate) fn should_emit_sprixel(self, protocol: SprixelProtocol) -> bool {
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
pub(crate) enum SprixelProtocol {
    Sixel,
    Iterm2,
    Unknown,
}

pub(super) fn sprixel_protocol(seq: &str) -> SprixelProtocol {
    if seq.starts_with("\x1bPq") {
        SprixelProtocol::Sixel
    } else if seq.starts_with("\x1b]1337;File=") {
        SprixelProtocol::Iterm2
    } else {
        SprixelProtocol::Unknown
    }
}

pub(super) fn flush_raw_sequences(
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
pub(super) fn sprixel_key(p: &crate::buffer::SprixelPlacement) -> SprixelKey {
    (p.content_hash, p.x, p.y, p.cols, p.rows)
}

pub(super) fn previous_only_sprixel_rows(
    current: &Buffer,
    previous: &Buffer,
    mut should_emit: impl FnMut(&crate::buffer::SprixelPlacement) -> bool,
) -> smallvec::SmallVec<[u32; 8]> {
    use crate::buffer::SprixelCell;

    if previous.sprixels.is_empty() {
        return smallvec::SmallVec::new();
    }
    let current_keys: std::collections::HashSet<SprixelKey> =
        current.sprixels.iter().map(sprixel_key).collect();
    let mut rows = smallvec::SmallVec::<[u32; 8]>::new();
    for placement in &previous.sprixels {
        if !should_emit(placement) || current_keys.contains(&sprixel_key(placement)) {
            continue;
        }
        let Ok(cols) = usize::try_from(placement.cols) else {
            continue;
        };
        for row in 0..placement.rows {
            let Ok(row_index) = usize::try_from(row) else {
                continue;
            };
            let Some(start) = row_index.checked_mul(cols) else {
                continue;
            };
            let end = start.saturating_add(cols).min(placement.cells.len());
            if placement.cells.get(start..end).is_some_and(|cells| {
                cells
                    .iter()
                    .any(|cell| matches!(cell, SprixelCell::Opaque | SprixelCell::Mixed))
            }) {
                let y = placement.y.saturating_add(row);
                if current.area.contains(current.area.x, y) {
                    rows.push(y);
                }
            }
        }
    }
    rows.sort_unstable();
    rows.dedup();
    rows
}

fn sprixel_intersects_rows(placement: &crate::buffer::SprixelPlacement, rows: &[u32]) -> bool {
    let end = placement.y.saturating_add(placement.rows);
    rows.iter().any(|row| *row >= placement.y && *row < end)
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
/// The footprint scan short-circuits an entire footprint row when the
/// written-span comparison proves it identical to the previous frame
/// ([`crate::buffer::row_diff_range`] returns `None`): no cell in such a row can
/// have changed, so no ink can have been annihilated. Buffers without a shared
/// layout always take the per-cell scan.
fn sprixel_needs_reblit(
    placement: &crate::buffer::SprixelPlacement,
    current: &Buffer,
    previous: &Buffer,
    prev_keys: &std::collections::HashSet<SprixelKey>,
    redrawn_rows: &[u32],
) -> bool {
    use crate::buffer::SprixelCell;

    // Position / content change: re-blit if no equal placement existed last
    // frame. The key mirrors `SprixelPlacement: PartialEq` (content_hash/x/y/
    // cols/rows; damage matrix excluded), so a moved or recolored image
    // re-blits. O(1) lookup vs the former O(n·m) `iter().any(..)` scan.
    if !prev_keys.contains(&sprixel_key(placement)) {
        return true;
    }

    // Removing another graphic repaints its old footprint rows with text. Any
    // surviving graphic on those rows must be restored after that repaint.
    if sprixel_intersects_rows(placement, redrawn_rows) {
        return true;
    }

    // Annihilation scan: a covered text cell that changed since last frame and
    // now shows ink forces a re-blit. `Transparent` cells are skipped so free
    // text edits in graphic gaps emit zero sprixel bytes.
    let layout_shared = crate::buffer::same_layout(current, previous);
    for row in 0..placement.rows {
        let y = placement.y + row;
        // Per-row shortcut: a row the written spans prove identical to the
        // previous frame cannot contain a changed cell, so the whole
        // footprint row is skipped without per-cell work.
        if layout_shared
            && y >= current.area.y
            && y < current.area.bottom()
            && crate::buffer::row_diff_range(current, previous, (y - current.area.y) as usize)
                .is_none()
        {
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
pub(super) fn flush_sprixels(
    stdout: &mut impl Write,
    current: &Buffer,
    previous: &Buffer,
    row_offset: u32,
) -> io::Result<()> {
    flush_sprixels_inner(stdout, current, previous, row_offset, |_| true, &[])
}

#[cfg(test)]
pub(super) fn flush_sprixels_checked(
    stdout: &mut impl Write,
    current: &Buffer,
    previous: &Buffer,
    row_offset: u32,
    graphics_support: GraphicsEmissionSupport,
) -> io::Result<()> {
    flush_sprixels_checked_with_rows(stdout, current, previous, row_offset, graphics_support, &[])
}

pub(super) fn flush_sprixels_checked_with_rows(
    stdout: &mut impl Write,
    current: &Buffer,
    previous: &Buffer,
    row_offset: u32,
    graphics_support: GraphicsEmissionSupport,
    redrawn_rows: &[u32],
) -> io::Result<()> {
    flush_sprixels_inner(
        stdout,
        current,
        previous,
        row_offset,
        |placement| graphics_support.should_emit_sprixel(sprixel_protocol(&placement.seq)),
        redrawn_rows,
    )
}

pub(super) fn flush_sprixels_inner(
    stdout: &mut impl Write,
    current: &Buffer,
    previous: &Buffer,
    row_offset: u32,
    mut should_emit: impl FnMut(&crate::buffer::SprixelPlacement) -> bool,
    redrawn_rows: &[u32],
) -> io::Result<()> {
    // Early out: no graphics to emit. Avoids building the key set on the
    // common text-only frame.
    if current.sprixels.is_empty() {
        return Ok(());
    }

    let prev_keys: std::collections::HashSet<SprixelKey> =
        previous.sprixels.iter().map(sprixel_key).collect();

    for placement in &current.sprixels {
        if should_emit(placement)
            && sprixel_needs_reblit(placement, current, previous, &prev_keys, redrawn_rows)
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

// ---------------------------------------------------------------------------
// Image passes, installed on first use (issue #491)
// ---------------------------------------------------------------------------

/// Rows a frame repaints with text because a sprixel left them.
pub(super) type RedrawnRows = smallvec::SmallVec<[u32; 8]>;

/// The image passes of a terminal flush: the Kitty upload / placement pass
/// and the Sixel / iTerm2 damage-tracked re-blit.
///
/// `Terminal::flush` and `InlineTerminal::flush` reach these only through
/// [`graphics_passes`], and only [`Buffer::kitty_place`] and
/// [`Buffer::sprixel_place`] install them, so a program that never draws an
/// image does not link the Kitty encoder, the sprixel damage tracker or
/// their hash tables. Until the first placement no buffer in the process
/// holds one, and every pass is a no-op on empty placement lists (the Kitty
/// pass returns at its unchanged fast path, the sprixel passes at their
/// empty checks), so skipping them writes the same bytes.
pub(crate) struct GraphicsPasses {
    /// Rows previous-frame sprixels covered that this frame must repaint.
    pub(super) redrawn_rows: fn(&Buffer, &Buffer, GraphicsEmissionSupport) -> RedrawnRows,
    /// Kitty uploads, placements and deletes for `current`.
    pub(super) kitty:
        fn(&mut KittyImageManager, &mut Sink, &[KittyPlacement], u32) -> io::Result<()>,
    /// Sixel / iTerm2 re-blit of damaged or moved graphics.
    pub(super) sprixels: SprixelPass,
}

/// `(sink, current, previous, row_offset, support, redrawn_rows)`.
type SprixelPass =
    fn(&mut Sink, &Buffer, &Buffer, u32, GraphicsEmissionSupport, &[u32]) -> io::Result<()>;

static GRAPHICS_PASSES: std::sync::OnceLock<GraphicsPasses> = std::sync::OnceLock::new();

/// Install the image passes for every terminal in the process. Called by
/// the placement methods; cheap after the first call.
pub(crate) fn enable_graphics_passes() {
    GRAPHICS_PASSES.get_or_init(|| GraphicsPasses {
        redrawn_rows: |current, previous, support| {
            previous_only_sprixel_rows(current, previous, |placement| {
                support.should_emit_sprixel(sprixel_protocol(&placement.seq))
            })
        },
        kitty: |manager, sink, placements, row_offset| manager.flush(sink, placements, row_offset),
        sprixels: |sink, current, previous, row_offset, support, rows| {
            flush_sprixels_checked_with_rows(sink, current, previous, row_offset, support, rows)
        },
    });
}

/// The installed image passes, or `None` while no image was placed.
#[inline]
pub(super) fn graphics_passes() -> Option<&'static GraphicsPasses> {
    GRAPHICS_PASSES.get()
}
