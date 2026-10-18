//! Display-width measurement shared by the buffer, layout and text widgets.
//!
//! East Asian Ambiguous characters (`→`, `±`, `①`) are one column by Unicode
//! default but two columns on CJK-locale terminals configured that way.
//! [`crate::RunConfig::ambiguous_wide`] selects the wide interpretation for
//! the frame (issue #449); the kernel records it here before running the
//! frame closure so every measurement on this thread agrees.

use std::cell::Cell;

use unicode_width::UnicodeWidthStr;

thread_local! {
    static AMBIGUOUS_WIDE: Cell<bool> = const { Cell::new(false) };
}

/// Select the ambiguous-width interpretation for measurements on this thread.
pub(crate) fn set_ambiguous_wide(enabled: bool) {
    AMBIGUOUS_WIDE.with(|cell| cell.set(enabled));
}

/// Whether East Asian Ambiguous characters measure two columns on this thread.
#[inline]
pub(crate) fn ambiguous_wide() -> bool {
    AMBIGUOUS_WIDE.with(Cell::get)
}

/// Display width of `text` in terminal columns under the current
/// ambiguous-width setting.
///
/// Printable ASCII is one column per byte in both interpretations and takes
/// part in no width sequence, so it is measured inline; everything else goes
/// through one out-of-line copy of the `unicode-width` walk (issue #491:
/// the walk is `#[inline]` upstream and was copied into every codegen unit
/// that measured text).
#[inline]
pub(crate) fn str_width(text: &str) -> usize {
    if crate::cell::is_printable_ascii(text) {
        text.len()
    } else {
        unicode_str_width(text, ambiguous_wide())
    }
}

/// `UnicodeWidthStr::width`: the Unicode default width, regardless of the
/// ambiguous-width setting, through the single out-of-line walk.
#[inline]
pub(crate) fn unicode_width(text: &str) -> usize {
    unicode_str_width(text, false)
}

/// `unicode-width` string width, ambiguous-wide (`width_cjk`) when asked.
/// ASCII text never measures differently under the CJK tables.
#[inline(never)]
pub(crate) fn unicode_str_width(text: &str, ambiguous_wide: bool) -> usize {
    if ambiguous_wide && !text.is_ascii() {
        UnicodeWidthStr::width_cjk(text)
    } else {
        UnicodeWidthStr::width(text)
    }
}

/// Grapheme segmentation with a single out-of-line copy of the segmenter
/// (issue #491).
///
/// `unicode_segmentation::Graphemes::next` is `#[inline]`, so every codegen
/// unit that iterates graphemes carried its own ~3.4 KiB copy (five in a
/// hello-world, more in larger apps). The call was already out of line at
/// those sites; these wrappers keep it out of line in exactly one place.
/// Import this trait instead of `unicode_segmentation::UnicodeSegmentation`.
pub(crate) trait UnicodeSegmentation {
    /// Extended (or legacy) grapheme clusters of `self`.
    fn graphemes(&self, is_extended: bool) -> Graphemes<'_>;
    /// Grapheme clusters of `self` with their byte offsets.
    fn grapheme_indices(&self, is_extended: bool) -> GraphemeIndices<'_>;
}

impl UnicodeSegmentation for str {
    #[inline]
    fn graphemes(&self, is_extended: bool) -> Graphemes<'_> {
        Graphemes(unicode_segmentation::UnicodeSegmentation::graphemes(
            self,
            is_extended,
        ))
    }

    #[inline]
    fn grapheme_indices(&self, is_extended: bool) -> GraphemeIndices<'_> {
        GraphemeIndices {
            start: self.as_ptr() as usize,
            inner: self.graphemes(is_extended),
        }
    }
}

/// See [`UnicodeSegmentation::graphemes`].
#[derive(Clone)]
pub(crate) struct Graphemes<'a>(unicode_segmentation::Graphemes<'a>);

impl<'a> Iterator for Graphemes<'a> {
    type Item = &'a str;

    #[inline(never)]
    fn next(&mut self) -> Option<&'a str> {
        self.0.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl DoubleEndedIterator for Graphemes<'_> {
    #[inline(never)]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

/// See [`UnicodeSegmentation::grapheme_indices`]. Built on [`Graphemes`]
/// (the same pointer arithmetic as `unicode_segmentation::GraphemeIndices`)
/// so it shares the single segmenter copy.
#[derive(Clone)]
pub(crate) struct GraphemeIndices<'a> {
    start: usize,
    inner: Graphemes<'a>,
}

