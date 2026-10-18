//! Differential tests: the span-skipping flush (#486) must emit exactly the
//! bytes of the pre-#486 per-cell compare, which is kept below as a
//! reference copy without the per-row skip.

use super::*;

/// Reference copy of `flush_buffer_diff_rows` before #486, minus its row
/// hash skip: every cell of every row is compared and normalized. It shares
/// one rule added with #486: a leading cell whose continuation cells changed
/// is redrawn.
#[allow(unused_assignments)]
fn reference_flush_rows(
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
                        queue!(stdout, ResetColor, SetAttribute(Attribute::Reset))?;
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
            let char_width = UnicodeWidthStr::width(symbol.as_str()).max(1) as u32;
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
        queue!(stdout, ResetColor, SetAttribute(Attribute::Reset))?;
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
                &mut String::new(), &forced,
            ).unwrap();
            let mut reference = Vec::new();
            reference_flush_rows(
                &mut reference, &current, &previous, ColorDepth::TrueColor, row_offset,
                &mut String::new(), &forced,
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
        )
        .unwrap();
        let text = String::from_utf8_lossy(&forced);
        assert!(text.contains("abc") && !text.contains("link"), "{text:?}");
    }
}
