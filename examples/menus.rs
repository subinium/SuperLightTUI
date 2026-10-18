//! Menus, popovers and interactive states (#458, #459, #462).
//!
//! Run: `cargo run --example menus`
//!
//! - `F10` or `Alt+F` / `Alt+E` / `Alt+H` opens the menu bar; arrows navigate,
//!   `Enter` activates, `Esc` closes.
//! - Right-click a file row (or focus it and press `Shift+F10` / `Menu`) for
//!   its context menu.
//! - `i` toggles an anchored popover next to the Info button.
//! - Rows highlight on hover (`hover_bg`), the form outlines while it holds
//!   focus (`focus_within_border_fg`), and Submit stays disabled until the
//!   name is filled in.
//! - `q` quits.

use slt::widgets::{MenuBarState, MenuItem, MenuState, SelectState, TextInputState};
use slt::{Border, Context, ModalOpts, MouseMode, Placement, RunConfig};

struct App {
    bar: MenuBarState,
    file_menu: MenuState,
    files: Vec<&'static str>,
    menu_row: usize,
    name: TextInputState,
    size: SelectState,
    info_open: bool,
    confirm_quit: bool,
    log: String,
}

impl App {
    fn new() -> Self {
        Self {
            bar: MenuBarState::new(vec![
                (
                    "File",
                    vec![
                        MenuItem::new("new", "New").shortcut("Ctrl+N"),
                        MenuItem::new("save", "Save").shortcut("Ctrl+S"),
                        MenuItem::separator(),
                        MenuItem::new("quit", "Quit").shortcut("q"),
                    ],
                ),
                (
                    "Edit",
                    vec![
                        MenuItem::new("undo", "Undo").disabled(true),
                        MenuItem::new("redo", "Redo"),
                        MenuItem::submenu(
                            "find",
                            "Find",
                            vec![
                                MenuItem::new("find-next", "Find next"),
                                MenuItem::new("find-prev", "Find previous"),
                            ],
                        ),
                    ],
                ),
                ("Help", vec![MenuItem::new("about", "About")]),
            ]),
            file_menu: MenuState::new(vec![
                MenuItem::new("open", "Open"),
                MenuItem::new("rename", "Rename").shortcut("F2"),
                MenuItem::separator(),
                MenuItem::new("delete", "Delete").shortcut("Del"),
            ]),
            files: vec!["notes.md", "todo.txt", "draft.rs"],
            menu_row: 0,
            name: TextInputState::with_placeholder("Your name"),
            size: SelectState::new(vec!["Small", "Medium", "Large"]),
            info_open: false,
            confirm_quit: false,
            log: String::from("Nothing chosen yet"),
        }
    }

    fn render(&mut self, ui: &mut Context) {
        if let Some(id) = ui.menu_bar(&mut self.bar) {
            if id == "quit" {
                self.confirm_quit = true;
            }
            self.log = format!("Menu bar: {id}");
        }
        if ui.key('q') {
            ui.quit();
        }
        if ui.key('i') {
            self.info_open = !self.info_open;
        }

        let _ = ui.container().p(1).gap(1).grow(1).col(|ui| {
            ui.text("Files (right-click a row)").bold();
            for index in 0..self.files.len() {
                let file = self.files[index];
                // A focusable row: hover and focus are styled separately.
                let focused = ui.register_focusable();
                let theme = *ui.theme();
                let mut row = ui
                    .container()
                    .px(1)
                    .hover_bg(theme.surface_hover)
                    .focus_bg(theme.selected_bg)
                    .focus_fg(theme.selected_fg)
                    .row(|ui| {
                        ui.text(file);
                    });
                row.focused = focused;
                // One menu serves every row: remember which row opened it.
                let was_open = self.file_menu.is_open();
                if let Some(id) = ui.context_menu(&mut self.file_menu, &row) {
                    self.log = format!("{id} {}", self.files[self.menu_row]);
                }
                if !was_open && self.file_menu.is_open() {
                    self.menu_row = index;
                }
            }

            let info = ui.button("Info");
            if info.clicked {
                self.info_open = !self.info_open;
            }
            if self.info_open {
                let pop = ui.popover(info.rect, Placement::Below, |ui| {
                    ui.text("Popovers float next to an element");
                    ui.text("and flip at the screen edge.").dim();
                });
                if pop.dismissed {
                    self.info_open = false;
                }
            }

            let ready = !self.name.value.trim().is_empty();
            let primary = ui.theme().primary;
            let _ = ui
                .bordered(Border::Rounded)
                .title("Order")
                .p(1)
                .gap(1)
                .focus_within_border_fg(primary)
                .col(|ui| {
                    let _ = ui.text_input(&mut self.name);
                    let _ = ui.select(&mut self.size);
                    let submit = ui.disabled(!ready, |ui| ui.button("Submit"));
                    if submit.clicked {
                        self.log = format!(
                            "Ordered {} for {}",
                            self.size.selected_item().unwrap_or("?"),
                            self.name.value
                        );
                    }
                });

            ui.text(&self.log).dim();
        });

        if self.confirm_quit {
            let _ = ui.modal_with(ModalOpts::new(), |ui| {
                let _ = ui.bordered(Border::Rounded).p(1).gap(1).col(|ui| {
                    ui.text("Quit the demo?");
                    let _ = ui.row(|ui| {
                        if ui.button("Quit").clicked {
                            ui.quit();
                        }
                        if ui.button("Cancel").clicked {
                            self.confirm_quit = false;
                        }
                    });
                });
            });
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut app = App::new();
    slt::run_with(
        RunConfig::default().mouse_mode(MouseMode::Hover),
        move |ui: &mut Context| app.render(ui),
    )
}
