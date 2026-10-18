use super::*;
use crate::RichLogState;

impl Context {
    /// Render a scrollable rich log view with styled entries.
    pub fn rich_log(&mut self, state: &mut RichLogState) -> Response {
        let focused = self.register_focusable();
        let (interaction_id, mut response) = self.begin_widget_interaction(focused);

        let widget_height = self.available_content_size().1 as usize;
        let viewport_height = widget_height.saturating_sub(2);
        let effective_height = viewport_height;
        let show_indicator = state.len() > effective_height;
        let visible_rows = if show_indicator {
            effective_height.saturating_sub(1)
        } else {
            effective_height
        };
        let max_offset = state.len().saturating_sub(visible_rows);
        if state.auto_scroll && state.scroll_offset == usize::MAX {
            state.scroll_offset = max_offset;
        } else {
            state.scroll_offset = state.scroll_offset.min(max_offset);
        }
        let old_offset = state.scroll_offset;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, key) in self.available_key_presses() {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        state.scroll_offset = state.scroll_offset.saturating_sub(1);
                        consumed_indices.push(i);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        state.scroll_offset = (state.scroll_offset + 1).min(max_offset);
                        consumed_indices.push(i);
                    }
                    KeyCode::PageUp => {
                        state.scroll_offset =
                            state.scroll_offset.saturating_sub(visible_rows.max(1));
                        consumed_indices.push(i);
                    }
                    KeyCode::PageDown => {
                        state.scroll_offset = state
                            .scroll_offset
                            .saturating_add(visible_rows.max(1))
                            .min(max_offset);
                        consumed_indices.push(i);
                    }
                    KeyCode::Home => {
                        state.scroll_offset = 0;
                        consumed_indices.push(i);
                    }
                    KeyCode::End => {
                        state.scroll_offset = max_offset;
                        consumed_indices.push(i);
                    }
                    _ => {}
                }
            }
            self.consume_indices(consumed_indices);
        }

        if let Some(rect) = self.prev_hit_map.get(interaction_id).copied() {
            let mut consumed = Vec::new();
            for (i, mouse) in self.mouse_events_in_rect(rect) {
                let delta = self.scroll_lines_per_event as usize;
                match mouse.kind {
                    MouseKind::ScrollUp => {
                        state.scroll_offset = state.scroll_offset.saturating_sub(delta);
                        consumed.push(i);
                    }
                    MouseKind::ScrollDown => {
                        state.scroll_offset = (state.scroll_offset + delta).min(max_offset);
                        consumed.push(i);
                    }
                    _ => {}
                }
            }
            self.consume_indices(consumed);
        }

        state.scroll_offset = state.scroll_offset.min(max_offset);
        let start = state
            .scroll_offset
            .min(state.len().saturating_sub(visible_rows));
        let end = (start + visible_rows).min(state.len());

        self.commands
            .push(Command::BeginContainer(BeginContainerArgs {
                direction: Direction::Column,
                gap: 0,
                align: Align::Start,
                align_self: None,
                justify: Justify::Start,
                border: Some(Border::Single),
                border_sides: BorderSides::all(),
                border_style: Style::new().fg(self.theme.border),
                bg_color: None,
                padding: Padding::default(),
                margin: Margin::default(),
                constraints: Constraints::default(),
                title: None,
                grow: 0,
                group_name: None,
            }));

        for entry in state.entries().skip(start).take(end.saturating_sub(start)) {
            self.commands.push(Command::RichText {
                segments: entry.segments.clone(),
                wrap: false,
                align: Align::Start,
                margin: Margin::default(),
                constraints: Constraints::default(),
            });
        }

        if show_indicator {
            let end_pos = end.min(state.len());
            let line = format!(
                "{}-{} / {}",
                if start < end { start + 1 } else { 0 },
                end_pos,
                state.len()
            );
            self.styled(line, Style::new().dim().fg(self.theme.text_dim));
        }

        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;
        response.changed = state.scroll_offset != old_offset;
        response
    }
}
