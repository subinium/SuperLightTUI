//! Cell pixel size probing (`CSI 16 t`) with a resize-invalidated cache.

use super::*;

/// Query the terminal for the actual cell pixel dimensions via CSI 16 t.
///
/// Returns `(cell_width, cell_height)` in pixels. Falls back to `(8, 16)` if
/// detection fails. Used by `kitty_image_fit` for accurate aspect ratio.
///
/// Success is cached until resize; failures retry at most once per second.
pub(crate) fn cell_pixel_size() -> (u32, u32) {
    let mut cache = CELL_PIXEL_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    cache.resolve(Instant::now(), detect_cell_pixel_size)
}

#[derive(Default)]
pub(super) struct CellPixelCache {
    size: Option<(u32, u32)>,
    pub(super) retry_after: Option<Instant>,
    pub(super) refresh: bool,
}

impl CellPixelCache {
    pub(super) fn resolve(
        &mut self,
        now: Instant,
        query: impl FnOnce() -> Option<(u32, u32)>,
    ) -> (u32, u32) {
        if (self.size.is_none() || self.refresh)
            && self.retry_after.is_none_or(|deadline| now >= deadline)
        {
            if let Some(size) = query() {
                self.size = Some(size);
                self.refresh = false;
            }
            self.retry_after = Some(now + Duration::from_secs(1));
        }
        self.size.unwrap_or((8, 16))
    }
}

pub(super) static CELL_PIXEL_CACHE: std::sync::Mutex<CellPixelCache> =
    std::sync::Mutex::new(CellPixelCache {
        size: None,
        retry_after: None,
        refresh: false,
    });

pub(crate) fn invalidate_cell_pixel_size() {
    let mut cache = CELL_PIXEL_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    cache.refresh = true;
    cache.retry_after = None;
}

fn detect_cell_pixel_size() -> Option<(u32, u32)> {
    if let Ok(size) = terminal::window_size()
        && size.columns > 0
        && size.rows > 0
    {
        let width = u32::from(size.width) / u32::from(size.columns);
        let height = u32::from(size.height) / u32::from(size.rows);
        if width > 0 && height > 0 {
            return Some((width, height));
        }
    }
    if !automatic_terminal_queries_allowed() {
        return None;
    }

    // CSI 16 t → reports cell size as CSI 6 ; height ; width t
    let response = read_stdin_reply(
        "\x1b[16t",
        Duration::from_millis(100),
        cell_size_reply_complete,
    )?;
    parse_cell_pixel_size(response.as_bytes())
}

pub(super) fn cell_size_reply_complete(bytes: &[u8]) -> bool {
    parse_cell_pixel_size(bytes).is_some()
}

pub(super) fn parse_cell_pixel_size(bytes: &[u8]) -> Option<(u32, u32)> {
    // Parse: ESC [ 6 ; <height> ; <width> t
    // Locate the reply anywhere in the buffer rather than anchoring to its
    // start/end: interleaved control bytes — e.g. a pump-retirement nudge
    // answer (`CSI 0 n`) from a previous reply session — may surround it.
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
    let tail = std::str::from_utf8(bytes.get(start..)?).ok()?;
    let body = &tail[..tail.find('t')?];
    let mut parts = body.split(';');
    let ch: u32 = parts.next()?.parse().ok()?;
    let cw: u32 = parts.next()?.parse().ok()?;
    if cw > 0 && ch > 0 && parts.next().is_none() {
        Some((cw, ch))
    } else {
        None
    }
}
