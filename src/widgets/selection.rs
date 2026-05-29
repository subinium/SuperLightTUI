/// State for a dropdown select widget.
///
/// Renders as a single-line button showing the selected option. When activated,
/// expands into a vertical list overlay for picking an option.
#[derive(Debug, Clone, Default)]
pub struct SelectState {
    /// Selectable option labels.
    pub items: Vec<String>,
    /// Selected option index.
    pub selected: usize,
    /// Whether the dropdown list is currently open.
    pub open: bool,
    /// Placeholder text shown when `items` is empty.
    pub placeholder: String,
    cursor: usize,
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
        }
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
    /// Selected option index.
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
    pub items: Vec<String>,
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
        if self.selected.contains(&index) {
            self.selected.remove(&index);
        } else {
            self.selected.insert(index);
        }
    }
}

// ── Tree ──────────────────────────────────────────────────────────────

/// A node in a tree view.
#[derive(Debug, Clone)]
pub struct TreeNode {
    /// Display label for this node.
    pub label: String,
    /// Child nodes.
    pub children: Vec<TreeNode>,
    /// Whether the node is expanded in the tree view.
    pub expanded: bool,
}

impl TreeNode {
    /// Create a collapsed tree node with no children.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            children: Vec::new(),
            expanded: false,
        }
    }

    /// Mark this node as expanded.
    pub fn expanded(mut self) -> Self {
        self.expanded = true;
        self
    }

    /// Set child nodes for this node.
    pub fn children(mut self, children: Vec<TreeNode>) -> Self {
        self.children = children;
        self
    }

    /// Returns `true` when this node has no children.
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    fn flatten(&self, depth: usize, out: &mut Vec<FlatTreeEntry>) {
        out.push(FlatTreeEntry {
            depth,
            label: self.label.clone(),
            is_leaf: self.is_leaf(),
            expanded: self.expanded,
        });
        if self.expanded {
            for child in &self.children {
                child.flatten(depth + 1, out);
            }
        }
    }
}

pub(crate) struct FlatTreeEntry {
    pub depth: usize,
    pub label: String,
    pub is_leaf: bool,
    pub expanded: bool,
}

/// State for a hierarchical tree view widget.
#[derive(Debug, Clone)]
pub struct TreeState {
    /// Root nodes of the tree.
    pub nodes: Vec<TreeNode>,
    /// Selected row index in the flattened visible tree.
    pub selected: usize,
}

impl TreeState {
    /// Create tree state from root nodes.
    pub fn new(nodes: Vec<TreeNode>) -> Self {
        Self { nodes, selected: 0 }
    }

    pub(crate) fn flatten(&self) -> Vec<FlatTreeEntry> {
        let mut entries = Vec::new();
        for node in &self.nodes {
            node.flatten(0, &mut entries);
        }
        entries
    }

    pub(crate) fn toggle_at(&mut self, flat_index: usize) {
        let mut counter = 0usize;
        Self::toggle_recursive(&mut self.nodes, flat_index, &mut counter);
    }

    fn toggle_recursive(nodes: &mut [TreeNode], target: usize, counter: &mut usize) -> bool {
        for node in nodes.iter_mut() {
            if *counter == target {
                if !node.is_leaf() {
                    node.expanded = !node.expanded;
                }
                return true;
            }
            *counter += 1;
            if node.expanded && Self::toggle_recursive(&mut node.children, target, counter) {
                return true;
            }
        }
        false
    }
}

/// State for the directory tree widget.
#[derive(Debug, Clone)]
pub struct DirectoryTreeState {
    /// The underlying tree state (reuses existing TreeState).
    pub tree: TreeState,
    /// Whether to show file/folder icons.
    pub show_icons: bool,
}

impl DirectoryTreeState {
    /// Create directory tree state from root nodes.
    pub fn new(nodes: Vec<TreeNode>) -> Self {
        Self {
            tree: TreeState::new(nodes),
            show_icons: true,
        }
    }

    /// Build a directory tree from slash-delimited paths.
    pub fn from_paths(paths: &[&str]) -> Self {
        let mut roots: Vec<TreeNode> = Vec::new();

        for raw_path in paths {
            let parts: Vec<&str> = raw_path
                .split('/')
                .filter(|part| !part.is_empty())
                .collect();
            if parts.is_empty() {
                continue;
            }
            insert_path(&mut roots, &parts, 0);
        }

        Self::new(roots)
    }

    /// Return selected node label if a node is selected.
    pub fn selected_label(&self) -> Option<&str> {
        let mut cursor = 0usize;
        selected_label_in_nodes(&self.tree.nodes, self.tree.selected, &mut cursor)
    }
}

impl Default for DirectoryTreeState {
    fn default() -> Self {
        Self::new(Vec::<TreeNode>::new())
    }
}

fn insert_path(nodes: &mut Vec<TreeNode>, parts: &[&str], depth: usize) {
    let Some(label) = parts.get(depth) else {
        return;
    };

    let is_last = depth + 1 == parts.len();
    let idx = nodes
        .iter()
        .position(|node| node.label == *label)
        .unwrap_or_else(|| {
            let mut node = TreeNode::new(*label);
            if !is_last {
                node.expanded = true;
            }
            nodes.push(node);
            nodes.len() - 1
        });

    if is_last {
        return;
    }

    nodes[idx].expanded = true;
    insert_path(&mut nodes[idx].children, parts, depth + 1);
}

fn selected_label_in_nodes<'a>(
    nodes: &'a [TreeNode],
    target: usize,
    cursor: &mut usize,
) -> Option<&'a str> {
    for node in nodes {
        if *cursor == target {
            return Some(node.label.as_str());
        }
        *cursor += 1;
        if node.expanded {
            if let Some(found) = selected_label_in_nodes(&node.children, target, cursor) {
                return Some(found);
            }
        }
    }
    None
}

// ── Command Palette ───────────────────────────────────────────────────

/// A single command entry in the palette.
#[derive(Debug, Clone)]
pub struct PaletteCommand {
    /// Primary command label.
    pub label: String,
    /// Supplemental command description.
    pub description: String,
    /// Optional keyboard shortcut hint.
    pub shortcut: Option<String>,
}

impl PaletteCommand {
    /// Create a new palette command.
    pub fn new(label: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: description.into(),
            shortcut: None,
        }
    }

    /// Set a shortcut hint displayed alongside the command.
    pub fn shortcut(mut self, s: impl Into<String>) -> Self {
        self.shortcut = Some(s.into());
        self
    }
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
        if self.mode == PickerMode::Hex {
            if let Some(c) = parse_hex_color(&self.hex_input.value) {
                return c;
            }
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
    let s = input.trim();
    let hex = s.strip_prefix('#')?;
    let (r, g, b) = match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            (r, g, b)
        }
        3 => {
            // #RGB expands each nibble to a byte (e.g. `f` -> `0xff`).
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 0x11;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 0x11;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 0x11;
            (r, g, b)
        }
        _ => return None,
    };
    Some(crate::Color::Rgb(r, g, b))
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
