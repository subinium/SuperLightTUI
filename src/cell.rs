//! Single terminal cell — the smallest unit of the render buffer.

use compact_str::CompactString;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthChar;

use crate::style::Style;

/// Maximum UTF-8 bytes retained for one terminal grapheme.
///
/// This bounds output work for pathological combining-mark sequences while
/// leaving enough room for common ZWJ emoji clusters.
pub(crate) const MAX_CELL_SYMBOL_BYTES: usize = 32;

/// Replace terminal control code points with the visible replacement glyph.
#[inline]
pub(crate) fn sanitize_cell_char(ch: char) -> char {
    let value = ch as u32;
    if value < 0x20 || value == 0x7f || (0x80..=0x9f).contains(&value) {
        '\u{FFFD}'
    } else {
        ch
    }
}

/// Return whether `ch` is a terminal control code point (C0, DEL or C1).
#[inline]
pub(crate) fn is_control_char(ch: char) -> bool {
    sanitize_cell_char(ch) != ch
}

/// Return whether `byte` is printable ASCII (`0x20..=0x7E`).
#[inline]
pub(crate) const fn is_printable_ascii_byte(byte: u8) -> bool {
    byte >= 0x20 && byte < 0x7f
}

/// Return whether every byte of `s` is printable ASCII (`0x20..=0x7E`).
///
/// Printable ASCII is the fast path of the cell-write and width code: each
/// byte is its own extended grapheme cluster (no ASCII code point is Extend,
/// ZWJ, SpacingMark, Prepend or Control-like under UAX #29), has display
/// width 1, is a strong-LTR or neutral bidi class, and needs no
/// sanitization. Checks eight bytes per step (SWAR) without `unsafe`.
#[inline]
pub(crate) fn is_printable_ascii(s: &str) -> bool {
    const ONES: u64 = u64::from_ne_bytes([0x01; 8]);
    const HIGHS: u64 = u64::from_ne_bytes([0x80; 8]);

    let (words, remainder) = s.as_bytes().as_chunks::<8>();
    for word in words {
        let x = u64::from_ne_bytes(*word);
        // Any byte < 0x20: the classic "has less than" test. Any byte > 0x7E
        // (DEL or non-ASCII): adding 1 sets its high bit, or it already had
        // one. Carries can only add false positives next to a true positive,
        // which is fine for an "any byte" test.
        let below = x.wrapping_sub(ONES.wrapping_mul(0x20)) & !x & HIGHS;
        let above = (x.wrapping_add(ONES) | x) & HIGHS;
        if below | above != 0 {
            return false;
        }
    }
    remainder.iter().all(|&byte| is_printable_ascii_byte(byte))
}

#[cfg(test)]
thread_local! {
    /// Test-only count of [`display_width`] calls on this thread, so layout
    /// tests can assert each text node is measured once per frame (#494).
    pub(crate) static DISPLAY_WIDTH_CALLS: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
}

/// Whether `ch` always forms its own extended grapheme cluster next to
/// printable ASCII or another such scalar: its Grapheme_Cluster_Break is
/// neither Prepend, Extend, ZWJ, SpacingMark, Regional_Indicator nor a
/// joining Hangul jamo, and it is not Extended_Pictographic (UAX #29).
///
/// Lets mixed-text writes skip the segmenter for the glyphs that dominate
/// terminal UIs. Conservative: `false` only costs the general path. An
/// exhaustive test checks the segmentation behaviour of every scalar for
/// which this returns `true`.
#[inline]
pub(crate) fn standalone_scalar(ch: char) -> bool {
    matches!(
        ch as u32,
        // Latin-1 punctuation and letters, Latin Extended-A/B (not ©/®,
        // which are Extended_Pictographic, nor the soft hyphen, a Control).
        0x00A0..=0x00A8
            | 0x00AA..=0x00AC
            | 0x00AF..=0x024F
            // Greek and Cyrillic letters (not the Cyrillic combining marks).
            | 0x0370..=0x0377
            | 0x037A..=0x037F
            | 0x0384..=0x0482
            | 0x048A..=0x052F
            // Dashes, quotes, bullets and ellipsis.
            | 0x2010..=0x2027
            | 0x2030..=0x203B
            // Arrows that are not Extended_Pictographic.
            | 0x2190..=0x2193
            // Box drawing, block elements.
            | 0x2500..=0x259F
            // Braille patterns (charts, sparklines).
            | 0x2800..=0x28FF
            // CJK unified ideographs.
            | 0x4E00..=0x9FFF
            // Fullwidth ASCII variants and punctuation.
            | 0xFF01..=0xFF60
    )
}

