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
        WidthPolicy::default(),
    )?;
    Ok(())
}

/// Whether `cluster` is an emoji sequence whose rendered width differs
/// between terminals (VS16, ZWJ, skin-tone modifier or regional-indicator
/// flag), #441. Plain CJK and ASCII never are.
#[inline]
pub(super) fn width_uncertain_cluster(cluster: &str) -> bool {
    let classes = classify_cluster(cluster);
    [
        ClusterClasses::VS16,
        ClusterClasses::ZWJ,
        ClusterClasses::SKIN_TONE,
        ClusterClasses::FLAG,
    ]
    .into_iter()
    .any(|class| classes.intersects(class))
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
    width_policy: WidthPolicy,
) -> io::Result<bool> {
    // Returns whether any cell was emitted (#490: an idle-skipping run loop
    // stops rendering follow-up frames once a frame changes nothing).
    //
    // Run-coalescing: consecutive changed cells in the same row that share
    // `Style` + `hyperlink` + contiguous x-coordinates are emitted as one text
    // run after one cursor move and one style delta, so a full redraw costs
    // O(style-change boundaries) escape sequences rather than O(cells).
    //
    // The whole frame is assembled in `run_buf` (caller-owned and reused
    // across frames, issue #269) and written with a single `write_all`
    // (#495).
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
    // `run_buf` keeps its capacity across frames, so the hot flush loop
    // performs no per-frame allocation. Start clean but keep capacity.
    run_buf.clear();
    let mut run_abs_y: u32 = 0;
    let mut run_style: Style = Style::new();
    let mut run_link: Option<&str> = None;
    let mut run_next_col: u32 = 0;
    let mut run_open = false;

    // Helper: close the currently open run, if any. Its text already follows
    // the positioning, style and OSC 8 header in `run_buf`; record where the
    // cursor ends up after it.
    macro_rules! flush_run {
        ($stdout:expr) => {
            if run_open {
                last_cursor = Some((run_next_col, run_abs_y));
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

            // The terminal's Unicode advance (not the ambiguous-wide layout
            // width); a mismatch only costs an extra cursor move.
            let char_width = if symbol.len() == 1 {
                1
            } else {
                UnicodeWidthStr::width(&*symbol).max(1) as u32
            };
            // #441: a wide emoji cluster (VS16, ZWJ, skin tone, flag) is drawn
            // one or two columns depending on the terminal. Never trust its
            // advance: paint the covered columns blank first, then draw the
            // cluster from its own column and reposition for the next cell.
            let uncertain = char_width > 1 && width_uncertain_cluster(&symbol);

            // Decide whether this cell extends the open run or starts a new one.
            let extends = !uncertain
                && run_open
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

                let start_x = if uncertain { x.saturating_add(1) } else { x };
                let need_move = last_cursor.is_none_or(|(lx, ly)| lx != start_x || ly != abs_y);
                if need_move {
                    push_move_to(run_buf, start_x, abs_y);
                }

                if cell.style != last_style {
                    if first_style {
                        // One SGR reset (#489).
                        run_buf.push_str("\x1b[0m");
                        push_style(run_buf, &cell.style, color_depth);
                        first_style = false;
                    } else {
                        push_style_delta(run_buf, &last_style, &cell.style, color_depth);
                    }
                    last_style = cell.style;
                }

                if cell_link != active_link {
                    if let Some(url) = cell_link {
                        // `"\x1b]8;;{url}\x07"` without a per-change `format!`
                        // (issue #269).
                        run_buf.push_str("\x1b]8;;");
                        run_buf.push_str(url);
                        run_buf.push('\x07');
                    } else {
                        run_buf.push_str("\x1b]8;;\x07");
                    }
                    active_link = cell_link;
                }

                if uncertain {
                    // Blank the continuation columns in the cell's style, then
                    // draw the cluster over them from its leading column. A
                    // two-column terminal overwrites the blanks, a one-column
                    // terminal keeps one as fill; neither leaves a stale glyph.
                    for _ in 1..char_width {
                        run_buf.push(' ');
                    }
                    push_move_to(run_buf, x, abs_y);
                    run_buf.push_str(&symbol);
                    // The real cursor column is unknown: position the next cell
                    // explicitly.
                    last_cursor = None;
                    continue;
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
            run_next_col = x + char_width;
            // #449 conservative mode: after any width-uncertain cluster the
            // next cell is positioned explicitly instead of trusting the
            // terminal's advance.
            if width_policy.reposition_after(&symbol) {
                flush_run!(stdout);
                last_cursor = None;
            }
        }

        // End of row: flush whatever is buffered before moving to the next row.
        flush_run!(stdout);
    }

    if has_updates {
        if active_link.is_some() {
            run_buf.push_str("\x1b]8;;\x07");
        }
        // Single end-of-frame SGR reset (#489).
        run_buf.push_str("\x1b[0m");
    }

    if !run_buf.is_empty() {
        stdout.write_all(run_buf.as_bytes())?;
    }
    Ok(has_updates)
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

// ---------------------------------------------------------------------------
// Hand-rolled escape writer (#495)
//
// The flush builds the whole frame into one reused `String` and hands it to
// the sink with a single `write_all`. Cursor moves and SGR sequences are
// formatted here with a small integer writer instead of crossterm `queue!`
// commands, whose `Attribute::sgr` allocates a `String` per call and whose
// `fmt` plumbing dominated full-redraw frames. The bytes are identical to
// the crossterm encodings (`CSI {row+1};{col+1}H`, `CSI {sgr}m`); an oracle
// test keeps the previous writer for comparison. Every legacy-console path
// already received raw SGR colour bytes, so nothing relied on crossterm's
// WinAPI fallback for these sequences.
// ---------------------------------------------------------------------------

/// SGR parameter codes, as crossterm's `Attribute::sgr` encodes them.
mod sgr {
    pub(super) const BOLD: u32 = 1;
    pub(super) const DIM: u32 = 2;
    pub(super) const ITALIC: u32 = 3;
    pub(super) const UNDERLINED: u32 = 4;
    pub(super) const SLOW_BLINK: u32 = 5;
    pub(super) const REVERSE: u32 = 7;
    pub(super) const CROSSED_OUT: u32 = 9;
    pub(super) const NORMAL_INTENSITY: u32 = 22;
    pub(super) const NO_ITALIC: u32 = 23;
    pub(super) const NO_UNDERLINE: u32 = 24;
    pub(super) const NO_BLINK: u32 = 25;
    pub(super) const NO_REVERSE: u32 = 27;
    pub(super) const NOT_CROSSED_OUT: u32 = 29;
    pub(super) const OVER_LINED: u32 = 53;
    pub(super) const NOT_OVER_LINED: u32 = 55;
}

/// Append the decimal digits of `value`.
#[inline]
fn push_uint(out: &mut String, mut value: u32) {
    let mut digits = [0u8; 10];
    let mut start = digits.len();
    loop {
        start -= 1;
        digits[start] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    for &digit in &digits[start..] {
        out.push(char::from(digit));
    }
}

/// Append `CSI {row};{col}H` for 0-based `(x, y)`, clamped to the `u16`
/// coordinates crossterm's `MoveTo` takes.
#[inline]
pub(super) fn push_move_to(out: &mut String, x: u32, y: u32) {
    out.push_str("\x1b[");
    push_uint(out, u32::from(sat_u16(y)) + 1);
    out.push(';');
    push_uint(out, u32::from(sat_u16(x)) + 1);
    out.push('H');
}

/// Append `CSI {code}m`.
#[inline]
fn push_sgr(out: &mut String, code: u32) {
    out.push_str("\x1b[");
    push_uint(out, code);
    out.push('m');
}

/// Append the SGR transition from `old` to `new`.
pub(super) fn push_style_delta(out: &mut String, old: &Style, new: &Style, depth: ColorDepth) {
    if old.fg != new.fg {
        match new.fg {
            Some(fg) => push_sgr_color(out, fg, depth, true),
            None => out.push_str("\x1b[39m"),
        }
    }
    if old.bg != new.bg {
        match new.bg {
            Some(bg) => push_sgr_color(out, bg, depth, false),
            None => out.push_str("\x1b[49m"),
        }
    }
    let removed = Modifiers(old.modifiers.0 & !new.modifiers.0);
    let added = Modifiers(new.modifiers.0 & !old.modifiers.0);
    if removed.contains(Modifiers::BOLD) || removed.contains(Modifiers::DIM) {
        push_sgr(out, sgr::NORMAL_INTENSITY);
        if new.modifiers.contains(Modifiers::BOLD) {
            push_sgr(out, sgr::BOLD);
        }
        if new.modifiers.contains(Modifiers::DIM) {
            push_sgr(out, sgr::DIM);
        }
    } else {
        if added.contains(Modifiers::BOLD) {
            push_sgr(out, sgr::BOLD);
        }
        if added.contains(Modifiers::DIM) {
            push_sgr(out, sgr::DIM);
        }
    }
    let toggles = [
        (Modifiers::ITALIC, sgr::NO_ITALIC, sgr::ITALIC),
        (Modifiers::UNDERLINE, sgr::NO_UNDERLINE, sgr::UNDERLINED),
        (Modifiers::REVERSED, sgr::NO_REVERSE, sgr::REVERSE),
        (
            Modifiers::STRIKETHROUGH,
            sgr::NOT_CROSSED_OUT,
            sgr::CROSSED_OUT,
        ),
        (Modifiers::BLINK, sgr::NO_BLINK, sgr::SLOW_BLINK),
        (Modifiers::OVERLINE, sgr::NOT_OVER_LINED, sgr::OVER_LINED),
    ];
    for (modifier, off, on) in toggles {
        if removed.contains(modifier) {
            push_sgr(out, off);
        }
        if added.contains(modifier) {
            push_sgr(out, on);
        }
    }
    // Underline style and color use raw escapes: crossterm cannot express
    // the `CSI 4:Nm` subparameters or the `SGR 58`/`59` underline color
    // reliably (its discriminants collide on these terminals).
    if new.modifiers.contains(Modifiers::UNDERLINE)
        && (old.underline_style != new.underline_style || added.contains(Modifiers::UNDERLINE))
    {
        push_underline_style(out, new.underline_style);
    }
    if old.underline_color != new.underline_color {
        push_underline_color(out, new.underline_color, depth);
    }
}

/// Append the full SGR sequence for `style` from a reset state.
pub(super) fn push_style(out: &mut String, style: &Style, depth: ColorDepth) {
    if let Some(fg) = style.fg {
        push_sgr_color(out, fg, depth, true);
    }
    if let Some(bg) = style.bg {
        push_sgr_color(out, bg, depth, false);
    }
    let m = style.modifiers;
    let attributes = [
        (Modifiers::BOLD, sgr::BOLD),
        (Modifiers::DIM, sgr::DIM),
        (Modifiers::ITALIC, sgr::ITALIC),
        (Modifiers::UNDERLINE, sgr::UNDERLINED),
        (Modifiers::REVERSED, sgr::REVERSE),
        (Modifiers::STRIKETHROUGH, sgr::CROSSED_OUT),
        (Modifiers::BLINK, sgr::SLOW_BLINK),
        (Modifiers::OVERLINE, sgr::OVER_LINED),
    ];
    for (modifier, code) in attributes {
        if m.contains(modifier) {
            push_sgr(out, code);
        }
    }
    if style.modifiers.contains(Modifiers::UNDERLINE)
        && style.underline_style != UnderlineStyle::Straight
    {
        push_underline_style(out, style.underline_style);
    }
    if style.underline_color.is_some() {
        push_underline_color(out, style.underline_color, depth);
    }
}

/// Write the SGR transition from `old` to `new` to `w`.
#[cfg(test)]
pub(super) fn apply_style_delta(
    w: &mut impl Write,
    old: &Style,
    new: &Style,
    depth: ColorDepth,
) -> io::Result<()> {
    let mut out = String::new();
    push_style_delta(&mut out, old, new, depth);
    w.write_all(out.as_bytes())
}

/// Write the full SGR sequence for `style` to `w`.
#[cfg(test)]
pub(super) fn apply_style(w: &mut impl Write, style: &Style, depth: ColorDepth) -> io::Result<()> {
    let mut out = String::new();
    push_style(&mut out, style, depth);
    w.write_all(out.as_bytes())
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

/// Append `CSI 4:{n}m` for `style`.
fn push_underline_style(out: &mut String, style: UnderlineStyle) {
    out.push_str("\x1b[4:");
    push_uint(out, u32::from(underline_style_param(style)));
    out.push('m');
}

/// Append the raw `SGR 58` underline-color sequence (or `SGR 59` to reset).
///
/// `None` resets the underline color to the foreground (`\x1b[59m`). Otherwise
/// the color is downsampled to the terminal's depth: true-color emits
/// `\x1b[58:2::r:g:bm`, while indexed/named colors emit `\x1b[58:5:im`.
fn push_underline_color(out: &mut String, color: Option<Color>, depth: ColorDepth) {
    let rgb = |out: &mut String, (r, g, b): (u8, u8, u8)| {
        out.push_str("\x1b[58:2::");
        push_uint(out, u32::from(r));
        out.push(':');
        push_uint(out, u32::from(g));
        out.push(':');
        push_uint(out, u32::from(b));
        out.push('m');
    };
    match color {
        None => out.push_str("\x1b[59m"),
        Some(c) => match c.downsampled(depth) {
            Color::Reset => out.push_str("\x1b[59m"),
            Color::Rgb(r, g, b) => rgb(out, (r, g, b)),
            Color::Indexed(i) => {
                out.push_str("\x1b[58:5:");
                push_uint(out, u32::from(i));
                out.push('m');
            }
            // Named colors have no direct SGR-58 form; resolve them to their
            // RGB equivalent and emit a true-color underline sequence.
            named => rgb(out, named.to_rgb()),
        },
    }
}

/// Append the SGR sequence selecting `color` as foreground or background.
fn push_sgr_color(out: &mut String, color: Color, depth: ColorDepth, foreground: bool) {
    match color.downsampled(depth) {
        Color::Reset => push_sgr(out, if foreground { 39 } else { 49 }),
        Color::Rgb(r, g, b) => {
            out.push_str(if foreground {
                "\x1b[38;2;"
            } else {
                "\x1b[48;2;"
            });
            push_uint(out, u32::from(r));
            out.push(';');
            push_uint(out, u32::from(g));
            out.push(';');
            push_uint(out, u32::from(b));
            out.push('m');
        }
        Color::Indexed(i) => {
            out.push_str(if foreground {
                "\x1b[38;5;"
            } else {
                "\x1b[48;5;"
            });
            push_uint(out, u32::from(i));
            out.push('m');
        }
        named => push_sgr(out, u32::from(named_sgr_code(named, foreground))),
    }
}

pub(super) fn named_sgr_code(color: Color, foreground: bool) -> u8 {
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
