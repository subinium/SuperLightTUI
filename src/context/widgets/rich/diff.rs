//! Diff viewer widget ([`Context::diff`]) — renders a [`DiffState`] in a
//! unified or side-by-side layout with per-hunk collapse and accept/reject.
//!
//! Rendering is virtualized: the widget emits one command per *visible* row,
//! so cost scales with the viewport, not the patch.

use super::*;
use crate::widgets::{
    DiffColors, DiffFileStatus, DiffLayout, DiffLine, DiffLineKind, DiffResponse, DiffRow,
    DiffState, HunkAction, HunkDecision, HunkHighlight,
};

/// Default tab stop width used to expand `\t` in diff content.
const DEFAULT_TAB_WIDTH: usize = 4;
/// Default minimum width (cells) for [`DiffLayout::Split`]; narrower areas
/// fall back to the unified layout.
const DEFAULT_SPLIT_MIN_WIDTH: u32 = 80;
/// Rows scrolled per mouse-wheel notch.
const WHEEL_ROWS: usize = 3;
/// Blend weight of `success` / `error` into `surface` for line backgrounds.
const LINE_BG_ALPHA: f64 = 0.25;

impl Context {
    /// Begin rendering a diff viewer for `state`.
    ///
    /// Shows the parsed patch file by file: a file header (status, path,
    /// `+added -removed`), then each hunk header followed by its lines with
    /// an old/new line-number gutter. Chain [`DiffView::layout`] for the
    /// side-by-side layout, [`DiffView::lang`] to force a syntax language,
    /// and [`DiffView::h`] for the viewport height. The returned
    /// [`DiffView`] renders on drop; call [`DiffView::show`] to get the
    /// [`DiffResponse`].
    ///
    /// While focused: Up/Down (`k`/`j`) move the cursor, `]`/`n` and `[`/`p`
    /// jump between hunks, Enter/Space collapse or expand the focused hunk,
    /// and `a` / `r` accept or reject it (rebind through
    /// [`DiffState::keys`]). Clicking a hunk header toggles it and the mouse
    /// wheel scrolls. Only visible rows are rendered.
    ///
    /// Syntax highlighting uses the `syntax-*` feature matching each file's
    /// extension (or [`DiffView::lang`]) and falls back to plain text.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::{DiffLayout, DiffState, HunkAction};
    ///
    /// let patch = "--- a/x.rs\n+++ b/x.rs\n@@ -1 +1 @@\n-let a = 1;\n+let a = 2;\n";
    /// let mut diff = DiffState::from_unified(patch).expect("valid patch");
    /// slt::run(|ui| {
    ///     let r = ui.diff(&mut diff).layout(DiffLayout::Split).h(20).show();
    ///     for (id, action) in &r.hunk_actions {
    ///         if *action == HunkAction::Accept {
    ///             let _ = id; // apply the hunk
    ///         }
    ///     }
    /// })?;
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub fn diff<'a>(&'a mut self, state: &'a mut DiffState) -> DiffView<'a> {
        DiffView {
            ctx: Some(self),
            state,
            opts: DiffOpts::default(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct DiffOpts<'a> {
    layout: DiffLayout,
    lang: Option<&'a str>,
    width: Option<u32>,
    height: Option<u32>,
    tab_width: usize,
    split_min_width: u32,
    line_numbers: bool,
    colors: DiffColors,
}

impl Default for DiffOpts<'_> {
    fn default() -> Self {
        Self {
            layout: DiffLayout::Unified,
            lang: None,
            width: None,
            height: None,
            tab_width: DEFAULT_TAB_WIDTH,
            split_min_width: DEFAULT_SPLIT_MIN_WIDTH,
            line_numbers: true,
            colors: DiffColors::default(),
        }
    }
}

/// Diff viewer builder returned by [`Context::diff`]. Renders on `Drop`.
///
/// Consuming-builder shape shared with [`CodeBlock`]: drop the value to
/// render without reading the response, or call [`Self::show`] to render
/// and obtain the [`DiffResponse`].
pub struct DiffView<'a> {
    ctx: Option<&'a mut Context>,
    state: &'a mut DiffState,
    opts: DiffOpts<'a>,
}

#[expect(
    clippy::return_self_not_must_use,
    reason = "the widget call already recorded the element (or renders on drop); modifiers edit it in place, so a statement-terminated chain is the intended use"
)]
impl<'a> DiffView<'a> {
    /// Unified (default) or side-by-side layout.
    pub fn layout(mut self, layout: DiffLayout) -> Self {
        self.opts.layout = layout;
        self
    }