/// Display width of `s` in terminal columns.
///
/// Equal to [`crate::width::str_width`] (which honours the frame's
/// ambiguous-width setting, #449) for every input, with an O(len) byte-check
/// fast path for printable ASCII, which is never ambiguous.
#[inline]
pub(crate) fn display_width(s: &str) -> usize {
    #[cfg(test)]
    DISPLAY_WIDTH_CALLS.with(|calls| calls.set(calls.get() + 1));
    if is_printable_ascii(s) {
        s.len()
    } else {
        mixed_display_width(s)
    }
}

/// [`display_width`] of text that is not all printable ASCII.
///
/// Long, mostly-ASCII strings (table rows, list rows) are measured by
/// counting their interior ASCII bytes and running the width tables only
/// over the stretches around non-ASCII scalars. The split points lie between
/// two printable-ASCII bytes, where no width rule of `unicode-width` (CR LF,
/// variation selectors, emoji ZWJ, Lisu tones, lam-alef) can span: a
/// proptest checks the result against [`crate::width::str_width`].
fn mixed_display_width(s: &str) -> usize {
    /// Strings this short gain nothing from splitting.
    const SPLIT_MIN_BYTES: usize = 16;
    if s.len() < SPLIT_MIN_BYTES {
        return crate::width::str_width(s);
    }
    let bytes = s.as_bytes();
    let printable = |i: usize| is_printable_ascii_byte(bytes[i]);
    // A byte whose neighbours are printable ASCII too: splitting on both
    // sides of it never separates a width sequence.
    let interior = |i: usize| {
        printable(i) && (i == 0 || printable(i - 1)) && (i + 1 == bytes.len() || printable(i + 1))
    };
    let mut width = 0usize;
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        if interior(i) {
            i += 1;
            while i < bytes.len() && interior(i) {
                i += 1;
            }
            width = width.saturating_add(i - start);
        } else {
            i += 1;
            while i < bytes.len() && !interior(i) {
                i += 1;
            }
            let stretch = &s[start..i];
            // Printable ASCII and standalone scalars take part in no width
            // sequence, so such a stretch is the sum of its scalars.
            let simple = stretch
                .chars()
                .all(|ch| ch.is_ascii_graphic() || ch == ' ' || standalone_scalar(ch));
            let stretch_width = if simple {
                stretch.chars().map(char_display_width).sum()
            } else {
                crate::width::str_width(stretch)
            };
            width = width.saturating_add(stretch_width);
        }
    }
    width
}

/// Display width of a single-scalar symbol, as [`crate::width::str_width`]
/// would measure the one-char string.
///
/// `UnicodeWidthChar` reports `None` for C0/DEL/C1 controls, which the
/// string measure counts as width 1; every other scalar agrees in both the
/// default and the ambiguous-wide interpretation (verified exhaustively in
/// the tests).
#[inline]
pub(crate) fn char_display_width(ch: char) -> usize {
    if ch.is_ascii() {
        return 1;
    }
    if crate::width::ambiguous_wide() {
        UnicodeWidthChar::width_cjk(ch).unwrap_or(1)
    } else {
        UnicodeWidthChar::width(ch).unwrap_or(1)
    }
}

/// Normalize an arbitrary string into one bounded, terminal-safe grapheme.
///
/// An empty result is reserved for continuation cells. Additional graphemes
/// are discarded because a `Cell` represents exactly one display atom.
///
/// The result is idempotent: `normalize_cell_symbol(&normalize_cell_symbol(s))
/// == normalize_cell_symbol(s)`. A grapheme that starts with a control code
/// point (a lone C0/DEL/C1 control, or the two-scalar `"\r\n"` cluster)
/// becomes exactly one `U+FFFD`, so the stored symbol always re-segments to
/// itself and its display width equals the cells reserved for it (#484).
pub(crate) fn normalize_cell_symbol(symbol: &str) -> CompactString {
    let Some(grapheme) = symbol.graphemes(true).next() else {
        return CompactString::const_new("");
    };
    normalize_grapheme(grapheme)
}

