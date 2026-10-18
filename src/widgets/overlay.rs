//! Overlay family state: menus for [`Context::context_menu`] and
//! [`Context::menu_bar`], plus the [`PopoverResponse`] returned by
//! [`Context::popover`].
//!
//! [`Context::context_menu`]: crate::Context::context_menu
//! [`Context::menu_bar`]: crate::Context::menu_bar
//! [`Context::popover`]: crate::Context::popover

use crate::context::{Placement, Response};
use crate::rect::Rect;

/// One entry of a [`MenuState`] or [`MenuBarState`] menu.
///
/// An entry is an action (returns its `id` when activated), a separator line,
/// or a submenu (opens a nested menu to the side). Disabled entries render
/// dimmed and are skipped by the keyboard and ignored by the mouse.
///
/// # Example
///
/// ```
/// use slt::widgets::MenuItem;
///
/// let items = vec![
///     MenuItem::new("copy", "Copy").shortcut("Ctrl+C"),
///     MenuItem::new("paste", "Paste").disabled(true),
///     MenuItem::separator(),
///     MenuItem::submenu("share", "Share", vec![MenuItem::new("mail", "Mail")]),
/// ];
/// assert!(items[1].disabled);
/// assert!(items[2].is_separator());
/// assert!(items[3].has_submenu());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MenuItem {
    /// Identifier returned when the entry is activated.
    pub id: String,
    /// Visible label.
    pub label: String,
    /// Optional shortcut hint rendered right-aligned (display only).
    pub shortcut: Option<String>,
    /// Disabled entries are dimmed, skipped by the keyboard and ignore clicks.
    pub disabled: bool,
    /// Entries of the nested menu this entry opens; empty for actions.
    pub children: Vec<MenuItem>,
    separator: bool,
}

impl MenuItem {
    /// An action entry returning `id` when activated.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            shortcut: None,
            disabled: false,
            children: Vec::new(),
            separator: false,
        }
    }

    /// A horizontal separator line. Never focusable or clickable.
    pub fn separator() -> Self {
        Self {
            separator: true,
            ..Self::new("", "")
        }
    }

    /// An entry that opens `children` as a nested menu (Right / Enter / click).
    pub fn submenu(
        id: impl Into<String>,
        label: impl Into<String>,
        children: Vec<MenuItem>,
    ) -> Self {
        Self {
            children,
            ..Self::new(id, label)
        }
    }

    /// Show a shortcut hint such as `"Ctrl+S"` on the right of the entry.
    ///
    /// Display only: bind the key yourself.
    pub fn shortcut(mut self, hint: impl Into<String>) -> Self {
        self.shortcut = Some(hint.into());
        self
    }

    /// Disable or enable the entry.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Whether this entry is a separator line.
    pub fn is_separator(&self) -> bool {
        self.separator
    }

    /// Whether this entry opens a nested menu.
    pub fn has_submenu(&self) -> bool {
        !self.children.is_empty()
    }

    /// Whether the keyboard and mouse can land on this entry.
    pub(crate) fn selectable(&self) -> bool {
        !self.separator && !self.disabled
    }
}

/// Index of the first selectable entry at or after `from` (wrapping), moving
/// forward or backward. `None` when no entry is selectable.
pub(crate) fn step_selectable(
    items: &[MenuItem],
    from: usize,
    forward: bool,
    include_from: bool,
) -> Option<usize> {
    let len = items.len();
    if len == 0 {
        return None;
    }
    let start = from.min(len - 1);
    for step in 0..len {
        let offset = if include_from { step } else { step + 1 };
        let index = if forward {
            (start + offset) % len
        } else {
            (start + len - offset % len) % len
        };
        if items[index].selectable() {
            return Some(index);
        }
    }
    None
}

/// Open/closed state and keyboard cursor of one menu, shared by
/// [`MenuState`] and the dropdowns of [`MenuBarState`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct MenuCursor {
    /// Highlighted entry per open level: `path[0]` in the root menu,
    /// `path[1]` in the submenu opened from `path[0]`, and so on. Empty
    /// while closed.
    pub path: Vec<usize>,
}

