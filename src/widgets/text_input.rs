//! Text-input family state — text input, validators, form fields and
//! forms, textarea, and number input.
//!
//! Rendered by `context/widgets/text_input.rs`.

use super::*;

use crate::width::UnicodeSegmentation as _;

mod code_editor;
pub use code_editor::*;
mod history;
use history::{TextInputHistory, TextInputMeta, TextareaHistory, TextareaMeta};

/// Locate the first complete grapheme boundary at or after an insertion edge.
pub(crate) fn grapheme_cursor_after(text: &str, byte: usize) -> usize {
    text.grapheme_indices(true)
        .take_while(|(start, _)| *start < byte)
        .count()
}

/// Prepare an insertion without mutating text or undo state. The caret has
/// downstream affinity when inserted text joins the following grapheme.
pub(crate) fn prepare_text_insert(
    value: &str,
    byte: usize,
    text: &str,
    max_length: Option<usize>,
) -> Option<(String, usize)> {
    if text.is_empty() {
        return None;
    }
    let build = |inserted: &str| {
        let mut result = String::with_capacity(value.len() + inserted.len());
        result.push_str(&value[..byte]);
        result.push_str(inserted);
        result.push_str(&value[byte..]);
        result
    };
    let result = build(text);
    if max_length.is_none_or(|max| result.graphemes(true).count() <= max) {
        return Some((result, byte + text.len()));
    }
    let max = max_length?;
    // The longest prefix of `text`, cut at one of its own grapheme
    // boundaries, whose *resulting* string fits `max` (issue #496).
    //
    // Inserted clusters can merge into the grapheme before the cursor (ZWJ
    // emoji sequences, conjoining jamo, combining marks, CR+LF), and the cut
    // end can merge into the grapheme after it, so the answer is not a fixed
    // number of clusters past the remaining budget. Two facts keep the
    // search exact and cheap:
    //
    // * `prefix(e) = graphemes(before + text[..e])` never decreases with `e`:
    //   UAX #29 boundaries depend only on what precedes them (plus the next
    //   scalar), so appending text never removes a boundary inside the
    //   already-segmented part.
    // * `prefix(e) <= total(e) <= prefix(e) + after_count`, and gluing `after`
    //   on removes at most two boundaries (the junction, plus one inside
    //   `after` when GB11 / GB9c look back across it; RI pairing keeps the
    //   count). So no end with `prefix(e) > max + 2 - after_count` can fit,
    //   and every end with `prefix(e) + after_count <= max` fits.
    //
    // Binary search the first bound, then walk down exactly; only the few
    // ends between the two bounds need a full count.
    const MAX_JUNCTION_LOSS: usize = 2;
    let before = &value[..byte];
    let after = &value[byte..];
    let after_count = after.graphemes(true).count();
    let ends: Vec<usize> = text
        .grapheme_indices(true)
        .map(|(start, cluster)| start + cluster.len())
        .filter(|&end| end < text.len())
        .collect();
    let prefix_count = |end: usize| {
        let mut joined = String::with_capacity(before.len() + end);
        joined.push_str(before);
        joined.push_str(&text[..end]);
        joined.graphemes(true).count()
    };
    let ceiling = (max + MAX_JUNCTION_LOSS).checked_sub(after_count)?;
    // `ends[..upper]` are the ends whose prefix count is within `ceiling`.
    let upper = ends.partition_point(|&end| prefix_count(end) <= ceiling);
    for &end in ends[..upper].iter().rev() {
        let candidate = build(&text[..end]);
        if candidate.graphemes(true).count() <= max {
            return Some((candidate, byte + end));
        }
    }
    None
}

#[cfg(test)]
mod insertion_regressions {
    use super::*;
    use proptest::prelude::*;

    fn text() -> impl Strategy<Value = String> {
        proptest::collection::vec(
            proptest::sample::select(vec![
                'a',
                'b',
                '\r',
                '\n',
                '\u{301}',
                '\u{200d}',
                '\u{600}',
                '\u{903}',
                '\u{915}',
                '\u{94d}',
                '\u{1100}',
                '\u{1161}',
                '\u{11a8}',
                '\u{1f1f0}',
                '\u{1f1f7}',
                '\u{1f1fa}',
                '\u{1f469}',
                '\u{1f4bb}',
                '\u{1f3fd}',
            ]),
            0..24,
        )
        .prop_map(|chars| chars.into_iter().collect())
    }

    /// Exhaustive reference: the longest prefix of `inserted` (cut at one of
    /// its own grapheme boundaries) whose *resulting* string fits `max`.
    fn reference(
        value: &str,
        cursor: usize,
        inserted: &str,
        max: usize,
    ) -> Option<(String, usize)> {
        let byte = value
            .grapheme_indices(true)
            .nth(cursor)
            .map_or(value.len(), |(byte, _)| byte);
        inserted
            .grapheme_indices(true)
            .map(|(start, cluster)| start + cluster.len())
            .filter_map(|end| {
                let candidate = format!("{}{}{}", &value[..byte], &inserted[..end], &value[byte..]);
                (candidate.graphemes(true).count() <= max).then_some((candidate, byte + end))
            })
            .next_back()
    }

    fn check(value: &str, cursor: usize, inserted: &str, max: usize) {
        let byte = value
            .grapheme_indices(true)
            .nth(cursor)
            .map_or(value.len(), |(byte, _)| byte);
        assert_eq!(
            prepare_text_insert(value, byte, inserted, Some(max)),
            reference(value, cursor, inserted, max),
            "value={value:?} cursor={cursor} inserted={inserted:?} max={max}"
        );
    }

    /// Issue #496: proptest shrink. The inserted ZWJ and emoji both merge
    /// into the emoji before the cursor, and a truncated prefix ending in a
    /// leading jamo joins the vowel jamo after the cursor, so the fitting
    /// prefix lies further out than a fixed `budget + 2` cluster window.
    #[test]
    fn issue_496_zwj_and_jamo_joins_across_the_cursor() {
        check("aकककa👩ᅡ🇰कaᄀaᄀ", 6, "\u{200d}👩\raᆨकaaaᅡकaaᄀa", 24);
    }

    #[test]
    fn leading_jamo_at_cursor_joins_inserted_vowel() {
        // value "ᄀ|x": inserting V after L forms one syllable.
        check("ᄀx", 1, "ᅡbcd", 3);
        check("ᄀx", 1, "ᅡᆨbcd", 3);
    }

