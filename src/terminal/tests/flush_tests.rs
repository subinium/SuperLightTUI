//! Buffer diff flush, SGR style-delta and output sink tests.

use super::*;

#[test]
fn underline_extended_to_plain_never_reenables() {
    for style in [
        UnderlineStyle::Straight,
        UnderlineStyle::Double,
        UnderlineStyle::Curly,
        UnderlineStyle::Dotted,
        UnderlineStyle::Dashed,
    ] {
        let old = Style {
            modifiers: Modifiers::UNDERLINE,
            underline_style: style,
            ..Style::default()
        };
        let bytes = delta_bytes(&old, &Style::default());
        assert!(contains_seq(&bytes, b"\x1b[24m"));
        assert!(!contains_seq(&bytes, b"\x1b[4:"));
        let bytes = delta_bytes(&Style::default(), &old);
        assert!(
            String::from_utf8_lossy(&bytes)
                .contains(&format!("\x1b[4:{}m", underline_style_param(style)))
        );
        let bytes = delta_bytes(
            &old,
            &Style {
                underline_style: UnderlineStyle::Straight,
                ..old
            },
        );
        if style != UnderlineStyle::Straight {
            assert!(contains_seq(&bytes, b"\x1b[4:1m"));
        }
    }
}

// Count occurrences of CSI cursor-move (`ESC [ ... H`) in flush output.
fn count_move_tos(s: &str) -> usize {
    let bytes = s.as_bytes();
    let mut count = 0;
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == 0x1b && bytes[i + 1] == b'[' {
            // Scan to the terminator — final byte in 0x40..=0x7e.
            let mut j = i + 2;
            while j < bytes.len() && !(0x40..=0x7e).contains(&bytes[j]) {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'H' {
                count += 1;
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
    count
}

#[test]
fn flush_coalesces_consecutive_same_style_cells_into_one_run() {
    // 10 cells, identical Style, contiguous columns -> 1 MoveTo + 1 Print.
    let area = Rect::new(0, 0, 20, 1);
    let mut current = Buffer::empty(area);
    let previous = Buffer::empty(area);
    let style = Style::new().fg(Color::Red);
    for x in 0..10u32 {
        let cell = current.get_mut(x, 0);
        cell.set_char('X');
        cell.set_style(style);
    }

    let mut out: Vec<u8> = Vec::new();
    flush_buffer_diff(
        &mut out,
        &current,
        &previous,
        ColorDepth::TrueColor,
        0,
        &mut String::new(),
    )
    .unwrap();
    let s = String::from_utf8(out).unwrap();

    // Exactly one cursor move for the whole run.
    assert_eq!(
        count_move_tos(&s),
        1,
        "expected 1 MoveTo for a coalesced run, got {} in {:?}",
        count_move_tos(&s),
        s
    );
    // The 10 glyphs are emitted contiguously as a single run.
    assert!(
        s.contains("XXXXXXXXXX"),
        "expected contiguous run 'XXXXXXXXXX' in {:?}",
        s
    );
}

#[test]
fn flush_normalizes_directly_mutated_cell_symbols() {
    let area = Rect::new(0, 0, 2, 1);
    let mut current = Buffer::empty(area);
    let previous = Buffer::empty(area);
    current.get_mut(0, 0).symbol = compact_str::CompactString::new("\x1b]52;c;payload");

    let mut out = Vec::new();
    flush_buffer_diff(
        &mut out,
        &current,
        &previous,
        ColorDepth::TrueColor,
        0,
        &mut String::new(),
    )
    .unwrap();

    let output = String::from_utf8(out).unwrap();
    assert!(output.contains('\u{FFFD}'));
    assert!(!output.contains("]52;c;payload"));
}

#[test]
fn flush_breaks_run_on_style_change() {
    // 5 red cells + 5 blue cells in the same row -> 2 MoveTo calls not 10.
    let area = Rect::new(0, 0, 20, 1);
    let mut current = Buffer::empty(area);
    let previous = Buffer::empty(area);
    let red = Style::new().fg(Color::Red);
    let blue = Style::new().fg(Color::Blue);
    for x in 0..5u32 {
        let cell = current.get_mut(x, 0);
        cell.set_char('R');
        cell.set_style(red);
    }
    for x in 5..10u32 {
        let cell = current.get_mut(x, 0);
        cell.set_char('B');
        cell.set_style(blue);
    }

    let mut out: Vec<u8> = Vec::new();
    flush_buffer_diff(
        &mut out,
        &current,
        &previous,
        ColorDepth::TrueColor,
        0,
        &mut String::new(),
    )
    .unwrap();
    let s = String::from_utf8(out).unwrap();

    // First run needs a MoveTo; the second run starts exactly where the
    // cursor already is, so `last_cursor` suppresses a redundant MoveTo.
    // Either way, we should see at most 2 MoveTos and far fewer than 10.
    let moves = count_move_tos(&s);
    assert!(
        moves <= 2,
        "expected at most 2 MoveTos across a style boundary, got {} in {:?}",
        moves,
        s
    );
    assert!(s.contains("RRRRR"), "missing 'RRRRR' run in {:?}", s);
    assert!(s.contains("BBBBB"), "missing 'BBBBB' run in {:?}", s);
}

#[test]
fn flush_breaks_run_on_column_gap() {
    // Cells at x=0..3 and x=6..9; gap at x=3,4,5 must split runs.
    let area = Rect::new(0, 0, 20, 1);
    let mut current = Buffer::empty(area);
    let previous = Buffer::empty(area);
    let style = Style::new().fg(Color::Green);
    for x in 0..3u32 {
        current.get_mut(x, 0).set_char('A').set_style(style);
    }
    for x in 6..9u32 {
        current.get_mut(x, 0).set_char('B').set_style(style);
    }

    let mut out: Vec<u8> = Vec::new();
    flush_buffer_diff(
        &mut out,
        &current,
        &previous,
        ColorDepth::TrueColor,
        0,
        &mut String::new(),
    )
    .unwrap();
    let s = String::from_utf8(out).unwrap();

    // Two separate runs means two MoveTo commands.
    assert_eq!(
        count_move_tos(&s),
        2,
        "expected 2 MoveTos across a column gap, got {} in {:?}",
        count_move_tos(&s),
        s
    );
    assert!(s.contains("AAA"), "missing 'AAA' run in {:?}", s);
    assert!(s.contains("BBB"), "missing 'BBB' run in {:?}", s);
}

/// Verifies that `flush_buffer_diff` produces identical ANSI output whether the
/// destination is a plain `Vec<u8>` or a `BufWriter<Vec<u8>>`. This ensures the
/// BufWriter wrapper introduced for stdout does not alter the byte stream.
#[test]
fn bufwriter_output_identical_to_direct_write() {
    let area = Rect::new(0, 0, 5, 1);
    let mut current = Buffer::empty(area);
    let previous = Buffer::empty(area);
    let style = Style::new().fg(Color::Rgb(255, 128, 0));
    for x in 0..5u32 {
        current.get_mut(x, 0).set_char('X').set_style(style);
    }

    let mut direct: Vec<u8> = Vec::new();
    flush_buffer_diff(
        &mut direct,
        &current,
        &previous,
        ColorDepth::TrueColor,
        0,
        &mut String::new(),
    )
    .unwrap();

    let mut buffered: BufWriter<Vec<u8>> = BufWriter::with_capacity(65536, Vec::new());
    flush_buffer_diff(
        &mut buffered,
        &current,
        &previous,
        ColorDepth::TrueColor,
        0,
        &mut String::new(),
    )
    .unwrap();
    buffered.flush().unwrap();
    let via_buf = buffered.into_inner().unwrap();

    assert_eq!(
        direct, via_buf,
        "BufWriter output must be byte-for-byte identical to direct write"
    );
}

/// Verifies that a `BufWriter<Vec<u8>>` sink accumulates all writes and only
/// issues a single underlying `write` call to the inner sink when flushed.
/// This is a proxy for the syscall-reduction guarantee on the real stdout.
#[test]
fn bufwriter_coalesces_writes_into_single_flush() {
    #[derive(Debug)]
    struct CountingWriter {
        buf: Vec<u8>,
        write_call_count: usize,
    }
    impl Write for CountingWriter {
        fn write(&mut self, data: &[u8]) -> io::Result<usize> {
            self.write_call_count += 1;
            self.buf.extend_from_slice(data);
            Ok(data.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    let area = Rect::new(0, 0, 10, 1);
    let mut current = Buffer::empty(area);
    let previous = Buffer::empty(area);
    // Alternate styles on every cell to maximise queue! calls inside flush_buffer_diff.
    for x in 0..10u32 {
        let color = if x % 2 == 0 {
            Color::Rgb(255, 0, 0)
        } else {
            Color::Rgb(0, 255, 0)
        };
        current
            .get_mut(x, 0)
            .set_char('Z')
            .set_style(Style::new().fg(color));
    }

    let sink = CountingWriter {
        buf: Vec::new(),
        write_call_count: 0,
    };
    let mut bw = BufWriter::with_capacity(65536, sink);
    flush_buffer_diff(
        &mut bw,
        &current,
        &previous,
        ColorDepth::TrueColor,
        0,
        &mut String::new(),
    )
    .unwrap();
    bw.flush().unwrap();
    let inner = bw.into_inner().unwrap();

    // BufWriter should have batched everything into 1 write call to the sink.
    assert_eq!(
        inner.write_call_count, 1,
        "expected 1 write syscall to sink, got {}",
        inner.write_call_count
    );
}

/// Issue #171/#486 regression: identical buffers must produce no flush
/// output. Validates that the per-row skip path is correctness-preserving —
/// a skipped row emits zero bytes, exactly like the per-cell path would for
/// an unchanged row.
#[test]
fn flush_skips_unchanged_rows() {
    let area = Rect::new(0, 0, 20, 4);
    let mut current = Buffer::empty(area);
    let mut previous = Buffer::empty(area);
    // Populate both buffers with identical content.
    for y in 0..4u32 {
        current.set_string(0, y, "identical-row-content", Style::new());
        previous.set_string(0, y, "identical-row-content", Style::new());
    }

    let mut out: Vec<u8> = Vec::new();
    flush_buffer_diff(
        &mut out,
        &current,
        &previous,
        ColorDepth::TrueColor,
        0,
        &mut String::new(),
    )
    .unwrap();
    assert!(
        out.is_empty(),
        "identical buffers must emit zero flush bytes; got {} bytes: {:?}",
        out.len(),
        out
    );
}

/// Issue #171/#486 regression: when only some rows match, only those rows
/// are skipped. The differing row must still drive its full per-cell
/// flush path so the terminal sees the correct glyphs.
#[test]
fn flush_skips_only_matching_rows_in_mixed_diff() {
    let area = Rect::new(0, 0, 6, 3);
    let mut current = Buffer::empty(area);
    let mut previous = Buffer::empty(area);
    current.set_string(0, 0, "abcdef", Style::new());
    previous.set_string(0, 0, "abcdef", Style::new());
    current.set_string(0, 1, "xxxxxx", Style::new());
    previous.set_string(0, 1, "yyyyyy", Style::new());
    current.set_string(0, 2, "zzzzzz", Style::new());
    previous.set_string(0, 2, "zzzzzz", Style::new());

    let mut out: Vec<u8> = Vec::new();
    flush_buffer_diff(
        &mut out,
        &current,
        &previous,
        ColorDepth::TrueColor,
        0,
        &mut String::new(),
    )
    .unwrap();
    let s = String::from_utf8_lossy(&out);
    // The mismatched row's new content must appear; matching rows'
    // glyphs must not (they share content with `previous`).
    assert!(s.contains("xxxxxx"), "differing row must flush: {s:?}");
    assert!(
        !s.contains("abcdef"),
        "matching row 0 must not flush: {s:?}"
    );
    assert!(
        !s.contains("zzzzzz"),
        "matching row 2 must not flush: {s:?}"
    );
}

fn delta_bytes(old: &Style, new: &Style) -> Vec<u8> {
    let mut out = Vec::new();
    apply_style_delta(&mut out, old, new, ColorDepth::TrueColor).unwrap();
    out
}

#[test]
fn apply_style_delta_emits_blink_set_and_reset() {
    let on = delta_bytes(&Style::new(), &Style::new().blink());
    // SGR 5 = SlowBlink.
    assert!(contains_seq(&on, b"\x1b[5m"), "blink set: {on:?}");
    let off = delta_bytes(&Style::new().blink(), &Style::new());
    // SGR 25 = NoBlink.
    assert!(contains_seq(&off, b"\x1b[25m"), "blink reset: {off:?}");
}

#[test]
fn apply_style_delta_emits_overline_set_and_reset() {
    let on = delta_bytes(&Style::new(), &Style::new().overline());
    // SGR 53 = OverLined.
    assert!(contains_seq(&on, b"\x1b[53m"), "overline set: {on:?}");
    let off = delta_bytes(&Style::new().overline(), &Style::new());
    // SGR 55 = NotOverLined.
    assert!(contains_seq(&off, b"\x1b[55m"), "overline reset: {off:?}");
}

#[test]
fn apply_style_delta_emits_curly_underline_subparameter() {
    let out = delta_bytes(
        &Style::new(),
        &Style::new().underline_style(UnderlineStyle::Curly),
    );
    assert!(contains_seq(&out, b"\x1b[4:3m"), "curly underline: {out:?}");
}

#[test]
fn apply_style_delta_emits_underline_color_and_reset() {
    let set = delta_bytes(
        &Style::new(),
        &Style::new().underline_color(Color::Rgb(255, 0, 0)),
    );
    assert!(
        contains_seq(&set, b"\x1b[58:2::255:0:0m"),
        "underline color set: {set:?}"
    );
    let clear = delta_bytes(
        &Style::new().underline_color(Color::Rgb(255, 0, 0)),
        &Style::new(),
    );
    assert!(
        contains_seq(&clear, b"\x1b[59m"),
        "underline color reset: {clear:?}"
    );
}

#[test]
fn apply_style_delta_underline_color_indexed_uses_sgr_58_5() {
    let out = delta_bytes(
        &Style::new(),
        &Style::new().underline_color(Color::Indexed(42)),
    );
    assert!(
        contains_seq(&out, b"\x1b[58:5:42m"),
        "indexed underline: {out:?}"
    );
}

#[test]
fn apply_style_full_emits_blink_overline_and_underline() {
    let mut out = Vec::new();
    let style = Style::new()
        .blink()
        .overline()
        .underline_style(UnderlineStyle::Dotted)
        .underline_color(Color::Rgb(0, 0, 255));
    apply_style(&mut out, &style, ColorDepth::TrueColor).unwrap();
    assert!(contains_seq(&out, b"\x1b[5m"), "blink: {out:?}");
    assert!(contains_seq(&out, b"\x1b[53m"), "overline: {out:?}");
    assert!(
        contains_seq(&out, b"\x1b[4:4m"),
        "dotted underline: {out:?}"
    );
    assert!(
        contains_seq(&out, b"\x1b[58:2::0:0:255m"),
        "underline color: {out:?}"
    );
}

/// Issue #274: a captured-sink `Terminal` routes a styled cell through the
/// real flush pipeline into the in-process byte sink, and dropping it does
/// not emit teardown escapes (no raw mode was entered).
#[test]
fn with_sink_captures_flush_bytes_and_drops_clean() {
    let mut term = Terminal::with_sink(10, 1, ColorDepth::TrueColor);
    term.buffer_mut()
        .set_string(0, 0, "Z", Style::new().fg(Color::Rgb(200, 50, 50)));
    term.flush().unwrap();
    let bytes = term.take_sink_bytes();
    let s = String::from_utf8_lossy(&bytes);
    // Real terminal control bytes + the printed glyph went to the sink.
    assert!(s.contains("\u{1b}["), "missing CSI: {s:?}");
    assert!(s.contains('Z'), "missing glyph: {s:?}");
    // A second take after no flush yields nothing (capture was drained).
    assert!(term.take_sink_bytes().is_empty());
    // Dropping the harness terminal must not panic or emit teardown.
    drop(term);
}

/// Issue #269: hoisting `run_buf` to a reused, caller-owned buffer must not
/// change the emitted bytes. Re-running the diff twice through the *same*
/// `run_buf` (which `clear()`s but keeps capacity at the top of each call)
/// produces the same output as a single fresh-buffer run.
#[test]
fn reused_run_buf_byte_identical_across_frames() {
    let area = Rect::new(0, 0, 12, 2);
    // `Buffer` is not `Clone`, so rebuild the frame pair on demand.
    let make_frame = || {
        let mut current = Buffer::empty(area);
        let previous = Buffer::empty(area);
        current.set_string(0, 0, "hello world", Style::new().fg(Color::Rgb(1, 2, 3)));
        current.set_string(0, 1, "second line", Style::new().fg(Color::Rgb(4, 5, 6)));
        (current, previous)
    };

    // Baseline: a fresh run_buf per call.
    let mut baseline: Vec<u8> = Vec::new();
    {
        let (mut a, mut b) = make_frame();
        __bench_flush_buffer_diff_mut_with_buf(
            &mut baseline,
            &mut a,
            &mut b,
            ColorDepth::TrueColor,
            &mut String::with_capacity(RUN_BUF_INITIAL_CAPACITY),
        )
        .unwrap();
    }

    // Reuse: run a throwaway frame first, then the real frame through the
    // SAME run_buf (now carrying leftover capacity, freshly cleared).
    let mut shared = String::with_capacity(RUN_BUF_INITIAL_CAPACITY);
    {
        let mut warm: Vec<u8> = Vec::new();
        let (mut a, mut b) = make_frame();
        __bench_flush_buffer_diff_mut_with_buf(
            &mut warm,
            &mut a,
            &mut b,
            ColorDepth::TrueColor,
            &mut shared,
        )
        .unwrap();
    }
    let cap_after_warm = shared.capacity();

    let mut reused: Vec<u8> = Vec::new();
    let (mut current, mut previous) = make_frame();
    __bench_flush_buffer_diff_mut_with_buf(
        &mut reused,
        &mut current,
        &mut previous,
        ColorDepth::TrueColor,
        &mut shared,
    )
    .unwrap();

    assert_eq!(
        baseline, reused,
        "reused run_buf must emit byte-identical output"
    );
    // The reuse path keeps capacity across frames (never re-grows below the
    // initial reservation) — the whole point of the hoist.
    assert!(
        shared.capacity() >= cap_after_warm,
        "run_buf capacity must persist across frames"
    );
}

/// Issue #269: the OSC 8 hyperlink open, rewritten from `format!` to three
/// borrowed `Print`s, must still emit the exact `\x1b]8;;<url>\x07 ...
/// \x1b]8;;\x07` sequence.
#[test]
fn osc8_hyperlink_emitted_verbatim_after_write_rewrite() {
    let area = Rect::new(0, 0, 8, 1);
    let mut current = Buffer::empty(area);
    let previous = Buffer::empty(area);
    let url = "https://example.com/x";
    // `set_string_linked` sanitizes + attaches the hyperlink to each cell.
    current.set_string_linked(0, 0, "link", Style::new(), url);

    let mut out: Vec<u8> = Vec::new();
    flush_buffer_diff(
        &mut out,
        &current,
        &previous,
        ColorDepth::TrueColor,
        0,
        &mut String::new(),
    )
    .unwrap();

    let open = format!("\x1b]8;;{url}\x07");
    assert!(
        contains_seq(&out, open.as_bytes()),
        "OSC 8 open must appear verbatim: {:?}",
        String::from_utf8_lossy(&out)
    );
    assert!(
        contains_seq(&out, b"\x1b]8;;\x07"),
        "OSC 8 close must appear: {:?}",
        String::from_utf8_lossy(&out)
    );
}

/// Minimal terminal emulator for flush tests: applies cursor moves
/// (`CSI row;col H`) and printable text to a symbol grid, ignoring SGR, OSC
/// and mode sequences. A grapheme of display width `w` occupies `w` cells;
/// the trailing cells become empty continuation markers.
pub(super) fn apply_ansi_to_screen(screen: &mut [Vec<String>], bytes: &[u8]) {
    apply_ansi_to_screen_with(screen, bytes, &|grapheme| UnicodeWidthStr::width(grapheme));
}

/// [`apply_ansi_to_screen`] for a terminal that advances `width_of(cluster)`
/// columns per grapheme (for example one column for VS16 emoji).
pub(super) fn apply_ansi_to_screen_with(
    screen: &mut [Vec<String>],
    bytes: &[u8],
    width_of: &dyn Fn(&str) -> usize,
) {
    use unicode_segmentation::UnicodeSegmentation;

    let put_text = |screen: &mut [Vec<String>], text: &str, row: usize, col: &mut usize| {
        for grapheme in text.graphemes(true) {
            let width = width_of(grapheme).max(1);
            if let Some(line) = screen.get_mut(row) {
                if let Some(cell) = line.get_mut(*col) {
                    *cell = grapheme.to_string();
                }
                for offset in 1..width {
                    if let Some(cell) = line.get_mut(*col + offset) {
                        cell.clear();
                    }
                }
            }
            *col += width;
        }
    };

    let mut row = 0usize;
    let mut col = 0usize;
    let mut i = 0usize;
    let mut text_start = None;
    while i < bytes.len() {
        if bytes[i] != 0x1b {
            text_start.get_or_insert(i);
            i += 1;
            continue;
        }
        if let Some(start) = text_start.take() {
            let text = std::str::from_utf8(&bytes[start..i]).expect("utf-8 text run");
            put_text(screen, text, row, &mut col);
        }
        match bytes.get(i + 1) {
            Some(b'[') => {
                let mut j = i + 2;
                while j < bytes.len() && !(0x40..=0x7e).contains(&bytes[j]) {
                    j += 1;
                }
                if bytes.get(j) == Some(&b'H') {
                    let params = std::str::from_utf8(&bytes[i + 2..j]).expect("ascii params");
                    let mut parts = params.split(';');
                    let r: usize = parts.next().and_then(|p| p.parse().ok()).unwrap_or(1);
                    let c: usize = parts.next().and_then(|p| p.parse().ok()).unwrap_or(1);
                    row = r.saturating_sub(1);
                    col = c.saturating_sub(1);
                }
                i = j + 1;
            }
            Some(b']') => {
                let mut j = i + 2;
                while j < bytes.len() && bytes[j] != 0x07 {
                    j += 1;
                }
                i = j + 1;
            }
            _ => i += 2,
        }
    }
    if let Some(start) = text_start {
        let text = std::str::from_utf8(&bytes[start..]).expect("utf-8 text run");
        put_text(screen, text, row, &mut col);
    }
}

/// Visible symbols of `buffer` in the same shape as [`apply_ansi_to_screen`].
pub(super) fn buffer_symbols(buffer: &Buffer) -> Vec<Vec<String>> {
    (buffer.area.y..buffer.area.bottom())
        .map(|y| {
            (buffer.area.x..buffer.area.right())
                .map(|x| buffer.get(x, y).symbol.to_string())
                .collect()
        })
        .collect()
}

/// Issue #484: `"\r\n"` is one extended grapheme. It used to normalize to
/// two `U+FFFD` (width 2, reserving a continuation cell) and then be stored
/// as one, so the flush skipped the orphan continuation and left the previous
/// frame's glyph on screen.
#[test]
fn crlf_write_leaves_no_stale_glyph_after_flush() {
    let area = Rect::new(0, 0, 6, 1);
    let mut screen = vec![vec![" ".to_string(); 6]];
    let mut previous = Buffer::empty(area);
    previous.set_string(0, 0, "aXYb", Style::new());
    let mut out = Vec::new();
    let mut blank = Buffer::empty(area);
    __bench_flush_buffer_diff_mut_with_buf(
        &mut out,
        &mut previous,
        &mut blank,
        ColorDepth::TrueColor,
        &mut String::new(),
    )
    .unwrap();
    apply_ansi_to_screen(&mut screen, &out);
    assert_eq!(screen, buffer_symbols(&previous));

    let mut current = Buffer::empty(area);
    current.set_string(0, 0, "a\r\nb", Style::new());
    assert_eq!(current.get(1, 0).symbol, "\u{FFFD}");
    assert!(!current.get(2, 0).is_continuation());
    assert_eq!(current.get(2, 0).symbol, "b");

    out.clear();
    __bench_flush_buffer_diff_mut_with_buf(
        &mut out,
        &mut current,
        &mut previous,
        ColorDepth::TrueColor,
        &mut String::new(),
    )
    .unwrap();
    apply_ansi_to_screen(&mut screen, &out);
    assert_eq!(
        screen,
        buffer_symbols(&current),
        "stale glyph left on screen"
    );
}

/// Issue #441: a VS16 emoji is drawn two columns wide by conformant
/// terminals and one column by others. The flush must not pad it with a
/// literal space (which shifts the rest of the row on two-column terminals)
/// and must position the next cell explicitly.
#[test]
fn vs16_cluster_is_repositioned_not_padded() {
    let area = Rect::new(0, 0, 8, 1);
    let mut current = Buffer::empty(area);
    current.set_string(0, 0, "A\u{2764}\u{FE0F}BC", Style::new());
    let mut out = Vec::new();
    flush_buffer_diff(
        &mut out,
        &current,
        &Buffer::empty(area),
        ColorDepth::TrueColor,
        0,
        &mut String::new(),
    )
    .unwrap();
    let text = String::from_utf8(out).unwrap();
    assert_eq!(
        text,
        "\x1b[1;1HA\x1b[1;3H \x1b[1;2H\u{2764}\u{FE0F}\x1b[1;4HBC\x1b[0m"
    );
    assert!(
        !text.contains("\u{FE0F} "),
        "no padding space after the cluster"
    );
}

/// Issue #441 on tmux < 3.5: tmux widens `U+2764 U+FE0F` but marks the
/// padding cell one column too far right, so the pane keeps a stray cell and
/// redraws shift the row. The fallback drops U+FE0F (text presentation),
/// keeps the blank fill for the buffer's second column and still positions
/// the next cell explicitly.
#[test]
fn tmux_vs16_fallback_emits_text_presentation() {
    let area = Rect::new(0, 0, 8, 1);
    let mut current = Buffer::empty(area);
    current.set_string(0, 0, "A\u{2764}\u{FE0F}BC", Style::new());
    let policy = WidthPolicy {
        conservative: true,
        ..WidthPolicy::default()
    }
    .with_tmux_vs16_fallback(true);
    assert!(policy.text_presentation);
    let mut out = Vec::new();
    flush_buffer_diff_rows(
        &mut out,
        &current,
        &Buffer::empty(area),
        ColorDepth::TrueColor,
        0,
        &mut String::new(),
        &[],
        policy,
    )
    .unwrap();
    assert_eq!(
        String::from_utf8(out).unwrap(),
        "\x1b[1;1HA\x1b[1;3H \x1b[1;2H\u{2764}\x1b[1;4HBC\x1b[0m"
    );

    // Explicitly non-conservative width handling trusts the terminal.
    let trusting = WidthPolicy::default().with_tmux_vs16_fallback(true);
    assert!(!trusting.text_presentation);
}

/// Issue #441: after frames that change only the trailing cell, the screen
/// equals the buffer on both one- and two-column VS16 terminals.
#[test]
fn vs16_row_stays_in_sync_on_one_and_two_column_terminals() {
    for vs16_columns in [1usize, 2] {
        let width_of = |grapheme: &str| {
            if grapheme.contains('\u{FE0F}') {
                vs16_columns
            } else {
                UnicodeWidthStr::width(grapheme)
            }
        };
        let area = Rect::new(0, 0, 8, 1);
        let mut screen = vec![vec![" ".to_string(); 8]];
        let mut previous = Buffer::empty(area);
        for tail in ["x", "y", "x"] {
            let mut current = Buffer::empty(area);
            current.set_string(0, 0, &format!("|A\u{2764}\u{FE0F}B{tail}|"), Style::new());
            let mut out = Vec::new();
            flush_buffer_diff(
                &mut out,
                &current,
                &previous,
                ColorDepth::TrueColor,
                0,
                &mut String::new(),
            )
            .unwrap();
            apply_ansi_to_screen_with(&mut screen, &out, &width_of);
            let mut expected = buffer_symbols(&current);
            if vs16_columns == 1 {
                // The terminal shows the blank fill where the buffer keeps
                // the cluster's continuation cell.
                expected[0][3] = " ".to_string();
            }
            assert_eq!(
                screen, expected,
                "{vs16_columns}-column terminal, tail {tail}"
            );
            previous = current;
        }
    }
}

/// Issue #441: ASCII and CJK rows keep their pre-#441 bytes (only the #489
/// single reset differs from those goldens).
#[test]
fn ascii_and_cjk_rows_keep_golden_bytes() {
    let area = Rect::new(0, 0, 10, 2);
    let mut current = Buffer::empty(area);
    current.set_string(0, 0, "AB\u{4E16}C", Style::new());
    current.set_string(2, 1, "hi \u{D55C}\u{AE00}", Style::new().fg(Color::Red));
    let mut out = Vec::new();
    flush_buffer_diff(
        &mut out,
        &current,
        &Buffer::empty(area),
        ColorDepth::TrueColor,
        0,
        &mut String::new(),
    )
    .unwrap();
    assert_eq!(
        String::from_utf8(out).unwrap(),
        "\x1b[1;1HAB\u{4E16}C\x1b[2;3H\x1b[0m\x1b[31mhi \u{D55C}\u{AE00}\x1b[0m"
    );
}

/// #449 conservative width mode repositions after every width-uncertain
/// cluster, including plain non-BMP emoji.
#[test]
fn conservative_width_policy_repositions_after_emoji() {
    let area = Rect::new(0, 0, 8, 1);
    let mut current = Buffer::empty(area);
    current.set_string(0, 0, "\u{1F600}ab", Style::new());
    let policy = WidthPolicy {
        conservative: true,
        ..WidthPolicy::default()
    };
    let mut out = Vec::new();
    flush_buffer_diff_rows(
        &mut out,
        &current,
        &Buffer::empty(area),
        ColorDepth::TrueColor,
        0,
        &mut String::new(),
        &[],
        policy,
    )
    .unwrap();
    assert_eq!(
        String::from_utf8(out).unwrap(),
        "\x1b[1;1H\u{1F600}\x1b[1;3Hab\x1b[0m"
    );
}
