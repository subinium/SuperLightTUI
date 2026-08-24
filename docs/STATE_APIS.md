# State APIs

Maintained reference for the public `*State` structs and their most useful
companions. `docs/WIDGETS.md` gives the quick catalog; this document separates
public fields from accessor and mutation methods. docs.rs and `src/widgets/`
remain authoritative for the complete generated API surface.

Every item below is declared `pub` in `src/widgets/*.rs`. Private helpers
and `pub(crate)` methods are intentionally omitted because they are not
part of the stable surface.

## Conventions

- Each type has H2 header with source file reference.
- "Fields" lists only public fields. Internal `pub(crate)` or private
  fields (cursors, dirty flags, caches) are omitted.
- "Constructors" covers `new`, `with_*`, `from_*`, `default`.
- "Methods" covers all other public methods on the type.
- `Self` in signatures means the enclosing type.

## TextInputState

Source: `src/widgets/input.rs`.

Single-line text input state. Pass `&mut TextInputState` to
`Context::text_input` each frame.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `value` | `String` | Current input text. |
| `cursor` | `usize` | Cursor position as a char index into `value`. |
| `placeholder` | `String` | Placeholder shown when `value` is empty. |
| `max_length` | `Option<usize>` | Reject input past this char count. |
| `validation_error` | `Option<String>` | Latest single-shot validation error. |
| `masked` | `bool` | Render `•` instead of characters (passwords). |
| `suggestions` | `Vec<String>` | Autocomplete candidates. |
| `suggestion_index` | `usize` | Highlighted suggestion row. |
| `show_suggestions` | `bool` | Whether suggestions popup is visible. |

### Constructors

- `TextInputState::new() -> Self`
- `TextInputState::with_placeholder(p: impl Into<String>) -> Self`
- `TextInputState::default() -> Self` (same as `new`)

### Methods

| Signature | Description |
|-----------|-------------|
| `max_length(self, len: usize) -> Self` | Builder: cap character count. |
| `validate(&mut self, validator: impl Fn(&str) -> Result<(), String>)` | Run one validator and store its error into `validation_error`. |
| `add_validator(&mut self, f: impl Fn(&str) -> Result<(), String> + 'static)` | Register a boxed validator. |
| `run_validators(&mut self)` | Run all validators; populates `errors()` and `validation_error`. |
| `errors(&self) -> &[String]` | All current validator errors. |
| `set_suggestions(&mut self, suggestions: Vec<String>)` | Replace suggestions and reset popup state. |
| `matched_suggestions(&self) -> Vec<&str>` | Suggestions whose prefix matches `value` (case-insensitive). |

> **`Clone` note**: `TextInputState::clone()` produces a copy *without* registered
> validators. Validators are stored as `Box<dyn Fn>`, which is not `Clone`, so
> the cloned state starts with an empty validator list. This is intentional —
> re-register validators on the clone if you need them. Inherent fields
> (`value`, `cursor`, `placeholder`, `max_length`, `validation_error`,
> `masked`, `suggestions`, `suggestion_index`, `show_suggestions`) are copied
> as expected.

### Minimal example

```rust
let mut input = TextInputState::with_placeholder("email");
input.add_validator(|v| {
    if v.contains('@') { Ok(()) } else { Err("needs @".into()) }
});
ui.text_input(&mut input);
input.run_validators();
for err in input.errors() { ui.text(err).fg(Color::Red); }
```

## TextareaState

Source: `src/widgets/input.rs`.

Multi-line text area state. Pass with a row count to `Context::textarea`.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `lines` | `Vec<String>` | One entry per logical line. |
| `cursor_row` | `usize` | Logical row of the cursor. |
| `cursor_col` | `usize` | Character column within the row. |
| `max_length` | `Option<usize>` | Max total character count. |
| `wrap_width` | `Option<u32>` | Soft-wrap column; `None` disables wrap. |
| `scroll_offset` | `usize` | First visible visual line (managed by `textarea()`). |

### Constructors

- `TextareaState::new() -> Self`
- `TextareaState::default() -> Self` (single blank line)

### Methods

| Signature | Description |
|-----------|-------------|
| `value(&self) -> String` | Join `lines` with `\n`. |
| `set_value(&mut self, text: impl Into<String>)` | Replace content, reset cursor and scroll. |
| `max_length(self, len: usize) -> Self` | Builder: cap total char count. |
| `word_wrap(self, width: u32) -> Self` | Builder: enable soft word-wrap. |

### Minimal example

```rust
let mut area = TextareaState::new().word_wrap(60).max_length(2000);
ui.textarea(&mut area, 8);
if ui.button("Save").clicked { save(&area.value()); }
```

## FormField

