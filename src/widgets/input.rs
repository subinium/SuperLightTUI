use unicode_segmentation::UnicodeSegmentation as _;

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
    /// Cursor position as a grapheme-cluster index into `value`.
    pub cursor: usize,
    /// Placeholder text shown when `value` is empty.
    pub placeholder: String,
    /// Maximum grapheme-cluster count. Input is rejected beyond this limit.
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

    /// Set the maximum allowed grapheme-cluster count.
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

/// A boxed, state-capturing field validator.
///
/// Unlike the deprecated [`FormValidator`] function pointer, a `Validator`
/// wraps a closure, so it can capture surrounding state — a compiled matcher,
/// a min/max pulled from config, or a sibling field's value. Built-in
/// constructors live in the [`validators`] module.
///
/// You rarely construct one directly: [`FormField::validate`] accepts a closure
/// and boxes it for you. Use [`Validator::new`] when you need to build a
/// `Validator` value yourself.
///
/// # Example
///
/// ```no_run
/// # use slt::widgets::Validator;
/// let min = 3usize; // captured state — impossible with a fn pointer
/// let v = Validator::new(move |s: &str| {
///     if s.len() >= min { Ok(()) } else { Err(format!("min {min} chars")) }
/// });
/// assert!(v.run("hello").is_ok());
/// assert!(v.run("hi").is_err());
/// ```
pub struct Validator(TextInputValidator);

impl Validator {
    /// Wrap a closure as a [`Validator`].
    ///
    /// The closure may capture state (it is `Box<dyn Fn>`, not a function
    /// pointer).
    pub fn new(f: impl Fn(&str) -> Result<(), String> + 'static) -> Self {
        Self(Box::new(f))
    }

    /// Run the validator against `value`, returning its `Err` message on
    /// failure.
    pub fn run(&self, value: &str) -> Result<(), String> {
        (self.0)(value)
    }
}

impl std::fmt::Debug for Validator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Validator(<fn>)")
    }
}

/// One in-flight asynchronous field validation.
///
/// Created by [`FormField::validate_async`] and polled each frame by
/// [`Context::form_field`](crate::Context::form_field) (or directly via
/// [`FormField::poll_async`]). Gated behind the `async` feature.
#[cfg(feature = "async")]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
pub struct AsyncValidation {
    rx: tokio::sync::oneshot::Receiver<Result<(), String>>,
    join: tokio::task::JoinHandle<()>,
}

#[cfg(feature = "async")]
impl std::fmt::Debug for AsyncValidation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AsyncValidation(<pending>)")
    }
}

#[cfg(feature = "async")]
impl Drop for AsyncValidation {
    fn drop(&mut self) {
        self.join.abort();
    }
}

/// When [`Context::form_field`](crate::Context::form_field) runs a field's
/// validators.
///
/// Defaults to [`OnBlur`](ValidateTrigger::OnBlur), matching the behavior of
/// `huh` and `bubbles/textinput`.
///
/// # Example
///
/// ```no_run
/// # use slt::widgets::{FormField, ValidateTrigger, validators};
/// let field = FormField::new("Email")
///     .validate(validators::email())
///     .on_change(); // validate as the user types
/// assert_eq!(field.trigger, ValidateTrigger::OnChange);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ValidateTrigger {
    /// Validate on every value change (each keystroke).
    OnChange,
    /// Validate when the field loses focus. The default.
    #[default]
    OnBlur,
    /// Never auto-validate; the app calls
    /// [`FormState::validate_all`] or [`FormField::run_validators`] manually.
    Manual,
}

