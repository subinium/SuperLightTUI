use slt::{Border, Color, Context, ListState, ScrollState, SpinnerState, TextInputState, Theme};

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

fn main() -> std::io::Result<()> {
    let mut search = TextInputState::with_placeholder("Search packages...");
    let mut pkg_list = ListState::new(
        PACKAGES
            .iter()
            .map(|p| p.name.to_string())
            .collect::<Vec<_>>(),
    );
    let mut output_scroll = ScrollState::new();
    let spinner = SpinnerState::dots();
    let mut installing = false;
    let mut install_progress = 0.0_f64;
    let mut output_lines: Vec<(Color, String)> = vec![
        (Color::Indexed(245), "cargo-slt v0.1.0".into()),
        (
            Color::Indexed(245),
            "Type to search, Enter to install/update".into(),
        ),
    ];
    let mut dark_mode = true;

    slt::run_with(
        slt::RunConfig {
            mouse: true,
            ..Default::default()
        },
        |ui: &mut Context| {
            if ui.key_mod('q', slt::KeyModifiers::CONTROL) {
                ui.quit();
            }
            if ui.key_code(slt::KeyCode::Esc) {
                installing = false;
                install_progress = 0.0;
            }
            if ui.key_mod('t', slt::KeyModifiers::CONTROL) {
                dark_mode = !dark_mode;
            }
            ui.set_theme(if dark_mode {
                Theme::dark()
            } else {
                Theme::light()
            });

            let filtered: Vec<usize> = PACKAGES
                .iter()
                .enumerate()
                .filter(|(_, p)| {
                    search.value.is_empty() || p.name.contains(&search.value.to_lowercase())
                })
                .map(|(i, _)| i)
                .collect();

            if filtered.is_empty() {
                pkg_list.selected = 0;
            } else {
                pkg_list.selected = pkg_list.selected.min(filtered.len().saturating_sub(1));
            }

            if installing {
                install_progress = (install_progress + 0.02).min(1.0);
                if install_progress >= 1.0 {
                    installing = false;
                    if let Some(&pkg_idx) = filtered.get(pkg_list.selected) {
                        let pkg = &PACKAGES[pkg_idx];
                        output_lines.push((
                            Color::Green,
                            format!("Installed {} v{}", pkg.name, pkg.version),
                        ));
                    }
                    install_progress = 0.0;
                }
            }

            ui.bordered(Border::Rounded)
                .title("cargo-slt")
                .pad(1)
                .grow(1)
                .col(|ui| {
                    ui.row(|ui| {
                        ui.text("cargo-slt").bold().fg(Color::Cyan);
                        ui.spacer();
                        ui.text(format!("{} packages", PACKAGES.len())).dim();
                    });
                    ui.separator();

                    ui.container().grow(1).row(|ui| {
                        // left: search + list
                        ui.bordered(Border::Rounded)
                            .title("Packages")
                            .pad(1)
                            .grow(1)
                            .col(|ui| {
                                ui.text_input(&mut search);
                                ui.separator();
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
                                            let color_char = match p.status {
                                                "outdated" => '!',
                                                "not installed" => ' ',
                                                _ => ' ',
                                            };
                                            let _ = color_char;
                                            format!(
                                                "{marker} {:<12} {:<10} {}",
                                                p.name, p.version, p.status
                                            )
                                        })
                                        .collect();
                                    pkg_list.set_items(items);
                                    ui.list(&mut pkg_list);
                                }
                            });

                        // right: detail + output
                        ui.container().grow(1).col(|ui| {
                            let sel = filtered.get(pkg_list.selected).copied().unwrap_or(0);
                            let pkg = &PACKAGES[sel];

                            ui.bordered(Border::Rounded)
                                .title("Details")
                                .pad(1)
                                .col(|ui| {
                                    ui.text(pkg.name).bold().fg(Color::Cyan);
                                    ui.text(format!("v{}", pkg.version)).dim();
                                    ui.separator();
                                    ui.text(pkg.desc);
                                    ui.row(|ui| {
                                        ui.text("License:").dim();
                                        ui.text(pkg.license);
                                    });
                                    ui.row(|ui| {
                                        ui.text("Dependencies:").dim();
                                        ui.text(format!("{}", pkg.deps));
                                    });
                                    ui.row(|ui| {
                                        ui.text("Size:").dim();
                                        ui.text(pkg.size);
                                    });
                                    ui.row(|ui| {
                                        ui.text("Status:").dim();
                                        let (label, color) = match pkg.status {
                                            "installed" => ("installed", Color::Green),
                                            "outdated" => ("update available", Color::Yellow),
                                            _ => ("not installed", Color::Indexed(245)),
                                        };
                                        ui.text(label).fg(color);
                                    });

                                    if installing {
                                        ui.separator();
                                        ui.row(|ui| {
                                            ui.spinner(&spinner);
                                            ui.text(format!(
                                                " Installing... {:.0}%",
                                                install_progress * 100.0
                                            ))
                                            .fg(Color::Yellow);
                                        });
                                        ui.progress(install_progress);
                                    } else {
                                        ui.separator();
                                        ui.row(|ui| {
                                            let action = match pkg.status {
                                                "installed" => "Reinstall",
                                                "outdated" => "Update",
                                                _ => "Install",
                                            };
                                            if ui.button(action).clicked {
                                                installing = true;
                                                install_progress = 0.0;
                                                output_lines.push((
                                                    Color::Yellow,
                                                    format!(
                                                        "Installing {} v{}...",
                                                        pkg.name, pkg.version
                                                    ),
                                                ));
                                            }
                                            if (pkg.status == "installed"
                                                || pkg.status == "outdated")
                                                && ui.button("Remove").clicked
                                            {
                                                output_lines.push((
                                                    Color::Red,
                                                    format!("Removed {}", pkg.name),
                                                ));
                                            }
                                        });
                                    }
                                });

                            ui.bordered(Border::Rounded)
                                .title("Output")
                                .pad(1)
                                .grow(1)
                                .col(|ui| {
                                    ui.scrollable(&mut output_scroll).grow(1).col(|ui| {
                                        for (color, line) in &output_lines {
                                            ui.text(line.as_str()).fg(*color);
                                        }
                                    });
                                });
                        });
                    });

                    ui.separator();
                    ui.help(&[
                        ("Ctrl+Q", "quit"),
                        ("Ctrl+T", "theme"),
                        ("Tab", "focus"),
                        ("Enter", "action"),
                        ("Esc", "cancel"),
                    ]);
                });
        },
    )
}