Source: `src/widgets/input.rs`.

One labeled input inside a `FormState`.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `label` | `String` | Label shown above the input. |
| `input` | `TextInputState` | Backing input state. |
| `error` | `Option<String>` | Error rendered below the input. |

### Constructors

- `FormField::new(label: impl Into<String>) -> Self`
- `FormField::default() -> Self`

### Methods

| Signature | Description |
|-----------|-------------|
| `placeholder(self, p: impl Into<String>) -> Self` | Builder: set the nested input's placeholder. |

### Minimal example

```rust
let email = FormField::new("Email").placeholder("you@example.com");
```

## FormState

Source: `src/widgets/input.rs`.

Multi-field form.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `fields` | `Vec<FormField>` | Ordered field list. |
| `submitted` | `bool` | Whether the form has been submitted successfully. |

### Constructors

- `FormState::new() -> Self`
- `FormState::default() -> Self`

### Methods

| Signature | Description |
|-----------|-------------|
| `field(self, field: FormField) -> Self` | Builder: append a field. |
| `validate(&mut self, validators: &[FormValidator]) -> bool` | Run per-field validators; returns `true` when every field passes. |
| `value(&self, index: usize) -> &str` | Read the Nth field's input value, `""` if out of range. |

### Minimal example

```rust
let mut form = FormState::new()
    .field(FormField::new("Name"))
    .field(FormField::new("Email"));
if form.validate(&validators) { form.submitted = true; }
```

## StaticOutput

Source: `src/widgets/input.rs`.

Buffer of static lines emitted above an inline TUI via `slt::run_static`.

### Fields

All fields are private.

### Constructors

- `StaticOutput::new() -> Self`
- `StaticOutput::default() -> Self`

### Methods

| Signature | Description |
|-----------|-------------|
| `println(&mut self, line: impl Into<String>)` | Append one line. |
| `lines(&self) -> &[String]` | All accumulated lines. |
| `drain_new(&mut self) -> Vec<String>` | Drain lines added since the previous drain. |
| `clear(&mut self)` | Remove every line. |

### Minimal example

```rust
let mut out = StaticOutput::new();
out.println("Saved record 123");
```

## ListState

Source: `src/widgets/collections.rs`.

Selectable list.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `selected` | `usize` | Current selection. |

Items and filter state are private because they are coupled to search and
viewport caches. Read or mutate them through the methods below.

### Constructors

- `ListState::new(items: Vec<impl Into<String>>) -> Self`
- `ListState::default() -> Self`

### Methods

| Signature | Description |
|-----------|-------------|
| `set_items(&mut self, items: Vec<impl Into<String>>)` | Replace items and rebuild filtered view. |
| `items(&self) -> &[String]` | Read all underlying items. |
| `len(&self) -> usize` / `is_empty(&self) -> bool` | Inspect the underlying collection size. |
| `item(&self, index: usize) -> Option<&str>` | Checked access by data index. |
| `push_item(...)`, `insert_item(...)`, `remove_item(...)`, `clear_items()` | Mutate items while rebuilding cache-coupled state. |
| `filter(&self) -> &str` | Read the active filter. |
| `set_filter(&mut self, filter: impl Into<String>)` | Update filter and recompute visible indices. |
| `visible_indices(&self) -> &[usize]` | Indices into `items` that survive the filter. |
| `selected_item(&self) -> Option<&str>` | Text of the currently selected item. |
| `with_item_heights(...)`, `set_item_heights(...)`, `clear_item_heights()` | Configure variable-height virtualization. |
| `move_item(&mut self, from: usize, to: usize) -> bool` | Reorder an item while preserving cache invariants. |

### Minimal example

```rust
let mut list = ListState::new(vec!["apple", "banana", "cherry"]);
list.set_filter("an");
ui.list(&mut list);
if let Some(item) = list.selected_item() { println!("{item}"); }
```

## FilePickerState

Source: `src/widgets/collections.rs`.

Directory browser with extension and hidden-file filters.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `current_dir` | `PathBuf` | Directory currently being browsed. |
| `entries` | `Vec<FileEntry>` | Visible entries. |
| `selected` | `usize` | Index into `entries`. |
| `selected_file` | `Option<PathBuf>` | The file the user confirmed. |
| `show_hidden` | `bool` | Include dotfiles. |
| `extensions` | `Vec<String>` | Allowed extensions (lowercase, no dot). |
| `dirty` | `bool` | Set when `refresh()` should be called. |

### Constructors

- `FilePickerState::new(dir: impl Into<PathBuf>) -> Self`
- `FilePickerState::default() -> Self` (current directory)

### Methods