/// A single form field with a label, an input, and its own validators.
///
/// Attach validators with the chainable [`validate`](Self::validate) builder
/// (multiple allowed); choose when they run with [`on_change`](Self::on_change)
/// / [`on_blur`](Self::on_blur). [`Context::form_field`](crate::Context::form_field)
/// runs them automatically per [`trigger`](Self::trigger).
///
/// # Example
///
/// ```no_run
/// # use slt::widgets::{FormField, validators};
/// let field = FormField::new("Email")
///     .placeholder("you@example.com")
///     .validate(validators::required("required"))
///     .validate(validators::email());
/// # let _ = field;
/// ```
#[derive(Debug, Default)]
pub struct FormField {
    /// Field label shown above the input.
    pub label: String,
    /// Text input state for this field.
    pub input: TextInputState,
    /// Validation error shown below the input when present.
    pub error: Option<String>,
    /// When the field's validators run. Defaults to
    /// [`ValidateTrigger::OnBlur`].
    pub trigger: ValidateTrigger,
    /// This field's validators. Mutate via [`validate`](Self::validate); run
    /// via [`run_validators`](Self::run_validators).
    validators: Vec<Validator>,
    /// Whether the field's input held keyboard focus on the previous frame.
    ///
    /// [`Context::form_field`](crate::Context::form_field) uses the
    /// focused → unfocused edge to detect blur for
    /// [`ValidateTrigger::OnBlur`]. This is tracked here (rather than read from
    /// the input's [`Response`]) because the `text_input` Response does not yet
    /// carry the `lost_focus` signal on its container-assembled response.
    was_focused: bool,
    /// One in-flight async validation, if any. Polled each frame by
    /// [`Context::form_field`](crate::Context::form_field).
    #[cfg(feature = "async")]
    pending: Option<AsyncValidation>,
}