/// Normalize one already-segmented extended grapheme cluster.
///
/// Callers must pass a single grapheme (for example one item of
/// `str::graphemes(true)`); see [`normalize_cell_symbol`] for the contract.
pub(crate) fn normalize_grapheme(grapheme: &str) -> CompactString {
    let mut chars = grapheme.chars();
    let Some(first) = chars.next() else {
        return CompactString::const_new("");
    };
    // UAX #29 GB4/GB5 break before and after every control except inside
    // CR LF, so a control can only start a cluster. Collapse the whole
    // cluster to one replacement glyph instead of one per scalar: `"\r\n"`
    // previously became two `U+FFFD` (width 2) that re-normalized to one.
    if is_control_char(first) {
        return CompactString::const_new("\u{FFFD}");
    }
    if grapheme.len() <= MAX_CELL_SYMBOL_BYTES && !chars.any(is_control_char) {
        return CompactString::new(grapheme);
    }

    // Defensive slow path: over-long clusters are truncated at a scalar
    // boundary (a prefix of a cluster is still one cluster), and any control
    // that is not at the start is replaced.
    let mut normalized = CompactString::const_new("");
    for ch in grapheme.chars() {
        let ch = sanitize_cell_char(ch);
        if normalized.len().saturating_add(ch.len_utf8()) > MAX_CELL_SYMBOL_BYTES {
            break;
        }
        normalized.push(ch);
    }
    normalized
}

// Compile-time size assertion for `Cell`.
//
// `Cell` is composed of `symbol: CompactString` + `style: Style` +
// `hyperlink: Option<CompactString>`. Upstream changes to any of these
// (e.g., `CompactString` inline-storage tweaks, `Style` field additions,
// or hyperlink type swaps) can silently grow the struct until runtime.
// A 64-byte budget keeps each cell within one cache line.
//
// If an intentional growth pushes us past 64 B, raise this bound and
// document why — but do not silently let it drift.
const _: () = assert!(
    std::mem::size_of::<Cell>() <= 64,
    "Cell exceeds one cache line (64 B). If the size increase is intentional, update this bound and document why."
);

/// A single terminal cell containing a character and style.
///
/// Each cell holds one grapheme cluster (stored as a [`CompactString`] for
/// inline storage of short strings — no heap allocation for ≤24 bytes).
/// Wide graphemes occupy adjacent cells. The leading cell stores the
/// grapheme and every continuation cell has an empty `symbol`.
#[derive(Debug, Clone, Eq)]
pub struct Cell {
    /// The grapheme cluster displayed in this cell. Defaults to a single space.
    pub symbol: CompactString,
    /// The visual style (colors and modifiers) for this cell.
    pub style: Style,
    /// Optional OSC 8 hyperlink URL. When set, the terminal renders this cell
    /// as a clickable link.
    pub hyperlink: Option<CompactString>,
}

/// Field-wise equality, as `#[derive(PartialEq)]` would compare, with an
/// inline byte compare for short symbols: the flush and `Buffer::diff`
/// compare every candidate cell, and a `memcmp` call per one-to-four-byte
/// symbol dominated that loop (#486).
impl PartialEq for Cell {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        symbol_eq(&self.symbol, &other.symbol)
            && self.style == other.style
            && self.hyperlink == other.hyperlink
    }
}

#[inline]
fn symbol_eq(a: &CompactString, b: &CompactString) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    match a.len() {
        0 => true,
        1 => a[0] == b[0],
        2 => a[0] == b[0] && a[1] == b[1],
        3 => a[0] == b[0] && a[1] == b[1] && a[2] == b[2],
        _ => a == b,
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            symbol: CompactString::const_new(" "),
            style: Style::new(),
            hyperlink: None,
        }
    }
}

impl Cell {
    /// Replace the cell's symbol with the given string slice.
    ///
    /// Only the first extended grapheme cluster is retained. C0, DEL, and C1
    /// controls are replaced with `U+FFFD`, and the optional hyperlink is
    /// cleared so direct symbol replacement cannot inherit stale link state.
    pub fn set_symbol(&mut self, s: &str) -> &mut Self {
        self.symbol = normalize_cell_symbol(s);
        self.hyperlink = None;
        self
    }

    /// Store a symbol that is already the output of [`normalize_cell_symbol`].
    ///
    /// Internal cell-write paths normalize each grapheme once and store it
    /// through here instead of [`Cell::set_symbol`], which would segment and
    /// sanitize it again. Clears the hyperlink like `set_symbol`.
    #[inline]
    pub(crate) fn set_symbol_normalized(&mut self, symbol: CompactString) -> &mut Self {
        debug_assert_eq!(
            normalize_cell_symbol(&symbol),
            symbol,
            "set_symbol_normalized requires a normalized symbol"
        );
        self.symbol = symbol;
        self.hyperlink = None;
        self
    }

