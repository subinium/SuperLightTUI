//! Widget state types passed to [`Context`](crate::Context) widget methods.
//!
//! Each interactive widget (text input, list, tabs, table, etc.) has a
//! corresponding state struct defined here. Create the state once, then pass
//! a `&mut` reference each frame.

use std::collections::HashSet;
use unicode_width::UnicodeWidthStr;

type FormValidator = fn(&str) -> Result<(), String>;

/// State for a single-line text input widget.
///
/// Pass a mutable reference to `Context::text_input` each frame. The widget
/// handles all keyboard events when focused.
///
/// # Example
///
/// ```no_run
/// # use slt::widgets::TextInputState;
/// # slt::run(|ui: &mut slt::Context| {
/// let mut input = TextInputState::with_placeholder("Type here...");
/// ui.text_input(&mut input);
/// println!("{}", input.value);
/// # });
/// ```
pub struct TextInputState {
    /// The current input text.
    pub value: String,
    /// Cursor position as a character index into `value`.
    pub cursor: usize,
    /// Placeholder text shown when `value` is empty.
    pub placeholder: String,
    /// Maximum character count. Input is rejected beyond this limit.
    pub max_length: Option<usize>,
    /// The most recent validation error message, if any.
    pub validation_error: Option<String>,
    /// When `true`, input is displayed as `•` characters (for passwords).
    pub masked: bool,
}

impl TextInputState {
    /// Create an empty text input state.
    pub fn new() -> Self {
        Self {
            value: String::new(),
            cursor: 0,
            placeholder: String::new(),
            max_length: None,
            validation_error: None,
            masked: false,
        }
    }

    /// Create a text input with placeholder text shown when the value is empty.
    pub fn with_placeholder(p: impl Into<String>) -> Self {
        Self {
            placeholder: p.into(),
            ..Self::new()
        }
    }

    /// Set the maximum allowed character count.
    pub fn max_length(mut self, len: usize) -> Self {
        self.max_length = Some(len);
        self
    }

    /// Validate the current value and store the latest error message.
    ///
    /// Sets [`TextInputState::validation_error`] to `None` when validation
    /// succeeds, or to `Some(error)` when validation fails.
    pub fn validate(&mut self, validator: impl Fn(&str) -> Result<(), String>) {
        self.validation_error = validator(&self.value).err();
    }
}

impl Default for TextInputState {
    fn default() -> Self {
        Self::new()
    }
}

/// A single form field with label and validation.
pub struct FormField {
    /// Field label shown above the input.
    pub label: String,
    /// Text input state for this field.
    pub input: TextInputState,
    /// Validation error shown below the input when present.
    pub error: Option<String>,
}

impl FormField {
    /// Create a new form field with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            input: TextInputState::new(),
            error: None,
        }
    }

    /// Set placeholder text for this field's input.
    pub fn placeholder(mut self, p: impl Into<String>) -> Self {
        self.input.placeholder = p.into();
        self
    }
}

/// State for a form with multiple fields.
pub struct FormState {
    /// Ordered list of form fields.
    pub fields: Vec<FormField>,
    /// Whether the form has been successfully submitted.
    pub submitted: bool,
}

impl FormState {
    /// Create an empty form state.
    pub fn new() -> Self {
        Self {
            fields: Vec::new(),
            submitted: false,
        }
    }

    /// Add a field and return the updated form for chaining.
    pub fn field(mut self, field: FormField) -> Self {
        self.fields.push(field);
        self
    }

    /// Validate all fields with the given validators.
    ///
    /// Returns `true` when all validations pass.
    pub fn validate(&mut self, validators: &[FormValidator]) -> bool {
        let mut all_valid = true;
        for (i, field) in self.fields.iter_mut().enumerate() {
            if let Some(validator) = validators.get(i) {
                match validator(&field.input.value) {
                    Ok(()) => field.error = None,
                    Err(msg) => {
                        field.error = Some(msg);
                        all_valid = false;
                    }
                }
            }
        }
        all_valid
    }

    /// Get field value by index.
    pub fn value(&self, index: usize) -> &str {
        self.fields
            .get(index)
            .map(|f| f.input.value.as_str())
            .unwrap_or("")
    }
}