impl FormField {
    /// Create a new form field with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            input: TextInputState::new(),
            error: None,
            trigger: ValidateTrigger::default(),
            validators: Vec::new(),
            was_focused: false,
            #[cfg(feature = "async")]
            pending: None,
        }
    }

    /// Set placeholder text for this field's input.
    pub fn placeholder(mut self, p: impl Into<String>) -> Self {
        self.input.placeholder = p.into();
        self
    }

    /// Attach a validator closure (chainable; call multiple times to stack
    /// validators — the first failure becomes the field error).
    ///
    /// The closure may capture state, unlike the deprecated positional
    /// [`FormValidator`]. Built-ins live in
    /// [`validators`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::widgets::{FormField, validators};
    /// let field = FormField::new("Name")
    ///     .validate(validators::required("required"))
    ///     .validate(validators::max_len(50, "too long"));
    /// # let _ = field;
    /// ```
    pub fn validate(mut self, f: impl Fn(&str) -> Result<(), String> + 'static) -> Self {
        self.validators.push(Validator::new(f));
        self
    }

    /// Run this field's validators on every change (each keystroke).
    pub fn on_change(mut self) -> Self {
        self.trigger = ValidateTrigger::OnChange;
        self
    }

    /// Run this field's validators when it loses focus (the default).
    pub fn on_blur(mut self) -> Self {
        self.trigger = ValidateTrigger::OnBlur;
        self
    }

    /// Disable automatic validation; the app must call
    /// [`run_validators`](Self::run_validators) or
    /// [`FormState::validate_all`] explicitly.
    pub fn manual(mut self) -> Self {
        self.trigger = ValidateTrigger::Manual;
        self
    }

    /// Number of validators attached to this field.
    pub fn validator_count(&self) -> usize {
        self.validators.len()
    }

    /// Run this field's validators now, setting [`error`](Self::error) to the
    /// first failure (or clearing it on success).
    ///
    /// Returns `true` when the field is valid.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::widgets::{FormField, validators};
    /// let mut field = FormField::new("Name").validate(validators::required("required"));
    /// assert!(!field.run_validators()); // empty -> error
    /// field.input.value = "Jane".into();
    /// assert!(field.run_validators()); // non-empty -> ok
    /// ```
    pub fn run_validators(&mut self) -> bool {
        self.error = self
            .validators
            .iter()
            .find_map(|v| v.run(&self.input.value).err());
        self.error.is_none()
    }

    /// Update the tracked focus edge and report whether the field *just* lost
    /// focus this frame (a focused → unfocused transition).
    ///
    /// Called by [`Context::form_field`](crate::Context::form_field) each frame
    /// with the input's current focus state. Kept crate-internal: blur
    /// detection is an implementation detail of the form-field trigger plumbing.
    pub(crate) fn observe_focus(&mut self, focused: bool) -> bool {
        let lost = self.was_focused && !focused;
        self.was_focused = focused;
        lost
    }

    /// Spawn an asynchronous validation of the current value, replacing any
    /// previously pending check.
    ///
    /// The future runs on the ambient tokio runtime; its `Result` is surfaced
    /// as [`error`](Self::error) once [`poll_async`](Self::poll_async) (called
    /// each frame by [`Context::form_field`](crate::Context::form_field)) sees
    /// it complete.
    ///
    /// Requires the `async` feature.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "async")]
    /// # async fn ex(field: &mut slt::widgets::FormField) {
    /// let value = field.input.value.clone();
    /// field.validate_async(async move {
    ///     // e.g. hit a "username taken?" endpoint
    ///     if value == "taken" { Err("already taken".into()) } else { Ok(()) }
    /// });
    /// # }
    /// ```
    #[cfg(feature = "async")]
    #[cfg_attr(docsrs, doc(cfg(feature = "async")))]
    pub fn validate_async<F>(&mut self, future: F)
    where
        F: std::future::Future<Output = Result<(), String>> + Send + 'static,
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let join = tokio::spawn(async move {
            let result = future.await;
            let _ = tx.send(result);
        });
        self.pending = Some(AsyncValidation { rx, join });
    }

    /// Whether an async validation is currently in flight.
    ///
    /// Requires the `async` feature.
    #[cfg(feature = "async")]
    #[cfg_attr(docsrs, doc(cfg(feature = "async")))]
    pub fn is_validating(&self) -> bool {
        self.pending.is_some()
    }

    /// Poll the in-flight async validation (if any) without blocking.
    ///
    /// When the future has resolved, its result is written to
    /// [`error`](Self::error) and the pending slot is cleared. Returns `true`
    /// when a result was just applied this call.
    ///
    /// Requires the `async` feature.
    #[cfg(feature = "async")]
    #[cfg_attr(docsrs, doc(cfg(feature = "async")))]
    pub fn poll_async(&mut self) -> bool {
        use tokio::sync::oneshot::error::TryRecvError;
        let Some(pending) = self.pending.as_mut() else {
            return false;
        };
        match pending.rx.try_recv() {
            Ok(result) => {
                self.error = result.err();
                self.pending = None;
                true
            }
            Err(TryRecvError::Empty) => false,
            Err(TryRecvError::Closed) => {
                // Sender dropped without sending — treat as resolved (no error
                // to surface) and clear the stuck pending slot.
                self.pending = None;
                true
            }
        }
    }
}

/// State for a form with multiple fields.
#[derive(Debug)]
pub struct FormState {
    /// Ordered list of form fields.
    pub fields: Vec<FormField>,
    /// Whether the form has been successfully submitted.
    pub submitted: bool,
    cross_field_errors: std::collections::HashMap<usize, String>,
}

impl FormState {
    /// Create an empty form state.
    pub fn new() -> Self {
        Self {
            fields: Vec::new(),
            submitted: false,
            cross_field_errors: std::collections::HashMap::new(),
        }
    }

    /// Add a field and return the updated form for chaining.
    pub fn field(mut self, field: FormField) -> Self {
        self.fields.push(field);
        self
    }

    /// Whether the form is currently valid — no field holds an error.
    ///
    /// Reflects the last run of each field's validators (auto-triggered by
    /// [`Context::form_field`](crate::Context::form_field) or run explicitly via
    /// [`validate_all`](Self::validate_all)). It does not re-run validation.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::widgets::{FormField, FormState, validators};
    /// let mut form = FormState::new().field(FormField::new("Name").validate(validators::required("required")));
    /// assert!(form.is_valid()); // no validation run yet
    /// form.validate_all();
    /// assert!(!form.is_valid()); // empty Name failed
    /// ```
    pub fn is_valid(&self) -> bool {
        self.fields.iter().all(|f| f.error.is_none())
    }

