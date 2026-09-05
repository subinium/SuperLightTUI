use super::*;

struct DeferredDrawClipGuard<'a> {
    buffer: &'a mut crate::buffer::Buffer,
    clip_depth: usize,
    kitty_clip_depth: usize,
    kitty_horizontal_clip_depth: usize,
}

impl<'a> DeferredDrawClipGuard<'a> {
    fn new(
        buffer: &'a mut crate::buffer::Buffer,
        rect: Rect,
        left_clip_cols: u32,
        top_clip_rows: u32,
        original_width: u32,
        original_height: u32,
    ) -> Self {
        let clip_depth = buffer.clip_stack.len();
        let kitty_clip_depth = buffer.kitty_clip_info_stack.len();
        let kitty_horizontal_clip_depth = buffer.kitty_horizontal_clip_stack.len();
        buffer.push_clip(rect);
        buffer.push_kitty_clip(crate::buffer::KittyClipInfo {
            top_clip_rows,
            original_height,
        });
        buffer.push_kitty_horizontal_clip(crate::buffer::KittyHorizontalClipInfo {
            left_clip_cols,
            original_width,
        });
        Self {
            buffer,
            clip_depth,
            kitty_clip_depth,
            kitty_horizontal_clip_depth,
        }
    }

    fn buffer(&mut self) -> &mut crate::buffer::Buffer {
        self.buffer
    }
}

impl Drop for DeferredDrawClipGuard<'_> {
    fn drop(&mut self) {
        self.buffer.clip_stack.truncate(self.clip_depth);
        self.buffer
            .kitty_clip_info_stack
            .truncate(self.kitty_clip_depth);
        self.buffer
            .kitty_horizontal_clip_stack
            .truncate(self.kitty_horizontal_clip_depth);
    }
}

/// Invoke one deferred raw-draw callback with balanced cell and Kitty clips.
///
/// The callback panic is returned to the frame kernel instead of crossing the
/// cleanup boundary. The caller decides whether to render an error-boundary
/// fallback or write persistent frame state back and resume unwinding.
#[allow(dead_code)] // Called by the #340 frame-kernel integration in src/lib.rs.
pub(crate) fn invoke_deferred_draw(
    buffer: &mut crate::buffer::Buffer,
    rect: Rect,
    left_clip_cols: u32,
    top_clip_rows: u32,
    original_width: u32,
    original_height: u32,
    draw: impl FnOnce(&mut crate::buffer::Buffer, Rect),
) -> Result<(), Box<dyn std::any::Any + Send>> {
    let mut clips = DeferredDrawClipGuard::new(
        buffer,
        rect,
        left_clip_cols,
        top_clip_rows,
        original_width,
        original_height,
    );
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        draw(clips.buffer(), rect);
    }));
    drop(clips);
    result
}

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
    if width == 0 {
        return String::new();
    }
    let cell_width = UnicodeWidthStr::width(cell);
    if cell_width <= width {
        let mut out = String::with_capacity(width);
        out.push_str(cell);
        out.extend(std::iter::repeat_n(' ', width - cell_width));
        return out;
    }
    if width == 1 {
        return "\u{2026}".to_string();
    }
    let target = width - 1;
    let mut out = String::with_capacity(width);
    let mut acc = 0usize;
    for grapheme in cell.graphemes(true) {
        let ch_width = UnicodeWidthStr::width(grapheme);
        if acc + ch_width > target {
            break;
        }
        out.push_str(grapheme);
        acc += ch_width;
    }
    out.push('\u{2026}');
    // Pad in case the last char was wide and left a one-cell gap before `…`.
    let out_width = UnicodeWidthStr::width(out.as_str());
    out.extend(std::iter::repeat_n(' ', width.saturating_sub(out_width)));
    out
}

#[cfg(test)]
mod v024_table_tests {
    use super::*;

