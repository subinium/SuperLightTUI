//! Differential tests: the span-skipping flush (#486) must emit exactly the
//! bytes of the pre-#486 per-cell compare, which is kept below as a
//! reference copy without the per-row skip.

use super::*;

/// The SGR writer before #495, built on crossterm `queue!` commands and
/// `write!` formatting. `push_style` / `push_style_delta` and the flush must
/// stay byte-identical to it.
mod old_writer {
    use super::*;

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

    pub(super) fn apply_style(
        w: &mut impl Write,
        style: &Style,
        depth: ColorDepth,
    ) -> io::Result<()> {
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
}

/// Reference copy of `flush_buffer_diff_rows` before #486, minus its row
/// hash skip: every cell of every row is compared and normalized. It shares
/// one rule added with #486: a leading cell whose continuation cells changed
/// is redrawn, the single SGR reset per frame site from #489, and the #441
/// emoji-cluster repositioning plus #449 conservative repositioning. It still
/// writes through crossterm `queue!` and the [`old_writer`] SGR encoder, so the
/// differential test also proves the #495 hand-rolled writer byte-identical.
#[allow(unused_assignments)]
fn reference_flush_rows(
    stdout: &mut impl Write,
    current: &Buffer,
    previous: &Buffer,
    color_depth: ColorDepth,
    row_offset: u32,
    run_buf: &mut String,
    forced_rows: &[u32],
    width_policy: WidthPolicy,
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

    for y in current.area.y..current.area.bottom() {
        let force_row = forced_rows.binary_search(&y).is_ok();
        for x in current.area.x..current.area.right() {
            let cell = current.get(x, y);
            let prev = previous.get(x, y);
            let symbol = cell.normalized_symbol();
            // Shared rule (#486): a leading cell whose continuation cells
            // changed is redrawn.
            let mut unchanged = !force_row && cell == prev;
            let mut next = x + 1;
            while unchanged && next < current.area.right() && current.get(next, y).is_continuation()
            {
                unchanged = current.get(next, y) == previous.get(next, y);
                next += 1;
            }
            if unchanged || symbol.is_empty() {
                // Gap — any open run on this row must be flushed.
                flush_run!(stdout);
                continue;
            }

            let abs_y = row_offset + y;
            // Defense-in-depth: `Cell::hyperlink` is a public field that can
            // be written directly. `set_string_linked` pre-sanitizes, but a
            // direct write could still smuggle control bytes into the OSC 8
            // payload. Validate here before flushing to stdout.
            let cell_link = cell
                .hyperlink
                .as_deref()
                .filter(|u| crate::buffer::is_valid_osc8_url(u));

            let char_width = UnicodeWidthStr::width(symbol.as_str()).max(1) as u32;
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
                    queue!(stdout, cursor::MoveTo(sat_u16(start_x), sat_u16(abs_y)))?;
                }

                if cell.style != last_style {
                    if first_style {
                        queue!(stdout, ResetColor)?;
                        old_writer::apply_style(stdout, &cell.style, color_depth)?;
                        first_style = false;
                    } else {
                        old_writer::apply_style_delta(
                            stdout,
                            &last_style,
                            &cell.style,
                            color_depth,
                        )?;
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

                if uncertain {
                    for _ in 1..char_width {
                        queue!(stdout, Print(" "))?;
                    }
                    queue!(
                        stdout,
                        cursor::MoveTo(sat_u16(x), sat_u16(abs_y)),
                        Print(&symbol)
                    )?;
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
            queue!(stdout, Print("\x1b]8;;\x07"))?;
        }
        queue!(stdout, ResetColor)?;
    }

    Ok(())
}

mod differential {
    use super::*;
    use crate::cell::test_strategies::cell_text;
    use proptest::prelude::*;

    #[derive(Debug, Clone)]
    enum Op {
        Text(u32, u32, String, u8),
        Linked(u32, u32, String),
        Char(u32, u32, char, u8),
        Fill(u32, u32, u32, u32, u8),
        Dim,
        DimAround(u32, u32, u32, u32),
        /// Direct public mutation through `get_mut` (recorded in the spans).
        RawSymbol(u32, u32, String),
        RawLink(u32, u32, String),
        Clip(u32, u32, u32, u32),
        Unclip,
    }

    fn style_of(index: u8) -> Style {
        match index % 6 {
            0 => Style::new(),
            1 => Style::new().fg(Color::Red),
            2 => Style::new().bg(Color::Rgb(1, 2, 3)).bold(),
            3 => Style::new().fg(Color::Indexed(200)).underline().italic(),
            4 => Style::new().bg(Color::Blue).dim().reversed(),
            _ => Style::new().fg(Color::Rgb(9, 8, 7)).strikethrough(),
        }
    }

    fn op() -> impl Strategy<Value = Op> {
        let c = 0u32..14;
        prop_oneof![
            8 => (c.clone(), c.clone(), cell_text(), any::<u8>())
                .prop_map(|(x, y, t, s)| Op::Text(x, y, t, s)),
            1 => (c.clone(), c.clone(), cell_text()).prop_map(|(x, y, t)| Op::Linked(x, y, t)),
            3 => (c.clone(), c.clone(), any::<char>(), any::<u8>())
                .prop_map(|(x, y, ch, s)| Op::Char(x, y, ch, s)),
            2 => (c.clone(), c.clone(), 0u32..10, 0u32..6, any::<u8>())
                .prop_map(|(x, y, w, h, s)| Op::Fill(x, y, w, h, s)),
            1 => Just(Op::Dim),
            1 => (c.clone(), c.clone(), 0u32..8, 0u32..5)
                .prop_map(|(x, y, w, h)| Op::DimAround(x, y, w, h)),
            1 => (c.clone(), c.clone(), prop_oneof![
                Just("\u{1b}]52;c;x".to_string()),
                Just("ab".to_string()),
                Just(String::new()),
                Just("e\u{301}".to_string()),
                Just("\r\n".to_string()),
            ])
                .prop_map(|(x, y, t)| Op::RawSymbol(x, y, t)),
            1 => (c.clone(), c.clone(), prop_oneof![
                Just("https://ok.example".to_string()),
                Just("https://bad\u{7}.example".to_string()),
            ])
                .prop_map(|(x, y, t)| Op::RawLink(x, y, t)),
            1 => (c.clone(), c.clone(), 0u32..12, 0u32..6).prop_map(|(x, y, w, h)| Op::Clip(x, y, w, h)),
            1 => Just(Op::Unclip),
        ]
    }

    fn apply(buf: &mut Buffer, op: &Op) {
        match op {
            Op::Text(x, y, t, s) => buf.set_string(*x, *y, t, style_of(*s)),
            Op::Linked(x, y, t) => buf.set_string_linked(*x, *y, t, Style::new(), "https://e.x/l"),
            Op::Char(x, y, ch, s) => buf.set_char(*x, *y, *ch, style_of(*s)),
            Op::Fill(x, y, w, h, s) => buf.fill_rect(Rect::new(*x, *y, *w, *h), style_of(*s)),
            Op::Dim => crate::layout::render_test_hooks::dim_entire(buf),
            Op::DimAround(x, y, w, h) => {
                crate::layout::render_test_hooks::dim_around(buf, Rect::new(*x, *y, *w, *h))
            }
            Op::RawSymbol(x, y, t) => {
                if let Some(cell) = buf.try_get_mut(*x, *y) {
                    cell.symbol = compact_str::CompactString::new(t);
                }
            }
            Op::RawLink(x, y, t) => {
                if let Some(cell) = buf.try_get_mut(*x, *y) {
                    cell.hyperlink = Some(compact_str::CompactString::new(t));
                }
            }
            Op::Clip(x, y, w, h) => buf.push_clip(Rect::new(*x, *y, *w, *h)),
            Op::Unclip => buf.pop_clip(),
        }
    }

    fn reset(buf: &mut Buffer, fill: u8) {
        match fill % 3 {
            0 => buf.reset(),
            1 => buf.reset_with_bg(Color::Rgb(20, 20, 30)),
            _ => buf.reset_with_bg(Color::Indexed(4)),
        }
    }

    /// A frame as the run loop produces it: earlier content, a reset (span
    /// refill or full refill depending on the fill), then this frame's ops.
    fn frame(area: Rect, before: &[Op], fill: u8, ops: &[Op]) -> Buffer {
        let mut buf = Buffer::empty(area);
        for op in before {
            apply(&mut buf, op);
        }
        reset(&mut buf, fill);
        for op in ops {
            apply(&mut buf, op);
        }
        buf
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1024))]

        #[test]
        fn span_skipping_flush_emits_the_same_bytes_as_full_compare(
            origin in prop_oneof![Just((0u32, 0u32)), Just((2u32, 1u32))],
            width in 1u32..14,
            height in 1u32..7,
            prev_before in prop::collection::vec(op(), 0..6),
            prev_fill in any::<u8>(),
            prev_ops in prop::collection::vec(op(), 0..12),
            cur_before in prop::collection::vec(op(), 0..6),
            cur_fill in any::<u8>(),
            cur_ops in prop::collection::vec(op(), 0..12),
            forced in prop::collection::btree_set(0u32..8, 0..3),
            row_offset in 0u32..3,
        ) {
            let area = Rect::new(origin.0, origin.1, width, height);
            let previous = frame(area, &prev_before, prev_fill, &prev_ops);
            let current = frame(area, &cur_before, cur_fill, &cur_ops);
            let forced: Vec<u32> = forced.into_iter().map(|row| row + origin.1).collect();

            let mut fast = Vec::new();
            flush_buffer_diff_rows(
                &mut fast, &current, &previous, ColorDepth::TrueColor, row_offset,
                &mut String::new(), &forced, WidthPolicy::default(),
            ).unwrap();
            let mut reference = Vec::new();
            reference_flush_rows(
                &mut reference, &current, &previous, ColorDepth::TrueColor, row_offset,
                &mut String::new(), &forced, WidthPolicy::default(),
            ).unwrap();
            prop_assert_eq!(
                String::from_utf8_lossy(&fast),
                String::from_utf8_lossy(&reference)
            );

            // `Buffer::diff` must agree with an exhaustive per-cell compare.
            let expected: Vec<(u32, u32)> = (area.y..area.bottom())
                .flat_map(|y| (area.x..area.right()).map(move |x| (x, y)))
                .filter(|&(x, y)| current.get(x, y) != previous.get(x, y))
                .collect();
            let actual: Vec<(u32, u32)> =
                current.diff(&previous).into_iter().map(|(x, y, _)| (x, y)).collect();
            prop_assert_eq!(actual, expected);
        }
    }