impl MenuCursor {
    pub(crate) fn open(&mut self, items: &[MenuItem]) {
        self.path.clear();
        self.path
            .push(step_selectable(items, 0, true, true).unwrap_or(0));
    }

    pub(crate) fn close(&mut self) {
        self.path.clear();
    }

    pub(crate) fn is_open(&self) -> bool {
        !self.path.is_empty()
    }
}

/// Resolve the entries shown at `level` for an open `path`.
pub(crate) fn level_items<'a>(
    items: &'a [MenuItem],
    path: &[usize],
    level: usize,
) -> &'a [MenuItem] {
    let mut current = items;
    for &index in path.iter().take(level) {
        match current.get(index) {
            Some(item) => current = &item.children,
            None => return &[],
        }
    }
    current
}

/// State for a right-click / keyboard context menu
/// ([`Context::context_menu`](crate::Context::context_menu)).
///
/// Holds the entries and whether the menu is open, where it opened and which
/// entry the keyboard cursor is on. Create it once and keep it with your app
/// state.
///
/// # Example
///
/// ```
/// use slt::widgets::{MenuItem, MenuState};
///
/// let mut menu = MenuState::new(vec![
///     MenuItem::new("rename", "Rename"),
///     MenuItem::new("delete", "Delete").shortcut("Del"),
/// ]);
/// assert!(!menu.is_open());
/// menu.open_at(slt::Rect::new(4, 2, 0, 0));
/// assert!(menu.is_open());
/// menu.close();
/// assert!(!menu.is_open());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MenuState {
    /// Menu entries. Edit freely while the menu is closed.
    pub items: Vec<MenuItem>,
    pub(crate) cursor: MenuCursor,
    pub(crate) anchor: Rect,
    pub(crate) placement: Placement,
    /// Frame tick the open menu was last drawn on: `None` until its first
    /// draw (the opening input is not also an outside click), and equal to
    /// the current tick once drawn this frame, so one `MenuState` shared by
    /// several rows is handled and drawn once per frame.
    pub(crate) drawn_tick: Option<u64>,
}

impl MenuState {
    /// Create a closed menu with `items`.
    pub fn new(items: Vec<MenuItem>) -> Self {
        Self {
            items,
            ..Self::default()
        }
    }

    /// Whether the menu is open.
    pub fn is_open(&self) -> bool {
        self.cursor.is_open()
    }

    /// Open the menu programmatically with its top-left corner below `anchor`
    /// (flipping above / clamping at the screen edges). A zero-size anchor at
    /// `(x, y)` opens the menu at that cell, as a right click does.
    pub fn open_at(&mut self, anchor: Rect) {
        self.anchor = anchor;
        self.placement = Placement::Below;
        self.drawn_tick = None;
        self.cursor.open(&self.items);
    }

    /// Close the menu (and any open submenu).
    pub fn close(&mut self) {
        self.cursor.close();
        self.drawn_tick = None;
    }

    /// Index of the highlighted entry in the innermost open menu level.
    pub fn cursor(&self) -> Option<usize> {
        self.cursor.path.last().copied()
    }
}

/// State for an application menu bar ([`Context::menu_bar`](crate::Context::menu_bar)).
///
/// Holds the menus (title + entries), whether the bar is active (keyboard
/// focus on the bar after `F10` / `Alt`), which title is selected and the
/// open dropdown's cursor.
///
/// # Example
///
/// ```
/// use slt::widgets::{MenuBarState, MenuItem};
///
/// let bar = MenuBarState::new(vec![
///     ("File", vec![MenuItem::new("open", "Open…"), MenuItem::new("quit", "Quit")]),
///     ("Edit", vec![MenuItem::new("undo", "Undo").shortcut("Ctrl+Z")]),
/// ]);
/// assert_eq!(bar.titles().count(), 2);
/// assert!(!bar.is_active());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MenuBarState {
    /// `(title, entries)` per menu, in display order.
    pub menus: Vec<(String, Vec<MenuItem>)>,
    pub(crate) active: bool,
    pub(crate) selected: usize,
    pub(crate) cursor: MenuCursor,
}