| Signature | Description |
|-----------|-------------|
| `show_hidden(self, show: bool) -> Self` | Builder: include dotfiles. |
| `extensions(self, exts: &[&str]) -> Self` | Builder: restrict to given extensions. |
| `selected_file(&self) -> Option<&PathBuf>` | The chosen file, if any. |
| `refresh(&mut self)` | Re-read the directory listing and retain errors in state. |
| `try_refresh(&mut self) -> Result<(), FilePickerScanError>` | Fallible refresh for immediate control flow. |
| `retry(&mut self)` | Mark the listing for another scan after a transient failure. |
| `scan_status(&self) -> FilePickerScanStatus` | Read listing freshness. |
| `scan_errors(&self) -> &[FilePickerScanError]` | Read all failures from the latest scan. |
| `last_error(&self) -> Option<&FilePickerScanError>` | Read the most recent scan failure. |

### Minimal example

```rust
let mut picker = FilePickerState::new(".").extensions(&["rs", "toml"]);
picker.refresh();
ui.file_picker(&mut picker);
```

## FileEntry

Source: `src/widgets/collections.rs`.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `name` | `String` | Base name. |
| `path` | `PathBuf` | Full path. |
| `is_dir` | `bool` | Directory flag. |
| `size` | `Option<u64>` | Size in bytes, or `None` for directories and unreadable metadata. |

### Constructors

- `FileEntry::default() -> Self`

### Methods

No additional methods beyond field access.

## TabsState

Source: `src/widgets/collections.rs`.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `labels` | `Vec<String>` | Tab labels. |
| `selected` | `usize` | Index of the active tab. |

### Constructors

- `TabsState::new(labels: Vec<impl Into<String>>) -> Self`
- `TabsState::default() -> Self`

### Methods

| Signature | Description |
|-----------|-------------|
| `selected_label(&self) -> Option<&str>` | Text of the active tab. |

### Minimal example

```rust
let mut tabs = TabsState::new(vec!["General", "Keys", "About"]);
ui.tabs(&mut tabs);
match tabs.selected_label() {
    Some("General") => ui.text("general pane"),
    Some("Keys") => ui.text("key bindings"),
    _ => ui.text("about pane"),
};
```

## TableState

Source: `src/widgets/collections.rs`.

Sortable, filterable, paginated table.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `selected` | `usize` | Selected row index (within the visible view). |
| `multi_selected` | `HashSet<usize>` | Multi-selected visible row indices. |
| `sort_column` | `Option<usize>` | Active sort column. |
| `sort_ascending` | `bool` | Sort direction. |
| `page` | `usize` | Current page (0-based). |
| `page_size` | `usize` | Rows per page; `0` disables pagination. |
| `zebra` | `bool` | Alternating row backgrounds. |

Headers, rows, filter text, and width caches are private. Use the methods below
so filtering, sorting, selection, and width caches stay synchronized.

### Constructors

- `TableState::new(headers: Vec<impl Into<String>>, rows: Vec<Vec<impl Into<String>>>) -> Self`
- `TableState::default() -> Self`

### Methods

| Signature | Description |
|-----------|-------------|
| `set_rows(&mut self, rows: Vec<Vec<impl Into<String>>>)` | Replace rows; clamps selection and rebuilds the filter cache. |
| `headers(&self) -> &[String]` / `set_headers(...)` | Read or replace column headers. |
| `rows(&self) -> &[Vec<String>]` / `row(index)` | Read all rows or one checked row. |
| `row_count(&self) -> usize` / `is_empty(&self) -> bool` | Inspect the underlying row collection. |
| `push_row(...)`, `insert_row(...)`, `remove_row(...)`, `clear_rows()` | Mutate rows while rebuilding cache-coupled state. |
| `filter(&self) -> &str` | Read the active filter. |
| `toggle_sort(&mut self, column: usize)` | Sort by column; toggles direction if already sorted. |
| `sort_by(&mut self, column: usize)` | Sort ascending by column. |
| `set_filter(&mut self, filter: impl Into<String>)` | Update filter; resets the page. |
| `clear_sort(&mut self)` | Clear any active sort. |
| `next_page(&mut self)` | Advance one page, clamped to last. |
| `prev_page(&mut self)` | Go back one page, clamped to 0. |
| `total_pages(&self) -> usize` | Pages given current filter and `page_size`. |
| `visible_indices(&self) -> &[usize]` | Row indices after filter + sort. |
| `selected_row(&self) -> Option<&[String]>` | The currently selected row. |
| `column_widths_spec(&mut self, specs: &[TableColumn])` | Configure per-column width policies. |
| `selected_rows(&self) -> Vec<&[String]>` | Resolve multi-selected rows in visible order. |
| `is_row_selected(&self, view_idx: usize) -> bool` | Check focused or multi-selected state. |
| `clear_selection(&mut self)` | Clear multi-selection. |

