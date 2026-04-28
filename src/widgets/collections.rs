/// State for a selectable list widget.
///
/// Pass a mutable reference to `Context::list` each frame. Up/Down arrow
/// keys (and `k`/`j`) move the selection when the widget is focused.
#[derive(Debug, Clone, Default)]
pub struct ListState {
    /// The list items as display strings.
    pub items: Vec<String>,
    /// Index of the currently selected item.
    pub selected: usize,
    /// Case-insensitive substring filter applied to list items.
    pub filter: String,
    view_indices: Vec<usize>,
    /// Lowercase cache parallel to `items`, rebuilt only on `set_items` / `new`.
    /// Mirrors the `row_search_cache` pattern in `TableState`.
    item_search_cache: Vec<String>,
}

impl ListState {
    /// Create a list with the given items. The first item is selected initially.
    pub fn new(items: Vec<impl Into<String>>) -> Self {
        let items: Vec<String> = items.into_iter().map(Into::into).collect();
        let item_search_cache: Vec<String> =
            items.iter().map(|s| s.to_lowercase()).collect();
        let len = items.len();
        Self {
            items,
            selected: 0,
            filter: String::new(),
            view_indices: (0..len).collect(),
            item_search_cache,
        }
    }

    /// Replace the list items and rebuild the view index.
    ///
    /// Use this instead of assigning `items` directly to ensure the internal
    /// filter/view state stays consistent.
    pub fn set_items(&mut self, items: Vec<impl Into<String>>) {
        self.items = items.into_iter().map(Into::into).collect();
        self.item_search_cache = self.items.iter().map(|s| s.to_lowercase()).collect();
        self.selected = self.selected.min(self.items.len().saturating_sub(1));
        self.rebuild_view();
    }

    /// Set the filter string. Multiple space-separated tokens are AND'd
    /// together — all tokens must match across any cell in the same row.
    /// Empty string disables filtering.
    pub fn set_filter(&mut self, filter: impl Into<String>) {
        self.filter = filter.into();
        self.rebuild_view();
    }

    /// Returns indices of items visible after filtering.
    pub fn visible_indices(&self) -> &[usize] {
        &self.view_indices
    }

    /// Get the currently selected item text, or `None` if the list is empty.
    pub fn selected_item(&self) -> Option<&str> {
        let data_idx = *self.view_indices.get(self.selected)?;
        self.items.get(data_idx).map(String::as_str)
    }

    fn rebuild_view(&mut self) {
        let tokens: Vec<String> = self
            .filter
            .split_whitespace()
            .map(|t| t.to_lowercase())
            .collect();
        self.view_indices = if tokens.is_empty() {
            (0..self.items.len()).collect()
        } else {
            (0..self.items.len())
                .filter(|&i| {
                    let cached = match self.item_search_cache.get(i) {
                        Some(s) => s.as_str(),
                        None => return false,
                    };
                    tokens.iter().all(|token| cached.contains(token.as_str()))
                })
                .collect()
        };
        if !self.view_indices.is_empty() && self.selected >= self.view_indices.len() {
            self.selected = self.view_indices.len() - 1;
        }
    }
}

/// State for a file picker widget.
///
/// Tracks the current directory listing, filtering options, and selected file.
#[derive(Debug, Clone)]
pub struct FilePickerState {
    /// Current directory being browsed.
    pub current_dir: PathBuf,
    /// Visible entries in the current directory.
    pub entries: Vec<FileEntry>,
    /// Selected entry index in `entries`.
    pub selected: usize,
    /// Currently selected file path, if any.
    pub selected_file: Option<PathBuf>,
    /// Whether dotfiles are included in the listing.
    pub show_hidden: bool,
    /// Allowed file extensions (lowercase, no leading dot).
    pub extensions: Vec<String>,
    /// Whether the directory listing needs refresh.
    pub dirty: bool,
}

/// A directory entry shown by [`FilePickerState`].
#[derive(Debug, Clone, Default)]
pub struct FileEntry {
    /// File or directory name.
    pub name: String,
    /// Full path to the entry.
    pub path: PathBuf,
    /// Whether this entry is a directory.
    pub is_dir: bool,
    /// File size in bytes (0 for directories).
    pub size: u64,
}