    #[test]
    fn vowel_and_trailing_jamo_join_across_the_cursor() {
        // "가" (L+V) before the cursor, T inserted: still one syllable.
        check("\u{1100}\u{1161}x", 1, "\u{11a8}yz", 2);
        // V before the cursor, T + more inserted, limit hit right away.
        check("\u{1100}\u{1161}\u{1100}", 1, "\u{11a8}\u{1161}q", 3);
    }

    #[test]
    fn limit_hit_mid_syllable() {
        // The cut must fall on a boundary of the inserted text while the
        // resulting syllable count stays within the limit.
        check("ab", 1, "\u{1100}\u{1161}\u{11a8}\u{1100}\u{1161}", 3);
        check("ab", 1, "\u{1100}\u{1161}\u{11a8}\u{1100}\u{1161}", 4);
        check("a\u{1100}", 2, "\u{1161}\u{11a8}\u{1100}\u{1161}x", 2);
    }

    #[test]
    fn long_zwj_chain_merges_into_the_emoji_before_the_cursor() {
        let chain: String = "\u{200d}👩".repeat(40);
        check("👩b", 1, &format!("{chain}xyz"), 3);
        check("👩b", 1, &format!("{chain}xyz"), 2);
    }

    fn cases() -> u32 {
        std::env::var("PROPTEST_CASES")
            .ok()
            .and_then(|cases| cases.parse().ok())
            .unwrap_or(2048)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(cases()))]
        #[test]
        fn limited_insert_matches_exhaustive_resulting_grapheme_reference(
            value in text(), inserted in text(), cursor in 0usize..24, max in 0usize..30,
        ) {
            let byte = value.grapheme_indices(true).nth(cursor).map_or(value.len(), |(byte, _)| byte);
            let expected = reference(&value, cursor, &inserted, max);
            prop_assert_eq!(prepare_text_insert(&value, byte, &inserted, Some(max)), expected);
        }
    }
}

/// Issue #465: mouse-selection bookkeeping shared by `text_input` and
/// `textarea` (press-drag state and a click counter for double / triple
/// clicks).
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct PointerSelect {
    /// A left-button press inside the widget started a drag selection that
    /// has not been released yet.
    pub(crate) dragging: bool,
    clicks: u8,
    last_at: Option<crate::clock::Instant>,
    last_pos: Option<(u32, u32)>,
}

impl PointerSelect {
    /// Count a left press at `pos`: 1 for a single click, 2 for a double and
    /// 3 for a triple click (same cell, each within the double-click window).
    pub(crate) fn register_click(&mut self, now: crate::clock::Instant, pos: (u32, u32)) -> u8 {
        let repeat = self.last_pos == Some(pos)
            && self
                .last_at
                .is_some_and(|at| now.duration_since(at) <= crate::DOUBLE_CLICK_WINDOW);
        self.clicks = if repeat { (self.clicks + 1).min(3) } else { 1 };
        self.last_at = Some(now);
        self.last_pos = Some(pos);
        self.clicks
    }
}

/// Issue #465: selection, undo history and pointer state of a
/// [`TextInputState`]. Private so the invariants stay inside the widget.
#[derive(Debug, Clone, Default)]
pub(crate) struct TextInputEditing {
    /// Selection anchor (grapheme index). The selection spans
    /// `anchor..cursor`, normalized.
    pub(crate) anchor: Option<usize>,
    /// Undo / redo steps as delta records (#415).
    history: TextInputHistory,
    /// The last edit was a typed character, so the next one joins its undo
    /// step.
    pub(crate) last_typing: bool,
    pub(crate) pointer: PointerSelect,
    /// `Ctrl+A` does *not* select all (opt-out flag; select-all is the
    /// default).
    pub(crate) ctrl_a_disabled: bool,
    /// Previous frame's rendered layout: `(cell, grapheme index)` for every
    /// visible grapheme plus a trailing end entry, relative to the first text
    /// cell. Used to map mouse positions to cursor positions.
    pub(crate) visual: Vec<(u32, usize)>,
}

/// Normalized grapheme range between an anchor and a cursor, or `None` when
/// they coincide.
pub(crate) fn normalized_range(anchor: usize, cursor: usize) -> Option<std::ops::Range<usize>> {
    match anchor.cmp(&cursor) {
        std::cmp::Ordering::Less => Some(anchor..cursor),
        std::cmp::Ordering::Greater => Some(cursor..anchor),
        std::cmp::Ordering::Equal => None,
    }
}

/// Byte offset of grapheme index `index` in `text` (the length when past the
/// end).
pub(crate) fn grapheme_byte(text: &str, index: usize) -> usize {
    text.grapheme_indices(true)
        .nth(index)
        .map_or(text.len(), |(byte, _)| byte)
}

/// Whether a grapheme counts as part of a word for word-wise movement and
/// double-click selection: its base scalar is alphanumeric (so CJK
/// ideographs and accented letters are word characters).
pub(crate) fn cluster_is_word(cluster: &str) -> bool {
    cluster
        .chars()
        .next()
        .is_some_and(|c| c.is_alphanumeric() || c == '_')
}

/// Grapheme range of the word (or run of non-word, non-space graphemes, or
/// run of spaces) containing grapheme `index`.
pub(crate) fn word_range_at(text: &str, index: usize) -> std::ops::Range<usize> {
    let clusters: Vec<&str> = text.graphemes(true).collect();
    if clusters.is_empty() {
        return 0..0;
    }
    let index = index.min(clusters.len() - 1);
    let class = |cluster: &str| {
        if cluster_is_word(cluster) {
            0u8
        } else if cluster.chars().all(char::is_whitespace) {
            1
        } else {
            2
        }
    };
    let target = class(clusters[index]);
    let mut start = index;
    while start > 0 && class(clusters[start - 1]) == target {
        start -= 1;
    }
    let mut end = index + 1;
    while end < clusters.len() && class(clusters[end]) == target {
        end += 1;
    }
    start..end
}

