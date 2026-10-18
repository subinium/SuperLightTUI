//! Collections family state — list, file picker, table, paginator, tree,
//! directory tree and calendar.
//!
//! Rendered by `context/widgets/collections.rs`.

use super::*;

/// State for a selectable list widget.
///
/// Pass a mutable reference to `Context::list` each frame. Up/Down arrow
/// keys (and `k`/`j`) move the selection when the widget is focused.
#[derive(Debug, Clone, Default)]
pub struct ListState {
    /// The list items as display strings.
    items: Vec<String>,
    /// Index of the currently selected item in the filtered **view**.
    ///
    /// This is not an index into [`items`](ListState::items): with a filter
    /// active, `remove_item(list.selected)` removes a different item. Use
    /// [`selected_index`](ListState::selected_index) (data index),
    /// [`selected_view_index`](ListState::selected_view_index),
    /// [`set_selected_index`](ListState::set_selected_index) and
    /// [`remove_selected`](ListState::remove_selected) instead.
    #[deprecated(
        since = "0.26.0",
        note = "view index, not a data index; use selected_index() / selected_view_index() / set_selected_index()"
    )]
    pub selected: usize,
    /// Case-insensitive substring filter applied to list items.
    filter: String,
    /// Top *item* index of the visible viewport for `list` and
    /// `virtual_list`. Defaults to `0` and is clamped each frame so the
    /// selection stays inside the viewport without forcing the cursor to the
    /// bottom row. For the uniform
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
    /// `row_prefix.len() == view_indices.len() + 1` after `ensure_row_prefix`.
    row_prefix: Vec<u32>,
    /// Dirty flag gating `row_prefix` rebuilds; set whenever items or heights
    /// change so a stale prefix sum is never consumed.
    heights_dirty: bool,
    view_indices: Vec<usize>,
    /// Lowercase cache parallel to `items`, rebuilt only on `set_items` / `new`.
    /// Mirrors the `row_search_cache` pattern in `TableState`.
    item_search_cache: Vec<String>,
    filter_tokens: Vec<String>,
}

