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
    /// Top *item* index of the visible viewport for `virtual_list`. Defaults to
    /// `0` and is clamped each frame so `selected` stays inside the viewport
    /// without forcing the cursor to the bottom row. For the uniform
    /// fixed-height path this equals the top row; with per-item heights set
    /// (see [`set_item_heights`](ListState::set_item_heights)) the cumulative
    /// row offset is tracked separately in `viewport_row_offset`.
    pub(crate) viewport_offset: usize,
    /// Cumulative top-row offset of the visible viewport for
    /// `virtual_list_variable`. Tracks the total row height of the items above
    /// `viewport_offset` so row-accurate scrolling and edge clipping work when
    /// per-item heights are present. Equals `viewport_offset` only when every
    /// item is one row tall.
    pub(crate) viewport_row_offset: usize,
    /// Optional per-item row heights (each clamped to `>= 1`). When present,
    /// [`Context::virtual_list_variable`](crate::Context::virtual_list_variable)
    /// uses them to compute a row-accurate visible range; when `None` the
    /// uniform one-row-per-item model is used.
    item_heights: Option<Vec<u32>>,
    /// Cached prefix sum of `item_heights`, rebuilt lazily when `heights_dirty`.
    /// `row_prefix[i]` is the total number of rows occupied by items `0..i`, so
    /// `row_prefix.len() == items.len() + 1` after `ensure_row_prefix`.
    row_prefix: Vec<u32>,
    /// Dirty flag gating `row_prefix` rebuilds; set whenever items or heights
    /// change so a stale prefix sum is never consumed.
    heights_dirty: bool,
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
            viewport_offset: 0,
            viewport_row_offset: 0,
            item_heights: None,
            row_prefix: Vec::new(),
            heights_dirty: true,
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
        if let Some(heights) = self.item_heights.as_mut() {
            heights.truncate(self.items.len());
        }
        // Item count changed, so any cached prefix sum is stale.
        self.heights_dirty = true;
        self.rebuild_view();
    }

    /// Provide a per-item row height (each clamped to `>= 1`) and return `self`.
    ///
    /// Enables variable-height virtualization via
    /// [`Context::virtual_list_variable`](crate::Context::virtual_list_variable),
    /// the chat/feed bubble use case where each item occupies a different
    /// number of rows. Each entry corresponds to the item at the same index;
    /// missing entries fall back to a height of `1`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::widgets::ListState;
    ///
    /// let state = ListState::new(vec!["short", "a\nthree\nline bubble", "ok"])
    ///     .with_item_heights(vec![1, 3, 1]);
    /// # let _ = state;
    /// ```
    ///
    /// Available since `0.21.0`.
    pub fn with_item_heights(mut self, heights: Vec<u32>) -> Self {
        self.set_item_heights(heights);
        self
    }

    /// Set per-item row heights (each clamped to `>= 1`).
    ///
    /// Marks the cached prefix sum dirty so it is rebuilt on the next render.
    /// Length should match [`items`](ListState::items); missing entries fall
    /// back to a height of `1` and extra entries are ignored.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::widgets::ListState;
    ///
    /// let mut state = ListState::new(vec!["a", "b", "c"]);
    /// state.set_item_heights(vec![2, 1, 4]);
    /// # let _ = state;
    /// ```
    ///
    /// Available since `0.21.0`.
    pub fn set_item_heights(&mut self, heights: Vec<u32>) {
        self.item_heights = Some(heights.into_iter().map(|h| h.max(1)).collect());
        self.heights_dirty = true;
    }

    /// Clear per-item heights, reverting to the uniform one-row-per-item model.
    ///
    /// After this call [`Context::virtual_list_variable`](crate::Context::virtual_list_variable)
    /// behaves identically to [`Context::virtual_list`](crate::Context::virtual_list).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::widgets::ListState;
    ///
    /// let mut state = ListState::new(vec!["a", "b"]).with_item_heights(vec![3, 2]);
    /// state.clear_item_heights();
    /// # let _ = state;
    /// ```
    ///
    /// Available since `0.21.0`.
    pub fn clear_item_heights(&mut self) {
        self.item_heights = None;
        self.heights_dirty = true;
    }

    /// Whether per-item heights are currently set.
    pub(crate) fn has_item_heights(&self) -> bool {
        self.item_heights.is_some()
    }

    /// Height of item `idx` in rows (`1` when no per-item heights are set or the
    /// index has no explicit height).
    pub(crate) fn item_height(&self, idx: usize) -> u32 {
        self.item_heights
            .as_ref()
            .and_then(|h| h.get(idx).copied())
            .unwrap_or(1)
    }

    /// Rebuild `row_prefix` if dirty. After this call `row_prefix[i]` is the
    /// total number of rows occupied by items `0..i`, and
    /// `row_prefix.len() == items.len() + 1`. Rebuild is `O(n)` and skipped
    /// entirely when `heights_dirty` is `false`.
    pub(crate) fn ensure_row_prefix(&mut self) {
        if !self.heights_dirty && self.row_prefix.len() == self.items.len() + 1 {
            return;
        }
        let n = self.items.len();
        self.row_prefix.clear();
        self.row_prefix.reserve(n + 1);
        let mut acc = 0u32;
        self.row_prefix.push(0);
        for i in 0..n {
            acc = acc.saturating_add(self.item_height(i));
            self.row_prefix.push(acc);
        }
        self.heights_dirty = false;
    }

    /// Read-only access to the cached prefix sum (test/helper use).
    pub(crate) fn row_prefix(&self) -> &[u32] {
        &self.row_prefix
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

    /// Move the item at data index `from` to data index `to`, preserving
    /// selection on the moved item.
    ///
    /// Indices address the underlying [`items`](ListState::items) vector (the
    /// unfiltered order), not the filtered view. Out-of-range indices and a
    /// no-op `from == to` move leave the list untouched and return `false`.
    /// The parallel search cache and any per-item heights are kept in sync, and
    /// the filtered view is rebuilt so `selected` continues to point at the item
    /// that was moved when it remains visible.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::widgets::ListState;
    ///
    /// let mut state = ListState::new(vec!["a", "b", "c"]);
    /// assert!(state.move_item(0, 2));
    /// assert_eq!(state.selected_item(), Some("a"));
    /// ```
    ///
    /// Available since `0.21.1`.
    pub fn move_item(&mut self, from: usize, to: usize) -> bool {
        let len = self.items.len();
        if from >= len || to >= len || from == to {
            return false;
        }

        // Remember which data index is currently selected so selection can
        // follow the moved item (or stay on whatever item the user had).
        let selected_data = self.view_indices.get(self.selected).copied();

        let item = self.items.remove(from);
        self.items.insert(to, item);

        // Keep the lowercase search cache aligned with `items`.
        if from < self.item_search_cache.len() {
            let cached = self.item_search_cache.remove(from);
            self.item_search_cache.insert(to.min(self.item_search_cache.len()), cached);
        }

        // Keep per-item heights aligned with `items` when present.
        if let Some(heights) = self.item_heights.as_mut()
            && from < heights.len()
        {
            let h = heights.remove(from);
            heights.insert(to.min(heights.len()), h);
        }
        self.heights_dirty = true;

        self.rebuild_view();

        // Re-point `selected` at the same data item if it is still visible.
        if let Some(data_idx) = selected_data {
            // The moved item's data index is now `to`; anything that was
            // `selected` shifts with the rotation, so re-derive from data idx.
            let new_data_idx = if data_idx == from {
                to
            } else if from < to && data_idx > from && data_idx <= to {
                data_idx - 1
            } else if to < from && data_idx >= to && data_idx < from {
                data_idx + 1
            } else {
                data_idx
            };
            if let Some(view_pos) = self.view_indices.iter().position(|&i| i == new_data_idx) {
                self.selected = view_pos;
            }
        }

        true
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

/// Response from [`Context::list_reorderable`](crate::Context::list_reorderable).
///
/// Wraps the row-level [`Response`] (selection/hover/rect/focus) and additionally
/// exposes the `(from, to)` data indices of an item that was reordered this frame
/// via the keyboard. Implements `Deref<Target = Response>` so `r.changed`,
/// `r.hovered`, `r.rect`, etc. work directly.
///
/// # Example
///
/// ```no_run
/// # use slt::widgets::ListState;
/// # let mut list = ListState::new(vec!["a", "b", "c"]);
/// # slt::run(move |ui: &mut slt::Context| {
/// let r = ui.list_reorderable(&mut list);
/// if let Some((from, to)) = r.reordered {
///     // persist the new order: item moved from `from` to `to`
///     let _ = (from, to);
/// }
/// # });
/// ```
///
/// Available since `0.21.1`.
#[derive(Debug, Clone, Default)]
#[must_use = "ListResponse contains interaction state — check .reordered, .changed, or .hovered"]
pub struct ListResponse {
    /// The row-level interaction response (selection change, hover, rect, focus).
    pub response: Response,
    /// `(from, to)` data indices of the item moved this frame, if any.
    pub reordered: Option<(usize, usize)>,
}

impl std::ops::Deref for ListResponse {
    type Target = Response;
    fn deref(&self) -> &Response {
        &self.response
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

    /// Return the currently selected file path, if any.
    ///
    /// Disambiguates from the [`selected: usize`](Self::selected) field, which
    /// is the entry index into [`entries`](Self::entries). This method returns
    /// the resolved file path that the user picked via Enter — `None` until a
    /// file (not a directory) is selected.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::widgets::FilePickerState;
    /// # slt::run(|ui: &mut slt::Context| {
    /// let mut state = FilePickerState::new(".");
    /// if ui.file_picker(&mut state).changed {
    ///     if let Some(path) = state.selected_file() {
    ///         println!("picked: {}", path.display());
    ///     }
    /// }
    /// # });
    /// ```
    pub fn selected_file(&self) -> Option<&PathBuf> {
        self.selected_file.as_ref()
    }

    /// Return the currently selected file path.
    ///
    /// Deprecated alias for [`selected_file`](Self::selected_file). The
    /// shorter name conflicts visually with the [`selected: usize`](Self::selected)
    /// field — a getter returning a path alongside a public field returning
    /// an index made call sites ambiguous. Migrate to `selected_file()` for
    /// new code; this stub stays callable until v1.0.
    #[deprecated(since = "0.20.0", note = "use selected_file() — disambiguates from the `selected: usize` field index")]
    pub fn selected(&self) -> Option<&PathBuf> {
        self.selected_file()
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

/// Per-column width policy for a [`TableState`].
///
/// Mirrors the semantics of [`GridColumn`] and
/// [`WidthSpec`](crate::WidthSpec) for the string-grid table model. Apply a
/// slice of these via [`TableState::column_widths_spec`]; columns without an
/// entry (or set to [`TableColumn::Auto`]) keep the default content-derived
/// sizing.
///
/// Available since v0.21.0.
///
/// # Example
///
/// ```no_run
/// use slt::{TableColumn, widgets::TableState};
/// # slt::run(|ui: &mut slt::Context| {
/// let mut table = TableState::new(
///     vec!["Name", "Status"],
///     vec![vec!["build", "ok"]],
/// );
/// // Pin the status column to 6 cells, leave the name column automatic.
/// table.column_widths_spec(&[TableColumn::Auto, TableColumn::Fixed(6)]);
/// ui.table(&mut table);
/// # });
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TableColumn {
    /// Size the column to its content (header + widest cell). Default.
    Auto,
    /// Exact cell width in character cells. Content is padded or truncated to fit.
    Fixed(u32),
    /// Content width, floored at `n` cells (never narrower than `n`).
    Min(u32),
    /// Content width, capped at `n` cells (truncated with an ellipsis if longer).
    Max(u32),
    /// Width as a percentage (`1..=100`) of the available table content width.
    Percent(u8),
}

/// State for a data table widget.
///
/// Pass a mutable reference to `Context::table` each frame. Up/Down arrow
/// keys move the row selection when the widget is focused. Column widths are
/// computed automatically from header and cell content, or constrained per
/// column via [`column_widths_spec`](TableState::column_widths_spec).
///
/// Multi-row selection (Space / Shift+Up/Down / Ctrl+Space and modifier
/// clicks) is tracked in [`multi_selected`](TableState::multi_selected); the
/// `selected` field always remains the focused/cursor row.
#[derive(Debug, Clone)]
pub struct TableState {
    /// Column header labels.
    pub headers: Vec<String>,
    /// Table rows, each a `Vec` of cell strings.
    pub rows: Vec<Vec<String>>,
    /// Focused/cursor row (view index). Unchanged single-select semantics.
    pub selected: usize,
    /// Multi-row selection as view indices. Empty means no multi-selection.
    ///
    /// Available since v0.21.0.
    pub multi_selected: HashSet<usize>,
    /// Range-selection anchor (view index) for Shift extension.
    pub(crate) selection_anchor: Option<usize>,
    /// Per-column width policy. Empty means every column is [`TableColumn::Auto`].
    column_specs: Vec<TableColumn>,
    column_widths: Vec<u32>,
    /// Content-derived widths before per-column specs are resolved.
    content_widths: Vec<u32>,
    widths_dirty: bool,
    /// Available content width used to resolve [`TableColumn::Percent`].
    resolved_width: u32,
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
            multi_selected: HashSet::new(),
            selection_anchor: None,
            column_specs: Vec::new(),
            column_widths: Vec::new(),
            content_widths: Vec::new(),
            widths_dirty: true,
            resolved_width: 0,
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
            multi_selected: HashSet::new(),
            selection_anchor: None,
            column_specs: Vec::new(),
            column_widths: Vec::new(),
            content_widths: Vec::new(),
            widths_dirty: true,
            resolved_width: 0,
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

    /// Set the per-column width policy.
    ///
    /// The slice is index-aligned with [`headers`](TableState::headers); a
    /// shorter slice leaves trailing columns at [`TableColumn::Auto`]. Passing
    /// an empty slice resets every column to automatic sizing.
    ///
    /// Available since v0.21.0.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::{TableColumn, widgets::TableState};
    /// # slt::run(|ui: &mut slt::Context| {
    /// let mut table = TableState::new(
    ///     vec!["Name", "Note"],
    ///     vec![vec!["a", "a very long note that should be capped"]],
    /// );
    /// table.column_widths_spec(&[TableColumn::Fixed(6), TableColumn::Max(10)]);
    /// ui.table(&mut table);
    /// # });
    /// ```
    pub fn column_widths_spec(&mut self, specs: &[TableColumn]) {
        self.column_specs = specs.to_vec();
        self.widths_dirty = true;
    }

    /// Return the multi-selected rows in ascending view order.
    ///
    /// View indices are resolved against the current sort/filter view, so the
    /// returned slices reflect what the user sees. Stale indices (beyond the
    /// current view) are skipped.
    ///
    /// Available since v0.21.0.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::widgets::TableState;
    /// # slt::run(|ui: &mut slt::Context| {
    /// let mut table = TableState::new(
    ///     vec!["Name"],
    ///     vec![vec!["a"], vec!["b"]],
    /// );
    /// ui.table(&mut table);
    /// for row in table.selected_rows() {
    ///     let _ = row;
    /// }
    /// # });
    /// ```
    pub fn selected_rows(&self) -> Vec<&[String]> {
        let mut indices: Vec<usize> = self.multi_selected.iter().copied().collect();
        indices.sort_unstable();
        indices
            .iter()
            .filter_map(|&view_idx| self.view_indices.get(view_idx))
            .filter_map(|&data_idx| self.rows.get(data_idx).map(|r| r.as_slice()))
            .collect()
    }

    /// Returns `true` if the row at `view_idx` is in the multi-selection set.
    ///
    /// Available since v0.21.0.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::widgets::TableState;
    /// # slt::run(|ui: &mut slt::Context| {
    /// let mut table = TableState::new(vec!["Name"], vec![vec!["a"]]);
    /// ui.table(&mut table);
    /// let _ = table.is_row_selected(0);
    /// # });
    /// ```
    pub fn is_row_selected(&self, view_idx: usize) -> bool {
        self.multi_selected.contains(&view_idx)
    }

    /// Clear the multi-selection set and the range anchor.
    ///
    /// The focused [`selected`](TableState::selected) cursor row is unaffected.
    ///
    /// Available since v0.21.0.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::widgets::TableState;
    /// # slt::run(|ui: &mut slt::Context| {
    /// let mut table = TableState::new(vec!["Name"], vec![vec!["a"]]);
    /// ui.table(&mut table);
    /// table.clear_selection();
    /// # });
    /// ```
    pub fn clear_selection(&mut self) {
        self.multi_selected.clear();
        self.selection_anchor = None;
    }

    /// Toggle the multi-selection state for the row at `view_idx`, and set the
    /// range anchor to it. Mirrors [`MultiSelectState::toggle`].
    pub(crate) fn toggle_row(&mut self, view_idx: usize) {
        if self.multi_selected.contains(&view_idx) {
            self.multi_selected.remove(&view_idx);
        } else {
            self.multi_selected.insert(view_idx);
        }
        self.selection_anchor = Some(view_idx);
    }

    /// Replace the multi-selection with the single row at `view_idx` and reset
    /// the anchor to it.
    pub(crate) fn select_single(&mut self, view_idx: usize) {
        self.multi_selected.clear();
        self.multi_selected.insert(view_idx);
        self.selection_anchor = Some(view_idx);
    }

    /// Select the inclusive contiguous range `[min(from,to)..=max(from,to)]`,
    /// replacing the current multi-selection. The anchor is left at `from`.
    pub(crate) fn select_range(&mut self, from: usize, to: usize) {
        let (lo, hi) = if from <= to { (from, to) } else { (to, from) };
        self.multi_selected.clear();
        for idx in lo..=hi {
            self.multi_selected.insert(idx);
        }
        self.selection_anchor = Some(from);
    }

    /// Remove any multi-selection indices that are no longer valid view
    /// indices, and clamp the anchor. Called after the view is rebuilt.
    fn prune_selection(&mut self) {
        let view_len = self.view_indices.len();
        self.multi_selected.retain(|&idx| idx < view_len);
        if let Some(anchor) = self.selection_anchor
            && anchor >= view_len
        {
            self.selection_anchor = None;
        }
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
        self.prune_selection();
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
        // set by `rebuild_view` (covers `set_rows`, `set_filter`, sort),
        // `column_widths_spec`, and at construction. Frames without data
        // mutation become a no-op.
        if !self.widths_dirty {
            return;
        }
        let col_count = self.headers.len();
        self.content_widths = vec![0u32; col_count];
        for (i, header) in self.headers.iter().enumerate() {
            let mut width = UnicodeWidthStr::width(header.as_str()) as u32;
            if self.sort_column == Some(i) {
                width += 2;
            }
            self.content_widths[i] = width;
        }
        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < col_count {
                    let w = UnicodeWidthStr::width(cell.as_str()) as u32;
                    self.content_widths[i] = self.content_widths[i].max(w);
                }
            }
        }
        // Default resolved widths to the content widths; `resolve_column_widths`
        // overlays the per-column specs each frame once the available width is
        // known. When no spec is set this is the pre-v0.21 behavior verbatim.
        self.column_widths = self.content_widths.clone();
        self.widths_dirty = false;
    }

    /// Resolve per-column width specs against the content widths, using
    /// `available` as the total table content width for `Percent`. A no-op
    /// when no spec is set, so all-`Auto` tables render byte-identically.
    pub(crate) fn resolve_column_widths(&mut self, available: u32) {
        if self.column_specs.is_empty() {
            return;
        }
        // Re-derive base content widths if the available width changed since
        // the last resolution (the previous frame may have shrunk a column).
        if self.resolved_width != available {
            self.column_widths = self.content_widths.clone();
            self.resolved_width = available;
        }
        let col_count = self.column_widths.len();
        for i in 0..col_count {
            let content = self.content_widths.get(i).copied().unwrap_or(0);
            let spec = self.column_specs.get(i).copied().unwrap_or(TableColumn::Auto);
            let resolved = match spec {
                TableColumn::Auto => content,
                TableColumn::Fixed(n) => n,
                TableColumn::Min(n) => content.max(n),
                TableColumn::Max(n) => content.min(n),
                TableColumn::Percent(pct) => {
                    let pct = pct.clamp(1, 100) as u32;
                    (available.saturating_mul(pct)) / 100
                }
            };
            self.column_widths[i] = resolved;
        }
    }

    pub(crate) fn column_widths(&self) -> &[u32] {
        &self.column_widths
    }

    pub(crate) fn is_dirty(&self) -> bool {
        self.widths_dirty
    }
}

/// Visual style for [`Context::paginator`](crate::Context::paginator).
///
/// `Dots` renders one `●`/`○` glyph per page and is the default; it falls back
/// to `Arabic` automatically once there are more than 12 pages so the indicator
/// never overflows. `Arabic` renders a compact `{page}/{total}` counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaginatorStyle {
    /// One `●`/`○` glyph per page. Auto-falls back to [`Self::Arabic`] past 12 pages.
    #[default]
    Dots,
    /// Compact `{page}/{total}` counter.
    Arabic,
}

/// Standalone pagination state, decoupled from any list or table.
///
/// Owns a page index over an arbitrary item count, so you can paginate a
/// wizard, slide deck, onboarding flow, carousel, or any non-table data. Pass a
/// mutable reference to [`Context::paginator`](crate::Context::paginator) each
/// frame; Left/`h`/PageUp move to the previous page and Right/`l`/PageDown move
/// to the next page when the widget is focused.
///
/// # Example
///
/// ```no_run
/// use slt::{PaginatorState, PaginatorStyle};
///
/// let mut state = PaginatorState::new(42, 10); // 42 items, 10 per page
/// state.style = PaginatorStyle::Arabic;
/// assert_eq!(state.total_pages(), 5);
/// let (start, end) = state.page_bounds(); // slice your own data with these
/// assert_eq!((start, end), (0, 10));
/// ```
#[derive(Debug, Clone)]
pub struct PaginatorState {
    /// Total number of items being paged over.
    pub total_items: usize,
    /// Items per page (clamped to `>= 1` internally).
    pub per_page: usize,
    /// Current page (0-based).
    pub page: usize,
    /// Rendering style.
    pub style: PaginatorStyle,
}

impl PaginatorState {
    /// Create a paginator over `total_items` with `per_page` items per page.
    ///
    /// `per_page` is clamped to at least `1` internally (so a `0` argument is
    /// treated as `1`, avoiding division by zero). The current page starts at
    /// `0` and the style defaults to [`PaginatorStyle::Dots`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::PaginatorState;
    ///
    /// let state = PaginatorState::new(30, 0); // 0 per_page -> clamped to 1
    /// assert_eq!(state.per_page, 1);
    /// assert_eq!(state.total_pages(), 30);
    /// ```
    pub fn new(total_items: usize, per_page: usize) -> Self {
        Self {
            total_items,
            per_page: per_page.max(1),
            page: 0,
            style: PaginatorStyle::default(),
        }
    }

    /// Total number of pages; always `>= 1` (returns `1` when there are no items).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::PaginatorState;
    ///
    /// assert_eq!(PaginatorState::new(0, 5).total_pages(), 1);
    /// assert_eq!(PaginatorState::new(10, 3).total_pages(), 4);
    /// assert_eq!(PaginatorState::new(9, 3).total_pages(), 3);
    /// ```
    pub fn total_pages(&self) -> usize {
        self.total_items.div_ceil(self.per_page.max(1)).max(1)
    }

    /// Inclusive-start / exclusive-end item indices for the current page.
    ///
    /// `end` is clamped to `total_items`, so callers can slice their own data
    /// with `&items[start..end]` without bounds-checking the tail page.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::PaginatorState;
    ///
    /// let mut state = PaginatorState::new(10, 3);
    /// assert_eq!(state.page_bounds(), (0, 3));
    /// state.set_page(3); // last (partial) page
    /// assert_eq!(state.page_bounds(), (9, 10));
    /// ```
    pub fn page_bounds(&self) -> (usize, usize) {
        let start = self
            .page
            .saturating_mul(self.per_page)
            .min(self.total_items);
        let end = start.saturating_add(self.per_page).min(self.total_items);
        (start, end)
    }

    /// Advance one page, clamped to the last page (no wrap).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::PaginatorState;
    ///
    /// let mut state = PaginatorState::new(6, 3); // 2 pages
    /// state.next_page();
    /// assert_eq!(state.page, 1);
    /// state.next_page(); // already last page -> clamped
    /// assert_eq!(state.page, 1);
    /// ```
    pub fn next_page(&mut self) {
        self.page = (self.page + 1).min(self.total_pages().saturating_sub(1));
    }

    /// Go back one page, clamped to `0` (no wrap).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::PaginatorState;
    ///
    /// let mut state = PaginatorState::new(6, 3);
    /// state.prev_page(); // already page 0 -> clamped
    /// assert_eq!(state.page, 0);
    /// ```
    pub fn prev_page(&mut self) {
        self.page = self.page.saturating_sub(1);
    }

    /// Jump to a specific page, clamped into `[0, total_pages() - 1]`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::PaginatorState;
    ///
    /// let mut state = PaginatorState::new(10, 3); // 4 pages
    /// state.set_page(99);
    /// assert_eq!(state.page, 3);
    /// ```
    pub fn set_page(&mut self, page: usize) {
        self.page = page.min(self.total_pages().saturating_sub(1));
    }

    /// Update the item count and re-clamp the current page into range.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::PaginatorState;
    ///
    /// let mut state = PaginatorState::new(10, 3);
    /// state.set_page(3); // last page
    /// state.set_total_items(3); // now only 1 page
    /// assert_eq!(state.page, 0);
    /// ```
    pub fn set_total_items(&mut self, total: usize) {
        self.total_items = total;
        self.page = self.page.min(self.total_pages().saturating_sub(1));
    }

    /// Update items-per-page (clamped to `>= 1`) and re-clamp the current page.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::PaginatorState;
    ///
    /// let mut state = PaginatorState::new(10, 3); // 4 pages
    /// state.set_page(3);
    /// state.set_per_page(10); // now only 1 page
    /// assert_eq!(state.per_page, 10);
    /// assert_eq!(state.page, 0);
    /// ```
    pub fn set_per_page(&mut self, per_page: usize) {
        self.per_page = per_page.max(1);
        self.page = self.page.min(self.total_pages().saturating_sub(1));
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
    ///
    /// Field-name pairing: `start_line` + `line_count` → constructor named
    /// `line`. Use [`Self::span`] for multi-line ranges.
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
///
/// Both axes are tracked (#247): the vertical axis (`offset`, [`scroll_up`] /
/// [`scroll_down`]) drives [`Context::scroll_col`], and the horizontal axis
/// (`offset_x`, [`scroll_left`] / [`scroll_right`]) drives
/// [`Context::scroll_row`]. A single [`ScrollState`] scrolls one axis per
/// container — nest a `scroll_row` inside a `scroll_col` for both. The vertical
/// API is unchanged from earlier versions.
///
/// [`scroll_up`]: ScrollState::scroll_up
/// [`scroll_down`]: ScrollState::scroll_down
/// [`scroll_left`]: ScrollState::scroll_left
/// [`scroll_right`]: ScrollState::scroll_right
/// [`Context::scroll_col`]: crate::Context::scroll_col
/// [`Context::scroll_row`]: crate::Context::scroll_row
#[derive(Debug, Clone)]
pub struct ScrollState {
    /// Current vertical scroll offset in rows.
    pub offset: usize,
    /// Current horizontal scroll offset in columns (#247).
    pub offset_x: usize,
    /// Whether the scrollbar thumb is currently being dragged.
    ///
    /// Set to `true` by [`Context::scrollbar`] on a mouse-down inside the
    /// thumb and back to `false` on mouse-up, mirroring
    /// [`SplitPaneState::dragging`](crate::widgets::SplitPaneState). Persists
    /// across frames so cursor motion outside the thumb (or even outside the
    /// track on the x-axis) keeps scrolling while the button is held.
    ///
    /// [`Context::scrollbar`]: crate::Context::scrollbar
    pub dragging: bool,
    content_height: u32,
    viewport_height: u32,
    content_width: u32,
    viewport_width: u32,
    highlights: Vec<HighlightRange>,
    current_highlight: Option<usize>,
}

impl ScrollState {
    /// Create scroll state starting at offset 0.
    pub fn new() -> Self {
        Self {
            offset: 0,
            offset_x: 0,
            dragging: false,
            content_height: 0,
            viewport_height: 0,
            content_width: 0,
            viewport_width: 0,
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

    /// Get the scroll progress as a ratio in `[0.0, 1.0]`.
    ///
    /// Returns `f64` to match the rest of the ratio surface unified in v0.20
    /// (`Gauge::ratio`, `SplitPaneState::ratio`, `progress(ratio)`,
    /// `progress_bar(ratio)`). Feed the value straight into [`Context::gauge`]
    /// or [`Context::progress_bar`] without a cast.
    ///
    /// [`Context::gauge`]: crate::Context::gauge
    /// [`Context::progress_bar`]: crate::Context::progress_bar
    ///
    /// ```no_run
    /// # use slt::ScrollState;
    /// let scroll = ScrollState::new();
    /// // Bounds are populated by the `scrollable` widget each frame; a fresh
    /// // state with no content reports 0.0.
    /// let ratio: f64 = scroll.progress_ratio();
    /// assert!((0.0..=1.0).contains(&ratio));
    /// ```
    pub fn progress_ratio(&self) -> f64 {
        let max = self.content_height.saturating_sub(self.viewport_height);
        if max == 0 {
            0.0
        } else {
            self.offset as f64 / max as f64
        }
    }

    /// Deprecated `f32` alias for [`progress_ratio`](Self::progress_ratio).
    ///
    /// `ScrollState::progress` was the only `f32` ratio left after the v0.20
    /// `f32 → f64` ratio unification. Migrate to [`progress_ratio`](Self::progress_ratio):
    /// call sites that wrapped the result in `as f64` can drop the cast, while
    /// call sites passing the value to `gauge` / `progress_bar` (which already
    /// take `f64`) need no cast at all.
    #[deprecated(
        since = "0.21.0",
        note = "use progress_ratio() — f64 matches the rest of the v0.20+ ratio surface (gauge/progress_bar take f64; drop any `as f64` cast)"
    )]
    pub fn progress(&self) -> f32 {
        self.progress_ratio() as f32
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

    /// Set the absolute scroll offset, clamped to `[0, content - viewport]`.
    ///
    /// Uses the same `max_offset` semantics as [`scroll_down`](Self::scroll_down).
    /// Click-to-jump and thumb-drag in [`Context::scrollbar`] route through
    /// this so an out-of-range target row never leaves the offset past the
    /// last full screen of content. Direct `state.offset = …` writes keep
    /// working; this is the clamping-safe alternative.
    ///
    /// [`Context::scrollbar`]: crate::Context::scrollbar
    ///
    /// ```no_run
    /// # use slt::widgets::ScrollState;
    /// let mut scroll = ScrollState::new();
    /// // Bounds are populated by the `scrollable` widget each frame; on a
    /// // fresh state max_offset is 0 so any target clamps to 0.
    /// scroll.set_offset(999);
    /// assert_eq!(scroll.offset, 0);
    /// ```
    pub fn set_offset(&mut self, offset: usize) {
        let max_offset = self.content_height.saturating_sub(self.viewport_height) as usize;
        self.offset = offset.min(max_offset);
    }

    pub(crate) fn set_bounds(&mut self, content_height: u32, viewport_height: u32) {
        self.content_height = content_height;
        self.viewport_height = viewport_height;
    }

    /// Update the horizontal (x-axis) bounds (#247).
    ///
    /// Called by [`Context::scroll_row`] / [`Context::scrollable`] each frame
    /// when the bound scrollable scrolls horizontally. The vertical
    /// [`set_bounds`](Self::set_bounds) is left untouched, keeping the two axes
    /// independent.
    ///
    /// [`Context::scroll_row`]: crate::Context::scroll_row
    /// [`Context::scrollable`]: crate::Context::scrollable
    pub(crate) fn set_bounds_x(&mut self, content_width: u32, viewport_width: u32) {
        self.content_width = content_width;
        self.viewport_width = viewport_width;
    }

    /// Check if scrolling left is possible (`offset_x` is greater than 0, #247).
    ///
    /// ```no_run
    /// # use slt::ScrollState;
    /// let scroll = ScrollState::new();
    /// assert!(!scroll.can_scroll_left());
    /// ```
    pub fn can_scroll_left(&self) -> bool {
        self.offset_x > 0
    }

    /// Check if scrolling right is possible (content extends past the right
    /// edge of the viewport, #247).
    ///
    /// ```no_run
    /// # use slt::ScrollState;
    /// let scroll = ScrollState::new();
    /// // A fresh state with no content cannot scroll right.
    /// assert!(!scroll.can_scroll_right());
    /// ```
    pub fn can_scroll_right(&self) -> bool {
        (self.offset_x as u32) + self.viewport_width < self.content_width
    }

    /// Total horizontal content width in columns (#247).
    pub fn content_width(&self) -> u32 {
        self.content_width
    }

    /// Horizontal viewport width in columns (#247).
    pub fn viewport_width(&self) -> u32 {
        self.viewport_width
    }

    /// Horizontal scroll progress as a ratio in `[0.0, 1.0]` (#247).
    ///
    /// The x-axis mirror of [`progress_ratio`](Self::progress_ratio). Returns
    /// `0.0` when the content fits the viewport (no horizontal overflow). Feed
    /// it to a future horizontal scrollbar, a position readout, or a minimap.
    ///
    /// ```no_run
    /// # use slt::ScrollState;
    /// let scroll = ScrollState::new();
    /// let p: f64 = scroll.progress_x();
    /// assert!((0.0..=1.0).contains(&p));
    /// ```
    pub fn progress_x(&self) -> f64 {
        let max = self.content_width.saturating_sub(self.viewport_width);
        if max == 0 {
            0.0
        } else {
            self.offset_x as f64 / max as f64
        }
    }

    /// Scroll left by the given number of columns, clamped to 0 (#247).
    ///
    /// ```no_run
    /// # use slt::ScrollState;
    /// let mut scroll = ScrollState::new();
    /// scroll.scroll_left(4); // clamps at 0 with no content
    /// assert_eq!(scroll.offset_x, 0);
    /// ```
    pub fn scroll_left(&mut self, amount: usize) {
        self.offset_x = self.offset_x.saturating_sub(amount);
    }

    /// Scroll right by the given number of columns, clamped to the maximum
    /// horizontal offset (#247).
    ///
    /// ```no_run
    /// # use slt::ScrollState;
    /// let mut scroll = ScrollState::new();
    /// scroll.scroll_right(4); // clamps to content bounds (0 with no content)
    /// assert_eq!(scroll.offset_x, 0);
    /// ```
    pub fn scroll_right(&mut self, amount: usize) {
        let max_offset = self.content_width.saturating_sub(self.viewport_width) as usize;
        self.offset_x = (self.offset_x + amount).min(max_offset);
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

/// State for a [`crate::Context::split_pane`] /
/// [`crate::Context::vsplit_pane`] container.
///
/// Tracks the split ratio and drag state. Pass a mutable reference each frame
/// — the widget updates `ratio` in place when the user drags the handle or
/// presses arrow keys with the handle focused.
#[derive(Debug, Clone, PartialEq)]
pub struct SplitPaneState {
    /// Fraction of space given to the first pane. Clamped to
    /// `[min_ratio, 1.0 - min_ratio]`.
    pub ratio: f64,
    /// Whether the handle is currently being dragged.
    pub dragging: bool,
    /// Minimum fraction allocated to either pane. Default: `0.10`.
    pub min_ratio: f64,
}

/// Default minimum fraction of either pane, used by [`SplitPaneState::new`].
///
/// Crate-internal: there is no public path that benefits from constructing
/// with this constant — call [`SplitPaneState::new`] for the default (0.10)
/// or [`SplitPaneState::with_min_ratio`] to override per-instance.
pub(crate) const DEFAULT_SPLIT_MIN_RATIO: f64 = 0.10;

impl SplitPaneState {
    /// Create split state with the given initial ratio, clamped to
    /// `[DEFAULT_SPLIT_MIN_RATIO, 1.0 - DEFAULT_SPLIT_MIN_RATIO]` (default
    /// `[0.10, 0.90]`).
    pub fn new(ratio: f64) -> Self {
        let min_ratio = DEFAULT_SPLIT_MIN_RATIO;
        let clamped = ratio.clamp(min_ratio, 1.0 - min_ratio);
        Self {
            ratio: clamped,
            dragging: false,
            min_ratio,
        }
    }

    /// Override the minimum ratio for either pane (clamped to `[0.0, 0.49]`).
    pub fn with_min_ratio(mut self, min: f64) -> Self {
        self.min_ratio = min.clamp(0.0, 0.49);
        self.ratio = self.ratio.clamp(self.min_ratio, 1.0 - self.min_ratio);
        self
    }

    /// Set the ratio, clamped to `[min_ratio, 1.0 - min_ratio]`.
    pub fn set_ratio(&mut self, ratio: f64) {
        self.ratio = ratio.clamp(self.min_ratio, 1.0 - self.min_ratio);
    }
}

impl Default for SplitPaneState {
    fn default() -> Self {
        Self::new(0.5)
    }
}

/// Column specification for [`crate::Context::grid_with()`].
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

#[cfg(test)]
mod table_v021_width_tests {
    use super::TableColumn;
    use super::TableState;

    fn resolved(specs: &[TableColumn], content: &str, available: u32) -> u32 {
        let mut state = TableState::new(vec!["H"], vec![vec![content]]);
        state.column_widths_spec(specs);
        state.recompute_widths();
        state.resolve_column_widths(available);
        state.column_widths()[0]
    }

    #[test]
    fn fixed_overrides_content() {
        assert_eq!(resolved(&[TableColumn::Fixed(5)], "averylongcell", 80), 5);
        assert_eq!(resolved(&[TableColumn::Fixed(20)], "x", 80), 20);
    }

    #[test]
    fn min_floors_content() {
        // Content/header width is at most 1 here; Min raises it to 10.
        assert_eq!(resolved(&[TableColumn::Min(10)], "x", 80), 10);
        // Content already exceeds the floor -> unchanged.
        assert_eq!(resolved(&[TableColumn::Min(2)], "abcdef", 80), 6);
    }

    #[test]
    fn max_caps_content() {
        assert_eq!(resolved(&[TableColumn::Max(4)], "abcdefghij", 80), 4);
        // Content below the cap -> unchanged.
        assert_eq!(resolved(&[TableColumn::Max(10)], "abc", 80), 3);
    }

    #[test]
    fn percent_of_available() {
        let mut state = TableState::new(vec!["A", "B"], vec![vec!["x", "y"]]);
        state.column_widths_spec(&[TableColumn::Percent(50), TableColumn::Percent(50)]);
        state.recompute_widths();
        state.resolve_column_widths(40);
        assert_eq!(state.column_widths(), &[20, 20]);
    }

    #[test]
    fn auto_equals_content_width() {
        // No spec -> resolve is a no-op and width is the content width.
        assert_eq!(resolved(&[], "hello", 80), 5);
        assert_eq!(resolved(&[TableColumn::Auto], "hello", 80), 5);
    }

    #[test]
    fn select_range_fills_inclusive() {
        let mut state = TableState::new(vec!["N"], vec![vec!["a"]; 5]);
        state.select_range(1, 3);
        let mut got: Vec<usize> = state.multi_selected.iter().copied().collect();
        got.sort_unstable();
        assert_eq!(got, vec![1, 2, 3]);
        // Reversed args produce the same inclusive set.
        state.select_range(3, 1);
        let mut got: Vec<usize> = state.multi_selected.iter().copied().collect();
        got.sort_unstable();
        assert_eq!(got, vec![1, 2, 3]);
    }

    #[test]
    fn toggle_row_inserts_then_removes() {
        let mut state = TableState::new(vec!["N"], vec![vec!["a"]; 3]);
        state.toggle_row(1);
        assert!(state.is_row_selected(1));
        state.toggle_row(1);
        assert!(!state.is_row_selected(1));
    }

    proptest::proptest! {
        #[test]
        fn fixed_min_max_invariants(
            content_len in 0usize..40,
            spec_kind in 0u8..4,
            n in 0u32..30,
            available in 1u32..200,
        ) {
            let content: String = "x".repeat(content_len);
            let spec = match spec_kind {
                0 => TableColumn::Fixed(n),
                1 => TableColumn::Min(n),
                2 => TableColumn::Max(n),
                _ => TableColumn::Auto,
            };
            let w = resolved(&[spec], &content, available);
            match spec {
                TableColumn::Fixed(n) => proptest::prop_assert_eq!(w, n),
                TableColumn::Min(n) => proptest::prop_assert!(w >= n),
                TableColumn::Max(n) => proptest::prop_assert!(w <= n),
                _ => {}
            }
        }

        #[test]
        fn percent_columns_never_exceed_available(
            pcts in proptest::collection::vec(1u8..=100, 1..6),
            available in 1u32..200,
        ) {
            let cols = pcts.len();
            let headers: Vec<String> = (0..cols).map(|i| format!("H{i}")).collect();
            let row: Vec<String> = (0..cols).map(|_| "v".to_string()).collect();
            let mut state = TableState::new(headers, vec![row]);
            let specs: Vec<TableColumn> = pcts.iter().map(|&p| TableColumn::Percent(p)).collect();
            state.column_widths_spec(&specs);
            state.recompute_widths();
            state.resolve_column_widths(available);
            // Each Percent column is floor(available * pct / 100) <= available.
            for (&w, &p) in state.column_widths().iter().zip(pcts.iter()) {
                let expected = (available.saturating_mul(p as u32)) / 100;
                proptest::prop_assert_eq!(w, expected);
                proptest::prop_assert!(w <= available);
            }
        }
    }
}

#[cfg(test)]
mod list_state_height_tests {
    use super::ListState;

    #[test]
    fn row_prefix_is_cumulative_sum() {
        let mut state = ListState::new(vec!["a", "b", "c", "d"]);
        state.set_item_heights(vec![2, 1, 3, 1]);
        state.ensure_row_prefix();
        // row_prefix[i] = total rows occupied by items 0..i.
        assert_eq!(state.row_prefix(), &[0, 2, 3, 6, 7]);
        // item_height reflects the stored (clamped) heights.
        assert_eq!(state.item_height(0), 2);
        assert_eq!(state.item_height(2), 3);
    }

    #[test]
    fn heights_below_one_are_clamped() {
        let mut state = ListState::new(vec!["a", "b", "c"]);
        state.set_item_heights(vec![0, 0, 0]);
        state.ensure_row_prefix();
        assert_eq!(state.row_prefix(), &[0, 1, 2, 3]);
        assert_eq!(state.item_height(0), 1);
    }

    #[test]
    fn dirty_gate_skips_rebuild_when_unchanged() {
        let mut state = ListState::new(vec!["a", "b"]);
        state.set_item_heights(vec![3, 2]);
        state.ensure_row_prefix();
        assert_eq!(state.row_prefix(), &[0, 3, 5]);
        // heights_dirty is now false; a second call must be a no-op and leave
        // the prefix intact (no panic, no recompute that changes the result).
        assert!(!state.heights_dirty);
        state.ensure_row_prefix();
        assert_eq!(state.row_prefix(), &[0, 3, 5]);
    }

    #[test]
    fn no_heights_falls_back_to_uniform() {
        let mut state = ListState::new(vec!["a", "b", "c"]);
        assert!(!state.has_item_heights());
        state.ensure_row_prefix();
        assert_eq!(state.row_prefix(), &[0, 1, 2, 3]);
        assert_eq!(state.item_height(0), 1);
    }

    #[test]
    fn clear_reverts_to_uniform() {
        let mut state = ListState::new(vec!["a", "b"]).with_item_heights(vec![4, 2]);
        state.ensure_row_prefix();
        assert_eq!(state.row_prefix(), &[0, 4, 6]);
        state.clear_item_heights();
        assert!(!state.has_item_heights());
        state.ensure_row_prefix();
        assert_eq!(state.row_prefix(), &[0, 1, 2]);
    }

    #[test]
    fn set_items_marks_dirty_and_resizes_prefix() {
        let mut state = ListState::new(vec!["a", "b", "c"]).with_item_heights(vec![2, 2, 2]);
        state.ensure_row_prefix();
        assert_eq!(state.row_prefix(), &[0, 2, 4, 6]);
        // Replacing items must invalidate the stale prefix.
        state.set_items(vec!["x", "y"]);
        assert!(state.heights_dirty);
        state.ensure_row_prefix();
        assert_eq!(state.row_prefix(), &[0, 2, 4]);
    }

    #[test]
    fn set_items_truncates_stale_per_item_heights() {
        let mut state = ListState::new(vec!["a", "b", "c", "d"])
            .with_item_heights(vec![2, 3, 4, 5]);
        state.set_items(vec!["x", "y"]);

        assert_eq!(state.item_height(0), 2);
        assert_eq!(state.item_height(1), 3);
        assert_eq!(state.item_height(2), 1);
        state.ensure_row_prefix();
        assert_eq!(state.row_prefix(), &[0, 2, 5]);
    }
}

#[cfg(test)]
mod scroll_state_progress_tests {
    use super::ScrollState;

    /// Build a state with the bounds the `scrollable` widget would set, plus an
    /// offset, so `progress_ratio` exercises a realistic non-zero ratio.
    fn scrolled(content_height: u32, viewport_height: u32, offset: usize) -> ScrollState {
        let mut state = ScrollState::new();
        state.set_bounds(content_height, viewport_height);
        state.offset = offset;
        state
    }

    #[test]
    fn progress_ratio_returns_f64_in_unit_range() {
        // Top of a scrollable region → 0.0.
        let top = scrolled(100, 20, 0);
        let ratio: f64 = top.progress_ratio();
        assert_eq!(ratio, 0.0);

        // Halfway through the scrollable range (offset 40 of max 80) → 0.5.
        let mid = scrolled(100, 20, 40);
        assert_eq!(mid.progress_ratio(), 0.5);

        // Fully scrolled (offset == max) → 1.0.
        let bottom = scrolled(100, 20, 80);
        assert_eq!(bottom.progress_ratio(), 1.0);
    }

    #[test]
    fn progress_ratio_is_zero_when_content_fits_viewport() {
        // No overflow → no scroll range → 0.0 (and no divide-by-zero).
        let fits = scrolled(20, 20, 0);
        assert_eq!(fits.progress_ratio(), 0.0);

        let smaller = scrolled(10, 20, 5);
        assert_eq!(smaller.progress_ratio(), 0.0);
    }

    #[test]
    fn progress_ratio_preserves_f64_precision() {
        // 1/3 is lossy in f32; the f64 surface keeps more digits than `as f32`.
        let third = scrolled(40, 10, 10); // max = 30, offset = 10 → 1/3
        let ratio = third.progress_ratio();
        assert!((ratio - 1.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    #[allow(deprecated)]
    fn deprecated_progress_delegates_to_progress_ratio() {
        // The deprecated f32 alias must agree with the f64 source within f32 epsilon.
        let state = scrolled(100, 20, 40);
        let expected = state.progress_ratio() as f32;
        assert_eq!(state.progress(), expected);
        assert!((state.progress() - 0.5).abs() < f32::EPSILON);
    }
}

#[cfg(test)]
mod list_state_reorder_tests {
    use super::ListState;

    #[test]
    fn move_item_forward_reorders_and_keeps_selection() {
        let mut state = ListState::new(vec!["a", "b", "c", "d"]);
        state.selected = 0; // "a"
        assert!(state.move_item(0, 2));
        assert_eq!(state.items, vec!["b", "c", "a", "d"]);
        // Selection follows the moved item.
        assert_eq!(state.selected_item(), Some("a"));
        assert_eq!(state.selected, 2);
    }

    #[test]
    fn move_item_backward_reorders_and_keeps_selection() {
        let mut state = ListState::new(vec!["a", "b", "c", "d"]);
        state.selected = 3; // "d"
        assert!(state.move_item(3, 1));
        assert_eq!(state.items, vec!["a", "d", "b", "c"]);
        assert_eq!(state.selected_item(), Some("d"));
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn move_item_keeps_search_cache_aligned() {
        let mut state = ListState::new(vec!["Apple", "Banana", "Cherry"]);
        assert!(state.move_item(0, 2));
        // After the move the filter must address the reordered items.
        state.set_filter("apple");
        assert_eq!(state.visible_indices().len(), 1);
        assert_eq!(state.selected_item(), Some("Apple"));
    }

    #[test]
    fn move_item_keeps_per_item_heights_aligned() {
        let mut state = ListState::new(vec!["a", "b", "c"]).with_item_heights(vec![1, 2, 3]);
        assert!(state.move_item(0, 2));
        state.ensure_row_prefix();
        // Heights travel with their items: order is now b(2), c(3), a(1).
        assert_eq!(state.item_height(0), 2);
        assert_eq!(state.item_height(1), 3);
        assert_eq!(state.item_height(2), 1);
    }

    #[test]
    fn move_item_noop_when_from_equals_to() {
        let mut state = ListState::new(vec!["a", "b", "c"]);
        state.selected = 1;
        assert!(!state.move_item(1, 1));
        assert_eq!(state.items, vec!["a", "b", "c"]);
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn move_item_out_of_bounds_is_rejected() {
        let mut state = ListState::new(vec!["a", "b", "c"]);
        assert!(!state.move_item(0, 9));
        assert!(!state.move_item(9, 0));
        assert_eq!(state.items, vec!["a", "b", "c"]);
    }

    #[test]
    fn move_item_empty_list_is_rejected() {
        let mut state = ListState::new(Vec::<String>::new());
        assert!(!state.move_item(0, 0));
        assert!(state.items.is_empty());
    }

    #[test]
    fn move_item_leaves_unrelated_selection_in_place() {
        // Moving an item that is not selected should keep selection on the
        // same logical item.
        let mut state = ListState::new(vec!["a", "b", "c", "d"]);
        state.selected = 3; // "d"
        assert!(state.move_item(0, 1)); // swap a/b; "d" stays last
        assert_eq!(state.items, vec!["b", "a", "c", "d"]);
        assert_eq!(state.selected_item(), Some("d"));
        assert_eq!(state.selected, 3);
    }
}