### Minimal example

```rust
let mut table = TableState::new(
    vec!["Name", "Score"],
    vec![vec!["Alice", "95"], vec!["Bob", "72"]],
);
table.sort_by(1);
ui.table(&mut table);
```

## ScrollState

Source: `src/widgets/collections.rs`.

Vertical scroll bookkeeping for `Context::scrollable`.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `offset` | `usize` | First visible row. |
| `offset_x` | `usize` | First visible horizontal column. |
| `dragging` | `bool` | Whether the scrollbar thumb is being dragged. |

### Constructors

- `ScrollState::new() -> Self`
- `ScrollState::default() -> Self`

### Methods

| Signature | Description |
|-----------|-------------|
| `can_scroll_up(&self) -> bool` | `offset > 0`. |
| `can_scroll_down(&self) -> bool` | Content extends past the viewport. |
| `content_height(&self) -> u32` | Total content rows. |
| `viewport_height(&self) -> u32` | Visible rows. |
| `progress_ratio(&self) -> f64` | Scroll progress in `[0.0, 1.0]`. |
| `scroll_up(&mut self, amount: usize)` | Clamp to 0. |
| `scroll_down(&mut self, amount: usize)` | Clamp to max offset. |
| `set_offset(&mut self, offset: usize)` | Set and clamp the vertical offset. |
| `can_scroll_left/right`, `content_width`, `viewport_width`, `progress_x` | Inspect horizontal scrolling. |
| `scroll_left/right`, `set_highlights`, `highlights`, `highlight_next/previous` | Horizontal and highlighted-range navigation. |

## SelectState

Source: `src/widgets/selection.rs`.

Dropdown.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `selected` | `usize` | Current option index. |
| `open` | `bool` | Whether the dropdown is expanded. |
| `placeholder` | `String` | Fallback label when `items` is empty. |
| `filter` | `String` | Live type-to-filter query while open. |

Option storage and the internal cursor are private. Use `items()` and
`set_items()` to preserve cursor and selection bounds.

### Constructors

- `SelectState::new(items: Vec<impl Into<String>>) -> Self`
- `SelectState::default() -> Self`

### Methods

| Signature | Description |
|-----------|-------------|
| `items(&self) -> &[String]` | Read all option labels. |
| `len(&self) -> usize` / `is_empty(&self) -> bool` | Inspect the option count. |
| `set_items(&mut self, items: Vec<impl Into<String>>)` | Replace options and clamp selection state. |
| `placeholder(self, p: impl Into<String>) -> Self` | Builder: set placeholder. |
| `selected_item(&self) -> Option<&str>` | Currently selected label. |

### Minimal example

```rust
let mut dropdown = SelectState::new(vec!["Low", "Medium", "High"])
    .placeholder("select priority");
ui.select(&mut dropdown);
```

## RadioState

Source: `src/widgets/selection.rs`.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `items` | `Vec<String>` | Option labels. |
| `selected` | `usize` | Current selection. |

### Constructors

- `RadioState::new(items: Vec<impl Into<String>>) -> Self`
- `RadioState::default() -> Self`

### Methods

| Signature | Description |
|-----------|-------------|
| `selected_item(&self) -> Option<&str>` | Current label. |

## MultiSelectState

Source: `src/widgets/selection.rs`.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `cursor` | `usize` | Keyboard focus cursor. |
| `selected` | `HashSet<usize>` | Checked indices. |

Option storage is private. Use `items()` and `set_items()` so stale selected
indices are pruned.

### Constructors

- `MultiSelectState::new(items: Vec<impl Into<String>>) -> Self`

### Methods

| Signature | Description |
|-----------|-------------|
| `items(&self) -> &[String]` | Read all option labels. |
| `len(&self) -> usize` / `is_empty(&self) -> bool` | Inspect the option count. |
| `set_items(&mut self, items: Vec<impl Into<String>>)` | Replace options and synchronize selection. |
| `selected_items(&self) -> Vec<&str>` | Selected labels in ascending-index order. |
| `toggle(&mut self, index: usize)` | Add or remove `index` from `selected`. |

### Minimal example

```rust
let mut ms = MultiSelectState::new(vec!["bold", "italic", "underline"]);
ms.toggle(0);
ui.multi_select(&mut ms);
```

## TreeNode

Source: `src/widgets/selection.rs`.

One node in a hierarchical tree.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `label` | `String` | Display label. |
| `children` | `Vec<TreeNode>` | Child nodes. |
| `expanded` | `bool` | Whether children are visible. |

### Constructors