impl FilePickerState {
    /// Create a file picker rooted at `dir`.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            current_dir: dir.into(),
            entries: Vec::new(),
            selected: 0,
            selected_file: None,
            show_hidden: false,
            extensions: Vec::new(),
            dirty: true,
        }
    }

    /// Configure whether hidden files should be shown.
    pub fn show_hidden(mut self, show: bool) -> Self {
        self.show_hidden = show;
        self.dirty = true;
        self
    }

    /// Restrict visible files to the provided extensions.
    pub fn extensions(mut self, exts: &[&str]) -> Self {
        self.extensions = exts
            .iter()
            .map(|ext| ext.trim().trim_start_matches('.').to_ascii_lowercase())
            .filter(|ext| !ext.is_empty())
            .collect();
        self.dirty = true;
        self
    }

    /// Return the currently selected file path.
    pub fn selected(&self) -> Option<&PathBuf> {
        self.selected_file.as_ref()
    }

    /// Re-scan the current directory and rebuild entries.
    pub fn refresh(&mut self) {
        let mut entries = Vec::new();

        if let Ok(read_dir) = fs::read_dir(&self.current_dir) {
            for dir_entry in read_dir.flatten() {
                let name = dir_entry.file_name().to_string_lossy().to_string();
                if !self.show_hidden && name.starts_with('.') {
                    continue;
                }

                let Ok(file_type) = dir_entry.file_type() else {
                    continue;
                };
                if file_type.is_symlink() {
                    continue;
                }

                let path = dir_entry.path();
                let is_dir = file_type.is_dir();

                if !is_dir && !self.extensions.is_empty() {
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|e| e.to_ascii_lowercase());
                    let Some(ext) = ext else {
                        continue;
                    };
                    if !self.extensions.iter().any(|allowed| allowed == &ext) {
                        continue;
                    }
                }

                let size = if is_dir {
                    0
                } else {
                    fs::symlink_metadata(&path).map(|m| m.len()).unwrap_or(0)
                };

                entries.push(FileEntry {
                    name,
                    path,
                    is_dir,
                    size,
                });
            }
        }

        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a
                .name
                .to_ascii_lowercase()
                .cmp(&b.name.to_ascii_lowercase())
                .then_with(|| a.name.cmp(&b.name)),
        });

        self.entries = entries;
        if self.entries.is_empty() {
            self.selected = 0;
        } else {
            self.selected = self.selected.min(self.entries.len().saturating_sub(1));
        }
        self.dirty = false;
    }
}

impl Default for FilePickerState {
    fn default() -> Self {
        Self::new(".")
    }
}

/// State for a tab navigation widget.
///
/// Pass a mutable reference to `Context::tabs` each frame. Left/Right arrow
/// keys cycle through tabs when the widget is focused.
#[derive(Debug, Clone, Default)]
pub struct TabsState {
    /// The tab labels displayed in the bar.
    pub labels: Vec<String>,
    /// Index of the currently active tab.
    pub selected: usize,
}

impl TabsState {
    /// Create tabs with the given labels. The first tab is active initially.
    pub fn new(labels: Vec<impl Into<String>>) -> Self {
        Self {
            labels: labels.into_iter().map(Into::into).collect(),
            selected: 0,
        }
    }

    /// Get the currently selected tab label, or `None` if there are no tabs.
    pub fn selected_label(&self) -> Option<&str> {
        self.labels.get(self.selected).map(String::as_str)
    }
}

/// State for a data table widget.
///
/// Pass a mutable reference to `Context::table` each frame. Up/Down arrow
/// keys move the row selection when the widget is focused. Column widths are
/// computed automatically from header and cell content.
#[derive(Debug, Clone)]
pub struct TableState {
    /// Column header labels.
    pub headers: Vec<String>,
    /// Table rows, each a `Vec` of cell strings.
    pub rows: Vec<Vec<String>>,
    /// Index of the currently selected row.
    pub selected: usize,
    column_widths: Vec<u32>,
    widths_dirty: bool,
    /// Sorted column index (`None` means no sorting).
    pub sort_column: Option<usize>,
    /// Sort direction (`true` for ascending).
    pub sort_ascending: bool,
    /// Case-insensitive substring filter applied across all cells.
    pub filter: String,
    /// Current page (0-based) when pagination is enabled.
    pub page: usize,
    /// Rows per page (`0` disables pagination).
    pub page_size: usize,
    /// Whether alternating row backgrounds are enabled.
    pub zebra: bool,
    view_indices: Vec<usize>,
    row_search_cache: Vec<String>,
    filter_tokens: Vec<String>,
}