    /// Force the syntax-highlight language for every file (e.g. `"rust"`).
    /// By default each file's extension picks the language.
    pub fn lang(mut self, lang: &'a str) -> Self {
        self.opts.lang = Some(lang);
        self
    }

    /// Fixed width in cells. Defaults to the parent's available width.
    pub fn w(mut self, width: u32) -> Self {
        self.opts.width = Some(width);
        self
    }

    /// Viewport height in rows. Defaults to the parent's available height.
    pub fn h(mut self, height: u32) -> Self {
        self.opts.height = Some(height);
        self
    }

    /// Tab stop width used to expand `\t` (default 4; `0` is treated as 1).
    pub fn tab_width(mut self, width: usize) -> Self {
        self.opts.tab_width = width.max(1);
        self
    }

    /// Minimum width for [`DiffLayout::Split`] (default 80). Narrower areas
    /// render the unified layout instead.
    pub fn split_min_width(mut self, width: u32) -> Self {
        self.opts.split_min_width = width;
        self
    }

    /// Show or hide the line-number gutter (shown by default).
    pub fn line_numbers(mut self, show: bool) -> Self {
        self.opts.line_numbers = show;
        self
    }

    /// Override the added / removed backgrounds or hunk-header color.
    pub fn colors(mut self, colors: DiffColors) -> Self {
        self.opts.colors = colors;
        self
    }

    /// Render now and return the [`DiffResponse`].
    pub fn show(mut self) -> DiffResponse {
        match self.ctx.take() {
            Some(ctx) => render_diff(ctx, self.state, self.opts),
            None => DiffResponse::default(),
        }
    }
}

impl Drop for DiffView<'_> {
    fn drop(&mut self) {
        if let Some(ctx) = self.ctx.take() {
            let _ = render_diff(ctx, self.state, self.opts);
        }
    }
}

/// Input actions resolved from [`crate::widgets::DiffKeys`] and fixed keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiffAction {
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
    NextHunk,
    PrevHunk,
    Toggle,
    Decide(HunkAction),
}

fn classify_key(state: &DiffState, code: &KeyCode) -> Option<DiffAction> {
    let keys = &state.keys;
    let action = if keys.up.contains(code) {
        DiffAction::Up
    } else if keys.down.contains(code) {
        DiffAction::Down
    } else if keys.next_hunk.contains(code) {
        DiffAction::NextHunk
    } else if keys.prev_hunk.contains(code) {
        DiffAction::PrevHunk
    } else if keys.toggle.contains(code) {
        DiffAction::Toggle
    } else if keys.accept.contains(code) {
        DiffAction::Decide(HunkAction::Accept)
    } else if keys.reject.contains(code) {
        DiffAction::Decide(HunkAction::Reject)
    } else {
        match code {
            KeyCode::PageUp => DiffAction::PageUp,
            KeyCode::PageDown => DiffAction::PageDown,
            KeyCode::Home => DiffAction::Home,
            KeyCode::End => DiffAction::End,
            _ => return None,
        }
    };
    Some(action)
}

fn toggle_hunk(state: &mut DiffState, global: usize, split: bool, out: &mut DiffResponse) {
    let collapsed = !state.collapsed_global(global);
    state.set_collapsed_global(global, collapsed);
    state.ensure_rows(split);
    out.toggled.push(state.id_of(global));
}

