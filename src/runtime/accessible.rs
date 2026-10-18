//! Accessible linear output mode (issue #469).
//!
//! Screen readers read a repainted cell grid poorly or not at all. This
//! module drives the same frame closure as the visual runtimes, but turns
//! each frame into plain, sequential lines, the way Huh's accessible mode
//! replaces a form with standard prompts and Ink's screen-reader mode
//! linearizes its tree:
//!
//! - display text (`text`, rich text, links, container titles, toasts) is
//!   printed once when it appears or changes, rows joined into one line;
//! - every interactive widget records an [`A11yNode`] (role, name, value,
//!   options, errors) while it renders, keeping its own cells out of the
//!   text stream;
//! - focus moves are announced as `Focused: <name>, <role>`;
//! - the focused node becomes a one-line prompt, and the answer read from
//!   stdin is handed back to that widget on the next frame, which applies it
//!   to its state and reports `changed` / `submitted` / `clicked` exactly as
//!   a keyboard edit would;
//! - focus then moves to the next answerable widget, so a form is completed
//!   field by field.
//!
//! The visual runtimes never set [`FrameState::accessible`], so widgets pay a
//! single `bool` check per call there.

use std::io;

use unicode_segmentation::UnicodeSegmentation;

use super::FrameState;
use crate::Context;
use crate::layout::{Command, Direction};

/// Reserved `named_states` key holding the in-flight [`A11yFrame`].
pub(crate) const A11Y_FRAME_KEY: &str = "__slt_a11y_frame";

/// Environment variable that enables accessible mode by default.
pub(crate) const ACCESSIBLE_ENV: &str = "SLT_ACCESSIBLE";

/// `true` when `SLT_ACCESSIBLE` is `1`, `true`, `yes` or `on` (any case).
pub(crate) fn accessible_from_env() -> bool {
    std::env::var(ACCESSIBLE_ENV).is_ok_and(|value| env_truthy(&value))
}

fn env_truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Semantic role of an interactive widget, in the spirit of ARIA roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum A11yRole {
    /// Single-line text input.
    TextField,
    /// Masked single-line text input. Its value is never recorded.
    Password,
    /// Multi-line text input.
    TextArea,
    /// Dropdown select.
    Select,
    /// Radio group.
    Radio,
    /// Multi-select list.
    MultiSelect,
    /// Checkbox.
    CheckBox,
    /// On/off toggle switch.
    Switch,
    /// Yes/No confirmation.
    Confirm,
    /// Button.
    Button,
}

impl A11yRole {
    fn name(self) -> &'static str {
        match self {
            Self::TextField => "text field",
            Self::Password => "password field",
            Self::TextArea => "multi-line text field",
            Self::Select => "select",
            Self::Radio => "radio group",
            Self::MultiSelect => "multi-select list",
            Self::CheckBox => "check box",
            Self::Switch => "switch",
            Self::Confirm => "yes or no question",
            Self::Button => "button",
        }
    }
}

/// An answer read from a prompt, applied by the widget on the next frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum A11yAnswer {
    /// Replace a text value and submit it.
    Text(String),
    /// Submit a text field without changing its value.
    Submit,
    /// Choose option `n` (zero-based) of a select or radio group.
    Choice(usize),
    /// Toggle option `n` (zero-based) of a multi-select.
    Toggle(usize),
    /// Set a checkbox, toggle or confirmation.
    Bool(bool),
    /// Activate a button.
    Activate,
}

/// What a widget reports about itself.
#[derive(Debug, Clone, Default)]
pub(crate) struct A11yInfo {
    /// Widget-intrinsic name (checkbox / button label, confirm question).
    pub label: Option<String>,
    pub placeholder: Option<String>,
    /// Current text value. `None` for masked inputs.
    pub value: Option<String>,
    pub options: Vec<String>,
    pub selected: Vec<usize>,
    pub checked: Option<bool>,
    pub errors: Vec<String>,
    pub max_len: Option<usize>,
}

/// One interactive widget recorded during a frame.
#[derive(Debug, Clone)]
pub(crate) struct A11yNode {
    pub role: A11yRole,
    /// Focus slot, `None` when focus is suppressed (outside an open modal).
    pub slot: Option<usize>,
    pub focused: bool,
    /// Name set by [`Context::a11y_label`] or a form field label.
    pub name: Option<String>,
    pub info: A11yInfo,
    /// Command range the widget emitted; kept out of the text stream.
    pub start: usize,
    pub end: usize,
}

impl A11yNode {
    /// Accessible name: explicit name, intrinsic label, placeholder, role.
    fn display_name(&self) -> String {
        let name = self
            .name
            .as_deref()
            .or(self.info.label.as_deref())
            .or(self.info.placeholder.as_deref())
            .map(clean_label)
            .filter(|name| !name.is_empty());
        match name {
            Some(name) => name,
            None => match self.role {
                A11yRole::Password => "Password".to_string(),
                A11yRole::Confirm => "Confirm".to_string(),
                A11yRole::Button => "Button".to_string(),
                _ => "Input".to_string(),
            },
        }
    }

    fn same_widget(&self, other: &Self) -> bool {
        self.role == other.role && self.display_name() == other.display_name()
    }
}

/// Linearized frame item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum A11yItem {
    Line(String),
    Node(usize),
}

