use super::*;

/// Byte offset of the `char_index`-th Unicode scalar boundary (clamped to
/// `value.len()`).
///
/// Prefer [`byte_index_for_grapheme`] at cursor / wrap sites: a scalar index
/// can fall inside a grapheme cluster (e.g. between the two regional indicators
/// of a flag emoji, or between a base char and its combining mark), so slicing
/// at a scalar boundary can cut a user-perceived character in half. This scalar
/// form is retained only for the few remaining callers whose state column is
/// still defined in scalar terms.
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

/// Number of extended grapheme clusters (user-perceived characters) in `s`.
///
/// This is the cluster-aware replacement for `s.chars().count()` at cursor /
/// column sites. A ZWJ flag (`🇰🇷`), family emoji (`👨‍👩‍👧‍👦`), Devanagari
/// syllable (`क्षि`), or Thai cluster (`กำ`) each counts as one.
#[inline]
pub(crate) fn grapheme_count(s: &str) -> usize {
    s.graphemes(true).count()
}

/// Byte offset of the `cluster_index`-th extended-grapheme-cluster boundary
/// (clamped to `s.len()`).
///
/// Replaces the scalar-based [`byte_index_for_char`] at cursor sites so that a
/// slice / insert / delete never falls inside a cluster.
#[inline]
pub(crate) fn byte_index_for_grapheme(s: &str, cluster_index: usize) -> usize {
    if cluster_index == 0 {
        return 0;
    }
    s.grapheme_indices(true)
        .nth(cluster_index)
        .map_or(s.len(), |(idx, _)| idx)
}

/// Display width (in terminal columns) of a single grapheme cluster string.
///
/// Measured on the whole cluster via [`UnicodeWidthStr::width`], which is
/// correct for ZWJ emoji — a cluster's column count is the width of its visible
/// glyph, not the per-scalar sum.
#[inline]
pub(crate) fn cluster_width(cluster: &str) -> u32 {
    UnicodeWidthStr::width(cluster) as u32
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
        row.push_str(&clamp_table_cell(
            cells.get(i).map(String::as_str).unwrap_or(""),
            *width,
        ));
    }
    row
}

/// Pad or truncate `cell` so its display width is exactly `width` cells.
///
/// Shorter content is right-padded with spaces (current behavior); longer
/// content is truncated with a `…` ellipsis. With an `Auto` column the
/// resolved width already equals the content width, so this is a pure pad —
/// preserving the pre-v0.21 string-grid output byte-for-byte.
pub(crate) fn clamp_table_cell(cell: &str, width: u32) -> String {
    let width = width as usize;
    let cell_width = UnicodeWidthStr::width(cell);
    if cell_width <= width {
        let mut out = String::with_capacity(width);
        out.push_str(cell);
        out.extend(std::iter::repeat(' ').take(width - cell_width));
        return out;
    }
    if width == 0 {
        return String::new();
    }
    if width == 1 {
        return "\u{2026}".to_string();
    }
    let target = width - 1;
    let mut out = String::with_capacity(width);
    let mut acc = 0usize;
    for ch in cell.chars() {
        let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if acc + ch_width > target {
            break;
        }
        out.push(ch);
        acc += ch_width;
    }
    out.push('\u{2026}');
    // Pad in case the last char was wide and left a one-cell gap before `…`.
    let out_width = UnicodeWidthStr::width(out.as_str());
    out.extend(std::iter::repeat(' ').take(width.saturating_sub(out_width)));
    out
}

pub(crate) fn table_visible_len(state: &TableState) -> usize {
    let visible = state.visible_indices();
    if state.page_size == 0 {
        return visible.len();
    }

    let start = state
        .page
        .saturating_mul(state.page_size)
        .min(visible.len());
    let end = (start + state.page_size).min(visible.len());
    end.saturating_sub(start)
}

pub(crate) fn handle_vertical_nav(
    selected: &mut usize,
    max_index: usize,
    key_code: KeyCode,
) -> bool {
    match key_code {
        KeyCode::Up | KeyCode::Char('k') if *selected > 0 => {
            *selected -= 1;
            true
        }
        KeyCode::Down | KeyCode::Char('j') if *selected < max_index => {
            *selected += 1;
            true
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
    /// Cluster index (extended grapheme cluster) of this visual segment's
    /// start within its logical row.
    pub(crate) char_start: usize,
    /// Number of grapheme clusters this visual segment spans.
    pub(crate) char_count: usize,
}

/// Build the visual (soft-wrapped) line layout for a textarea.
///
/// `char_start` / `char_count` are **grapheme-cluster** indices, not scalar
/// indices, so a soft-wrap break never lands inside a cluster (a ZWJ emoji or
/// combining sequence stays whole on one visual line).
pub(crate) fn textarea_build_visual_lines(lines: &[String], wrap_width: u32) -> Vec<TextareaVLine> {
    let mut out = Vec::new();
    for (row, line) in lines.iter().enumerate() {
        if line.is_empty() || wrap_width == u32::MAX {
            out.push(TextareaVLine {
                logical_row: row,
                char_start: 0,
                char_count: grapheme_count(line),
            });
            continue;
        }
        let mut seg_start = 0usize;
        let mut seg_chars = 0usize;
        let mut seg_width = 0u32;
        for (idx, g) in line.graphemes(true).enumerate() {
            let cw = cluster_width(g);
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
