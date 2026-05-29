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
    /// # Clone behavior
    ///
    /// `validators` registered via [`TextInputState::add_validator`] are **not**
    /// cloned because closures are not `Clone`. `validation_errors` is preserved
    /// in the clone, but it becomes stale — calling
    /// [`TextInputState::run_validators`] on the clone will clear errors without
    /// re-running any validation.
    ///
    /// Re-register validators on the clone before calling `run_validators()`.
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
    ///
    /// # Note on cloning
    ///
    /// Validators are **not** preserved across [`Clone`] because closures are
    /// not `Clone`. Re-register after cloning the state.
    pub fn add_validator(&mut self, f: impl Fn(&str) -> Result<(), String> + 'static) {
        self.validators.push(Box::new(f));
    }

    /// Run all registered validators and collect their error messages.
    ///
    /// Updates `validation_errors` with all errors from all validators.
    /// Also updates `validation_error` to the first error for backward compatibility.
    ///
    /// # Note on cloning
    ///
    /// Validators do not survive [`Clone`]. Calling this on a cloned state with
    /// no re-registered validators clears `validation_errors` without re-running
    /// any check. Re-register validators on the clone first.
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

/// Severity level for alert widgets.
#[non_exhaustive]
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

/// Default maximum number of [`TextareaSnapshot`] entries kept in
/// [`TextareaState::history`]. Used by [`TextareaState::new`] and the
/// `Default` impl. Override per-instance via
/// [`TextareaState::history_max`].
pub(crate) const DEFAULT_TEXTAREA_HISTORY_MAX: usize = 100;

/// Snapshot of textarea content + cursor for the undo/redo history stack.
///
/// One snapshot is pushed before every destructive mutation (char insert,
/// delete, Enter, Backspace, paste). `Ctrl+Z` walks the index backward to a
/// previous snapshot; `Ctrl+Y` walks it forward.
///
/// Crate-internal — the `pub(crate)` visibility keeps the history layout an
/// implementation detail. Inspect via the public undo/redo behavior instead.
#[derive(Debug, Clone)]
pub(crate) struct TextareaSnapshot {
    /// Lines of text at the time of the snapshot.
    pub(crate) lines: Vec<String>,
    /// Cursor row at the time of the snapshot.
    pub(crate) cursor_row: usize,
    /// Cursor column at the time of the snapshot.
    pub(crate) cursor_col: usize,
}

