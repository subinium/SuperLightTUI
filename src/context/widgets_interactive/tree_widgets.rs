use super::*;
use crate::{DirectoryTreeState, TreeNode};

impl Context {
    /// Render a tree view. Left/Right to collapse/expand, Up/Down to navigate.
    pub fn tree(&mut self, state: &mut TreeState) -> Response {
        let entries = state.flatten();
        if entries.is_empty() {
            return Response::none();
        }
        state.selected = state.selected.min(entries.len().saturating_sub(1));
        let old_selected = state.selected;
        let focused = self.register_focusable();
        let interaction_id = self.next_interaction_id();
        let mut response = self.response_for(interaction_id);
        response.focused = focused;
        let mut changed = false;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if self.consumed[i] {
                    continue;
                }
                if let Event::Key(key) = event {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                            let max_index = state.flatten().len().saturating_sub(1);
                            let _ = handle_vertical_nav(
                                &mut state.selected,
                                max_index,
                                key.code.clone(),
                            );
                            changed = changed || state.selected != old_selected;
                            consumed_indices.push(i);
                        }
                        KeyCode::Right | KeyCode::Enter | KeyCode::Char(' ') => {
                            state.toggle_at(state.selected);
                            changed = true;
                            consumed_indices.push(i);
                        }
                        KeyCode::Left => {
                            let entry = &entries[state.selected.min(entries.len() - 1)];
                            if entry.expanded {
                                state.toggle_at(state.selected);
                                changed = true;
                            }
                            consumed_indices.push(i);
                        }
                        _ => {}
                    }
                }
            }
            for idx in consumed_indices {
                self.consumed[idx] = true;
            }
        }

        self.commands.push(Command::BeginContainer {
            direction: Direction::Column,
            gap: 0,
            align: Align::Start,
            align_self: None,
            justify: Justify::Start,
            border: None,
            border_sides: BorderSides::all(),
            border_style: Style::new().fg(self.theme.border),
            bg_color: None,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
            group_name: None,
        });

        let entries = state.flatten();
        for (idx, entry) in entries.iter().enumerate() {
            let indent = "  ".repeat(entry.depth);
            let icon = if entry.is_leaf {
                "  "
            } else if entry.expanded {
                "▾ "
            } else {
                "▸ "
            };
            let is_selected = idx == state.selected;
            let style = if is_selected && focused {
                Style::new().bold().fg(self.theme.primary)
            } else if is_selected {
                Style::new().fg(self.theme.primary)
            } else {
                Style::new().fg(self.theme.text)
            };
            let cursor = if is_selected && focused { "▸" } else { " " };
            let mut row =
                String::with_capacity(cursor.len() + indent.len() + icon.len() + entry.label.len());
            row.push_str(cursor);
            row.push_str(&indent);
            row.push_str(icon);
            row.push_str(&entry.label);
            self.styled(row, style);
        }

        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;
        response.changed = changed || state.selected != old_selected;
        response
    }

    /// Render a directory tree with guide lines and tree connectors.
    pub fn directory_tree(&mut self, state: &mut DirectoryTreeState) -> Response {
        let entries = state.tree.flatten();
        if entries.is_empty() {
            return Response::none();
        }
        state.tree.selected = state.tree.selected.min(entries.len().saturating_sub(1));
        let old_selected = state.tree.selected;
        let focused = self.register_focusable();
        let interaction_id = self.next_interaction_id();
        let mut response = self.response_for(interaction_id);
        response.focused = focused;
        let mut changed = false;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if self.consumed[i] {
                    continue;
                }
                if let Event::Key(key) = event {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                            let max_index = state.tree.flatten().len().saturating_sub(1);
                            let _ = handle_vertical_nav(
                                &mut state.tree.selected,
                                max_index,
                                key.code.clone(),
                            );
                            changed = changed || state.tree.selected != old_selected;
                            consumed_indices.push(i);
                        }
                        KeyCode::Right => {
                            let current_entries = state.tree.flatten();
                            let entry = &current_entries
                                [state.tree.selected.min(current_entries.len() - 1)];
                            if !entry.is_leaf && !entry.expanded {
                                state.tree.toggle_at(state.tree.selected);
                                changed = true;
                            }
                            consumed_indices.push(i);
                        }
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            state.tree.toggle_at(state.tree.selected);
                            changed = true;
                            consumed_indices.push(i);
                        }
                        KeyCode::Left => {
                            let current_entries = state.tree.flatten();
                            let entry = &current_entries
                                [state.tree.selected.min(current_entries.len() - 1)];
                            if entry.expanded {
                                state.tree.toggle_at(state.tree.selected);
                                changed = true;
                            }
                            consumed_indices.push(i);
                        }
                        _ => {}
                    }
                }
            }
            for idx in consumed_indices {
                self.consumed[idx] = true;
            }
        }

        self.commands.push(Command::BeginContainer {
            direction: Direction::Column,
            gap: 0,
            align: Align::Start,
            align_self: None,
            justify: Justify::Start,
            border: None,
            border_sides: BorderSides::all(),
            border_style: Style::new().fg(self.theme.border),
            bg_color: None,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
            group_name: None,
        });

        let mut rows = Vec::new();
        flatten_directory_rows(&state.tree.nodes, Vec::new(), &mut rows);
        for (idx, row_entry) in rows.iter().enumerate() {
            let mut row = String::new();
            let cursor = if idx == state.tree.selected && focused {
                "▸"
            } else {
                " "
            };
            row.push_str(cursor);
            row.push(' ');

            if row_entry.depth > 0 {
                for has_more in &row_entry.branch_mask {
                    if *has_more {
                        row.push_str("│   ");
                    } else {
                        row.push_str("    ");
                    }
                }
                if row_entry.is_last {
                    row.push_str("└── ");
                } else {
                    row.push_str("├── ");
                }
            }

            let icon = if row_entry.is_leaf {
                "  "
            } else if row_entry.expanded {
                "▾ "
            } else {
                "▸ "
            };
            if state.show_icons {
                row.push_str(icon);
            }
            row.push_str(&row_entry.label);

            let style = if idx == state.tree.selected && focused {
                Style::new().bold().fg(self.theme.primary)
            } else if idx == state.tree.selected {
                Style::new().fg(self.theme.primary)
            } else {
                Style::new().fg(self.theme.text)
            };
            self.styled(row, style);
        }

        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;
        response.changed = changed || state.tree.selected != old_selected;
        response
    }
}

struct DirectoryRenderRow {
    depth: usize,
    label: String,
    is_leaf: bool,
    expanded: bool,
    is_last: bool,
    branch_mask: Vec<bool>,
}

fn flatten_directory_rows(
    nodes: &[TreeNode],
    branch_mask: Vec<bool>,
    out: &mut Vec<DirectoryRenderRow>,
) {
    for (idx, node) in nodes.iter().enumerate() {
        let is_last = idx + 1 == nodes.len();
        out.push(DirectoryRenderRow {
            depth: branch_mask.len(),
            label: node.label.clone(),
            is_leaf: node.children.is_empty(),
            expanded: node.expanded,
            is_last,
            branch_mask: branch_mask.clone(),
        });

        if node.expanded && !node.children.is_empty() {
            let mut next_mask = branch_mask.clone();
            next_mask.push(!is_last);
            flatten_directory_rows(&node.children, next_mask, out);
        }
    }
}
