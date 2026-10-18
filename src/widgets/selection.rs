//! Selection family state — select, radio, multi-select, tabs, button
//! variants and the color picker.
//!
//! Rendered by `context/widgets/selection.rs`.

use super::*;

// ── Select / Dropdown ─────────────────────────────────────────────────

/// State for a dropdown select widget.
///
/// Renders as a single-line button showing the selected option. When activated,
/// expands into a vertical list overlay for picking an option.
#[derive(Debug, Clone, Default)]
pub struct SelectState {
    /// Selectable option labels.
    items: Vec<String>,
    /// Selected option, as a data index into [`items`](SelectState::items).
    /// The type-to-filter query only narrows the open dropdown; it never
    /// changes what this index refers to.
    pub selected: usize,
    /// Whether the dropdown list is currently open.
    pub open: bool,
    /// Placeholder text shown when `items` is empty.
    pub placeholder: String,
    cursor: usize,
    /// Type-to-filter query, active only while the dropdown is open. Reset on
    /// open/close and on selection. Printable keys append; Backspace pops.
    /// Public so callers can pre-fill or inspect the live query.
    pub filter: String,
    /// Whether the floating dropdown (and its light-dismiss backdrop) was
    /// drawn on the previous frame, so this frame's presses hit it (#459).
    pub(crate) dropdown_drawn: bool,
}

impl SelectState {
    /// Create select state with the provided options.
    pub fn new(items: Vec<impl Into<String>>) -> Self {
        Self {
            items: items.into_iter().map(Into::into).collect(),
            selected: 0,
            open: false,
            placeholder: String::new(),
            cursor: 0,
            filter: String::new(),
            dropdown_drawn: false,
        }
    }

    /// Return all selectable option labels.
    pub fn items(&self) -> &[String] {
        &self.items
    }

    /// Return the number of options.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Return whether there are no options.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Replace all options and clamp selection state immediately.
    pub fn set_items(&mut self, items: Vec<impl Into<String>>) {
        self.items = items.into_iter().map(Into::into).collect();
        self.selected = self.selected.min(self.items.len().saturating_sub(1));
        self.cursor = self.cursor.min(self.items.len().saturating_sub(1));
        if self.items.is_empty() {
            self.selected = 0;
            self.cursor = 0;
            self.open = false;
        }
    }

    /// Indices of `items` that match the current type-to-filter query, in
    /// original order. An empty query matches every item. Matching reuses the
    /// shared fuzzy matcher so a gapped pattern (e.g. `"sf"` → `"San Francisco"`)
    /// still hits.
    pub(crate) fn filtered_indices(&self) -> Vec<usize> {
        if self.filter.is_empty() {
            return (0..self.items.len()).collect();
        }
        (0..self.items.len())
            .filter(|&i| {
                crate::widgets::CommandPaletteState::fuzzy_score(&self.filter, &self.items[i])
                    .is_some()
            })
            .collect()
    }

    /// Set placeholder text shown when no item can be displayed.
    pub fn placeholder(mut self, p: impl Into<String>) -> Self {
        self.placeholder = p.into();
        self
    }

    /// Returns the currently selected item label, or `None` if empty.
    pub fn selected_item(&self) -> Option<&str> {
        self.items.get(self.selected).map(String::as_str)
    }

    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }

    pub(crate) fn set_cursor(&mut self, c: usize) {
        self.cursor = c;
    }
}

// ── Radio ─────────────────────────────────────────────────────────────

/// State for a radio button group.
///
/// Renders a vertical list of mutually-exclusive options with `●`/`○` markers.
#[derive(Debug, Clone, Default)]
pub struct RadioState {
    /// Radio option labels.
    pub items: Vec<String>,
    /// Selected option, as an index into [`items`](RadioState::items).
    pub selected: usize,
}

impl RadioState {
    /// Create radio state with the provided options.
    pub fn new(items: Vec<impl Into<String>>) -> Self {
        Self {
            items: items.into_iter().map(Into::into).collect(),
            selected: 0,
        }
    }

    /// Returns the currently selected option label, or `None` if empty.
    pub fn selected_item(&self) -> Option<&str> {
        self.items.get(self.selected).map(String::as_str)
    }
}

// ── Multi-Select ──────────────────────────────────────────────────────

/// State for a multi-select list.
///
/// Like [`ListState`] but allows toggling multiple items with Space.
#[derive(Debug, Clone)]
pub struct MultiSelectState {
    /// Multi-select option labels.
    items: Vec<String>,
    /// Focused option index used for keyboard navigation.
    pub cursor: usize,
    /// Set of selected option indices.
    pub selected: HashSet<usize>,
}