impl ListState {
    /// Create a list with the given items. The first item is selected initially.
    // The struct literal initialises the deprecated `selected` storage (#455).
    #[expect(
        deprecated,
        reason = "crate-internal storage access to the deprecated public selected field (#455)"
    )]
    pub fn new(items: Vec<impl Into<String>>) -> Self {
        let items: Vec<String> = items.into_iter().map(Into::into).collect();
        let item_search_cache: Vec<String> = items.iter().map(|s| s.to_lowercase()).collect();
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
            filter_tokens: Vec::new(),
        }
    }

    /// Replace the list items and rebuild the view index.
    ///
    /// Use this instead of assigning `items` directly to ensure the internal
    /// filter/view state stays consistent.
    pub fn set_items(&mut self, items: Vec<impl Into<String>>) {
        self.items = items.into_iter().map(Into::into).collect();
        self.rebuild_item_caches();
    }

    /// Return all items in their underlying, unfiltered order.
    pub fn items(&self) -> &[String] {
        &self.items
    }

    /// Return the number of underlying items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Return whether the underlying list is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Return an item by underlying data index.
    pub fn item(&self, index: usize) -> Option<&str> {
        self.items.get(index).map(String::as_str)
    }

    /// Append an item, updating only its search cache and filtered-view entry.
    /// New items have a height of one row when per-item heights are enabled.
    pub fn push_item(&mut self, item: impl Into<String>) {
        let item = item.into();
        let cached = item.to_lowercase();
        let visible = self
            .filter_tokens
            .iter()
            .all(|token| cached.contains(token));
        let index = self.items.len();
        self.items.push(item);
        self.item_search_cache.push(cached);
        if let Some(heights) = &mut self.item_heights {
            heights.push(1);
        }
        if visible {
            self.view_indices.push(index);
            if !self.heights_dirty && !self.row_prefix.is_empty() {
                let bottom = self.row_prefix.last().copied().unwrap_or(0);
                self.row_prefix.push(bottom.saturating_add(1));
            } else {
                self.heights_dirty = true;
            }
        }
        *self.view_cursor_mut() = self
            .view_cursor()
            .min(self.view_indices.len().saturating_sub(1));
        self.viewport_offset = self
            .viewport_offset
            .min(self.view_indices.len().saturating_sub(1));
    }

    /// Insert an item at a data index. Returns `false` when out of bounds.
    pub fn insert_item(&mut self, index: usize, item: impl Into<String>) -> bool {
        if index > self.items.len() {
            return false;
        }
        let selected = self.view_indices.get(self.view_cursor()).copied();
        let item = item.into();
        self.item_search_cache.insert(index, item.to_lowercase());
        self.items.insert(index, item);
        if let Some(heights) = &mut self.item_heights {
            heights.insert(index, 1);
        }
        self.rebuild_view();
        self.restore_selection(selected.map(|raw| raw + usize::from(raw >= index)));
        true
    }

    /// Remove and return an item by data index.
    pub fn remove_item(&mut self, index: usize) -> Option<String> {
        if index >= self.items.len() {
            return None;
        }
        let selected = self.view_indices.get(self.view_cursor()).copied();
        let item = self.items.remove(index);
        self.item_search_cache.remove(index);
        if let Some(heights) = &mut self.item_heights {
            heights.remove(index);
        }
        self.rebuild_view();
        self.restore_selection(
            selected
                .filter(|&raw| raw != index)
                .map(|raw| raw - usize::from(raw > index)),
        );
        Some(item)
    }

    /// Remove all items and reset cache-coupled selection and viewport state.
    pub fn clear_items(&mut self) {
        self.items.clear();
        self.rebuild_item_caches();
    }

    /// Return the active filter text.
    pub fn filter(&self) -> &str {
        &self.filter
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
        let mut heights: Vec<u32> = heights.into_iter().map(|h| h.max(1)).collect();
        heights.resize(self.items.len(), 1);
        self.item_heights = Some(heights);
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
    /// total number of rows occupied by filtered items `0..i`, and
    /// `row_prefix.len() == view_indices.len() + 1`. Rebuild is `O(n)` and skipped
    /// entirely when `heights_dirty` is `false`.
    pub(crate) fn ensure_row_prefix(&mut self) {
        if !self.heights_dirty && self.row_prefix.len() == self.view_indices.len() + 1 {
            return;
        }
        let n = self.view_indices.len();
        self.row_prefix.clear();
        self.row_prefix.reserve(n + 1);
        let mut acc = 0u32;
        self.row_prefix.push(0);
        for &i in &self.view_indices {
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
        let filter = filter.into();
        if self.filter == filter {
            return;
        }
        self.filter = filter;
        self.filter_tokens = self
            .filter
            .split_whitespace()
            .map(str::to_lowercase)
            .collect();
        self.rebuild_view();
    }

    /// Returns indices of items visible after filtering.
    pub fn visible_indices(&self) -> &[usize] {
        &self.view_indices
    }

    /// Get the currently selected item text, or `None` if the list is empty.
    pub fn selected_item(&self) -> Option<&str> {
        let data_idx = *self.view_indices.get(self.view_cursor())?;
        self.items.get(data_idx).map(String::as_str)
    }

    /// Data index (into [`items`](ListState::items)) of the selected item, or
    /// `None` when the filtered view is empty.
    ///
    /// Equals `visible_indices()[selected_view_index()]`, so it is the index
    /// to pass to [`remove_item`](ListState::remove_item),
    /// [`item`](ListState::item) or [`move_item`](ListState::move_item).
    ///
    /// # Example
    ///
    /// ```
    /// use slt::widgets::ListState;
    ///
    /// let mut list = ListState::new(vec!["apple", "banana", "cherry"]);
    /// list.set_filter("cherry");
    /// assert_eq!(list.selected_view_index(), 0);
    /// assert_eq!(list.selected_index(), Some(2));
    /// ```
    ///
    /// Available since `0.26.0`.
    pub fn selected_index(&self) -> Option<usize> {
        self.view_indices.get(self.view_cursor()).copied()
    }

    /// Position of the selected item in the filtered view (what the
    /// deprecated `selected` field holds). `0` when the view is empty.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::widgets::ListState;
    ///
    /// let mut list = ListState::new(vec!["apple", "banana", "cherry"]);
    /// assert!(list.set_selected_index(1));
    /// list.set_filter("an");
    /// assert_eq!(list.selected_view_index(), 0); // "banana" is the only row
    /// ```
    ///
    /// Available since `0.26.0`.
    pub fn selected_view_index(&self) -> usize {
        self.view_cursor()
    }

    /// Select the item at position `view` of the filtered view (the write
    /// counterpart of [`selected_view_index`](ListState::selected_view_index)).
    /// Returns `false` (selection unchanged) when `view` is past the end.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::widgets::ListState;
    ///
    /// let mut list = ListState::new(vec!["apple", "banana", "cherry"]);
    /// list.set_filter("an");
    /// assert!(list.set_selected_view_index(0));
    /// assert_eq!(list.selected_item(), Some("banana"));
    /// assert!(!list.set_selected_view_index(1));
    /// ```
    ///
    /// Available since `0.26.0`.
    pub fn set_selected_view_index(&mut self, view: usize) -> bool {
        if view >= self.view_indices.len() {
            return false;
        }
        *self.view_cursor_mut() = view;
        true
    }

    /// Select the item at data index `data`. Returns `false` (selection
    /// unchanged) when the index is out of range or hidden by the filter.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::widgets::ListState;
    ///
    /// let mut list = ListState::new(vec!["apple", "banana", "cherry"]);
    /// list.set_filter("a");
    /// assert!(list.set_selected_index(1));
    /// assert_eq!(list.selected_item(), Some("banana"));
    /// assert!(!list.set_selected_index(2)); // "cherry" is filtered out
    /// ```
    ///
    /// Available since `0.26.0`.
    pub fn set_selected_index(&mut self, data: usize) -> bool {
        // The filtered view keeps data order, so it is sorted ascending.
        match self.view_indices.binary_search(&data) {
            Ok(view) => {
                *self.view_cursor_mut() = view;
                true
            }
            Err(_) => false,
        }
    }

    /// Remove and return the selected item — the one the user sees
    /// highlighted — or `None` when the filtered view is empty.
    ///
    /// The selection stays at the same view position, so it moves to the next
    /// visible item (or the new last one).
    ///
    /// # Example
    ///
    /// ```
    /// use slt::widgets::ListState;
    ///
    /// let mut list = ListState::new(vec!["apple", "banana", "cherry"]);
    /// list.set_filter("cherry");
    /// assert_eq!(list.remove_selected().as_deref(), Some("cherry"));
    /// assert_eq!(list.items(), ["apple", "banana"]);
    /// ```
    ///
    /// Available since `0.26.0`.
    pub fn remove_selected(&mut self) -> Option<String> {
        let data = self.selected_index()?;
        self.remove_item(data)
    }

    /// Storage accessors for the deprecated public `selected` field (#455):
    /// crate code reads and writes the view cursor through these so the
    /// deprecation only reaches downstream direct field access.
    #[expect(
        deprecated,
        reason = "crate-internal storage access to the deprecated public selected field (#455)"
    )]
    #[inline]
    pub(crate) fn view_cursor(&self) -> usize {
        self.selected
    }

    #[expect(
        deprecated,
        reason = "crate-internal storage access to the deprecated public selected field (#455)"
    )]
    #[inline]
    pub(crate) fn view_cursor_mut(&mut self) -> &mut usize {
        &mut self.selected
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
        let selected_data = self.view_indices.get(self.view_cursor()).copied();

        let item = self.items.remove(from);
        self.items.insert(to, item);

        // Keep the lowercase search cache aligned with `items`.
        if from < self.item_search_cache.len() {
            let cached = self.item_search_cache.remove(from);
            self.item_search_cache
                .insert(to.min(self.item_search_cache.len()), cached);
        }

        // Keep per-item heights aligned with `items` when present.
        if let Some(heights) = self.item_heights.as_mut() {
            let h = heights.remove(from);
            heights.insert(to, h);
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
                *self.view_cursor_mut() = view_pos;
            }
        }

        true
    }

    fn rebuild_view(&mut self) {
        self.view_indices.clear();
        self.view_indices
            .extend(
                self.item_search_cache
                    .iter()
                    .enumerate()
                    .filter_map(|(i, cached)| {
                        self.filter_tokens
                            .iter()
                            .all(|token| cached.contains(token))
                            .then_some(i)
                    }),
            );
        self.heights_dirty = true;
        self.viewport_offset = self
            .viewport_offset
            .min(self.view_indices.len().saturating_sub(1));
        self.viewport_row_offset = 0;
        if !self.view_indices.is_empty() && self.view_cursor() >= self.view_indices.len() {
            *self.view_cursor_mut() = self.view_indices.len() - 1;
        } else if self.view_indices.is_empty() {
            *self.view_cursor_mut() = 0;
        }
    }

    fn restore_selection(&mut self, raw: Option<usize>) {
        if let Some(view) = raw.and_then(|raw| self.view_indices.iter().position(|&i| i == raw)) {
            *self.view_cursor_mut() = view;
        }
    }

    fn rebuild_item_caches(&mut self) {
        self.item_search_cache = self.items.iter().map(|item| item.to_lowercase()).collect();
        if let Some(heights) = self.item_heights.as_mut() {
            heights.resize(self.items.len(), 1);
            release_excess_capacity(heights);
        }
        self.heights_dirty = true;
        self.viewport_offset = self.viewport_offset.min(self.items.len().saturating_sub(1));
        self.rebuild_view();
        // Bulk replacement (`set_items` / `clear_items`) is the only path that
        // can shrink the list by orders of magnitude, so release index buffers
        // sized for the old data here (#416). Filter changes keep capacity to
        // avoid reallocating on every keystroke.
        release_excess_capacity(&mut self.view_indices);
        // `heights_dirty` is set, so the prefix sum is rebuilt before any read.
        self.row_prefix.clear();
        release_excess_capacity(&mut self.row_prefix);
    }
}

/// Shrink `buf` when its capacity is far above its length, so a collection
/// replaced by a much smaller one does not keep the old index buffers alive.
fn release_excess_capacity<T>(buf: &mut Vec<T>) {
    const SLACK: usize = 64;
    if buf.capacity() > buf.len().saturating_mul(2).saturating_add(SLACK) {
        buf.shrink_to(buf.len());
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

/// Filesystem operation that failed during a file-picker scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilePickerScanOperation {
    /// Opening the root directory failed. No replacement listing is produced.
    ReadDirectory,
    /// Reading one entry from an open directory failed. Other entries are kept.
    ReadEntry,
    /// Inspecting an entry's file type failed. That entry is skipped.
    ReadFileType,
    /// Reading file metadata failed. The entry is kept with an unknown size.
    ReadMetadata,
}

/// A structured failure captured while scanning a file-picker directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilePickerScanError {
    /// Operation that failed.
    pub operation: FilePickerScanOperation,
    /// Path associated with the failure.
    pub path: PathBuf,
    /// Portable I/O error category.
    pub kind: std::io::ErrorKind,
    /// Original operating-system error message.
    pub message: String,
}

impl std::fmt::Display for FilePickerScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {} ({})",
            self.path.display(),
            self.message,
            match self.operation {
                FilePickerScanOperation::ReadDirectory => "read directory",
                FilePickerScanOperation::ReadEntry => "read entry",
                FilePickerScanOperation::ReadFileType => "read file type",
                FilePickerScanOperation::ReadMetadata => "read metadata",
            }
        )
    }
}