/// Per-frame accessible state, stored under [`A11Y_FRAME_KEY`].
#[derive(Debug, Default)]
pub(crate) struct A11yFrame {
    pub nodes: Vec<A11yNode>,
    pub pending_name: Option<String>,
    /// Answer for the widget in slot `.0`, taken by that widget.
    pub answer: Option<(usize, A11yAnswer)>,
    pub items: Vec<A11yItem>,
}

/// Bookkeeping a widget carries between [`Context::a11y_begin`] and
/// [`Context::a11y_end`].
#[derive(Debug)]
pub(crate) struct A11yBegin {
    start: usize,
    slot: Option<usize>,
    focused: bool,
    name: Option<String>,
    /// Answer addressed to this widget, if any. Widgets `take()` it.
    pub answer: Option<A11yAnswer>,
}

impl Context {
    /// Whether this frame runs in accessible linear output mode.
    ///
    /// `true` under [`RunConfig::accessible`](crate::RunConfig::accessible)
    /// (or `SLT_ACCESSIBLE=1`), where the frame is printed as plain lines and
    /// interactive widgets are answered as sequential prompts. Use it to drop
    /// visual-only hints such as key legends. Like Ink's
    /// `useIsScreenReaderEnabled`.
    ///
    /// # Example
    ///
    /// ```
    /// # slt::TestBackend::new(40, 4).render(|ui| {
    /// if !ui.is_accessible() {
    ///     ui.text("Tab: next field  Enter: submit");
    /// }
    /// # });
    /// ```
    pub fn is_accessible(&self) -> bool {
        self.accessible
    }

    /// Set the accessible name of the next interactive widget.
    ///
    /// The name is announced on focus and used as the prompt in accessible
    /// mode, overriding the widget's own label, the preceding text line and
    /// the placeholder. Applies to `text_input`, `textarea`, `select`,
    /// `radio`, `multi_select`, `checkbox`, `toggle`, `confirm`, `button`
    /// and `form_field`. Outside accessible mode it records nothing.
    ///
    /// # Example
    ///
    /// ```
    /// # use slt::widgets::TextInputState;
    /// # let mut email = TextInputState::new();
    /// # slt::TestBackend::new(40, 4).render(|ui| {
    /// let _ = ui.a11y_label("Email address").text_input(&mut email);
    /// # });
    /// ```
    pub fn a11y_label(&mut self, label: impl Into<String>) -> &mut Self {
        if self.accessible
            && let Some(frame) = self.a11y_frame_mut()
        {
            frame.pending_name = Some(label.into());
        }
        self
    }

    fn a11y_frame_mut(&mut self) -> Option<&mut A11yFrame> {
        self.named_states
            .get_mut(A11Y_FRAME_KEY)
            .and_then(|boxed| boxed.downcast_mut::<A11yFrame>())
    }

    /// Start recording an interactive widget. Call right after the widget's
    /// focus registration. `None` (one `bool` check) outside accessible mode.
    #[inline]
    pub(crate) fn a11y_begin(&mut self, focused: bool) -> Option<A11yBegin> {
        if !self.accessible {
            return None;
        }
        self.a11y_begin_slow(focused)
    }

    #[cold]
    fn a11y_begin_slow(&mut self, focused: bool) -> Option<A11yBegin> {
        let slot = self.rollback.last_focusable_id;
        let start = self.commands.len();
        let frame = self.a11y_frame_mut()?;
        let name = frame.pending_name.take();
        let addressed = focused
            && slot.is_some()
            && frame.answer.as_ref().map(|(target, _)| Some(*target)) == Some(slot);
        let answer = if addressed {
            frame.answer.take().map(|(_, answer)| answer)
        } else {
            None
        };
        Some(A11yBegin {
            start,
            slot,
            focused,
            name,
            answer,
        })
    }

    /// Finish recording a widget started with [`a11y_begin`](Self::a11y_begin).
    #[cold]
    pub(crate) fn a11y_end(&mut self, begin: A11yBegin, role: A11yRole, info: A11yInfo) {
        let end = self.commands.len();
        if let Some(frame) = self.a11y_frame_mut() {
            frame.nodes.push(A11yNode {
                role,
                slot: begin.slot,
                focused: begin.focused,
                name: begin.name,
                info,
                start: begin.start,
                end,
            });
        }
    }

    /// Name the next widget unless a name is already pending (form fields).
    #[cold]
    pub(crate) fn a11y_default_name(&mut self, name: &str) {
        if let Some(frame) = self.a11y_frame_mut()
            && frame.pending_name.is_none()
        {
            frame.pending_name = Some(name.to_string());
        }
    }

    /// Widen the last node recorded since `start` to cover `start..` and add
    /// `error` (form field label and error text).
    #[cold]
    pub(crate) fn a11y_extend_last(&mut self, start: usize, error: Option<&str>) {
        let end = self.commands.len();
        if let Some(frame) = self.a11y_frame_mut()
            && let Some(node) = frame.nodes.last_mut()
            && node.start >= start
        {
            node.start = start;
            node.end = node.end.max(end);
            if let Some(error) = error
                && !node.info.errors.iter().any(|known| known == error)
            {
                node.info.errors.push(error.to_string());
            }
        }
    }