fn apply_action(
    state: &mut DiffState,
    action: DiffAction,
    page: usize,
    split: bool,
    out: &mut DiffResponse,
) {
    let total = state.rows.rows.len();
    if total == 0 {
        return;
    }
    let last = total - 1;
    match action {
        DiffAction::Up => state.cursor = state.cursor.saturating_sub(1),
        DiffAction::Down => state.cursor = (state.cursor + 1).min(last),
        DiffAction::PageUp => state.cursor = state.cursor.saturating_sub(page),
        DiffAction::PageDown => state.cursor = state.cursor.saturating_add(page).min(last),
        DiffAction::Home => state.cursor = 0,
        DiffAction::End => state.cursor = last,
        DiffAction::NextHunk => {
            let rows = &state.rows.hunk_rows;
            let next = rows.partition_point(|&row| row <= state.cursor);
            if let Some(&row) = rows.get(next) {
                state.cursor = row;
            }
        }
        DiffAction::PrevHunk => {
            let rows = &state.rows.hunk_rows;
            let prev = rows.partition_point(|&row| row < state.cursor);
            if prev > 0 {
                state.cursor = rows[prev - 1];
            }
        }
        DiffAction::Toggle => {
            if let Some(global) = state.row_hunk(state.cursor) {
                toggle_hunk(state, global as usize, split, out);
            }
        }
        DiffAction::Decide(action) => {
            if let Some(global) = state.row_hunk(state.cursor) {
                let decision = match action {
                    HunkAction::Accept => HunkDecision::Accepted,
                    HunkAction::Reject => HunkDecision::Rejected,
                };
                if state.decide_global(global as usize, decision) {
                    out.hunk_actions
                        .push((state.id_of(global as usize), action));
                }
            }
        }
    }
}

fn render_diff(ctx: &mut Context, state: &mut DiffState, opts: DiffOpts<'_>) -> DiffResponse {
    let focused = ctx.register_focusable();
    let (interaction_id, response) = ctx.begin_widget_interaction(focused);
    let (avail_w, avail_h) = ctx.available_content_size();
    let width = opts.width.unwrap_or(avail_w);
    let height = opts.height.unwrap_or(avail_h);
    let split = opts.layout == DiffLayout::Split && width >= opts.split_min_width;
    state.ensure_rows(split);

    let vh = height as usize;
    let mut out = DiffResponse::default();

    // Clicks map through the previous frame's rect and scroll offset.
    if let Some((rect, clicks)) = ctx.left_clicks_for_interaction(interaction_id) {
        let clicks: Vec<(usize, u32)> = clicks
            .into_iter()
            .map(|(index, mouse)| (index, mouse.y.saturating_sub(rect.y)))
            .collect();
        let scroll = state.scroll;
        let mut consumed = Vec::with_capacity(clicks.len());
        for (index, dy) in clicks {
            consumed.push(index);
            let row = scroll.saturating_add(dy as usize);
            let Some(&kind) = state.rows.rows.get(row) else {
                continue;
            };
            state.cursor = row;
            if let DiffRow::Hunk(global) = kind {
                toggle_hunk(state, global as usize, split, &mut out);
            }
        }
        ctx.consume_indices(consumed);
    }

    if focused {
        let presses: Vec<(usize, KeyCode)> = ctx
            .available_key_presses()
            .filter(|(_, key)| !has_global_shortcut_modifier(key.modifiers))
            .map(|(index, key)| (index, key.code.clone()))
            .collect();
        let mut consumed = Vec::new();
        for (index, code) in presses {
            if let Some(action) = classify_key(state, &code) {
                apply_action(state, action, vh.max(1), split, &mut out);
                consumed.push(index);
            }
        }
        ctx.consume_indices(consumed);
    }

    // Keep the cursor visible, then let the wheel move the viewport.
    let total = state.rows.rows.len();
    state.cursor = state.cursor.min(total.saturating_sub(1));
    if vh > 0 {
        if state.cursor < state.scroll {
            state.scroll = state.cursor;
        } else if state.cursor >= state.scroll + vh {
            state.scroll = state.cursor + 1 - vh;
        }
    }
    let max_scroll = total.saturating_sub(vh);
    state.scroll = state.scroll.min(max_scroll);
    if response.scroll_delta != 0 && vh > 0 && total > 0 {
        let delta = (response.scroll_delta.unsigned_abs() as usize).saturating_mul(WHEEL_ROWS);
        state.scroll = if response.scroll_delta > 0 {
            state.scroll.saturating_sub(delta)
        } else {
            state.scroll.saturating_add(delta).min(max_scroll)
        };
        let bottom = (state.scroll + vh - 1).min(total - 1);
        state.cursor = state.cursor.clamp(state.scroll, bottom);
    }

    let theme = ctx.theme;
    reset_highlight_if_stale(state, opts.lang, &theme);
    let palette = DiffPalette::new(&theme, opts.colors, focused);

    ctx.commands
        .push(Command::BeginContainer(BeginContainerArgs {
            direction: Direction::Column,
            gap: 0,
            align: Align::Start,
            align_self: None,
            justify: Justify::Start,
            border: None,
            border_sides: BorderSides::all(),
            border_style: Style::new().fg(theme.border),
            bg_color: None,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default().w(width).h(height),
            title: None,
            grow: 0,
            group_name: None,
        }));

    let row_opts = RowOpts {
        width: width as usize,
        tab_width: opts.tab_width.max(1),
        line_numbers: opts.line_numbers,
    };
    if total == 0 {
        let mut row = RowText::new(row_opts.width, row_opts.tab_width);
        row.push("No changes", Style::new().fg(theme.text_dim).dim());
        push_row(ctx, row.segs);
    } else {
        let end = state.scroll.saturating_add(vh).min(total);
        for row in state.scroll..end {
            if let Some(global) = state.row_hunk(row) {
                ensure_highlight(state, global as usize, opts.lang, &theme);
            }
            let is_cursor = focused && row == state.cursor;
            let segs = render_row(state, row, is_cursor, &palette, row_opts);
            push_row(ctx, segs);
        }
    }

    ctx.commands.push(Command::EndContainer);
    ctx.rollback.last_text_idx = None;

    out.focused_hunk = state
        .row_hunk(state.cursor)
        .map(|global| state.id_of(global as usize));
    out.response = response;
    out.response.changed = !out.hunk_actions.is_empty() || !out.toggled.is_empty();
    out
}