impl std::error::Error for FilePickerScanError {}

/// Current freshness of a file-picker directory listing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilePickerScanStatus {
    /// The directory has not been scanned yet.
    Pending,
    /// The listing is complete and current.
    Ready,
    /// Configuration or the directory changed and the retained listing is stale.
    Stale,
    /// The latest scan failed fully or partially.
    Error,
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
    scan_status: FilePickerScanStatus,
    scan_errors: Vec<FilePickerScanError>,
    viewport_offset: usize,
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
    /// File size in bytes, or `None` for directories and unreadable metadata.
    pub size: Option<u64>,
    sort_key: String,
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
            scan_status: FilePickerScanStatus::Pending,
            scan_errors: Vec::new(),
            viewport_offset: 0,
        }
    }

    /// Configure whether hidden files should be shown.
    pub fn show_hidden(mut self, show: bool) -> Self {
        self.show_hidden = show;
        self.invalidate_scan();
        self
    }

    /// Restrict visible files to the provided extensions.
    pub fn extensions(mut self, exts: &[&str]) -> Self {
        self.extensions = exts
            .iter()
            .map(|ext| ext.trim().trim_start_matches('.').to_ascii_lowercase())
            .filter(|ext| !ext.is_empty())
            .collect();
        self.invalidate_scan();
        self
    }

    /// Return the freshness of the current directory listing.
    pub fn scan_status(&self) -> FilePickerScanStatus {
        self.scan_status
    }

    /// Return every failure captured by the most recent scan.
    ///
    /// A root `read_dir` failure produces one error and retains the previous
    /// listing. Per-entry and metadata failures produce a best-effort listing.
    pub fn scan_errors(&self) -> &[FilePickerScanError] {
        &self.scan_errors
    }

    /// Return the most recent failure from the latest scan.
    pub fn last_error(&self) -> Option<&FilePickerScanError> {
        self.scan_errors.last()
    }

    /// Request another scan after a transient failure.
    pub fn retry(&mut self) {
        self.scan_errors.clear();
        self.scan_status = if self.entries.is_empty() {
            FilePickerScanStatus::Pending
        } else {
            FilePickerScanStatus::Stale
        };
        self.dirty = true;
    }

    pub(crate) fn invalidate_scan(&mut self) {
        self.retry();
        self.viewport_offset = 0;
    }

    pub(crate) fn viewport_offset(&self) -> usize {
        self.viewport_offset
    }

    pub(crate) fn keep_selected_visible(&mut self, visible_rows: usize) {
        if self.entries.is_empty() || visible_rows == 0 {
            self.viewport_offset = 0;
            return;
        }
        let max_offset = self.entries.len().saturating_sub(visible_rows);
        if self.selected < self.viewport_offset {
            self.viewport_offset = self.selected;
        } else if self.selected >= self.viewport_offset.saturating_add(visible_rows) {
            self.viewport_offset = self.selected + 1 - visible_rows;
        }
        self.viewport_offset = self.viewport_offset.min(max_offset);
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
    /// use slt::widgets::FilePickerState;
    ///
    /// let mut state = FilePickerState::new(".");
    /// slt::run(|ui: &mut slt::Context| {
    ///     if ui.file_picker(&mut state).changed {
    ///         if let Some(path) = state.selected_file() {
    ///             println!("picked: {}", path.display());
    ///         }
    ///     }
    /// })?;
    /// # Ok::<_, std::io::Error>(())
    /// ```
    pub fn selected_file(&self) -> Option<&PathBuf> {
        self.selected_file.as_ref()
    }

    /// Re-scan the current directory, retaining failures in [`scan_errors`](Self::scan_errors).
    ///
    /// This compatibility wrapper ignores the returned error. Use
    /// [`try_refresh`](Self::try_refresh) when the caller needs immediate
    /// fallible control flow.
    pub fn refresh(&mut self) {
        let _ = self.try_refresh();
    }

    /// Re-scan the current directory and rebuild entries.
    ///
    /// Root directory failures retain the previous listing. Entry-read and
    /// file-type failures skip only the affected entry. Metadata failures keep
    /// the file with [`FileEntry::size`] set to `None`. Any partial failure is
    /// returned and remains inspectable through [`scan_errors`](Self::scan_errors).
    pub fn try_refresh(&mut self) -> Result<(), FilePickerScanError> {
        self.try_refresh_impl(|_| {})
    }

    #[cfg(test)]
    pub(crate) fn try_refresh_with_metadata_hook(
        &mut self,
        hook: impl FnMut(&std::path::Path),
    ) -> Result<(), FilePickerScanError> {
        self.try_refresh_impl(hook)
    }

    fn try_refresh_impl(
        &mut self,
        mut before_metadata: impl FnMut(&std::path::Path),
    ) -> Result<(), FilePickerScanError> {
        let mut entries = Vec::new();
        let mut errors = Vec::new();

        let read_dir = match fs::read_dir(&self.current_dir) {
            Ok(read_dir) => read_dir,
            Err(error) => {
                let error = FilePickerScanError {
                    operation: FilePickerScanOperation::ReadDirectory,
                    path: self.current_dir.clone(),
                    kind: error.kind(),
                    message: error.to_string(),
                };
                self.scan_errors = vec![error.clone()];
                self.scan_status = FilePickerScanStatus::Error;
                self.dirty = true;
                return Err(error);
            }
        };

        for result in read_dir {
            let dir_entry = match result {
                Ok(entry) => entry,
                Err(error) => {
                    errors.push(FilePickerScanError {
                        operation: FilePickerScanOperation::ReadEntry,
                        path: self.current_dir.clone(),
                        kind: error.kind(),
                        message: error.to_string(),
                    });
                    continue;
                }
            };
            let name = dir_entry.file_name().to_string_lossy().to_string();
            if !self.show_hidden && name.starts_with('.') {
                continue;
            }

            let file_type = match dir_entry.file_type() {
                Ok(file_type) => file_type,
                Err(error) => {
                    errors.push(FilePickerScanError {
                        operation: FilePickerScanOperation::ReadFileType,
                        path: dir_entry.path(),
                        kind: error.kind(),
                        message: error.to_string(),
                    });
                    continue;
                }
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
                None
            } else {
                before_metadata(&path);
                match fs::symlink_metadata(&path) {
                    Ok(metadata) => Some(metadata.len()),
                    Err(error) => {
                        errors.push(FilePickerScanError {
                            operation: FilePickerScanOperation::ReadMetadata,
                            path: path.clone(),
                            kind: error.kind(),
                            message: error.to_string(),
                        });
                        None
                    }
                }
            };

            let sort_key = name.to_lowercase();
            entries.push(FileEntry {
                name,
                path,
                is_dir,
                size,
                sort_key,
            });
        }

        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a
                .sort_key
                .cmp(&b.sort_key)
                .then_with(|| a.name.cmp(&b.name)),
        });

        self.entries = entries;
        if self.entries.is_empty() {
            self.selected = 0;
        } else {
            self.selected = self.selected.min(self.entries.len().saturating_sub(1));
        }
        self.viewport_offset = self
            .viewport_offset
            .min(self.entries.len().saturating_sub(1));
        self.scan_errors = errors;
        if let Some(error) = self.scan_errors.last().cloned() {
            self.scan_status = FilePickerScanStatus::Error;
            self.dirty = true;
            Err(error)
        } else {
            self.scan_status = FilePickerScanStatus::Ready;
            self.dirty = false;
            Ok(())
        }
    }
}

