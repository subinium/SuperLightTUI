//! Rich-content family state — rich log, streaming text / markdown, tool
//! approval, context items and the diff viewer.
//!
//! Rendered by `context/widgets/rich.rs`.

use super::*;

mod diff;
pub use diff::*;

/// State for the rich log viewer widget.
#[derive(Debug, Clone)]
pub struct RichLogState {
    /// Log entries to display, ordered from oldest to newest.
    entries: std::collections::VecDeque<RichLogEntry>,
    /// Scroll offset (0 = top).
    pub(crate) scroll_offset: usize,
    /// Whether to auto-scroll to bottom when new entries are added.
    pub auto_scroll: bool,
    /// Maximum number of entries to keep (None = unlimited).
    pub max_entries: Option<usize>,
}

/// A single entry in a RichLog.
#[derive(Debug, Clone)]
pub struct RichLogEntry {
    /// Styled text segments for this entry.
    pub segments: Vec<(String, Style)>,
}

impl RichLogState {
    /// Default maximum entry cap used by [`RichLogState::new`].
    ///
    /// Long-running apps that push log entries continuously would otherwise
    /// accumulate state without bound. Use [`RichLogState::new_unbounded`] to
    /// opt out explicitly.
    pub const DEFAULT_MAX_ENTRIES: usize = 10_000;

    /// Create an empty rich log state with the default entry cap
    /// ([`Self::DEFAULT_MAX_ENTRIES`]).
    pub fn new() -> Self {
        Self {
            max_entries: Some(Self::DEFAULT_MAX_ENTRIES),
            ..Self::new_unbounded()
        }
    }

    /// Create an empty rich log state without an entry cap.
    ///
    /// Prefer [`RichLogState::new`] in long-running apps. Use this constructor
    /// only when the host explicitly bounds growth elsewhere.
    pub fn new_unbounded() -> Self {
        Self {
            entries: std::collections::VecDeque::new(),
            scroll_offset: 0,
            auto_scroll: true,
            max_entries: None,
        }
    }

    /// Add a single-style entry to the log.
    pub fn push(&mut self, text: impl Into<String>, style: Style) {
        self.push_segments(vec![(text.into(), style)]);
    }

    /// Add a plain text entry using default style.
    pub fn push_plain(&mut self, text: impl Into<String>) {
        self.push(text, Style::new());
    }

    /// Add a multi-segment styled entry to the log.
    pub fn push_segments(&mut self, segments: Vec<(String, Style)>) {
        self.push_entry(RichLogEntry { segments });
    }

    /// Add a pre-built entry to the log and enforce the configured cap.
    ///
    /// This replaces direct `entries.push(...)` access from earlier releases.
    pub fn push_entry(&mut self, entry: RichLogEntry) {
        self.entries.push_back(entry);

        if let Some(max_entries) = self.max_entries
            && self.entries.len() > max_entries
        {
            let remove_count = self.entries.len() - max_entries;
            for _ in 0..remove_count {
                let _ = self.entries.pop_front();
            }
            self.scroll_offset = self.scroll_offset.saturating_sub(remove_count);
        }

        if self.auto_scroll {
            self.scroll_offset = usize::MAX;
        }
    }

    /// Clear all entries and reset scroll position.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.scroll_offset = 0;
    }

    /// Return number of entries in the log.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return true when no entries are present.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over entries from oldest to newest.
    ///
    /// This replaces `entries.iter()` from earlier releases. Use [`entry`](Self::entry)
    /// for indexed access.
    pub fn entries(
        &self,
    ) -> impl DoubleEndedIterator<Item = &RichLogEntry> + ExactSizeIterator + '_ {
        self.entries.iter()
    }

    /// Mutably iterate over entries from oldest to newest.
    ///
    /// Entry contents may be changed, but insertion and removal remain behind
    /// [`push_entry`](Self::push_entry) and [`clear`](Self::clear) so retention
    /// invariants cannot be bypassed.
    pub fn entries_mut(
        &mut self,
    ) -> impl DoubleEndedIterator<Item = &mut RichLogEntry> + ExactSizeIterator + '_ {
        self.entries.iter_mut()
    }

    /// Return an entry by its zero-based position in oldest-to-newest order.
    ///
    /// This replaces `entries[index]` from earlier releases without exposing
    /// the retention storage type.
    pub fn entry(&self, index: usize) -> Option<&RichLogEntry> {
        self.entries.get(index)
    }

    /// Return a mutable entry by its zero-based position in retention order.
    pub fn entry_mut(&mut self, index: usize) -> Option<&mut RichLogEntry> {
        self.entries.get_mut(index)
    }
}

