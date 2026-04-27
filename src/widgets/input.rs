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
    /// Set by mutation arms; consumed by `textarea()` for change detection.
    pub(crate) dirty: bool,
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
            dirty: false,
        }
    }

    /// Return all lines joined with newline characters.
    pub fn value(&self) -> String {
        self.lines.join("\n")
    }

    /// Returns `true` if the contents were mutated since the last frame.
    ///
    /// Use this to drive "unsaved changes" prompts before navigation. The
    /// flag is cleared by `textarea()` once `Response.changed` has been
    /// reported for the frame.
    pub fn is_dirty(&self) -> bool {
        self.dirty
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
        self.dirty = true;
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