fn push_row(ctx: &mut Context, segments: Vec<(String, Style)>) {
    ctx.commands.push(Command::RichText {
        segments,
        wrap: false,
        align: Align::Start,
        margin: Margin::default(),
        constraints: Constraints::default(),
    });
}

// ── syntax highlighting ──────────────────────────────────────────────────

fn reset_highlight_if_stale(state: &mut DiffState, lang: Option<&str>, theme: &Theme) {
    let current = state
        .highlight
        .key
        .as_ref()
        .map(|(lang, text, syntax)| (lang.as_deref(), *text, *syntax));
    if current != Some((lang, theme.text, theme.syntax))
        || state.highlight.hunks.len() != state.hunk_count()
    {
        state.highlight.key = Some((lang.map(str::to_string), theme.text, theme.syntax));
        state.highlight.hunks = vec![None; state.hunk_count()];
    }
}

/// Highlight the reconstructed old / new text of one hunk on first use.
fn ensure_highlight(state: &mut DiffState, global: usize, lang: Option<&str>, theme: &Theme) {
    if state
        .highlight
        .hunks
        .get(global)
        .is_none_or(Option::is_some)
    {
        return;
    }
    let file = state.id_of(global).file;
    let lang = lang.unwrap_or_else(|| state.files()[file].lang_hint());
    let mut highlight = HunkHighlight::default();
    if crate::syntax::is_language_supported(lang) {
        let hunk = state.hunk_global(global);
        let side = |keep: DiffLineKind| {
            let mut text = String::new();
            for line in hunk
                .lines
                .iter()
                .filter(|line| line.kind == DiffLineKind::Context || line.kind == keep)
            {
                text.push_str(&line.text);
                text.push('\n');
            }
            text
        };
        let old = side(DiffLineKind::Removed);
        let new = side(DiffLineKind::Added);
        highlight.old = crate::syntax::highlight_code_cached(&old, lang, theme);
        highlight.new = crate::syntax::highlight_code_cached(&new, lang, theme);
    }
    state.highlight.hunks[global] = Some(highlight);
}

