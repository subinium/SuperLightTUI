//! Kitty graphics protocol image manager: upload dedup, placement diffing,
//! raster identity and payload compression.

use super::*;

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
    pub(super) uploaded: HashMap<(u64, u32, u32), u32>,
    /// Previous frame's placements (for diff).
    prev_placements: Vec<KittyPlacement>,
    /// Reused dedup scratch for already-deleted image IDs in `flush`. Typical
    /// placement counts are 0–8 (well below where a `HashSet` beats a linear /
    /// sorted scan), so a `SmallVec` stays on the stack and carries its
    /// capacity across frames — no per-frame heap allocation, no SipHash.
    scratch_ids: smallvec::SmallVec<[u32; 8]>,
    /// Reused scratch for content hashes still referenced this frame, used to
    /// prune stale uploads. Sorted in place for `binary_search` membership.
    scratch_hashes: smallvec::SmallVec<[(u64, u32, u32); 8]>,
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
        let result = self.flush_inner(stdout, current, row_offset);
        if result.is_err() {
            // A partial transmit/place/delete leaves terminal state unknown.
            // Force a fresh upload on the next successful frame.
            let _ = self.delete_all(stdout);
        }
        result
    }

    fn flush_inner(
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
                if let Some(&img_id) = self.uploaded.get(&kitty_raster_key(p))
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
            let img_id = if let Some(&existing_id) = self.uploaded.get(&kitty_raster_key(p)) {
                existing_id
            } else {
                // Upload new image with zlib compression if available
                let id = self.next_id;
                self.next_id += 1;
                self.upload_image(stdout, id, p)?;
                self.uploaded.insert(kitty_raster_key(p), id);
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
            .extend(current.iter().map(kitty_raster_key));
        self.scratch_hashes.sort_unstable();
        let scratch_hashes = &self.scratch_hashes;
        let stale: smallvec::SmallVec<[(u64, u32, u32); 8]> = self
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
    pub(crate) fn delete_all(&mut self, stdout: &mut impl Write) -> io::Result<()> {
        // Even partial output can delete terminal data. Never reuse its IDs.
        self.uploaded.clear();
        self.prev_placements.clear();
        queue!(stdout, Print("\x1b_Ga=d,d=A,q=2\x1b\\"))
    }
}

fn kitty_raster_key(p: &KittyPlacement) -> (u64, u32, u32) {
    (p.content_hash, p.src_width, p.src_height)
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
    kitty_raster_key(current) == kitty_raster_key(prev)
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