    #[test]
    fn rows_differing_only_in_link_style_or_continuation_are_emitted() {
        let area = Rect::new(0, 0, 6, 3);
        let mut previous = Buffer::empty(area);
        let mut current = Buffer::empty(area);
        previous.set_string(0, 0, "link", Style::new());
        current.set_string_linked(0, 0, "link", Style::new(), "https://example.com");
        previous.set_string(0, 1, "abc", Style::new());
        current.set_string(0, 1, "abc", Style::new().fg(Color::Red));
        previous.set_string(0, 2, "世", Style::new());
        current.set_string(0, 2, "世", Style::new());
        previous.get_mut(1, 2).set_char('z');

        let mut fast = Vec::new();
        flush_buffer_diff_rows(
            &mut fast,
            &current,
            &previous,
            ColorDepth::TrueColor,
            0,
            &mut String::new(),
            &[],
            WidthPolicy::default(),
        )
        .unwrap();
        let text = String::from_utf8_lossy(&fast);
        assert!(
            text.contains("\x1b]8;;https://example.com\x07link"),
            "{text:?}"
        );
        assert!(text.contains("abc"), "{text:?}");
        assert!(text.contains("世"), "{text:?}");

        // Identical rows are skipped, but forced rows are re-emitted.
        let mut same = Vec::new();
        flush_buffer_diff_rows(
            &mut same,
            &current,
            &current,
            ColorDepth::TrueColor,
            0,
            &mut String::new(),
            &[],
            WidthPolicy::default(),
        )
        .unwrap();
        assert!(same.is_empty());
        let mut forced = Vec::new();
        flush_buffer_diff_rows(
            &mut forced,
            &current,
            &current,
            ColorDepth::TrueColor,
            0,
            &mut String::new(),
            &[1],
            WidthPolicy::default(),
        )
        .unwrap();
        let text = String::from_utf8_lossy(&forced);
        assert!(text.contains("abc") && !text.contains("link"), "{text:?}");
    }
}