impl Default for TableState {
    fn default() -> Self {
        Self {
            headers: Vec::new(),
            rows: Vec::new(),
            selected: 0,
            column_widths: Vec::new(),
            widths_dirty: true,
            sort_column: None,
            sort_ascending: true,
            filter: String::new(),
            page: 0,
            page_size: 0,
            zebra: false,
            view_indices: Vec::new(),
            row_search_cache: Vec::new(),
            filter_tokens: Vec::new(),
        }
    }
}

impl TableState {
    /// Create a table with headers and rows. Column widths are computed immediately.
    pub fn new(headers: Vec<impl Into<String>>, rows: Vec<Vec<impl Into<String>>>) -> Self {
        let headers: Vec<String> = headers.into_iter().map(Into::into).collect();
        let rows: Vec<Vec<String>> = rows
            .into_iter()
            .map(|r| r.into_iter().map(Into::into).collect())
            .collect();
        let mut state = Self {
            headers,
            rows,
            selected: 0,
            column_widths: Vec::new(),
            widths_dirty: true,
            sort_column: None,
            sort_ascending: true,
            filter: String::new(),
            page: 0,
            page_size: 0,
            zebra: false,
            view_indices: Vec::new(),
            row_search_cache: Vec::new(),
            filter_tokens: Vec::new(),
        };
        state.rebuild_row_search_cache();
        state.rebuild_view();
        state.recompute_widths();
        state
    }

    /// Replace all rows, preserving the selection index if possible.
    ///
    /// If the current selection is beyond the new row count, it is clamped to
    /// the last row.
    pub fn set_rows(&mut self, rows: Vec<Vec<impl Into<String>>>) {
        self.rows = rows
            .into_iter()
            .map(|r| r.into_iter().map(Into::into).collect())
            .collect();
        self.rebuild_row_search_cache();
        self.rebuild_view();
    }

    /// Sort by a specific column index. If already sorted by this column, toggles direction.
    pub fn toggle_sort(&mut self, column: usize) {
        if self.sort_column == Some(column) {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_column = Some(column);
            self.sort_ascending = true;
        }
        self.rebuild_view();
    }

    /// Sort by column without toggling (always sets to ascending first).
    pub fn sort_by(&mut self, column: usize) {
        if self.sort_column == Some(column) && self.sort_ascending {
            return;
        }
        self.sort_column = Some(column);
        self.sort_ascending = true;
        self.rebuild_view();
    }

    /// Set the filter string. Multiple space-separated tokens are AND'd
    /// together — all tokens must match across any cell in the same row.
    /// Empty string disables filtering.
    pub fn set_filter(&mut self, filter: impl Into<String>) {
        let filter = filter.into();
        if self.filter == filter {
            return;
        }
        self.filter = filter;
        self.filter_tokens = Self::tokenize_filter(&self.filter);
        self.page = 0;
        self.rebuild_view();
    }

    /// Clear sorting.
    pub fn clear_sort(&mut self) {
        if self.sort_column.is_none() && self.sort_ascending {
            return;
        }
        self.sort_column = None;
        self.sort_ascending = true;
        self.rebuild_view();
    }

    /// Move to the next page. Does nothing if already on the last page.
    pub fn next_page(&mut self) {
        if self.page_size == 0 {
            return;
        }
        let last_page = self.total_pages().saturating_sub(1);
        self.page = (self.page + 1).min(last_page);
    }

    /// Move to the previous page. Does nothing if already on page 0.
    pub fn prev_page(&mut self) {
        self.page = self.page.saturating_sub(1);
    }

    /// Total number of pages based on filtered rows and page_size. Returns 1 if page_size is 0.
    pub fn total_pages(&self) -> usize {
        if self.page_size == 0 {
            return 1;
        }

        let len = self.view_indices.len();
        if len == 0 {
            1
        } else {
            len.div_ceil(self.page_size)
        }
    }

