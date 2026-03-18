//! Widget state types passed to [`Context`](crate::Context) widget methods.
//!
//! Each interactive widget (text input, list, tabs, table, etc.) has a
//! corresponding state struct defined here. Create the state once, then pass
//! a `&mut` reference each frame.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use unicode_width::UnicodeWidthStr;

type FormValidator = fn(&str) -> Result<(), String>;
type TextInputValidator = Box<dyn Fn(&str) -> Result<(), String>>;

/// Accumulated static output lines for [`crate::run_static`].
///
/// Use [`println`](Self::println) to append lines above the dynamic inline TUI.
#[derive(Debug, Clone, Default)]
pub struct StaticOutput {
    lines: Vec<String>,
    new_lines: Vec<String>,
}

impl StaticOutput {
    /// Create an empty static output buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one line of static output.
    pub fn println(&mut self, line: impl Into<String>) {
        let line = line.into();
        self.lines.push(line.clone());
        self.new_lines.push(line);
    }

    /// Return all accumulated static lines.
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// Drain and return only lines added since the previous drain.
    pub fn drain_new(&mut self) -> Vec<String> {
        std::mem::take(&mut self.new_lines)
    }

    /// Clear all accumulated lines.
    pub fn clear(&mut self) {
        self.lines.clear();
        self.new_lines.clear();
    }
}

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
    /// Autocomplete candidates shown below the input.
    pub suggestions: Vec<String>,
    /// Highlighted index within the currently shown suggestions.
    pub suggestion_index: usize,
    /// Whether the suggestions popup should be rendered.
    pub show_suggestions: bool,
    /// Multiple validators that produce their own error messages.
    validators: Vec<TextInputValidator>,
    /// All current validation errors from all validators.
    validation_errors: Vec<String>,
}

impl std::fmt::Debug for TextInputState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextInputState")
            .field("value", &self.value)
            .field("cursor", &self.cursor)
            .field("placeholder", &self.placeholder)
            .field("max_length", &self.max_length)
            .field("validation_error", &self.validation_error)
            .field("masked", &self.masked)
            .field("suggestions", &self.suggestions)
            .field("suggestion_index", &self.suggestion_index)
            .field("show_suggestions", &self.show_suggestions)
            .field("validators_len", &self.validators.len())
            .field("validation_errors", &self.validation_errors)
            .finish()
    }
}

impl Clone for TextInputState {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            cursor: self.cursor,
            placeholder: self.placeholder.clone(),
            max_length: self.max_length,
            validation_error: self.validation_error.clone(),
            masked: self.masked,
            suggestions: self.suggestions.clone(),
            suggestion_index: self.suggestion_index,
            show_suggestions: self.show_suggestions,
            validators: Vec::new(),
            validation_errors: self.validation_errors.clone(),
        }
    }
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
            suggestions: Vec::new(),
            suggestion_index: 0,
            show_suggestions: false,
            validators: Vec::new(),
            validation_errors: Vec::new(),
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
    ///
    /// This is a backward-compatible shorthand that runs a single validator.
    /// For multiple validators, use [`add_validator`](Self::add_validator) and [`run_validators`](Self::run_validators).
    pub fn validate(&mut self, validator: impl Fn(&str) -> Result<(), String>) {
        self.validation_error = validator(&self.value).err();
    }

    /// Add a validator function that produces its own error message.
    ///
    /// Multiple validators can be added. Call [`run_validators`](Self::run_validators)
    /// to execute all validators and collect their errors.
    pub fn add_validator(&mut self, f: impl Fn(&str) -> Result<(), String> + 'static) {
        self.validators.push(Box::new(f));
    }

    /// Run all registered validators and collect their error messages.
    ///
    /// Updates `validation_errors` with all errors from all validators.
    /// Also updates `validation_error` to the first error for backward compatibility.
    pub fn run_validators(&mut self) {
        self.validation_errors.clear();
        for validator in &self.validators {
            if let Err(err) = validator(&self.value) {
                self.validation_errors.push(err);
            }
        }
        self.validation_error = self.validation_errors.first().cloned();
    }

    /// Get all current validation errors from all validators.
    pub fn errors(&self) -> &[String] {
        &self.validation_errors
    }

    /// Set autocomplete suggestions and reset popup state.
    pub fn set_suggestions(&mut self, suggestions: Vec<String>) {
        self.suggestions = suggestions;
        self.suggestion_index = 0;
        self.show_suggestions = !self.suggestions.is_empty();
    }

    /// Return suggestions that start with the current input (case-insensitive).
    pub fn matched_suggestions(&self) -> Vec<&str> {
        if self.value.is_empty() {
            return Vec::new();
        }
        let lower = self.value.to_lowercase();
        self.suggestions
            .iter()
            .filter(|s| s.to_lowercase().starts_with(&lower))
            .map(|s| s.as_str())
            .collect()
    }
}

