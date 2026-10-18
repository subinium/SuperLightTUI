//! Context menus and the menu bar (#459).
//!
//! An open menu renders through a non-dimming modal layer: widgets behind it
//! ignore the mouse and keyboard, a click outside closes the menu without
//! reaching the widget under the pointer, and when the menu closes focus
//! returns to the widget that held it before the menu opened. Geometry is
//! computed from the menu entries alone, so input is hit-tested against the
//! same rects the menu was drawn at.

use super::popover::place_popover;
use super::*;
use crate::widgets::{MenuBarState, MenuCursor, MenuItem, MenuState, level_items, step_selectable};
use smallvec::SmallVec;
use unicode_width::UnicodeWidthStr;

/// Glyph marking an entry that opens a submenu.
const SUBMENU_MARK: &str = "▸";

/// Rects of the open menu levels, outermost first.
type LevelRects = SmallVec<[Rect; 4]>;

/// An unconsumed mouse press: `(event index, cell, button)`.
pub(crate) type OpenPress = (usize, (u32, u32), MouseButton);

/// Column widths of one menu level.
struct MenuColumns {
    label: u32,
    shortcut: u32,
    submenu: bool,
}

impl MenuColumns {
    fn of(items: &[MenuItem]) -> Self {
        let mut columns = Self {
            label: 0,
            shortcut: 0,
            submenu: false,
        };
        for item in items.iter().filter(|item| !item.is_separator()) {
            columns.label = columns
                .label
                .max(UnicodeWidthStr::width(item.label.as_str()) as u32);
            if let Some(hint) = &item.shortcut {
                columns.shortcut = columns
                    .shortcut
                    .max(UnicodeWidthStr::width(hint.as_str()) as u32);
            }
            columns.submenu |= item.has_submenu();
        }
        columns
    }

    /// Width inside the border: ` label  shortcut ▸ `.
    fn inner_width(&self) -> u32 {
        let mut width = 1 + self.label + 1;
        if self.shortcut > 0 {
            width += 2 + self.shortcut;
        }
        if self.submenu {
            width += 2;
        }
        width
    }

    fn row_text(&self, item: &MenuItem) -> String {
        let inner = self.inner_width() as usize;
        if item.is_separator() {
            return "─".repeat(inner);
        }
        let mut row = String::with_capacity(inner + 8);
        row.push(' ');
        row.push_str(&item.label);
        let label_width = UnicodeWidthStr::width(item.label.as_str());
        row.extend(std::iter::repeat_n(' ', self.label as usize - label_width));
        if self.shortcut > 0 {
            let hint = item.shortcut.as_deref().unwrap_or("");
            let hint_width = UnicodeWidthStr::width(hint);
            row.extend(std::iter::repeat_n(
                ' ',
                2 + self.shortcut as usize - hint_width,
            ));
            row.push_str(hint);
        }
        if self.submenu {
            row.push(' ');
            row.push_str(if item.has_submenu() {
                SUBMENU_MARK
            } else {
                " "
            });
        }
        row.push(' ');
        row
    }
}

/// Panel size (border included) for a menu level.
fn menu_size(items: &[MenuItem]) -> (u32, u32) {
    let inner = MenuColumns::of(items).inner_width();
    (inner + 2, items.len() as u32 + 2)
}

/// Rects of every open level of `path`, placed from `anchor` / `placement`
/// for the root and to the right of the parent row (flipping left) for
/// submenus.
fn menu_level_rects(
    items: &[MenuItem],
    path: &[usize],
    anchor: Rect,
    placement: Placement,
    area: Rect,
) -> LevelRects {
    let mut rects = LevelRects::new();
    for level in 0..path.len() {
        let entries = level_items(items, path, level);
        if entries.is_empty() {
            break;
        }
        let rect = match rects.last() {
            None => place_popover(anchor, menu_size(entries), placement, area).0,
            Some(parent) => {
                let row = parent.y + 1 + path[level - 1] as u32;
                let parent_row = Rect::new(parent.x, row.saturating_sub(1), parent.width, 1);
                place_popover(parent_row, menu_size(entries), Placement::Right, area).0
            }
        };
        rects.push(rect);
    }
    rects
}