/// Highlighted segments for `line`, if the hunk was highlighted.
fn line_segments<'s>(
    state: &'s DiffState,
    global: usize,
    line: &DiffLine,
    old_side: bool,
) -> Option<&'s [(String, Style)]> {
    let highlight = state.highlight.hunks.get(global)?.as_ref()?;
    let hunk = state.hunk_global(global);
    let (lines, number, start) = if old_side {
        (highlight.old.as_ref()?, line.old_line?, hunk.old_start)
    } else {
        (highlight.new.as_ref()?, line.new_line?, hunk.new_start)
    };
    let index = number.checked_sub(start)? as usize;
    lines.get(index).map(Vec::as_slice)
}

// ── row rendering ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
struct RowOpts {
    width: usize,
    tab_width: usize,
    line_numbers: bool,
}

/// Resolved colors for one frame.
struct DiffPalette {
    text: Style,
    dim: Style,
    added_bg: Color,
    removed_bg: Color,
    added_sign: Style,
    removed_sign: Style,
    header_bg: Color,
    hunk: Style,
    hunk_focused: Style,
    file: Style,
    filler: Style,
    separator: Style,
    cursor: Style,
    status_added: Style,
    status_removed: Style,
    status_other: Style,
    status_modified: Style,
}

impl DiffPalette {
    fn new(theme: &Theme, colors: DiffColors, focused: bool) -> Self {
        let added_bg = colors
            .added_bg
            .unwrap_or_else(|| theme.success.blend_f64(theme.surface, LINE_BG_ALPHA));
        let removed_bg = colors
            .removed_bg
            .unwrap_or_else(|| theme.error.blend_f64(theme.surface, LINE_BG_ALPHA));
        let hunk_fg = colors.hunk_fg.unwrap_or(theme.text_dim);
        let header_bg = theme.surface;
        let hunk = Style::new().fg(hunk_fg).bg(header_bg);
        Self {
            text: Style::new().fg(theme.text),
            dim: Style::new().fg(theme.text_dim),
            added_bg,
            removed_bg,
            added_sign: Style::new().fg(theme.success).bold(),
            removed_sign: Style::new().fg(theme.error).bold(),
            header_bg,
            hunk,
            hunk_focused: if focused {
                Style::new().fg(theme.primary).bg(header_bg).bold()
            } else {
                hunk
            },
            file: Style::new().fg(theme.text).bg(header_bg).bold(),
            filler: Style::new().bg(theme.surface),
            separator: Style::new().fg(theme.border),
            cursor: Style::new().fg(theme.selected_fg).bg(theme.selected_bg),
            status_added: Style::new().fg(theme.success).bg(header_bg).bold(),
            status_removed: Style::new().fg(theme.error).bg(header_bg).bold(),
            status_other: Style::new().fg(theme.warning).bg(header_bg).bold(),
            status_modified: Style::new().fg(theme.primary).bg(header_bg).bold(),
        }
    }
}

/// Builds one row's segments, truncating at grapheme boundaries and
/// expanding tabs, so the row never exceeds `width` cells.
struct RowText {
    segs: Vec<(String, Style)>,
    col: usize,
    width: usize,
    tab_width: usize,
    /// Column tab stops are measured from (start of the line content).
    origin: usize,
    full: bool,
}

impl RowText {
    fn new(width: usize, tab_width: usize) -> Self {
        Self {
            segs: Vec::new(),
            col: 0,
            width,
            tab_width,
            origin: 0,
            full: false,
        }
    }

    fn mark_origin(&mut self) {
        self.origin = self.col;
    }