    /// Collect every current field error as `(field_index, message)` pairs.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::widgets::{FormField, FormState, validators};
    /// let mut form = FormState::new().field(FormField::new("Name").validate(validators::required("required")));
    /// form.validate_all();
    /// assert_eq!(form.errors(), vec![(0, "required")]);
    /// ```
    pub fn errors(&self) -> Vec<(usize, &str)> {
        self.fields
            .iter()
            .enumerate()
            .filter_map(|(i, f)| f.error.as_deref().map(|e| (i, e)))
            .collect()
    }

    /// Run every field's own validators, returning `true` when all pass.
    ///
    /// This is the replacement for the deprecated positional
    /// [`validate`](Self::validate) — validators are co-located with their
    /// fields, so there is no index slice to misalign.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::widgets::{FormField, FormState, validators};
    /// let mut form = FormState::new()
    ///     .field(FormField::new("Email").validate(validators::email()));
    /// let ok = form.validate_all();
    /// # let _ = ok;
    /// ```
    pub fn validate_all(&mut self) -> bool {
        let mut ok = true;
        for field in &mut self.fields {
            ok &= field.run_validators();
        }
        ok
    }

    /// Apply cross-field validation rules.
    ///
    /// The closure receives the whole form and returns `(field_index, message)`
    /// pairs; each pair sets that field's [`error`](FormField::error). Returns
    /// `true` when the closure reports no errors. Useful for rules like
    /// "confirm password must match password".
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::widgets::{FormField, FormState};
    /// let mut form = FormState::new()
    ///     .field(FormField::new("Password"))
    ///     .field(FormField::new("Confirm"));
    /// let ok = form.validate_with(|f| {
    ///     if f.value(0) != f.value(1) {
    ///         vec![(1, "passwords must match".to_string())]
    ///     } else {
    ///         vec![]
    ///     }
    /// });
    /// # let _ = ok;
    /// ```
    pub fn validate_with(&mut self, f: impl Fn(&FormState) -> Vec<(usize, String)>) -> bool {
        for (index, message) in self.cross_field_errors.drain() {
            if let Some(field) = self.fields.get_mut(index)
                && field.error.as_deref() == Some(message.as_str())
            {
                field.error = None;
            }
        }

        let extra = f(self);
        for (i, msg) in &extra {
            if let Some(field) = self.fields.get_mut(*i) {
                field.error = Some(msg.clone());
                self.cross_field_errors.insert(*i, msg.clone());
            }
        }
        extra.is_empty()
    }

    /// Validate all fields with a positional slice of function-pointer
    /// validators.
    ///
    /// Returns `true` when all validations pass. A field whose index has no
    /// matching validator is silently skipped.
    #[deprecated(
        since = "0.21.0",
        note = "Attach validators per-field via FormField::validate and call validate_all(); positional slices misalign silently."
    )]
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

#[cfg(all(test, feature = "async"))]
mod async_validation_tests {
    use super::FormField;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    #[tokio::test]
    async fn replacing_async_validation_aborts_previous_task() {
        let completed = Arc::new(AtomicBool::new(false));
        let completed_in_task = Arc::clone(&completed);
        let mut field = FormField::new("Username");

        field.validate_async(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            completed_in_task.store(true, Ordering::SeqCst);
            Ok(())
        });
        field.validate_async(async { Ok(()) });