    /// Replace the cell's symbol with a single character.
    pub fn set_char(&mut self, ch: char) -> &mut Self {
        self.symbol.clear();
        self.symbol.push(sanitize_cell_char(ch));
        self.hyperlink = None;
        self
    }

    /// Return whether this cell continues a grapheme stored in a prior cell.
    ///
    /// Empty symbols are reserved as continuation metadata; ordinary blank
    /// cells contain a single space.
    #[inline]
    pub fn is_continuation(&self) -> bool {
        self.symbol.is_empty()
    }

    /// Mark this cell as a continuation of a preceding wide grapheme.
    pub(crate) fn set_continuation(&mut self, style: Style) -> &mut Self {
        self.symbol.clear();
        self.style = style;
        self.hyperlink = None;
        self
    }

    /// Return a defensively normalized symbol for terminal output.
    ///
    /// This is required at the flush boundary because `symbol` remains public
    /// for compatibility and callers can mutate it without using the setters.
    pub(crate) fn normalized_symbol(&self) -> CompactString {
        normalize_cell_symbol(&self.symbol)
    }

    /// The symbol to write to the terminal, borrowed when it is already a
    /// normalization fixed point (#486).
    ///
    /// One printable-ASCII byte and one non-control scalar (the forms every
    /// setter stores for ASCII, CJK, box drawing and most symbols) are
    /// returned as-is; anything else is defensively re-normalized because
    /// `symbol` is a public field. Equal to [`Cell::normalized_symbol`].
    #[inline]
    #[cfg_attr(not(feature = "crossterm"), allow(dead_code))]
    pub(crate) fn emit_symbol(&self) -> std::borrow::Cow<'_, str> {
        let symbol = self.symbol.as_str();
        let bytes = symbol.as_bytes();
        if bytes.len() == 1 {
            if is_printable_ascii_byte(bytes[0]) {
                return std::borrow::Cow::Borrowed(symbol);
            }
        } else {
            let mut chars = symbol.chars();
            if let (Some(ch), None) = (chars.next(), chars.next())
                && !is_control_char(ch)
            {
                return std::borrow::Cow::Borrowed(symbol);
            }
        }
        std::borrow::Cow::Owned(normalize_cell_symbol(symbol).into())
    }

    /// Set the cell's style.
    pub fn set_style(&mut self, style: Style) -> &mut Self {
        self.style = style;
        self
    }

    /// Reset the cell to a blank space with default style.
    #[inline]
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Proptest strategies shared by the cell, buffer and flush test suites.
#[cfg(test)]
pub(crate) mod test_strategies {
    use proptest::prelude::*;

    /// One text fragment drawn from the classes that stress grapheme,
    /// width, sanitization and bidi handling.
    pub(crate) fn fragment() -> impl Strategy<Value = String> {
        prop_oneof![
            4 => "[ -~]{1,6}",
            1 => Just("\r\n".to_string()),
            1 => prop::sample::select(vec![
                "\r", "\n", "\t", "\x1b", "\x07", "\x00", "\x7f", "\u{85}", "\u{9b}",
                "\x1b[31m", "\x1b]52;c;x\x07",
            ])
            .prop_map(str::to_string),
            1 => prop::sample::select(vec![
                "\u{301}", "\u{200D}", "\u{FE0F}", "\u{FE0E}", "\u{200B}", "\u{0903}",
            ])
            .prop_map(str::to_string),
            1 => prop::sample::select(vec![
                "世", "界", "한", "ｱ", "👩\u{200D}💻", "❤\u{FE0F}", "🇰🇷", "\u{1100}\u{1161}\u{11A8}",
                "e\u{301}", "क\u{094D}\u{0937}",
                // Standalone scalars of the segmenter-free path.
                "│", "—", "é", "Ж", "⣿",
                // Prepend (GB9b): joins the following ASCII letter.
                "\u{0600}", "\u{0D4E}", "\u{110BD}",
            ])
            .prop_map(str::to_string),
            1 => prop::sample::select(vec![
                "\u{05E9}\u{05DC}", "\u{0627}\u{0644}", "\u{202E}", "\u{200F}", "\u{2067}",
            ])
            .prop_map(str::to_string),
            1 => (1usize..24).prop_map(|n| format!("a{}", "\u{301}".repeat(n))),
            1 => any::<char>().prop_map(String::from),
        ]
    }