mod writer_equivalence {
    use super::*;
    use proptest::prelude::*;

    fn color() -> impl Strategy<Value = Color> {
        prop_oneof![
            Just(Color::Reset),
            prop::sample::select(vec![
                Color::Black,
                Color::Red,
                Color::Green,
                Color::Yellow,
                Color::Blue,
                Color::Magenta,
                Color::Cyan,
                Color::White,
                Color::DarkGray,
                Color::LightRed,
                Color::LightGreen,
                Color::LightYellow,
                Color::LightBlue,
                Color::LightMagenta,
                Color::LightCyan,
                Color::LightWhite,
            ]),
            (any::<u8>(), any::<u8>(), any::<u8>()).prop_map(|(r, g, b)| Color::Rgb(r, g, b)),
            any::<u8>().prop_map(Color::Indexed),
        ]
    }

    fn style() -> impl Strategy<Value = Style> {
        (
            prop::option::of(color()),
            prop::option::of(color()),
            any::<u8>(),
            prop::option::of(color()),
            prop::sample::select(vec![
                UnderlineStyle::Straight,
                UnderlineStyle::Double,
                UnderlineStyle::Curly,
                UnderlineStyle::Dotted,
                UnderlineStyle::Dashed,
            ]),
        )
            .prop_map(
                |(fg, bg, modifiers, underline_color, underline_style)| Style {
                    fg,
                    bg,
                    modifiers: Modifiers(modifiers),
                    underline_color,
                    underline_style,
                },
            )
    }