impl Default for FormState {
    fn default() -> Self {
        Self::new()
    }
}

/// State for toast notification display.
///
/// Add messages with [`ToastState::info`], [`ToastState::success`],
/// [`ToastState::warning`], or [`ToastState::error`], then pass the state to
/// `Context::toast` each frame. Expired messages are removed automatically.
pub struct ToastState {
    /// Active toast messages, ordered oldest-first.
    pub messages: Vec<ToastMessage>,
}

/// A single toast notification message.
pub struct ToastMessage {
    /// The text content of the notification.
    pub text: String,
    /// Severity level, used to choose the display color.
    pub level: ToastLevel,
    /// The tick at which this message was created.
    pub created_tick: u64,
    /// How many ticks the message remains visible.
    pub duration_ticks: u64,
}

/// Severity level for a [`ToastMessage`].
pub enum ToastLevel {
    /// Informational message (primary color).
    Info,
    /// Success message (success color).
    Success,
    /// Warning message (warning color).
    Warning,
    /// Error message (error color).
    Error,
}

impl ToastState {
    /// Create an empty toast state with no messages.
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    /// Push an informational toast visible for 30 ticks.
    pub fn info(&mut self, text: impl Into<String>, tick: u64) {
        self.push(text, ToastLevel::Info, tick, 30);
    }

    /// Push a success toast visible for 30 ticks.
    pub fn success(&mut self, text: impl Into<String>, tick: u64) {
        self.push(text, ToastLevel::Success, tick, 30);
    }

    /// Push a warning toast visible for 50 ticks.
    pub fn warning(&mut self, text: impl Into<String>, tick: u64) {
        self.push(text, ToastLevel::Warning, tick, 50);
    }

    /// Push an error toast visible for 80 ticks.
    pub fn error(&mut self, text: impl Into<String>, tick: u64) {
        self.push(text, ToastLevel::Error, tick, 80);
    }

    /// Push a toast with a custom level and duration.
    pub fn push(
        &mut self,
        text: impl Into<String>,
        level: ToastLevel,
        tick: u64,
        duration_ticks: u64,
    ) {
        self.messages.push(ToastMessage {
            text: text.into(),
            level,
            created_tick: tick,
            duration_ticks,
        });
    }

    /// Remove all messages whose display duration has elapsed.
    ///
    /// Called automatically by `Context::toast` before rendering.
    pub fn cleanup(&mut self, current_tick: u64) {
        self.messages.retain(|message| {
            current_tick < message.created_tick.saturating_add(message.duration_ticks)
        });
    }
}

impl Default for ToastState {
    fn default() -> Self {
        Self::new()
    }
}

/// State for a multi-line text area widget.
///
/// Pass a mutable reference to `Context::textarea` each frame along with the
/// number of visible rows. The widget handles all keyboard events when focused.
pub struct TextareaState {
    /// The lines of text, one entry per line.
    pub lines: Vec<String>,
    /// Row index of the cursor (0-based, logical line).
    pub cursor_row: usize,
    /// Column index of the cursor within the current row (character index).
    pub cursor_col: usize,
    /// Maximum total character count across all lines.
    pub max_length: Option<usize>,
    /// When set, lines longer than this display-column width are soft-wrapped.
    pub wrap_width: Option<u32>,
    /// First visible visual line (managed internally by `textarea()`).
    pub scroll_offset: usize,
}

impl TextareaState {
    /// Create an empty text area state with one blank line.
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            max_length: None,
            wrap_width: None,
            scroll_offset: 0,
        }
    }

    /// Return all lines joined with newline characters.
    pub fn value(&self) -> String {
        self.lines.join("\n")
    }

    /// Replace the content with the given text, splitting on newlines.
    ///
    /// Resets the cursor to the beginning of the first line.
    pub fn set_value(&mut self, text: impl Into<String>) {
        let value = text.into();
        self.lines = value.split('\n').map(str::to_string).collect();
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0;
    }

    /// Set the maximum allowed total character count.
    pub fn max_length(mut self, len: usize) -> Self {
        self.max_length = Some(len);
        self
    }

    /// Enable soft word-wrap at the given display-column width.
    pub fn word_wrap(mut self, width: u32) -> Self {
        self.wrap_width = Some(width);
        self
    }
}

