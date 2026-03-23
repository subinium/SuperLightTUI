use super::*;

#[inline]
pub(crate) fn byte_index_for_char(value: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }
    value
        .char_indices()
        .nth(char_index)
        .map_or(value.len(), |(idx, _)| idx)
}

pub(crate) fn format_token_count(count: usize) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}k", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

pub(crate) fn format_table_row(cells: &[String], widths: &[u32], separator: &str) -> String {
    let sep_width = UnicodeWidthStr::width(separator);
    let total_cells_width: usize = widths.iter().map(|w| *w as usize).sum();
    let mut row = String::with_capacity(
        total_cells_width + sep_width.saturating_mul(widths.len().saturating_sub(1)),
    );
    for (i, width) in widths.iter().enumerate() {
        if i > 0 {
            row.push_str(separator);
        }
        let cell = cells.get(i).map(String::as_str).unwrap_or("");
        let cell_width = UnicodeWidthStr::width(cell) as u32;
        let padding = (*width).saturating_sub(cell_width) as usize;
        row.push_str(cell);
        row.extend(std::iter::repeat(' ').take(padding));
    }
    row
}

pub(crate) fn table_visible_len(state: &TableState) -> usize {
    if state.page_size == 0 {
        return state.visible_indices().len();
    }

    let start = state
        .page
        .saturating_mul(state.page_size)
        .min(state.visible_indices().len());
    let end = (start + state.page_size).min(state.visible_indices().len());
    end.saturating_sub(start)
}

pub(crate) fn handle_vertical_nav(
    selected: &mut usize,
    max_index: usize,
    key_code: KeyCode,
) -> bool {
    match key_code {
        KeyCode::Up | KeyCode::Char('k') => {
            if *selected > 0 {
                *selected -= 1;
                true
            } else {
                false
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if *selected < max_index {
                *selected += 1;
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

pub(crate) fn format_compact_number(value: f64) -> String {
    if value.fract().abs() < f64::EPSILON {
        return format!("{value:.0}");
    }

    let mut s = format!("{value:.2}");
    while s.contains('.') && s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    s
}

pub(crate) fn center_text(text: &str, width: usize) -> String {
    let text_width = UnicodeWidthStr::width(text);
    if text_width >= width {
        return text.to_string();
    }

    let total = width - text_width;
    let left = total / 2;
    let right = total - left;
    let mut centered = String::with_capacity(width);
    centered.extend(std::iter::repeat(' ').take(left));
    centered.push_str(text);
    centered.extend(std::iter::repeat(' ').take(right));
    centered
}

pub(crate) struct TextareaVLine {
    pub(crate) logical_row: usize,
    pub(crate) char_start: usize,
    pub(crate) char_count: usize,
}

pub(crate) fn textarea_build_visual_lines(lines: &[String], wrap_width: u32) -> Vec<TextareaVLine> {
    let mut out = Vec::new();
    for (row, line) in lines.iter().enumerate() {
        if line.is_empty() || wrap_width == u32::MAX {
            out.push(TextareaVLine {
                logical_row: row,
                char_start: 0,
                char_count: line.chars().count(),
            });
            continue;
        }
        let mut seg_start = 0usize;
        let mut seg_chars = 0usize;
        let mut seg_width = 0u32;
        for (idx, ch) in line.chars().enumerate() {
            let cw = UnicodeWidthChar::width(ch).unwrap_or(0) as u32;
            if seg_width + cw > wrap_width && seg_chars > 0 {
                out.push(TextareaVLine {
                    logical_row: row,
                    char_start: seg_start,
                    char_count: seg_chars,
                });
                seg_start = idx;
                seg_chars = 0;
                seg_width = 0;
            }
            seg_chars += 1;
            seg_width += cw;
        }
        out.push(TextareaVLine {
            logical_row: row,
            char_start: seg_start,
            char_count: seg_chars,
        });
    }
    out
}

pub(crate) fn textarea_logical_to_visual(
    vlines: &[TextareaVLine],
    logical_row: usize,
    logical_col: usize,
) -> (usize, usize) {
    for (i, vl) in vlines.iter().enumerate() {
        if vl.logical_row != logical_row {
            continue;
        }
        let seg_end = vl.char_start + vl.char_count;
        if logical_col >= vl.char_start && logical_col < seg_end {
            return (i, logical_col - vl.char_start);
        }
        if logical_col == seg_end {
            let is_last_seg = vlines
                .get(i + 1)
                .map_or(true, |next| next.logical_row != logical_row);
            if is_last_seg {
                return (i, logical_col - vl.char_start);
            }
        }
    }
    (vlines.len().saturating_sub(1), 0)
}

pub(crate) fn textarea_visual_to_logical(
    vlines: &[TextareaVLine],
    visual_row: usize,
    visual_col: usize,
) -> (usize, usize) {
    if let Some(vl) = vlines.get(visual_row) {
        let logical_col = vl.char_start + visual_col.min(vl.char_count);
        (vl.logical_row, logical_col)
    } else {
        (0, 0)
    }
}

#[allow(unused_variables)]
pub(crate) fn open_url(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(url).spawn()?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open").arg(url).spawn()?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", "", url])
            .spawn()?;
    }
    Ok(())
}