impl Default for FilePickerState {
    fn default() -> Self {
        Self::new(".")
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
///
/// let mut table = TableState::new(
///     vec!["Name", "Status"],
///     vec![vec!["build", "ok"]],
/// );
/// // Pin the status column to 6 cells, leave the name column automatic.
/// table.column_widths_spec(&[TableColumn::Auto, TableColumn::Fixed(6)]);
/// slt::run(|ui: &mut slt::Context| {
///     ui.table(&mut table);
/// })?;
/// # Ok::<_, std::io::Error>(())
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
/// focused/cursor row ([`selected_view_index`](TableState::selected_view_index)) is
/// tracked separately.
#[derive(Debug, Clone)]
pub struct TableState {
    /// Column header labels.
    headers: Vec<String>,
    /// Table rows, each a `Vec` of cell strings.
    rows: Vec<Vec<String>>,
    /// Focused/cursor row as a **view** index (after filter and sort).
    ///
    /// This is not an index into [`rows`](TableState::rows): with a filter or
    /// sort active, `remove_row(table.selected)` removes a different row. Use
    /// [`selected_index`](TableState::selected_index) (data index),
    /// [`selected_view_index`](TableState::selected_view_index),
    /// [`set_selected_index`](TableState::set_selected_index) and
    /// [`remove_selected`](TableState::remove_selected) instead.
    #[deprecated(
        since = "0.26.0",
        note = "view index, not a data index; use selected_index() / selected_view_index() / set_selected_index()"
    )]
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
    filter: String,
    /// Current page (0-based) when pagination is enabled.
    pub page: usize,
    /// Rows per page (`0` disables pagination).
    pub page_size: usize,
    /// Whether alternating row backgrounds are enabled.
    pub zebra: bool,
    /// Top view row drawn by an unpaged `table` (#487). Clamped each frame so
    /// the focused row stays inside the body viewport; ignored when paged.
    pub(crate) viewport_offset: usize,
    view_indices: Vec<usize>,
    pub(super) row_search_cache: Vec<String>,
    pub(super) filter_tokens: Vec<String>,
}

impl Default for TableState {
    // The struct literal initialises the deprecated `selected` storage (#455).
    #[expect(
        deprecated,
        reason = "crate-internal storage access to the deprecated public selected field (#455)"
    )]
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
            viewport_offset: 0,
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
            ..Self::default()
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

    /// Return column headers.
    pub fn headers(&self) -> &[String] {
        &self.headers
    }

    /// Replace column headers and invalidate width caches.
    pub fn set_headers(&mut self, headers: Vec<impl Into<String>>) {
        self.headers = headers.into_iter().map(Into::into).collect();
        if self
            .sort_column
            .is_some_and(|column| column >= self.headers.len())
        {
            self.sort_column = None;
            self.rebuild_view();
        }
        self.widths_dirty = true;
    }

    /// Return all rows in underlying data order.
    pub fn rows(&self) -> &[Vec<String>] {
        &self.rows
    }

    /// Return a row by underlying data index.
    pub fn row(&self, index: usize) -> Option<&[String]> {
        self.rows.get(index).map(Vec::as_slice)
    }

    /// Return the number of underlying rows.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Return whether the table has no rows.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Append a row with an incremental search cache update.
    /// Unsorted views append matching rows directly; sorted views rebuild
    /// their order. Use [`set_rows`](Self::set_rows) for bulk sorted loads.
    pub fn push_row(&mut self, row: Vec<impl Into<String>>) {
        let row: Vec<String> = row.into_iter().map(Into::into).collect();
        let cached = Self::searchable_row(&row);
        let visible = self
            .filter_tokens
            .iter()
            .all(|token| cached.contains(token));
        let index = self.rows.len();
        if !self.widths_dirty {
            for (width, cell) in self.content_widths.iter_mut().zip(&row) {
                *width = (*width).max(crate::width::str_width(cell.as_str()) as u32);
            }
            self.column_widths.clone_from(&self.content_widths);
        }
        self.rows.push(row);
        self.row_search_cache.push(cached);
        if self.sort_column.is_some() {
            self.rebuild_view();
        } else {
            if visible {
                self.view_indices.push(index);
            }
            self.page = if self.page_size > 0 {
                self.page.min(self.total_pages().saturating_sub(1))
            } else {
                0
            };
            *self.view_cursor_mut() = self
                .view_cursor()
                .min(self.view_indices.len().saturating_sub(1));
            self.prune_selection();
        }
    }

    /// Insert a row at a data index. Returns `false` when out of bounds.
    pub fn insert_row(&mut self, index: usize, row: Vec<impl Into<String>>) -> bool {
        if index > self.rows.len() {
            return false;
        }
        self.rows
            .insert(index, row.into_iter().map(Into::into).collect());
        self.rebuild_row_state();
        true
    }

    /// Remove and return a row by underlying data index.
    pub fn remove_row(&mut self, index: usize) -> Option<Vec<String>> {
        if index >= self.rows.len() {
            return None;
        }
        let row = self.rows.remove(index);
        self.rebuild_row_state();
        Some(row)
    }

    /// Remove all rows and reset cache-coupled selection and pagination state.
    pub fn clear_rows(&mut self) {
        self.rows.clear();
        self.rebuild_row_state();
    }

    /// Return the active filter text.
    pub fn filter(&self) -> &str {
        &self.filter
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
        let data_idx = self.view_indices.get(self.view_cursor())?;
        self.rows.get(*data_idx).map(|r| r.as_slice())
    }

    /// Data index (into [`rows`](TableState::rows)) of the focused row, or
    /// `None` when the filtered view is empty.
    ///
    /// Maps through the current filter, sort and (absolute) page view, so it
    /// is the index to pass to [`row`](TableState::row) or
    /// [`remove_row`](TableState::remove_row).
    ///
    /// # Example
    ///
    /// ```
    /// use slt::widgets::TableState;
    ///
    /// let mut table = TableState::new(vec!["Name"], vec![vec!["carol"], vec!["alice"]]);
    /// table.sort_by(0); // alice, carol
    /// assert_eq!(table.selected_view_index(), 0);
    /// assert_eq!(table.selected_index(), Some(1)); // "alice" is data row 1
    /// ```
    ///
    /// Available since `0.26.0`.
    pub fn selected_index(&self) -> Option<usize> {
        self.view_indices.get(self.view_cursor()).copied()
    }

    /// Position of the focused row in the filtered and sorted view (what the
    /// deprecated `selected` field holds). Multi-selection
    /// ([`is_row_selected`](TableState::is_row_selected)) and the
    /// [`table_with`](crate::Context::table_with) renderer use the same space.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::widgets::TableState;
    ///
    /// let table = TableState::new(vec!["Name"], vec![vec!["a"], vec!["b"]]);
    /// assert_eq!(table.selected_view_index(), 0);
    /// ```
    ///
    /// Available since `0.26.0`.
    pub fn selected_view_index(&self) -> usize {
        self.view_cursor()
    }

    /// Focus the row at data index `data`, switching to its page when
    /// pagination is enabled. Returns `false` (selection unchanged) when the
    /// index is out of range or hidden by the filter.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::widgets::TableState;
    ///
    /// let mut table = TableState::new(vec!["Name"], vec![vec!["carol"], vec!["alice"]]);
    /// table.sort_by(0);
    /// assert!(table.set_selected_index(0)); // "carol"
    /// assert_eq!(table.selected_view_index(), 1);
    /// ```
    ///
    /// Available since `0.26.0`.
    pub fn set_selected_index(&mut self, data: usize) -> bool {
        match self.view_indices.iter().position(|&raw| raw == data) {
            Some(view) => self.set_selected_view_index(view),
            None => false,
        }
    }

    /// Focus the row at position `view` of the filtered and sorted view,
    /// switching to its page when pagination is enabled. Returns `false`
    /// (selection unchanged) when `view` is past the end.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::widgets::TableState;
    ///
    /// let mut table = TableState::new(vec!["Name"], vec![vec!["carol"], vec!["alice"]]);
    /// table.sort_by(0);
    /// assert!(table.set_selected_view_index(1));
    /// assert_eq!(table.selected_row(), Some(&["carol".to_string()][..]));
    /// ```
    ///
    /// Available since `0.26.0`.
    pub fn set_selected_view_index(&mut self, view: usize) -> bool {
        if view >= self.view_indices.len() {
            return false;
        }
        *self.view_cursor_mut() = view;
        if let Some(page) = view.checked_div(self.page_size) {
            self.page = page;
        }
        true
    }

    /// Remove and return the focused row — the one the user sees
    /// highlighted — or `None` when the filtered view is empty.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::widgets::TableState;
    ///
    /// let mut table = TableState::new(vec!["Name"], vec![vec!["carol"], vec!["alice"]]);
    /// table.set_filter("alice");
    /// assert_eq!(table.remove_selected(), Some(vec!["alice".to_string()]));
    /// assert_eq!(table.row_count(), 1);
    /// ```
    ///
    /// Available since `0.26.0`.
    pub fn remove_selected(&mut self) -> Option<Vec<String>> {
        let data = self.selected_index()?;
        self.remove_row(data)
    }

    /// Storage accessors for the deprecated public `selected` field (#455);
    /// see [`ListState::view_cursor`].
    #[expect(
        deprecated,
        reason = "crate-internal storage access to the deprecated public selected field (#455)"
    )]
    #[inline]
    pub(crate) fn view_cursor(&self) -> usize {
        self.selected
    }

    #[expect(
        deprecated,
        reason = "crate-internal storage access to the deprecated public selected field (#455)"
    )]
    #[inline]
    pub(crate) fn view_cursor_mut(&mut self) -> &mut usize {
        &mut self.selected
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
    ///
    /// let mut table = TableState::new(
    ///     vec!["Name", "Note"],
    ///     vec![vec!["a", "a very long note that should be capped"]],
    /// );
    /// table.column_widths_spec(&[TableColumn::Fixed(6), TableColumn::Max(10)]);
    /// slt::run(|ui: &mut slt::Context| {
    ///     ui.table(&mut table);
    /// })?;
    /// # Ok::<_, std::io::Error>(())
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
    ///
    /// let mut table = TableState::new(
    ///     vec!["Name"],
    ///     vec![vec!["a"], vec!["b"]],
    /// );
    /// slt::run(|ui: &mut slt::Context| {
    ///     ui.table(&mut table);
    ///     for row in table.selected_rows() {
    ///         let _ = row;
    ///     }
    /// })?;
    /// # Ok::<_, std::io::Error>(())
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
    ///
    /// let mut table = TableState::new(vec!["Name"], vec![vec!["a"]]);
    /// slt::run(|ui: &mut slt::Context| {
    ///     ui.table(&mut table);
    ///     let _ = table.is_row_selected(0);
    /// })?;
    /// # Ok::<_, std::io::Error>(())
    /// ```
    pub fn is_row_selected(&self, view_idx: usize) -> bool {
        self.multi_selected.contains(&view_idx)
    }

    /// Clear the multi-selection set and the range anchor.
    ///
    /// The focused cursor row ([`selected_view_index`](TableState::selected_view_index)) is unaffected.
    ///
    /// Available since v0.21.0.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::widgets::TableState;
    ///
    /// let mut table = TableState::new(vec!["Name"], vec![vec!["a"]]);
    /// slt::run(|ui: &mut slt::Context| {
    ///     ui.table(&mut table);
    ///     table.clear_selection();
    /// })?;
    /// # Ok::<_, std::io::Error>(())
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

        *self.view_cursor_mut() = self
            .view_cursor()
            .min(self.view_indices.len().saturating_sub(1));
        self.prune_selection();
        self.widths_dirty = true;
    }

    fn rebuild_row_search_cache(&mut self) {
        self.row_search_cache = self
            .rows
            .iter()
            .map(|row| Self::searchable_row(row))
            .collect();
        self.filter_tokens = Self::tokenize_filter(&self.filter);
        self.widths_dirty = true;
    }

    fn searchable_row(row: &[String]) -> String {
        let mut searchable = String::new();
        for (idx, cell) in row.iter().enumerate() {
            if idx > 0 {
                searchable.push('\n');
            }
            searchable.extend(cell.chars().flat_map(char::to_lowercase));
        }
        searchable
    }

    fn rebuild_row_state(&mut self) {
        self.rebuild_row_search_cache();
        self.rebuild_view();
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
            let mut width = crate::width::str_width(header.as_str()) as u32;
            if self.sort_column == Some(i) {
                width += 2;
            }
            self.content_widths[i] = width;
        }
        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < col_count {
                    let w = crate::width::str_width(cell.as_str()) as u32;
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
            let spec = self
                .column_specs
                .get(i)
                .copied()
                .unwrap_or(TableColumn::Auto);
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

// ── Tree ──────────────────────────────────────────────────────────────

/// A node in a tree view.
#[derive(Debug, Clone)]
pub struct TreeNode {
    /// Display label for this node.
    pub label: String,
    /// Child nodes.
    pub children: Vec<TreeNode>,
    /// Whether the node is expanded in the tree view.
    pub expanded: bool,
}

impl TreeNode {
    /// Create a collapsed tree node with no children.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            children: Vec::new(),
            expanded: false,
        }
    }

    /// Mark this node as expanded.
    pub fn expanded(mut self) -> Self {
        self.expanded = true;
        self
    }

    /// Set child nodes for this node.
    pub fn children(mut self, children: Vec<TreeNode>) -> Self {
        self.children = children;
        self
    }

    /// Returns `true` when this node has no children.
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    fn flatten(&self, depth: usize, out: &mut Vec<FlatTreeEntry>) {
        out.push(FlatTreeEntry {
            depth,
            label: self.label.clone(),
            is_leaf: self.is_leaf(),
            expanded: self.expanded,
        });
        if self.expanded {
            for child in &self.children {
                child.flatten(depth + 1, out);
            }
        }
    }
}

