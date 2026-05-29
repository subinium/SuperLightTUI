//! Demo: cargo-style CLI / package manager UI with search, install, and an
//! output log scrolling region.
//!
//! Archetype: Standard. No overlays; uses theming via `set_theme` in the
//! standalone `main`. The composable [`render`] keeps theme decisions to the
//! caller so a parent tour's theme stays in charge.

use slt::widgets::{ListState, ScrollState, SpinnerState, TextInputState};
use slt::{Border, Color, Context, Theme};

struct PackageInfo {
    name: &'static str,
    version: &'static str,
    desc: &'static str,
    license: &'static str,
    deps: u32,
    size: &'static str,
    status: &'static str,
}

const PACKAGES: &[PackageInfo] = &[
    PackageInfo {
        name: "tokio",
        version: "1.41.1",
        desc: "Async runtime for Rust",
        license: "MIT",
        deps: 12,
        size: "2.1MB",
        status: "installed",
    },
    PackageInfo {
        name: "serde",
        version: "1.0.215",
        desc: "Serialization framework",
        license: "MIT/Apache-2.0",
        deps: 2,
        size: "340KB",
        status: "installed",
    },
    PackageInfo {
        name: "axum",
        version: "0.7.9",
        desc: "Web framework built on hyper",
        license: "MIT",
        deps: 28,
        size: "1.8MB",
        status: "installed",
    },
    PackageInfo {
        name: "clap",
        version: "4.5.23",
        desc: "Command line argument parser",
        license: "MIT/Apache-2.0",
        deps: 8,
        size: "890KB",
        status: "installed",
    },
    PackageInfo {
        name: "reqwest",
        version: "0.12.9",
        desc: "HTTP client library",
        license: "MIT/Apache-2.0",
        deps: 34,
        size: "3.2MB",
        status: "outdated",
    },
    PackageInfo {
        name: "sqlx",
        version: "0.8.3",
        desc: "Async SQL toolkit",
        license: "MIT/Apache-2.0",
        deps: 42,
        size: "4.5MB",
        status: "outdated",
    },
    PackageInfo {
        name: "tracing",
        version: "0.1.41",
        desc: "Application-level tracing",
        license: "MIT",
        deps: 5,
        size: "520KB",
        status: "installed",
    },
    PackageInfo {
        name: "anyhow",
        version: "1.0.94",
        desc: "Flexible error handling",
        license: "MIT/Apache-2.0",
        deps: 0,
        size: "85KB",
        status: "installed",
    },
    PackageInfo {
        name: "thiserror",
        version: "2.0.8",
        desc: "Derive macro for Error",
        license: "MIT/Apache-2.0",
        deps: 1,
        size: "65KB",
        status: "installed",
    },
    PackageInfo {
        name: "rayon",
        version: "1.10.0",
        desc: "Data parallelism library",
        license: "MIT/Apache-2.0",
        deps: 4,
        size: "410KB",
        status: "not installed",
    },
    PackageInfo {
        name: "regex",
        version: "1.11.1",
        desc: "Regular expressions",
        license: "MIT/Apache-2.0",
        deps: 6,
        size: "1.1MB",
        status: "installed",
    },
    PackageInfo {
        name: "chrono",
        version: "0.4.39",
        desc: "Date and time library",
        license: "MIT/Apache-2.0",
        deps: 3,
        size: "720KB",
        status: "outdated",
    },
];

/// State persisted across frames for the CLI demo.
pub struct DemoState {
    pub search: TextInputState,
    pub pkg_list: ListState,
    pub output_scroll: ScrollState,
    pub spinner: SpinnerState,
    pub installing: bool,
    pub install_progress: f64,
    pub output_lines: Vec<(Color, String)>,
}

impl Default for DemoState {
    fn default() -> Self {
        Self {
            search: TextInputState::with_placeholder("Search packages..."),
            pkg_list: ListState::new(
                PACKAGES
                    .iter()
                    .map(|p| p.name.to_string())
                    .collect::<Vec<_>>(),
            ),
            output_scroll: ScrollState::new(),
            spinner: SpinnerState::dots(),
            installing: false,
            install_progress: 0.0,
            output_lines: vec![
                (Color::Indexed(245), "cargo-slt v0.1.0".into()),
                (
                    Color::Indexed(245),
                    "Type to search, Enter to install/update".into(),
                ),
            ],
        }
    }
}