    fn depth() -> impl Strategy<Value = ColorDepth> {
        prop::sample::select(vec![
            ColorDepth::TrueColor,
            ColorDepth::EightBit,
            ColorDepth::Basic,
            ColorDepth::NoColor,
        ])
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(4096))]

        #[test]
        fn style_writer_matches_crossterm_writer(old in style(), new in style(), depth in depth()) {
            let mut expected = Vec::new();
            old_writer::apply_style(&mut expected, &new, depth).unwrap();
            let mut actual = String::new();
            push_style(&mut actual, &new, depth);
            prop_assert_eq!(actual.as_bytes(), expected.as_slice());

            let mut expected = Vec::new();
            old_writer::apply_style_delta(&mut expected, &old, &new, depth).unwrap();
            let mut actual = String::new();
            push_style_delta(&mut actual, &old, &new, depth);
            prop_assert_eq!(actual.as_bytes(), expected.as_slice());
        }

        #[test]
        fn move_to_matches_crossterm(x in 0u32..70_000, y in 0u32..70_000) {
            // crossterm adds one in u16 arithmetic, so its encoding is only
            // defined below u16::MAX; both clamp the input with `sat_u16`.
            prop_assume!(x < u32::from(u16::MAX) && y < u32::from(u16::MAX));
            let mut expected = Vec::new();
            queue!(expected, cursor::MoveTo(sat_u16(x), sat_u16(y))).unwrap();
            let mut actual = String::new();
            push_move_to(&mut actual, x, y);
            prop_assert_eq!(actual.as_bytes(), expected.as_slice());
        }
    }

    #[test]
    fn move_to_clamps_at_the_u16_edge() {
        let mut out = String::new();
        push_move_to(&mut out, u32::MAX, 0);
        assert_eq!(out, "\x1b[1;65536H");
    }
}