pub(crate) struct FlatTreeEntry {
    pub depth: usize,
    pub label: String,
    pub is_leaf: bool,
    pub expanded: bool,
}

/// State for a hierarchical tree view widget.
#[derive(Debug, Clone)]
pub struct TreeState {
    /// Root nodes of the tree.
    pub nodes: Vec<TreeNode>,
    /// Selected row index in the flattened visible tree.
    pub selected: usize,
}

impl TreeState {
    /// Create tree state from root nodes.
    pub fn new(nodes: Vec<TreeNode>) -> Self {
        Self { nodes, selected: 0 }
    }

    pub(crate) fn flatten(&self) -> Vec<FlatTreeEntry> {
        let mut entries = Vec::new();
        for node in &self.nodes {
            node.flatten(0, &mut entries);
        }
        entries
    }

    pub(crate) fn toggle_at(&mut self, flat_index: usize) {
        let mut counter = 0usize;
        Self::toggle_recursive(&mut self.nodes, flat_index, &mut counter);
    }

    fn toggle_recursive(nodes: &mut [TreeNode], target: usize, counter: &mut usize) -> bool {
        for node in nodes.iter_mut() {
            if *counter == target {
                if !node.is_leaf() {
                    node.expanded = !node.expanded;
                }
                return true;
            }
            *counter += 1;
            if node.expanded && Self::toggle_recursive(&mut node.children, target, counter) {
                return true;
            }
        }
        false
    }
}

/// State for the directory tree widget.
#[derive(Debug, Clone)]
pub struct DirectoryTreeState {
    /// The underlying tree state (reuses existing TreeState).
    pub tree: TreeState,
    /// Whether to show file/folder icons.
    pub show_icons: bool,
}

impl DirectoryTreeState {
    /// Create directory tree state from root nodes.
    pub fn new(nodes: Vec<TreeNode>) -> Self {
        Self {
            tree: TreeState::new(nodes),
            show_icons: true,
        }
    }