impl Default for TextInputState {
    fn default() -> Self {
        Self::new()
    }
}

/// A single form field with label and validation.
#[derive(Debug, Default)]
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
#[derive(Debug)]
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
#[derive(Debug, Clone)]
pub struct ToastState {
    /// Active toast messages, ordered oldest-first.
    pub messages: Vec<ToastMessage>,
}

/// A single toast notification message.
#[derive(Debug, Clone)]
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

impl Default for ToastMessage {
    fn default() -> Self {
        Self {
            text: String::new(),
            level: ToastLevel::Info,
            created_tick: 0,
            duration_ticks: 30,
        }
    }
}

/// Severity level for a [`ToastMessage`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertLevel {
    /// Informational alert.
    Info,
    /// Success alert.
    Success,
    /// Warning alert.
    Warning,
    /// Error alert.
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
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone, Default)]
pub struct ListState {
    /// The list items as display strings.
    pub items: Vec<String>,
    /// Index of the currently selected item.
    pub selected: usize,
    /// Case-insensitive substring filter applied to list items.
    pub filter: String,
    view_indices: Vec<usize>,
}

impl ListState {
    /// Create a list with the given items. The first item is selected initially.
    pub fn new(items: Vec<impl Into<String>>) -> Self {
        let len = items.len();
        Self {
            items: items.into_iter().map(Into::into).collect(),
            selected: 0,
            filter: String::new(),
            view_indices: (0..len).collect(),
        }
    }

