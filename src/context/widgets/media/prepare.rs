//! Image preparation caches, protocol encoders and half-block / RGBA
//! fallback drawing for image widgets.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PreparedRgbaKey {
    content_hash: u64,
    width: u32,
    height: u32,
    bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PreparedHalfblockKey {
    content_hash: u64,
    width: u32,
    height: u32,
    cells: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg(feature = "crossterm")]
struct PreparedSixelKey {
    content_hash: u64,
    width: u32,
    height: u32,
    colors: u32,
    target_width: u32,
    target_height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg(feature = "crossterm")]
struct PreparedItermKey {
    content_hash: u64,
    bytes: usize,
    cols: u32,
    rows: u32,
    preserve_aspect: bool,
}

struct PreparationCacheEntry<K, V> {
    key: K,
    value: V,
    weight: usize,
}

struct PreparationCache<K, V> {
    entries: std::collections::VecDeque<PreparationCacheEntry<K, V>>,
    total_weight: usize,
}

impl<K: PartialEq, V: Clone> PreparationCache<K, V> {
    const MAX_ENTRIES: usize = 16;
    const MAX_WEIGHT: usize = 32 * 1024 * 1024;

    fn new() -> Self {
        Self {
            entries: std::collections::VecDeque::new(),
            total_weight: 0,
        }
    }

    fn get(&mut self, key: &K) -> Option<V> {
        let index = self.entries.iter().position(|entry| &entry.key == key)?;
        let entry = self.entries.remove(index)?;
        let value = entry.value.clone();
        self.entries.push_back(entry);
        Some(value)
    }

    fn insert(&mut self, key: K, value: V, weight: usize) {
        if weight > Self::MAX_WEIGHT {
            return;
        }
        while self.entries.len() >= Self::MAX_ENTRIES
            || self.total_weight.saturating_add(weight) > Self::MAX_WEIGHT
        {
            let Some(entry) = self.entries.pop_front() else {
                break;
            };
            self.total_weight = self.total_weight.saturating_sub(entry.weight);
        }
        self.total_weight = self.total_weight.saturating_add(weight);
        self.entries
            .push_back(PreparationCacheEntry { key, value, weight });
    }
}

pub(super) fn prepare_rgba(
    data: &[u8],
    width: u32,
    height: u32,
) -> Option<(u64, std::sync::Arc<Vec<u8>>)> {
    let pixels = u64::from(width).saturating_mul(u64::from(height));
    if pixels == 0 || pixels > crate::buffer::MAX_IMAGE_PIXELS {
        return None;
    }
    let expected = usize::try_from(pixels).ok()?.checked_mul(4)?;
    if data.len() < expected {
        return None;
    }
    let normalized = &data[..expected];
    let content_hash = crate::buffer::hash_rgba(normalized);
    let key = PreparedRgbaKey {
        content_hash,
        width,
        height,
        bytes: expected,
    };

    static CACHE: std::sync::OnceLock<
        std::sync::Mutex<PreparationCache<PreparedRgbaKey, std::sync::Arc<Vec<u8>>>>,
    > = std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(PreparationCache::new()));
    let mut cache = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(prepared) = cache.get(&key)
        && prepared.as_slice() == normalized
    {
        return Some((content_hash, prepared));
    }

    let prepared = std::sync::Arc::new(normalized.to_vec());
    cache.insert(key, std::sync::Arc::clone(&prepared), expected);
    Some((content_hash, prepared))
}

pub(super) fn prepare_halfblock(
    pixels: &[(Color, Color)],
    width: u32,
    height: u32,
) -> Option<std::sync::Arc<Vec<(Color, Color)>>> {
    use std::hash::{Hash, Hasher};

    let cells = usize::try_from(u64::from(width).saturating_mul(u64::from(height))).ok()?;
    if cells == 0 || pixels.len() < cells {
        return None;
    }
    let pixels = &pixels[..cells];
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    pixels.hash(&mut hasher);
    let key = PreparedHalfblockKey {
        content_hash: hasher.finish(),
        width,
        height,
        cells,
    };
    type HalfblockPreparationCache = std::sync::Mutex<
        PreparationCache<PreparedHalfblockKey, std::sync::Arc<Vec<(Color, Color)>>>,
    >;
    static CACHE: std::sync::OnceLock<HalfblockPreparationCache> = std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(PreparationCache::new()));
    let mut cache = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(prepared) = cache.get(&key)
        && prepared.as_slice() == pixels
    {
        return Some(prepared);
    }

    let prepared = std::sync::Arc::new(pixels.to_vec());
    let weight = cells.saturating_mul(std::mem::size_of::<(Color, Color)>());
    cache.insert(key, std::sync::Arc::clone(&prepared), weight);
    Some(prepared)
}