    /// Build a directory tree from slash-delimited paths.
    pub fn from_paths(paths: &[&str]) -> Self {
        let mut roots: Vec<TreeNode> = Vec::new();

        for raw_path in paths {
            let parts: Vec<&str> = raw_path
                .split('/')
                .filter(|part| !part.is_empty())
                .collect();
            if parts.is_empty() {
                continue;
            }
            insert_path(&mut roots, &parts, 0);
        }

        Self::new(roots)
    }

    /// Return selected node label if a node is selected.
    pub fn selected_label(&self) -> Option<&str> {
        let mut cursor = 0usize;
        selected_label_in_nodes(&self.tree.nodes, self.tree.selected, &mut cursor)
    }
}

impl Default for DirectoryTreeState {
    fn default() -> Self {
        Self::new(Vec::<TreeNode>::new())
    }
}

fn insert_path(nodes: &mut Vec<TreeNode>, parts: &[&str], depth: usize) {
    let Some(label) = parts.get(depth) else {
        return;
    };

    let is_last = depth + 1 == parts.len();
    let idx = nodes
        .iter()
        .position(|node| node.label == *label)
        .unwrap_or_else(|| {
            let mut node = TreeNode::new(*label);
            if !is_last {
                node.expanded = true;
            }
            nodes.push(node);
            nodes.len() - 1
        });

    if is_last {
        return;
    }

    nodes[idx].expanded = true;
    insert_path(&mut nodes[idx].children, parts, depth + 1);
}

fn selected_label_in_nodes<'a>(
    nodes: &'a [TreeNode],
    target: usize,
    cursor: &mut usize,
) -> Option<&'a str> {
    for node in nodes {
        if *cursor == target {
            return Some(node.label.as_str());
        }
        *cursor += 1;
        if node.expanded
            && let Some(found) = selected_label_in_nodes(&node.children, target, cursor)
        {
            return Some(found);
        }
    }
    None
}

/// An absolute calendar date `(year, month 1–12, day 1–31)`.
///
/// Used by [`CalendarState`] to represent range endpoints that can span
/// month and year boundaries (a `selected_day` alone is scoped to the
/// currently displayed month). Available since `0.21.0`.
///
/// # Example
///
/// ```no_run
/// use slt::CalDate;
///
/// let d = CalDate { year: 2024, month: 12, day: 31 };
/// assert_eq!((d.year, d.month, d.day), (2024, 12, 31));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalDate {
    /// Calendar year.
    pub year: i32,
    /// Month of year, `1`–`12`.
    pub month: u32,
    /// Day of month, `1`–`31`.
    pub day: u32,
}

impl CalDate {
    /// Sort key ordering this date against another by year, then month, then day.
    fn key(&self) -> (i32, u32, u32) {
        (self.year, self.month, self.day)
    }
}

/// Selection behavior for [`CalendarState`].
///
/// Defaults to [`Single`](CalendarSelect::Single), preserving the original
/// single-date pick. Switch to [`Range`](CalendarSelect::Range) via
/// [`CalendarState::with_range`] for start/end range selection. Available
/// since `0.21.0`.
///
/// # Example
///
/// ```no_run
/// use slt::{CalendarSelect, CalendarState};
///
/// let mut cal = CalendarState::from_ym(2024, 3);
/// assert_eq!(cal.mode(), CalendarSelect::Single);
/// cal.with_range();
/// assert_eq!(cal.mode(), CalendarSelect::Range);
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CalendarSelect {
    /// Pick exactly one date (default).
    #[default]
    Single,
    /// Pick a start/end date range via Shift-extend.
    Range,
}

/// State for the calendar date picker widget.
#[derive(Debug, Clone)]
pub struct CalendarState {
    /// Current display year.
    pub year: i32,
    /// Current display month (1–12).
    pub month: u32,
    /// Currently selected day, if any (single-date mode).
    pub selected_day: Option<u32>,
    pub(crate) cursor_day: u32,
    pub(crate) mode: CalendarSelect,
    pub(crate) anchor: Option<CalDate>,
    pub(crate) extent: Option<CalDate>,
    pub(crate) time_enabled: bool,
    pub(crate) hour: u8,
    pub(crate) minute: u8,
}

impl CalendarState {
    /// Create a new `CalendarState` initialized to the current month.
    pub fn new() -> Self {
        let (year, month) = Self::current_year_month();
        Self::from_ym(year, month)
    }

    /// Create a `CalendarState` for a specific year and month.
    pub fn from_ym(year: i32, month: u32) -> Self {
        let month = month.clamp(1, 12);
        Self {
            year,
            month,
            selected_day: None,
            cursor_day: 1,
            mode: CalendarSelect::Single,
            anchor: None,
            extent: None,
            time_enabled: false,
            hour: 0,
            minute: 0,
        }
    }

    /// Enable date-range selection (start/end via Shift-extend).
    ///
    /// Single-date mode remains the default; call this to opt in. Returns
    /// `&mut Self` for chaining. Available since `0.21.0`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::CalendarState;
    ///
    /// let mut cal = CalendarState::from_ym(2024, 3);
    /// cal.with_range();
    /// ```
    pub fn with_range(&mut self) -> &mut Self {
        self.mode = CalendarSelect::Range;
        self
    }

    /// Enable hour/minute selection, rendered as `HH:MM` below the grid.
    ///
    /// Off by default — no time row is rendered unless enabled. Returns
    /// `&mut Self` for chaining. Available since `0.21.0`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::CalendarState;
    ///
    /// let mut cal = CalendarState::from_ym(2024, 3);
    /// cal.with_time();
    /// ```
    pub fn with_time(&mut self) -> &mut Self {
        self.time_enabled = true;
        self
    }

    /// The active selection mode (`Single` by default).
    ///
    /// Available since `0.21.0`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::{CalendarSelect, CalendarState};
    ///
    /// let cal = CalendarState::from_ym(2024, 3);
    /// assert_eq!(cal.mode(), CalendarSelect::Single);
    /// ```
    pub fn mode(&self) -> CalendarSelect {
        self.mode
    }

    /// Returns the selected date as `(year, month, day)`, if any.
    pub fn selected_date(&self) -> Option<(i32, u32, u32)> {
        self.selected_day.map(|day| (self.year, self.month, day))
    }

    /// The normalized selected range as `(start, end)` with `start <= end`.
    ///
    /// Returns `None` in single-date mode or until an anchor has been set in
    /// range mode. Endpoints are absolute [`CalDate`]s, so a range may span
    /// month or year boundaries. Available since `0.21.0`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::CalendarState;
    ///
    /// let mut cal = CalendarState::from_ym(2024, 3);
    /// cal.with_range();
    /// assert!(cal.selected_range().is_none());
    /// ```
    pub fn selected_range(&self) -> Option<(CalDate, CalDate)> {
        if self.mode != CalendarSelect::Range {
            return None;
        }
        let anchor = self.anchor?;
        let extent = self.extent.unwrap_or(anchor);
        if anchor.key() <= extent.key() {
            Some((anchor, extent))
        } else {
            Some((extent, anchor))
        }
    }

    /// The selected `(hour, minute)` when time is enabled, else `None`.
    ///
    /// Available since `0.21.0`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::CalendarState;
    ///
    /// let mut cal = CalendarState::from_ym(2024, 3);
    /// assert!(cal.selected_time().is_none());
    /// cal.with_time();
    /// assert_eq!(cal.selected_time(), Some((0, 0)));
    /// ```
    pub fn selected_time(&self) -> Option<(u8, u8)> {
        self.time_enabled.then_some((self.hour, self.minute))
    }

    /// The cursor day as an absolute [`CalDate`] in the displayed month.
    pub(crate) fn cursor_date(&self) -> CalDate {
        CalDate {
            year: self.year,
            month: self.month,
            day: self.cursor_day,
        }
    }

