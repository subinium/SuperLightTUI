use unicode_width::UnicodeWidthStr;

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
}

impl TextInputState {
    /// Create an empty text input state.
    pub fn new() -> Self {
        Self {
            value: String::new(),
            cursor: 0,
            placeholder: String::new(),
        }
    }

    /// Create a text input with placeholder text shown when the value is empty.
    pub fn with_placeholder(p: impl Into<String>) -> Self {
        Self {
            placeholder: p.into(),
            ..Self::new()
        }
    }
}

impl Default for TextInputState {
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
    /// Row index of the cursor (0-based).
    pub cursor_row: usize,
    /// Column index of the cursor within the current row (character index).
    pub cursor_col: usize,
}

impl TextareaState {
    /// Create an empty text area state with one blank line.
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
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
        };
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
        self.dirty = true;
        self.selected = self.selected.min(self.rows.len().saturating_sub(1));
    }

    /// Get the currently selected row data, or `None` if the table is empty.
    pub fn selected_row(&self) -> Option<&[String]> {
        self.rows.get(self.selected).map(|r| r.as_slice())
    }

    pub(crate) fn recompute_widths(&mut self) {
        let col_count = self.headers.len();
        self.column_widths = vec![0u32; col_count];
        for (i, header) in self.headers.iter().enumerate() {
            self.column_widths[i] = UnicodeWidthStr::width(header.as_str()) as u32;
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