    fn push(&mut self, text: &str, style: Style) {
        if self.full || text.is_empty() {
            return;
        }
        let mut buf = String::new();
        for grapheme in text.graphemes(true) {
            if grapheme == "\t" {
                let spaces = self.tab_width - (self.col - self.origin) % self.tab_width;
                for _ in 0..spaces {
                    if self.col >= self.width {
                        self.full = true;
                        break;
                    }
                    buf.push(' ');
                    self.col += 1;
                }
                if self.full {
                    break;
                }
                continue;
            }
            let symbol = crate::cell::normalize_cell_symbol(grapheme);
            let cells = crate::width::unicode_width(symbol.as_str());
            if self.col + cells > self.width {
                self.full = true;
                break;
            }
            buf.push_str(&symbol);
            self.col += cells;
        }
        if !buf.is_empty() {
            self.segs.push((buf, style));
        }
    }

    /// Pad with spaces in `style` up to the full width.
    fn fill(&mut self, style: Style) {
        let pad = self.width.saturating_sub(self.col);
        if pad > 0 {
            self.segs.push((" ".repeat(pad), style));
            self.col = self.width;
        }
    }
}

fn with_bg(style: Style, bg: Option<Color>) -> Style {
    match bg {
        Some(color) => style.bg(color),
        None => style,
    }
}

fn render_row(
    state: &DiffState,
    row: usize,
    is_cursor: bool,
    palette: &DiffPalette,
    opts: RowOpts,
) -> Vec<(String, Style)> {
    let mut out = RowText::new(opts.width, opts.tab_width);
    match state.rows.rows[row] {
        DiffRow::File(file) => {
            render_file_header(state, file as usize, is_cursor, palette, &mut out)
        }
        DiffRow::Binary(_) => {
            out.push("  binary file not shown", palette.dim.dim());
        }
        DiffRow::Hunk(global) => {
            render_hunk_header(state, global as usize, is_cursor, palette, &mut out)
        }
        DiffRow::Line { hunk, line } => {
            let global = hunk as usize;
            let line = &state.hunk_global(global).lines[line as usize];
            let digits = state.line_no_width();
            let gutter = opts.line_numbers.then(|| {
                format!(
                    "{:>digits$} {:>digits$} │",
                    number_or_blank(line.old_line),
                    number_or_blank(line.new_line)
                )
            });
            render_side(
                state,
                global,
                line,
                line.kind == DiffLineKind::Removed,
                gutter.as_deref(),
                is_cursor,
                palette,
                &mut out,
            );
        }
        DiffRow::Pair { hunk, old, new } => {
            let global = hunk as usize;
            let lines = &state.hunk_global(global).lines;
            let left_w = opts.width.saturating_sub(1) / 2;
            let right_w = opts.width.saturating_sub(1 + left_w);
            let digits = state.line_no_width();
            let mut left = RowText::new(left_w, opts.tab_width);
            let mut right = RowText::new(right_w, opts.tab_width);
            for (half, index, old_side, cursor) in [
                (&mut left, old, true, is_cursor),
                (&mut right, new, false, false),
            ] {
                match index.and_then(|i| lines.get(i as usize)) {
                    Some(line) => {
                        let number = if old_side {
                            line.old_line
                        } else {
                            line.new_line
                        };
                        let gutter = opts
                            .line_numbers
                            .then(|| format!("{:>digits$} │", number_or_blank(number)));
                        render_side(
                            state,
                            global,
                            line,
                            old_side,
                            gutter.as_deref(),
                            cursor,
                            palette,
                            half,
                        );
                    }
                    None => {
                        if cursor {
                            half.push(" ", palette.cursor);
                        }
                        half.fill(palette.filler);
                    }
                }
            }
            out.segs = left.segs;
            if opts.width > 0 {
                out.segs.push(("│".to_string(), palette.separator));
            }
            out.segs.extend(right.segs);
            return out.segs;
        }
    }
    out.segs
}

fn number_or_blank(number: Option<u32>) -> String {
    number.map(|n| n.to_string()).unwrap_or_default()
}