    /// Get the visible row indices after filtering and sorting (used internally by table()).
    pub fn visible_indices(&self) -> &[usize] {
        &self.view_indices
    }

    /// Get the currently selected row data, or `None` if the table is empty.
    pub fn selected_row(&self) -> Option<&[String]> {
        if self.view_indices.is_empty() {
            return None;
        }
        let data_idx = self.view_indices.get(self.selected)?;
        self.rows.get(*data_idx).map(|r| r.as_slice())
    }

    /// Recompute view_indices based on current sort + filter settings.
    fn rebuild_view(&mut self) {
        let mut indices: Vec<usize> = (0..self.rows.len()).collect();

        if !self.filter_tokens.is_empty() {
            indices.retain(|&idx| {
                let searchable = match self.row_search_cache.get(idx) {
                    Some(row) => row,
                    None => return false,
                };
                self.filter_tokens
                    .iter()
                    .all(|token| searchable.contains(token.as_str()))
            });
        }

        if let Some(column) = self.sort_column {
            indices.sort_by(|a, b| {
                let left = self
                    .rows
                    .get(*a)
                    .and_then(|row| row.get(column))
                    .map(String::as_str)
                    .unwrap_or("");
                let right = self
                    .rows
                    .get(*b)
                    .and_then(|row| row.get(column))
                    .map(String::as_str)
                    .unwrap_or("");

                match (left.parse::<f64>(), right.parse::<f64>()) {
                    (Ok(l), Ok(r)) => l.partial_cmp(&r).unwrap_or(std::cmp::Ordering::Equal),
                    _ => left
                        .chars()
                        .flat_map(char::to_lowercase)
                        .cmp(right.chars().flat_map(char::to_lowercase)),
                }
            });

            if !self.sort_ascending {
                indices.reverse();
            }
        }

        self.view_indices = indices;

        if self.page_size > 0 {
            self.page = self.page.min(self.total_pages().saturating_sub(1));
        } else {
            self.page = 0;
        }

        self.selected = self.selected.min(self.view_indices.len().saturating_sub(1));
        self.widths_dirty = true;
    }

    fn rebuild_row_search_cache(&mut self) {
        self.row_search_cache = self
            .rows
            .iter()
            .map(|row| {
                let mut searchable = String::new();
                for (idx, cell) in row.iter().enumerate() {
                    if idx > 0 {
                        searchable.push('\n');
                    }
                    searchable.extend(cell.chars().flat_map(char::to_lowercase));
                }
                searchable
            })
            .collect();
        self.filter_tokens = Self::tokenize_filter(&self.filter);
        self.widths_dirty = true;
    }

    fn tokenize_filter(filter: &str) -> Vec<String> {
        filter
            .split_whitespace()
            .map(|t| t.to_lowercase())
            .collect()
    }

    pub(crate) fn recompute_widths(&mut self) {
        // Skip when no mutation since the last computation. `widths_dirty` is
        // set by `rebuild_view` (covers `set_rows`, `set_filter`, sort) and at
        // construction. Frames without data mutation become a no-op.
        if !self.widths_dirty {
            return;
        }
        let col_count = self.headers.len();
        self.column_widths = vec![0u32; col_count];
        for (i, header) in self.headers.iter().enumerate() {
            let mut width = UnicodeWidthStr::width(header.as_str()) as u32;
            if self.sort_column == Some(i) {
                width += 2;
            }
            self.column_widths[i] = width;
        }
        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < col_count {
                    let w = UnicodeWidthStr::width(cell.as_str()) as u32;
                    self.column_widths[i] = self.column_widths[i].max(w);
                }
            }
        }
        self.widths_dirty = false;
    }

    pub(crate) fn column_widths(&self) -> &[u32] {
        &self.column_widths
    }

    pub(crate) fn is_dirty(&self) -> bool {
        self.widths_dirty
    }
}

