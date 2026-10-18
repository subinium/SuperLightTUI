//! Text-input family — single-line text input, textarea, number input and
//! form helpers.
//!
//! State types ([`crate::widgets::TextInputState`],
//! [`crate::widgets::TextareaState`], [`crate::widgets::NumberInputState`],
//! [`crate::widgets::FormState`], ...) live in `src/widgets/text_input.rs`.

use super::*;
use crate::event::MouseEvent;
use crate::widgets::word_range_at;

mod number_input;
mod textarea;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod selection_tests;

/// Issue #465: map a cell column back to a grapheme index using the
/// previous frame's rendered layout (`TextInputEditing::visual`).
fn text_input_index_at(state: &TextInputState, x: u32, text_x: u32) -> usize {
    let len = grapheme_count(&state.value);
    let visual = &state.editing.visual;
    if x < text_x {
        return visual.first().map_or(0, |&(_, idx)| idx).min(len);
    }
    let cell = x - text_x;
    visual
        .iter()
        .rev()
        .find(|&&(start, _)| start <= cell)
        .map_or(len, |&(_, idx)| idx)
        .min(len)
}

/// Issue #465: press / drag / release handling for mouse selection in a
/// focused `text_input`. Returns `true` when the event belongs to the input.
fn text_input_pointer(
    state: &mut TextInputState,
    mouse: &MouseEvent,
    rect: Rect,
    text_x: u32,
    now: crate::clock::Instant,
) -> bool {
    match mouse.kind {
        MouseKind::Down(MouseButton::Left) => {
            if !rect.contains(mouse.x, mouse.y) {
                state.editing.pointer.dragging = false;
                return false;
            }
            let index = text_input_index_at(state, mouse.x, text_x);
            match state
                .editing
                .pointer
                .register_click(now, (mouse.x, mouse.y))
            {
                1 => {
                    if mouse.modifiers.contains(KeyModifiers::SHIFT) {
                        state.move_cursor(index, true);
                    } else {
                        state.set_selection(index, index);
                        state.editing.last_typing = false;
                    }
                    state.editing.pointer.dragging = true;
                }
                // Word boundaries would reveal the structure of a masked
                // value, so a double-click there selects everything.
                2 if !state.masked => {
                    let word = word_range_at(&state.value, index);
                    state.set_selection(word.start, word.end);
                    state.editing.pointer.dragging = false;
                }
                _ => {
                    state.select_all();
                    state.editing.pointer.dragging = false;
                }
            }
            true
        }
        MouseKind::Drag(MouseButton::Left) if state.editing.pointer.dragging => {
            let index = text_input_index_at(state, mouse.x, text_x);
            state.editing.anchor.get_or_insert(state.cursor);
            state.cursor = index;
            true
        }
        MouseKind::Up(MouseButton::Left) if state.editing.pointer.dragging => {
            state.editing.pointer.dragging = false;
            true
        }
        _ => false,
    }
}

impl Context {
    /// Issue #465: render text runs (`(text, selected)`) as one line. A
    /// single run renders exactly like `styled_with_cursor_privacy`; several
    /// runs render in a gap-less row so selected runs can carry the
    /// selection style. `cursor` is `(run, grapheme offset)` of the caret.
    pub(super) fn text_runs_with_cursor(
        &mut self,
        runs: &[(String, bool)],
        style: Style,
        selected_style: Style,
        cursor: Option<(usize, usize)>,
        cursor_masked: bool,
    ) {
        let style_for = |selected: bool| if selected { selected_style } else { style };
        if let [(text, selected)] = runs {
            self.styled_with_cursor_privacy(
                text.clone(),
                style_for(*selected),
                cursor.map(|(_, offset)| offset),
                cursor_masked,
            );
            return;
        }
        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                direction: Direction::Row,
                gap: 0,
                align: Align::Start,
                align_self: None,
                justify: Justify::Start,
                border: None,
                border_sides: BorderSides::all(),
                border_style: Style::new(),
                bg_color: None,
                padding: Padding::default(),
                margin: Margin::default(),
                constraints: Constraints::default(),
                title: None,
                grow: 0,
                group_name: None,
            })));
        for (index, (text, selected)) in runs.iter().enumerate() {
            let offset = cursor
                .filter(|&(run, _)| run == index)
                .map(|(_, offset)| offset);
            self.styled_with_cursor_privacy(
                text.clone(),
                style_for(*selected),
                offset,
                cursor_masked,
            );
        }
        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;
    }
}