        tokio::time::sleep(Duration::from_millis(120)).await;
        assert!(
            !completed.load(Ordering::SeqCst),
            "superseded validation task must be aborted"
        );
    }

    #[tokio::test]
    async fn dropping_field_aborts_pending_validation_task() {
        let completed = Arc::new(AtomicBool::new(false));
        let completed_in_task = Arc::clone(&completed);
        let mut field = FormField::new("Username");

        field.validate_async(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            completed_in_task.store(true, Ordering::SeqCst);
            Ok(())
        });
        drop(field);

        tokio::time::sleep(Duration::from_millis(120)).await;
        assert!(
            !completed.load(Ordering::SeqCst),
            "dropping a field must abort its pending validation task"
        );
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
    /// Column index of the cursor within the current row (grapheme-cluster index).
    pub cursor_col: usize,
    /// Maximum grapheme-cluster count, including logical newline separators.
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
    /// Live state captured by the first undo. Kept outside `history` so redo
    /// does not consume one of the bounded past-snapshot slots.
    pub(crate) redo_tip: Option<TextareaSnapshot>,
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
            redo_tip: None,
            last_was_char_insert: false,
        }
    }

    /// Return all lines joined with newline characters.
    pub fn value(&self) -> String {
        self.lines.join("\n")
    }

    /// Return the grapheme-cluster count, including logical newlines.
    pub fn grapheme_len(&self) -> usize {
        self.lines
            .iter()
            .map(|line| line.graphemes(true).count())
            .sum::<usize>()
            .saturating_add(self.lines.len().saturating_sub(1))
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
        self.redo_tip = None;
        self.last_was_char_insert = false;
    }

    /// Set the maximum grapheme-cluster count, including logical newlines.
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
        self.redo_tip = None;
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
        // Keep the live tip outside the bounded past-snapshot vector so the
        // first undo remains redoable even when `history_max == 1`.
        if self.history_index == self.history.len() {
            self.redo_tip = Some(TextareaSnapshot {
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
        if self.history_index < self.history.len().saturating_sub(1) {
            self.history_index += 1;
            let snap = &self.history[self.history_index];
            self.lines = snap.lines.clone();
            self.cursor_row = snap.cursor_row;
            self.cursor_col = snap.cursor_col;
            return true;
        }
        if self.history_index + 1 != self.history.len() {
            return false;
        }
        let Some(snap) = self.redo_tip.as_ref() else {
            return false;
        };
        self.history_index = self.history.len();
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

/// Named throbber preset for [`SpinnerState`].
///
/// Each variant maps to a fixed frame sequence (parity with the common
/// `cli-spinners` / `ratatui-throbber` sets). Construct a spinner from a preset
/// with [`SpinnerState::preset`], or use the matching named constructor such as
/// [`SpinnerState::moon`].
///
/// # Example
///
/// ```
/// # use slt::widgets::{SpinnerState, SpinnerPreset};
/// let s = SpinnerState::preset(SpinnerPreset::Arrow);
/// assert_eq!(s, SpinnerState::arrow());
/// ```
///
/// Available since `0.21.1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpinnerPreset {
    /// Braille dots: `⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏`.
    Dots,
    /// ASCII line: `| / - \`.
    Line,
    /// Moon phases: `🌑 🌒 🌓 🌔 🌕 🌖 🌗 🌘`.
    Moon,
    /// Bouncing bar between brackets: `(●    )` … `(    ●)` and back.
    Bounce,
    /// Quarter-circle arc: `◜ ◠ ◝ ◞ ◡ ◟`.
    Circle,
    /// Travelling braille dot: `⠁ ⠂ ⠄ ⡀ ⢀ ⠠ ⠐ ⠈`.
    Points,
    /// Half-circle arc: `◜ ◠ ◝ ◞ ◡ ◟`.
    Arc,
    /// Toggle pulse: `⊶ ⊷`.
    Toggle,
    /// Clockwise arrow: `← ↖ ↑ ↗ → ↘ ↓ ↙`.
    Arrow,
}

/// State for an animated spinner widget.
///
/// Create with a named constructor such as [`SpinnerState::dots`] or
/// [`SpinnerState::line`] (or from a [`SpinnerPreset`] via
/// [`SpinnerState::preset`]), then pass to `Context::spinner` each frame. The
/// frame advances automatically with the tick counter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpinnerState {
    chars: &'static [char],
}

static DOTS_CHARS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
static LINE_CHARS: &[char] = &['|', '/', '-', '\\'];
static MOON_CHARS: &[char] = &['🌑', '🌒', '🌓', '🌔', '🌕', '🌖', '🌗', '🌘'];
static BOUNCE_CHARS: &[char] = &['⠁', '⠂', '⠄', '⠂'];
static CIRCLE_CHARS: &[char] = &['◜', '◠', '◝', '◞', '◡', '◟'];
static POINTS_CHARS: &[char] = &['⠁', '⠂', '⠄', '⡀', '⢀', '⠠', '⠐', '⠈'];
static ARC_CHARS: &[char] = &['◜', '◠', '◝', '◞', '◡', '◟'];
static TOGGLE_CHARS: &[char] = &['⊶', '⊷'];
static ARROW_CHARS: &[char] = &['←', '↖', '↑', '↗', '→', '↘', '↓', '↙'];

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

    /// Create a moon-phase spinner.
    ///
    /// Cycles through: `🌑 🌒 🌓 🌔 🌕 🌖 🌗 🌘`
    ///
    /// Available since `0.21.1`.
    pub fn moon() -> Self {
        Self { chars: MOON_CHARS }
    }

    /// Create a bouncing single-dot spinner.
    ///
    /// Cycles through `⠁ ⠂ ⠄ ⠂`, giving a dot that rises and falls in place.
    ///
    /// Available since `0.21.1`.
    pub fn bounce() -> Self {
        Self {
            chars: BOUNCE_CHARS,
        }
    }

    /// Create a quarter-circle arc spinner.
    ///
    /// Cycles through: `◜ ◠ ◝ ◞ ◡ ◟`
    ///
    /// Available since `0.21.1`.
    pub fn circle() -> Self {
        Self {
            chars: CIRCLE_CHARS,
        }
    }

    /// Create a travelling braille-dot ("points") spinner.
    ///
    /// Cycles through: `⠁ ⠂ ⠄ ⡀ ⢀ ⠠ ⠐ ⠈`
    ///
    /// Available since `0.21.1`.
    pub fn points() -> Self {
        Self {
            chars: POINTS_CHARS,
        }
    }

    /// Create a half-circle arc spinner.
    ///
    /// Cycles through: `◜ ◠ ◝ ◞ ◡ ◟`
    ///
    /// Available since `0.21.1`.
    pub fn arc() -> Self {
        Self { chars: ARC_CHARS }
    }

    /// Create a two-frame toggle/pulse spinner.
    ///
    /// Cycles through: `⊶ ⊷`
    ///
    /// Available since `0.21.1`.
    pub fn toggle() -> Self {
        Self {
            chars: TOGGLE_CHARS,
        }
    }

    /// Create a rotating-arrow spinner.
    ///
    /// Cycles clockwise through: `← ↖ ↑ ↗ → ↘ ↓ ↙`
    ///
    /// Available since `0.21.1`.
    pub fn arrow() -> Self {
        Self { chars: ARROW_CHARS }
    }

    /// Create a spinner from a named [`SpinnerPreset`].
    ///
    /// Equivalent to calling the matching named constructor.
    ///
    /// # Example
    ///
    /// ```
    /// # use slt::widgets::{SpinnerState, SpinnerPreset};
    /// let s = SpinnerState::preset(SpinnerPreset::Moon);
    /// assert_eq!(s, SpinnerState::moon());
    /// ```
    ///
    /// Available since `0.21.1`.
    pub fn preset(preset: SpinnerPreset) -> Self {
        match preset {
            SpinnerPreset::Dots => Self::dots(),
            SpinnerPreset::Line => Self::line(),
            SpinnerPreset::Moon => Self::moon(),
            SpinnerPreset::Bounce => Self::bounce(),
            SpinnerPreset::Circle => Self::circle(),
            SpinnerPreset::Points => Self::points(),
            SpinnerPreset::Arc => Self::arc(),
            SpinnerPreset::Toggle => Self::toggle(),
            SpinnerPreset::Arrow => Self::arrow(),
        }
    }

    /// Number of distinct frames in this spinner's cycle.
    ///
    /// Useful for tests and for detecting wrap-around.
    ///
    /// # Example
    ///
    /// ```
    /// # use slt::widgets::SpinnerState;
    /// assert_eq!(SpinnerState::line().frame_count(), 4);
    /// ```
    ///
    /// Available since `0.21.1`.
    pub fn frame_count(&self) -> usize {
        self.chars.len()
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

const FINITE_NUMERIC_LIMIT: f64 = f64::MAX / 4.0;

pub(crate) fn normalize_numeric_range(start: f64, end: f64) -> (f64, f64) {
    let normalize_bound = |value: f64| {
        if value.is_nan() {
            0.0
        } else if value == f64::INFINITY {
            FINITE_NUMERIC_LIMIT
        } else if value == f64::NEG_INFINITY {
            -FINITE_NUMERIC_LIMIT
        } else {
            value.clamp(-FINITE_NUMERIC_LIMIT, FINITE_NUMERIC_LIMIT)
        }
    };
    let start = normalize_bound(start);
    let end = normalize_bound(end);
    if start <= end {
        (start, end)
    } else {
        (end, start)
    }
}

pub(crate) fn normalize_numeric_value(value: f64, min: f64, max: f64) -> f64 {
    let value = if value.is_nan() {
        0.0
    } else if value == f64::INFINITY {
        max
    } else if value == f64::NEG_INFINITY {
        min
    } else {
        value.clamp(-FINITE_NUMERIC_LIMIT, FINITE_NUMERIC_LIMIT)
    };
    value.clamp(min, max)
}

pub(crate) fn normalize_numeric_step(step: f64) -> f64 {
    if step.is_finite() && step > 0.0 {
        step.min(FINITE_NUMERIC_LIMIT)
    } else {
        0.0
    }
}

/// Optional configuration for [`Context::slider_with`](crate::Context::slider_with).
#[derive(Debug, Clone)]
pub struct SliderOpts {
    pub(crate) label: String,
    pub(crate) range: std::ops::RangeInclusive<f64>,
    pub(crate) step: Option<f64>,
}

impl SliderOpts {
    /// Create slider options with an automatic step of one twentieth of the span.
    pub fn new(label: impl Into<String>, range: std::ops::RangeInclusive<f64>) -> Self {
        Self {
            label: label.into(),
            range,
            step: None,
        }
    }

    /// Set an explicit finite positive keyboard step.
    pub fn step(mut self, step: f64) -> Self {
        self.step = Some(normalize_numeric_step(step));
        self
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
        let (min, max) = normalize_numeric_range(min, max);
        Self {
            value: normalize_numeric_value(value, min, max),
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
        self.step = normalize_numeric_step(step);
        self
    }

    /// Normalize bounds, value, and step after direct public-field mutation.
    pub fn normalize(&mut self) {
        (self.min, self.max) = normalize_numeric_range(self.min, self.max);
        self.value = normalize_numeric_value(self.value, self.min, self.max);
        self.step = normalize_numeric_step(self.step);
        if self.integer {
            self.value = self.value.round().clamp(self.min, self.max);
        }
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
        let (min, max) = normalize_numeric_range(self.min, self.max);
        let v = normalize_numeric_value(self.value, min, max);
        if self.integer {
            v.round().clamp(min, max)
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

#[cfg(test)]
mod spinner_tests {
    use super::{SpinnerPreset, SpinnerState};

    /// Collect one full cycle of frames for a spinner.
    fn cycle(s: &SpinnerState) -> Vec<char> {
        (0..s.frame_count() as u64).map(|t| s.frame(t)).collect()
    }

    #[test]
    fn existing_presets_unchanged() {
        // dots() and line() must keep their historic sequences.
        assert_eq!(
            cycle(&SpinnerState::dots()),
            vec!['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏']
        );
        assert_eq!(cycle(&SpinnerState::line()), vec!['|', '/', '-', '\\']);
        // Default stays dots().
        assert_eq!(SpinnerState::default(), SpinnerState::dots());
    }

    #[test]
    fn new_presets_have_expected_lengths() {
        assert_eq!(SpinnerState::dots().frame_count(), 10);
        assert_eq!(SpinnerState::line().frame_count(), 4);
        assert_eq!(SpinnerState::moon().frame_count(), 8);
        assert_eq!(SpinnerState::bounce().frame_count(), 4);
        assert_eq!(SpinnerState::circle().frame_count(), 6);
        assert_eq!(SpinnerState::points().frame_count(), 8);
        assert_eq!(SpinnerState::arc().frame_count(), 6);
        assert_eq!(SpinnerState::toggle().frame_count(), 2);
        assert_eq!(SpinnerState::arrow().frame_count(), 8);
    }

    #[test]
    fn new_presets_yield_expected_sequences() {
        assert_eq!(
            cycle(&SpinnerState::moon()),
            vec!['🌑', '🌒', '🌓', '🌔', '🌕', '🌖', '🌗', '🌘']
        );
        assert_eq!(cycle(&SpinnerState::bounce()), vec!['⠁', '⠂', '⠄', '⠂']);
        assert_eq!(
            cycle(&SpinnerState::circle()),
            vec!['◜', '◠', '◝', '◞', '◡', '◟']
        );
        assert_eq!(
            cycle(&SpinnerState::points()),
            vec!['⠁', '⠂', '⠄', '⡀', '⢀', '⠠', '⠐', '⠈']
        );
        assert_eq!(
            cycle(&SpinnerState::arc()),
            vec!['◜', '◠', '◝', '◞', '◡', '◟']
        );
        assert_eq!(cycle(&SpinnerState::toggle()), vec!['⊶', '⊷']);
        assert_eq!(
            cycle(&SpinnerState::arrow()),
            vec!['←', '↖', '↑', '↗', '→', '↘', '↓', '↙']
        );
    }

    #[test]
    fn frame_cycles_modulo_length() {
        let s = SpinnerState::arrow();
        let n = s.frame_count() as u64;
        // Tick 0 and one full revolution later yield the same frame.
        assert_eq!(s.frame(0), s.frame(n));
        assert_eq!(s.frame(1), s.frame(n + 1));
        // Wrap-around at the boundary.
        assert_eq!(s.frame(n - 1), '↙');
        assert_eq!(s.frame(n), '←');
    }

    #[test]
    fn frame_advances_through_sequence() {
        let s = SpinnerState::toggle();
        assert_eq!(s.frame(0), '⊶');
        assert_eq!(s.frame(1), '⊷');
        assert_eq!(s.frame(2), '⊶');
        assert_eq!(s.frame(3), '⊷');
    }

    #[test]
    fn preset_matches_named_constructor() {
        let cases = [
            (SpinnerPreset::Dots, SpinnerState::dots()),
            (SpinnerPreset::Line, SpinnerState::line()),
            (SpinnerPreset::Moon, SpinnerState::moon()),
            (SpinnerPreset::Bounce, SpinnerState::bounce()),
            (SpinnerPreset::Circle, SpinnerState::circle()),
            (SpinnerPreset::Points, SpinnerState::points()),
            (SpinnerPreset::Arc, SpinnerState::arc()),
            (SpinnerPreset::Toggle, SpinnerState::toggle()),
            (SpinnerPreset::Arrow, SpinnerState::arrow()),
        ];
        for (preset, expected) in cases {
            assert_eq!(SpinnerState::preset(preset), expected);
        }
    }

    #[test]
    fn frame_handles_large_tick_without_panicking() {
        // Edge case: very large tick must wrap, not overflow/panic.
        let s = SpinnerState::moon();
        let n = s.frame_count() as u64;
        assert_eq!(s.frame(u64::MAX), s.frame(u64::MAX % n));
    }
}
