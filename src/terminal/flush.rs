//! Buffer diff flushing, cursor placement and SGR style-delta emission.

use super::*;

#[allow(clippy::too_many_arguments)]
#[allow(unused_assignments)]
pub(super) fn flush_buffer_diff(
    stdout: &mut impl Write,
    current: &Buffer,
    previous: &Buffer,
    color_depth: ColorDepth,
    row_offset: u32,
    run_buf: &mut String,
) -> io::Result<()> {
    flush_buffer_diff_rows(
        stdout,
        current,
        previous,
        color_depth,
        row_offset,
        run_buf,
        &[],
    )
}

#[allow(clippy::too_many_arguments)]
#[allow(unused_assignments)]
pub(super) fn flush_buffer_diff_rows(
    stdout: &mut impl Write,
    current: &Buffer,
    previous: &Buffer,
    color_depth: ColorDepth,
    row_offset: u32,
    run_buf: &mut String,
    forced_rows: &[u32],
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

    // Issue #486: rows are compared through the written-span invariant
    // instead of per-row digests. A row untouched in both buffers with the
    // same fill is skipped in O(1); otherwise only the columns between the
    // first and last differing cell are scanned. Forced rows (sprixel
    // repaints) and buffers without a shared layout scan every column.
    let layout_shared = crate::buffer::same_layout(current, previous);
    let width = current.area.width as usize;
    for y in current.area.y..current.area.bottom() {
        let force_row = forced_rows.binary_search(&y).is_ok();
        let row = (y - current.area.y) as usize;
        let (mut lo, hi) = if force_row || !layout_shared {
            (0, width)
        } else {
            match crate::buffer::row_diff_range(current, previous, row) {
                Some(range) => range,
                None => continue,
            }
        };
        // `(current, previous)` cells of column `col` on this row.
        let cells_at = |col: usize| -> (&crate::cell::Cell, Option<&crate::cell::Cell>) {
            if layout_shared {
                let index = row * width + col;
                (&current.content[index], Some(&previous.content[index]))
            } else {
                let x = current.area.x + col as u32;
                (current.get(x, y), previous.try_get(x, y))
            }
        };
        // A changed continuation cell is drawn by its leading cell, which may
        // sit left of the first differing column.
        while lo > 0 && lo < hi && cells_at(lo).0.is_continuation() {
            lo -= 1;
        }
        for col in lo..hi {
            let x = current.area.x + col as u32;
            let (cell, prev) = cells_at(col);
            // Compare before any symbol work; continuation cells (empty
            // symbol) are drawn by their leading cell.
            if cell.is_continuation() {
                flush_run!(stdout);
                continue;
            }
            let mut unchanged = !force_row && prev == Some(cell);
            // A leading cell is redrawn when any of its continuation cells
            // changed, so a stale glyph cannot survive behind it.
            let mut next = col + 1;
            while unchanged && next < width {
                let (cont, prev_cont) = cells_at(next);
                if !cont.is_continuation() {
                    break;
                }
                unchanged = prev_cont == Some(cont);
                next += 1;
            }
            if unchanged {
                // Gap — any open run on this row must be flushed.
                flush_run!(stdout);
                continue;
            }
            let symbol = cell.emit_symbol();

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
                        // One SGR reset (#489): crossterm's `ResetColor` and
                        // `SetAttribute(Reset)` both encode `CSI 0m`, and only
                        // `ResetColor` acts on the legacy WinAPI console.
                        queue!(stdout, ResetColor)?;
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
            run_buf.push_str(&symbol);
            let char_width = crate::cell::display_width(&symbol).max(1) as u32;
            if char_width > 1 && symbol.chars().any(|c| c == '\u{FE0F}') {
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
        // Single end-of-frame SGR reset (#489).
        queue!(stdout, ResetColor)?;
    }

    Ok(())
}

pub(super) fn flush_cursor(
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

pub(super) fn apply_style_delta(
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
    // Underline style and color use raw escapes: crossterm 0.29 cannot
    // express the `CSI 4:Nm` subparameters or the `SGR 58`/`59` underline
    // color reliably (its discriminants collide on these terminals).
    if new.modifiers.contains(Modifiers::UNDERLINE)
        && (old.underline_style != new.underline_style || added.contains(Modifiers::UNDERLINE))
    {
        write!(w, "\x1b[4:{}m", underline_style_param(new.underline_style))?;
    }
    if old.underline_color != new.underline_color {
        emit_underline_color(w, new.underline_color, depth)?;
    }
    Ok(())
}

/// Map an [`UnderlineStyle`] to its `CSI 4:Nm` subparameter value.
pub(super) fn underline_style_param(style: UnderlineStyle) -> u8 {
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

pub(super) fn apply_style(w: &mut impl Write, style: &Style, depth: ColorDepth) -> io::Result<()> {
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
    if style.modifiers.contains(Modifiers::UNDERLINE)
        && style.underline_style != UnderlineStyle::Straight
    {
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

pub(super) fn reset_current_buffer(buffer: &mut Buffer, theme_bg: Option<Color>) {
    if let Some(bg) = theme_bg {
        buffer.reset_with_bg(bg);
    } else {
        buffer.reset();
    }
}