/// A highlighted line range within a scrollable region.
///
/// Used with [`ScrollState::set_highlights`] to mark search results, error
/// lines, or any per-line emphasis. The `scrollable_with_gutter` widget reads
/// the active highlights and renders a background band on matching lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HighlightRange {
    /// First line (0-based, relative to content top).
    pub start_line: usize,
    /// Number of lines in the range (1 = single line).
    pub line_count: usize,
}

impl HighlightRange {
    /// Create a single-line highlight at `line`.
    pub fn line(line: usize) -> Self {
        Self {
            start_line: line,
            line_count: 1,
        }
    }

    /// Create a multi-line highlight starting at `start_line` covering `line_count` rows.
    pub fn span(start_line: usize, line_count: usize) -> Self {
        Self {
            start_line,
            line_count: line_count.max(1),
        }
    }

    /// Check whether the given absolute line index falls within this range.
    pub fn contains(&self, line: usize) -> bool {
        line >= self.start_line && line < self.start_line + self.line_count
    }
}

/// State for a scrollable container.
///
/// Pass a mutable reference to `Context::scrollable` each frame. The context
/// updates `offset` and the internal bounds automatically based on mouse wheel
/// and drag events.
#[derive(Debug, Clone)]
pub struct ScrollState {
    /// Current vertical scroll offset in rows.
    pub offset: usize,
    content_height: u32,
    viewport_height: u32,
    highlights: Vec<HighlightRange>,
    current_highlight: Option<usize>,
}

impl ScrollState {
    /// Create scroll state starting at offset 0.
    pub fn new() -> Self {
        Self {
            offset: 0,
            content_height: 0,
            viewport_height: 0,
            highlights: Vec::new(),
            current_highlight: None,
        }
    }

    /// Check if scrolling upward is possible (offset is greater than 0).
    pub fn can_scroll_up(&self) -> bool {
        self.offset > 0
    }

    /// Check if scrolling downward is possible (content extends below the viewport).
    pub fn can_scroll_down(&self) -> bool {
        (self.offset as u32) + self.viewport_height < self.content_height
    }

    /// Get the total content height in rows.
    pub fn content_height(&self) -> u32 {
        self.content_height
    }

    /// Get the viewport height in rows.
    pub fn viewport_height(&self) -> u32 {
        self.viewport_height
    }

    /// Get the scroll progress as a ratio in [0.0, 1.0].
    pub fn progress(&self) -> f32 {
        let max = self.content_height.saturating_sub(self.viewport_height);
        if max == 0 {
            0.0
        } else {
            self.offset as f32 / max as f32
        }
    }

    /// Scroll up by the given number of rows, clamped to 0.
    pub fn scroll_up(&mut self, amount: usize) {
        self.offset = self.offset.saturating_sub(amount);
    }

    /// Scroll down by the given number of rows, clamped to the maximum offset.
    pub fn scroll_down(&mut self, amount: usize) {
        let max_offset = self.content_height.saturating_sub(self.viewport_height) as usize;
        self.offset = (self.offset + amount).min(max_offset);
    }

    pub(crate) fn set_bounds(&mut self, content_height: u32, viewport_height: u32) {
        self.content_height = content_height;
        self.viewport_height = viewport_height;
    }