impl Context {
    /// Render a single-line text input. Auto-handles cursor, typing, and backspace.
    ///
    /// The widget claims focus via [`Context::register_focusable`]. When focused,
    /// it consumes character, backspace, arrow, Home, and End key events.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::widgets::TextInputState;
    ///
    /// // Create state once, outside the frame closure, so typed text persists.
    /// let mut input = TextInputState::with_placeholder("Search...");
    /// slt::run(|ui: &mut slt::Context| {
    ///     ui.text_input(&mut input);
    ///     // input.value holds the current text
    /// })?;
    /// # Ok::<_, std::io::Error>(())
    /// ```
    pub fn text_input(&mut self, state: &mut TextInputState) -> Response {
        let colors = self.widget_theme.text_input;
        self.text_input_colored(state, &colors)
    }

    /// Render a text input with custom widget colors.
    pub fn text_input_colored(
        &mut self,
        state: &mut TextInputState,
        colors: &WidgetColors,
    ) -> Response {
        slt_assert(
            !state.value.contains('\n'),
            "text_input got a newline — use textarea instead",
        );
        let focused = self.register_text_entry_focusable(
            crate::runtime::TextEntryKind::SingleLine,
            state.focus_key(),
        );
        let focus_slot = self.rollback.last_focusable_id;
        // Issue #469: an accessible-mode line answer replaces the value.
        let mut a11y = self.a11y_begin(focused);
        // v0.21.1: capture the focus-edge flags immediately — this consumes the
        // `register_focusable` marker, so the result is correct regardless of
        // the child containers rendered below. Issue #208 left text_input never
        // populating gained_focus/lost_focus because it assembles its Response
        // by hand instead of via `begin_widget_interaction`.
        let (gained_focus, lost_focus) = self.focus_transitions(focused);
        let mut submitted = false;
        let old_value = state.value.clone();
        if let Some(text) = crate::runtime::A11yBegin::take_text(a11y.as_mut()) {
            if let Some(text) = text {
                state.set_value(text);
            }
            submitted = true;
        }
        state.cursor = state.cursor.min(grapheme_count(&state.value));
        let input_padx = self.theme.spacing.xs();
        // Issue #465: the bordered box below takes the next interaction slot;
        // its previous-frame rect locates the text for mouse selection.
        let box_rect = self
            .prev_hit_map
            .get(self.rollback.interaction_count)
            .copied()
            .filter(|rect| !rect.is_empty());

        if focused {
            let mut consumed_indices = Vec::new();
            let mut clipboard: Option<String> = None;
            let mut pointer_captured = false;
            // Hoist matched_suggestions out of the loop and recompute only
            // after a mutation key (Char/Backspace/Delete) sets the dirty flag.
            // A 10-key burst with one mutation: 10 calls -> 2 calls.
            let compute_matched = |state: &TextInputState| -> Vec<String> {
                if state.show_suggestions {
                    state
                        .matched_suggestions()
                        .into_iter()
                        .map(str::to_string)
                        .collect()
                } else {
                    Vec::new()
                }
            };
            let reopen_suggestions = |state: &mut TextInputState| {
                if !state.suggestions.is_empty() {
                    state.show_suggestions = true;
                    state.suggestion_index = 0;
                }
            };
            let mut matched_suggestions = compute_matched(state);
            let mut suggestions_dirty = false;
            for (i, event) in self
                .events
                .iter()
                .enumerate()
                .filter(|(i, _)| !self.consumed[*i])
            {
                if let Event::Paste(text) = event {
                    let inserted: String = text
                        .graphemes(true)
                        .filter(|cluster| {
                            cluster
                                .chars()
                                .all(|ch| (ch as u32) >= 0x20 && ch != '\u{7f}')
                        })
                        .collect();
                    if !inserted.is_empty() && state.insert_text(&inserted) {
                        reopen_suggestions(state);
                        suggestions_dirty = true;
                    }
                    consumed_indices.push(i);
                    continue;
                }
                if let Event::Mouse(mouse) = event {
                    if let Some(rect) = box_rect
                        && text_input_pointer(
                            state,
                            mouse,
                            rect,
                            rect.x.saturating_add(1).saturating_add(input_padx),
                            self.frame_instant,
                        )
                    {
                        consumed_indices.push(i);
                        pointer_captured = true;
                    }
                    continue;
                }
                let Event::Key(key) = event else {
                    continue;
                };
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if suggestions_dirty {
                    matched_suggestions = compute_matched(state);
                    suggestions_dirty = false;
                }
                let suggestions_visible = !matched_suggestions.is_empty();
                if suggestions_visible {
                    state.suggestion_index = state
                        .suggestion_index
                        .min(matched_suggestions.len().saturating_sub(1));
                }
                let shift = key.modifiers.contains(KeyModifiers::SHIFT);
                let command = key.modifiers.contains(KeyModifiers::CONTROL)
                    || key.modifiers.contains(KeyModifiers::SUPER);
                let word = key.modifiers.contains(KeyModifiers::CONTROL)
                    || key.modifiers.contains(KeyModifiers::ALT);
                match key.code {
                    KeyCode::Up if suggestions_visible => {
                        state.suggestion_index = state.suggestion_index.saturating_sub(1);
                        consumed_indices.push(i);
                    }
                    KeyCode::Down if suggestions_visible => {
                        state.suggestion_index = (state.suggestion_index + 1)
                            .min(matched_suggestions.len().saturating_sub(1));
                        consumed_indices.push(i);
                    }
                    KeyCode::Esc if state.show_suggestions => {
                        state.show_suggestions = false;
                        state.suggestion_index = 0;
                        suggestions_dirty = true;
                        consumed_indices.push(i);
                    }
                    KeyCode::Tab
                        if suggestions_visible && !key.modifiers.contains(KeyModifiers::SHIFT) =>
                    {
                        if let Some(selected) = matched_suggestions
                            .get(state.suggestion_index)
                            .or_else(|| matched_suggestions.first())
                        {
                            state.push_undo();
                            state.value = selected.clone();
                            state.cursor = grapheme_count(&state.value);
                            state.clear_selection();
                            state.show_suggestions = false;
                            state.suggestion_index = 0;
                            suggestions_dirty = true;
                        }
                        consumed_indices.push(i);
                    }
                    // Issue #465: selection / clipboard / history chords.
                    KeyCode::Char('a' | 'A') if command && state.ctrl_a_selects_all() => {
                        state.select_all();
                        consumed_indices.push(i);
                    }
                    KeyCode::Char('c' | 'C') if command && !shift => {
                        // Without a selection Ctrl+C stays an app shortcut.
                        if let Some(text) = state.selected_text() {
                            if !state.masked {
                                clipboard = Some(text.to_string());
                            }
                            consumed_indices.push(i);
                        }
                    }
                    KeyCode::Char('x' | 'X') if command && !shift => {
                        if let Some(text) = state.selected_text() {
                            if !state.masked {
                                clipboard = Some(text.to_string());
                            }
                            state.delete_selection();
                            reopen_suggestions(state);
                            suggestions_dirty = true;
                            consumed_indices.push(i);
                        }
                    }
                    KeyCode::Char('z' | 'Z') if command && !shift => {
                        if state.undo() {
                            suggestions_dirty = true;
                        }
                        consumed_indices.push(i);
                    }
                    KeyCode::Char('y' | 'Y') if command => {
                        if state.redo() {
                            suggestions_dirty = true;
                        }
                        consumed_indices.push(i);
                    }
                    KeyCode::Char('z' | 'Z') if command && shift => {
                        if state.redo() {
                            suggestions_dirty = true;
                        }
                        consumed_indices.push(i);
                    }
                    KeyCode::Char(ch) if !has_global_shortcut_modifier(key.modifiers) => {
                        let mut encoded = [0; 4];
                        if state.type_text(ch.encode_utf8(&mut encoded)) {
                            reopen_suggestions(state);
                            suggestions_dirty = true;
                        }
                        consumed_indices.push(i);
                    }
                    KeyCode::Backspace => {
                        state.delete_backward();
                        reopen_suggestions(state);
                        suggestions_dirty = true;
                        consumed_indices.push(i);
                    }
                    KeyCode::Delete => {
                        state.delete_forward();
                        reopen_suggestions(state);
                        suggestions_dirty = true;
                        consumed_indices.push(i);
                    }
                    KeyCode::Left => {
                        let target = match state.selection() {
                            Some(range) if !shift && !word => range.start,
                            _ if word => textarea::prev_word_col(&state.value, state.cursor),
                            _ => state.cursor.saturating_sub(1),
                        };
                        state.move_cursor(target, shift);
                        consumed_indices.push(i);
                    }
                    KeyCode::Right => {
                        let len = grapheme_count(&state.value);
                        let target = match state.selection() {
                            Some(range) if !shift && !word => range.end,
                            _ if word => textarea::next_word_col(&state.value, state.cursor),
                            _ => (state.cursor + 1).min(len),
                        };
                        state.move_cursor(target, shift);
                        consumed_indices.push(i);
                    }
                    KeyCode::Home => {
                        state.move_cursor(0, shift);
                        consumed_indices.push(i);
                    }
                    KeyCode::End => {
                        state.move_cursor(grapheme_count(&state.value), shift);
                        consumed_indices.push(i);
                    }
                    KeyCode::Enter => {
                        // v0.21.1: Enter submits the input. If the suggestion
                        // dropdown is open, accept the highlighted suggestion
                        // instead (Tab also accepts) — only a bare Enter with
                        // no open suggestions reports `submitted`.
                        if suggestions_visible {
                            if let Some(selected) = matched_suggestions
                                .get(state.suggestion_index)
                                .or_else(|| matched_suggestions.first())
                            {
                                state.push_undo();
                                state.value = selected.clone();
                                state.cursor = grapheme_count(&state.value);
                                state.clear_selection();
                                state.show_suggestions = false;
                                state.suggestion_index = 0;
                                suggestions_dirty = true;
                            }
                        } else {
                            submitted = true;
                        }
                        consumed_indices.push(i);
                    }
                    _ => {}
                }
            }
            self.consume_indices(consumed_indices);
            if let Some(text) = clipboard {
                self.copy_to_clipboard(text);
            }
            if pointer_captured {
                self.text_pointer_capture = true;
            }
        } else {
            state.editing.pointer.dragging = false;
        }
        let mut chords = crate::runtime::CHORD_HISTORY;
        if state.ctrl_a_selects_all() {
            chords |= crate::runtime::CHORD_SELECT_ALL;
        }
        if state.selection().is_some() {
            chords |= crate::runtime::CHORD_COPY_CUT;
        }
        self.set_focus_slot_chords(focus_slot, chords);

        if state.value.is_empty() {
            state.show_suggestions = false;
            state.suggestion_index = 0;
        }

        let matched_suggestions = if state.show_suggestions {
            state
                .matched_suggestions()
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<String>>()
        } else {
            Vec::new()
        };
        if !matched_suggestions.is_empty() {
            state.suggestion_index = state
                .suggestion_index
                .min(matched_suggestions.len().saturating_sub(1));
        }

        let visible_width = self.area_width.saturating_sub(4) as usize;
        let selection = state.selection();
        let mut visual = std::mem::take(&mut state.editing.visual);
        visual.clear();
        // Runs of `(text, selected)`; one unselected run unless a selection
        // is visible. `cursor` is `(run, grapheme offset)` of the caret glyph.
        let mut runs: Vec<(String, bool)> = Vec::with_capacity(3);
        let mut cursor: Option<(usize, usize)> = None;
        if state.value.is_empty() {
            if state.placeholder.len() > 100 {
                slt_warn(
                    "text_input placeholder is very long (>100 chars) — consider shortening it",
                );
            }
            let mut ph = state.placeholder.clone();
            if focused {
                ph.insert(0, '▎');
                cursor = Some((0, 0));
            }
            visual.push((0, 0));
            runs.push((ph, false));
        } else {
            // Display units are grapheme clusters: `state.cursor` is a cluster
            // index, so each rendered unit (one source cluster, or one mask
            // glyph standing in for it) advances the cursor index by one.
            let clusters: Vec<&str> = state.value.graphemes(true).collect();
            let display_units: Vec<&str> = if state.masked {
                vec!["•"; clusters.len()]
            } else {
                clusters.clone()
            };

            let cursor_display_pos: usize = display_units[..state.cursor.min(display_units.len())]
                .iter()
                .map(|g| cluster_width(g).max(1) as usize)
                .sum();

            let scroll_offset = if cursor_display_pos >= visible_width {
                cursor_display_pos - visible_width + 1
            } else {
                0
            };

            let mut out_cells: u32 = 0;
            let push = |runs: &mut Vec<(String, bool)>, text: &str, selected: bool| match runs
                .last_mut()
            {
                Some((run, sel)) if *sel == selected => run.push_str(text),
                _ => runs.push((text.to_string(), selected)),
            };
            let caret = |runs: &[(String, bool)]| {
                let run = runs.len().saturating_sub(1);
                let offset = runs.last().map_or(0, |(text, _)| grapheme_count(text));
                (run, offset)
            };
            let mut current_width: usize = 0;
            for (idx, g) in display_units.iter().enumerate() {
                let cw = cluster_width(g).max(1) as usize;
                if current_width + cw <= scroll_offset {
                    current_width += cw;
                    continue;
                }
                if current_width.saturating_sub(scroll_offset) >= visible_width {
                    break;
                }
                // Preserve the cells occupied by a cluster crossing the left
                // edge, without emitting a partial wide grapheme.
                if current_width < scroll_offset {
                    let pad = (current_width + cw - scroll_offset).min(visible_width);
                    push(&mut runs, &" ".repeat(pad), false);
                    out_cells += pad as u32;
                    current_width += cw;
                    continue;
                }
                if focused && idx == state.cursor {
                    push(&mut runs, "", false);
                    cursor = Some(caret(&runs));
                    push(&mut runs, "▎", false);
                    // A click on the caret glyph keeps the cursor there.
                    visual.push((out_cells, idx));
                    out_cells += 1;
                }
                let cursor_cells = usize::from(cursor.is_some());
                if current_width - scroll_offset + cw + cursor_cells > visible_width {
                    break;
                }
                let selected = selection.as_ref().is_some_and(|range| range.contains(&idx));
                visual.push((out_cells, idx));
                push(&mut runs, g, selected);
                out_cells += cw as u32;
                current_width += cw;
            }
            let end = visual.last().map_or(0, |&(_, idx)| idx + 1);
            if focused && visible_width > 0 && state.cursor >= display_units.len() {
                push(&mut runs, "", false);
                cursor = Some(caret(&runs));
                push(&mut runs, "▎", false);
            }
            visual.push((out_cells, end));
        }
        state.editing.visual = visual;
        if runs.is_empty() {
            runs.push((String::new(), false));
        }
        let input_style = if state.value.is_empty() && !focused {
            Style::new()
                .dim()
                .fg(colors.fg.unwrap_or(self.theme.text_dim))
        } else {
            Style::new().fg(colors.fg.unwrap_or(self.theme.text))
        };
        let selected_style = self.theme.text_selection_style();

        let border_color = if focused {
            colors.accent.unwrap_or(self.theme.primary)
        } else if state.validation_error.is_some() {
            colors.accent.unwrap_or(self.theme.error)
        } else {
            colors.border.unwrap_or(self.theme.border)
        };

        let masked = state.masked;
        let mut response = self
            .bordered(Border::Rounded)
            .border_style(Style::new().fg(border_color))
            .px(input_padx)
            .col(|ui| {
                ui.text_runs_with_cursor(&runs, input_style, selected_style, cursor, masked);
            });
        response.focused = focused;
        response.changed = state.value != old_value;
        response.gained_focus = gained_focus;
        response.lost_focus = lost_focus;
        response.submitted = submitted;

        let errors = state.errors();
        if !errors.is_empty() {
            for error in errors {
                let mut warning = String::with_capacity(2 + error.len());
                warning.push_str("⚠ ");
                warning.push_str(error);
                self.styled(
                    warning,
                    Style::new()
                        .dim()
                        .fg(colors.accent.unwrap_or(self.theme.error)),
                );
            }
        } else if let Some(error) = state.validation_error.clone() {
            let mut warning = String::with_capacity(2 + error.len());
            warning.push_str("⚠ ");
            warning.push_str(&error);
            self.styled(
                warning,
                Style::new()
                    .dim()
                    .fg(colors.accent.unwrap_or(self.theme.error)),
            );
        }

        if state.show_suggestions && !matched_suggestions.is_empty() {
            let start = state.suggestion_index.saturating_sub(4);
            let end = (start + 5).min(matched_suggestions.len());
            let suggestion_border = colors.border.unwrap_or(self.theme.border);
            let suggestion_padx = self.theme.spacing.xs();
            let _ = self
                .bordered(Border::Rounded)
                .border_style(Style::new().fg(suggestion_border))
                .px(suggestion_padx)
                .col(|ui| {
                    for (idx, suggestion) in matched_suggestions[start..end].iter().enumerate() {
                        let actual_idx = start + idx;
                        if actual_idx == state.suggestion_index {
                            ui.styled(
                                suggestion.clone(),
                                Style::new()
                                    .bg(colors.accent.unwrap_or(ui.theme().selected_bg))
                                    .fg(colors.fg.unwrap_or(ui.theme().selected_fg)),
                            );
                        } else {
                            ui.styled(
                                suggestion.clone(),
                                Style::new().fg(colors.fg.unwrap_or(ui.theme().text)),
                            );
                        }
                    }
                });
        }
        if let Some(begin) = a11y {
            self.a11y_end_text_input(begin, state);
        }
        response
    }