impl Default for TextareaState {
    fn default() -> Self {
        Self::new()
    }
}

/// State for an animated spinner widget.
///
/// Create with [`SpinnerState::dots`] or [`SpinnerState::line`], then pass to
/// `Context::spinner` each frame. The frame advances automatically with the
/// tick counter.
pub struct SpinnerState {
    chars: Vec<char>,
}

impl SpinnerState {
    /// Create a dots-style spinner using braille characters.
    ///
    /// Cycles through: `⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏`
    pub fn dots() -> Self {
        Self {
            chars: vec!['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'],
        }
    }

    /// Create a line-style spinner using ASCII characters.
    ///
    /// Cycles through: `| / - \`
    pub fn line() -> Self {
        Self {
            chars: vec!['|', '/', '-', '\\'],
        }
    }

    /// Return the spinner character for the given tick.
    pub fn frame(&self, tick: u64) -> char {
        if self.chars.is_empty() {
            return ' ';
        }
        self.chars[tick as usize % self.chars.len()]
    }
}

impl Default for SpinnerState {
    fn default() -> Self {
        Self::dots()
    }
}

/// State for a selectable list widget.
///
/// Pass a mutable reference to `Context::list` each frame. Up/Down arrow
/// keys (and `k`/`j`) move the selection when the widget is focused.
pub struct ListState {
    /// The list items as display strings.
    pub items: Vec<String>,
    /// Index of the currently selected item.
    pub selected: usize,
}

impl ListState {
    /// Create a list with the given items. The first item is selected initially.
    pub fn new(items: Vec<impl Into<String>>) -> Self {
        Self {
            items: items.into_iter().map(Into::into).collect(),
            selected: 0,
        }
    }

    /// Get the currently selected item text, or `None` if the list is empty.
    pub fn selected_item(&self) -> Option<&str> {
        self.items.get(self.selected).map(String::as_str)
    }
}