    /// Replace the list items and rebuild the view index.
    ///
    /// Use this instead of assigning `items` directly to ensure the internal
    /// filter/view state stays consistent.
    pub fn set_items(&mut self, items: Vec<impl Into<String>>) {
        self.items = items.into_iter().map(Into::into).collect();
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
                    tokens
                        .iter()
                        .all(|token| self.items[i].to_lowercase().contains(token.as_str()))
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
    /// Whether alternating row backgrounds are enabled.
    pub zebra: bool,
    view_indices: Vec<usize>,
}

impl Default for TableState {
    fn default() -> Self {
        Self {
            headers: Vec::new(),
            rows: Vec::new(),
            selected: 0,
            column_widths: Vec::new(),
            dirty: true,
            sort_column: None,
            sort_ascending: true,
            filter: String::new(),
            page: 0,
            page_size: 0,
            zebra: false,
            view_indices: Vec::new(),
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
            dirty: true,
            sort_column: None,
            sort_ascending: true,
            filter: String::new(),
            page: 0,
            page_size: 0,
            zebra: false,
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

    /// Set the filter string. Multiple space-separated tokens are AND'd
    /// together — all tokens must match across any cell in the same row.
    /// Empty string disables filtering.
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

        let tokens: Vec<String> = self
            .filter
            .split_whitespace()
            .map(|t| t.to_lowercase())
            .collect();
        if !tokens.is_empty() {
            indices.retain(|&idx| {
                let row = match self.rows.get(idx) {
                    Some(r) => r,
                    None => return false,
                };
                tokens.iter().all(|token| {
                    row.iter()
                        .any(|cell| cell.to_lowercase().contains(token.as_str()))
                })
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
#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct CalendarState {
    pub year: i32,
    pub month: u32,
    pub selected_day: Option<u32>,
    pub(crate) cursor_day: u32,
}

impl CalendarState {
    pub fn new() -> Self {
        let (year, month) = Self::current_year_month();
        Self::from_ym(year, month)
    }

    pub fn from_ym(year: i32, month: u32) -> Self {
        let month = month.clamp(1, 12);
        Self {
            year,
            month,
            selected_day: None,
            cursor_day: 1,
        }
    }

    pub fn selected_date(&self) -> Option<(i32, u32, u32)> {
        self.selected_day.map(|day| (self.year, self.month, day))
    }

    pub fn prev_month(&mut self) {
        if self.month == 1 {
            self.month = 12;
            self.year -= 1;
        } else {
            self.month -= 1;
        }
        self.clamp_days();
    }

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trend {
    /// Positive movement.
    Up,
    /// Negative movement.
    Down,
}

// ── Select / Dropdown ─────────────────────────────────────────────────

/// State for a dropdown select widget.
///
/// Renders as a single-line button showing the selected option. When activated,
/// expands into a vertical list overlay for picking an option.
#[derive(Debug, Clone, Default)]
pub struct SelectState {
    /// Selectable option labels.
    pub items: Vec<String>,
    /// Selected option index.
    pub selected: usize,
    /// Whether the dropdown list is currently open.
    pub open: bool,
    /// Placeholder text shown when `items` is empty.
    pub placeholder: String,
    cursor: usize,
}

impl SelectState {
    /// Create select state with the provided options.
    pub fn new(items: Vec<impl Into<String>>) -> Self {
        Self {
            items: items.into_iter().map(Into::into).collect(),
            selected: 0,
            open: false,
            placeholder: String::new(),
            cursor: 0,
        }
    }

    /// Set placeholder text shown when no item can be displayed.
    pub fn placeholder(mut self, p: impl Into<String>) -> Self {
        self.placeholder = p.into();
        self
    }

    /// Returns the currently selected item label, or `None` if empty.
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
#[derive(Debug, Clone, Default)]
pub struct RadioState {
    /// Radio option labels.
    pub items: Vec<String>,
    /// Selected option index.
    pub selected: usize,
}

impl RadioState {
    /// Create radio state with the provided options.
    pub fn new(items: Vec<impl Into<String>>) -> Self {
        Self {
            items: items.into_iter().map(Into::into).collect(),
            selected: 0,
        }
    }

    /// Returns the currently selected option label, or `None` if empty.
    pub fn selected_item(&self) -> Option<&str> {
        self.items.get(self.selected).map(String::as_str)
    }
}

// ── Multi-Select ──────────────────────────────────────────────────────

/// State for a multi-select list.
///
/// Like [`ListState`] but allows toggling multiple items with Space.
#[derive(Debug, Clone)]
pub struct MultiSelectState {
    /// Multi-select option labels.
    pub items: Vec<String>,
    /// Focused option index used for keyboard navigation.
    pub cursor: usize,
    /// Set of selected option indices.
    pub selected: HashSet<usize>,
}

impl MultiSelectState {
    /// Create multi-select state with the provided options.
    pub fn new(items: Vec<impl Into<String>>) -> Self {
        Self {
            items: items.into_iter().map(Into::into).collect(),
            cursor: 0,
            selected: HashSet::new(),
        }
    }

    /// Return selected item labels in ascending index order.
    pub fn selected_items(&self) -> Vec<&str> {
        let mut indices: Vec<usize> = self.selected.iter().copied().collect();
        indices.sort();
        indices
            .iter()
            .filter_map(|&i| self.items.get(i).map(String::as_str))
            .collect()
    }

    /// Toggle selection state for `index`.
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

// ── Command Palette ───────────────────────────────────────────────────

/// A single command entry in the palette.
#[derive(Debug, Clone)]
pub struct PaletteCommand {
    /// Primary command label.
    pub label: String,
    /// Supplemental command description.
    pub description: String,
    /// Optional keyboard shortcut hint.
    pub shortcut: Option<String>,
}

impl PaletteCommand {
    /// Create a new palette command.
    pub fn new(label: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: description.into(),
            shortcut: None,
        }
    }

    /// Set a shortcut hint displayed alongside the command.
    pub fn shortcut(mut self, s: impl Into<String>) -> Self {
        self.shortcut = Some(s.into());
        self
    }
}

/// State for a command palette overlay.
///
/// Renders as a modal with a search input and filtered command list.
#[derive(Debug, Clone)]
pub struct CommandPaletteState {
    /// Available commands.
    pub commands: Vec<PaletteCommand>,
    /// Current search query.
    pub input: String,
    /// Cursor index within `input`.
    pub cursor: usize,
    /// Whether the palette modal is open.
    pub open: bool,
    /// The last selected command index, set when the user confirms a selection.
    /// Check this after `response.changed` is true.
    pub last_selected: Option<usize>,
    selected: usize,
}

impl CommandPaletteState {
    /// Create command palette state from a command list.
    pub fn new(commands: Vec<PaletteCommand>) -> Self {
        Self {
            commands,
            input: String::new(),
            cursor: 0,
            open: false,
            last_selected: None,
            selected: 0,
        }
    }

    /// Toggle open/closed state and reset input when opening.
    pub fn toggle(&mut self) {
        self.open = !self.open;
        if self.open {
            self.input.clear();
            self.cursor = 0;
            self.selected = 0;
        }
    }

    fn fuzzy_score(pattern: &str, text: &str) -> Option<i32> {
        let pattern = pattern.trim();
        if pattern.is_empty() {
            return Some(0);
        }

        let text_chars: Vec<char> = text.chars().collect();
        let mut score = 0;
        let mut search_start = 0usize;
        let mut prev_match: Option<usize> = None;

        for p in pattern.chars() {
            let mut found = None;
            for (idx, ch) in text_chars.iter().enumerate().skip(search_start) {
                if ch.eq_ignore_ascii_case(&p) {
                    found = Some(idx);
                    break;
                }
            }

            let idx = found?;
            if prev_match.is_some_and(|prev| idx == prev + 1) {
                score += 3;
            } else {
                score += 1;
            }

            if idx == 0 {
                score += 2;
            } else {
                let prev = text_chars[idx - 1];
                let curr = text_chars[idx];
                if matches!(prev, ' ' | '_' | '-') || prev.is_uppercase() || curr.is_uppercase() {
                    score += 2;
                }
            }

            prev_match = Some(idx);
            search_start = idx + 1;
        }

        Some(score)
    }

    pub(crate) fn filtered_indices(&self) -> Vec<usize> {
        let query = self.input.trim();
        if query.is_empty() {
            return (0..self.commands.len()).collect();
        }

        let mut scored: Vec<(usize, i32)> = self
            .commands
            .iter()
            .enumerate()
            .filter_map(|(i, cmd)| {
                let mut haystack =
                    String::with_capacity(cmd.label.len() + cmd.description.len() + 1);
                haystack.push_str(&cmd.label);
                haystack.push(' ');
                haystack.push_str(&cmd.description);
                Self::fuzzy_score(query, &haystack).map(|score| (i, score))
            })
            .collect();

        if scored.is_empty() {
            let tokens: Vec<String> = query.split_whitespace().map(|t| t.to_lowercase()).collect();
            return self
                .commands
                .iter()
                .enumerate()
                .filter(|(_, cmd)| {
                    let label = cmd.label.to_lowercase();
                    let desc = cmd.description.to_lowercase();
                    tokens.iter().all(|token| {
                        label.contains(token.as_str()) || desc.contains(token.as_str())
                    })
                })
                .map(|(i, _)| i)
                .collect();
        }

        scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        scored.into_iter().map(|(idx, _)| idx).collect()
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
#[derive(Debug, Clone)]
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

/// State for a streaming markdown display.
///
/// Accumulates markdown chunks as they arrive from an LLM stream.
/// Pass to [`Context::streaming_markdown`](crate::Context::streaming_markdown) each frame.
#[derive(Debug, Clone)]
pub struct StreamingMarkdownState {
    /// The accumulated markdown content.
    pub content: String,
    /// Whether the stream is still receiving data.
    pub streaming: bool,
    /// Cursor blink state (for the typing indicator).
    pub cursor_visible: bool,
    /// Cursor animation tick counter.
    pub cursor_tick: u64,
    /// Whether the parser is currently inside a fenced code block.
    pub in_code_block: bool,
    /// Language label of the active fenced code block.
    pub code_block_lang: String,
}

impl StreamingMarkdownState {
    /// Create a new empty streaming markdown state.
    pub fn new() -> Self {
        Self {
            content: String::new(),
            streaming: false,
            cursor_visible: true,
            cursor_tick: 0,
            in_code_block: false,
            code_block_lang: String::new(),
        }
    }

    /// Append a markdown chunk (e.g., from an LLM stream delta).
    pub fn push(&mut self, chunk: &str) {
        self.content.push_str(chunk);
    }

    /// Start a new streaming session, clearing previous content.
    pub fn start(&mut self) {
        self.content.clear();
        self.streaming = true;
        self.cursor_visible = true;
        self.cursor_tick = 0;
        self.in_code_block = false;
        self.code_block_lang.clear();
    }

    /// Mark the stream as complete (hides the typing cursor).
    pub fn finish(&mut self) {
        self.streaming = false;
    }

    /// Clear all content and reset state.
    pub fn clear(&mut self) {
        self.content.clear();
        self.streaming = false;
        self.cursor_visible = true;
        self.cursor_tick = 0;
        self.in_code_block = false;
        self.code_block_lang.clear();
    }
}

impl Default for StreamingMarkdownState {
    fn default() -> Self {
        Self::new()
    }
}

/// Navigation stack state for multi-screen apps.
///
/// Tracks screen names in a push/pop stack while preserving the root screen.
/// Pass this state through your render closure and branch on [`ScreenState::current`].
#[derive(Debug, Clone)]
pub struct ScreenState {
    stack: Vec<String>,
}

impl ScreenState {
    /// Create a screen stack with an initial root screen.
    pub fn new(initial: impl Into<String>) -> Self {
        Self {
            stack: vec![initial.into()],
        }
    }

    /// Return the current screen name (top of the stack).
    pub fn current(&self) -> &str {
        self.stack
            .last()
            .expect("ScreenState always contains at least one screen")
            .as_str()
    }

    /// Push a new screen onto the stack.
    pub fn push(&mut self, name: impl Into<String>) {
        self.stack.push(name.into());
    }

    /// Pop the current screen, preserving the root screen.
    pub fn pop(&mut self) {
        if self.can_pop() {
            self.stack.pop();
        }
    }

    /// Return current stack depth.
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Return `true` if popping is allowed.
    pub fn can_pop(&self) -> bool {
        self.stack.len() > 1
    }

    /// Reset to only the root screen.
    pub fn reset(&mut self) {
        self.stack.truncate(1);
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
#[derive(Debug, Clone)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_output_accumulates_and_drains_new_lines() {
        let mut output = StaticOutput::new();
        output.println("Building crate...");
        output.println("Compiling foo v0.1.0");

        assert_eq!(
            output.lines(),
            &[
                "Building crate...".to_string(),
                "Compiling foo v0.1.0".to_string()
            ]
        );

        let first = output.drain_new();
        assert_eq!(
            first,
            vec![
                "Building crate...".to_string(),
                "Compiling foo v0.1.0".to_string()
            ]
        );
        assert!(output.drain_new().is_empty());

        output.println("Finished");
        assert_eq!(output.drain_new(), vec!["Finished".to_string()]);
    }

    #[test]
    fn static_output_clear_resets_all_buffers() {
        let mut output = StaticOutput::new();
        output.println("line");
        output.clear();

        assert!(output.lines().is_empty());
        assert!(output.drain_new().is_empty());
    }

    #[test]
    fn form_field_default_values() {
        let field = FormField::default();
        assert_eq!(field.label, "");
        assert_eq!(field.input.value, "");
        assert_eq!(field.input.cursor, 0);
        assert_eq!(field.error, None);
    }

    #[test]
    fn toast_message_default_values() {
        let msg = ToastMessage::default();
        assert_eq!(msg.text, "");
        assert!(matches!(msg.level, ToastLevel::Info));
        assert_eq!(msg.created_tick, 0);
        assert_eq!(msg.duration_ticks, 30);
    }

    #[test]
    fn list_state_default_values() {
        let state = ListState::default();
        assert!(state.items.is_empty());
        assert_eq!(state.selected, 0);
        assert_eq!(state.filter, "");
        assert_eq!(state.visible_indices(), &[]);
        assert_eq!(state.selected_item(), None);
    }

    #[test]
    fn file_entry_default_values() {
        let entry = FileEntry::default();
        assert_eq!(entry.name, "");
        assert_eq!(entry.path, PathBuf::new());
        assert!(!entry.is_dir);
        assert_eq!(entry.size, 0);
    }

    #[test]
    fn tabs_state_default_values() {
        let state = TabsState::default();
        assert!(state.labels.is_empty());
        assert_eq!(state.selected, 0);
        assert_eq!(state.selected_label(), None);
    }

    #[test]
    fn table_state_default_values() {
        let state = TableState::default();
        assert!(state.headers.is_empty());
        assert!(state.rows.is_empty());
        assert_eq!(state.selected, 0);
        assert_eq!(state.sort_column, None);
        assert!(state.sort_ascending);
        assert_eq!(state.filter, "");
        assert_eq!(state.page, 0);
        assert_eq!(state.page_size, 0);
        assert!(!state.zebra);
        assert_eq!(state.visible_indices(), &[]);
    }

    #[test]
    fn select_state_default_values() {
        let state = SelectState::default();
        assert!(state.items.is_empty());
        assert_eq!(state.selected, 0);
        assert!(!state.open);
        assert_eq!(state.placeholder, "");
        assert_eq!(state.selected_item(), None);
        assert_eq!(state.cursor(), 0);
    }

    #[test]
    fn radio_state_default_values() {
        let state = RadioState::default();
        assert!(state.items.is_empty());
        assert_eq!(state.selected, 0);
        assert_eq!(state.selected_item(), None);
    }

    #[test]
    fn text_input_state_default_uses_new() {
        let state = TextInputState::default();
        assert_eq!(state.value, "");
        assert_eq!(state.cursor, 0);
        assert_eq!(state.placeholder, "");
        assert_eq!(state.max_length, None);
        assert_eq!(state.validation_error, None);
        assert!(!state.masked);
    }

    #[test]
    fn tabs_state_new_sets_labels() {
        let state = TabsState::new(vec!["a", "b"]);
        assert_eq!(state.labels, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(state.selected, 0);
        assert_eq!(state.selected_label(), Some("a"));
    }

    #[test]
    fn list_state_new_selected_item_points_to_first_item() {
        let state = ListState::new(vec!["alpha", "beta"]);
        assert_eq!(state.items, vec!["alpha".to_string(), "beta".to_string()]);
        assert_eq!(state.selected, 0);
        assert_eq!(state.visible_indices(), &[0, 1]);
        assert_eq!(state.selected_item(), Some("alpha"));
    }

    #[test]
    fn select_state_placeholder_builder_sets_value() {
        let state = SelectState::new(vec!["one", "two"]).placeholder("Pick one");
        assert_eq!(state.items, vec!["one".to_string(), "two".to_string()]);
        assert_eq!(state.placeholder, "Pick one");
        assert_eq!(state.selected_item(), Some("one"));
    }

    #[test]
    fn radio_state_new_sets_items_and_selection() {
        let state = RadioState::new(vec!["red", "green"]);
        assert_eq!(state.items, vec!["red".to_string(), "green".to_string()]);
        assert_eq!(state.selected, 0);
        assert_eq!(state.selected_item(), Some("red"));
    }

    #[test]
    fn table_state_new_sets_sort_ascending_true() {
        let state = TableState::new(vec!["Name"], vec![vec!["Alice"], vec!["Bob"]]);
        assert_eq!(state.headers, vec!["Name".to_string()]);
        assert_eq!(state.rows.len(), 2);
        assert!(state.sort_ascending);
        assert_eq!(state.sort_column, None);
        assert!(!state.zebra);
        assert_eq!(state.visible_indices(), &[0, 1]);
    }

    #[test]
    fn command_palette_fuzzy_score_matches_gapped_pattern() {
        assert!(CommandPaletteState::fuzzy_score("sf", "Save File").is_some());
        assert!(CommandPaletteState::fuzzy_score("cmd", "Command Palette").is_some());
        assert_eq!(CommandPaletteState::fuzzy_score("xyz", "Save File"), None);
    }

    #[test]
    fn command_palette_filtered_indices_uses_fuzzy_and_sorts() {
        let mut state = CommandPaletteState::new(vec![
            PaletteCommand::new("Save File", "Write buffer"),
            PaletteCommand::new("Search Files", "Find in workspace"),
            PaletteCommand::new("Quit", "Exit app"),
        ]);

        state.input = "sf".to_string();
        let filtered = state.filtered_indices();
        assert_eq!(filtered, vec![0, 1]);

        state.input = "buffer".to_string();
        let filtered = state.filtered_indices();
        assert_eq!(filtered, vec![0]);
    }

    #[test]
    fn screen_state_push_pop_tracks_current_screen() {
        let mut screens = ScreenState::new("home");
        assert_eq!(screens.current(), "home");
        assert_eq!(screens.depth(), 1);
        assert!(!screens.can_pop());

        screens.push("settings");
        assert_eq!(screens.current(), "settings");
        assert_eq!(screens.depth(), 2);
        assert!(screens.can_pop());

        screens.push("profile");
        assert_eq!(screens.current(), "profile");
        assert_eq!(screens.depth(), 3);

        screens.pop();
        assert_eq!(screens.current(), "settings");
        assert_eq!(screens.depth(), 2);
    }

    #[test]
    fn screen_state_pop_never_removes_root() {
        let mut screens = ScreenState::new("home");
        screens.push("settings");
        screens.pop();
        screens.pop();

        assert_eq!(screens.current(), "home");
        assert_eq!(screens.depth(), 1);
        assert!(!screens.can_pop());
    }

    #[test]
    fn screen_state_reset_keeps_only_root() {
        let mut screens = ScreenState::new("home");
        screens.push("settings");
        screens.push("profile");
        assert_eq!(screens.current(), "profile");

        screens.reset();
        assert_eq!(screens.current(), "home");
        assert_eq!(screens.depth(), 1);
        assert!(!screens.can_pop());
    }

    #[test]
    fn calendar_days_in_month_handles_leap_years() {
        assert_eq!(CalendarState::days_in_month(2024, 2), 29);
        assert_eq!(CalendarState::days_in_month(2023, 2), 28);
        assert_eq!(CalendarState::days_in_month(2024, 1), 31);
        assert_eq!(CalendarState::days_in_month(2024, 4), 30);
    }

    #[test]
    fn calendar_first_weekday_known_dates() {
        assert_eq!(CalendarState::first_weekday(2024, 1), 0);
        assert_eq!(CalendarState::first_weekday(2023, 10), 6);
    }

    #[test]
    fn calendar_prev_next_month_handles_year_boundary() {
        let mut state = CalendarState::from_ym(2024, 12);
        state.prev_month();
        assert_eq!((state.year, state.month), (2024, 11));

        let mut state = CalendarState::from_ym(2024, 1);
        state.prev_month();
        assert_eq!((state.year, state.month), (2023, 12));

        state.next_month();
        assert_eq!((state.year, state.month), (2024, 1));
    }
}