#[cfg(feature = "crossterm")]
type EncodedMedia = (u64, std::sync::Arc<str>);

#[cfg(feature = "crossterm")]
type EncodedMediaCache<K> = std::sync::Mutex<PreparationCache<K, EncodedMedia>>;

#[cfg(feature = "crossterm")]
pub(super) fn prepare_sixel(
    data: &[u8],
    width: u32,
    height: u32,
    colors: u32,
    target_width: u32,
    target_height: u32,
) -> Option<EncodedMedia> {
    let (content_hash, rgba) = prepare_rgba(data, width, height)?;
    let key = PreparedSixelKey {
        content_hash,
        width,
        height,
        colors,
        target_width,
        target_height,
    };
    static CACHE: std::sync::OnceLock<EncodedMediaCache<PreparedSixelKey>> =
        std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(PreparationCache::new()));
    let cached = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(&key);
    if let Some(prepared) = cached {
        return Some(prepared);
    }

    let scaled = crate::sixel::resize_rgba(&rgba, width, height, target_width, target_height)?;
    let encoded: std::sync::Arc<str> =
        crate::sixel::encode_sixel(&scaled, target_width, target_height, colors).into();
    if encoded.is_empty() {
        return None;
    }
    let payload_hash = crate::buffer::hash_rgba(encoded.as_bytes());
    cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(key, (payload_hash, encoded.clone()), encoded.len());
    Some((payload_hash, encoded))
}

#[cfg(feature = "crossterm")]
pub(super) fn prepare_iterm(
    data: &[u8],
    cols: u32,
    rows: u32,
    preserve_aspect: bool,
) -> Option<EncodedMedia> {
    encoded_image_dimensions(data)?;
    let content_hash = crate::buffer::hash_rgba(data);
    let key = PreparedItermKey {
        content_hash,
        bytes: data.len(),
        cols,
        rows,
        preserve_aspect,
    };
    static CACHE: std::sync::OnceLock<EncodedMediaCache<PreparedItermKey>> =
        std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(PreparationCache::new()));
    let cached = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(&key);
    if let Some(prepared) = cached {
        return Some(prepared);
    }

    let encoded: std::sync::Arc<str> =
        crate::iterm::encode_iterm_osc1337(data, cols, rows, preserve_aspect).into();
    if encoded.is_empty() {
        return None;
    }
    let payload_hash = crate::buffer::hash_rgba(encoded.as_bytes());
    cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(key, (payload_hash, encoded.clone()), encoded.len());
    Some((payload_hash, encoded))
}