/// Which level / row of the open menu is under `pos`. Deeper levels paint on
/// top, so they are tested first. `Some((level, None))` is the panel border.
fn menu_hit(
    rects: &[Rect],
    items: &[MenuItem],
    path: &[usize],
    pos: (u32, u32),
) -> Option<(usize, Option<usize>)> {
    for (level, rect) in rects.iter().enumerate().rev() {
        if !rect.contains(pos.0, pos.1) {
            continue;
        }
        let entries = level_items(items, path, level);
        let row = pos
            .1
            .checked_sub(rect.y + 1)
            .map(|row| row as usize)
            .filter(|&row| row < entries.len() && pos.0 > rect.x && pos.0 + 1 < rect.right());
        return Some((level, row));
    }
    None
}

/// Result of one key press on an open menu.
enum MenuKey {
    /// Moved the cursor or opened / closed a submenu.
    Handled,
    /// An action entry was activated.
    Activated(String),
    /// `Esc` at the root level.
    Close,
    /// `Left` at the root level or `Right` on an action (`true` = forward).
    Sideways(bool),
    /// Not a menu key.
    Ignored,
}

fn menu_key(items: &[MenuItem], cursor: &mut MenuCursor, code: KeyCode) -> MenuKey {
    let Some(&current) = cursor.path.last() else {
        return MenuKey::Ignored;
    };
    let level = cursor.path.len() - 1;
    let entries = level_items(items, &cursor.path, level);
    let set = |cursor: &mut MenuCursor, index: Option<usize>| {
        if let (Some(index), Some(slot)) = (index, cursor.path.last_mut()) {
            *slot = index;
        }
    };
    match code {
        KeyCode::Up => {
            set(cursor, step_selectable(entries, current, false, false));
            MenuKey::Handled
        }
        KeyCode::Down => {
            set(cursor, step_selectable(entries, current, true, false));
            MenuKey::Handled
        }
        KeyCode::Home => {
            set(cursor, step_selectable(entries, 0, true, true));
            MenuKey::Handled
        }
        KeyCode::End => {
            set(
                cursor,
                step_selectable(entries, entries.len().saturating_sub(1), false, true),
            );
            MenuKey::Handled
        }
        KeyCode::Right | KeyCode::Enter | KeyCode::Char(' ') => match entries.get(current) {
            Some(item) if item.selectable() && item.has_submenu() => {
                cursor
                    .path
                    .push(step_selectable(&item.children, 0, true, true).unwrap_or(0));
                MenuKey::Handled
            }
            Some(item) if item.selectable() && code != KeyCode::Right => {
                MenuKey::Activated(item.id.clone())
            }
            _ if code == KeyCode::Right => MenuKey::Sideways(true),
            _ => MenuKey::Handled,
        },
        KeyCode::Left if level > 0 => {
            cursor.path.pop();
            MenuKey::Handled
        }
        KeyCode::Left => MenuKey::Sideways(false),
        KeyCode::Esc if level > 0 => {
            cursor.path.pop();
            MenuKey::Handled
        }
        KeyCode::Esc => MenuKey::Close,
        _ => MenuKey::Ignored,
    }
}

/// Result of this frame's pointer input on an open menu.
enum MenuPointer {
    Idle,
    Activated(String),
    /// A press outside every menu level, at this cell.
    Outside((u32, u32)),
}

impl Context {
    /// Whether the pointer moved this frame (hover tracking for menus).
    fn pointer_moved(&self) -> bool {
        self.events.iter().any(|event| {
            matches!(event, Event::Mouse(mouse) if matches!(mouse.kind, MouseKind::Moved | MouseKind::Drag(_)))
        })
    }

    /// Unconsumed mouse presses this frame: `(event index, cell, button)`.
    pub(crate) fn open_presses(&self) -> SmallVec<[OpenPress; 2]> {
        self.pointer
            .presses
            .iter()
            .filter(|press| {
                !self
                    .consumed
                    .get(press.event_index)
                    .copied()
                    .unwrap_or(true)
            })
            .map(|press| (press.event_index, press.pos, press.button))
            .collect()
    }