impl MultiSelectState {
    /// Create multi-select state with the provided options.
    pub fn new(items: Vec<impl Into<String>>) -> Self {
        Self {
            items: items.into_iter().map(Into::into).collect(),
            cursor: 0,
            selected: HashSet::new(),
        }
    }

    /// Return all option labels.
    pub fn items(&self) -> &[String] {
        &self.items
    }

    /// Return the number of options.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Return whether there are no options.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Replace all options and prune stale selected indices.
    pub fn set_items(&mut self, items: Vec<impl Into<String>>) {
        self.items = items.into_iter().map(Into::into).collect();
        self.selected.retain(|index| *index < self.items.len());
        self.cursor = self.cursor.min(self.items.len().saturating_sub(1));
        if self.items.is_empty() {
            self.cursor = 0;
        }
    }

    /// Return selected item labels in ascending index order.
    pub fn selected_items(&self) -> Vec<&str> {
        let mut indices: Vec<usize> = self.selected.iter().copied().collect();
        indices.sort();
        indices
            .iter()
            .filter_map(|&i| self.items.get(i).map(String::as_str))
            .collect()
    }

    /// Toggle selection state for `index`.
    pub fn toggle(&mut self, index: usize) {
        if index >= self.items.len() {
            return;
        }
        if self.selected.contains(&index) {
            self.selected.remove(&index);
        } else {
            self.selected.insert(index);
        }
    }
}

/// State for a tab navigation widget.
///
/// Pass a mutable reference to `Context::tabs` each frame. Left/Right arrow
/// keys cycle through tabs when the widget is focused.
#[derive(Debug, Clone, Default)]
pub struct TabsState {
    /// The tab labels displayed in the bar.
    pub labels: Vec<String>,
    /// Index of the currently active tab.
    pub selected: usize,
}

impl TabsState {
    /// Create tabs with the given labels. The first tab is active initially.
    pub fn new(labels: Vec<impl Into<String>>) -> Self {
        Self {
            labels: labels.into_iter().map(Into::into).collect(),
            selected: 0,
        }
    }

    /// Get the currently selected tab label, or `None` if there are no tabs.
    pub fn selected_label(&self) -> Option<&str> {
        self.labels.get(self.selected).map(String::as_str)
    }
}

/// Visual variant for buttons.
///
/// Controls the color scheme used when rendering a button. Pass to
/// [`crate::Context::button_with`] to create styled button variants.
///
/// - `Default` — theme text color, primary when focused (same as `button()`)
/// - `Primary` — primary color background with contrasting text
/// - `Danger` — error/red color for destructive actions
/// - `Outline` — bordered appearance without fill
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ButtonVariant {
    /// Standard button style.
    #[default]
    Default,
    /// Filled button with primary background color.
    Primary,
    /// Filled button with error/danger background color.
    Danger,
    /// Bordered button without background fill.
    Outline,
}

// ── Color Picker ──────────────────────────────────────────────────────

/// Interaction mode of a [`ColorPickerState`].
///
/// Toggle between the two modes with `Tab` when the picker is focused.
///
/// # Example
///
/// ```no_run
/// # use slt::widgets::{ColorPickerState, PickerMode};
/// let mut picker = ColorPickerState::tailwind();
/// assert_eq!(picker.mode, PickerMode::Palette);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PickerMode {
    /// Navigate the 2D swatch grid with the arrow keys / `hjkl`.
    #[default]
    Palette,
    /// Enter a `#RRGGBB` or `#RGB` hex string in the embedded text field.
    Hex,
}

/// State for an interactive color picker over the [`Color`](crate::Color) model.
///
/// Renders a grid of color swatches plus an optional hex-entry field. Pass a
/// mutable reference to [`Context::color_picker`](crate::Context::color_picker)
/// each frame; read the chosen color back via [`selected`](Self::selected).
///
/// Swatches are emitted with a full-RGB background; the terminal backend
/// downsamples each cell to the active [`ColorDepth`](crate::ColorDepth) on
/// flush (see [`Color::downsampled`](crate::Color::downsampled)), so the picker
/// degrades correctly on 256-color, 16-color, and no-color terminals. Every
/// `Color::Rgb` swatch also carries a `#RRGGBB` label rendered with a
/// [`Color::contrast_fg`](crate::Color::contrast_fg) foreground, so the picker
/// stays legible even when no background color is emitted.
///
/// # Example
///
/// ```no_run
/// # use slt::widgets::ColorPickerState;
/// # slt::run(|ui: &mut slt::Context| {
/// let mut picker = ColorPickerState::tailwind();
/// let resp = ui.color_picker(&mut picker);
/// if resp.changed {
///     let chosen = picker.selected();
///     // persist `chosen` somewhere…
/// }
/// # });
/// ```
#[derive(Debug, Clone)]
pub struct ColorPickerState {
    /// Swatch colors laid out row-major.
    pub colors: Vec<crate::Color>,
    /// Number of swatches per row (minimum 1, default 8).
    pub columns: usize,
    /// Flat index of the selected swatch into [`colors`](Self::colors).
    pub selected: usize,
    /// Whether the picker is in palette-grid or hex-entry mode.
    pub mode: PickerMode,
    /// Backing text field used in [`PickerMode::Hex`].
    pub hex_input: TextInputState,
}

