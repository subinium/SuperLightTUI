const SIXEL_START: &str = "\x1bPq";
const SIXEL_END: &str = "\x1b\\";

pub(crate) fn encode_sixel(rgba: &[u8], width: u32, height: u32, max_colors: u32) -> String {
    if rgba.is_empty() || width == 0 || height == 0 {
        return String::new();
    }

    let width_usize = width as usize;
    let height_usize = height as usize;
    let pixel_count = width_usize.saturating_mul(height_usize);
    if pixel_count == 0 {
        return String::new();
    }

    let color_limit = max_colors.clamp(1, 216) as usize;

    let mut pixels: Vec<Option<u8>> = vec![None; pixel_count];
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

    let mut out = String::new();
    out.push_str(SIXEL_START);

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

        for (idx, reg) in row_regs.iter().enumerate() {
            out.push('#');
            out.push_str(&reg.to_string());

            let mut encoded = String::with_capacity(width_usize);
            for x in 0..width_usize {
                let mut bits = 0_u8;
                for bit in 0..6 {
                    let y = y_base + bit;
                    if y >= height_usize {
                        break;
                    }
                    let pidx = y * width_usize + x;
                    if pixels[pidx] == Some(*reg) {
                        bits |= 1 << bit;
                    }
                }
                encoded.push((b'?' + bits) as char);
            }

            push_rle_encoded(&mut out, &encoded);
            if idx + 1 < row_regs.len() {
                out.push('$');
            }
        }

        out.push('-');
    }

    out.push_str(SIXEL_END);
    out
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
) -> Vec<u8> {
    let mut used = vec![false; reg_count];

    for bit in 0..6 {
        let y = y_base + bit;
        if y >= height {
            break;
        }
        let start = y * width;
        let end = start + width;
        for &pixel in &pixels[start..end] {
            if let Some(reg) = pixel {
                used[reg as usize] = true;
            }
        }
    }

    used.into_iter()
        .enumerate()
        .filter_map(|(reg, is_used)| is_used.then_some(reg as u8))
        .collect()
}

fn push_rle_encoded(out: &mut String, data: &str) {
    if data.is_empty() {
        return;
    }

    let mut chars = data.chars();
    let Some(mut current) = chars.next() else {
        return;
    };
    let mut run_len = 1_usize;

    for ch in chars {
        if ch == current {
            run_len += 1;
            continue;
        }

        push_run(out, current, run_len);
        current = ch;
        run_len = 1;
    }

    push_run(out, current, run_len);
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
    use super::encode_sixel;
    use crate::TestBackend;

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
}