    /// Set the range anchor to the cursor, clearing any prior extent.
    pub(crate) fn set_anchor_to_cursor(&mut self) {
        let cur = self.cursor_date();
        self.anchor = Some(cur);
        self.extent = None;
    }

    /// Set the range extent endpoint to the cursor.
    ///
    /// If no anchor exists yet, the cursor becomes the anchor.
    pub(crate) fn extend_to_cursor(&mut self) {
        let cur = self.cursor_date();
        if self.anchor.is_none() {
            self.anchor = Some(cur);
        }
        self.extent = Some(cur);
    }

    /// Whether the given absolute date falls inside the selected range
    /// (inclusive of both endpoints).
    pub(crate) fn in_range(&self, d: CalDate) -> bool {
        match self.selected_range() {
            Some((start, end)) => start.key() <= d.key() && d.key() <= end.key(),
            None => false,
        }
    }

    /// Whether the given absolute date is one of the range endpoints.
    pub(crate) fn is_range_endpoint(&self, d: CalDate) -> bool {
        match self.selected_range() {
            Some((start, end)) => d == start || d == end,
            None => false,
        }
    }

    /// Navigate to the previous month.
    pub fn prev_month(&mut self) {
        if self.month == 1 {
            self.month = 12;
            self.year -= 1;
        } else {
            self.month -= 1;
        }
        self.clamp_days();
    }

    /// Navigate to the next month.
    pub fn next_month(&mut self) {
        if self.month == 12 {
            self.month = 1;
            self.year += 1;
        } else {
            self.month += 1;
        }
        self.clamp_days();
    }

    pub(crate) fn days_in_month(year: i32, month: u32) -> u32 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if Self::is_leap_year(year) {
                    29
                } else {
                    28
                }
            }
            _ => 30,
        }
    }

    pub(crate) fn first_weekday(year: i32, month: u32) -> u32 {
        let month = month.clamp(1, 12);
        let offsets = [0_i32, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
        let mut y = year;
        if month < 3 {
            y -= 1;
        }
        let sunday_based = (y + y / 4 - y / 100 + y / 400 + offsets[(month - 1) as usize] + 1) % 7;
        ((sunday_based + 6) % 7) as u32
    }

    fn clamp_days(&mut self) {
        let max_day = Self::days_in_month(self.year, self.month);
        self.cursor_day = self.cursor_day.clamp(1, max_day);
        if let Some(day) = self.selected_day {
            self.selected_day = Some(day.min(max_day));
        }
    }

    fn is_leap_year(year: i32) -> bool {
        (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
    }

    fn current_year_month() -> (i32, u32) {
        let Ok(duration) = SystemTime::now().duration_since(UNIX_EPOCH) else {
            return (1970, 1);
        };
        let days_since_epoch = (duration.as_secs() / 86_400) as i64;
        let (year, month, _) = Self::civil_from_days(days_since_epoch);
        (year, month)
    }

    fn civil_from_days(days_since_epoch: i64) -> (i32, u32, u32) {
        let z = days_since_epoch + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
        let mut year = (yoe as i32) + (era as i32) * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let month = (mp + if mp < 10 { 3 } else { -9 }) as u32;
        if month <= 2 {
            year += 1;
        }
        (year, month, day)
    }
}

impl Default for CalendarState {
    fn default() -> Self {
        Self::new()
    }
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
        let mut state =
            ListState::new(vec!["a", "b", "c", "d"]).with_item_heights(vec![2, 3, 4, 5]);
        state.set_items(vec!["x", "y"]);

        assert_eq!(state.item_height(0), 2);
        assert_eq!(state.item_height(1), 3);
        assert_eq!(state.item_height(2), 1);
        state.ensure_row_prefix();
        assert_eq!(state.row_prefix(), &[0, 2, 5]);
    }
}

#[cfg(test)]
mod list_state_reorder_tests {
    use super::ListState;

    #[test]
    fn move_item_forward_reorders_and_keeps_selection() {
        let mut state = ListState::new(vec!["a", "b", "c", "d"]);
        assert!(state.set_selected_view_index(0)); // "a"
        assert!(state.move_item(0, 2));
        assert_eq!(state.items, vec!["b", "c", "a", "d"]);
        // Selection follows the moved item.
        assert_eq!(state.selected_item(), Some("a"));
        assert_eq!(state.selected_view_index(), 2);
    }

    #[test]
    fn move_item_backward_reorders_and_keeps_selection() {
        let mut state = ListState::new(vec!["a", "b", "c", "d"]);
        assert!(state.set_selected_view_index(3)); // "d"
        assert!(state.move_item(3, 1));
        assert_eq!(state.items, vec!["a", "d", "b", "c"]);
        assert_eq!(state.selected_item(), Some("d"));
        assert_eq!(state.selected_view_index(), 1);
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
        assert!(state.set_selected_view_index(1));
        assert!(!state.move_item(1, 1));
        assert_eq!(state.items, vec!["a", "b", "c"]);
        assert_eq!(state.selected_view_index(), 1);
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
        assert!(state.set_selected_view_index(3)); // "d"
        assert!(state.move_item(0, 1)); // swap a/b; "d" stays last
        assert_eq!(state.items, vec!["b", "a", "c", "d"]);
        assert_eq!(state.selected_item(), Some("d"));
        assert_eq!(state.selected_view_index(), 3);
    }
}

#[cfg(test)]
mod v024_collection_state_tests {
    use super::*;

    fn heights(state: &ListState) -> Vec<u32> {
        (0..state.len()).map(|raw| state.item_height(raw)).collect()
    }

    #[test]
    fn mutation_metadata_tracks_full_and_partial_heights() {
        for initial in [vec![1, 10, 2], vec![1, 10]] {
            let mut state = ListState::new(vec!["a", "b", "c"]).with_item_heights(initial);
            let last = state.item_height(2);
            assert!(state.set_selected_view_index(1));
            state.remove_item(0);
            assert_eq!(heights(&state), [10, last]);
            assert_eq!(state.selected_item(), Some("b"));
            state.insert_item(1, "new");
            assert_eq!(heights(&state), [10, 1, last]);
            state.move_item(2, 0);
            assert_eq!(heights(&state), [last, 10, 1]);
            assert_eq!(state.selected_item(), Some("b"));
            state.push_item("tail");
            assert_eq!(heights(&state), [last, 10, 1, 1]);
            state.set_filter("b");
            state.ensure_row_prefix();
            assert_eq!(state.row_prefix(), [0, 10]);
            state.clear_items();
            state.set_filter("");
            state.push_item("fresh");
            state.ensure_row_prefix();
            assert_eq!(state.row_prefix(), [0, 1]);
        }
    }

    #[test]
    fn partial_height_move_from_unspecified_item_preserves_explicit_height() {
        let mut state = ListState::new(vec!["a", "b", "c"]).with_item_heights(vec![10]);
        state.move_item(2, 0);
        assert_eq!(heights(&state), [1, 10, 1]);
    }

    #[test]
    fn append_extends_warm_filtered_prefix_without_rebuild() {
        let mut state = ListState::new(vec!["match-a"]).with_item_heights(vec![7]);
        state.set_filter("match");
        state.ensure_row_prefix();
        state.push_item("hidden");
        state.push_item("match-b");
        assert!(!state.heights_dirty);
        assert_eq!(state.visible_indices(), [0, 2]);
        assert_eq!(state.row_prefix(), [0, 7, 8]);
        state.ensure_row_prefix();
        assert_eq!(state.row_prefix(), [0, 7, 8]);
    }

    #[test]
    fn table_unsorted_append_preserves_selection_and_width_cache() {
        let mut state = TableState::new(vec!["H"], vec![vec!["one"], vec!["two"]]);
        state.select_single(1);
        assert!(state.set_selected_view_index(1));
        state.push_row(vec!["longer"]);
        assert!(!state.is_dirty());
        assert_eq!(state.column_widths(), [6]);
        assert_eq!(state.selected_row().unwrap(), ["two"]);
        assert!(state.is_row_selected(1));
    }