    /// Linearize this frame's commands into lines and widget slots. Runs at
    /// the end of the frame closure, before layout drains the commands.
    pub(crate) fn a11y_harvest(&mut self) {
        if !self.accessible {
            return;
        }
        let Some(frame) = self
            .named_states
            .get_mut(A11Y_FRAME_KEY)
            .and_then(|boxed| boxed.downcast_mut::<A11yFrame>())
        else {
            return;
        };
        harvest(&self.commands, frame);
        for (message, _, _) in &self.rollback.notification_queue {
            push_plain_lines(&mut frame.items, message);
        }
    }
}

/// Widget adapters: apply an answer and describe the widget. Kept here so
/// each widget method only gains a `begin` / `apply` / `end` hook.
impl Context {
    #[cold]
    pub(crate) fn a11y_end_text_input(
        &mut self,
        begin: A11yBegin,
        state: &crate::widgets::TextInputState,
    ) {
        let mut errors = state.errors().to_vec();
        if errors.is_empty()
            && let Some(error) = &state.validation_error
        {
            errors.push(error.clone());
        }
        let role = if state.masked {
            A11yRole::Password
        } else {
            A11yRole::TextField
        };
        let info = A11yInfo {
            placeholder: non_empty(&state.placeholder),
            value: (!state.masked).then(|| state.value.clone()),
            errors,
            max_len: state.max_length,
            ..A11yInfo::default()
        };
        self.a11y_end(begin, role, info);
    }

    #[cold]
    pub(crate) fn a11y_end_textarea(
        &mut self,
        begin: A11yBegin,
        state: &crate::widgets::TextareaState,
    ) {
        let info = A11yInfo {
            value: Some(state.lines.join("\n")),
            max_len: state.max_length,
            ..A11yInfo::default()
        };
        self.a11y_end(begin, A11yRole::TextArea, info);
    }

    #[cold]
    pub(crate) fn a11y_end_choice(
        &mut self,
        begin: A11yBegin,
        role: A11yRole,
        options: &[String],
        selected: Vec<usize>,
        placeholder: &str,
    ) {
        let info = A11yInfo {
            placeholder: non_empty(placeholder),
            options: options.to_vec(),
            selected,
            ..A11yInfo::default()
        };
        self.a11y_end(begin, role, info);
    }

    #[cold]
    pub(crate) fn a11y_end_labelled(
        &mut self,
        begin: A11yBegin,
        role: A11yRole,
        label: &str,
        checked: Option<bool>,
    ) {
        let info = A11yInfo {
            label: non_empty(label),
            checked,
            ..A11yInfo::default()
        };
        self.a11y_end(begin, role, info);
    }
}

fn non_empty(text: &str) -> Option<String> {
    (!text.trim().is_empty()).then(|| text.to_string())
}

impl A11yBegin {
    /// Take a text answer: `Some(Some(text))` replaces the value,
    /// `Some(None)` submits it unchanged.
    pub(crate) fn take_text(begin: Option<&mut Self>) -> Option<Option<String>> {
        match begin?.answer.take()? {
            A11yAnswer::Text(text) => Some(Some(text)),
            A11yAnswer::Submit => Some(None),
            _ => None,
        }
    }

    /// Take a select / radio choice.
    pub(crate) fn take_choice(begin: Option<&mut Self>) -> Option<usize> {
        match begin?.answer.take()? {
            A11yAnswer::Choice(index) => Some(index),
            _ => None,
        }
    }

    /// Take a multi-select toggle.
    pub(crate) fn take_toggle(begin: Option<&mut Self>) -> Option<usize> {
        match begin?.answer.take()? {
            A11yAnswer::Toggle(index) => Some(index),
            _ => None,
        }
    }

    /// Take a yes / no answer.
    pub(crate) fn take_bool(begin: Option<&mut Self>) -> Option<bool> {
        match begin?.answer.take()? {
            A11yAnswer::Bool(value) => Some(value),
            _ => None,
        }
    }

    /// Take a button activation.
    pub(crate) fn take_activate(begin: Option<&mut Self>) -> bool {
        begin.is_some_and(|begin| begin.answer.take() == Some(A11yAnswer::Activate))
    }
}

/// Strip decoration and a trailing colon from a label.
fn clean_label(label: &str) -> String {
    label
        .trim()
        .trim_end_matches(':')
        .trim_end()
        .chars()
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .collect()
}

/// Replace control characters so printed text cannot carry escapes.
pub(crate) fn plain_text(input: &str) -> String {
    input
        .chars()
        .map(|ch| {
            if ch.is_control() || ('\u{80}'..='\u{9f}').contains(&ch) {
                '?'
            } else {
                ch
            }
        })
        .collect()
}

/// Lines with no letter or digit (rules, box drawing, bullets, spinners)
/// carry nothing for a listener.
fn is_meaningful(line: &str) -> bool {
    line.chars().any(char::is_alphanumeric)
}

fn push_plain_lines(items: &mut Vec<A11yItem>, text: &str) {
    for line in text.lines() {
        let line = plain_text(line.trim());
        if is_meaningful(&line) {
            items.push(A11yItem::Line(line));
        }
    }
}

struct LineBuilder {
    /// One entry per open container: `true` for rows.
    rows: Vec<bool>,
    line: String,
}

impl LineBuilder {
    fn in_row(&self) -> bool {
        self.rows.iter().any(|row| *row)
    }