- `TreeNode::new(label: impl Into<String>) -> Self`

### Methods

| Signature | Description |
|-----------|-------------|
| `expanded(self) -> Self` | Builder: mark as expanded. |
| `children(self, children: Vec<TreeNode>) -> Self` | Builder: set children. |
| `is_leaf(&self) -> bool` | `true` when there are no children. |

## TreeState

Source: `src/widgets/selection.rs`.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `nodes` | `Vec<TreeNode>` | Root nodes. |
| `selected` | `usize` | Row index in the flattened visible tree. |

### Constructors

- `TreeState::new(nodes: Vec<TreeNode>) -> Self`

### Methods

The user-facing tree API is built on the public fields plus `TreeNode`
helpers above; `TreeState` itself exposes only `new`. Internal expand
and flatten helpers are `pub(crate)` and called by `Context::tree`.

### Minimal example

```rust
let root = TreeNode::new("project").expanded().children(vec![
    TreeNode::new("src").children(vec![TreeNode::new("main.rs")]),
    TreeNode::new("Cargo.toml"),
]);
let mut tree = TreeState::new(vec![root]);
ui.tree(&mut tree);
```

## DirectoryTreeState

Source: `src/widgets/selection.rs`.

Directory-flavored tree with icon toggle.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `tree` | `TreeState` | Underlying tree state. |
| `show_icons` | `bool` | Whether to render file icons. |

### Constructors

- `DirectoryTreeState::new(nodes: Vec<TreeNode>) -> Self`
- `DirectoryTreeState::from_paths(paths: &[&str]) -> Self`
- `DirectoryTreeState::default() -> Self` (empty)

### Methods

| Signature | Description |
|-----------|-------------|
| `selected_label(&self) -> Option<&str>` | Label of the currently selected node. |

### Minimal example

```rust
let mut dt = DirectoryTreeState::from_paths(&[
    "src/main.rs", "src/lib.rs", "Cargo.toml",
]);
ui.directory_tree(&mut dt);
```

## PaletteCommand

Source: `src/widgets/selection.rs`.

One entry in a command palette.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `label` | `String` | Primary label. |
| `description` | `String` | Supplemental description. |
| `shortcut` | `Option<String>` | Keyboard hint text. |

### Constructors

- `PaletteCommand::new(label: impl Into<String>, description: impl Into<String>) -> Self`

### Methods

| Signature | Description |
|-----------|-------------|
| `shortcut(self, s: impl Into<String>) -> Self` | Builder: attach a shortcut hint. |

## CommandPaletteState

Source: `src/widgets/commanding.rs`.

Modal palette with fuzzy search.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `input` | `String` | Current search query. |
| `cursor` | `usize` | Cursor into `input`. |
| `open` | `bool` | Whether the modal is visible. |
| `last_selected` | `Option<usize>` | Last confirmed command index. Consume after `response.changed`. |

Command storage and its filtered-selection cache are private. Mutate commands
through the synchronization methods below.

### Constructors

- `CommandPaletteState::new(commands: Vec<PaletteCommand>) -> Self`

### Methods

| Signature | Description |
|-----------|-------------|
| `commands(&self) -> &[PaletteCommand]` | Read all commands. |
| `set_commands(&mut self, commands: Vec<PaletteCommand>)` | Replace commands and synchronize cached state. |
| `push_command(...)`, `remove_command(...)`, `clear_commands()` | Mutate commands while invalidating filter state. |
| `toggle(&mut self)` | Open or close; resets input when opening. |

### Minimal example

```rust
let mut palette = CommandPaletteState::new(vec![
    PaletteCommand::new("Open File", "Pick a file").shortcut("Ctrl+O"),
]);
if ui.key_mod('p', KeyModifiers::CONTROL) { palette.toggle(); }
let r = ui.command_palette(&mut palette);
if r.changed { if let Some(i) = palette.last_selected { run(i); } }
```

## StreamingTextState

Source: `src/widgets/commanding.rs`.

Plain-text streaming buffer.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `content` | `String` | Accumulated text. |
| `streaming` | `bool` | Whether the stream is still active. |

### Constructors

- `StreamingTextState::new() -> Self`
- `StreamingTextState::default() -> Self`

### Methods

| Signature | Description |
|-----------|-------------|
| `push(&mut self, chunk: &str)` | Append a chunk. |
| `finish(&mut self)` | Mark the stream complete. |
| `start(&mut self)` | Begin a fresh session, clearing previous content. |
| `clear(&mut self)` | Wipe everything. |

## StreamingMarkdownState

Source: `src/widgets/commanding.rs`.

