const SIXEL_START: &str = "\x1bPq";
const SIXEL_END: &str = "\x1b\\";
const MAX_SIXEL_WORK_UNITS: u64 = 268_435_456;
const MAX_SIXEL_OUTPUT_BYTES: u64 = 16 * 1024 * 1024;

pub(crate) fn encode_sixel(rgba: &[u8], width: u32, height: u32, max_colors: u32) -> String {
    if rgba.is_empty() || width == 0 || height == 0 {
        return String::new();
    }

    // Guard against oversized or attacker-controlled dimensions: `vec![None;
    // pixel_count]` at `width=height=65535` would saturate at ~4 GiB. Reject
    // anything beyond the image pixel cap and let the caller fall back to a
    // placeholder.
    let pixels = u64::from(width).saturating_mul(u64::from(height));
    if pixels == 0 || pixels > crate::buffer::MAX_IMAGE_PIXELS {
        return String::new();
    }

    let width_usize = width as usize;
    let height_usize = height as usize;
    let Some(pixel_count) = width_usize.checked_mul(height_usize) else {
        return String::new();
    };
    if pixel_count == 0 {
        return String::new();
    }
    let Some(required_bytes) = pixel_count.checked_mul(4) else {
        return String::new();
    };
    if rgba.len() < required_bytes {
        return String::new();
    }

    let color_limit = max_colors.clamp(1, 216) as usize;

    let mut pixels: Vec<Option<u8>> = Vec::new();
    if pixels.try_reserve_exact(pixel_count).is_err() {
        return String::new();
    }
    pixels.resize(pixel_count, None);
    let mut palette_to_reg: [Option<u8>; 216] = [None; 216];
    let mut reg_to_palette: Vec<u8> = Vec::with_capacity(color_limit);

    for (i, pixel_slot) in pixels.iter_mut().enumerate().take(pixel_count) {
        let base = i.saturating_mul(4);
        if base + 3 >= rgba.len() {
            break;
        }

        let a = rgba[base + 3];
        if a < 16 {
            continue;
        }

        let quant = quantize_6cube(rgba[base], rgba[base + 1], rgba[base + 2]);

        let reg = if let Some(existing) = palette_to_reg[quant as usize] {
            existing
        } else if reg_to_palette.len() < color_limit {
            let new_reg = reg_to_palette.len() as u8;
            reg_to_palette.push(quant);
            palette_to_reg[quant as usize] = Some(new_reg);
            new_reg
        } else {
            nearest_existing_register(quant, &reg_to_palette)
        };

        *pixel_slot = Some(reg);
    }

    if reg_to_palette.is_empty() {
        return String::new();
    }

    let Some(output_budget) = sixel_output_budget(width, height, reg_to_palette.len()) else {
        return String::new();
    };

    let mut out = String::new();
    if out.try_reserve(output_budget).is_err() {
        return String::new();
    }
    out.push_str(SIXEL_START);
    out.push_str(&format!("\"1;1;{width};{height}"));

    for (reg, &palette_idx) in reg_to_palette.iter().enumerate() {
        let (r, g, b) = palette_index_to_rgb_percent(palette_idx);
        out.push('#');
        out.push_str(&reg.to_string());
        out.push_str(";2;");
        out.push_str(&r.to_string());
        out.push(';');
        out.push_str(&g.to_string());
        out.push(';');
        out.push_str(&b.to_string());
    }

    let sixel_rows = height_usize.div_ceil(6);
    for row in 0..sixel_rows {
        let y_base = row * 6;
        let row_regs = row_registers(
            &pixels,
            width_usize,
            height_usize,
            y_base,
            reg_to_palette.len(),
        );

        let mut first_register = true;
        for (reg, used) in row_regs[..reg_to_palette.len()].iter().enumerate() {
            if !used {
                continue;
            }
            if !first_register {
                out.push('$');
            }
            first_register = false;
            out.push('#');
            out.push_str(&reg.to_string());
            push_register_band(
                &mut out,
                &pixels,
                width_usize,
                height_usize,
                y_base,
                reg as u8,
            );
        }

        if row + 1 < sixel_rows {
            out.push('-');
        }
    }

    out.push_str(SIXEL_END);
    out
}

/// Nearest-neighbor resampling to the final terminal footprint, with bounded
/// work and fallible allocation before touching the source raster.
pub(crate) fn resize_rgba(
    rgba: &[u8],
    width: u32,
    height: u32,
    cols: u32,
    rows: u32,
) -> Option<Vec<u8>> {
    let count = u64::from(cols).checked_mul(u64::from(rows))?;
    let source_count = u64::from(width).checked_mul(u64::from(height))?;
    if count == 0
        || source_count == 0
        || count > crate::buffer::MAX_IMAGE_PIXELS
        || source_count > crate::buffer::MAX_IMAGE_PIXELS
        || rgba.len() < usize::try_from(source_count.checked_mul(4)?).ok()?
    {
        return None;
    }
    let len = usize::try_from(count.checked_mul(4)?).ok()?;
    let mut result = Vec::new();
    result.try_reserve_exact(len).ok()?;
    for y in 0..rows {
        let sy = u64::from(y) * u64::from(height) / u64::from(rows);
        for x in 0..cols {
            let sx = u64::from(x) * u64::from(width) / u64::from(cols);
            let offset = usize::try_from((sy * u64::from(width) + sx) * 4).ok()?;
            result.extend_from_slice(&rgba[offset..offset + 4]);
        }
    }
    Some(result)
}

