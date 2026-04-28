// Scrollable container variant with a per-line left gutter and search-style
// highlight rendering.
//
// Introduced in v0.20.0 (#235). Companion to the existing `scrollable` /
// `scroll_col` / `scroll_row` widgets in `layout.rs` and the `ScrollState`
// highlight extensions in `widgets/collections.rs`.
//
// API consistency pass (v0.20.0): the four+ positional args were collapsed
// into a [`GutterOpts<G>`] struct so callers don't have to remember argument
// order. The 90% case (line numbers) gets a [`GutterOpts::line_numbers`]
// shortcut so most callers never write the closure manually.

use super::*;

/// Options for [`Context::scrollable_with_gutter`].
///
/// Carries the bookkeeping arguments together so call sites become readable:
///
/// ```no_run
/// # use slt::{GutterOpts, ScrollState};
/// # let mut scroll = ScrollState::default();
/// # slt::run(|ui: &mut slt::Context| {
/// // 90% case — automatic line numbers.
/// ui.scrollable_with_gutter(
///     &mut scroll,
///     GutterOpts::line_numbers(120, 24),
///     |ui, line| { ui.text(format!("line {line}")); },
/// );
///
/// // Custom gutter labels.
/// ui.scrollable_with_gutter(
///     &mut scroll,
///     GutterOpts::new(120, 24, |i| if i == 7 { "!".to_string() } else { String::new() }),
///     |ui, line| { ui.text(format!("line {line}")); },
/// );
/// # });
/// ```
pub struct GutterOpts<G> {
    /// Total number of content lines.
    pub total_lines: usize,
    /// Viewport height in rows.
    pub viewport_height: u32,
    /// Closure that returns the gutter label for a given absolute line index.
    pub gutter_fn: G,
}

impl<G> GutterOpts<G>
where
    G: Fn(usize) -> String,
{
    /// Build options with an explicit gutter labeling closure.
    pub fn new(total_lines: usize, viewport_height: u32, gutter_fn: G) -> Self {
        Self {
            total_lines,
            viewport_height,
            gutter_fn,
        }
    }
}

impl GutterOpts<fn(usize) -> String> {
    /// Shortcut for the 90% case: render 1-based line numbers in the gutter.
    ///
    /// Equivalent to `GutterOpts::new(total, viewport, |i| format!("{}", i + 1))`
    /// but uses a function pointer to avoid forcing the caller to name the
    /// closure type.
    pub fn line_numbers(total_lines: usize, viewport_height: u32) -> Self {
        fn label(i: usize) -> String {
            format!("{}", i + 1)
        }
        Self {
            total_lines,
            viewport_height,
            gutter_fn: label,
        }
    }
}

impl Context {
    /// Scrollable column with a left gutter rendered per visible line.
    ///
    /// `state` is the active scroll state. `opts` carries `total_lines`,
    /// `viewport_height`, and the gutter labeling closure (use
    /// [`GutterOpts::line_numbers`] for the common case). `body_fn` is invoked
    /// for each visible line and renders that line's content. Highlighted
    /// lines (set via [`ScrollState::set_highlights`]) receive an accent
    /// background.
    ///
    /// Returns a [`GutterResponse`] with the current highlight index and
    /// total highlight count for callers wiring up `n` / `N` search-result
    /// navigation keys.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::{GutterOpts, HighlightRange, ScrollState};
    /// # let mut scroll = ScrollState::new();
    /// # scroll.set_highlights(&[HighlightRange::single(7), HighlightRange::single(15)]);
    /// # let lines: Vec<&str> = vec![];
    /// # slt::run(|ui: &mut slt::Context| {
    /// let r = ui.scrollable_with_gutter(
    ///     &mut scroll,
    ///     GutterOpts::line_numbers(lines.len(), 10),
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
        opts: GutterOpts<G>,
        mut f: F,
    ) -> GutterResponse
    where
        G: Fn(usize) -> String,
        F: FnMut(&mut Context, usize),
    {
        let GutterOpts {
            total_lines,
            viewport_height,
            gutter_fn,
        } = opts;

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