pub(super) fn encoded_image_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() >= 24
        && data[..8] == [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]
        && data[12..16] == *b"IHDR"
    {
        let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
        let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
        return (width > 0 && height > 0).then_some((width, height));
    }

    if data.len() >= 10 && (&data[..6] == b"GIF87a" || &data[..6] == b"GIF89a") {
        let width = u32::from(u16::from_le_bytes([data[6], data[7]]));
        let height = u32::from(u16::from_le_bytes([data[8], data[9]]));
        return (width > 0 && height > 0).then_some((width, height));
    }

    if data.starts_with(&[0xff, 0xd8]) {
        let mut offset = 2usize;
        let scan_limit = data.len().min(1024 * 1024);
        while offset + 3 < scan_limit {
            if data[offset] != 0xff {
                offset += 1;
                continue;
            }
            while offset < scan_limit && data[offset] == 0xff {
                offset += 1;
            }
            if offset >= scan_limit {
                break;
            }
            let marker = data[offset];
            offset += 1;
            if marker == 0xd9 || marker == 0xda {
                break;
            }
            if marker == 0x01 || (0xd0..=0xd7).contains(&marker) {
                continue;
            }
            if offset + 1 >= scan_limit {
                break;
            }
            let segment_len = usize::from(u16::from_be_bytes([data[offset], data[offset + 1]]));
            if segment_len < 2 || offset.saturating_add(segment_len) > scan_limit {
                break;
            }
            let is_start_of_frame = matches!(
                marker,
                0xc0 | 0xc1
                    | 0xc2
                    | 0xc3
                    | 0xc5
                    | 0xc6
                    | 0xc7
                    | 0xc9
                    | 0xca
                    | 0xcb
                    | 0xcd
                    | 0xce
                    | 0xcf
            );
            if is_start_of_frame && segment_len >= 7 {
                let height = u32::from(u16::from_be_bytes([data[offset + 3], data[offset + 4]]));
                let width = u32::from(u16::from_be_bytes([data[offset + 5], data[offset + 6]]));
                return (width > 0 && height > 0).then_some((width, height));
            }
            offset = offset.saturating_add(segment_len);
        }
    }

    None
}

pub(super) fn image_fit_rows(
    src_width: u32,
    src_height: u32,
    cols: u32,
    cell_w: u32,
    cell_h: u32,
) -> u32 {
    if src_width == 0 || src_height == 0 || cell_w == 0 || cell_h == 0 {
        return 1;
    }

    ((cols as f64 * src_height as f64 * cell_w as f64) / (src_width as f64 * cell_h as f64))
        .ceil()
        .max(1.0) as u32
}

fn sample_rgba_color(
    data: &[u8],
    src_width: u32,
    src_height: u32,
    dst_x: u32,
    dst_y: u32,
    dst_width: u32,
    dst_height: u32,
) -> Option<Color> {
    if src_width == 0 || src_height == 0 || dst_width == 0 || dst_height == 0 {
        return None;
    }

    let src_x = (u64::from(dst_x) * u64::from(src_width) / u64::from(dst_width))
        .min(u64::from(src_width.saturating_sub(1)));
    let src_y = (u64::from(dst_y) * u64::from(src_height) / u64::from(dst_height))
        .min(u64::from(src_height.saturating_sub(1)));
    let pixel = src_y
        .saturating_mul(u64::from(src_width))
        .saturating_add(src_x);
    let idx = usize::try_from(pixel).ok().and_then(|p| p.checked_mul(4))?;
    let px = data.get(idx..idx + 4)?;
    if px[3] == 0 {
        None
    } else {
        Some(Color::Rgb(px[0], px[1], px[2]))
    }
}

fn draw_halfblock_cell(
    buf: &mut crate::Buffer,
    x: u32,
    y: u32,
    upper: Option<Color>,
    lower: Option<Color>,
) {
    match (upper, lower) {
        (Some(upper), Some(lower)) => {
            buf.set_char(x, y, '▀', Style::new().fg(upper).bg(lower));
        }
        (Some(upper), None) => {
            buf.set_char(x, y, '▀', Style::new().fg(upper));
        }
        (None, Some(lower)) => {
            buf.set_char(x, y, '▄', Style::new().fg(lower));
        }
        (None, None) => {
            buf.set_char(x, y, ' ', Style::new());
        }
    }
}

pub(super) fn draw_rgba_fallback(
    buf: &mut crate::Buffer,
    rect: Rect,
    rgba: &[u8],
    width: u32,
    height: u32,
    cols: u32,
    rows: u32,
) {
    let (source_x, source_y) = buf.draw_source_offset();
    let pixel_rows = rows.saturating_mul(2).max(1);
    for y in 0..rect.height {
        for x in 0..rect.width {
            let sx = source_x.saturating_add(x);
            let sy = source_y.saturating_add(y).saturating_mul(2);
            let upper = sample_rgba_color(rgba, width, height, sx, sy, cols, pixel_rows);
            let lower = sample_rgba_color(
                rgba,
                width,
                height,
                sx,
                sy.saturating_add(1),
                cols,
                pixel_rows,
            );
            draw_halfblock_cell(buf, rect.x + x, rect.y + y, upper, lower);
        }
    }
}