fn sixel_output_budget(width: u32, height: u32, registers: usize) -> Option<usize> {
    let bands = u64::from(height).div_ceil(6);
    let register_bands = bands.checked_mul(registers as u64)?;
    let encoded_columns = register_bands.checked_mul(u64::from(width))?;
    let work = encoded_columns.checked_mul(6)?;
    if work > MAX_SIXEL_WORK_UNITS {
        return None;
    }

    // Raw columns are the worst case because Sixel RLE never expands a run.
    // Include conservative selector, palette, separator, and envelope space.
    let overhead = register_bands
        .checked_mul(12)?
        .checked_add((registers as u64).checked_mul(24)?)?
        .checked_add(bands)?
        .checked_add(16)?;
    let output = encoded_columns.checked_add(overhead)?;
    if output > MAX_SIXEL_OUTPUT_BYTES {
        return None;
    }
    usize::try_from(output).ok()
}

fn quantize_6cube(r: u8, g: u8, b: u8) -> u8 {
    let ri = ((u16::from(r) * 5 + 127) / 255) as u8;
    let gi = ((u16::from(g) * 5 + 127) / 255) as u8;
    let bi = ((u16::from(b) * 5 + 127) / 255) as u8;
    ri * 36 + gi * 6 + bi
}

fn palette_index_to_rgb_percent(index: u8) -> (u8, u8, u8) {
    let ri = index / 36;
    let gi = (index % 36) / 6;
    let bi = index % 6;

    let r = level_to_percent(ri);
    let g = level_to_percent(gi);
    let b = level_to_percent(bi);
    (r, g, b)
}

fn level_to_percent(level: u8) -> u8 {
    ((u16::from(level) * 100 + 2) / 5) as u8
}

fn nearest_existing_register(target_palette: u8, reg_to_palette: &[u8]) -> u8 {
    let (tr, tg, tb) = palette_triplet(target_palette);
    let mut best_reg = 0_u8;
    let mut best_dist = u16::MAX;

    for (reg, &palette_idx) in reg_to_palette.iter().enumerate() {
        let (r, g, b) = palette_triplet(palette_idx);
        let dr = tr.abs_diff(r);
        let dg = tg.abs_diff(g);
        let db = tb.abs_diff(b);
        let dist = dr * dr + dg * dg + db * db;
        if dist < best_dist {
            best_dist = dist;
            best_reg = reg as u8;
        }
    }

    best_reg
}

fn palette_triplet(index: u8) -> (u16, u16, u16) {
    let r = u16::from(index / 36);
    let g = u16::from((index % 36) / 6);
    let b = u16::from(index % 6);
    (r, g, b)
}

fn row_registers(
    pixels: &[Option<u8>],
    width: usize,
    height: usize,
    y_base: usize,
    reg_count: usize,
) -> [bool; 216] {
    // `reg_count` is bounded by the sixel 6-cube quantization (`color_limit`
    // in `encode_sixel` clamps `max_colors` to ≤ 216). Use a fixed-size stack
    // array to eliminate per-row heap allocation.
    let mut used = [false; 216];

    for bit in 0..6 {
        let y = y_base + bit;
        if y >= height {
            break;
        }
        let start = y * width;
        let end = start + width;
        for &pixel in &pixels[start..end] {
            if let Some(reg) = pixel
                && (reg as usize) < reg_count
            {
                used[reg as usize] = true;
            }
        }
    }

    used
}

fn push_register_band(
    out: &mut String,
    pixels: &[Option<u8>],
    width: usize,
    height: usize,
    y_base: usize,
    register: u8,
) {
    let mut current = None;
    let mut run_len = 0usize;
    for x in 0..width {
        let mut bits = 0u8;
        for bit in 0..6 {
            let y = y_base + bit;
            if y >= height {
                break;
            }
            let index = y * width + x;
            if pixels[index] == Some(register) {
                bits |= 1 << bit;
            }
        }
        let ch = (b'?' + bits) as char;
        let Some(previous) = current else {
            current = Some(ch);
            run_len = 1;
            continue;
        };
        if ch == previous {
            run_len += 1;
            continue;
        }

        push_run(out, previous, run_len);
        current = Some(ch);
        run_len = 1;
    }
    if let Some(current) = current {
        push_run(out, current, run_len);
    }
}