    #[test]
    fn truncation_keeps_clusters_and_exact_column_budgets() {
        for cluster in [
            "\u{1f469}\u{200d}\u{1f4bb}",
            "\u{1f1f0}\u{1f1f7}",
            "e\u{301}",
            "\u{915}\u{94d}\u{937}\u{93f}",
        ] {
            let source = format!("{cluster}abcd");
            for width in 0..=10 {
                let rendered = clamp_table_cell(&source, width);
                assert_eq!(
                    UnicodeWidthStr::width(rendered.as_str()),
                    width as usize,
                    "{source:?} width={width}"
                );
                if rendered.contains('\u{2026}') {
                    let prefix = rendered.split('\u{2026}').next().unwrap();
                    assert!(
                        source
                            .grapheme_indices(true)
                            .any(|(byte, _)| byte == prefix.len()),
                        "partial cluster: {rendered:?}"
                    );
                }
            }
        }
        assert_eq!(clamp_table_cell("abcdef", 4), "abc\u{2026}");
        assert_eq!(clamp_table_cell("\u{301}", 0), "");
        assert_eq!(
            format_table_row(
                &["\u{1f469}\u{200d}\u{1f4bb}abcd".into(), "Z".into()],
                &[4, 2],
                "|"
            ),
            "\u{1f469}\u{200d}\u{1f4bb}a\u{2026}|Z "
        );
    }
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
    centered.extend(std::iter::repeat_n(' ', left));
    centered.push_str(text);
    centered.extend(std::iter::repeat_n(' ', right));
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
                .is_none_or(|next| next.logical_row != logical_row);
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

/// Intrinsic-size measurement API (v0.21.1).
///
/// These read-only queries expose the layout engine's text-wrapping math and
/// the previous frame's named-container geometry without changing any rendering
/// path. They let app code reserve space, decide pagination, or position
/// floating UI relative to a widget that was laid out last frame.
impl Context {
    /// The intrinsic `(width, height_in_rows)` `text` would occupy, in cells.
    ///
    /// Reuses the exact word-wrap kernel the layout engine runs
    /// (`wrap_lines` via this crate's `tree`
    /// module), so the answer always matches what a `ui.text(text).wrap()`
    /// would actually render — width logic is never duplicated here.
    ///
    /// * When `max_width` is `None`, the text is measured unwrapped: width is
    ///   the widest hard-break line, height is the number of `'\n'`-separated
    ///   lines (at least 1).
    /// * When `max_width` is `Some(w)` with `w > 0`, the text is wrapped to
    ///   `w` columns; the returned width is the widest wrapped line (`<= w`)
    ///   and the height is the wrapped line count.
    /// * `Some(0)` is treated like `None` (no width budget — honor hard breaks
    ///   only), mirroring the layout kernel's zero-width contract.
    ///
    /// Width is the terminal display width (wide CJK glyphs count as 2,
    /// zero-width combining marks as 0). The result is clamped to `u16`; a
    /// pathological line wider than `u16::MAX` cells saturates rather than
    /// wrapping.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// // Unwrapped: width is the longest line, height the line count.
    /// let (w, h) = ui.measure_text("hello\nworld!", None);
    /// assert_eq!((w, h), (6, 2));
    ///
    /// // Wrapped to 5 columns: the long word breaks across rows.
    /// let (w, h) = ui.measure_text("alpha beta gamma", Some(5));
    /// assert!(w <= 5 && h >= 1);
    /// # });
    /// ```
    pub fn measure_text(&self, text: &str, max_width: Option<u16>) -> (u16, u16) {
        // `Some(0)` collapses to the "no budget" path so we never feed a
        // zero-width wrap (which the kernel treats as hard-break-only anyway).
        let budget = match max_width {
            Some(w) if w > 0 => w as u32,
            // `u32::MAX` is the layout engine's "unbounded width" sentinel
            // (see `textarea_build_visual_lines`); `wrap_lines` honors only
            // hard breaks at that width, giving the unwrapped measurement.
            _ => u32::MAX,
        };

        let lines = crate::layout::wrap_lines(text, budget);
        let height = lines.len().max(1);
        let width = lines
            .iter()
            .map(|line| UnicodeWidthStr::width(line.as_str()))
            .max()
            .unwrap_or(0);

        (clamp_u16(width), clamp_u16(height))
    }