    fn text(&mut self, items: &mut Vec<A11yItem>, text: &str) {
        if self.in_row() && !text.contains('\n') {
            let piece = text.trim();
            if piece.is_empty() {
                return;
            }
            if !self.line.is_empty() {
                self.line.push(' ');
            }
            self.line.push_str(piece);
        } else {
            self.flush(items);
            push_plain_lines(items, text);
        }
    }

    fn flush(&mut self, items: &mut Vec<A11yItem>) {
        if !self.line.is_empty() {
            let line = std::mem::take(&mut self.line);
            push_plain_lines(items, &line);
        }
    }

    fn open(&mut self, items: &mut Vec<A11yItem>, row: bool, title: Option<&str>) {
        if let Some(title) = title {
            self.flush(items);
            push_plain_lines(items, title);
        }
        self.rows.push(row);
    }

    fn close(&mut self, items: &mut Vec<A11yItem>) {
        self.rows.pop();
        if !self.in_row() {
            self.flush(items);
        }
    }
}

fn harvest(commands: &[Command], frame: &mut A11yFrame) {
    let items = &mut frame.items;
    items.clear();
    let mut order: Vec<usize> = (0..frame.nodes.len()).collect();
    order.sort_by_key(|&index| (frame.nodes[index].start, index));
    let mut next = 0;
    let mut builder = LineBuilder {
        rows: Vec::new(),
        line: String::new(),
    };
    let mut index = 0;
    while index <= commands.len() {
        // Nodes that start here claim their command range. A node nested
        // inside another node's range was already skipped with it.
        let mut claimed = None;
        while next < order.len() && frame.nodes[order[next]].start <= index {
            let node = &frame.nodes[order[next]];
            if node.start == index {
                builder.flush(items);
                items.push(A11yItem::Node(order[next]));
                claimed = Some(claimed.map_or(node.end, |end: usize| end.max(node.end)));
            }
            next += 1;
        }
        if let Some(end) = claimed {
            // Containers opened and closed inside the widget are balanced,
            // so the row stack is unaffected by skipping them.
            index = end.max(index + 1);
            continue;
        }
        let Some(command) = commands.get(index) else {
            break;
        };
        match command {
            Command::Text { content, .. } => builder.text(items, content),
            Command::RichText { segments, .. } => {
                let joined: String = segments.iter().map(|(text, _)| text.as_str()).collect();
                builder.text(items, &joined);
            }
            Command::Link { text, .. } => builder.text(items, text),
            Command::BeginContainer(args) => builder.open(
                items,
                args.direction == Direction::Row,
                args.title.as_ref().map(|(title, _)| title.as_str()),
            ),
            Command::BeginScrollable(args) => builder.open(
                items,
                args.direction == Direction::Row,
                args.title.as_ref().map(|(title, _)| title.as_str()),
            ),
            Command::BeginOverlay { .. } => builder.open(items, false, None),
            Command::EndContainer | Command::EndOverlay => builder.close(items),
            _ => {}
        }
        index += 1;
    }
    builder.flush(items);
}

/// Line-oriented input and output used by the accessible session.
#[cfg(any(feature = "crossterm", test))]
pub(crate) trait A11yIo {
    /// Write `text` and flush.
    fn write_str(&mut self, text: &str) -> io::Result<()>;
    /// Write `prompt`, then read one line without its terminator. `masked`
    /// input must not be echoed, so echo is switched off before the prompt
    /// appears. `None` ends the session (end of input or Ctrl+C).
    fn ask(&mut self, prompt: &str, masked: bool) -> io::Result<Option<String>>;
}

#[cfg(any(feature = "crossterm", test))]
pub(crate) use session::run_accessible_session;

#[cfg(any(feature = "crossterm", test))]
mod session {
    use super::*;
    use crate::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
    use crate::{Buffer, RunConfig};

    /// What the prompt for a node produced.
    enum Outcome {
        /// Apply the answer; `advance` moves focus on once it settles.
        Answer {
            answer: A11yAnswer,
            advance: bool,
            kept: bool,
        },
        /// Move to the next answerable widget without answering.
        Skip,
        /// Invalid input, already explained; ask again.
        Again,
        Quit,
    }