    #[test]
    #[ignore = "collection retained-capacity measurements"]
    fn retained_capacity_workload() {
        for count in [1_000, 10_000, 100_000] {
            let mut state = ListState::new((0..count).map(|i| format!("item-{i:06}")).collect());
            state.set_item_heights(vec![2; count]);
            state.ensure_row_prefix();
            let bytes = std::mem::size_of::<ListState>()
                + (state.items.capacity() + state.item_search_cache.capacity())
                    * std::mem::size_of::<String>()
                + state
                    .items
                    .iter()
                    .chain(&state.item_search_cache)
                    .map(String::capacity)
                    .sum::<usize>()
                + state.view_indices.capacity() * std::mem::size_of::<usize>()
                + state.row_prefix.capacity() * std::mem::size_of::<u32>()
                + state.item_heights.as_ref().unwrap().capacity() * std::mem::size_of::<u32>();
            eprintln!("COLLECTION_RETAINED n={count} owned_capacity_bytes={bytes}");
        }
    }
}

#[cfg(test)]
mod v026_selection_index_tests {
    //! Issue #455: selected-item reads and removal use the data index of the
    //! row the user sees highlighted, with or without a filter or sort.
    use super::*;

    fn fruit() -> ListState {
        ListState::new(vec!["apple", "banana", "cherry"])
    }

    #[test]
    fn remove_selected_under_filter_removes_the_highlighted_item() {
        let mut list = fruit();
        list.set_filter("cherry");
        assert_eq!(list.selected_item(), Some("cherry"));
        assert_eq!(list.remove_selected().as_deref(), Some("cherry"));
        assert_eq!(list.items(), ["apple", "banana"]);
        // Nothing left in the filtered view.
        assert_eq!(list.remove_selected(), None);
        assert_eq!(list.items(), ["apple", "banana"]);
    }

    #[test]
    fn selected_index_maps_view_to_data_in_every_view_state() {
        let mut list = fruit();
        assert!(list.set_selected_index(2));
        assert_eq!(list.selected_view_index(), 2);
        assert_eq!(list.selected_index(), Some(list.visible_indices()[2]));

        list.set_filter("an"); // banana only
        assert_eq!(list.selected_view_index(), 0);
        assert_eq!(list.selected_index(), Some(1));
        assert_eq!(
            list.selected_index(),
            Some(list.visible_indices()[list.selected_view_index()])
        );

        list.set_filter("zzz");
        assert_eq!(list.selected_index(), None);

        let empty = ListState::new(Vec::<String>::new());
        assert_eq!(empty.selected_index(), None);
        assert_eq!(empty.selected_view_index(), 0);
    }

    #[test]
    fn set_selected_index_selects_only_visible_items() {
        let mut list = fruit();
        list.set_filter("a"); // apple, banana
        assert!(list.set_selected_index(1));
        assert_eq!(list.selected_item(), Some("banana"));
        assert!(!list.set_selected_index(2), "cherry is filtered out");
        assert_eq!(list.selected_item(), Some("banana"));
        assert!(!list.set_selected_index(99));
    }

    fn people() -> TableState {
        TableState::new(
            vec!["Name", "Age"],
            vec![vec!["carol", "31"], vec!["alice", "45"], vec!["bob", "27"]],
        )
    }

    fn row(cells: &[&str]) -> Vec<String> {
        cells.iter().map(|cell| cell.to_string()).collect()
    }

    #[test]
    fn table_remove_selected_under_filter_removes_the_highlighted_row() {
        let mut table = people();
        table.set_filter("bob");
        assert_eq!(table.selected_row(), Some(&row(&["bob", "27"])[..]));
        assert_eq!(table.remove_selected(), Some(row(&["bob", "27"])));
        assert_eq!(table.row_count(), 2);
        assert!(table.rows().iter().all(|r| r[0] != "bob"));
    }

    #[test]
    fn table_remove_selected_under_sort_removes_the_highlighted_row() {
        let mut table = people();
        table.sort_by(0); // alice, bob, carol
        assert!(table.set_selected_index(2)); // bob (data index 2)
        assert_eq!(table.selected_view_index(), 1);
        assert_eq!(table.selected_index(), Some(2));
        assert_eq!(table.remove_selected(), Some(row(&["bob", "27"])));
        let names: Vec<&str> = table.rows().iter().map(|r| r[0].as_str()).collect();
        assert_eq!(names, ["carol", "alice"]);
    }

    #[test]
    fn table_set_selected_index_moves_to_the_rows_page() {
        let mut table = TableState::new(
            vec!["N"],
            (0..10).map(|i| vec![i.to_string()]).collect::<Vec<_>>(),
        );
        table.page_size = 3;
        assert!(table.set_selected_index(7));
        assert_eq!(table.page, 2);
        assert_eq!(table.selected_index(), Some(7));
        table.set_filter("zzz");
        assert!(!table.set_selected_index(7));
        assert_eq!(table.selected_index(), None);
    }

    #[test]
    fn set_items_to_a_smaller_list_releases_index_buffers() {
        // #416: replacing 10k items with 2 used to keep ~10k-entry
        // view/prefix/height buffers alive for the life of the state.
        let mut list = ListState::new((0..10_000).map(|i| i.to_string()).collect::<Vec<_>>())
            .with_item_heights(vec![2; 10_000]);
        list.ensure_row_prefix();
        list.set_items(vec!["a", "b"]);
        assert!(list.view_indices.capacity() <= 66, "view kept its capacity");
        assert!(list.row_prefix.capacity() <= 66, "prefix kept its capacity");
        assert!(
            list.item_heights
                .as_ref()
                .is_some_and(|h| h.capacity() <= 66),
            "heights kept their capacity"
        );
        assert_eq!(list.visible_indices(), [0, 1]);
        list.ensure_row_prefix();
        assert_eq!(list.row_prefix(), [0, 2, 4]);
        // Filter churn keeps capacity (no reallocation per keystroke).
        let mut big = ListState::new((0..1_000).map(|i| i.to_string()).collect::<Vec<_>>());
        big.set_filter("999");
        assert!(big.view_indices.capacity() >= 1_000);
    }

    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig::with_cases(256))]

        #[test]
        fn remove_selected_always_removes_selected_item(
            items in proptest::collection::vec("[a-c]{1,3}", 0..12),
            filter in "[a-c]{0,2}",
            pick in 0usize..16,
        ) {
            let mut list = ListState::new(items.clone());
            list.set_filter(filter.clone());
            let visible = list.visible_indices().len().max(1);
            if let Some(&data) = list.visible_indices().get(pick % visible) {
                proptest::prop_assert!(list.set_selected_index(data));
            }
            let expected = list.selected_item().map(str::to_string);
            proptest::prop_assert_eq!(
                list.selected_index(),
                list.visible_indices().get(list.selected_view_index()).copied()
            );
            let removed = list.remove_selected();
            proptest::prop_assert_eq!(&removed, &expected);
            proptest::prop_assert_eq!(
                list.len(),
                items.len() - usize::from(expected.is_some())
            );

            let mut table = TableState::new(
                vec!["C"],
                items.iter().map(|item| vec![item.clone()]).collect::<Vec<_>>(),
            );
            table.sort_by(0);
            table.set_filter(filter);
            let visible = table.visible_indices().len().max(1);
            if let Some(&data) = table.visible_indices().get(pick % visible) {
                proptest::prop_assert!(table.set_selected_index(data));
            }
            let expected = table.selected_row().map(<[String]>::to_vec);
            proptest::prop_assert_eq!(table.remove_selected(), expected);
        }
    }
}