/// State for a single-line text input widget.
///
/// Pass a mutable reference to `Context::text_input` each frame. The widget
/// handles all keyboard events when focused.
///
/// # Selection, clipboard and undo
///
/// While focused, `Shift` + `Left` / `Right` / `Home` / `End` (and
/// `Shift+Ctrl`/`Shift+Alt` + arrows for whole words) extend a selection,
/// `Ctrl+A` selects everything (opt out with
/// [`ctrl_a_select_all(false)`](Self::ctrl_a_select_all)), and with mouse
/// capture a press-drag selects a range, a double-click a word and a
/// triple-click everything. Typing or pasting replaces the selection as one
/// undo step, Backspace / Delete remove it, `Ctrl+C` copies it and `Ctrl+X`
/// cuts it through [`Context::copy_to_clipboard`](crate::Context::copy_to_clipboard)
/// (OSC 52). `Ctrl+Z` undoes and `Ctrl+Y` / `Ctrl+Shift+Z` redoes. Masked
/// inputs never put their plaintext on the clipboard: `Ctrl+C` / `Ctrl+X`
/// do not copy (cut still deletes). `Ctrl+C` without a selection is left to
/// the app; note that the default `RunConfig` quits on `Ctrl+C` before the
/// frame runs, so copying needs `RunConfig::handle_ctrl_c(false)`.
///
/// # Example
///
/// ```no_run
/// use slt::widgets::TextInputState;
///
/// // Create state once, outside the frame closure, so typed text persists.
/// let mut input = TextInputState::with_placeholder("Type here...");
/// slt::run(|ui: &mut slt::Context| {
///     ui.text_input(&mut input);
///     // input.value holds the current text
/// })?;
/// # Ok::<_, std::io::Error>(())
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
    /// Issue #454: process-unique focus identity (fresh on `Clone`).
    focus_key: u64,
    /// Issue #465: selection, undo history and pointer state.
    pub(crate) editing: TextInputEditing,
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
            .field("focus_key", &self.focus_key)
            .field("selection", &self.selection())
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
    ///
    /// The clone gets its own [`focus_key`](TextInputState::focus_key), so
    /// rendering both copies yields two independent focus slots.
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
            focus_key: crate::runtime::next_state_focus_key(),
            editing: TextInputEditing {
                pointer: PointerSelect::default(),
                ..self.editing.clone()
            },
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
            focus_key: crate::runtime::next_state_focus_key(),
            editing: TextInputEditing::default(),
        }
    }

    /// Stable focus identity of this input (issue #454).
    ///
    /// [`Context::text_input`](crate::Context::text_input) registers its focus
    /// slot under this key, so focus stays on this input when other
    /// focusables are inserted or removed before it. Each state (including a
    /// [`Clone`]) has its own key. Compare with
    /// [`Context::focused_key`](crate::Context::focused_key).
    pub fn focus_key(&self) -> u64 {
        self.focus_key
    }

    /// Create a text input with placeholder text shown when the value is empty.
    pub fn with_placeholder(p: impl Into<String>) -> Self {
        Self {
            placeholder: p.into(),
            ..Self::new()
        }
    }

    /// Set the maximum allowed grapheme-cluster count.
    #[must_use]
    pub fn max_length(mut self, len: usize) -> Self {
        self.max_length = Some(len);
        self
    }

    /// Create an empty masked input for passwords: shorthand for
    /// `TextInputState::new().masked(true)`.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::{TestBackend, widgets::TextInputState};
    ///
    /// let mut pw = TextInputState::password();
    /// pw.set_value("hunter2");
    /// let mut tb = TestBackend::new(20, 3);
    /// tb.render(|ui| {
    ///     let _ = ui.text_input(&mut pw);
    /// });
    /// tb.assert_contains("•••••••");
    /// tb.assert_not_contains("hunter2");
    /// ```
    pub fn password() -> Self {
        Self::new().masked(true)
    }

    /// Builder: display the value as `•` glyphs (`true`) or as plain text.
    ///
    /// Sets the public [`masked`](TextInputState::masked) field.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::widgets::TextInputState;
    ///
    /// let pin = TextInputState::with_placeholder("PIN").masked(true);
    /// assert!(pin.masked);
    /// ```
    #[must_use]
    pub fn masked(mut self, on: bool) -> Self {
        self.masked = on;
        self
    }

    /// Clear the input: empties the value, moves the cursor to the start and
    /// resets validation errors and the suggestion popup.
    ///
    /// Registered validators and the placeholder are kept. Use it after a
    /// submit, e.g. in a chat or todo input.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::widgets::TextInputState;
    ///
    /// let mut todo = TextInputState::new();
    /// todo.set_value("buy milk");
    /// todo.clear();
    /// assert_eq!(todo.value, "");
    /// assert_eq!(todo.cursor, 0);
    /// ```
    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
        self.reset_editing();
        self.validation_error = None;
        self.validation_errors.clear();
        self.suggestion_index = 0;
        self.show_suggestions = false;
    }

    /// Replace the value and move the cursor to its end.
    ///
    /// The text is truncated to [`max_length`](TextInputState::max_length)
    /// grapheme clusters when a limit is set. Line breaks are not allowed in
    /// a single-line input: `\r` and `\n` are replaced with spaces.
    /// Validation state is left untouched; call
    /// [`run_validators`](Self::run_validators) if the new value should be
    /// re-checked.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::widgets::TextInputState;
    ///
    /// let mut code = TextInputState::new().max_length(4);
    /// code.set_value("123456");
    /// assert_eq!(code.value, "1234");
    /// assert_eq!(code.cursor, 4);
    /// ```
    pub fn set_value(&mut self, value: impl Into<String>) {
        let mut value: String = value.into();
        if value.contains(['\r', '\n']) {
            value = value.replace("\r\n", " ").replace(['\r', '\n'], " ");
        }
        if let Some(max) = self.max_length
            && let Some((byte, _)) = value.grapheme_indices(true).nth(max)
        {
            value.truncate(byte);
        }
        self.cursor = value.graphemes(true).count();
        self.value = value;
        self.reset_editing();
    }

    /// Programmatic replacement starts a fresh editing session: no selection
    /// and no undo history (matching `TextareaState::set_value`).
    fn reset_editing(&mut self) {
        self.editing.anchor = None;
        self.editing.history.clear();
        self.editing.last_typing = false;
    }

    // ── selection (issue #465) ─────────────────────────────────────────

    /// The selected range as grapheme-cluster indices into
    /// [`value`](Self::value), normalized so `start <= end`. `None` when
    /// nothing is selected.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::widgets::TextInputState;
    ///
    /// let mut s = TextInputState::new();
    /// s.set_value("hello world");
    /// s.set_selection(6, 11);
    /// assert_eq!(s.selection(), Some(6..11));
    /// assert_eq!(s.selected_text(), Some("world"));
    /// ```
    pub fn selection(&self) -> Option<std::ops::Range<usize>> {
        let len = self.value.graphemes(true).count();
        let anchor = self.editing.anchor?.min(len);
        normalized_range(anchor, self.cursor.min(len))
    }

    /// The selected text, or `None` when nothing is selected. For a
    /// [`masked`](Self::masked) input this is still the plaintext; the widget
    /// itself never copies it to the clipboard.
    pub fn selected_text(&self) -> Option<&str> {
        let range = self.selection()?;
        let start = grapheme_byte(&self.value, range.start);
        let end = grapheme_byte(&self.value, range.end);
        Some(&self.value[start..end])
    }

    /// Select the whole value (cursor at the end).
    pub fn select_all(&mut self) {
        let len = self.value.graphemes(true).count();
        self.editing.anchor = Some(0);
        self.cursor = len;
    }

    /// Select from `anchor` to `cursor` (grapheme indices, clamped to the
    /// value). The cursor ends at `cursor`; `anchor == cursor` clears the
    /// selection.
    pub fn set_selection(&mut self, anchor: usize, cursor: usize) {
        let len = self.value.graphemes(true).count();
        self.editing.anchor = Some(anchor.min(len));
        self.cursor = cursor.min(len);
    }

    /// Drop the selection, keeping the cursor where it is.
    pub fn clear_selection(&mut self) {
        self.editing.anchor = None;
    }

    /// Builder: whether `Ctrl+A` selects all text while focused (default
    /// `true`). Set `false` to leave `Ctrl+A` to app shortcuts.
    #[must_use]
    pub fn ctrl_a_select_all(mut self, on: bool) -> Self {
        self.editing.ctrl_a_disabled = !on;
        self
    }

    pub(crate) fn ctrl_a_selects_all(&self) -> bool {
        !self.editing.ctrl_a_disabled
    }

    // ── editing primitives used by `Context::text_input` ───────────────

    fn history_meta(&self) -> TextInputMeta {
        TextInputMeta {
            cursor: self.cursor,
            anchor: self.editing.anchor,
        }
    }

    fn restore(&mut self, meta: TextInputMeta) {
        self.cursor = meta.cursor;
        self.editing.anchor = meta.anchor;
        self.editing.last_typing = false;
    }

    /// Record the current state as an undo step (dropping the redo branch).
    ///
    /// Issue #415: steps are stored as edit records against one shared copy
    /// of the value, capped at `DEFAULT_TEXTAREA_HISTORY_MAX` steps and
    /// `DEFAULT_HISTORY_MAX_BYTES` of records.
    pub(crate) fn push_undo(&mut self) {
        let meta = self.history_meta();
        self.editing.history.push(
            &self.value,
            meta,
            DEFAULT_TEXTAREA_HISTORY_MAX,
            history::DEFAULT_HISTORY_MAX_BYTES,
        );
    }

    /// Undo the last edit. Returns `true` when something changed.
    pub(crate) fn undo(&mut self) -> bool {
        let live = self.history_meta();
        let Some(meta) = self.editing.history.undo(&mut self.value, live) else {
            return false;
        };
        self.restore(meta);
        true
    }

    /// Redo the last undone edit. Returns `true` when something changed.
    pub(crate) fn redo(&mut self) -> bool {
        let live = self.history_meta();
        let Some(meta) = self.editing.history.redo(&mut self.value, live) else {
            return false;
        };
        self.restore(meta);
        true
    }

    /// Move the cursor to `target`, extending the selection when `extend`,
    /// otherwise dropping it.
    pub(crate) fn move_cursor(&mut self, target: usize, extend: bool) {
        if extend {
            self.editing.anchor.get_or_insert(self.cursor);
        } else {
            self.editing.anchor = None;
        }
        self.cursor = target;
        self.editing.last_typing = false;
    }

    /// Delete the selection as one undo step. Returns `false` (and does
    /// nothing) when nothing is selected.
    pub(crate) fn delete_selection(&mut self) -> bool {
        let Some(range) = self.selection() else {
            return false;
        };
        self.push_undo();
        let start = grapheme_byte(&self.value, range.start);
        let end = grapheme_byte(&self.value, range.end);
        self.value.replace_range(start..end, "");
        self.cursor = grapheme_cursor_after(&self.value, start);
        self.editing.anchor = None;
        self.editing.last_typing = false;
        true
    }

    /// Insert `text` at the cursor, replacing the selection (one undo step
    /// for delete + insert). Consecutive typed characters (`typing`) share
    /// an undo step. Returns `true` when the value changed.
    pub(crate) fn insert_text(&mut self, text: &str) -> bool {
        self.insert_text_as(text, false)
    }

    /// Typed-character form of [`insert_text`](Self::insert_text).
    pub(crate) fn type_text(&mut self, text: &str) -> bool {
        self.insert_text_as(text, true)
    }

    fn insert_text_as(&mut self, text: &str, typing: bool) -> bool {
        if let Some(range) = self.selection() {
            let start = grapheme_byte(&self.value, range.start);
            let end = grapheme_byte(&self.value, range.end);
            let mut base = String::with_capacity(self.value.len());
            base.push_str(&self.value[..start]);
            base.push_str(&self.value[end..]);
            self.push_undo();
            match prepare_text_insert(&base, start, text, self.max_length) {
                Some((value, caret)) => {
                    self.cursor = grapheme_cursor_after(&value, caret);
                    self.value = value;
                }
                None => {
                    self.cursor = grapheme_cursor_after(&base, start);
                    self.value = base;
                }
            }
            self.editing.anchor = None;
            self.editing.last_typing = typing;
            return true;
        }
        let byte = grapheme_byte(&self.value, self.cursor);
        let Some((value, caret)) = prepare_text_insert(&self.value, byte, text, self.max_length)
        else {
            return false;
        };
        if !(typing && self.editing.last_typing) {
            self.push_undo();
        }
        self.cursor = grapheme_cursor_after(&value, caret);
        self.value = value;
        self.editing.last_typing = typing;
        true
    }

    /// Backspace: delete the selection, or the grapheme before the cursor.
    pub(crate) fn delete_backward(&mut self) -> bool {
        if self.delete_selection() {
            return true;
        }
        self.editing.last_typing = false;
        if self.cursor == 0 {
            return false;
        }
        self.push_undo();
        let start = grapheme_byte(&self.value, self.cursor - 1);
        let end = grapheme_byte(&self.value, self.cursor);
        self.value.replace_range(start..end, "");
        self.cursor = grapheme_cursor_after(&self.value, start);
        true
    }

    /// Delete: delete the selection, or the grapheme after the cursor.
    pub(crate) fn delete_forward(&mut self) -> bool {
        if self.delete_selection() {
            return true;
        }
        self.editing.last_typing = false;
        if self.cursor >= self.value.graphemes(true).count() {
            return false;
        }
        self.push_undo();
        let start = grapheme_byte(&self.value, self.cursor);
        let end = grapheme_byte(&self.value, self.cursor + 1);
        self.value.replace_range(start..end, "");
        self.cursor = grapheme_cursor_after(&self.value, start);
        true
    }

    /// Run `validator` against the current value once, now, and store the
    /// result in [`validation_error`](TextInputState::validation_error)
    /// (`None` on success).
    ///
    /// For validators that should run again on demand, register them with
    /// [`add_validator`](Self::add_validator) and call
    /// [`run_validators`](Self::run_validators). Inside a form, attach them
    /// with [`FormField::validate`], which `form_field` runs automatically.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::widgets::TextInputState;
    ///
    /// let mut email = TextInputState::new();
    /// email.set_value("nobody");
    /// email.validate_now(|v| if v.contains('@') { Ok(()) } else { Err("invalid".into()) });
    /// assert_eq!(email.validation_error.as_deref(), Some("invalid"));
    /// ```
    pub fn validate_now(&mut self, validator: impl Fn(&str) -> Result<(), String>) {
        self.validation_error = validator(&self.value).err();
    }

    /// Deprecated name of [`validate_now`](Self::validate_now).
    ///
    /// On [`FormField`], `validate` *attaches* a validator that runs later;
    /// here it ran immediately, so code copied between the two silently
    /// changed meaning. The immediate form is now spelled `validate_now`.
    #[deprecated(
        since = "0.26.0",
        note = "renamed to `validate_now`; `FormField::validate` attaches a validator instead of running it"
    )]
    pub fn validate(&mut self, validator: impl Fn(&str) -> Result<(), String>) {
        self.validate_now(validator);
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
    /// Stable lookup key for [`FormState::get`]; defaults to the label.
    key: Option<String>,
    /// This field's validators. Mutate via [`validate`](Self::validate); run
    /// via [`run_validators`](Self::run_validators).
    validators: Vec<Validator>,
    /// Whether the field's input held keyboard focus on the previous frame.
    ///
    /// [`Context::form_field`](crate::Context::form_field) tracks this field's
    /// focused-to-unfocused edge for [`ValidateTrigger::OnBlur`], even when
    /// surrounding positional focus slots change between frames.
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
            key: None,
            validators: Vec::new(),
            was_focused: false,
            #[cfg(feature = "async")]
            pending: None,
        }
    }

    /// Set placeholder text for this field's input.
    #[must_use]
    pub fn placeholder(mut self, p: impl Into<String>) -> Self {
        self.input.placeholder = p.into();
        self
    }

    /// Builder: mask this field's input as `•` glyphs (passwords, PINs).
    ///
    /// # Example
    ///
    /// ```
    /// use slt::{TestBackend, widgets::FormField};
    ///
    /// let mut field = FormField::new("Password").masked(true);
    /// field.input.set_value("s3cret");
    /// let mut tb = TestBackend::new(30, 6);
    /// tb.render(|ui| {
    ///     ui.form_field(&mut field);
    /// });
    /// tb.assert_contains("••••••");
    /// tb.assert_not_contains("s3cret");
    /// ```
    #[must_use]
    pub fn masked(mut self, on: bool) -> Self {
        self.input.masked = on;
        self
    }

    /// Builder: shorthand for [`masked(true)`](Self::masked).
    #[must_use]
    pub fn password(self) -> Self {
        self.masked(true)
    }

    /// Builder: set the key [`FormState::get`] and
    /// [`FormState::field_mut`] look this field up by. Defaults to the
    /// label, so set it when labels are translated or may change.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::widgets::{FormField, FormState};
    ///
    /// let form = FormState::new().field(FormField::new("Courriel").key("email"));
    /// assert_eq!(form.get("email"), Some(""));
    /// ```
    #[must_use]
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// The key this field is looked up by: the one set with
    /// [`key`](Self::key), or the label.
    pub fn lookup_key(&self) -> &str {
        self.key.as_deref().unwrap_or(&self.label)
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
    #[must_use]
    pub fn validate(mut self, f: impl Fn(&str) -> Result<(), String> + 'static) -> Self {
        self.validators.push(Validator::new(f));
        self
    }

    /// Run this field's validators on every change (each keystroke).
    #[must_use]
    pub fn on_change(mut self) -> Self {
        self.trigger = ValidateTrigger::OnChange;
        self
    }

    /// Run this field's validators when it loses focus (the default).
    #[must_use]
    pub fn on_blur(mut self) -> Self {
        self.trigger = ValidateTrigger::OnBlur;
        self
    }

    /// Disable automatic validation; the app must call
    /// [`run_validators`](Self::run_validators) or
    /// [`FormState::validate_all`] explicitly.
    #[must_use]
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
    #[must_use]
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
    /// This replaced the positional `validate(&[FormValidator])` (removed in
    /// 0.26) — validators are co-located with their
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

    /// Get field value by index.
    ///
    /// Positional: reordering fields changes which value is read. Prefer
    /// [`get`](Self::get).
    pub fn value(&self, index: usize) -> &str {
        self.fields
            .get(index)
            .map(|f| f.input.value.as_str())
            .unwrap_or("")
    }

    /// Get a field's value by its [`key`](FormField::key); fields without an
    /// explicit key are matched by label. Returns `None` when no field
    /// matches.
    ///
    /// Unlike [`value`](Self::value), this keeps reading the right field when
    /// fields are reordered.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::widgets::{FormField, FormState};
    ///
    /// let mut form = FormState::new()
    ///     .field(FormField::new("Name"))
    ///     .field(FormField::new("Email").key("email"));
    /// form.field_mut("email").unwrap().input.set_value("a@b.c");
    /// assert_eq!(form.get("email"), Some("a@b.c"));
    /// assert_eq!(form.get("Name"), Some(""));
    /// assert_eq!(form.get("phone"), None);
    /// ```
    pub fn get(&self, key: &str) -> Option<&str> {
        self.field_index(key)
            .map(|index| self.fields[index].input.value.as_str())
    }

    /// Get a field mutably by its [`key`](FormField::key); fields without an
    /// explicit key are matched by label.
    pub fn field_mut(&mut self, key: &str) -> Option<&mut FormField> {
        self.field_index(key).map(|index| &mut self.fields[index])
    }

    /// Explicit keys win over labels, so a field whose label equals another
    /// field's key never shadows it.
    fn field_index(&self, key: &str) -> Option<usize> {
        self.fields
            .iter()
            .position(|field| field.key.as_deref() == Some(key))
            .or_else(|| {
                self.fields
                    .iter()
                    .position(|field| field.key.is_none() && field.label == key)
            })
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

/// Default maximum number of undo steps kept in [`TextareaState`] history.
/// Used by [`TextareaState::new`] and the `Default` impl. Override
/// per-instance via [`TextareaState::history_max`].
pub(crate) const DEFAULT_TEXTAREA_HISTORY_MAX: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TextareaNavigation {
    pub(crate) row: usize,
    pub(crate) col: usize,
    pub(crate) wrap_width: Option<u32>,
    pub(crate) upstream: bool,
    pub(crate) desired_cells: usize,
}

/// Issue #415: the soft-wrap layout of a [`TextareaState`], kept across
/// frames so an unchanged wrapped document is not re-wrapped every frame.
///
/// Validity is checked against a contiguous copy of the rows the layout was
/// built from (one byte comparison per frame), because `lines` is public and
/// may change without the widget seeing an edit.
#[derive(Default)]
pub(crate) struct TextareaWrapCache {
    width: u32,
    /// The rows the layout was built from, concatenated.
    text: String,
    /// End offset of each row in `text`.
    ends: Vec<usize>,
    vlines: Vec<crate::context::TextareaVLine>,
    built: bool,
}

impl std::fmt::Debug for TextareaWrapCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextareaWrapCache")
            .field("width", &self.width)
            .field("rows", &self.ends.len())
            .field("visual_lines", &self.vlines.len())
            .finish_non_exhaustive()
    }
}