    /// The [`Rect`] a named widget/container occupied on the **last completed
    /// frame**, or `None` if no group with that `name` was rendered.
    ///
    /// Reads the same `name → rect` bookkeeping that powers group hover/focus
    /// styling (`prev_group_rects`), captured at the end of the previous
    /// frame's collect pass. Register a name with
    /// [`Context::group`](crate::Context::group):
    ///
    /// ```ignore
    /// ui.group("sidebar").border(slt::Border::Rounded).col(|ui| { /* … */ });
    /// // …next frame:
    /// if let Some(r) = ui.measured_rect("sidebar") {
    ///     ui.text(format!("sidebar is {}x{}", r.width, r.height));
    /// }
    /// ```
    ///
    /// Returns `None` on the first frame (nothing measured yet) and for any
    /// name that was not rendered as a `group(...)` last frame. If the same
    /// name is used for multiple groups, the first match in render order is
    /// returned.
    pub fn measured_rect(&self, name: &str) -> Option<Rect> {
        self.prev_group_rects
            .iter()
            .find(|(group_name, _)| group_name.as_ref() == name)
            .map(|(_, rect)| *rect)
    }
}

/// Saturating `usize -> u16` for intrinsic-size results.
///
/// A measured extent wider/taller than `u16::MAX` cells is pathological (no
/// real terminal is that large); saturating keeps the public return type a
/// compact `u16` without an overflow panic.
#[inline]
fn clamp_u16(value: usize) -> u16 {
    value.min(u16::MAX as usize) as u16
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

#[cfg(test)]
mod measure_tests {
    use crate::test_utils::TestBackend;
    use crate::{Border, Context, FrameState, Theme};

    #[test]
    fn measure_text_unwrapped_reports_widest_line_and_line_count() {
        let mut state = FrameState::default();
        let ui = Context::new(Vec::new(), 40, 10, &mut state, Theme::dark());

        // Two hard-break lines: width = widest line, height = line count.
        let (w, h) = ui.measure_text("hello\nworld!", None);
        assert_eq!((w, h), (6, 2));

        // Single line, no breaks → height 1.
        assert_eq!(ui.measure_text("abc", None), (3, 1));

        // Empty string is one blank line of zero width.
        assert_eq!(ui.measure_text("", None), (0, 1));
    }

    #[test]
    fn measure_text_wraps_to_budget_and_never_exceeds_it() {
        let mut state = FrameState::default();
        let ui = Context::new(Vec::new(), 40, 10, &mut state, Theme::dark());

        // "alpha beta gamma" wrapped to 5 columns: every word is <= 5 wide so
        // it lands one word per line → 3 rows, widest line "gamma" = 5.
        let (w, h) = ui.measure_text("alpha beta gamma", Some(5));
        assert!(w <= 5, "wrapped width {w} must not exceed the budget");
        assert_eq!(h, 3, "three 5-wide words wrap onto three rows");
        assert_eq!(w, 5);

        // A word longer than the budget is hard-split across rows; height
        // grows but width still stays within the budget.
        let (w, h) = ui.measure_text("abcdefghij", Some(4));
        assert!(w <= 4);
        assert!(h >= 3, "10 chars at width 4 need at least 3 rows, got {h}");
    }

    #[test]
    fn measure_text_some_zero_is_treated_as_unbounded() {
        // Edge case: `Some(0)` must not feed a zero-width wrap. It honors hard
        // breaks only, identical to `None`.
        let mut state = FrameState::default();
        let ui = Context::new(Vec::new(), 40, 10, &mut state, Theme::dark());
        assert_eq!(
            ui.measure_text("a b c\nlonger line", Some(0)),
            ui.measure_text("a b c\nlonger line", None),
        );
    }

    #[test]
    fn measure_text_counts_wide_cjk_glyphs_as_two_cells() {
        let mut state = FrameState::default();
        let ui = Context::new(Vec::new(), 40, 10, &mut state, Theme::dark());
        // Two double-width CJK glyphs measure as 4 cells, one row.
        assert_eq!(ui.measure_text("한글", None), (4, 1));
    }

    #[test]
    fn measured_rect_is_none_on_first_frame() {
        let mut state = FrameState::default();
        let ui = Context::new(Vec::new(), 40, 10, &mut state, Theme::dark());
        // Nothing has been rendered yet → no prior geometry.
        assert!(ui.measured_rect("panel").is_none());
    }

    #[test]
    fn measured_rect_returns_group_geometry_after_a_render() {
        // Render a named group on frame 1; on frame 2 the previous frame's
        // collected `prev_group_rects` makes the rect queryable.
        let mut backend = TestBackend::new(40, 10);

        backend.render(|ui| {
            let _ = ui.group("panel").border(Border::Rounded).col(|ui| {
                ui.text("hi");
            });
        });

        let mut seen: Option<crate::Rect> = None;
        backend.render(|ui| {
            seen = ui.measured_rect("panel");
            // A name that was never rendered stays `None` — edge case guard.
            assert!(ui.measured_rect("does-not-exist").is_none());
        });

        let rect = seen.expect("named group must have a measured rect after render");
        assert!(
            rect.width > 0 && rect.height > 0,
            "measured rect must be non-empty, got {rect:?}"
        );
        // The group fits inside the 40x10 backend area.
        assert!(rect.x + rect.width <= 40);
        assert!(rect.y + rect.height <= 10);
    }
}

#[cfg(test)]
mod deferred_draw_tests {
    use super::invoke_deferred_draw;
    use crate::buffer::{Buffer, KittyClipInfo};
    use crate::{Rect, Style};

    #[test]
    fn nested_draw_panic_restores_both_clip_stacks() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 10));
        let outer_clip = Rect::new(1, 1, 18, 8);
        let outer_kitty = KittyClipInfo {
            top_clip_rows: 1,
            original_height: 12,
        };
        let outer_horizontal = crate::buffer::KittyHorizontalClipInfo {
            left_clip_cols: 1,
            original_width: 20,
        };
        buffer.push_clip(outer_clip);
        buffer.push_kitty_clip(outer_kitty);
        buffer.push_kitty_horizontal_clip(outer_horizontal);