    /// A string assembled from [`fragment`]s.
    pub(crate) fn cell_text() -> impl Strategy<Value = String> {
        prop::collection::vec(fragment(), 0..10).prop_map(|parts| parts.concat())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use unicode_width::UnicodeWidthStr;

    #[test]
    fn standalone_scalars_never_join_their_neighbours() {
        let count = |s: &str| s.graphemes(true).count();
        for ch in (0..=u32::from(char::MAX))
            .filter_map(char::from_u32)
            .filter(|&ch| standalone_scalar(ch))
        {
            let code = ch as u32;
            // Not Extend / ZWJ / SpacingMark: breaks after an ASCII letter.
            assert_eq!(
                count(&format!("a{ch}")),
                2,
                "U+{code:04X} joins a preceding letter"
            );
            // Not Prepend: breaks before an ASCII letter.
            assert_eq!(
                count(&format!("{ch}a")),
                2,
                "U+{code:04X} joins a following letter"
            );
            // Not a regional indicator or joining Hangul jamo.
            assert_eq!(count(&format!("{ch}{ch}")), 2, "U+{code:04X} joins itself");
            // Not Extended_Pictographic: no emoji ZWJ sequence (GB11).
            assert_eq!(
                count(&format!("{ch}\u{200D}\u{1F600}")),
                2,
                "U+{code:04X} starts an emoji ZWJ sequence"
            );
            assert!(!is_control_char(ch), "U+{code:04X} is a control");
        }
        // Pairs of distinct standalone scalars also break (GB999).
        assert_eq!(count("│─▸—世"), 5);
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1024))]

        #[test]
        fn mixed_display_width_matches_str_width(
            parts in prop::collection::vec(
                prop_oneof![
                    3 => "[ -~]{1,12}",
                    1 => test_strategies::fragment(),
                ],
                0..12,
            ),
            ambiguous_wide in any::<bool>(),
        ) {
            let s = parts.concat();
            crate::width::set_ambiguous_wide(ambiguous_wide);
            let (got, want) = (display_width(&s), crate::width::str_width(&s));
            crate::width::set_ambiguous_wide(false);
            prop_assert_eq!(got, want, "{:?}", s);
        }

        #[test]
        fn normalize_cell_symbol_is_idempotent(s in test_strategies::cell_text()) {
            let once = normalize_cell_symbol(&s);
            prop_assert_eq!(normalize_cell_symbol(&once), once.clone());
            prop_assert!(once.len() <= MAX_CELL_SYMBOL_BYTES);
            prop_assert!(!once.chars().any(is_control_char));
        }

        #[test]
        fn emit_symbol_equals_normalized_symbol(s in prop_oneof![test_strategies::cell_text(), ".{0,6}"]) {
            let cell = Cell { symbol: CompactString::new(&s), ..Cell::default() };
            let (emitted, normalized) = (cell.emit_symbol().into_owned(), cell.normalized_symbol());
            prop_assert_eq!(emitted.as_str(), normalized.as_str());
        }

        #[test]
        fn cell_eq_matches_field_wise_equality(
            a in prop_oneof![Just(String::new()), test_strategies::fragment(), ".{0,12}"],
            b in prop_oneof![Just(String::new()), test_strategies::fragment(), ".{0,12}"],
            same_symbol in any::<bool>(),
            bold in any::<(bool, bool)>(),
            link in any::<(bool, bool)>(),
        ) {
            let b = if same_symbol { a.clone() } else { b };
            let make = |symbol: &str, bold: bool, link: bool| Cell {
                symbol: CompactString::new(symbol),
                style: if bold { Style::new().bold() } else { Style::new() },
                hyperlink: link.then(|| CompactString::new("https://example.com")),
            };
            let (x, y) = (make(&a, bold.0, link.0), make(&b, bold.1, link.1));
            let field_wise = x.symbol.as_str() == y.symbol.as_str()
                && x.style == y.style
                && x.hyperlink == y.hyperlink;
            prop_assert_eq!(x == y, field_wise);
        }

        #[test]
        fn normalize_cell_symbol_is_idempotent_for_arbitrary_text(s in ".{0,16}") {
            let once = normalize_cell_symbol(&s);
            prop_assert_eq!(normalize_cell_symbol(&once), once);
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(2048))]

        #[test]
        fn display_width_matches_unicode_width(s in test_strategies::cell_text()) {
            prop_assert_eq!(display_width(&s), UnicodeWidthStr::width(s.as_str()));
        }

        #[test]
        fn display_width_matches_unicode_width_for_arbitrary_text(s in ".{0,40}") {
            prop_assert_eq!(display_width(&s), UnicodeWidthStr::width(s.as_str()));
        }

        #[test]
        fn printable_ascii_check_matches_bytewise_reference(
            bytes in prop::collection::vec(0u8..=0x7f, 0..40),
            tail in ".{0,3}",
        ) {
            let mut s = String::from_utf8(bytes).expect("ASCII is UTF-8");
            s.push_str(&tail);
            let reference = s.bytes().all(|b| (0x20..=0x7e).contains(&b));
            prop_assert_eq!(is_printable_ascii(&s), reference);
        }
    }

    #[test]
    fn printable_ascii_check_rejects_every_non_printable_byte_in_every_lane() {
        for len in 1..=17usize {
            assert!(is_printable_ascii(&"~".repeat(len)));
            for position in 0..len {
                for byte in (0u8..0x20).chain([0x7f]) {
                    let mut bytes = vec![b' '; len];
                    bytes[position] = byte;
                    let s = String::from_utf8(bytes).expect("ASCII");
                    assert!(!is_printable_ascii(&s), "{len} {position} {byte:#x}");
                }
                let mut s = " ".repeat(position);
                s.push('é');
                s.push_str(&" ".repeat(len - position));
                assert!(!is_printable_ascii(&s), "non-ASCII at {position}");
            }
        }
        assert!(is_printable_ascii(""));
    }

    #[test]
    fn char_display_width_matches_string_width_for_every_scalar() {
        struct Reset;
        impl Drop for Reset {
            fn drop(&mut self) {
                crate::width::set_ambiguous_wide(false);
            }
        }
        let _reset = Reset;
        let mut encoded = [0u8; 4];
        let mut mismatches = Vec::new();
        for ambiguous_wide in [false, true] {
            crate::width::set_ambiguous_wide(ambiguous_wide);
            for ch in (0..=u32::from(char::MAX)).filter_map(char::from_u32) {
                let s = ch.encode_utf8(&mut encoded);
                let expected = if ambiguous_wide {
                    UnicodeWidthStr::width_cjk(&*s)
                } else {
                    UnicodeWidthStr::width(&*s)
                };
                if char_display_width(ch) != expected || display_width(s) != expected {
                    mismatches.push((
                        ambiguous_wide,
                        u32::from(ch),
                        char_display_width(ch),
                        expected,
                    ));
                }
            }
        }
        assert!(
            mismatches.is_empty(),
            "{} mismatches (ambiguous_wide, scalar, char path, str width): {:x?}",
            mismatches.len(),
            &mismatches[..mismatches.len().min(64)]
        );
    }

    #[test]
    fn control_led_graphemes_collapse_to_one_replacement() {
        for input in [
            "\r\n", "\r\nb", "\r", "\n", "\x1b", "\x7f", "\u{85}", "\u{9f}",
        ] {
            let normalized = normalize_cell_symbol(input);
            assert_eq!(normalized, "\u{FFFD}", "input {input:?}");
            assert_eq!(UnicodeWidthStr::width(normalized.as_str()), 1);
        }
    }

    #[test]
    fn cell_size_within_cache_line() {
        let size = std::mem::size_of::<Cell>();
        assert!(
            size <= 64,
            "Cell size = {size}B; exceeds 64B cache-line budget. If intentional, update the const-assert and this test together."
        );
    }

    #[test]
    fn setters_keep_one_safe_grapheme_and_clear_links() {
        let mut cell = Cell::default();
        cell.hyperlink = Some(CompactString::new("https://example.com"));
        cell.set_symbol("👩‍💻tail\x1b");

        assert_eq!(cell.symbol, "👩‍💻");
        assert!(cell.hyperlink.is_none());

        cell.set_char('\x1b');
        assert_eq!(cell.symbol, "\u{FFFD}");
    }

    #[test]
    fn empty_symbol_is_explicit_continuation_state() {
        let mut cell = Cell::default();
        assert!(!cell.is_continuation());
        cell.set_continuation(Style::new());
        assert!(cell.is_continuation());
    }

    #[test]
    fn normalized_symbol_defends_against_direct_public_mutation() {
        let mut cell = Cell::default();
        cell.symbol = CompactString::new("\x1b]52;c;payload");
        assert_eq!(cell.normalized_symbol(), "\u{FFFD}");
    }
}