    struct Session<'a, I: A11yIo, F: FnMut(&mut Context)> {
        io: &'a mut I,
        config: &'a RunConfig,
        f: F,
        state: FrameState,
        buffer: Buffer,
        size: (u32, u32),
        printed: Vec<String>,
        announced: Option<(Option<usize>, A11yRole, String)>,
        raw_notice: bool,
        /// The last answer was invalid: ask again without re-listing options.
        repeat: bool,
    }

    /// Drive `f` as an accessible line-oriented session until it quits or
    /// input ends. `extra_lines` supplies lines printed before each frame's
    /// output (`run_static` scrollback).
    pub(crate) fn run_accessible_session<I: A11yIo>(
        io: &mut I,
        config: &RunConfig,
        size: (u32, u32),
        f: impl FnMut(&mut Context),
        extra_lines: &mut dyn FnMut() -> Vec<String>,
    ) -> io::Result<()> {
        let state = FrameState {
            accessible: true,
            ..FrameState::default()
        };
        let size = (size.0.max(1), size.1.max(1));
        let mut session = Session {
            io,
            config,
            f,
            state,
            buffer: Buffer::empty(crate::rect::Rect::new(0, 0, size.0, size.1)),
            size,
            printed: Vec::new(),
            announced: None,
            raw_notice: false,
            repeat: false,
        };
        session.run(extra_lines)
    }

    impl<I: A11yIo, F: FnMut(&mut Context)> Session<'_, I, F> {
        fn run(&mut self, extra: &mut dyn FnMut() -> Vec<String>) -> io::Result<()> {
            let Some(mut frame) = self.render(Vec::new(), None, extra)? else {
                return Ok(());
            };
            let mut retargets = 0;
            loop {
                let focused = frame
                    .nodes
                    .iter()
                    .position(|node| node.focused && node.slot.is_some());
                let Some(index) = focused else {
                    // Focus sits on a widget without a prompt (or nowhere):
                    // move to the next answerable widget, once or twice.
                    if retargets < 2
                        && let Some(slot) = next_slot(&frame.nodes, self.focus_slot(), true)
                    {
                        retargets += 1;
                        self.set_focus(slot);
                        match self.render(Vec::new(), None, extra)? {
                            Some(next) => frame = next,
                            None => return Ok(()),
                        }
                        continue;
                    }
                    match self.raw_keys(extra)? {
                        Some(next) => frame = next,
                        None => return Ok(()),
                    }
                    retargets = 0;
                    continue;
                };
                retargets = 0;
                self.raw_notice = false;
                let node = labelled(&frame, index);
                self.announce(&node)?;
                let slot = node.slot.unwrap_or_default();
                let outcome = self.prompt(&node)?;
                self.repeat = matches!(outcome, Outcome::Again);
                match outcome {
                    Outcome::Quit => return Ok(()),
                    Outcome::Again => continue,
                    Outcome::Skip => {
                        if let Some(next) = next_slot(&frame.nodes, slot, false) {
                            self.set_focus(next);
                        }
                        match self.render(Vec::new(), None, extra)? {
                            Some(next) => frame = next,
                            None => return Ok(()),
                        }
                    }
                    Outcome::Answer {
                        answer,
                        advance,
                        kept,
                    } => {
                        let focus_before = self.state.focus.focus_index;
                        // Answer frame: the widget applies the answer and the
                        // app reacts. Settle frame: app-side validation and
                        // screen changes become visible.
                        if self
                            .render(Vec::new(), Some((slot, answer)), extra)?
                            .is_none()
                        {
                            return Ok(());
                        }
                        let Some(settled) = self.render(Vec::new(), None, extra)? else {
                            return Ok(());
                        };
                        frame = settled;
                        let same = frame
                            .nodes
                            .iter()
                            .position(|candidate| candidate.slot == Some(slot))
                            .map(|at| labelled(&frame, at))
                            .filter(|candidate| candidate.same_widget(&node));
                        let Some(current) = same else {
                            // The answer replaced the UI; prompt what is there.
                            continue;
                        };
                        if self.state.focus.focus_index != focus_before {
                            // The app moved focus itself.
                            continue;
                        }
                        if !current.info.errors.is_empty() {
                            for error in &current.info.errors {
                                self.line(&format!("Error: {}", plain_text(error)))?;
                            }
                            if !kept {
                                continue;
                            }
                        }
                        if !advance {
                            continue;
                        }
                        if let Some(next) = next_slot(&frame.nodes, slot, false) {
                            self.set_focus(next);
                            match self.render(Vec::new(), None, extra)? {
                                Some(next) => frame = next,
                                None => return Ok(()),
                            }
                        }
                    }
                }
            }
        }

        fn focus_slot(&self) -> usize {
            let count = self.state.focus.prev_focus_count.max(1);
            self.state.focus.focus_index % count
        }

        fn set_focus(&mut self, slot: usize) {
            self.state.focus.focus_index = slot;
        }

        fn line(&mut self, text: &str) -> io::Result<()> {
            let mut out = String::with_capacity(text.len() + 1);
            out.push_str(text);
            out.push('\n');
            self.io.write_str(&out)
        }

        /// Run one frame and print its new lines. `None` once the app quit.
        fn render(
            &mut self,
            events: Vec<Event>,
            answer: Option<(usize, A11yAnswer)>,
            extra: &mut dyn FnMut() -> Vec<String>,
        ) -> io::Result<Option<A11yFrame>> {
            for line in extra() {
                self.line(&plain_text(&line))?;
            }
            self.state.named_states.insert(
                A11Y_FRAME_KEY,
                Box::new(A11yFrame {
                    answer,
                    ..A11yFrame::default()
                }),
            );
            self.buffer.reset();
            let f = &mut self.f;
            let result = crate::runtime::run_frame_kernel(
                &mut self.buffer,
                &mut self.state,
                self.config,
                self.size,
                events,
                false,
                &mut |ui: &mut Context| {
                    f(ui);
                    ui.a11y_harvest();
                },
            );
            self.state.diagnostics.tick = self.state.diagnostics.tick.wrapping_add(1);
            let frame = self
                .state
                .named_states
                .remove(A11Y_FRAME_KEY)
                .and_then(|boxed| boxed.downcast::<A11yFrame>().ok())
                .map(|boxed| *boxed)
                .unwrap_or_default();
            for line in self.take_static_log() {
                self.line(&plain_text(&line))?;
            }
            self.print_lines(&frame)?;
            if result.should_quit {
                return Ok(None);
            }
            Ok(Some(frame))
        }

        fn take_static_log(&mut self) -> Vec<String> {
            self.state
                .named_states
                .get_mut(crate::runtime::STATIC_LOG_NAMED_STATE_KEY)
                .and_then(|boxed| boxed.downcast_mut::<Vec<String>>())
                .map(std::mem::take)
                .unwrap_or_default()
        }

        /// Print lines that were not on the previous frame (multiset diff).
        fn print_lines(&mut self, frame: &A11yFrame) -> io::Result<()> {
            let lines = visible_lines(frame);
            let mut previous = std::mem::take(&mut self.printed);
            for line in &lines {
                if let Some(at) = previous.iter().position(|seen| seen == line) {
                    previous.swap_remove(at);
                } else {
                    self.line(line)?;
                }
            }
            self.printed = lines;
            Ok(())
        }

        fn announce(&mut self, node: &A11yNode) -> io::Result<()> {
            let name = node.display_name();
            let key = (node.slot, node.role, name.clone());
            if self.announced.as_ref() == Some(&key) {
                return Ok(());
            }
            self.announced = Some(key);
            let detail = match node.role {
                A11yRole::Select | A11yRole::Radio | A11yRole::MultiSelect => {
                    let count = node.info.options.len();
                    let noun = if count == 1 { "option" } else { "options" };
                    format!(", {count} {noun}")
                }
                A11yRole::CheckBox => {
                    if node.info.checked == Some(true) {
                        ", checked".to_string()
                    } else {
                        ", not checked".to_string()
                    }
                }
                A11yRole::Switch => {
                    if node.info.checked == Some(true) {
                        ", on".to_string()
                    } else {
                        ", off".to_string()
                    }
                }
                _ => String::new(),
            };
            self.line(&format!("Focused: {name}, {}{detail}", node.role.name()))?;
            for error in &node.info.errors {
                self.line(&format!("Error: {}", plain_text(error)))?;
            }
            Ok(())
        }

        fn ask(&mut self, prompt: &str, masked: bool) -> io::Result<Option<String>> {
            self.io.ask(prompt, masked)
        }

        fn prompt(&mut self, node: &A11yNode) -> io::Result<Outcome> {
            let name = node.display_name();
            match node.role {
                A11yRole::TextField | A11yRole::Password => {
                    let masked = node.role == A11yRole::Password;
                    let current = node.info.value.as_deref().unwrap_or_default();
                    let prompt = if masked {
                        format!("{name} (hidden): ")
                    } else if current.is_empty() {
                        format!("{name}: ")
                    } else {
                        format!("{name} [{}]: ", plain_text(current))
                    };
                    let Some(answer) = self.ask(&prompt, masked)? else {
                        return Ok(Outcome::Quit);
                    };
                    if answer.is_empty() {
                        return Ok(Outcome::Answer {
                            answer: A11yAnswer::Submit,
                            advance: true,
                            kept: true,
                        });
                    }
                    if let Some(max) = node.info.max_len
                        && answer.graphemes(true).count() > max
                    {
                        self.line(&format!("Invalid: must be at most {max} characters"))?;
                        return Ok(Outcome::Again);
                    }
                    Ok(Outcome::Answer {
                        answer: A11yAnswer::Text(answer),
                        advance: true,
                        kept: false,
                    })
                }
                A11yRole::TextArea => {
                    let Some(first) =
                        self.ask(&format!("{name} (end with an empty line): "), false)?
                    else {
                        return Ok(Outcome::Quit);
                    };
                    if first.is_empty() {
                        return Ok(Outcome::Answer {
                            answer: A11yAnswer::Submit,
                            advance: true,
                            kept: true,
                        });
                    }
                    let mut lines = vec![first];
                    while let Some(more) = self.ask("> ", false)? {
                        if more.is_empty() {
                            break;
                        }
                        lines.push(more);
                    }
                    let text = lines.join("\n");
                    if let Some(max) = node.info.max_len
                        && text.graphemes(true).count() > max
                    {
                        self.line(&format!("Invalid: must be at most {max} characters"))?;
                        return Ok(Outcome::Again);
                    }
                    Ok(Outcome::Answer {
                        answer: A11yAnswer::Text(text),
                        advance: true,
                        kept: false,
                    })
                }
                A11yRole::Select | A11yRole::Radio => self.prompt_choice(node),
                A11yRole::MultiSelect => self.prompt_multi(node),
                A11yRole::CheckBox | A11yRole::Switch | A11yRole::Confirm => {
                    let current = node.info.checked.unwrap_or(false);
                    let hint = if current { "[Y/n]" } else { "[y/N]" };
                    let Some(answer) = self.ask(&format!("{name} {hint}: "), false)? else {
                        return Ok(Outcome::Quit);
                    };
                    let value = match answer.trim().to_ascii_lowercase().as_str() {
                        "" => current,
                        "y" | "yes" => true,
                        "n" | "no" => false,
                        _ => {
                            self.line("Invalid: please answer y or n")?;
                            return Ok(Outcome::Again);
                        }
                    };
                    let state = match (node.role, value) {
                        (A11yRole::CheckBox, true) => Some("checked"),
                        (A11yRole::CheckBox, false) => Some("not checked"),
                        (A11yRole::Switch, true) => Some("on"),
                        (A11yRole::Switch, false) => Some("off"),
                        _ => None,
                    };
                    if let Some(state) = state {
                        self.line(&format!("{name}: {state}"))?;
                    }
                    Ok(Outcome::Answer {
                        answer: A11yAnswer::Bool(value),
                        advance: true,
                        kept: false,
                    })
                }
                A11yRole::Button => {
                    let Some(answer) = self.ask(
                        &format!("Press Enter to activate {name}, or type s to skip: "),
                        false,
                    )?
                    else {
                        return Ok(Outcome::Quit);
                    };
                    match answer.trim().to_ascii_lowercase().as_str() {
                        "" => Ok(Outcome::Answer {
                            answer: A11yAnswer::Activate,
                            advance: true,
                            kept: false,
                        }),
                        "s" | "skip" => Ok(Outcome::Skip),
                        _ => {
                            self.line("Invalid: press Enter to activate, or type s to skip")?;
                            Ok(Outcome::Again)
                        }
                    }
                }
            }
        }

        fn print_options(&mut self, node: &A11yNode) -> io::Result<()> {
            if self.repeat {
                return Ok(());
            }
            for (index, option) in node.info.options.iter().enumerate() {
                let marker = if node.info.selected.contains(&index) {
                    " (selected)"
                } else {
                    ""
                };
                self.line(&format!("{}. {}{marker}", index + 1, plain_text(option)))?;
            }
            Ok(())
        }

        fn prompt_choice(&mut self, node: &A11yNode) -> io::Result<Outcome> {
            let count = node.info.options.len();
            if count == 0 {
                self.line("No options available.")?;
                return Ok(Outcome::Skip);
            }
            self.print_options(node)?;
            let current = node
                .info
                .selected
                .first()
                .copied()
                .unwrap_or(0)
                .min(count - 1);
            let prompt = if count == 1 {
                format!("There is only one option; enter 1 [{}]: ", current + 1)
            } else {
                format!("Enter a number between 1 and {count} [{}]: ", current + 1)
            };
            let Some(answer) = self.ask(&prompt, false)? else {
                return Ok(Outcome::Quit);
            };
            let trimmed = answer.trim();
            let choice = if trimmed.is_empty() {
                current
            } else {
                match trimmed.parse::<usize>() {
                    Ok(number) if (1..=count).contains(&number) => number - 1,
                    _ => {
                        self.line(&format!("Invalid: must be a number between 1 and {count}"))?;
                        return Ok(Outcome::Again);
                    }
                }
            };
            self.line(&format!(
                "Selected: {}",
                plain_text(&node.info.options[choice])
            ))?;
            Ok(Outcome::Answer {
                answer: A11yAnswer::Choice(choice),
                advance: true,
                kept: false,
            })
        }

        fn prompt_multi(&mut self, node: &A11yNode) -> io::Result<Outcome> {
            let count = node.info.options.len();
            if !self.repeat {
                self.print_options(node)?;
                self.line("0. Done")?;
            }
            let prompt = format!("Enter a number between 0 and {count} to toggle, 0 when done: ");
            let Some(answer) = self.ask(&prompt, false)? else {
                return Ok(Outcome::Quit);
            };
            let trimmed = answer.trim();
            if trimmed.is_empty() || trimmed == "0" {
                return Ok(Outcome::Skip);
            }
            match trimmed.parse::<usize>() {
                Ok(number) if (1..=count).contains(&number) => Ok(Outcome::Answer {
                    answer: A11yAnswer::Toggle(number - 1),
                    advance: false,
                    kept: false,
                }),
                _ => {
                    self.line(&format!("Invalid: must be a number between 0 and {count}"))?;
                    Ok(Outcome::Again)
                }
            }
        }

        /// No answerable widget: forward a typed line as key presses.
        fn raw_keys(
            &mut self,
            extra: &mut dyn FnMut() -> Vec<String>,
        ) -> io::Result<Option<A11yFrame>> {
            if !self.raw_notice {
                self.raw_notice = true;
                self.announced = None;
                self.line(
                    "No prompt is active. Type keys and press Enter to send them; \
                     an empty line sends Enter. End input to quit.",
                )?;
            }
            let Some(line) = self.ask("> ", false)? else {
                return Ok(None);
            };
            let key = |code| {
                Event::Key(KeyEvent {
                    code,
                    modifiers: KeyModifiers::NONE,
                    kind: KeyEventKind::Press,
                })
            };
            let events = if line.is_empty() {
                vec![key(KeyCode::Enter)]
            } else {
                line.chars()
                    .map(|ch| match ch {
                        '\t' => key(KeyCode::Tab),
                        ch => key(KeyCode::Char(ch)),
                    })
                    .collect()
            };
            self.render(events, None, extra)
        }
    }

    /// The node with the preceding text line as its name when it has none.
    fn labelled(frame: &A11yFrame, index: usize) -> A11yNode {
        let mut node = frame.nodes[index].clone();
        if node.name.is_none()
            && node.info.label.is_none()
            && let Some(line) = preceding_line(frame, index)
        {
            node.name = Some(line.to_string());
        }
        node
    }

    fn preceding_line(frame: &A11yFrame, index: usize) -> Option<&str> {
        let at = frame
            .items
            .iter()
            .position(|item| *item == A11yItem::Node(index))?;
        match frame.items.get(at.checked_sub(1)?)? {
            A11yItem::Line(line) => Some(line.as_str()),
            A11yItem::Node(_) => None,
        }
    }

    /// Lines to print: every text line except those that name the widget
    /// right after them.
    fn visible_lines(frame: &A11yFrame) -> Vec<String> {
        let mut lines = Vec::new();
        for (at, item) in frame.items.iter().enumerate() {
            let A11yItem::Line(line) = item else {
                continue;
            };
            let names_next = matches!(
                frame.items.get(at + 1),
                Some(A11yItem::Node(index))
                    if frame.nodes[*index].name.is_none() && frame.nodes[*index].info.label.is_none()
            );
            if !names_next {
                lines.push(line.clone());
            }
        }
        lines
    }

    /// Next answerable slot after `slot` (or at it when `inclusive`),
    /// wrapping to the first one.
    fn next_slot(nodes: &[A11yNode], slot: usize, inclusive: bool) -> Option<usize> {
        let mut slots: Vec<usize> = nodes.iter().filter_map(|node| node.slot).collect();
        slots.sort_unstable();
        slots.dedup();
        slots
            .iter()
            .copied()
            .find(|&candidate| candidate > slot || (inclusive && candidate == slot))
            .or_else(|| slots.first().copied())
    }
}

