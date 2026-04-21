# SLT Cookbook

Copy-paste recipes for common TUI apps. Each recipe is also shipped as a standalone runnable example at `examples/cookbook_<name>.rs`, so you can run and tweak it directly.

Recipes assume familiarity with the core mental model from [QUICK_START.md](QUICK_START.md). If a widget surprises you, cross-check [WIDGETS.md](WIDGETS.md) for the full API table and [PATTERNS.md](PATTERNS.md) for composition advice.

## Index

- [Login Form with Validation](#login-form-with-validation) — `cargo run --example cookbook_login`
- [Data Table with Search and Sort](#data-table-with-search-and-sort) — `cargo run --example cookbook_table`
- [Modal Confirmation with Toast](#modal-confirmation-with-toast) — `cargo run --example cookbook_modal_toast`
- [Real-time Dashboard with Charts](#real-time-dashboard-with-charts) — `cargo run --example cookbook_dashboard`
- [File Picker with Preview](#file-picker-with-preview) — `cargo run --example cookbook_file_picker`

## Prerequisites

Add SLT to a fresh project:

```toml
# Cargo.toml
[dependencies]
superlighttui = "0.18"
```

Every recipe follows the same outer shape:

```rust
fn main() -> std::io::Result<()> {
    // 1. Create state once, outside the closure.
    let mut state = /* ... */;

    // 2. Your closure runs each frame.
    slt::run(|ui: &mut slt::Context| {
        // Global shortcuts first.
        if ui.key('q') || ui.key_code(slt::KeyCode::Esc) {
            ui.quit();
        }

        // Describe your UI.
        ui.text("hello");
    })
}
```

The closure is the whole app. Plain Rust variables hold state; `slt::Context` mutates on interaction. No framework object, no stores.

---

## Login Form with Validation

### What it shows

- Two `TextInputState` fields with `add_validator(...)` chains that run every frame.
- Password masking via `TextInputState::masked`.
- Automatic Tab / Shift+Tab focus cycling — no manual focus handling required.
- Submit button that is only enabled when both fields pass validation.

### Full code

```rust
use slt::{AlertLevel, Border, Color, Context, KeyCode, TextInputState};

fn main() -> std::io::Result<()> {
    let mut email = TextInputState::with_placeholder("you@example.com");
    email.add_validator(|v| {
        if v.is_empty() {
            Err("Email is required".into())
        } else if !v.contains('@') || !v.contains('.') {
            Err("Invalid email format".into())
        } else {
            Ok(())
        }
    });

    let mut password = TextInputState::with_placeholder("At least 8 characters");
    password.masked = true;
    password.add_validator(|v| {
        if v.len() < 8 {
            Err("Password must be at least 8 characters".into())
        } else {
            Ok(())
        }
    });

    let mut submitted: Option<String> = None;

    slt::run(|ui: &mut Context| {
        if ui.key_mod('q', slt::KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }

        // Run validators every frame; cheap because inputs are tiny.
        email.run_validators();
        password.run_validators();
        let valid = email.errors().is_empty() && password.errors().is_empty();

        let _ = ui
            .bordered(Border::Rounded)
            .title("Sign In")
            .pad(2)
            .gap(1)
            .col(|ui| {
                ui.text("Welcome back").bold().fg(Color::Cyan);
                ui.text("Use Tab / Shift+Tab to move between fields.").dim();
                ui.separator();

                ui.text("Email").bold();
                let _ = ui.text_input(&mut email);
                for err in email.errors() {
                    ui.text(format!("  {err}")).fg(Color::Red);
                }

                ui.text("Password").bold();
                let _ = ui.text_input(&mut password);
                for err in password.errors() {
                    ui.text(format!("  {err}")).fg(Color::Red);
                }

                ui.separator();
                let _ = ui.row(|ui| {
                    // Disable the button when invalid by swapping the label.
                    let label = if valid { "Submit" } else { "Submit (fix errors)" };
                    if ui.button(label).clicked && valid {
                        submitted = Some(email.value.clone());
                    }
                    ui.spacer();
                    if let Some(ref user) = submitted {
                        let _ = ui.alert(
                            &format!("Signed in as {user}"),
                            AlertLevel::Success,
                        );
                    }
                });
            });
    })
}
```

### Key patterns

- `add_validator(|v| -> Result<(), String>)` is additive — call it once per rule. `run_validators()` populates `errors()`, which is the source of truth for the UI.
- Never guard `ui.text_input(...)` behind an `if`. Call it every frame; otherwise focus and key consumption state drifts.
- Focus cycling is handled by SLT. Register widgets in the order you want them traversed.
- The submit button stays rendered even when invalid — this keeps hook order stable. We gate the effect (`submitted = ...`) instead.

---

## Data Table with Search and Sort

### What it shows

- `TableState` with built-in search, pagination, and column sort.
- A `TextInputState` driving the filter each frame.
- Header-click sort (`toggle_sort`) via keyboard shortcuts `H` / `L`.
- Zebra rows + row-size footer with live row count.

### Full code

```rust
use slt::{Border, Color, Context, KeyCode, TableState, TextInputState};

fn main() -> std::io::Result<()> {
    let mut table = TableState::new(
        vec!["Name", "Role", "Stars"],
        vec![
            vec!["slt",       "TUI library",  "500"],
            vec!["ratatui",   "TUI library",  "12000"],
            vec!["bubbletea", "TUI framework", "30000"],
            vec!["ink",       "React for CLI", "28000"],
            vec!["textual",   "Python TUI",    "26000"],
            vec!["cursive",   "Rust TUI",      "4200"],
            vec!["termion",   "terminal lib",  "1900"],
            vec!["notcurses", "rendering lib", "2300"],
            vec!["blessed",   "JS TUI",        "10700"],
            vec!["prompt-kit","form lib",      "890"],
        ],
    );
    table.page_size = 5;
    table.zebra = true;

    let mut filter = TextInputState::with_placeholder("Filter across all columns...");

    slt::run(|ui: &mut Context| {
        if ui.key_mod('q', slt::KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }
        // Column-sort shortcuts (1..=3).
        for i in 1..=3u8 {
            if ui.key((b'0' + i) as char) {
                table.toggle_sort((i - 1) as usize);
            }
        }
        if ui.key_code(KeyCode::Right) {
            table.next_page();
        }
        if ui.key_code(KeyCode::Left) {
            table.prev_page();
        }

        let _ = ui
            .bordered(Border::Rounded)
            .title("Projects")
            .pad(1)
            .gap(1)
            .col(|ui| {
                ui.text("Press 1/2/3 to sort, Left/Right to page").dim();
                let _ = ui.text_input(&mut filter);
                // Pipe the text input straight into the table filter each frame.
                table.set_filter(&filter.value);

                let _ = ui.table(&mut table);

                let _ = ui.row(|ui| {
                    ui.text(format!(
                        "Showing {}/{} rows",
                        table.visible_indices().len(),
                        table.rows.len(),
                    ))
                    .dim();
                    ui.spacer();
                    if let Some(col) = table.sort_column {
                        let arrow = if table.sort_ascending { "asc" } else { "desc" };
                        ui.text(format!("Sort: {} {arrow}", table.headers[col]))
                            .fg(Color::Cyan);
                    }
                    ui.spacer();
                    ui.text(format!(
                        "page {}/{}",
                        table.page + 1,
                        table.total_pages()
                    ))
                    .dim();
                });

                if let Some(row) = table.selected_row() {
                    ui.separator();
                    ui.text(format!("Selected: {}", row.join(" | "))).bold();
                }
            });
    })
}
```

### Key patterns

- `TableState::new` accepts headers and string rows. `Stars` contains numbers inside strings — SLT's sort detects numeric columns automatically by trying `str::parse::<f64>()`.
- `set_filter` runs a cached, lowercase, token-AND search across all cells. Calling it with the same value is a no-op.
- `toggle_sort(col)` flips asc/desc on the second press; `sort_by(col)` always sets ascending.
- `page_size` of 0 disables pagination. When set, `visible_indices()` is still the full filtered set; SLT slices it internally for the current page.

---

## Modal Confirmation with Toast

### What it shows

- A destructive "Delete" action guarded by `ui.modal(...)`.
- `ToastState` for transient notifications with severity levels.
- `raw_key_code(KeyCode::Esc)` to close overlays regardless of modal state.
- Split logic: the button in the main UI opens the modal; the modal itself handles Confirm/Cancel.

### Full code

```rust
use slt::{Border, ButtonVariant, Color, Context, KeyCode, ToastState};

fn main() -> std::io::Result<()> {
    let mut items: Vec<String> = (1..=6).map(|i| format!("Document {i}.txt")).collect();
    let mut selected: usize = 0;
    let mut show_modal = false;
    let mut toasts = ToastState::new();

    slt::run(|ui: &mut Context| {
        let tick = ui.tick();

        if !show_modal && ui.key('q') {
            ui.quit();
        }
        if ui.key_code(KeyCode::Up) && selected > 0 {
            selected -= 1;
        }
        if ui.key_code(KeyCode::Down) && selected + 1 < items.len() {
            selected += 1;
        }

        let _ = ui
            .bordered(Border::Rounded)
            .title("Files")
            .pad(1)
            .gap(1)
            .col(|ui| {
                ui.text("Select a file, then press Delete.").dim();
                for (i, name) in items.iter().enumerate() {
                    let prefix = if i == selected { "> " } else { "  " };
                    let color = if i == selected { Color::Cyan } else { Color::White };
                    ui.text(format!("{prefix}{name}")).fg(color);
                }

                ui.separator();
                let _ = ui.row(|ui| {
                    if ui.button("Refresh").clicked {
                        toasts.info("List refreshed", tick);
                    }
                    ui.spacer();
                    if ui.button_with("Delete", ButtonVariant::Danger).clicked
                        && !items.is_empty()
                    {
                        show_modal = true;
                    }
                });
            });

        if show_modal {
            // Esc closes the modal no matter what is below it.
            if ui.raw_key_code(KeyCode::Esc) {
                show_modal = false;
            }

            let _ = ui.modal(|ui| {
                let _ = ui
                    .bordered(Border::Double)
                    .title("Confirm delete")
                    .pad(2)
                    .gap(1)
                    .col(|ui| {
                        let target = items
                            .get(selected)
                            .cloned()
                            .unwrap_or_else(|| "<nothing>".into());
                        ui.text("You are about to delete:").bold();
                        ui.text(format!("  {target}")).fg(Color::Yellow);
                        ui.text("This cannot be undone.").dim();

                        let _ = ui.row(|ui| {
                            if ui.button("Cancel").clicked {
                                show_modal = false;
                                toasts.info("Canceled", tick);
                            }
                            ui.spacer();
                            if ui
                                .button_with("Delete", ButtonVariant::Danger)
                                .clicked
                            {
                                let name = items.remove(selected);
                                if selected >= items.len() && selected > 0 {
                                    selected -= 1;
                                }
                                toasts.success(format!("Deleted {name}"), tick);
                                show_modal = false;
                            }
                        });
                    });
            });
        }

        // Toast rendering must happen every frame so `cleanup()` can expire old messages.
        ui.toast(&mut toasts);
    })
}
```

### Key patterns

- `ui.modal(|ui| { ... })` dims the background and traps focus inside the closure. The UI behind it still renders, but interaction routes to the modal.
- `ui.raw_key_code(...)` bypasses the modal's input consumption — ideal for global keys like Esc. Plain `ui.key_code(...)` inside the modal is for modal-local keys.
- `ToastState::info/success/warning/error` take `(message, tick)`. `ui.toast(&mut toasts)` both renders and cleans up expired entries — always call it unconditionally every frame.
- Gate global shortcuts (like `q`) behind `!show_modal` so they don't fire while the modal is open.

---

## Real-time Dashboard with Charts

### What it shows

- Simulated per-tick metrics feeding a `sparkline` and `bar_chart`.
- `ui.chart(|c| ...)` multi-series builder with a `line` and `area` overlay.
- `stat_trend` and `stat_colored` for KPI tiles.
- A ring buffer of recent values — the classic "rolling window" pattern for TUIs.

> Dashboards often want hover-driven highlight or drag-to-pan. Both use previous-frame layout data, so the first frame after layout change can look odd. See [DEBUGGING.md](DEBUGGING.md) for why — it is not a bug, it is immediate mode's trade-off.

### Full code

```rust
use slt::{Border, Color, Context, KeyCode, SpinnerState, Trend};

fn main() -> std::io::Result<()> {
    let spinner = SpinnerState::dots();
    let mut cpu_hist: Vec<f64> = vec![0.0; 60];
    let mut req_hist: Vec<f64> = vec![0.0; 60];

    slt::run(|ui: &mut Context| {
        if ui.key_mod('q', slt::KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }

        let tick = ui.tick() as f64;
        let cpu = 45.0 + 30.0 * (tick * 0.08).sin() + 10.0 * (tick * 0.3).cos();
        let reqs = (120.0 + 80.0 * (tick * 0.05).sin()).max(0.0);

        // Ring-buffer update: drop oldest, append newest.
        cpu_hist.rotate_left(1);
        if let Some(last) = cpu_hist.last_mut() {
            *last = cpu;
        }
        req_hist.rotate_left(1);
        if let Some(last) = req_hist.last_mut() {
            *last = reqs;
        }

        let _ = ui
            .bordered(Border::Rounded)
            .title("Live Dashboard")
            .pad(1)
            .gap(1)
            .grow(1)
            .col(|ui| {
                let _ = ui.row(|ui| {
                    ui.spinner(&spinner);
                    ui.text(" LIVE").bold().fg(Color::Green);
                    ui.spacer();
                    ui.text(format!("tick {tick:.0}")).dim();
                });

                let _ = ui.row(|ui| {
                    let _ = ui.bordered(Border::Single).pad(1).grow(1).col(|ui| {
                        let _ = ui.stat_trend("Requests / min", &format!("{reqs:.0}"), Trend::Up);
                    });
                    let _ = ui.bordered(Border::Single).pad(1).grow(1).col(|ui| {
                        let color = if cpu > 80.0 { Color::Red } else { Color::Cyan };
                        let _ = ui.stat_colored("CPU", &format!("{cpu:.1}%"), color);
                    });
                    let _ = ui.bordered(Border::Single).pad(1).grow(1).col(|ui| {
                        let _ = ui.stat_colored("P99", "42ms", Color::Yellow);
                    });
                });

                let _ = ui
                    .bordered(Border::Single)
                    .title("CPU (last 60 ticks)")
                    .pad(1)
                    .col(|ui| {
                        let _ = ui.sparkline(&cpu_hist, 60);
                    });

                let _ = ui
                    .bordered(Border::Single)
                    .title("Requests / min")
                    .pad(1)
                    .col(|ui| {
                        let _ = ui.chart(
                            |c| {
                                c.area(&req_hist).label("rpm").color(Color::Cyan);
                                c.grid(true);
                            },
                            60,
                            10,
                        );
                    });

                let top_routes: Vec<(&str, f64)> = vec![
                    ("/api/users", 420.0),
                    ("/api/items", 312.0),
                    ("/auth", 280.0),
                    ("/health", 95.0),
                    ("/metrics", 60.0),
                ];
                let _ = ui
                    .bordered(Border::Single)
                    .title("Top Routes")
                    .pad(1)
                    .col(|ui| {
                        let _ = ui.bar_chart(&top_routes, 30);
                    });

                let _ = ui.help(&[("Ctrl+Q", "quit"), ("Esc", "quit")]);
            });
    })
}
```

### Key patterns

- Store history in a fixed-size `Vec<f64>` and rotate it. This is cheaper than `VecDeque` for tiny buffers and keeps data contiguous for `sparkline(&history, width)`.
- `ui.chart(|c| { ... }, width, height)` takes an explicit size. For grow-to-fit behavior, read `ui.width()` / `ui.height()` and compute manually.
- `stat_trend` pairs a value with an up/down arrow via `Trend::Up` / `Trend::Down`. `stat_colored` lets you encode severity.
- Spinners animate off the frame tick automatically — just create one `SpinnerState::dots()` at startup and hand the reference in each frame.

---

## File Picker with Preview

### What it shows

- `FilePickerState` on the left, text preview on the right in a two-pane layout.
- Extension filtering (`.extensions(&["rs", "toml", "md"])`).
- Safe file reading with a size cap — avoids locking the UI on multi-MB files.
- `notify` toasts for error reporting without growing the main UI.

### Full code

```rust
use slt::{Border, Color, Context, FilePickerState, KeyCode, ScrollState, ToastState};
use std::path::PathBuf;

const PREVIEW_MAX_BYTES: u64 = 64 * 1024; // 64 KiB cap

fn main() -> std::io::Result<()> {
    let mut picker = FilePickerState::new(
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    )
    .extensions(&["rs", "toml", "md", "txt", "json"])
    .show_hidden(false);

    let mut preview: Option<(PathBuf, String)> = None;
    let mut preview_scroll = ScrollState::new();
    let mut toasts = ToastState::new();

    slt::run(|ui: &mut Context| {
        let tick = ui.tick();
        if ui.key_mod('q', slt::KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }

        let _ = ui
            .bordered(Border::Rounded)
            .title("File Picker")
            .pad(1)
            .gap(1)
            .grow(1)
            .col(|ui| {
                ui.text(format!("Dir: {}", picker.current_dir.display()))
                    .dim()
                    .wrap();
                ui.text("Enter opens, Backspace goes up, filter: .rs .toml .md .txt .json")
                    .dim();
                ui.separator();

                let _ = ui.row(|ui| {
                    // Left pane: picker.
                    let _ = ui
                        .bordered(Border::Single)
                        .title("Browse")
                        .pad(1)
                        .grow(1)
                        .col(|ui| {
                            if ui.file_picker(&mut picker).changed {
                                if let Some(path) = picker.selected().cloned() {
                                    match read_preview(&path) {
                                        Ok(content) => {
                                            preview = Some((path.clone(), content));
                                            preview_scroll.offset = 0;
                                            let name = path
                                                .file_name()
                                                .and_then(|s| s.to_str())
                                                .unwrap_or("?");
                                            toasts.success(format!("Loaded {name}"), tick);
                                        }
                                        Err(err) => {
                                            toasts.error(err, tick);
                                        }
                                    }
                                }
                            }
                        });

                    // Right pane: preview.
                    let _ = ui
                        .bordered(Border::Single)
                        .title("Preview")
                        .pad(1)
                        .grow(2)
                        .col(|ui| match &preview {
                            None => {
                                ui.text("Select a file to preview.").dim();
                            }
                            Some((path, content)) => {
                                ui.text(path.display().to_string()).fg(Color::Cyan).wrap();
                                ui.separator();
                                let _ = ui.scrollable(&mut preview_scroll).grow(1).col(|ui| {
                                    for line in content.lines() {
                                        ui.text(line);
                                    }
                                });
                            }
                        });
                });
            });

        ui.toast(&mut toasts);
    })
}

fn read_preview(path: &std::path::Path) -> Result<String, String> {
    let metadata = std::fs::metadata(path)
        .map_err(|e| format!("metadata: {e}"))?;
    if metadata.len() > PREVIEW_MAX_BYTES {
        return Err(format!(
            "File too large ({} bytes, limit {} bytes)",
            metadata.len(),
            PREVIEW_MAX_BYTES
        ));
    }
    std::fs::read_to_string(path).map_err(|e| format!("read: {e}"))
}
```

### Key patterns

- `FilePickerState` owns the directory listing. `.extensions(&[...])` filters non-directory entries; directories are always shown so you can navigate in.
- `picker.selected()` returns `Option<&PathBuf>` only when the user picks a file with Enter — directories set `current_dir` instead and emit `changed = true` but no selection.
- Always gate `std::fs::read_to_string` behind a size check. A runaway file read blocks the thread and the UI stops responding.
- Pair a `ScrollState` with every long content region. Reset `offset = 0` when you swap the content — otherwise the previous file's scroll position leaks into the new file.

---

## Where to go next

- [PATTERNS.md](PATTERNS.md) — when these recipes are not enough and you need structural advice (render helpers, screens, hooks vs app state).
- [THEMING.md](THEMING.md) — all recipes honor `ui.theme()`; swap `Theme::dark()` to `Theme::tokyo_night()` and see everything recolor.
- [TESTING.md](TESTING.md) — each recipe's logic can be exercised headlessly with `TestBackend::new(w, h).render(|ui| { ... })`.
- [ANIMATION.md](ANIMATION.md) — tween the dashboard tiles or stagger-fade the file picker list.
- [EXAMPLES.md](EXAMPLES.md) — full index of runnable examples, including the `cookbook_*` entries paired with this page.