        let result = invoke_deferred_draw(
            &mut buffer,
            Rect::new(2, 2, 10, 4),
            0,
            2,
            10,
            8,
            |buf, _| {
                let inner =
                    invoke_deferred_draw(buf, Rect::new(3, 3, 4, 2), 0, 0, 4, 2, |buf, rect| {
                        buf.set_string(rect.x, rect.y, "partial", Style::new());
                        panic!("nested raw draw failed");
                    });
                std::panic::resume_unwind(inner.expect_err("inner draw should panic"));
            },
        );

        assert!(result.is_err());
        assert_eq!(buffer.clip_stack, vec![outer_clip]);
        assert_eq!(buffer.kitty_clip_info_stack, vec![outer_kitty]);
        assert_eq!(buffer.kitty_horizontal_clip_stack, vec![outer_horizontal]);
    }

    #[test]
    fn multiple_regions_leave_no_clip_state_after_success() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 10));
        for x in [0, 10] {
            invoke_deferred_draw(
                &mut buffer,
                Rect::new(x, 0, 10, 5),
                0,
                0,
                10,
                5,
                |buf, rect| {
                    buf.set_string(rect.x, rect.y, "ok", Style::new());
                },
            )
            .expect("draw should succeed");
        }

        assert!(buffer.clip_stack.is_empty());
        assert!(buffer.kitty_clip_info_stack.is_empty());
    }
}