impl Default for RichLogState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod rich_log_tests {
    use super::*;

    fn texts(state: &RichLogState) -> Vec<&str> {
        state
            .entries()
            .map(|entry| entry.segments[0].0.as_str())
            .collect()
    }

    #[test]
    fn cap_zero_discards_every_push() {
        let mut state = RichLogState::new();
        state.auto_scroll = false;
        state.max_entries = Some(0);

        state.push_plain("discarded");

        assert!(state.is_empty());
        assert_eq!(state.scroll_offset, 0);
    }

    #[test]
    fn cap_one_retains_only_the_newest_entry() {
        let mut state = RichLogState::new();
        state.max_entries = Some(1);

        state.push_plain("first");
        state.push_plain("second");

        assert_eq!(texts(&state), ["second"]);
    }

    #[test]
    fn default_cap_retains_order_without_front_shifts() {
        let mut state = RichLogState::new();
        for index in 0..=RichLogState::DEFAULT_MAX_ENTRIES {
            state.push_plain(index.to_string());
        }

        assert_eq!(state.len(), RichLogState::DEFAULT_MAX_ENTRIES);
        assert_eq!(state.entry(0).unwrap().segments[0].0, "1");
        assert_eq!(
            state.entry(state.len() - 1).unwrap().segments[0].0,
            RichLogState::DEFAULT_MAX_ENTRIES.to_string()
        );
    }

    #[test]
    fn unbounded_state_retains_more_than_the_default_cap() {
        let mut state = RichLogState::new_unbounded();
        for index in 0..=RichLogState::DEFAULT_MAX_ENTRIES {
            state.push_plain(index.to_string());
        }

        assert_eq!(state.len(), RichLogState::DEFAULT_MAX_ENTRIES + 1);
        assert_eq!(state.entry(0).unwrap().segments[0].0, "0");
    }

    #[test]
    fn lowering_cap_evicts_in_bulk_on_the_next_push_and_adjusts_scroll() {
        let mut state = RichLogState::new_unbounded();
        state.auto_scroll = false;
        for index in 0..5 {
            state.push_plain(index.to_string());
        }
        state.scroll_offset = 4;
        state.max_entries = Some(2);

        state.push_plain("5");

        assert_eq!(texts(&state), ["4", "5"]);
        assert_eq!(state.scroll_offset, 0);
    }

    #[test]
    fn raising_and_disabling_cap_preserves_existing_order() {
        let mut state = RichLogState::new();
        state.max_entries = Some(1);
        state.push_plain("first");
        state.max_entries = Some(3);
        state.push_plain("second");
        state.push_plain("third");
        state.max_entries = None;
        state.push_plain("fourth");

        assert_eq!(texts(&state), ["first", "second", "third", "fourth"]);
    }

    #[test]
    fn entry_mutation_and_clear_preserve_state_contract() {
        let mut state = RichLogState::new();
        state.push_plain("before");
        state.entry_mut(0).unwrap().segments[0].0 = "after".to_owned();

        assert_eq!(texts(&state), ["after"]);

        state.clear();
        assert!(state.is_empty());
        assert_eq!(state.scroll_offset, 0);
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
    /// Monotonic content version, bumped on every content mutation
    /// (`push` / `start` / `clear`). See [`StreamingTextState::version`].
    pub(crate) version: u64,
}

impl StreamingTextState {
    /// Create a new empty streaming text state.
    pub fn new() -> Self {
        Self {
            content: String::new(),
            streaming: false,
            cursor_visible: true,
            cursor_tick: 0,
            version: 0,
        }
    }

    /// Append a chunk of text (e.g., from an LLM stream delta).
    pub fn push(&mut self, chunk: &str) {
        self.content.push_str(chunk);
        self.version = self.version.wrapping_add(1);
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
        self.version = self.version.wrapping_add(1);
    }