    /// Hover and press handling for an open menu drawn at `rects`.
    fn menu_pointer(
        &mut self,
        items: &[MenuItem],
        cursor: &mut MenuCursor,
        rects: &[Rect],
    ) -> MenuPointer {
        // `rects` were laid out for the path drawn last frame; hit-test
        // every event of this frame against that path, not against cursor
        // moves made earlier in the batch.
        let drawn: SmallVec<[usize; 4]> = cursor.path.iter().copied().collect();
        // Point the cursor at `row` of `level` on the drawn path, opening its
        // submenu. Returns the entry when it is selectable.
        let select = |cursor: &mut MenuCursor, level: usize, row: usize| -> Option<MenuItem> {
            let item = level_items(items, &drawn, level)
                .get(row)
                .filter(|item| item.selectable())?;
            cursor.path.clear();
            cursor.path.extend_from_slice(&drawn[..level]);
            cursor.path.push(row);
            if item.has_submenu() {
                cursor
                    .path
                    .push(step_selectable(&item.children, 0, true, true).unwrap_or(0));
            }
            Some(item.clone())
        };
        if self.pointer_moved()
            && let Some(pos) = self.mouse_pos
            && let Some((level, Some(row))) = menu_hit(rects, items, &drawn, pos)
        {
            let _ = select(cursor, level, row);
        }
        for (event_index, pos, button) in self.open_presses() {
            self.consumed[event_index] = true;
            match menu_hit(rects, items, &drawn, pos) {
                None => return MenuPointer::Outside(pos),
                Some((_, None)) => {}
                Some((_, Some(_))) if button == MouseButton::Middle => {}
                Some((level, Some(row))) => {
                    if let Some(item) = select(cursor, level, row)
                        && !item.has_submenu()
                    {
                        return MenuPointer::Activated(item.id);
                    }
                }
            }
        }
        MenuPointer::Idle
    }

    /// Draw every open level of a menu.
    fn render_menu_levels(&mut self, items: &[MenuItem], path: &[usize], rects: &[Rect]) {
        let text = self.theme.surface_text;
        let dim = self.theme.text_dim;
        let rule = self.theme.border;
        let cursor_style = Style::new()
            .fg(self.theme.selected_fg)
            .bg(self.theme.selected_bg);
        for (level, rect) in rects.iter().enumerate() {
            let entries = level_items(items, path, level);
            let columns = MenuColumns::of(entries);
            let current = path.get(level).copied();
            let _ = self.floating_panel(*rect, false, |ui| {
                for (row, item) in entries.iter().enumerate() {
                    let style = if item.is_separator() {
                        Style::new().fg(rule)
                    } else if item.disabled {
                        Style::new().fg(dim)
                    } else if current == Some(row) {
                        cursor_style
                    } else {
                        Style::new().fg(text)
                    };
                    ui.styled(columns.row_text(item), style);
                }
            });
        }
    }