Streaming markdown buffer with fenced-code awareness.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `content` | `String` | Accumulated markdown. |
| `streaming` | `bool` | Stream active flag. |
| `cursor_visible` | `bool` | Blink state for the typing indicator. |
| `cursor_tick` | `u64` | Cursor animation tick. |
| `in_code_block` | `bool` | Whether the parser is inside a fenced block. |
| `code_block_lang` | `String` | Language label of the active fenced block. |

### Constructors

- `StreamingMarkdownState::new() -> Self`
- `StreamingMarkdownState::default() -> Self`

### Methods

| Signature | Description |
|-----------|-------------|
| `push(&mut self, chunk: &str)` | Append a chunk. |
| `start(&mut self)` | Begin a fresh session. |
| `finish(&mut self)` | Mark complete; hides the typing cursor. |
| `clear(&mut self)` | Reset state. |

## ScreenState

Source: `src/widgets/commanding.rs`.

Navigation stack for multi-screen apps. Used with `Context::screen`.

### Fields

All fields are private. Use the methods below to read and mutate state.

### Constructors

- `ScreenState::new(initial: impl Into<String>) -> Self`

### Methods

| Signature | Description |
|-----------|-------------|
| `current(&self) -> &str` | Top-of-stack screen name. |
| `push(&mut self, name: impl Into<String>)` | Navigate forward. |
| `pop(&mut self)` | Navigate back; preserves the root screen. |
| `depth(&self) -> usize` | Stack size. |
| `can_pop(&self) -> bool` | `true` when more than one screen is pushed. |
| `reset(&mut self)` | Truncate to the root screen. |

### Minimal example

```rust
let mut screens = ScreenState::new("home");
if ui.button("Settings").clicked { screens.push("settings"); }
if screens.current() == "settings" && ui.button("Back").clicked {
    screens.pop();
}
```

## ModeState

Source: `src/widgets/commanding.rs`.

Named modes, each containing an independent `ScreenState`.

### Fields

All fields are private.

### Constructors

- `ModeState::new(mode: impl Into<String>, screen: impl Into<String>) -> Self`

### Methods

| Signature | Description |
|-----------|-------------|
| `add_mode(&mut self, mode: impl Into<String>, screen: impl Into<String>)` | Register a mode with its root screen. |
| `switch_mode(&mut self, mode: impl Into<String>)` | Switch; panics if the mode is unknown. |
| `try_switch_mode(&mut self, mode: impl Into<String>) -> bool` | Non-panicking variant that returns `false` for unknown modes. |
| `active_mode(&self) -> &str` | Current mode name. |
| `screens(&self) -> &ScreenState` | Immutable view of the active mode's screen stack. |
| `screens_mut(&mut self) -> &mut ScreenState` | Mutable view of the active mode's screen stack. |

### Minimal example

```rust
let mut modes = ModeState::new("app", "home");
modes.add_mode("settings", "general");
if ui.key('2') { modes.try_switch_mode("settings"); }
modes.screens_mut().push("advanced");
```

## ToolApprovalState

Source: `src/widgets/commanding.rs`.

Human-in-the-loop approval prompt for an AI tool call.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `tool_name` | `String` | Name of the tool. |
| `description` | `String` | What the tool will do. |
| `action` | `ApprovalAction` | Current status. |

### Constructors

- `ToolApprovalState::new(tool_name: impl Into<String>, description: impl Into<String>) -> Self`

### Methods

| Signature | Description |
|-----------|-------------|
| `reset(&mut self)` | Set `action` back to `Pending`. |

### Related enum

`ApprovalAction` has variants `Pending`, `Approved`, `Rejected`.

## ContextItem

Source: `src/widgets/commanding.rs`.

Entry in a context bar for AI chat UIs.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `label` | `String` | Display label. |
| `tokens` | `usize` | Token count or size indicator. |

### Constructors

- `ContextItem::new(label: impl Into<String>, tokens: usize) -> Self`

### Methods

None beyond the constructor.

## RichLogState

Source: `src/widgets/feedback.rs`.

Multi-style append-only log.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `auto_scroll` | `bool` | Follow the tail when new rows arrive. |
| `max_entries` | `Option<usize>` | Optional deque retention cap (`None` = unbounded). |

### Constructors

- `RichLogState::new() -> Self` — bounded at `RichLogState::DEFAULT_MAX_ENTRIES`
  (10,000 entries). Older entries are dropped from the front when the cap is
  exceeded. **Recommended default for long-running apps** to prevent unbounded
  memory growth.
- `RichLogState::new_unbounded() -> Self` — opt-in unbounded log
  (`max_entries = None`). Use only when the host explicitly bounds growth
  elsewhere (e.g. you call `clear()` periodically or cap entries upstream).