impl ColorPickerState {
    /// Create a picker over the given swatches with the first one selected.
    ///
    /// Defaults to 8 columns and [`PickerMode::Palette`]. An empty `colors`
    /// vector is allowed; the widget renders nothing and reports no change.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::widgets::ColorPickerState;
    /// # use slt::Color;
    /// let picker = ColorPickerState::new(vec![
    ///     Color::Rgb(239, 68, 68),
    ///     Color::Rgb(59, 130, 246),
    /// ]);
    /// assert_eq!(picker.columns, 8);
    /// ```
    pub fn new(colors: Vec<crate::Color>) -> Self {
        Self {
            colors,
            columns: 8,
            selected: 0,
            mode: PickerMode::Palette,
            hex_input: TextInputState::with_placeholder("#RRGGBB"),
        }
    }

    /// Build a picker from the Tailwind `c500` shades in
    /// [`crate::palette::tailwind`].
    ///
    /// Includes all 22 palettes from `SLATE` through `ROSE`, in declaration
    /// order, giving a balanced default swatch grid.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::widgets::ColorPickerState;
    /// let picker = ColorPickerState::tailwind();
    /// assert_eq!(picker.colors.len(), 22);
    /// ```
    pub fn tailwind() -> Self {
        use crate::palette::tailwind;
        let colors = vec![
            tailwind::SLATE.c500,
            tailwind::GRAY.c500,
            tailwind::ZINC.c500,
            tailwind::NEUTRAL.c500,
            tailwind::STONE.c500,
            tailwind::RED.c500,
            tailwind::ORANGE.c500,
            tailwind::AMBER.c500,
            tailwind::YELLOW.c500,
            tailwind::LIME.c500,
            tailwind::GREEN.c500,
            tailwind::EMERALD.c500,
            tailwind::TEAL.c500,
            tailwind::CYAN.c500,
            tailwind::SKY.c500,
            tailwind::BLUE.c500,
            tailwind::INDIGO.c500,
            tailwind::VIOLET.c500,
            tailwind::PURPLE.c500,
            tailwind::FUCHSIA.c500,
            tailwind::PINK.c500,
            tailwind::ROSE.c500,
        ];
        Self::new(colors)
    }

    /// Set the number of swatches per row (clamped to at least 1).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::widgets::ColorPickerState;
    /// let picker = ColorPickerState::tailwind().columns(6);
    /// assert_eq!(picker.columns, 6);
    /// ```
    pub fn columns(mut self, n: usize) -> Self {
        self.columns = n.max(1);
        self
    }

    /// Return the currently selected color.
    ///
    /// In [`PickerMode::Hex`] a successfully parsed `#RRGGBB` / `#RGB` value
    /// takes precedence; otherwise the highlighted palette swatch is returned.
    /// Falls back to [`Color::Reset`](crate::Color::Reset) when the palette is
    /// empty and no valid hex value has been entered.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::widgets::ColorPickerState;
    /// # use slt::Color;
    /// let picker = ColorPickerState::new(vec![Color::Rgb(59, 130, 246)]);
    /// assert_eq!(picker.selected(), Color::Rgb(59, 130, 246));
    /// ```
    pub fn selected(&self) -> crate::Color {
        if self.mode == PickerMode::Hex
            && let Some(c) = parse_hex_color(&self.hex_input.value)
        {
            return c;
        }
        self.colors
            .get(self.selected)
            .copied()
            .unwrap_or(crate::Color::Reset)
    }
}

/// Parse a `#RRGGBB` or `#RGB` hex string into a [`Color::Rgb`](crate::Color).
///
/// Returns `None` for malformed input (wrong length, non-hex digits, missing
/// `#`). The leading `#` is required; surrounding whitespace is trimmed.
pub(crate) fn parse_hex_color(input: &str) -> Option<crate::Color> {
    crate::Color::from_hex(input.trim())
}

/// Render `color` as a `#RRGGBB` label, or `None` for non-RGB colors.
///
/// Used by the color picker to label swatches so they stay legible when the
/// terminal emits no background color.
pub(crate) fn color_hex_label(color: crate::Color) -> Option<String> {
    match color {
        crate::Color::Rgb(r, g, b) => Some(format!("#{r:02X}{g:02X}{b:02X}")),
        _ => None,
    }
}
