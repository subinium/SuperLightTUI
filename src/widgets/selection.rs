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