fn push_run(out: &mut String, ch: char, run_len: usize) {
    if run_len >= 4 {
        out.push('!');
        out.push_str(&run_len.to_string());
        out.push(ch);
    } else {
        for _ in 0..run_len {
            out.push(ch);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{encode_sixel, sixel_output_budget};
    use crate::TestBackend;

    #[test]
    fn v024_sixel_resample_decodes_to_expected_four_color_quadrants() {
        let rgba = [
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
        ];
        let resized = super::resize_rgba(&rgba, 2, 2, 4, 4).unwrap();
        let encoded = encode_sixel(&resized, 4, 4, 256);
        let data = encoded
            .strip_prefix("\x1bPq\"1;1;4;4")
            .unwrap()
            .strip_suffix("\x1b\\")
            .unwrap()
            .as_bytes();
        let mut decoded = [[usize::MAX; 4]; 4];
        let (mut x, mut y, mut color, mut index) = (0, 0, 0, 0);
        while index < data.len() {
            match data[index] {
                b'#' => {
                    index += 1;
                    let start = index;
                    while index < data.len() && data[index].is_ascii_digit() {
                        index += 1;
                    }
                    color = std::str::from_utf8(&data[start..index])
                        .unwrap()
                        .parse()
                        .unwrap();
                    if data.get(index) == Some(&b';') {
                        while index < data.len()
                            && (data[index] == b';' || data[index].is_ascii_digit())
                        {
                            index += 1;
                        }
                    }
                    continue;
                }
                b'$' => x = 0,
                b'-' => {
                    x = 0;
                    y += 6;
                }
                value @ b'?'..=b'~' => {
                    for bit in 0..6 {
                        if value - b'?' & (1 << bit) != 0 {
                            assert!(x < 4 && y + bit < 4, "out-of-footprint pixel");
                            decoded[y + bit][x] = color;
                        }
                    }
                    x += 1;
                }
                other => panic!("unexpected fixture byte {other}"),
            }
            index += 1;
        }
        assert_eq!(
            decoded,
            [[0, 0, 1, 1], [0, 0, 1, 1], [2, 2, 3, 3], [2, 2, 3, 3]]
        );
        assert!(super::resize_rgba(&rgba, 2, 2, u32::MAX, 2).is_none());
        assert!(super::resize_rgba(&rgba[..3], 2, 2, 4, 4).is_none());
    }

    #[test]
    fn encode_sixel_single_color_image_has_wrapper() {
        let mut rgba = Vec::with_capacity(2 * 6 * 4);
        for _ in 0..(2 * 6) {
            rgba.extend_from_slice(&[255, 0, 0, 255]);
        }

        let sixel = encode_sixel(&rgba, 2, 6, 256);
        assert!(sixel.starts_with("\x1bPq"));
        assert!(sixel.ends_with("\x1b\\"));
    }

    #[test]
    fn encode_sixel_empty_input_returns_empty() {
        let sixel = encode_sixel(&[], 0, 0, 256);
        assert!(sixel.is_empty());
    }

    #[test]
    fn encode_sixel_declares_multiple_color_registers() {
        let rgba = vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255];

        let sixel = encode_sixel(&rgba, 3, 1, 256);
        assert!(sixel.contains("#0;2;"));
        assert!(sixel.contains("#1;2;"));
        assert!(sixel.contains("#2;2;"));
    }

    #[test]
    fn sixel_image_on_test_backend_does_not_panic() {
        let rgba = vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ];
        let mut tb = TestBackend::new(20, 4);
        tb.render(|ui| {
            let _ = ui.sixel_image(&rgba, 2, 2, 20, 2);
        });
    }

    #[test]
    fn sixel_image_on_test_backend_renders_fallback() {
        // Issue #264: a headless backend has no real TTY, so the probe never
        // runs and `capabilities()` returns the conservative default. The
        // sixel path must degrade gracefully (no panic) to the fallback string
        // rather than emitting raw protocol bytes.
        let rgba = vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ];
        let mut tb = TestBackend::new(20, 2);
        tb.render(|ui| {
            // Conservative default on a non-real terminal.
            let caps = ui.capabilities();
            assert!(!caps.sixel);
            assert!(!caps.kitty_graphics);
            assert_eq!(caps.best_blitter(), crate::Blitter::HalfBlock);
            let _ = ui.sixel_image(&rgba, 2, 2, 20, 2);
        });
        tb.assert_contains("[sixel unsupported]");
    }

    #[test]
    fn encode_sixel_rejects_oversized_dimensions() {
        // Would request ~65k × 65k × 1 byte-of-pixel-slot ≈ 4 GiB pre-fix.
        // After the MAX_IMAGE_PIXELS gate, must return empty without
        // allocating.
        let sixel = encode_sixel(&[0u8], 65_535, 65_535, 256);
        assert!(sixel.is_empty());
    }

    #[test]
    fn encode_sixel_rejects_truncated_rgba_before_pixel_allocation() {
        let sixel = encode_sixel(&[255, 0, 0, 255], 4096, 4096, 256);
        assert!(sixel.is_empty());
    }

    #[test]
    fn sixel_work_and_output_estimates_are_bounded() {
        assert!(sixel_output_budget(1024, 1024, 1).is_some());
        assert!(sixel_output_budget(4096, 4096, 216).is_none());
    }
}