    /// Render a form that groups input fields vertically.
    ///
    /// Wraps the fields in a column container and forwards the form state
    /// to the closure. Use [`Context::form_field`] inside the closure to
    /// render each field with label + input + error display.
    ///
    /// Submission is driven by [`Context::form_submit`]. Per-field validators
    /// attached via [`FormField::validate`](crate::widgets::FormField::validate)
    /// run automatically inside [`Context::form_field`]; aggregate validity is
    /// read via [`FormState::is_valid`](crate::widgets::FormState::is_valid).
    pub fn form(
        &mut self,
        state: &mut FormState,
        f: impl FnOnce(&mut Context, &mut FormState),
    ) -> &mut Self {
        let _ = self.col(|ui| {
            f(ui, state);
        });
        self
    }

    /// Render a single form field with label and input, running its validators.
    ///
    /// The field's own validators (attached via
    /// [`FormField::validate`](crate::widgets::FormField::validate)) run
    /// automatically according to its
    /// [`trigger`](crate::widgets::FormField::trigger):
    /// [`OnChange`](crate::widgets::ValidateTrigger::OnChange) re-validates on
    /// each keystroke, [`OnBlur`](crate::widgets::ValidateTrigger::OnBlur)
    /// (the default) re-validates when focus leaves the field, and
    /// [`Manual`](crate::widgets::ValidateTrigger::Manual) never auto-validates.
    /// The resulting [`error`](crate::widgets::FormField::error) is shown below
    /// the input.
    ///
    /// With the `async` feature, any in-flight
    /// [`validate_async`](crate::widgets::FormField::validate_async) check is
    /// polled each frame and its result surfaced as the field error.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::widgets::{FormField, validators};
    /// # slt::run(|ui: &mut slt::Context| {
    /// let mut field = FormField::new("Email")
    ///     .validate(validators::email()); // OnBlur by default
    /// ui.form_field(&mut field);
    /// # });
    /// ```
    pub fn form_field(&mut self, field: &mut FormField) -> &mut Self {
        let _ = self.form_field_response(field);
        self
    }