/// State for a tab navigation widget.
///
/// Pass a mutable reference to `Context::tabs` each frame. Left/Right arrow
/// keys cycle through tabs when the widget is focused.
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
pub struct TableState {
    /// Column header labels.
    pub headers: Vec<String>,
    /// Table rows, each a `Vec` of cell strings.
    pub rows: Vec<Vec<String>>,
    /// Index of the currently selected row.
    pub selected: usize,
    column_widths: Vec<u32>,
    dirty: bool,
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
    view_indices: Vec<usize>,
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
            dirty: true,
            sort_column: None,
            sort_ascending: true,
            filter: String::new(),
            page: 0,
            page_size: 0,
            view_indices: Vec::new(),
        };
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
        self.sort_column = Some(column);
        self.sort_ascending = true;
        self.rebuild_view();
    }

    /// Set the filter string. Empty string disables filtering.
    pub fn set_filter(&mut self, filter: impl Into<String>) {
        self.filter = filter.into();
        self.page = 0;
        self.rebuild_view();
    }

    /// Clear sorting.
    pub fn clear_sort(&mut self) {
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

        if !self.filter.is_empty() {
            let needle = self.filter.to_lowercase();
            indices.retain(|&idx| {
                self.rows
                    .get(idx)
                    .map(|row| {
                        row.iter()
                            .any(|cell| cell.to_lowercase().contains(needle.as_str()))
                    })
                    .unwrap_or(false)
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
                    _ => left.to_lowercase().cmp(&right.to_lowercase()),
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
        self.dirty = true;
    }

    pub(crate) fn recompute_widths(&mut self) {
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
        self.dirty = false;
    }

    pub(crate) fn column_widths(&self) -> &[u32] {
        &self.column_widths
    }

    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }
}

/// State for a scrollable container.
///
/// Pass a mutable reference to `Context::scrollable` each frame. The context
/// updates `offset` and the internal bounds automatically based on mouse wheel
/// and drag events.
pub struct ScrollState {
    /// Current vertical scroll offset in rows.
    pub offset: usize,
    content_height: u32,
    viewport_height: u32,
}

impl ScrollState {
    /// Create scroll state starting at offset 0.
    pub fn new() -> Self {
        Self {
            offset: 0,
            content_height: 0,
            viewport_height: 0,
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
}

impl Default for ScrollState {
    fn default() -> Self {
        Self::new()
    }
}

/// Visual variant for buttons.
///
/// Controls the color scheme used when rendering a button. Pass to
/// [`crate::Context::button_with`] to create styled button variants.
///
/// - `Default` — theme text color, primary when focused (same as `button()`)
/// - `Primary` — primary color background with contrasting text
/// - `Danger` — error/red color for destructive actions
/// - `Outline` — bordered appearance without fill
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ButtonVariant {
    /// Standard button style.
    #[default]
    Default,
    /// Filled button with primary background color.
    Primary,
    /// Filled button with error/danger background color.
    Danger,
    /// Bordered button without background fill.
    Outline,
}

// ── Select / Dropdown ─────────────────────────────────────────────────

/// State for a dropdown select widget.
///
/// Renders as a single-line button showing the selected option. When activated,
/// expands into a vertical list overlay for picking an option.
pub struct SelectState {
    pub items: Vec<String>,
    pub selected: usize,
    pub open: bool,
    pub placeholder: String,
    cursor: usize,
}

impl SelectState {
    pub fn new(items: Vec<impl Into<String>>) -> Self {
        Self {
            items: items.into_iter().map(Into::into).collect(),
            selected: 0,
            open: false,
            placeholder: String::new(),
            cursor: 0,
        }
    }

    pub fn placeholder(mut self, p: impl Into<String>) -> Self {
        self.placeholder = p.into();
        self
    }

    pub fn selected_item(&self) -> Option<&str> {
        self.items.get(self.selected).map(String::as_str)
    }

    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }

    pub(crate) fn set_cursor(&mut self, c: usize) {
        self.cursor = c;
    }
}

// ── Radio ─────────────────────────────────────────────────────────────

/// State for a radio button group.
///
/// Renders a vertical list of mutually-exclusive options with `●`/`○` markers.
pub struct RadioState {
    pub items: Vec<String>,
    pub selected: usize,
}

impl RadioState {
    pub fn new(items: Vec<impl Into<String>>) -> Self {
        Self {
            items: items.into_iter().map(Into::into).collect(),
            selected: 0,
        }
    }

    pub fn selected_item(&self) -> Option<&str> {
        self.items.get(self.selected).map(String::as_str)
    }
}

// ── Multi-Select ──────────────────────────────────────────────────────

/// State for a multi-select list.
///
/// Like [`ListState`] but allows toggling multiple items with Space.
pub struct MultiSelectState {
    pub items: Vec<String>,
    pub cursor: usize,
    pub selected: HashSet<usize>,
}

impl MultiSelectState {
    pub fn new(items: Vec<impl Into<String>>) -> Self {
        Self {
            items: items.into_iter().map(Into::into).collect(),
            cursor: 0,
            selected: HashSet::new(),
        }
    }

    pub fn selected_items(&self) -> Vec<&str> {
        let mut indices: Vec<usize> = self.selected.iter().copied().collect();
        indices.sort();
        indices
            .iter()
            .filter_map(|&i| self.items.get(i).map(String::as_str))
            .collect()
    }

    pub fn toggle(&mut self, index: usize) {
        if self.selected.contains(&index) {
            self.selected.remove(&index);
        } else {
            self.selected.insert(index);
        }
    }
}

// ── Tree ──────────────────────────────────────────────────────────────

/// A node in a tree view.
pub struct TreeNode {
    pub label: String,
    pub children: Vec<TreeNode>,
    pub expanded: bool,
}

impl TreeNode {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            children: Vec::new(),
            expanded: false,
        }
    }

    pub fn expanded(mut self) -> Self {
        self.expanded = true;
        self
    }

    pub fn children(mut self, children: Vec<TreeNode>) -> Self {
        self.children = children;
        self
    }

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
pub struct TreeState {
    pub nodes: Vec<TreeNode>,
    pub selected: usize,
}

impl TreeState {
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

// ── Command Palette ───────────────────────────────────────────────────

/// A single command entry in the palette.
pub struct PaletteCommand {
    pub label: String,
    pub description: String,
    pub shortcut: Option<String>,
}

impl PaletteCommand {
    pub fn new(label: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: description.into(),
            shortcut: None,
        }
    }

    pub fn shortcut(mut self, s: impl Into<String>) -> Self {
        self.shortcut = Some(s.into());
        self
    }
}

/// State for a command palette overlay.
///
/// Renders as a modal with a search input and filtered command list.
pub struct CommandPaletteState {
    pub commands: Vec<PaletteCommand>,
    pub input: String,
    pub cursor: usize,
    pub open: bool,
    selected: usize,
}

impl CommandPaletteState {
    pub fn new(commands: Vec<PaletteCommand>) -> Self {
        Self {
            commands,
            input: String::new(),
            cursor: 0,
            open: false,
            selected: 0,
        }
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
        if self.open {
            self.input.clear();
            self.cursor = 0;
            self.selected = 0;
        }
    }

    pub(crate) fn filtered_indices(&self) -> Vec<usize> {
        if self.input.is_empty() {
            return (0..self.commands.len()).collect();
        }
        let query = self.input.to_lowercase();
        self.commands
            .iter()
            .enumerate()
            .filter(|(_, cmd)| {
                cmd.label.to_lowercase().contains(&query)
                    || cmd.description.to_lowercase().contains(&query)
            })
            .map(|(i, _)| i)
            .collect()
    }

    pub(crate) fn selected(&self) -> usize {
        self.selected
    }

    pub(crate) fn set_selected(&mut self, s: usize) {
        self.selected = s;
    }
}

/// State for a streaming text display.
///
/// Accumulates text chunks as they arrive from an LLM stream.
/// Pass to [`Context::streaming_text`](crate::Context::streaming_text) each frame.
pub struct StreamingTextState {
    /// The accumulated text content.
    pub content: String,
    /// Whether the stream is still receiving data.
    pub streaming: bool,
    /// Cursor blink state (for the typing indicator).
    pub(crate) cursor_visible: bool,
    pub(crate) cursor_tick: u64,
}

impl StreamingTextState {
    /// Create a new empty streaming text state.
    pub fn new() -> Self {
        Self {
            content: String::new(),
            streaming: false,
            cursor_visible: true,
            cursor_tick: 0,
        }
    }

    /// Append a chunk of text (e.g., from an LLM stream delta).
    pub fn push(&mut self, chunk: &str) {
        self.content.push_str(chunk);
    }

    /// Mark the stream as complete (hides the typing cursor).
    pub fn finish(&mut self) {
        self.streaming = false;
    }

    /// Start a new streaming session, clearing previous content.
    pub fn start(&mut self) {
        self.content.clear();
        self.streaming = true;
        self.cursor_visible = true;
        self.cursor_tick = 0;
    }

    /// Clear all content and reset state.
    pub fn clear(&mut self) {
        self.content.clear();
        self.streaming = false;
        self.cursor_visible = true;
        self.cursor_tick = 0;
    }
}

impl Default for StreamingTextState {
    fn default() -> Self {
        Self::new()
    }
}

/// Approval state for a tool call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalAction {
    /// No action taken yet.
    Pending,
    /// User approved the tool call.
    Approved,
    /// User rejected the tool call.
    Rejected,
}

/// State for a tool approval widget.
///
/// Displays a tool call with approve/reject buttons for human-in-the-loop
/// AI workflows. Pass to [`Context::tool_approval`](crate::Context::tool_approval)
/// each frame.
pub struct ToolApprovalState {
    /// The name of the tool being invoked.
    pub tool_name: String,
    /// A human-readable description of what the tool will do.
    pub description: String,
    /// The current approval status.
    pub action: ApprovalAction,
}

impl ToolApprovalState {
    /// Create a new tool approval prompt.
    pub fn new(tool_name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            description: description.into(),
            action: ApprovalAction::Pending,
        }
    }

    /// Reset to pending state.
    pub fn reset(&mut self) {
        self.action = ApprovalAction::Pending;
    }
}

/// Item in a context bar showing active context sources.
#[derive(Debug, Clone)]
pub struct ContextItem {
    /// Display label for this context source.
    pub label: String,
    /// Token count or size indicator.
    pub tokens: usize,
}

impl ContextItem {
    /// Create a new context item with a label and token count.
    pub fn new(label: impl Into<String>, tokens: usize) -> Self {
        Self {
            label: label.into(),
            tokens,
        }
    }
}