#[cfg(test)]
mod media_preparation_tests {
    use super::*;

    #[cfg(feature = "crossterm")]
    #[test]
    fn sixel_preparation_shares_payload_and_keys_final_geometry() {
        let rgba = [255, 0, 0, 255].repeat(4);
        let (_, first) = prepare_sixel(&rgba, 2, 2, 256, 8, 16).unwrap();
        let (_, second) = prepare_sixel(&rgba, 2, 2, 256, 8, 16).unwrap();
        assert!(std::sync::Arc::ptr_eq(&first, &second));
        let (_, resized) = prepare_sixel(&rgba, 2, 2, 256, 16, 8).unwrap();
        assert_ne!(first, resized);
    }

    #[test]
    fn rgba_preparation_is_strict_and_reuses_cached_allocation() {
        let rgba = [255u8, 0, 0, 255, 0, 255, 0, 255];
        let (_, first) = prepare_rgba(&rgba, 2, 1).expect("valid rgba");
        let (_, second) = prepare_rgba(&rgba, 2, 1).expect("cached rgba");
        assert!(std::sync::Arc::ptr_eq(&first, &second));
        assert!(prepare_rgba(&rgba[..7], 2, 1).is_none());
        assert!(prepare_rgba(&rgba, 0, 1).is_none());
    }

    #[test]
    fn halfblock_preparation_reuses_cached_allocation() {
        let pixels = vec![(Color::Red, Color::Blue); 4];
        let first = prepare_halfblock(&pixels, 2, 2).expect("valid halfblock image");
        let second = prepare_halfblock(&pixels, 2, 2).expect("cached halfblock image");
        assert!(std::sync::Arc::ptr_eq(&first, &second));
        assert!(prepare_halfblock(&pixels[..3], 2, 2).is_none());
    }

    #[test]
    fn encoded_image_dimensions_reads_png_gif_and_jpeg_headers() {
        let mut png = vec![0u8; 24];
        png[..8].copy_from_slice(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
        png[12..16].copy_from_slice(b"IHDR");
        png[16..20].copy_from_slice(&320u32.to_be_bytes());
        png[20..24].copy_from_slice(&200u32.to_be_bytes());
        assert_eq!(encoded_image_dimensions(&png), Some((320, 200)));

        let mut gif = b"GIF89a".to_vec();
        gif.extend_from_slice(&64u16.to_le_bytes());
        gif.extend_from_slice(&32u16.to_le_bytes());
        assert_eq!(encoded_image_dimensions(&gif), Some((64, 32)));

        let jpeg = [
            0xff, 0xd8, 0xff, 0xc0, 0x00, 0x11, 0x08, 0x00, 0x30, 0x00, 0x40, 0x03, 0x01, 0x11,
            0x00, 0x02, 0x11, 0x00, 0x03, 0x11, 0x00,
        ];
        assert_eq!(encoded_image_dimensions(&jpeg), Some((64, 48)));
        assert_eq!(encoded_image_dimensions(b"not an image"), None);
    }

    #[test]
    fn image_fit_rows_uses_source_aspect_ratio() {
        assert_eq!(image_fit_rows(400, 200, 20, 8, 16), 5);
        assert_eq!(image_fit_rows(200, 400, 20, 8, 16), 20);
    }

    #[test]
    fn preparation_cache_evicts_old_entries_at_fixed_capacity() {
        let mut cache = PreparationCache::new();
        for key in 0..(PreparationCache::<usize, usize>::MAX_ENTRIES + 2) {
            cache.insert(key, key, 1);
        }
        assert_eq!(
            cache.entries.len(),
            PreparationCache::<usize, usize>::MAX_ENTRIES
        );
        assert!(cache.get(&0).is_none());
        assert!(
            cache
                .get(&(PreparationCache::<usize, usize>::MAX_ENTRIES + 1))
                .is_some()
        );
    }
}