    /// Render a form field and return its focus, input and full layout feedback.
    ///
    /// Runs the same validators as [`form_field`](Self::form_field), but returns
    /// a [`FormFieldResponse`](crate::FormFieldResponse) instead of a chainable
    /// context. Geometry is from the previous completed frame; see
    /// [`focused_layout_rect`](Self::focused_layout_rect) for coordinate rules.
    pub fn form_field_response(&mut self, field: &mut FormField) -> crate::FormFieldResponse {
        let id = self.rollback.interaction_count;
        let layout_rect = self
            .prev_allocated_areas
            .get(id)
            .copied()
            .filter(|rect| !rect.is_empty());
        #[cfg(feature = "async")]
        let async_resolved = field.poll_async();
        // Bind the input's single focus slot to the entire field so keyboard
        // reveal includes its label/error, while a tiny viewport prefers the
        // actual input caret. Named reservations remain attached to this slot.
        self.register_text_entry_focusable(
            crate::runtime::TextEntryKind::SingleLine,
            field.input.focus_key(),
        );
        self.rollback.pending_focusable_id = self.rollback.last_focusable_id;
        let layout_rect = if self.rollback.new_focus_slot {
            None
        } else {
            layout_rect
        };
        let mut resp = Response::none();
        // Issue #469: the field label names the input in accessible mode.
        let a11y_start = self.commands.len();
        if self.accessible {
            self.a11y_default_name(&field.label);
        }
        let mut response = self.col(|ui| {
            ui.styled(field.label.as_str(), Style::new().bold().fg(ui.theme.text));
            resp = ui.text_input(&mut field.input);
            if let Some(error) = field.error.as_deref() {
                ui.styled(error, Style::new().dim().fg(ui.theme.error));
            }
        });
        #[cfg(feature = "async")]
        let _ = async_resolved;
        // Validation retains a field-local edge even if positional focus slots
        // change as other fields are inserted or removed.
        let lost_focus = field.observe_focus(resp.focused);
        match field.trigger {
            ValidateTrigger::OnChange if resp.changed => {
                field.run_validators();
            }
            // An accessible-mode answer commits the field like a blur, so
            // its validation message is read back before moving on.
            ValidateTrigger::OnBlur if lost_focus || (self.accessible && resp.submitted) => {
                field.run_validators();
            }
            _ => {}
        }
        if self.accessible {
            self.a11y_extend_last(a11y_start, field.error.as_deref());
        }
        response.focused = resp.focused;
        response.gained_focus = resp.gained_focus;
        response.lost_focus = resp.lost_focus;
        response.changed = resp.changed;
        response.submitted = resp.submitted;
        crate::FormFieldResponse {
            response,
            input: resp,
            layout_rect,
        }
    }

    /// Render a primary-styled submit button.
    ///
    /// Distinguishes the submit affordance from incidental buttons in the
    /// same form by rendering in the theme's primary color (via
    /// [`ButtonVariant::Primary`]). Returns `true` in `.clicked` when the
    /// user clicks it, presses Enter while focused, or activates it with
    /// Space. Pair with
    /// [`FormState::validate_all`](crate::widgets::FormState::validate_all) /
    /// [`FormState::is_valid`](crate::widgets::FormState::is_valid) to gate
    /// submission on all fields being valid.
    pub fn form_submit(&mut self, label: impl Into<String>) -> Response {
        self.button_with(label, ButtonVariant::Primary)
    }
}