    /// Clear all content and reset state.
    pub fn clear(&mut self) {
        self.content.clear();
        self.streaming = false;
        self.cursor_visible = true;
        self.cursor_tick = 0;
        self.version = self.version.wrapping_add(1);
    }

    /// Monotonic version counter, bumped on every content mutation
    /// (`push` / `start` / `clear`).
    ///
    /// The stream itself changes every token, so this value is **not** a
    /// useful cache key for the streaming region. Its purpose is the
    /// inverse: it lets you detect when the stream *did* change so you can
    /// decide whether the *surrounding static chrome* is stable. Combine a
    /// hash of your non-streaming inputs into a key for
    /// [`ContainerBuilder::cached`](crate::ContainerBuilder::cached) and wrap
    /// the chrome — not the stream — in it.
    ///
    /// Since 0.21.0.
    ///
    /// # Example
    /// ```no_run
    /// let mut stream = slt::StreamingTextState::new();
    /// stream.push("hello");
    /// assert_eq!(stream.version(), 1);
    /// stream.push(" world");
    /// assert_eq!(stream.version(), 2);
    /// ```
    pub fn version(&self) -> u64 {
        self.version
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
    /// Monotonic content version, bumped on every content mutation
    /// (`push` / `start` / `clear`). See [`StreamingMarkdownState::version`].
    pub(crate) version: u64,
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
            version: 0,
        }
    }

    /// Append a markdown chunk (e.g., from an LLM stream delta).
    pub fn push(&mut self, chunk: &str) {
        self.content.push_str(chunk);
        self.version = self.version.wrapping_add(1);
    }

    /// Start a new streaming session, clearing previous content.
    pub fn start(&mut self) {
        self.content.clear();
        self.streaming = true;
        self.cursor_visible = true;
        self.cursor_tick = 0;
        self.in_code_block = false;
        self.code_block_lang.clear();
        self.version = self.version.wrapping_add(1);
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
        self.version = self.version.wrapping_add(1);
    }

    /// Monotonic version counter, bumped on every content mutation
    /// (`push` / `start` / `clear`).
    ///
    /// As with [`StreamingTextState::version`], use this to detect stream
    /// deltas and key the *surrounding static chrome* into
    /// [`ContainerBuilder::cached`](crate::ContainerBuilder::cached) — not to
    /// cache the stream region itself.
    ///
    /// Since 0.21.0.
    ///
    /// # Example
    /// ```no_run
    /// let mut md = slt::StreamingMarkdownState::new();
    /// md.push("# Title");
    /// assert_eq!(md.version(), 1);
    /// ```
    pub fn version(&self) -> u64 {
        self.version
    }
}

impl Default for StreamingMarkdownState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod streaming_version_tests {
    //! Issue #273 — the monotonic `version()` counter on streaming states.
    use super::{StreamingMarkdownState, StreamingTextState};

    #[test]
    fn text_version_starts_at_zero_and_bumps_on_mutation() {
        let mut s = StreamingTextState::new();
        assert_eq!(s.version(), 0, "fresh state has version 0");
        s.push("a");
        assert_eq!(s.version(), 1);
        s.push("b");
        assert_eq!(s.version(), 2);
        s.start();
        assert_eq!(s.version(), 3, "start() is a mutation");
        s.clear();
        assert_eq!(s.version(), 4, "clear() is a mutation");
    }

    #[test]
    fn text_finish_does_not_bump_version() {
        let mut s = StreamingTextState::new();
        s.push("x");
        let v = s.version();
        s.finish();
        assert_eq!(s.version(), v, "finish() only toggles the streaming flag");
    }

    #[test]
    fn markdown_version_bumps_on_mutation() {
        let mut s = StreamingMarkdownState::new();
        assert_eq!(s.version(), 0);
        s.push("# h");
        assert_eq!(s.version(), 1);
        s.start();
        assert_eq!(s.version(), 2);
        s.clear();
        assert_eq!(s.version(), 3);
        let v = s.version();
        s.finish();
        assert_eq!(s.version(), v, "finish() does not bump");
    }
}

/// Approval state for a tool call.
#[non_exhaustive]
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