impl TextareaWrapCache {
    /// Whether the cached layout was built from exactly `lines` at `width`.
    fn matches(&self, lines: &[String], width: u32) -> bool {
        if !self.built || self.width != width || self.ends.len() != lines.len() {
            return false;
        }
        let text = self.text.as_bytes();
        let mut start = 0;
        lines.iter().zip(&self.ends).all(|(line, &end)| {
            let same = text.get(start..end) == Some(line.as_bytes());
            start = end;
            same
        })
    }

    /// The soft-wrap layout of `lines` at `width`, rebuilt only when either
    /// changed since the previous call.
    pub(crate) fn visual_lines(
        &mut self,
        lines: &[String],
        width: u32,
    ) -> &[crate::context::TextareaVLine] {
        if !self.matches(lines, width) {
            self.vlines = crate::context::textarea_build_visual_lines(lines, width);
            self.text.clear();
            self.ends.clear();
            for line in lines {
                self.text.push_str(line);
                self.ends.push(self.text.len());
            }
            self.width = width;
            self.built = true;
        }
        &self.vlines
    }
}

/// State for a multi-line text area widget.
///
/// Pass a mutable reference to `Context::textarea` each frame along with the
/// number of visible rows. The widget handles all keyboard events when focused.
///
/// # Undo / redo
///
/// `Ctrl+Z` undoes the most recent edit and `Ctrl+Y` redoes it. The widget
/// records an undo step before every destructive mutation (char insert,
/// delete, Enter, Backspace, paste). Rapid character typing coalesces into a
/// single undoable batch — only the first char of a typing burst records a
/// step. Rejected or empty pastes do not record history or discard redo. An
/// event batch is not an undo transaction: accepted edits retain their normal
/// undo grouping even when their net content change is zero.
///
/// Steps are stored as edit records (the changed rows before and after),
/// not as copies of the document, so retained memory scales with what was
/// edited: history keeps one copy of the document, taken at the first
/// recorded edit, plus the records. History is capped at
/// [`history_max`](Self::history_max) steps (default `100`) and
/// [`history_max_bytes`](Self::history_max_bytes) of records (default
/// 8 MiB); the oldest steps are dropped first and the newest step is always
/// kept. Changing [`lines`](Self::lines) directly between edits is recorded
/// exactly like an edit made by the widget.
///
/// # Selection
///
/// `Shift` + arrows / `Home` / `End` (and `Shift+Ctrl`/`Shift+Alt` +
/// `Left`/`Right` for words) extend a selection, `Ctrl+A` selects all (opt
/// out with [`ctrl_a_select_all(false)`](Self::ctrl_a_select_all)), and with
/// mouse capture a press-drag selects a range, a double-click a word and a
/// triple-click a line. Typing, `Enter` or a paste replaces the selection as
/// one undo step; Backspace / Delete remove it; `Ctrl+C` copies and `Ctrl+X`
/// cuts through OSC 52. Positions are `(row, column)` pairs in grapheme
/// clusters, like [`cursor_row`](Self::cursor_row) /
/// [`cursor_col`](Self::cursor_col).
///
/// # Example
///
/// ```no_run
/// use slt::widgets::TextareaState;
///
/// let mut state = TextareaState::new();
/// slt::run(|ui: &mut slt::Context| {
///     // Type, then press Ctrl+Z to undo or Ctrl+Y to redo.
///     ui.textarea(&mut state, 5);
/// })?;
/// # Ok::<_, std::io::Error>(())
/// ```
#[derive(Debug)]
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
    /// Undo/redo steps as delta records (#415); the count cap and byte
    /// budget live inside.
    pub(crate) history: TextareaHistory,
    /// Whether the previous keypress was a `Char` insert. Used to coalesce
    /// rapid typing into a single undoable burst — when true, the next `Char`
    /// keypress does not push a snapshot.
    pub(crate) last_was_char_insert: bool,
    pub(crate) navigation: Option<TextareaNavigation>,
    /// Issue #454: process-unique focus identity (fresh on `Clone`).
    focus_key: u64,
    /// Issue #465: selection anchor `(row, col)`; the selection spans
    /// `anchor..cursor`, normalized.
    pub(crate) anchor: Option<(usize, usize)>,
    /// Issue #465: press-drag and click-count state.
    pub(crate) pointer: PointerSelect,
    /// Issue #465: `Ctrl+A` does *not* select all (opt-out flag).
    pub(crate) ctrl_a_disabled: bool,
    /// Issue #465: visual `(row, col)` where the previous frame drew the
    /// caret glyph, which shifts the cells after it by one. Used to map
    /// mouse positions back to text positions.
    pub(crate) caret_glyph: Option<(usize, usize)>,
    /// Issue #415: soft-wrap layout reused while the document is unchanged.
    pub(crate) wrap_cache: TextareaWrapCache,
}