impl<'a> Iterator for GraphemeIndices<'a> {
    type Item = (usize, &'a str);

    #[inline]
    fn next(&mut self) -> Option<(usize, &'a str)> {
        let start = self.start;
        self.inner.next().map(|s| (s.as_ptr() as usize - start, s))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl DoubleEndedIterator for GraphemeIndices<'_> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        let start = self.start;
        self.inner
            .next_back()
            .map(|s| (s.as_ptr() as usize - start, s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RunConfig, TestBackend};

    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            set_ambiguous_wide(false);
        }
    }

    #[test]
    fn issue_449_ambiguous_width_switch() {
        let _reset = Reset;
        assert_eq!(str_width("→±①"), 3);
        set_ambiguous_wide(true);
        assert_eq!(str_width("→±①"), 6);
        assert_eq!(str_width("abc"), 3);
        assert_eq!(str_width("한글"), 4);
        set_ambiguous_wide(false);
        assert_eq!(str_width("→±①"), 3);
    }

    /// Issue #491: the inline printable-ASCII path and the out-of-line walk
    /// agree with `unicode-width` on every ASCII pair (controls, CR LF) in
    /// both interpretations, and graphemes match upstream segmentation.
    #[test]
    fn issue_491_shared_walks_match_upstream() {
        let _reset = Reset;
        for wide in [false, true] {
            set_ambiguous_wide(wide);
            for a in 0u8..0x80 {
                for b in 0u8..0x80 {
                    let s = String::from_utf8(vec![a, b]).unwrap();
                    assert_eq!(str_width(&s), UnicodeWidthStr::width(s.as_str()), "{s:?}");
                }
            }
            for s in [
                "→±①",
                "한글 ok",
                "e\u{301}\u{1F469}\u{200D}\u{1F4BB}",
                "a\r\nb",
            ] {
                let want = if wide {
                    UnicodeWidthStr::width_cjk(s)
                } else {
                    UnicodeWidthStr::width(s)
                };
                assert_eq!(str_width(s), want, "{s:?}");
                assert_eq!(unicode_width(s), UnicodeWidthStr::width(s), "{s:?}");
            }
        }
        let text = "a\u{301}\r\n\u{1F1F0}\u{1F1F7}x\u{1F44D}\u{1F3FD}";
        let ours: Vec<_> = UnicodeSegmentation::grapheme_indices(text, true).collect();
        let upstream: Vec<_> =
            unicode_segmentation::UnicodeSegmentation::grapheme_indices(text, true).collect();
        assert_eq!(ours, upstream);
        let ours: Vec<_> = UnicodeSegmentation::graphemes(text, true).rev().collect();
        let upstream: Vec<_> = unicode_segmentation::UnicodeSegmentation::graphemes(text, true)
            .rev()
            .collect();
        assert_eq!(ours, upstream);
        let ours: Vec<_> = UnicodeSegmentation::grapheme_indices(text, true)
            .rev()
            .collect();
        let upstream: Vec<_> =
            unicode_segmentation::UnicodeSegmentation::grapheme_indices(text, true)
                .rev()
                .collect();
        assert_eq!(ours, upstream);
    }

    /// Minimal headless backend so the frame runs with a custom
    /// [`RunConfig`] (`TestBackend` always renders with the default config).
    struct ConfiguredBackend {
        buffer: crate::Buffer,
    }

    impl crate::Backend for ConfiguredBackend {
        fn size(&self) -> (u32, u32) {
            (self.buffer.area.width, self.buffer.area.height)
        }
        fn buffer_mut(&mut self) -> &mut crate::Buffer {
            &mut self.buffer
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn render_row(
        width: u32,
        height: u32,
        config: &RunConfig,
        f: impl Fn(&mut crate::Context),
    ) -> Vec<String> {
        let mut backend = ConfiguredBackend {
            buffer: crate::Buffer::empty(crate::Rect::new(0, 0, width, height)),
        };
        let mut state = crate::AppState::new();
        crate::frame_owned(&mut backend, &mut state, config, Vec::new(), &mut |ui| {
            f(ui)
        })
        .unwrap();
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| backend.buffer.get(x, y).symbol.to_string())
                    .collect::<Vec<_>>()
                    .join("|")
            })
            .collect()
    }

    #[test]
    fn issue_449_ambiguous_wide_lays_out_six_columns() {
        let _reset = Reset;
        let wide = RunConfig::default().ambiguous_wide(true);
        // Each ambiguous glyph occupies two cells, so the bar lands at x = 6.
        let rows = render_row(10, 1, &wide, |ui| {
            ui.text("→±①#");
        });
        assert_eq!(rows[0], "→||±||①||#| | | ");

        // Wrapping: eight columns of content in a 5-wide box break after `→±`.
        let rows = render_row(5, 2, &wide, |ui| {
            ui.text("→±①→").wrap();
        });
        assert_eq!(rows, ["→||±|| ", "①||→|| "]);

        // Truncation: a 5-wide line cannot hold the third two-column glyph.
        let rows = render_row(5, 1, &wide, |ui| {
            ui.text("→±①→").truncate();
        });
        assert!(!rows[0].contains('①'), "{rows:?}");

        // The TestBackend default keeps the one-column Unicode default.
        let mut tb = TestBackend::new(10, 1);
        tb.render(|ui| {
            ui.text("→±①#");
        });
        assert_eq!(tb.buffer().get(3, 0).symbol.as_str(), "#");
    }
}