    /// Set the active highlight ranges. Replaces any previous highlights.
    ///
    /// Selecting the first highlight automatically when the list is non-empty
    /// matches the behavior of search-result navigation in code editors.
    pub fn set_highlights(&mut self, ranges: &[HighlightRange]) {
        self.highlights.clear();
        self.highlights.extend_from_slice(ranges);
        self.current_highlight = if self.highlights.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    /// Read-only access to the active highlight ranges.
    pub fn highlights(&self) -> &[HighlightRange] {
        &self.highlights
    }

    /// Index of the currently focused highlight, if any.
    pub fn current_highlight(&self) -> Option<usize> {
        self.current_highlight
    }

    /// Clear all highlights and reset the current index.
    pub fn clear_highlights(&mut self) {
        self.highlights.clear();
        self.current_highlight = None;
    }

    /// Advance to the next highlight, scrolling the viewport to show it.
    /// Wraps from last to first.
    pub fn highlight_next(&mut self) {
        if self.highlights.is_empty() {
            return;
        }
        let next = match self.current_highlight {
            Some(i) => (i + 1) % self.highlights.len(),
            None => 0,
        };
        self.current_highlight = Some(next);
        self.scroll_to_current_highlight();
    }

    /// Move to the previous highlight, scrolling the viewport to show it.
    /// Wraps from first to last.
    pub fn highlight_previous(&mut self) {
        if self.highlights.is_empty() {
            return;
        }
        let next = match self.current_highlight {
            Some(i) => {
                if i == 0 {
                    self.highlights.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.current_highlight = Some(next);
        self.scroll_to_current_highlight();
    }

    /// Scroll the viewport so the currently focused highlight is visible
    /// with one line of context above when possible.
    pub fn scroll_to_current_highlight(&mut self) {
        let Some(idx) = self.current_highlight else {
            return;
        };
        let Some(range) = self.highlights.get(idx).copied() else {
            return;
        };
        let target = range.start_line;
        let viewport = self.viewport_height as usize;
        let content = self.content_height as usize;
        let max_offset = content.saturating_sub(viewport);
        if target < self.offset {
            self.offset = target.saturating_sub(1).min(max_offset);
        } else if viewport > 0 && target >= self.offset + viewport {
            let desired = target + 2;
            let new_offset = desired.saturating_sub(viewport);
            self.offset = new_offset.min(max_offset);
        } else if self.offset > max_offset {
            self.offset = max_offset;
        }
    }
}

impl Default for ScrollState {
    fn default() -> Self {
        Self::new()
    }
}

/// State for a [`Context::split_pane`] / [`Context::vsplit_pane`] container.
///
/// Tracks the split ratio and drag state. Pass a mutable reference each frame
/// — the widget updates `ratio` in place when the user drags the handle or
/// presses arrow keys with the handle focused.
#[derive(Debug, Clone, PartialEq)]
pub struct SplitPaneState {
    /// Fraction of space given to the first pane. Clamped to
    /// `[min_ratio, 1.0 - min_ratio]`.
    pub ratio: f32,
    /// Whether the handle is currently being dragged.
    pub dragging: bool,
    /// Minimum fraction allocated to either pane. Default: `0.10`.
    pub min_ratio: f32,
}

impl SplitPaneState {
    /// Create split state with the given initial ratio (clamped to `[0.05, 0.95]`).
    pub fn new(ratio: f32) -> Self {
        let min_ratio = 0.10;
        let clamped = ratio.clamp(min_ratio, 1.0 - min_ratio);
        Self {
            ratio: clamped,
            dragging: false,
            min_ratio,
        }
    }

    /// Override the minimum ratio for either pane (clamped to `[0.0, 0.49]`).
    pub fn with_min_ratio(mut self, min: f32) -> Self {
        self.min_ratio = min.clamp(0.0, 0.49);
        self.ratio = self.ratio.clamp(self.min_ratio, 1.0 - self.min_ratio);
        self
    }

    /// Set the ratio, clamped to `[min_ratio, 1.0 - min_ratio]`.
    pub fn set_ratio(&mut self, ratio: f32) {
        self.ratio = ratio.clamp(self.min_ratio, 1.0 - self.min_ratio);
    }
}

impl Default for SplitPaneState {
    fn default() -> Self {
        Self::new(0.5)
    }
}

/// Column specification for [`Context::grid_with()`].
///
/// Controls the width allocation of individual columns in a grid layout.
///
/// # Example
///
/// ```no_run
/// use slt::GridColumn;
/// # slt::run(|ui: &mut slt::Context| {
/// ui.grid_with(&[
///     GridColumn::Fixed(8),   // label column: exactly 8 chars
///     GridColumn::Grow(1),    // flexible column
///     GridColumn::Grow(1),    // flexible column
///     GridColumn::Fixed(4),   // status column: exactly 4 chars
/// ], |ui| {
///     // children placed left-to-right, wrapping to next row
/// });
/// # });
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GridColumn {
    /// Equal-width column with grow weight 1 (default `grid()` behavior).
    Auto,
    /// Fixed-width column in character cells. Does not grow or shrink.
    Fixed(u32),
    /// Flexible column with a custom grow weight. Higher values take
    /// proportionally more space.
    Grow(u16),
    /// Column sized as a percentage (1–100) of the grid width.
    Percent(u8),
}