- `RichLogState::default() -> Self` — same as `new()` (bounded at 10,000).

### Associated constants

- `RichLogState::DEFAULT_MAX_ENTRIES: usize = 10_000`

### Methods

| Signature | Description |
|-----------|-------------|
| `push(&mut self, text: impl Into<String>, style: Style)` | Append one styled entry. |
| `push_plain(&mut self, text: impl Into<String>)` | Append with default style. |
| `push_segments(&mut self, segments: Vec<(String, Style)>)` | Append a mixed-style entry; honors `max_entries` and `auto_scroll`. |
| `push_entry(&mut self, entry: RichLogEntry)` | Append a pre-built entry while enforcing retention. |
| `clear(&mut self)` | Remove everything and reset scroll. |
| `len(&self) -> usize` | Entry count. |
| `is_empty(&self) -> bool` | `true` when there are no entries. |
| `entries(&self)` / `entries_mut(&mut self)` | Iterate oldest-to-newest without exposing storage. |
| `entry(&self, index)` / `entry_mut(&mut self, index)` | Checked indexed access. |

### Related type

`RichLogEntry { pub segments: Vec<(String, Style)> }`.

## CalendarState

Source: `src/widgets/feedback.rs`.

Date picker.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `year` | `i32` | Displayed year. |
| `month` | `u32` | Displayed month (1–12). |
| `selected_day` | `Option<u32>` | Currently selected day. |

### Constructors

- `CalendarState::new() -> Self` (current month)
- `CalendarState::from_ym(year: i32, month: u32) -> Self`
- `CalendarState::default() -> Self`

### Methods

| Signature | Description |
|-----------|-------------|
| `with_range(&mut self) -> &mut Self` | Switch to date-range selection. |
| `with_time(&mut self) -> &mut Self` | Enable time selection. |
| `mode(&self) -> CalendarSelect` | Read the active selection mode. |
| `selected_date(&self) -> Option<(i32, u32, u32)>` | Returns `(year, month, day)` when a day is selected. |
| `selected_range(&self) -> Option<(CalDate, CalDate)>` | Read the normalized selected range. |
| `selected_time(&self) -> Option<(u8, u8)>` | Read the selected hour and minute. |
| `prev_month(&mut self)` | Move view back one month. |
| `next_month(&mut self)` | Move view forward one month. |

### Minimal example

```rust
let mut cal = CalendarState::new();
ui.calendar(&mut cal);
if let Some((y, m, d)) = cal.selected_date() { println!("{y}-{m:02}-{d:02}"); }
```

## SpinnerState

Source: `src/widgets/input.rs`.

Tick-driven spinner.

### Fields

Internal character set only.

### Constructors

- `SpinnerState::dots() -> Self` — braille dots
- `SpinnerState::line() -> Self` — ASCII bar
- `SpinnerState::default() -> Self` — equivalent to `dots`

### Methods

| Signature | Description |
|-----------|-------------|
| `frame(&self, tick: u64) -> char` | Character for the given frame tick. |

### Minimal example

```rust
let spinner = SpinnerState::dots();
let ch = spinner.frame(ui.tick());
ui.text(format!("{ch} Loading..."));
```

## ToastState

Source: `src/widgets/input.rs`.

Stack of timed toast notifications. Pass to `Context::toast` each frame.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `messages` | `Vec<ToastMessage>` | Active messages, oldest first. |

### Constructors

- `ToastState::new() -> Self`
- `ToastState::default() -> Self`

### Methods

| Signature | Description |
|-----------|-------------|
| `info(&mut self, text: impl Into<String>, tick: u64)` | Push an info toast for 30 ticks. |
| `success(&mut self, text: impl Into<String>, tick: u64)` | Push a success toast for 30 ticks. |
| `warning(&mut self, text: impl Into<String>, tick: u64)` | Push a warning toast for 50 ticks. |
| `error(&mut self, text: impl Into<String>, tick: u64)` | Push an error toast for 80 ticks. |
| `push(&mut self, text: impl Into<String>, level: ToastLevel, tick: u64, duration_ticks: u64)` | Push a toast with a custom level and duration. |
| `cleanup(&mut self, current_tick: u64)` | Drop expired messages. Called automatically by `Context::toast`. |

### Related types

- `ToastMessage { pub text, level, created_tick, duration_ticks }` — impls `Default`.
- `ToastLevel` with variants `Info`, `Success`, `Warning`, `Error`.
- `AlertLevel` with variants `Info`, `Success`, `Warning`, `Error` (non-exhaustive). Used by alert widgets rather than toasts.

### Minimal example

```rust
let mut toasts = ToastState::new();
if ui.key('s') { toasts.success("Saved", ui.tick()); }
ui.toast(&mut toasts);
```

