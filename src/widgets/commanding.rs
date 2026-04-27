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
    /// Cached filtered indices for the last `input` value. Avoids running
    /// `fuzzy_score` twice per frame (clamp + render).
    filter_cache: Option<(String, Vec<usize>)>,
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
            filter_cache: None,
        }
    }

    /// Toggle open/closed state and reset input when opening.
    pub fn toggle(&mut self) {
        self.open = !self.open;
        if self.open {
            self.input.clear();
            self.cursor = 0;
            self.selected = 0;
            self.filter_cache = None;
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

    /// Cached variant of [`Self::filtered_indices`].
    ///
    /// Reuses the previous result when `self.input` has not changed since the
    /// last call. `command_palette()` invokes this twice per frame (before key
    /// handling, to clamp the selection index, and again for render); on idle
    /// frames the second call is served from cache instead of re-running
    /// `fuzzy_score` over the full command list.
    pub(crate) fn filtered_indices_cached(&mut self) -> &[usize] {
        let needs_recompute = match &self.filter_cache {
            Some((cached_input, _)) => *cached_input != self.input,
            None => true,
        };
        if needs_recompute {
            let indices = self.filtered_indices();
            self.filter_cache = Some((self.input.clone(), indices));
        }
        &self
            .filter_cache
            .as_ref()
            .expect("filter_cache populated above")
            .1
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
/// Each screen gets isolated focus and hook state when used with
/// [`Context::screen`].
///
/// # Example
///
/// ```no_run
/// let mut screens = slt::ScreenState::new("main");
///
/// slt::run(|ui| {
///     let current = screens.current().to_string();
///     if current == "main" {
///         if ui.button("Settings").clicked { screens.push("settings"); }
///     }
///     if current == "settings" {
///         if ui.button("Back").clicked { screens.pop(); }
///     }
/// });
/// ```
#[derive(Debug, Clone)]
pub struct ScreenState {
    stack: Vec<String>,
    focus_state: std::collections::HashMap<String, (usize, usize)>,
}

impl ScreenState {
    /// Create a screen stack with an initial root screen.
    pub fn new(initial: impl Into<String>) -> Self {
        Self {
            stack: vec![initial.into()],
            focus_state: std::collections::HashMap::new(),
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

    pub(crate) fn save_focus(&mut self, name: &str, focus_index: usize, focus_count: usize) {
        self.focus_state
            .insert(name.to_string(), (focus_index, focus_count));
    }

    pub(crate) fn restore_focus(&self, name: &str) -> (usize, usize) {
        self.focus_state.get(name).copied().unwrap_or((0, 0))
    }
}

/// Named mode system with independent screen stacks.
///
/// Each mode contains its own [`ScreenState`]. Switching modes preserves
/// the previous mode's screen stack, focus, and hook state.
///
/// # Example
///
/// ```no_run
/// let mut modes = slt::ModeState::new("app", "home");
/// modes.add_mode("settings", "general");
///
/// slt::run(|ui| {
///     if ui.key('1') { modes.switch_mode("app"); }
///     if ui.key('2') { modes.switch_mode("settings"); }
///     let mode = modes.active_mode().to_string();
///     ui.text(format!("Mode: {}", mode));
/// });
/// ```
#[derive(Debug, Clone)]
pub struct ModeState {
    modes: std::collections::HashMap<String, ScreenState>,
    active: String,
}

impl ModeState {
    /// Create a mode system with an initial mode and screen.
    pub fn new(mode: impl Into<String>, screen: impl Into<String>) -> Self {
        let mode = mode.into();
        let mut modes = std::collections::HashMap::new();
        modes.insert(mode.clone(), ScreenState::new(screen));
        Self {
            modes,
            active: mode,
        }
    }

    /// Add a new mode with an initial screen.
    pub fn add_mode(&mut self, mode: impl Into<String>, screen: impl Into<String>) {
        let mode = mode.into();
        self.modes
            .entry(mode)
            .or_insert_with(|| ScreenState::new(screen));
    }

    /// Switch to a different mode. The mode must have been added with [`Self::add_mode`].
    ///
    /// Panics if the mode does not exist. For a non-panicking variant that
    /// reports success, use [`Self::try_switch_mode`].
    pub fn switch_mode(&mut self, mode: impl Into<String>) {
        let mode = mode.into();
        assert!(
            self.modes.contains_key(&mode),
            "mode '{}' not found",
            mode
        );
        self.active = mode;
    }

    /// Switch modes, returning `true` when the mode exists and the switch
    /// happened, or `false` when the mode has not been registered via
    /// [`Self::add_mode`].
    ///
    /// Prefer this over [`Self::switch_mode`] when the mode name comes from
    /// user input, key bindings, or anywhere the value could be unexpected
    /// at runtime — an unknown mode should not crash the host application.
    pub fn try_switch_mode(&mut self, mode: impl Into<String>) -> bool {
        let mode = mode.into();
        if !self.modes.contains_key(&mode) {
            return false;
        }
        self.active = mode;
        true
    }

    /// Return the active mode name.
    pub fn active_mode(&self) -> &str {
        &self.active
    }

    /// Get a reference to the active mode's screen state.
    pub fn screens(&self) -> &ScreenState {
        self.modes
            .get(&self.active)
            .expect("active mode must exist")
    }

    /// Get a mutable reference to the active mode's screen state.
    pub fn screens_mut(&mut self) -> &mut ScreenState {
        self.modes
            .get_mut(&self.active)
            .expect("active mode must exist")
    }
}

#[cfg(test)]
mod mode_state_tests {
    use super::ModeState;

    #[test]
    fn try_switch_mode_returns_false_for_unknown_mode() {
        let mut modes = ModeState::new("app", "home");
        modes.add_mode("settings", "general");
        assert!(modes.try_switch_mode("settings"));
        assert_eq!(modes.active_mode(), "settings");
        assert!(!modes.try_switch_mode("nonexistent"));
        // Active mode must not change when the switch is rejected.
        assert_eq!(modes.active_mode(), "settings");
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