/// Standard input / output for the native accessible runtime.
#[cfg(feature = "crossterm")]
pub(crate) struct StdA11yIo {
    stdin_is_terminal: bool,
}

#[cfg(feature = "crossterm")]
impl StdA11yIo {
    pub(crate) fn new() -> Self {
        use std::io::IsTerminal;
        Self {
            stdin_is_terminal: io::stdin().is_terminal(),
        }
    }
}

#[cfg(feature = "crossterm")]
impl A11yIo for StdA11yIo {
    fn write_str(&mut self, text: &str) -> io::Result<()> {
        use std::io::Write;
        let mut out = io::stdout().lock();
        out.write_all(text.as_bytes())?;
        out.flush()
    }

    fn ask(&mut self, prompt: &str, masked: bool) -> io::Result<Option<String>> {
        use std::io::BufRead;
        if masked && self.stdin_is_terminal {
            let answer = read_masked_line(|| self.write_str(prompt))?;
            self.write_str("\n")?;
            return Ok(answer);
        }
        self.write_str(prompt)?;
        let mut line = String::new();
        if io::stdin().lock().read_line(&mut line)? == 0 {
            return Ok(None);
        }
        while line.ends_with(['\n', '\r']) {
            line.pop();
        }
        Ok(Some(line))
    }
}