/// One line (unified row, or one half of a split row).
#[expect(
    clippy::too_many_arguments,
    reason = "one diff row: state, line indices, gutter widths and per-side styles"
)]
fn render_side(
    state: &DiffState,
    global: usize,
    line: &DiffLine,
    old_side: bool,
    gutter: Option<&str>,
    is_cursor: bool,
    palette: &DiffPalette,
    out: &mut RowText,
) {
    let (bg, sign, sign_style) = match line.kind {
        DiffLineKind::Added => (Some(palette.added_bg), "+ ", palette.added_sign),
        DiffLineKind::Removed => (Some(palette.removed_bg), "- ", palette.removed_sign),
        DiffLineKind::Context => (None, "  ", palette.text),
    };
    let gutter_style = if is_cursor {
        palette.cursor
    } else {
        with_bg(palette.dim, bg)
    };
    match gutter {
        Some(gutter) => out.push(gutter, gutter_style),
        None if is_cursor => out.push("▌", palette.cursor),
        None => {}
    }
    out.push(sign, with_bg(sign_style, bg));
    out.mark_origin();
    match line_segments(state, global, line, old_side) {
        Some(segments) => {
            for (text, style) in segments {
                out.push(text, with_bg(*style, bg));
            }
        }
        None => out.push(&line.text, with_bg(palette.text, bg)),
    }
    if line.no_newline {
        out.push(" (no newline)", with_bg(palette.dim, bg));
    }
    out.fill(with_bg(Style::new(), bg));
}

fn render_hunk_header(
    state: &DiffState,
    global: usize,
    is_cursor: bool,
    palette: &DiffPalette,
    out: &mut RowText,
) {
    let hunk = state.hunk_global(global);
    let collapsed = state.collapsed_global(global);
    let focused_hunk = state.row_hunk(state.cursor) == Some(global as u32);
    let style = if focused_hunk {
        palette.hunk_focused
    } else {
        palette.hunk
    };
    out.push(
        if collapsed { "▸ " } else { "▾ " },
        if is_cursor { palette.cursor } else { style },
    );
    let header = format!(
        "@@ -{},{} +{},{} @@",
        hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count
    );
    out.push(&header, style);
    if !hunk.section.is_empty() {
        out.push(" ", style);
        out.push(&hunk.section, style);
    }
    if collapsed {
        out.push(&format!(" ({} lines)", hunk.lines.len()), style);
    }
    match state.decision_global(global) {
        HunkDecision::Accepted => out.push("  ✓ accepted", palette.status_added),
        HunkDecision::Rejected => out.push("  ✗ rejected", palette.status_removed),
        HunkDecision::Pending => {}
    }
    out.fill(Style::new().bg(palette.header_bg));
}

fn render_file_header(
    state: &DiffState,
    file_idx: usize,
    is_cursor: bool,
    palette: &DiffPalette,
    out: &mut RowText,
) {
    let file = &state.files()[file_idx];
    let (letter, letter_style) = match file.status {
        DiffFileStatus::Added => ("A ", palette.status_added),
        DiffFileStatus::Deleted => ("D ", palette.status_removed),
        DiffFileStatus::Renamed => ("R ", palette.status_other),
        DiffFileStatus::Copied => ("C ", palette.status_other),
        _ => ("M ", palette.status_modified),
    };
    out.push(
        letter,
        if is_cursor {
            palette.cursor
        } else {
            letter_style
        },
    );
    match file.status {
        DiffFileStatus::Renamed | DiffFileStatus::Copied => {
            out.push(file.old_path.as_deref().unwrap_or(""), palette.file);
            out.push(" → ", palette.file);
            out.push(file.new_path.as_deref().unwrap_or(""), palette.file);
        }
        _ => out.push(file.path(), palette.file),
    }
    let (added, removed) = state.file_stats(file_idx);
    if file.binary {
        out.push("  binary", palette.status_other);
    }
    if added > 0 {
        out.push(&format!("  +{added}"), palette.status_added);
    }
    if removed > 0 {
        out.push(&format!(" -{removed}"), palette.status_removed);
    }
    out.fill(Style::new().bg(palette.header_bg));
}

#[cfg(test)]
mod tests;
