// Scrollable container variant with a per-line left gutter and search-style
// highlight rendering.
//
// Introduced in v0.20.0 (#235). Companion to the existing `scrollable` /
// `scroll_col` / `scroll_row` widgets in `layout.rs` and the `ScrollState`
// highlight extensions in `widgets/collections.rs`.

use super::*;

impl Context {
    /// Scrollable column with a left gutter rendered per visible line.
    ///
    /// `total_lines` is the absolute count of content lines. `viewport_height`
    /// is the number of rows the viewport should occupy. `gutter_fn` receives
    /// the absolute content line index (0-based) and returns the gutter label.
    /// `f` is invoked for each visible line (0-indexed within the viewport)
    /// and renders that line's content. Highlighted lines (set via
    /// [`ScrollState::set_highlights`]) receive an accent background.
    ///
    /// Returns a [`GutterResponse`] with the current highlight index and
    /// total highlight count for callers wiring up `n` / `N` search-result
    /// navigation keys.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::{HighlightRange, ScrollState};
    /// # let mut scroll = ScrollState::new();
    /// # scroll.set_highlights(&[HighlightRange::line(7), HighlightRange::line(15)]);
    /// # let lines: Vec<&str> = vec![];
    /// # slt::run(|ui: &mut slt::Context| {
    /// let r = ui.scrollable_with_gutter(
    ///     &mut scroll,
    ///     lines.len(),
    ///     10,
    ///     |idx| format!("{:>4}", idx + 1),
    ///     |ui, abs_line| {
    ///         if let Some(line) = lines.get(abs_line) {
    ///             ui.text(*line);
    ///         }
    ///     },
    /// );
    /// if let Some(i) = r.current_highlight {
    ///     // show "match i of N" status
    /// }
    /// # });
    /// ```
    pub fn scrollable_with_gutter<G, F>(
        &mut self,
        state: &mut ScrollState,
        total_lines: usize,
        viewport_height: u32,
        gutter_fn: G,
        mut f: F,
    ) -> GutterResponse
    where
        G: Fn(usize) -> String,
        F: FnMut(&mut Context, usize),
    {
        // Sync state's bounds and clamp offset.
        state.set_bounds(total_lines as u32, viewport_height);
        let max_offset = total_lines.saturating_sub(viewport_height as usize);
        state.offset = state.offset.min(max_offset);

        // Wheel scroll consumption — mirror the standard `scrollable` widget.
        let next_id = self.rollback.interaction_count;
        if let Some(rect) = self.prev_hit_map.get(next_id).copied() {
            self.gutter_consume_wheel(rect, state);
        }

        // Compute gutter width across visible lines.
        let visible_count =
            (viewport_height as usize).min(total_lines.saturating_sub(state.offset));
        let mut gutter_w = 1usize;
        for i in 0..visible_count {
            let abs = state.offset + i;
            let label = gutter_fn(abs);
            let w = UnicodeWidthStr::width(label.as_str());
            if w > gutter_w {
                gutter_w = w;
            }
        }

        let highlights: Vec<HighlightRange> = state.highlights().to_vec();
        let current = state.current_highlight();
        let theme = self.theme;

        let response = self.row(|ui| {
            // Gutter column.
            let _ = ui.container().w(gutter_w as u32 + 1).col(|ui| {
                for i in 0..visible_count {
                    let abs = state.offset + i;
                    let label = gutter_fn(abs);
                    let label_w = UnicodeWidthStr::width(label.as_str());
                    let pad = gutter_w.saturating_sub(label_w);
                    let mut padded = String::with_capacity(label.len() + pad + 1);
                    for _ in 0..pad {
                        padded.push(' ');
                    }
                    padded.push_str(&label);
                    padded.push(' ');

                    let hit = highlights.iter().enumerate().find(|(_, h)| h.contains(abs));
                    let style = match hit {
                        Some((idx, _)) if Some(idx) == current => {
                            Style::new().fg(theme.bg).bg(theme.accent).bold()
                        }
                        Some(_) => Style::new().fg(theme.text).bg(theme.surface_hover),
                        None => Style::new().fg(theme.text_dim),
                    };
                    ui.styled(padded, style);
                }
            });

            // Content column. Each visible line is rendered by the closure;
            // highlights receive a background accent on the entire row.
            let _ = ui.container().grow(1).col(|ui| {
                for i in 0..visible_count {
                    let abs = state.offset + i;
                    let hit = highlights.iter().enumerate().find(|(_, h)| h.contains(abs));
                    match hit {
                        Some((idx, _)) if Some(idx) == current => {
                            let _ = ui.container().bg(theme.surface_hover).row(|ui| f(ui, abs));
                        }
                        Some(_) => {
                            let _ = ui.container().bg(theme.surface).row(|ui| f(ui, abs));
                        }
                        None => {
                            let _ = ui.row(|ui| f(ui, abs));
                        }
                    }
                }
            });
        });

        GutterResponse {
            response,
            current_highlight: current,
            total_highlights: highlights.len(),
        }
    }

    fn gutter_consume_wheel(&mut self, rect: Rect, state: &mut ScrollState) {
        let mut consumed: Vec<usize> = Vec::new();
        let delta = self.scroll_lines_per_event as usize;
        for (i, mouse) in self.mouse_events_in_rect(rect) {
            match mouse.kind {
                MouseKind::ScrollUp => {
                    state.scroll_up(delta);
                    consumed.push(i);
                }
                MouseKind::ScrollDown => {
                    state.scroll_down(delta);
                    consumed.push(i);
                }
                _ => {}
            }
        }
        self.consume_indices(consumed);
    }
}