    /// Show a context menu for `trigger` and return the id of the entry
    /// activated this frame.
    ///
    /// The menu opens at the pointer when `trigger` is right-clicked, or below
    /// the trigger when it is focused and the user presses `Shift+F10` or the
    /// `Menu` key. While it is open:
    ///
    /// - `Up` / `Down` move between entries (skipping separators and disabled
    ///   entries), `Home` / `End` jump, `Right` / `Enter` open a submenu,
    ///   `Left` closes it, `Enter` / `Space` activate, `Esc` closes one level.
    /// - Hovering highlights entries and opens submenus; clicking an entry
    ///   activates it.
    /// - A click outside closes the menu and is **not** delivered to the
    ///   widget under the pointer; widgets behind the menu ignore the
    ///   keyboard too.
    /// - The menu flips above / to the left at screen edges.
    ///
    /// When the menu closes, keyboard focus returns to the widget that had it
    /// before the menu opened (the trigger, when it was focused).
    ///
    /// One `MenuState` can serve many triggers, such as every row of a list:
    /// the open menu is handled and drawn once per frame, and the call whose
    /// trigger opened it is the one after which [`MenuState::is_open`] turns
    /// `true`, so remember that row there.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::widgets::{MenuItem, MenuState};
    /// use slt::{EventBuilder, KeyCode, TestBackend};
    ///
    /// let mut menu = MenuState::new(vec![
    ///     MenuItem::new("open", "Open"),
    ///     MenuItem::new("rename", "Rename").shortcut("F2"),
    /// ]);
    /// let mut chosen = None;
    /// let mut tb = TestBackend::new(40, 10);
    /// let mut frame = |tb: &mut TestBackend, events| {
    ///     tb.sequence()
    ///         .events(events, |ui| {
    ///             let file = ui.button("notes.md").show();
    ///             if let Some(id) = ui.context_menu(&mut menu, &file) {
    ///                 chosen = Some(id);
    ///             }
    ///         })
    ///         .run();
    /// };
    /// frame(&mut tb, vec![]);
    /// frame(&mut tb, EventBuilder::new().key_with(KeyCode::F(10), slt::KeyModifiers::SHIFT).build());
    /// frame(&mut tb, EventBuilder::new().key_code(KeyCode::Down).build());
    /// frame(&mut tb, EventBuilder::new().key_code(KeyCode::Enter).build());
    /// assert_eq!(chosen.as_deref(), Some("rename"));
    /// ```
    pub fn context_menu(&mut self, menu: &mut MenuState, trigger: &Response) -> Option<String> {
        let area = self.screen_area();
        let mut activated = None;
        if !menu.is_open() {
            let mut anchor = None;
            if trigger.right_clicked {
                let pos = self
                    .pointer
                    .presses
                    .iter()
                    .rev()
                    .find(|press| {
                        press.button == MouseButton::Right
                            && trigger.rect.contains(press.pos.0, press.pos.1)
                    })
                    .map(|press| press.pos);
                anchor = Some(match pos {
                    Some((x, y)) => Rect::new(x, y, 0, 0),
                    None => trigger.rect,
                });
            } else if trigger.focused {
                let keys: SmallVec<[usize; 2]> = self
                    .available_key_presses()
                    .filter(|(_, key)| {
                        key.code == KeyCode::Menu
                            || (key.code == KeyCode::F(10)
                                && key.modifiers.contains(KeyModifiers::SHIFT))
                    })
                    .map(|(index, _)| index)
                    .collect();
                if !keys.is_empty() {
                    self.consume_indices(keys);
                    anchor = Some(trigger.rect);
                }
            }
            if let Some(anchor) = anchor {
                menu.open_at(anchor);
            }
        } else if menu.drawn_tick == Some(self.tick) {
            // Already handled and drawn by an earlier call this frame.
            return None;
        } else if menu.drawn_tick.is_some() {
            let rects = menu_level_rects(
                &menu.items,
                &menu.cursor.path,
                menu.anchor,
                menu.placement,
                area,
            );
            match self.menu_pointer(&menu.items, &mut menu.cursor, &rects) {
                MenuPointer::Activated(id) => activated = Some(id),
                MenuPointer::Outside(_) => menu.close(),
                MenuPointer::Idle => {}
            }
            if menu.is_open() && activated.is_none() {
                let keys: SmallVec<[(usize, KeyCode); 4]> = self
                    .available_key_presses()
                    .map(|(index, key)| (index, key.code.clone()))
                    .collect();
                for (index, code) in keys {
                    // The open menu owns the keyboard.
                    self.consumed[index] = true;
                    match menu_key(&menu.items, &mut menu.cursor, code) {
                        MenuKey::Activated(id) => {
                            activated = Some(id);
                            break;
                        }
                        MenuKey::Close => {
                            menu.close();
                            break;
                        }
                        MenuKey::Handled | MenuKey::Sideways(_) | MenuKey::Ignored => {}
                    }
                }
            }
            if activated.is_some() {
                menu.close();
            }
        }

        if menu.is_open() {
            menu.drawn_tick = Some(self.tick);
            let rects = menu_level_rects(
                &menu.items,
                &menu.cursor.path,
                menu.anchor,
                menu.placement,
                area,
            );
            let items = &menu.items;
            let path = &menu.cursor.path;
            let _ = self.modal_layer(true, false, |ui| ui.render_menu_levels(items, path, &rects));
        }
        activated
    }