impl Clone for TextareaState {
    /// Clones content, cursor and history. The clone gets its own
    /// [`focus_key`](TextareaState::focus_key) so both copies can be rendered
    /// as independent focus slots.
    fn clone(&self) -> Self {
        Self {
            lines: self.lines.clone(),
            cursor_row: self.cursor_row,
            cursor_col: self.cursor_col,
            max_length: self.max_length,
            wrap_width: self.wrap_width,
            scroll_offset: self.scroll_offset,
            history: self.history.clone(),
            last_was_char_insert: self.last_was_char_insert,
            navigation: self.navigation,
            focus_key: crate::runtime::next_state_focus_key(),
            anchor: self.anchor,
            pointer: PointerSelect::default(),
            ctrl_a_disabled: self.ctrl_a_disabled,
            caret_glyph: None,
            wrap_cache: TextareaWrapCache::default(),
        }
    }
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
            history: TextareaHistory::new(DEFAULT_TEXTAREA_HISTORY_MAX),
            last_was_char_insert: false,
            navigation: None,
            focus_key: crate::runtime::next_state_focus_key(),
            anchor: None,
            pointer: PointerSelect::default(),
            ctrl_a_disabled: false,
            caret_glyph: None,
            wrap_cache: TextareaWrapCache::default(),
        }
    }

    /// Stable focus identity of this text area (issue #454). See
    /// [`TextInputState::focus_key`].
    pub fn focus_key(&self) -> u64 {
        self.focus_key
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
        self.last_was_char_insert = false;
        self.navigation = None;
        self.anchor = None;
    }

    // ── selection (issue #465) ─────────────────────────────────────────

    /// Clamp a `(row, col)` position into the current text.
    fn clamp_pos(&self, (row, col): (usize, usize)) -> (usize, usize) {
        let row = row.min(self.lines.len().saturating_sub(1));
        let len = self
            .lines
            .get(row)
            .map_or(0, |line| line.graphemes(true).count());
        (row, col.min(len))
    }

    /// The selected range as `(row, col)` grapheme positions, normalized so
    /// `start <= end`. `None` when nothing is selected.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::widgets::TextareaState;
    ///
    /// let mut s = TextareaState::new();
    /// s.set_value("one\ntwo");
    /// s.set_selection((0, 1), (1, 2));
    /// assert_eq!(s.selection(), Some((0, 1)..(1, 2)));
    /// assert_eq!(s.selected_text().as_deref(), Some("ne\ntw"));
    /// ```
    pub fn selection(&self) -> Option<std::ops::Range<(usize, usize)>> {
        let anchor = self.clamp_pos(self.anchor?);
        let cursor = self.clamp_pos((self.cursor_row, self.cursor_col));
        match anchor.cmp(&cursor) {
            std::cmp::Ordering::Less => Some(anchor..cursor),
            std::cmp::Ordering::Greater => Some(cursor..anchor),
            std::cmp::Ordering::Equal => None,
        }
    }

    /// The selected text with `\n` between lines, or `None` when nothing is
    /// selected.
    pub fn selected_text(&self) -> Option<String> {
        let range = self.selection()?;
        let ((r1, c1), (r2, c2)) = (range.start, range.end);
        let b1 = grapheme_byte(&self.lines[r1], c1);
        let b2 = grapheme_byte(&self.lines[r2], c2);
        if r1 == r2 {
            return Some(self.lines[r1][b1..b2].to_string());
        }
        let mut out = String::from(&self.lines[r1][b1..]);
        for line in &self.lines[r1 + 1..r2] {
            out.push('\n');
            out.push_str(line);
        }
        out.push('\n');
        out.push_str(&self.lines[r2][..b2]);
        Some(out)
    }

    /// Select the whole document (cursor at the end).
    pub fn select_all(&mut self) {
        self.anchor = Some((0, 0));
        self.cursor_row = self.lines.len().saturating_sub(1);
        self.cursor_col = self
            .lines
            .last()
            .map_or(0, |line| line.graphemes(true).count());
        self.navigation = None;
    }

    /// Select from `anchor` to `cursor` (`(row, col)` grapheme positions,
    /// clamped). The cursor ends at `cursor`.
    pub fn set_selection(&mut self, anchor: (usize, usize), cursor: (usize, usize)) {
        self.anchor = Some(self.clamp_pos(anchor));
        (self.cursor_row, self.cursor_col) = self.clamp_pos(cursor);
        self.navigation = None;
    }

    /// Drop the selection, keeping the cursor where it is.
    pub fn clear_selection(&mut self) {
        self.anchor = None;
    }

    /// Builder: whether `Ctrl+A` selects all text while focused (default
    /// `true`).
    #[must_use]
    pub fn ctrl_a_select_all(mut self, on: bool) -> Self {
        self.ctrl_a_disabled = !on;
        self
    }

    /// Delete the selection as one undo step. Returns `false` when nothing
    /// is selected.
    pub(crate) fn delete_selection(&mut self) -> bool {
        let Some(range) = self.selection() else {
            return false;
        };
        self.push_history();
        self.remove_range(range);
        true
    }

    /// Remove `range` without recording history and put the cursor at its
    /// start.
    fn remove_range(&mut self, range: std::ops::Range<(usize, usize)>) {
        let ((r1, c1), (r2, c2)) = (range.start, range.end);
        let b1 = grapheme_byte(&self.lines[r1], c1);
        let b2 = grapheme_byte(&self.lines[r2], c2);
        let mut joined = String::with_capacity(b1 + self.lines[r2].len() - b2);
        joined.push_str(&self.lines[r1][..b1]);
        joined.push_str(&self.lines[r2][b2..]);
        self.lines.splice(r1..=r2, std::iter::once(joined));
        self.cursor_row = r1;
        self.cursor_col = grapheme_cursor_after(&self.lines[r1], b1);
        self.anchor = None;
        self.last_was_char_insert = false;
        self.navigation = None;
    }

    /// Insert `text`, replacing the selection. Replacement is one undo step
    /// (a single snapshot before the delete + insert).
    pub(crate) fn replace_selection_or_insert(&mut self, text: &str, typing: bool) -> bool {
        let Some(range) = self.selection() else {
            return self.insert_text(text, typing);
        };
        self.push_history();
        self.remove_range(range);
        // The snapshot above already covers this edit.
        self.last_was_char_insert = true;
        self.insert_text(text, true);
        self.last_was_char_insert = typing;
        true
    }

    pub(crate) fn insert_text(&mut self, text: &str, typing: bool) -> bool {
        let line = &self.lines[self.cursor_row];
        let byte = line
            .grapheme_indices(true)
            .nth(self.cursor_col)
            .map_or(line.len(), |(byte, _)| byte);
        let max = self.max_length.map(|max| {
            max.saturating_sub(
                self.grapheme_len()
                    .saturating_sub(line.graphemes(true).count()),
            )
        });
        // Logical newline separators count independently even if an existing
        // trailing CR would combine with an inserted LF in Unicode text.
        let max = if line[..byte].ends_with('\r') && text.starts_with('\n') {
            max.map(|max| max.saturating_sub(1))
        } else {
            max
        };
        let Some((result, end)) = prepare_text_insert(line, byte, text, max) else {
            return false;
        };
        let prefix = &result[..end];
        let added_rows = prefix.bytes().filter(|byte| *byte == b'\n').count();
        let line_start = prefix.rfind('\n').map_or(0, |byte| byte + 1);
        let last_line = result[line_start..].split('\n').next().unwrap_or("");
        let col = grapheme_cursor_after(last_line, end - line_start);
        if !typing || !self.last_was_char_insert {
            self.push_history();
        }
        if result.contains('\n') {
            self.lines.splice(
                self.cursor_row..=self.cursor_row,
                result.split('\n').map(str::to_owned),
            );
        } else {
            self.lines[self.cursor_row] = result;
        }
        self.cursor_row += added_rows;
        self.cursor_col = col;
        self.last_was_char_insert = typing;
        self.navigation = None;
        true
    }

    /// Set the maximum grapheme-cluster count, including logical newlines.
    #[must_use]
    pub fn max_length(mut self, len: usize) -> Self {
        self.max_length = Some(len);
        self
    }

    /// Enable soft word-wrap at the given display-column width.
    #[must_use]
    pub fn word_wrap(mut self, width: u32) -> Self {
        self.wrap_width = Some(width);
        self
    }

    /// Override the maximum number of undo steps kept (default `100`).
    ///
    /// When the history exceeds this cap the oldest step is dropped.
    /// Setting `0` disables undo recording — the field is read every keypress.
    #[must_use]
    pub fn history_max(mut self, cap: usize) -> Self {
        self.history.max = cap;
        self
    }

    /// Override the byte budget for retained undo records (default 8 MiB).
    ///
    /// Each undo step stores the rows it changed, before and after; when the
    /// records together exceed `bytes` the oldest steps are dropped. The
    /// newest step is always kept, however large. The budget does not
    /// include the one copy of the document that history keeps once an
    /// edit has been recorded. Use [`history_max`](Self::history_max)`(0)`
    /// to disable history entirely.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::widgets::TextareaState;
    ///
    /// let state = TextareaState::new().history_max_bytes(1 << 20);
    /// assert_eq!(state.history_bytes_cap(), 1 << 20);
    /// assert_eq!(state.history_bytes(), 0);
    /// ```
    #[must_use]
    pub fn history_max_bytes(mut self, bytes: usize) -> Self {
        self.history.max_bytes = bytes;
        self
    }

    /// Number of undo steps currently retained.
    ///
    /// Read-only — useful for tests and debugging the history cap. The cap
    /// itself is set via [`history_max`](Self::history_max).
    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// Maximum number of undo steps retained.
    ///
    /// Mirrors [`history_max`](Self::history_max) (the builder setter) but as
    /// a getter — useful for tests asserting the cap stays bounded.
    pub fn history_cap(&self) -> usize {
        self.history.max
    }

    /// Approximate bytes retained by undo records (the changed rows of each
    /// step plus per-record overhead), excluding the history's one copy of
    /// the document. Bounded by
    /// [`history_max_bytes`](Self::history_max_bytes) except for the newest
    /// step.
    pub fn history_bytes(&self) -> usize {
        self.history.bytes()
    }

    /// Byte budget for undo records, as set by
    /// [`history_max_bytes`](Self::history_max_bytes).
    pub fn history_bytes_cap(&self) -> usize {
        self.history.max_bytes
    }

    fn history_meta(&self) -> TextareaMeta {
        TextareaMeta {
            cursor_row: self.cursor_row,
            cursor_col: self.cursor_col,
            navigation: self.navigation,
        }
    }

    fn restore_meta(&mut self, meta: TextareaMeta) {
        self.cursor_row = meta.cursor_row;
        self.cursor_col = meta.cursor_col;
        self.navigation = meta.navigation;
    }

    /// Record the current content + cursor as an undo step.
    ///
    /// Drops any redo branch, then evicts the oldest steps beyond
    /// [`history_max`](Self::history_max) or
    /// [`history_max_bytes`](Self::history_max_bytes). The next `Ctrl+Z`
    /// returns to the recorded state.
    pub(crate) fn push_history(&mut self) {
        let meta = self.history_meta();
        self.history.push(&self.lines, meta);
    }

    /// Step back to the previous recorded state.
    ///
    /// No-op when the history is empty or already at the start. Returns `true`
    /// when a state was restored.
    pub(crate) fn undo(&mut self) -> bool {
        let live = self.history_meta();
        let Some(meta) = self.history.undo(&mut self.lines, live) else {
            return false;
        };
        self.restore_meta(meta);
        true
    }

    /// Step forward to the next recorded state (or the state before the
    /// first undo).
    ///
    /// No-op when already at the newest state. Returns `true` when a state
    /// was restored.
    pub(crate) fn redo(&mut self) -> bool {
        let Some(meta) = self.history.redo(&mut self.lines) else {
            return false;
        };
        self.restore_meta(meta);
        true
    }
}

impl Default for TextareaState {
    fn default() -> Self {
        Self::new()
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
    #[must_use]
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

/// Response from [`Context::form_field_response`](crate::Context::form_field_response).
///
/// The outer field response covers label, input and validation output; its
/// focus, edit and submit flags are forwarded from the input. The existing
/// `Context::form_field` chaining API remains available.
#[derive(Debug, Clone, Default)]
#[must_use = "FormFieldResponse contains focus, validation and layout feedback"]
pub struct FormFieldResponse {
    /// Field-level hit rectangle with the input's focus/edit/submit flags.
    pub response: Response,
    /// The input's own interaction response and clipped hit rectangle.
    pub input: Response,
    /// Previous-frame full field border box, before scroll offsets or clipping.
    /// Uses root/overlay logical coordinates, not local scroll coordinates.
    /// `None` before measurement, after resize, or for an empty layout area.
    pub layout_rect: Option<crate::Rect>,
}

impl std::ops::Deref for FormFieldResponse {
    type Target = Response;
    fn deref(&self) -> &Response {
        &self.response
    }
}