/// Read a line from the terminal with echo off. Raw mode only switches the
/// line discipline (no bytes are written), and is restored on every exit.
/// `prompt` runs once echo is off, so keys typed in answer to it are never
/// echoed.
#[cfg(feature = "crossterm")]
fn read_masked_line(prompt: impl FnOnce() -> io::Result<()>) -> io::Result<Option<String>> {
    use std::io::Read;

    struct RawGuard;
    impl Drop for RawGuard {
        fn drop(&mut self) {
            let _ = crossterm::terminal::disable_raw_mode();
        }
    }

    crossterm::terminal::enable_raw_mode()?;
    let _guard = RawGuard;
    prompt()?;
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut bytes: Vec<u8> = Vec::new();
    let mut byte = [0_u8; 1];
    loop {
        if input.read(&mut byte)? == 0 {
            if bytes.is_empty() {
                return Ok(None);
            }
            break;
        }
        match byte[0] {
            b'\r' | b'\n' => break,
            // Ctrl+C cancels the session; Ctrl+D on an empty line ends input.
            0x03 => return Ok(None),
            0x04 if bytes.is_empty() => return Ok(None),
            // Backspace / Delete remove one character.
            0x08 | 0x7f => {
                while let Some(last) = bytes.pop() {
                    if last & 0xC0 != 0x80 {
                        break;
                    }
                }
            }
            // Ctrl+U clears the line.
            0x15 => bytes.clear(),
            control if control < 0x20 => {}
            other => bytes.push(other),
        }
    }
    Ok(Some(String::from_utf8_lossy(&bytes).into_owned()))
}

/// Native entry: run `f` as an accessible session on stdin / stdout.
#[cfg(feature = "crossterm")]
pub(crate) fn run_accessible(
    config: &crate::RunConfig,
    f: impl FnMut(&mut Context),
    extra_lines: &mut dyn FnMut() -> Vec<String>,
) -> io::Result<()> {
    use std::io::IsTerminal;
    const FALLBACK_SIZE: (u32, u32) = (80, 24);
    let size = if io::stdout().is_terminal() {
        crossterm::terminal::size()
            .map(|(w, h)| (u32::from(w), u32::from(h)))
            .ok()
            .filter(|(w, h)| *w > 0 && *h > 0)
            .unwrap_or(FALLBACK_SIZE)
    } else {
        FALLBACK_SIZE
    };
    let mut io = StdA11yIo::new();
    run_accessible_session(&mut io, config, size, f, extra_lines)
}

#[cfg(test)]
mod tests;