    /// Render an application menu bar and return the id of the entry
    /// activated this frame.
    ///
    /// Draw it as the first row of the screen. Titles open their dropdown on
    /// click or with `Alt` + the title's first letter; `F10` activates the bar
    /// with the first title selected. While the bar is active:
    ///
    /// - `Left` / `Right` switch menus (also from inside a dropdown, unless a
    ///   submenu handles the key), `Down` / `Enter` open the selected menu.
    /// - Inside a dropdown, `Up` / `Down` skip separators and disabled
    ///   entries, `Right` / `Enter` open a submenu, `Left` / `Esc` close it,
    ///   `Enter` / `Space` activate.
    /// - `Esc` closes the dropdown, a second `Esc` (or `F10`) deactivates the
    ///   bar. Hovering another title while a dropdown is open switches to it.
    /// - A click outside the bar and its dropdown deactivates the bar and is
    ///   not delivered to the widget under the pointer.
    ///
    /// When the bar deactivates, focus returns to the widget that had it.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::widgets::{MenuBarState, MenuItem};
    /// use slt::{EventBuilder, KeyCode, TestBackend};
    ///
    /// let mut bar = MenuBarState::new(vec![
    ///     ("File", vec![MenuItem::new("new", "New"), MenuItem::new("quit", "Quit")]),
    ///     ("Edit", vec![MenuItem::new("undo", "Undo")]),
    /// ]);
    /// let mut chosen = None;
    /// let mut tb = TestBackend::new(40, 10);
    /// for key in [KeyCode::F(10), KeyCode::Right, KeyCode::Down, KeyCode::Enter] {
    ///     tb.sequence()
    ///         .events(EventBuilder::new().key_code(key).build(), |ui| {
    ///             if let Some(id) = ui.menu_bar(&mut bar) {
    ///                 chosen = Some(id);
    ///             }
    ///         })
    ///         .run();
    /// }
    /// assert_eq!(chosen.as_deref(), Some("undo"));
    /// ```
    pub fn menu_bar(&mut self, state: &mut MenuBarState) -> Option<String> {
        let area = self.screen_area();
        if state.menus.is_empty() {
            state.close();
            return None;
        }
        state.selected = state.selected.min(state.menus.len() - 1);
        let bar_id = self.next_interaction_id();
        let bar_rect = self.prev_hit_map.get(bar_id).copied().unwrap_or_default();
        // Title cells: ` Title ` per menu, laid out left to right.
        let mut spans: SmallVec<[Rect; 8]> = SmallVec::new();
        let mut x = bar_rect.x;
        for (title, _) in &state.menus {
            let width = UnicodeWidthStr::width(title.as_str()) as u32 + 2;
            spans.push(Rect::new(x, bar_rect.y, width, 1));
            x = x.saturating_add(width);
        }
        let title_at = |pos: (u32, u32)| spans.iter().position(|span| span.contains(pos.0, pos.1));
        let mnemonic = |menus: &[(String, Vec<MenuItem>)], c: char| {
            menus.iter().position(|(title, _)| {
                title
                    .chars()
                    .next()
                    .is_some_and(|first| first.to_lowercase().eq(c.to_lowercase()))
            })
        };
        let open_menu = |state: &mut MenuBarState, index: usize| {
            state.active = true;
            state.selected = index;
            state.cursor.open(&state.menus[index].1);
        };

        let mut activated = None;
        if !state.active {
            if self.interaction_allowed() && !self.is_disabled() {
                let keys: SmallVec<[(usize, KeyCode, KeyModifiers); 4]> = self
                    .available_key_presses()
                    .map(|(index, key)| (index, key.code.clone(), key.modifiers))
                    .collect();
                for (index, code, modifiers) in keys {
                    match code {
                        KeyCode::F(10) if modifiers == KeyModifiers::NONE => {
                            self.consumed[index] = true;
                            state.active = true;
                            state.selected = 0;
                            state.cursor.close();
                            break;
                        }
                        KeyCode::Char(c) if modifiers == KeyModifiers::ALT => {
                            if let Some(menu) = mnemonic(&state.menus, c) {
                                self.consumed[index] = true;
                                open_menu(state, menu);
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                if !state.active
                    && let Some((_, clicks)) = self.left_clicks_for_interaction(bar_id)
                {
                    let hit = clicks.iter().find_map(|(index, mouse)| {
                        title_at((mouse.x, mouse.y)).map(|t| (*index, t))
                    });
                    if let Some((index, title)) = hit {
                        self.consumed[index] = true;
                        open_menu(state, title);
                    }
                }
            }
        } else {
            let selected = state.selected;
            let rects = if state.cursor.is_open() {
                menu_level_rects(
                    &state.menus[selected].1,
                    &state.cursor.path,
                    spans[selected],
                    Placement::Below,
                    area,
                )
            } else {
                LevelRects::new()
            };
            // Presses are hit-tested against the dropdown drawn last frame
            // before hover may switch menus: a click on the title that hover
            // switches to in this same frame then opens it like any click on
            // another title, instead of being skipped (and left unconsumed)
            // because the open menu no longer matches the drawn rects.
            if state.cursor.is_open() {
                let MenuBarState { menus, cursor, .. } = &mut *state;
                match self.menu_pointer(&menus[selected].1, cursor, &rects) {
                    MenuPointer::Activated(id) => activated = Some(id),
                    MenuPointer::Outside(pos) => match title_at(pos) {
                        Some(title) if title == selected => state.close(),
                        Some(title) => open_menu(state, title),
                        None => state.close(),
                    },
                    MenuPointer::Idle => {}
                }
            } else if !state.cursor.is_open()
                && let Some(&(event_index, pos, _)) = self.open_presses().first()
            {
                self.consumed[event_index] = true;
                match title_at(pos) {
                    Some(title) => open_menu(state, title),
                    None => state.close(),
                }
            }
            // Hovering another title while a dropdown is open switches menus.
            if activated.is_none()
                && state.cursor.is_open()
                && self.pointer_moved()
                && let Some(title) = self.mouse_pos.and_then(title_at)
                && title != state.selected
            {
                open_menu(state, title);
            }
            if state.active && activated.is_none() {
                let keys: SmallVec<[(usize, KeyCode, KeyModifiers); 4]> = self
                    .available_key_presses()
                    .map(|(index, key)| (index, key.code.clone(), key.modifiers))
                    .collect();
                let count = state.menus.len();
                for (index, code, modifiers) in keys {
                    // The active bar owns the keyboard.
                    self.consumed[index] = true;
                    if code == KeyCode::F(10) {
                        state.close();
                        break;
                    }
                    if let KeyCode::Char(c) = code
                        && modifiers == KeyModifiers::ALT
                    {
                        if let Some(menu) = mnemonic(&state.menus, c) {
                            open_menu(state, menu);
                        }
                        continue;
                    }
                    let sideways = |state: &mut MenuBarState, forward: bool, open: bool| {
                        let next = if forward {
                            (state.selected + 1) % count
                        } else {
                            (state.selected + count - 1) % count
                        };
                        if open {
                            open_menu(state, next);
                        } else {
                            state.selected = next;
                        }
                    };
                    if state.cursor.is_open() {
                        let selected = state.selected;
                        let MenuBarState { menus, cursor, .. } = &mut *state;
                        match menu_key(&menus[selected].1, cursor, code) {
                            MenuKey::Activated(id) => {
                                activated = Some(id);
                                break;
                            }
                            MenuKey::Close => state.cursor.close(),
                            MenuKey::Sideways(forward) => sideways(state, forward, true),
                            MenuKey::Handled | MenuKey::Ignored => {}
                        }
                    } else {
                        match code {
                            KeyCode::Left => sideways(state, false, false),
                            KeyCode::Right => sideways(state, true, false),
                            KeyCode::Down | KeyCode::Up | KeyCode::Enter | KeyCode::Char(' ') => {
                                let selected = state.selected;
                                open_menu(state, selected);
                            }
                            KeyCode::Esc => {
                                state.close();
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
            if activated.is_some() {
                state.close();
            }
        }

        // The bar itself: one full-width row in the base layout.
        let surface = self.theme.surface;
        let text = self.theme.surface_text;
        let highlight = Style::new()
            .fg(self.theme.selected_fg)
            .bg(self.theme.selected_bg);
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
                bg_color: Some(surface),
                padding: Padding::default(),
                margin: Margin::default(),
                constraints: Constraints::default().w_pct(100),
                title: None,
                grow: 0,
                group_name: None,
            })));
        for (index, (title, _)) in state.menus.iter().enumerate() {
            let style = if state.active && index == state.selected {
                highlight
            } else {
                Style::new().fg(text)
            };
            let mut cell = String::with_capacity(title.len() + 2);
            cell.push(' ');
            cell.push_str(title);
            cell.push(' ');
            self.styled(cell, style);
        }
        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;

        if state.active {
            let rects = if state.cursor.is_open() {
                menu_level_rects(
                    &state.menus[state.selected].1,
                    &state.cursor.path,
                    spans[state.selected],
                    Placement::Below,
                    area,
                )
            } else {
                LevelRects::new()
            };
            let items = &state.menus[state.selected].1;
            let path = &state.cursor.path;
            let _ = self.modal_layer(true, false, |ui| ui.render_menu_levels(items, path, &rects));
        }
        activated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items() -> Vec<MenuItem> {
        vec![
            MenuItem::new("copy", "Copy").shortcut("Ctrl+C"),
            MenuItem::separator(),
            MenuItem::submenu("more", "More", vec![MenuItem::new("x", "Extra")]),
        ]
    }

    #[test]
    fn rows_share_one_width() {
        let items = items();
        let columns = MenuColumns::of(&items);
        let widths: Vec<usize> = items
            .iter()
            .map(|item| UnicodeWidthStr::width(columns.row_text(item).as_str()))
            .collect();
        assert!(
            widths.iter().all(|&w| w == columns.inner_width() as usize),
            "{widths:?}"
        );
        assert_eq!(menu_size(&items), (columns.inner_width() + 2, 5));
    }

    #[test]
    fn submenu_opens_to_the_right_of_its_row() {
        let items = items();
        let area = Rect::new(0, 0, 60, 20);
        let rects = menu_level_rects(
            &items,
            &[2, 0],
            Rect::new(3, 1, 0, 0),
            Placement::Below,
            area,
        );
        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].x, 3);
        assert_eq!(rects[0].y, 1);
        // Row 2 of the root sits at y = 1 (top) + 1 (border) + 2.
        assert_eq!(rects[1].x, rects[0].right());
        assert_eq!(rects[1].y, 3);
    }

    #[test]
    fn hit_test_reports_rows_and_borders() {
        let items = items();
        let rects = [Rect::new(0, 0, 12, 5)];
        assert_eq!(menu_hit(&rects, &items, &[0], (3, 1)), Some((0, Some(0))));
        assert_eq!(menu_hit(&rects, &items, &[0], (3, 0)), Some((0, None)));
        assert_eq!(menu_hit(&rects, &items, &[0], (0, 2)), Some((0, None)));
        assert_eq!(menu_hit(&rects, &items, &[0], (20, 2)), None);
    }

    #[test]
    fn keys_walk_skip_and_open_submenus() {
        let items = items();
        let mut cursor = MenuCursor::default();
        cursor.open(&items);
        assert_eq!(cursor.path, vec![0]);
        assert!(matches!(
            menu_key(&items, &mut cursor, KeyCode::Down),
            MenuKey::Handled
        ));
        assert_eq!(cursor.path, vec![2], "separator skipped");
        assert!(matches!(
            menu_key(&items, &mut cursor, KeyCode::Right),
            MenuKey::Handled
        ));
        assert_eq!(cursor.path, vec![2, 0]);
        assert!(matches!(
            menu_key(&items, &mut cursor, KeyCode::Enter),
            MenuKey::Activated(id) if id == "x"
        ));
        assert!(matches!(
            menu_key(&items, &mut cursor, KeyCode::Esc),
            MenuKey::Handled
        ));
        assert_eq!(cursor.path, vec![2]);
        assert!(matches!(
            menu_key(&items, &mut cursor, KeyCode::Left),
            MenuKey::Sideways(false)
        ));
        assert!(matches!(
            menu_key(&items, &mut cursor, KeyCode::Esc),
            MenuKey::Close
        ));
    }
}