/// Render one frame of the CLI / package-manager demo.
///
/// Theming is left to the caller — a parent tour can wrap this in a
/// `container().theme(...)` subtree, and the standalone `main` toggles
/// the global theme directly.
pub fn render(ui: &mut Context, state: &mut DemoState) {
    let filtered: Vec<usize> = PACKAGES
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            state.search.value.is_empty() || p.name.contains(&state.search.value.to_lowercase())
        })
        .map(|(i, _)| i)
        .collect();

    if filtered.is_empty() {
        state.pkg_list.selected = 0;
    } else {
        state.pkg_list.selected = state
            .pkg_list
            .selected
            .min(filtered.len().saturating_sub(1));
    }

    if state.installing {
        state.install_progress = (state.install_progress + 0.02).min(1.0);
        if state.install_progress >= 1.0 {
            state.installing = false;
            if let Some(&pkg_idx) = filtered.get(state.pkg_list.selected) {
                let pkg = &PACKAGES[pkg_idx];
                state.output_lines.push((
                    Color::Green,
                    format!("Installed {} v{}", pkg.name, pkg.version),
                ));
            }
            state.install_progress = 0.0;
        }
    }

    let _ = ui
        .bordered(Border::Rounded)
        .title("cargo-slt")
        .p(1)
        .grow(1)
        .col(|ui| {
            let _ = ui.row(|ui| {
                ui.text("cargo-slt").bold().fg(Color::Cyan);
                ui.spacer();
                ui.text(format!("{} packages", PACKAGES.len())).dim();
            });
            let _ = ui.separator();

            let _ = ui.container().grow(1).row(|ui| {
                // left: search + list
                let _ = ui
                    .bordered(Border::Rounded)
                    .title("Packages")
                    .p(1)
                    .grow(1)
                    .col(|ui| {
                        let _ = ui.text_input(&mut state.search);
                        let _ = ui.separator();
                        if filtered.is_empty() {
                            ui.text("No packages found").dim();
                        } else {
                            let items: Vec<String> = filtered
                                .iter()
                                .map(|&i| {
                                    let p = &PACKAGES[i];
                                    let marker = match p.status {
                                        "outdated" => "↑",
                                        "not installed" => "○",
                                        _ => "●",
                                    };
                                    format!(
                                        "{marker} {:<12} {:<10} {}",
                                        p.name, p.version, p.status
                                    )
                                })
                                .collect();
                            state.pkg_list.set_items(items);
                            let _ = ui.list(&mut state.pkg_list);
                        }
                    });

                // right: detail + output
                let _ = ui.container().grow(1).col(|ui| {
                    let sel = filtered.get(state.pkg_list.selected).copied().unwrap_or(0);
                    let pkg = &PACKAGES[sel];

                    let _ = ui
                        .bordered(Border::Rounded)
                        .title("Details")
                        .p(1)
                        .col(|ui| {
                            ui.text(pkg.name).bold().fg(Color::Cyan);
                            ui.text(format!("v{}", pkg.version)).dim();
                            let _ = ui.separator();
                            ui.text(pkg.desc);
                            let _ = ui.row(|ui| {
                                ui.text("License:").dim();
                                ui.text(pkg.license);
                            });
                            let _ = ui.row(|ui| {
                                ui.text("Dependencies:").dim();
                                ui.text(format!("{}", pkg.deps));
                            });
                            let _ = ui.row(|ui| {
                                ui.text("Size:").dim();
                                ui.text(pkg.size);
                            });
                            let _ = ui.row(|ui| {
                                ui.text("Status:").dim();
                                let (label, color) = match pkg.status {
                                    "installed" => ("installed", Color::Green),
                                    "outdated" => ("update available", Color::Yellow),
                                    _ => ("not installed", Color::Indexed(245)),
                                };
                                ui.text(label).fg(color);
                            });

                            if state.installing {
                                let _ = ui.separator();
                                let _ = ui.row(|ui| {
                                    let _ = ui.spinner(&state.spinner);
                                    ui.text(format!(
                                        " Installing... {:.0}%",
                                        state.install_progress * 100.0
                                    ))
                                    .fg(Color::Yellow);
                                });
                                let _ = ui.progress(state.install_progress);
                            } else {
                                let _ = ui.separator();
                                let _ = ui.row(|ui| {
                                    let action = match pkg.status {
                                        "installed" => "Reinstall",
                                        "outdated" => "Update",
                                        _ => "Install",
                                    };
                                    if ui.button(action).clicked {
                                        state.installing = true;
                                        state.install_progress = 0.0;
                                        state.output_lines.push((
                                            Color::Yellow,
                                            format!("Installing {} v{}...", pkg.name, pkg.version),
                                        ));
                                    }
                                    if (pkg.status == "installed" || pkg.status == "outdated")
                                        && ui.button("Remove").clicked
                                    {
                                        state
                                            .output_lines
                                            .push((Color::Red, format!("Removed {}", pkg.name)));
                                    }
                                });
                            }
                        });

                    let _ = ui
                        .bordered(Border::Rounded)
                        .title("Output")
                        .p(1)
                        .grow(1)
                        .col(|ui| {
                            let _ = ui.scrollable(&mut state.output_scroll).grow(1).col(|ui| {
                                for (color, line) in &state.output_lines {
                                    ui.text(line.as_str()).fg(*color);
                                }
                            });
                        });
                });
            });

            let _ = ui.separator();
            let _ = ui.help(&[
                ("Ctrl+Q", "quit"),
                ("Ctrl+T", "theme"),
                ("Tab", "focus"),
                ("Enter", "action"),
                ("Esc", "cancel"),
            ]);
        });
}

fn main() -> std::io::Result<()> {
    let mut state = DemoState::default();
    let mut dark_mode = true;

    slt::run_with(
        slt::RunConfig::default().mouse(true),
        move |ui: &mut Context| {
            if ui.key_mod('q', slt::KeyModifiers::CONTROL) {
                ui.quit();
            }
            if ui.key_code(slt::KeyCode::Esc) {
                state.installing = false;
                state.install_progress = 0.0;
            }
            if ui.key_mod('t', slt::KeyModifiers::CONTROL) {
                dark_mode = !dark_mode;
            }
            ui.set_theme(if dark_mode {
                Theme::dark()
            } else {
                Theme::light()
            });

            render(ui, &mut state);
        },
    )
}
