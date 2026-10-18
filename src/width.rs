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
#[inline]
pub(crate) fn str_width(text: &str) -> usize {
    if text.is_ascii() || !ambiguous_wide() {
        UnicodeWidthStr::width(text)
    } else {
        UnicodeWidthStr::width_cjk(text)
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