impl MenuBarState {
    /// Create an inactive menu bar from `(title, entries)` pairs.
    pub fn new<S: Into<String>>(menus: impl IntoIterator<Item = (S, Vec<MenuItem>)>) -> Self {
        Self {
            menus: menus
                .into_iter()
                .map(|(title, items)| (title.into(), items))
                .collect(),
            ..Self::default()
        }
    }

    /// Menu titles in display order.
    pub fn titles(&self) -> impl Iterator<Item = &str> {
        self.menus.iter().map(|(title, _)| title.as_str())
    }

    /// Whether the bar holds the keyboard (after `F10`, `Alt+letter` or a
    /// click on a title).
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Whether a dropdown is open.
    pub fn is_open(&self) -> bool {
        self.active && self.cursor.is_open()
    }

    /// Index of the selected (highlighted) title.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Deactivate the bar and close any dropdown.
    pub fn close(&mut self) {
        self.active = false;
        self.cursor.close();
    }
}

/// Response of [`Context::popover`](crate::Context::popover).
///
/// Derefs to the [`Response`] of the floating panel (its `rect`, `hovered`,
/// ...). `placement` is the side the panel ended up on after flipping at a
/// screen edge, and `dismissed` reports a light-dismiss gesture this frame:
/// `Esc`, or a mouse press outside both the panel and its anchor.
#[derive(Debug, Clone, Default)]
#[must_use = "PopoverResponse reports `dismissed`; close the popover when it is set"]
pub struct PopoverResponse {
    /// Response of the floating panel.
    pub response: Response,
    /// Side the panel was placed on after flipping.
    pub placement: Placement,
    /// `Esc` was pressed, or a press landed outside the panel and anchor.
    pub dismissed: bool,
}

impl std::ops::Deref for PopoverResponse {
    type Target = Response;

    fn deref(&self) -> &Response {
        &self.response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items() -> Vec<MenuItem> {
        vec![
            MenuItem::new("a", "A").disabled(true),
            MenuItem::new("b", "B"),
            MenuItem::separator(),
            MenuItem::new("c", "C"),
        ]
    }

    #[test]
    fn step_selectable_skips_disabled_and_separators() {
        let items = items();
        assert_eq!(step_selectable(&items, 0, true, true), Some(1));
        assert_eq!(step_selectable(&items, 1, true, false), Some(3));
        assert_eq!(step_selectable(&items, 3, true, false), Some(1));
        assert_eq!(step_selectable(&items, 1, false, false), Some(3));
        assert_eq!(step_selectable(&items, 3, false, false), Some(1));
        let none = vec![
            MenuItem::separator(),
            MenuItem::new("x", "X").disabled(true),
        ];
        assert_eq!(step_selectable(&none, 0, true, true), None);
        assert_eq!(step_selectable(&[], 0, true, true), None);
    }

    #[test]
    fn level_items_follows_the_open_path() {
        let items = vec![MenuItem::submenu(
            "more",
            "More",
            vec![MenuItem::new("x", "X"), MenuItem::new("y", "Y")],
        )];
        assert_eq!(level_items(&items, &[0], 0).len(), 1);
        assert_eq!(level_items(&items, &[0, 1], 1).len(), 2);
        assert!(level_items(&items, &[5, 0], 1).is_empty());
    }

    #[test]
    fn menu_opens_on_first_selectable_entry() {
        let mut menu = MenuState::new(items());
        menu.open_at(Rect::new(1, 1, 0, 0));
        assert_eq!(menu.cursor(), Some(1));
        menu.close();
        assert_eq!(menu.cursor(), None);
    }
}