## ButtonVariant

Source: `src/widgets/feedback.rs`.

Visual variant accepted by `Context::button_with`.

### Variants

- `Default` — theme text color, primary when focused (same as `button()`).
- `Primary` — primary color background with contrasting text.
- `Danger` — error color for destructive actions.
- `Outline` — bordered appearance without fill.

`#[non_exhaustive]` — do not `match` without a wildcard arm. Derives
`Default` (to `Default`), `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`,
`Debug`.

## Trend

Source: `src/widgets/feedback.rs`.

### Variants

- `Up` — positive movement.
- `Down` — negative movement.

`#[non_exhaustive]`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Debug`.

## NumberInputState

Source: `src/widgets/input.rs`.

Editable numeric stepper with normalized bounds.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `value` | `f64` | Committed value. |
| `min` / `max` | `f64` | Inclusive normalized range. |
| `step` | `f64` | Per-action increment. |
| `integer` | `bool` | Round and render as a whole number. |
| `editing` | `Option<String>` | In-progress typed buffer. |
| `parse_error` | `Option<String>` | Latest typed-value parse error. |

### Constructors and methods

- `NumberInputState::new(value: f64, min: f64, max: f64) -> Self`
- `NumberInputState::integer(value: i64, min: i64, max: i64) -> Self`
- `step(self, step: f64) -> Self`
- `normalize(&mut self)` after direct public-field mutation
- `clamped(&self) -> f64`

## PaginatorState

Source: `src/widgets/collections.rs`.

Standalone page navigation for arbitrary data.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `total_items` | `usize` | Total item count. |
| `per_page` | `usize` | Items per page, normalized to at least 1 by methods. |
| `page` | `usize` | Current zero-based page. |
| `style` | `PaginatorStyle` | `Dots` or compact `Arabic` rendering. |

### Constructors and methods

- `PaginatorState::new(total_items: usize, per_page: usize) -> Self`
- `total_pages(&self) -> usize` and `page_bounds(&self) -> (usize, usize)`
- `next_page(&mut self)`, `prev_page(&mut self)`, `set_page(&mut self, page: usize)`
- `set_total_items(&mut self, total: usize)` and `set_per_page(&mut self, per_page: usize)`

## SplitPaneState

Source: `src/widgets/collections.rs`.

Shared state for `Context::split_pane` and `Context::vsplit_pane`.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `ratio` | `f64` | Fraction assigned to the first pane. |
| `dragging` | `bool` | Whether the divider is being dragged. |
| `min_ratio` | `f64` | Minimum fraction assigned to either pane. |

### Constructors and methods

- `SplitPaneState::new(ratio: f64) -> Self`
- `SplitPaneState::default() -> Self` (ratio 0.5)
- `with_min_ratio(self, min: f64) -> Self`
- `set_ratio(&mut self, ratio: f64)`

## ColorPickerState

Source: `src/widgets/selection.rs`.

Palette-grid and hex-entry color selection.

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `colors` | `Vec<Color>` | Row-major palette swatches. |
| `columns` | `usize` | Swatches per row. |
| `selected` | `usize` | Selected palette index. |
| `mode` | `PickerMode` | `Palette` or `Hex`. |
| `hex_input` | `TextInputState` | Backing field for hex mode. |

### Constructors and methods

- `ColorPickerState::new(colors: Vec<Color>) -> Self`
- `ColorPickerState::tailwind() -> Self`
- `columns(self, n: usize) -> Self`
- `selected(&self) -> Color`

## ChordState

Source: `src/widgets/commanding.rs`.

All fields are private and there are no user-facing methods. `Context` owns the
default-constructed state across frames; callers use `Context::key_chord` and
`Context::key_chord_timeout` rather than constructing or mutating it directly.

## SchedulerState

Source: `src/widgets/feedback.rs`.

All fields and methods are crate-internal. `Context` owns the default-constructed
timer and exclusive-claim state across frames; callers use `Context::schedule`,
`Context::every`, their keyed variants, and `Context::exclusive`.

## Coverage contract

Public method counts are intentionally not copied into this guide: they drift
as soon as a state type gains a method. Rustdoc is the authoritative inventory,
and repository contract tests compile maintained snippets and compare advertised
commands with Cargo metadata. A public method missing from this guide remains a
documentation bug, but a stale hand-count cannot hide it.

## See also

- `docs/WIDGETS.md` — field-level reference
- `docs/PATTERNS.md` — how to combine state types with container layout
- `docs/AI_GUIDE.md` — general rules for AI agents
- `docs/PREVIOUS_FRAME_GUIDE.md` — when `Response.rect` is valid