/// State for a multi-line text area widget.
///
/// Pass a mutable reference to `Context::textarea` each frame along with the
/// number of visible rows. The widget handles all keyboard events when focused.
///
/// # Undo / redo
///
/// `Ctrl+Z` undoes the most recent edit and `Ctrl+Y` redoes it. The widget
/// pushes a snapshot before every destructive mutation (char insert, delete,
/// Enter, Backspace, paste). Rapid character typing coalesces into a single
/// undoable batch — only the first char of a typing burst pushes a snapshot.
/// History is capped at [`history_max`](Self::history_max) entries (default
/// `100`); the oldest snapshot is dropped when the cap is exceeded.
///
/// # Example
///
/// ```no_run
/// # use slt::widgets::TextareaState;
/// # slt::run(|ui: &mut slt::Context| {
/// let mut state = TextareaState::new();
/// // Type, then press Ctrl+Z to undo or Ctrl+Y to redo.
/// ui.textarea(&mut state, 5);
/// # });
/// ```
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
    /// Undo/redo snapshot stack. Newest entry is at the tip; the index walks
    /// backward on `Ctrl+Z` and forward on `Ctrl+Y`.
    pub(crate) history: Vec<TextareaSnapshot>,
    /// Pointer into [`history`](Self::history) for the next undo target.
    pub(crate) history_index: usize,
    /// Maximum [`history`](Self::history) length before the oldest snapshot is
    /// evicted. Defaults to [`DEFAULT_TEXTAREA_HISTORY_MAX`].
    pub(crate) history_max: usize,
    /// Whether the previous keypress was a `Char` insert. Used to coalesce
    /// rapid typing into a single undoable burst — when true, the next `Char`
    /// keypress does not push a snapshot.
    pub(crate) last_was_char_insert: bool,
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
            history: Vec::new(),
            history_index: 0,
            history_max: DEFAULT_TEXTAREA_HISTORY_MAX,
            last_was_char_insert: false,
        }
    }

    /// Return all lines joined with newline characters.
    pub fn value(&self) -> String {
        self.lines.join("\n")
    }

    /// Replace the content with the given text, splitting on newlines.
    ///
    /// Resets the cursor to the beginning of the first line and clears the
    /// undo history — programmatic replacement is treated as a fresh state,
    /// not an undoable edit.
    pub fn set_value(&mut self, text: impl Into<String>) {
        let value = text.into();
        self.lines = value.split('\n').map(str::to_string).collect();
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0;
        self.history.clear();
        self.history_index = 0;
        self.last_was_char_insert = false;
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

    /// Override the maximum number of undo snapshots kept (default `100`).
    ///
    /// When the history exceeds this cap the oldest snapshot is dropped.
    /// Setting `0` disables undo recording — the field is read every keypress.
    pub fn history_max(mut self, cap: usize) -> Self {
        self.history_max = cap;
        self
    }

    /// Number of undo snapshots currently retained.
    ///
    /// Read-only — useful for tests and debugging the history cap. The cap
    /// itself is set via [`history_max`](Self::history_max).
    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// Maximum number of undo snapshots retained.
    ///
    /// Mirrors [`history_max`](Self::history_max) (the builder setter) but as
    /// a getter — useful for tests asserting the cap stays bounded.
    pub fn history_cap(&self) -> usize {
        self.history_max
    }

    /// Push a snapshot of the current content + cursor onto the undo stack.
    ///
    /// Truncates any redo tail beyond `history_index`, appends the snapshot,
    /// and caps the stack at [`history_max`](Self::history_max) by dropping the
    /// oldest entry. `history_index` is left pointing one past the newest
    /// snapshot so the next `Ctrl+Z` returns to the just-pushed state.
    pub(crate) fn push_history(&mut self) {
        if self.history_max == 0 {
            return;
        }
        // Drop any redo tail — a fresh edit invalidates the redo branch.
        if self.history_index < self.history.len() {
            self.history.truncate(self.history_index);
        }
        self.history.push(TextareaSnapshot {
            lines: self.lines.clone(),
            cursor_row: self.cursor_row,
            cursor_col: self.cursor_col,
        });
        // Evict oldest when over the cap. `Vec::remove(0)` is O(n) but the
        // history cap is small (default 100) and this only runs at the cap
        // boundary, so the cost is bounded.
        while self.history.len() > self.history_max {
            self.history.remove(0);
        }
        self.history_index = self.history.len();
    }

    /// Walk the undo index back one step and apply the snapshot.
    ///
    /// No-op when the history is empty or already at the start. Returns `true`
    /// when a snapshot was applied.
    pub(crate) fn undo(&mut self) -> bool {
        if self.history.is_empty() || self.history_index == 0 {
            return false;
        }
        // First Ctrl+Z after edits captures the current (unsaved) tip so the
        // user can redo back to it; subsequent presses walk down the stack.
        if self.history_index == self.history.len() {
            self.history.push(TextareaSnapshot {
                lines: self.lines.clone(),
                cursor_row: self.cursor_row,
                cursor_col: self.cursor_col,
            });
        }
        self.history_index -= 1;
        let snap = &self.history[self.history_index];
        self.lines = snap.lines.clone();
        self.cursor_row = snap.cursor_row;
        self.cursor_col = snap.cursor_col;
        true
    }

    /// Walk the undo index forward one step and apply the snapshot.
    ///
    /// No-op when already at the redo tip. Returns `true` when a snapshot was
    /// applied.
    pub(crate) fn redo(&mut self) -> bool {
        if self.history_index + 1 >= self.history.len() {
            return false;
        }
        self.history_index += 1;
        let snap = &self.history[self.history_index];
        self.lines = snap.lines.clone();
        self.cursor_row = snap.cursor_row;
        self.cursor_col = snap.cursor_col;
        true
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
    chars: &'static [char],
}

static DOTS_CHARS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
static LINE_CHARS: &[char] = &['|', '/', '-', '\\'];

impl SpinnerState {
    /// Create a dots-style spinner using braille characters.
    ///
    /// Cycles through: `⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏`
    pub fn dots() -> Self {
        Self { chars: DOTS_CHARS }
    }

    /// Create a line-style spinner using ASCII characters.
    ///
    /// Cycles through: `| / - \`
    pub fn line() -> Self {
        Self { chars: LINE_CHARS }
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

/// State for a numeric stepper field (clamp + step, integer or float).
///
/// A numeric stepper renders the value as an editable field with `▾`/`▴`
/// affordances. When focused it adjusts via Up/Down (or `k`/`j`) and the scroll
/// wheel, or the user can type a value directly and press `Enter` to commit it.
/// The committed [`value`](NumberInputState::value) is always clamped into
/// `[min, max]` (and rounded to a whole number in integer mode).
///
/// Create with [`NumberInputState::new`] (float) or
/// [`NumberInputState::integer`], then pass to
/// [`Context::number_input`](crate::Context::number_input) each frame.
///
/// # Example
///
/// ```no_run
/// # use slt::widgets::NumberInputState;
/// # slt::run(|ui: &mut slt::Context| {
/// let mut qty = NumberInputState::integer(3, 0, 10).step(1.0);
/// let r = ui.number_input(&mut qty);
/// if r.changed {
///     // qty.value was adjusted this frame
/// }
/// # });
/// ```
///
/// Available since `0.21.0`.
#[derive(Debug, Clone)]
pub struct NumberInputState {
    /// Committed numeric value, always within `[min, max]`.
    pub value: f64,
    /// Inclusive lower bound.
    pub min: f64,
    /// Inclusive upper bound.
    pub max: f64,
    /// Increment applied per Up/Down/scroll tick.
    pub step: f64,
    /// When true, the value is whole-number only and rendered without a decimal point.
    pub integer: bool,
    /// In-progress typed text; `Some` while the user is editing the field.
    pub editing: Option<String>,
    /// Last parse failure from `Enter` on an invalid buffer, if any.
    pub parse_error: Option<String>,
}

impl NumberInputState {
    /// Float stepper with the given starting value and inclusive range.
    ///
    /// `value` is clamped into `[min, max]` immediately. If `min > max` the two
    /// bounds are swapped so the range is always well-formed.
    ///
    /// # Example
    ///
    /// ```
    /// # use slt::widgets::NumberInputState;
    /// let s = NumberInputState::new(1.5, 0.0, 10.0);
    /// assert_eq!(s.value, 1.5);
    /// assert!(!s.integer);
    /// ```
    pub fn new(value: f64, min: f64, max: f64) -> Self {
        let (min, max) = if min <= max { (min, max) } else { (max, min) };
        Self {
            value: value.clamp(min, max),
            min,
            max,
            step: 1.0,
            integer: false,
            editing: None,
            parse_error: None,
        }
    }

    /// Integer stepper (rounds value, renders without a decimal point).
    ///
    /// Convenience constructor that sets `integer = true` and a default step of
    /// `1.0`. `value` is clamped into `[min, max]`.
    ///
    /// # Example
    ///
    /// ```
    /// # use slt::widgets::NumberInputState;
    /// let s = NumberInputState::integer(42, 0, 100);
    /// assert_eq!(s.value, 42.0);
    /// assert!(s.integer);
    /// ```
    pub fn integer(value: i64, min: i64, max: i64) -> Self {
        let mut s = Self::new(value as f64, min as f64, max as f64);
        s.integer = true;
        s
    }

    /// Set the per-tick increment (consumes self, builder style).
    ///
    /// Negative or zero steps are coerced to `0.0` (no adjustment).
    ///
    /// # Example
    ///
    /// ```
    /// # use slt::widgets::NumberInputState;
    /// let s = NumberInputState::new(0.0, 0.0, 1.0).step(0.1);
    /// assert!((s.step - 0.1).abs() < f64::EPSILON);
    /// ```
    pub fn step(mut self, step: f64) -> Self {
        self.step = step.max(0.0);
        self
    }

    /// Clamp `value` into `[min, max]` (and round if `integer`).
    ///
    /// Used internally after every adjustment and typed commit, and exposed so
    /// callers that mutate [`value`](NumberInputState::value) directly can
    /// re-normalize it.
    ///
    /// # Example
    ///
    /// ```
    /// # use slt::widgets::NumberInputState;
    /// let mut s = NumberInputState::integer(0, 0, 10);
    /// s.value = 99.0;
    /// assert_eq!(s.clamped(), 10.0);
    /// s.value = 3.7;
    /// assert_eq!(s.clamped(), 4.0);
    /// ```
    pub fn clamped(&self) -> f64 {
        let v = self.value.clamp(self.min, self.max);
        if self.integer {
            v.round()
        } else {
            v
        }
    }
}

impl Default for NumberInputState {
    fn default() -> Self {
        Self::new(0.0, 0.0, 100.0)
    }
}
