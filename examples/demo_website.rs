use slt::{Border, Color, Context, Padding, ScrollState, Style, TabsState, Theme, ToastState};

fn main() -> std::io::Result<()> {
    let mut nav = TabsState::new(vec!["Home", "Docs", "Blog", "Pricing", "Contact"]);
    let mut scroll = ScrollState::new();
    let mut dark_mode = true;
    let mut email = slt::TextInputState::with_placeholder("you@example.com");
    let mut blog_view: Option<usize> = None;
    let mut toasts = ToastState::new();
    let mut subscribed = false;
    let mut nav_target: Option<usize> = None;

    slt::run_with(
        slt::RunConfig {
            mouse: true,
            ..Default::default()
        },
        |ui: &mut Context| {
            if ui.key('q') {
                ui.quit();
            }
            if ui.key('t') {
                dark_mode = !dark_mode;
            }
            if ui.key_code(slt::KeyCode::Esc) {
                blog_view = None;
            }
            for (i, ch) in ['1', '2', '3', '4', '5'].iter().enumerate() {
                if ui.key(*ch) {
                    nav_target = Some(i);
                }
            }
            ui.set_theme(if dark_mode {
                Theme::dark()
            } else {
                Theme::light()
            });

            let tick = ui.tick();

            if let Some(target) = nav_target.take() {
                nav.selected = target;
                scroll = ScrollState::new();
            }

            ui.container().grow(1).col(|ui| {
                // ── navbar ──
                ui.bordered(Border::Thick).pad(1).col(|ui| {
                    ui.row(|ui| {
                        ui.text("SLT").bold().fg(Color::Cyan);
                        ui.text("Framework").fg(Color::Indexed(245));
                        ui.spacer();
                        ui.tabs(&mut nav);
                    });
                });

                let selected = nav.selected;
                ui.scrollable(&mut scroll).grow(1).col(|ui| {
                    match selected {
                        0 => render_home(
                            ui,
                            &mut email,
                            &mut nav_target,
                            &mut toasts,
                            &mut subscribed,
                            tick,
                        ),
                        1 => render_docs(ui),
                        2 => render_blog(ui, &mut blog_view),
                        3 => render_pricing(ui, &mut toasts, tick),
                        _ => render_contact(ui, &mut nav_target),
                    }

                    // ── footer ──
                    ui.separator();
                    ui.container().padding(Padding::xy(2, 1)).col(|ui| {
                        ui.row(|ui| {
                            ui.text("SLT Framework").bold().fg(Color::Cyan);
                            ui.spacer();
                            ui.text("MIT License").dim();
                        });
                        ui.row(|ui| {
                            ui.text("GitHub").fg(Color::Blue).underline();
                            ui.text(" · ").dim();
                            if ui.button("Docs") {
                                nav_target = Some(1);
                            }
                            ui.text(" · ").dim();
                            ui.text("Discord").fg(Color::Blue).underline();
                            ui.spacer();
                            ui.text("Built with SLT v0.1.0").dim();
                        });
                    });
                });

                ui.toast(&mut toasts);

                ui.help(&[
                    ("q", "quit"),
                    ("t", "theme"),
                    ("1-5", "tabs"),
                    ("Esc", "back"),
                    ("Tab", "focus"),
                ]);
            });
        },
    )
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Markdown-like rendering helpers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn md_h1(ui: &mut Context, text: &str) {
    ui.text(text).bold().fg(Color::Cyan);
    ui.text("━".repeat(text.len().min(60)))
        .fg(Color::Indexed(240));
    ui.text("");
}

fn md_h2(ui: &mut Context, text: &str) {
    ui.text(format!("## {text}")).bold().fg(Color::Cyan);
    ui.text("");
}

fn md_h3(ui: &mut Context, text: &str) {
    ui.text(format!("### {text}")).bold().fg(Color::White);
}

fn md_p(ui: &mut Context, text: &str) {
    ui.text_wrap(text);
    ui.text("");
}

fn md_p_dim(ui: &mut Context, text: &str) {
    ui.text_wrap(text).dim();
    ui.text("");
}

fn md_blockquote(ui: &mut Context, text: &str) {
    ui.container().padding(Padding::new(0, 0, 2, 0)).col(|ui| {
        for line in text.lines() {
            ui.row(|ui| {
                ui.text("  ▎ ").fg(Color::Indexed(245));
                ui.text_wrap(line).italic().fg(Color::Indexed(252));
            });
        }
    });
    ui.text("");
}

fn md_bullet(ui: &mut Context, items: &[&str]) {
    for item in items {
        ui.row(|ui| {
            ui.text("  • ").fg(Color::Cyan);
            ui.text_wrap(*item);
        });
    }
    ui.text("");
}

fn md_numbered(ui: &mut Context, items: &[&str]) {
    for (i, item) in items.iter().enumerate() {
        ui.row(|ui| {
            ui.styled(
                format!("  {}. ", i + 1),
                Style::new().fg(Color::Cyan).bold(),
            );
            ui.text_wrap(*item);
        });
    }
    ui.text("");
}

fn md_code_block(ui: &mut Context, lang: &str, code: &str) {
    ui.bordered(Border::Rounded).pad(1).col(|ui| {
        ui.row(|ui| {
            ui.text(format!(" {lang} "))
                .fg(Color::Black)
                .bg(Color::Indexed(245));
            ui.spacer();
        });
        ui.text("");
        for line in code.lines() {
            ui.text(line).fg(Color::Green);
        }
    });
    ui.text("");
}

fn md_inline_code(ui: &mut Context, text: &str) {
    ui.styled(
        format!(" {text} "),
        Style::new().fg(Color::Yellow).bg(Color::Indexed(236)),
    );
}

fn md_link(ui: &mut Context, label: &str, url: &str) {
    ui.row(|ui| {
        ui.text(label).fg(Color::Blue).underline();
        ui.text(format!(" ({url})")).dim();
    });
}

fn md_hr(ui: &mut Context) {
    ui.separator();
    ui.text("");
}

fn md_tag(ui: &mut Context, tag: &str, color: Color) {
    ui.styled(
        format!(" {tag} "),
        Style::new().fg(Color::Black).bg(color).bold(),
    );
}

fn md_meta(ui: &mut Context, date: &str, reading_time: &str) {
    ui.row(|ui| {
        ui.text(date).dim();
        ui.text(" · ").dim();
        ui.text(reading_time).dim();
    });
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// HOME
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn render_home(
    ui: &mut Context,
    email: &mut slt::TextInputState,
    nav_target: &mut Option<usize>,
    toasts: &mut ToastState,
    subscribed: &mut bool,
    tick: u64,
) {
    // hero
    ui.container()
        .px(4)
        .py(2)
        .col(|ui| {
            ui.text("Build TUIs in minutes, not hours.").bold().fg(Color::Cyan);
            ui.text("");
            md_p(ui, "SLT is a lightweight, immediate-mode terminal UI framework for Rust. \
                       Describe your UI each frame with closures. No retained state, no virtual DOM, \
                       no message passing. Just Rust.");
            ui.row(|ui| {
                md_inline_code(ui, "cargo add superlighttui");
            });
            ui.text("");
            ui.row(|ui| {
                if ui.button("Get Started") {
                    *nav_target = Some(1);
                }
                if ui.button("View on GitHub") {
                    toasts.info("github.com/user/superlighttui", tick);
                }
            });
        });

    md_hr(ui);

    // stats
    ui.container().padding(Padding::xy(2, 1)).col(|ui| {
        ui.row(|ui| {
            stat_block(ui, "~5k", "Lines of Code");
            stat_block(ui, "14", "Widgets");
            stat_block(ui, "2", "Dependencies");
            stat_block(ui, "0", "unsafe blocks");
        });
    });

    md_hr(ui);

    // quick start guide
    ui.container().padding(Padding::xy(2, 1)).col(|ui| {
        md_h2(ui, "Quick Start");

        md_p(
            ui,
            "Get a TUI running in 5 lines of Rust. No App struct, no trait impls, \
                   no event loop boilerplate. Ctrl+C works out of the box.",
        );

        md_code_block(
            ui,
            "rust",
            "fn main() -> std::io::Result<()> {\n\
             \x20   slt::run(|ui: &mut slt::Context| {\n\
             \x20       ui.text(\"hello, world\");\n\
             \x20   })\n\
             }",
        );

        md_p(
            ui,
            "That's the whole app. Your closure runs every frame. Call methods on \
                   the Context to describe your UI. SLT handles layout, diffing, and rendering.",
        );
    });

    md_hr(ui);

    // why SLT
    ui.container().padding(Padding::xy(2, 1)).col(|ui| {
        md_h2(ui, "Why SLT?");

        md_p(
            ui,
            "Terminal UIs have been stuck in the 90s. You either wire up a massive \
                   event loop or fight a framework that was designed for a different era. \
                   SLT takes the best ideas from egui, React, and Tailwind CSS and brings \
                   them to the terminal.",
        );

        ui.row(|ui| {
            feature_card(
                ui,
                "Immediate Mode",
                "No retained state. Your closure IS the UI. Like egui, but for terminals.",
            );
            feature_card(
                ui,
                "Flexbox Layout",
                "row() and col() with gap, grow, align. CSS Flexbox semantics without the CSS.",
            );
        });
        ui.row(|ui| {
            feature_card(
                ui,
                "Auto Everything",
                "Focus cycling, scroll, hit testing, event consumption. Zero boilerplate.",
            );
            feature_card(
                ui,
                "Two Dependencies",
                "crossterm + unicode-width. No OpenSSL, no system libs, compiles everywhere.",
            );
        });
    });

    md_hr(ui);

    // newsletter
    ui.container().px(4).py(1).col(|ui| {
        md_h2(ui, "Stay Updated");
        if *subscribed {
            ui.text("Subscribed!").bold().fg(Color::Green);
            md_p_dim(ui, "You'll receive updates at the address you provided.");
        } else {
            md_p_dim(
                ui,
                "Get notified about new releases, tutorials, and community highlights.",
            );
            ui.row(|ui| {
                ui.text_input(email);
                if ui.button("Subscribe") {
                    if !email.value.is_empty() {
                        *subscribed = true;
                        toasts.success("Subscribed!", tick);
                    } else {
                        toasts.error("Enter an email first", tick);
                    }
                }
            });
        }
    });
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// DOCS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn render_docs(ui: &mut Context) {
    ui.container().padding(Padding::xy(2, 1)).col(|ui| {
        md_h1(ui, "Documentation");

        // ── Getting Started ──
        md_h2(ui, "Getting Started");
        md_p(ui, "Add SLT to your project:");
        md_code_block(ui, "sh", "cargo add superlighttui");
        md_p(
            ui,
            "The crate re-exports everything under `slt`, so you can write:",
        );
        md_code_block(ui, "rust", "use slt::*;");

        md_hr(ui);

        // ── Layout System ──
        md_h2(ui, "Layout System");
        md_p(
            ui,
            "SLT uses a flexbox-inspired layout. Every container is either a column (vertical) \
                   or a row (horizontal). Children are placed in order along the main axis.",
        );

        md_h3(ui, "Columns and Rows");
        md_p(
            ui,
            "Use `col()` for vertical stacking and `row()` for horizontal placement:",
        );
        md_code_block(
            ui,
            "rust",
            "ui.col(|ui| {\n\
             \x20   ui.text(\"top\");\n\
             \x20   ui.text(\"bottom\");\n\
             });\n\
             \n\
             ui.row(|ui| {\n\
             \x20   ui.text(\"left\");\n\
             \x20   ui.text(\"right\");\n\
             });",
        );

        md_h3(ui, "Growing and Spacing");
        md_p(
            ui,
            "Use `grow()` to distribute remaining space. `spacer()` pushes siblings apart:",
        );
        md_code_block(
            ui,
            "rust",
            "ui.row(|ui| {\n\
             \x20   ui.text(\"left\");\n\
             \x20   ui.spacer();           // fills remaining space\n\
             \x20   ui.text(\"right\");\n\
             });\n\
             \n\
             ui.container().grow(1).col(|ui| {\n\
             \x20   ui.text(\"I fill all available height\");\n\
             });",
        );

        md_h3(ui, "Gap, Padding, Margin");
        md_p(ui, "Chain layout modifiers on containers:");
        md_code_block(
            ui,
            "rust",
            "ui.container()\n\
             \x20   .gap(1)                // space between children\n\
             \x20   .pad(2)                // inner padding (all sides)\n\
             \x20   .padding(Padding::xy(4, 1))  // horizontal=4, vertical=1\n\
             \x20   .margin(Margin::new(1,1,0,0))\n\
             \x20   .col(|ui| { /* ... */ });",
        );

        md_hr(ui);

        // ── Styling ──
        md_h2(ui, "Styling");
        md_p(
            ui,
            "Style text by chaining methods. Colors support named, 256-indexed, and RGB:",
        );
        md_code_block(
            ui,
            "rust",
            "ui.text(\"Bold cyan\").bold().fg(Color::Cyan);\n\
             ui.text(\"Dim italic\").dim().italic();\n\
             ui.text(\"Custom\").fg(Color::Rgb(255, 100, 50));\n\
             ui.text(\"Indexed\").fg(Color::Indexed(208));",
        );

        md_h3(ui, "Borders and Titles");
        md_p(ui, "Containers can have borders with optional titles:");
        md_code_block(
            ui,
            "rust",
            "ui.bordered(Border::Rounded)\n\
             \x20   .title(\"My Section\")\n\
             \x20   .pad(1)\n\
             \x20   .col(|ui| {\n\
             \x20       ui.text(\"inside\");\n\
             \x20   });",
        );

        md_h3(ui, "Themes");
        md_p(ui, "Switch the entire color scheme in one call:");
        md_code_block(
            ui,
            "rust",
            "// In your run loop:\n\
             ui.set_theme(Theme::dark());   // or Theme::light()\n\
             \n\
             // Custom theme:\n\
             let my_theme = Theme {\n\
             \x20   bg: Color::Rgb(30, 30, 46),\n\
             \x20   fg: Color::Rgb(205, 214, 244),\n\
             \x20   accent: Color::Rgb(137, 180, 250),\n\
             \x20   ..Theme::dark()\n\
             };",
        );

        md_hr(ui);

        // ── Widgets Reference ──
        md_h2(ui, "Widget Reference");
        md_p(
            ui,
            "SLT ships 14 widgets. All handle their own keyboard/mouse events. \
                   Focus cycling via Tab/Shift+Tab is automatic.",
        );

        md_h3(ui, "Text Input");
        md_code_block(
            ui,
            "rust",
            "let mut state = TextInputState::with_placeholder(\"Email...\");\n\
             ui.text_input(&mut state);\n\
             // state.value() returns the current text",
        );

        md_h3(ui, "Textarea");
        md_code_block(
            ui,
            "rust",
            "let mut state = TextareaState::new();\n\
             ui.textarea(&mut state, 5);  // 5 visible rows",
        );

        md_h3(ui, "Button");
        md_code_block(
            ui,
            "rust",
            "if ui.button(\"Submit\") {\n\
             \x20   // clicked!\n\
             }",
        );

        md_h3(ui, "Checkbox & Toggle");
        md_code_block(
            ui,
            "rust",
            "let mut dark = true;\n\
             ui.checkbox(\"Dark mode\", &mut dark);\n\
             ui.toggle(\"Notifications\", &mut enabled);",
        );

        md_h3(ui, "Tabs");
        md_code_block(
            ui,
            "rust",
            "let mut tabs = TabsState::new(vec![\"Home\", \"Settings\"]);\n\
             ui.tabs(&mut tabs);\n\
             match tabs.selected {\n\
             \x20   0 => render_home(ui),\n\
             \x20   _ => render_settings(ui),\n\
             }",
        );

        md_h3(ui, "List & Table");
        md_code_block(
            ui,
            "rust",
            "let mut list = ListState::new(vec![\"Alpha\", \"Beta\"]);\n\
             ui.list(&mut list);\n\
             \n\
             let mut table = TableState::new(\n\
             \x20   vec![\"Name\", \"Lang\"],\n\
             \x20   vec![vec![\"SLT\", \"Rust\"]],\n\
             );\n\
             ui.table(&mut table);",
        );

        md_h3(ui, "Scrollable");
        md_p(
            ui,
            "Wraps any content in a scrollable viewport. Handles mouse wheel and drag-to-scroll:",
        );
        md_code_block(
            ui,
            "rust",
            "let mut scroll = ScrollState::new();\n\
             ui.scrollable(&mut scroll).grow(1).col(|ui| {\n\
             \x20   for i in 0..100 {\n\
             \x20       ui.text(format!(\"Line {i}\"));\n\
             \x20   }\n\
             });",
        );

        md_h3(ui, "Spinner, Progress, Toast");
        md_code_block(
            ui,
            "rust",
            "let spinner = SpinnerState::dots();\n\
             ui.spinner(&spinner);\n\
             ui.progress(0.75);\n\
             \n\
             let mut toasts = ToastState::new();\n\
             toasts.success(\"Saved!\", ui.tick());\n\
             ui.toast(&mut toasts);",
        );

        md_hr(ui);

        // ── Events ──
        md_h2(ui, "Event Handling");
        md_p(
            ui,
            "Events are checked per-frame. Widgets auto-consume their events so you \
                   never accidentally handle the same keypress twice.",
        );

        md_code_block(
            ui,
            "rust",
            "if ui.key('q') { ui.quit(); }\n\
             if ui.key('j') { scroll_down(); }\n\
             if ui.key_code(KeyCode::Enter) { submit(); }\n\
             if ui.key_with_mod('s', KeyModifiers::CONTROL) { save(); }",
        );

        md_hr(ui);

        // ── Advanced ──
        md_h2(ui, "Advanced Topics");

        md_h3(ui, "Animation");
        md_p(ui, "Tween and Spring primitives for smooth transitions:");
        md_code_block(
            ui,
            "rust",
            "let mut tween = Tween::new(0.0, 100.0, 60);\n\
             let value = tween.value(ui.tick());\n\
             \n\
             let mut spring = Spring::new(0.0, 180.0, 12.0);\n\
             spring.set_target(50.0);",
        );

        md_h3(ui, "Inline Mode");
        md_p(
            ui,
            "Render below the prompt without entering the alternate screen:",
        );
        md_code_block(
            ui,
            "rust",
            "slt::run_inline(3, |ui| {\n\
             \x20   ui.text(\"No alt screen!\");\n\
             });",
        );

        md_h3(ui, "Async");
        md_p(ui, "Optional tokio integration for background data:");
        md_code_block(
            ui,
            "rust",
            "let tx = slt::run_async(|ui, msgs: &mut Vec<String>| {\n\
             \x20   for m in msgs.drain(..) { ui.text(m); }\n\
             })?;\n\
             tx.send(\"hello\".into()).await?;",
        );
        md_p_dim(ui, "Requires: cargo add superlighttui --features async");
    });
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BLOG
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

struct BlogPost {
    date: &'static str,
    title: &'static str,
    reading_time: &'static str,
    tags: &'static [(&'static str, Color)],
    excerpt: &'static str,
    render: fn(&mut Context),
}

const BLOG_POSTS: &[BlogPost] = &[
    BlogPost {
        date: "2025-03-10",
        title: "Announcing SLT v0.1.0",
        reading_time: "5 min read",
        tags: &[("release", Color::Green), ("announcement", Color::Cyan)],
        excerpt: "The first public release of Super Light TUI is here. Two dependencies, zero unsafe, 14 widgets, and an API that gets out of your way.",
        render: render_post_announcement,
    },
    BlogPost {
        date: "2025-03-08",
        title: "Why Immediate Mode for TUIs?",
        reading_time: "8 min read",
        tags: &[("architecture", Color::Magenta), ("deep-dive", Color::Yellow)],
        excerpt: "How egui-style rendering makes terminal UI development 10x faster, and why retained-mode frameworks add complexity you don't need.",
        render: render_post_immediate_mode,
    },
    BlogPost {
        date: "2025-03-05",
        title: "Building a Dashboard in 50 Lines",
        reading_time: "4 min read",
        tags: &[("tutorial", Color::Blue), ("beginner", Color::Green)],
        excerpt: "Step-by-step guide to building a real-time system dashboard with SLT. Metrics, tables, and live updates in under a minute of reading.",
        render: render_post_dashboard_tutorial,
    },
    BlogPost {
        date: "2025-03-01",
        title: "The Case for u32 Coordinates",
        reading_time: "3 min read",
        tags: &[("technical", Color::Red), ("design-decision", Color::Indexed(208))],
        excerpt: "Why every TUI library using u16 coordinates has a latent overflow bug, and how SLT avoids it with u32 at zero runtime cost.",
        render: render_post_u32,
    },
    BlogPost {
        date: "2025-02-25",
        title: "Flexbox for Terminals: How SLT Layout Works",
        reading_time: "6 min read",
        tags: &[("internals", Color::Magenta), ("layout", Color::Cyan)],
        excerpt: "A deep dive into SLT's layout engine: how row(), col(), grow(), and gap() map to CSS Flexbox concepts, and where they intentionally diverge.",
        render: render_post_flexbox,
    },
];

fn render_blog(ui: &mut Context, blog_view: &mut Option<usize>) {
    ui.container().padding(Padding::xy(2, 1)).col(|ui| {
        if let Some(idx) = *blog_view {
            // ── Single post view ──
            if let Some(post) = BLOG_POSTS.get(idx) {
                ui.row(|ui| {
                    if ui.button("<< Back to Blog") {
                        *blog_view = None;
                    }
                    ui.spacer();
                });
                ui.text("");
                (post.render)(ui);
            }
        } else {
            // ── Blog listing ──
            md_h1(ui, "Blog");
            md_p_dim(
                ui,
                "Thoughts on terminal UI design, Rust patterns, and building tools that developers actually enjoy using.",
            );

            for (i, post) in BLOG_POSTS.iter().enumerate() {
                let resp = ui.bordered(Border::Rounded).pad(1).col(|ui| {
                    md_meta(ui, post.date, post.reading_time);
                    ui.text(post.title).bold().fg(Color::Cyan);
                    ui.row(|ui| {
                        for (tag, color) in post.tags {
                            md_tag(ui, tag, *color);
                            ui.text(" ");
                        }
                    });
                    ui.text("");
                    ui.text_wrap(post.excerpt);
                    ui.text("");
                    ui.text("Read more ->").fg(Color::Cyan);
                });
                if resp.clicked {
                    *blog_view = Some(i);
                }
                ui.text("");
            }
        }
    });
}

// ── Blog Post: Announcing SLT v0.1.0 ──

fn render_post_announcement(ui: &mut Context) {
    md_h1(ui, "Announcing SLT v0.1.0");
    md_meta(ui, "2025-03-10", "5 min read");
    ui.text("");

    md_p(
        ui,
        "After months of iteration, the first public release of SLT (Super Light TUI) \
              is available on crates.io. This post covers what SLT is, why it exists, and where \
              we're headed.",
    );

    md_h2(ui, "What is SLT?");
    md_p(
        ui,
        "SLT is an immediate-mode terminal UI framework for Rust. If you've used egui for \
              graphical UIs, the programming model will feel familiar: you describe your UI each \
              frame by calling methods on a Context, and the framework handles layout, diffing, \
              and rendering.",
    );

    md_blockquote(
        ui,
        "The name is longer than your hello world.\nThat's the point.",
    );

    md_h2(ui, "Design Principles");
    md_numbered(
        ui,
        &[
            "Minimal API surface: learn 5 methods, build anything.",
            "Zero boilerplate: no App struct, no Model/Update/View, no trait impls.",
            "CSS-like layout: row(), col(), gap(), grow() map directly to Flexbox.",
            "Batteries included: 14 widgets with built-in keyboard and mouse support.",
            "Tiny dependency tree: crossterm + unicode-width. That's it.",
        ],
    );

    md_h2(ui, "Hello World");
    md_p(ui, "The smallest SLT program is genuinely 5 lines:");
    md_code_block(
        ui,
        "rust",
        "fn main() -> std::io::Result<()> {\n\
         \x20   slt::run(|ui: &mut slt::Context| {\n\
         \x20       ui.text(\"hello, world\");\n\
         \x20   })\n\
         }",
    );
    md_p(
        ui,
        "No App struct. No message enum. No event loop. Ctrl+C is handled by default. \
              State lives in your closure's scope as regular Rust variables.",
    );

    md_h2(ui, "What's Included in v0.1.0");
    md_bullet(ui, &[
        "14 widgets: TextInput, Textarea, Button, Checkbox, Toggle, Tabs, List, Table, Spinner, Progress, Scrollable, Toast, Separator, HelpBar",
        "Flexbox layout engine with row/col, gap, grow, shrink, alignment",
        "Double-buffer diff rendering (only changed cells hit the terminal)",
        "Mouse support: click, hover, drag-to-scroll",
        "Automatic focus management with Tab/Shift+Tab",
        "Dark and light theme presets, or bring your own",
        "Animation primitives: Tween with 9 easings, Spring physics",
        "Inline mode for rendering below the prompt",
        "Optional async/tokio integration",
        "Layout debugger (F12)",
    ]);

    md_h2(ui, "What's Next");
    md_p(ui, "v0.2.0 will focus on:");
    md_bullet(
        ui,
        &[
            "Custom widget API for third-party extensions",
            "Color palette presets (Catppuccin, Dracula, Nord, etc.)",
            "Performance benchmarks and optimization",
            "More examples and a cookbook",
        ],
    );

    md_p(
        ui,
        "We'd love your feedback. File issues, send PRs, or just try it out:",
    );
    md_code_block(
        ui,
        "sh",
        "cargo add superlighttui\ncargo run --example demo",
    );
}

// ── Blog Post: Why Immediate Mode? ──

fn render_post_immediate_mode(ui: &mut Context) {
    md_h1(ui, "Why Immediate Mode for TUIs?");
    md_meta(ui, "2025-03-08", "8 min read");
    ui.text("");

    md_p(
        ui,
        "Most TUI frameworks use a retained-mode architecture. You define widgets, \
              register callbacks, and the framework manages a widget tree that persists \
              between frames. Ratatui, Cursive, and tui-rs all follow this pattern.",
    );

    md_p(
        ui,
        "SLT takes a different approach: immediate mode. Every frame, your closure runs \
              from scratch. There is no widget tree. There is no diffing of widget state. \
              You simply describe what should be on screen right now.",
    );

    md_h2(ui, "The Problem with Retained Mode");
    md_p(
        ui,
        "Retained-mode TUI frameworks inherit a problem from GUI frameworks: state \
              synchronization. Your application has state (a counter, a list of items, \
              a form). The framework also has state (which widget is focused, what text \
              is in an input, which list item is selected). You must keep them in sync.",
    );

    md_code_block(
        ui,
        "rust",
        "// Retained mode: state lives in two places\n\
         struct App {\n\
         \x20   items: Vec<String>,        // your state\n\
         \x20   list_state: ListState,     // framework state\n\
         }\n\
         \n\
         // You must manually sync them:\n\
         fn update(&mut self, msg: Msg) {\n\
         \x20   match msg {\n\
         \x20       Msg::AddItem(s) => {\n\
         \x20           self.items.push(s);\n\
         \x20           // Don't forget to update list_state!\n\
         \x20       }\n\
         \x20   }\n\
         }",
    );

    md_p(
        ui,
        "This is the source of most TUI bugs. Forget to update the framework state \
              and your UI is out of sync. Update it in the wrong order and you get flicker. \
              The entire Elm/MVU architecture exists to manage this complexity.",
    );

    md_h2(ui, "Immediate Mode: No Sync Required");
    md_p(
        ui,
        "In immediate mode, there is no framework state to sync. Your closure runs \
              every frame and describes the current UI based on your application state:",
    );

    md_code_block(
        ui,
        "rust",
        "// Immediate mode: state lives in one place\n\
         let mut items = vec![\"alpha\", \"beta\"];\n\
         let mut list = ListState::new(items.clone());\n\
         \n\
         slt::run(|ui| {\n\
         \x20   // UI is always a pure function of state\n\
         \x20   ui.list(&mut list);\n\
         \x20   if ui.button(\"Add\") {\n\
         \x20       items.push(\"new\");\n\
         \x20       list = ListState::new(items.clone());\n\
         \x20   }\n\
         });",
    );

    md_blockquote(
        ui,
        "Your UI is always a pure function of your state.\nThere is nothing to get out of sync.",
    );

    md_h2(ui, "Performance Concerns");
    md_p(
        ui,
        "The common objection: doesn't re-describing the entire UI every frame waste \
              work? In a terminal, no. Terminals update at ~60fps max, and a typical TUI \
              has maybe 200 widgets. Building a flat list of layout commands and diffing \
              a character buffer takes microseconds.",
    );

    md_p(
        ui,
        "SLT's double-buffer diff means only changed cells hit the terminal. The \
              rendering cost is proportional to what changed, not the total UI size. \
              In practice, immediate mode with diffing is faster than retained mode with \
              full redraws, which is what most TUI frameworks actually do.",
    );

    md_h2(ui, "When Retained Mode Wins");
    md_p(ui, "Retained mode is better when:");
    md_bullet(
        ui,
        &[
            "You have thousands of widgets (terminal UIs rarely do)",
            "Widget construction is expensive (network calls, etc.)",
            "You need fine-grained partial updates (SLT's diff handles this)",
        ],
    );

    md_p(
        ui,
        "For 99% of terminal applications, immediate mode gives you simpler code, \
              fewer bugs, and equivalent performance. That's the bet SLT makes.",
    );
}

// ── Blog Post: Dashboard Tutorial ──

fn render_post_dashboard_tutorial(ui: &mut Context) {
    md_h1(ui, "Building a Dashboard in 50 Lines");
    md_meta(ui, "2025-03-05", "4 min read");
    ui.text("");

    md_p(
        ui,
        "Let's build a real-time system dashboard with SLT. We'll display CPU usage, \
              memory, a process table, and a log stream. The whole thing fits in 50 lines.",
    );

    md_h2(ui, "Step 1: Scaffold");
    md_p(
        ui,
        "Start with the standard SLT boilerplate. We need mouse support for our table:",
    );

    md_code_block(
        ui,
        "rust",
        "use slt::*;\n\
         \n\
         fn main() -> std::io::Result<()> {\n\
         \x20   let mut scroll = ScrollState::new();\n\
         \n\
         \x20   slt::run_with(\n\
         \x20       RunConfig { mouse: true, ..Default::default() },\n\
         \x20       |ui: &mut Context| {\n\
         \x20           if ui.key('q') { ui.quit(); }\n\
         \x20           // ... UI goes here\n\
         \x20       },\n\
         \x20   )\n\
         }",
    );

    md_h2(ui, "Step 2: Metrics Row");
    md_p(
        ui,
        "Use `row()` with `grow(1)` containers to create evenly spaced metric cards:",
    );

    md_code_block(
        ui,
        "rust",
        "ui.row(|ui| {\n\
         \x20   ui.bordered(Border::Rounded).grow(1).pad(1).col(|ui| {\n\
         \x20       ui.text(\"CPU\").dim();\n\
         \x20       ui.text(\"42%\").bold().fg(Color::Cyan);\n\
         \x20       ui.progress(0.42);\n\
         \x20   });\n\
         \x20   ui.bordered(Border::Rounded).grow(1).pad(1).col(|ui| {\n\
         \x20       ui.text(\"Memory\").dim();\n\
         \x20       ui.text(\"2.1 GB / 8 GB\").bold().fg(Color::Green);\n\
         \x20       ui.progress(0.26);\n\
         \x20   });\n\
         });",
    );

    md_h2(ui, "Step 3: Process Table");
    md_p(
        ui,
        "SLT's Table widget handles headers, selection, and column sizing:",
    );

    md_code_block(
        ui,
        "rust",
        "let mut table = TableState::new(\n\
         \x20   vec![\"PID\", \"Name\", \"CPU\", \"Memory\"],\n\
         \x20   vec![\n\
         \x20       vec![\"1234\", \"rust-analyzer\", \"12.3%\", \"420MB\"],\n\
         \x20       vec![\"5678\", \"cargo\",          \"8.1%\", \"180MB\"],\n\
         \x20   ],\n\
         );\n\
         ui.table(&mut table);",
    );

    md_h2(ui, "Step 4: Log Stream");
    md_p(
        ui,
        "Wrap logs in a scrollable container. New entries push older ones up:",
    );

    md_code_block(
        ui,
        "rust",
        "ui.scrollable(&mut scroll).max_height(10).col(|ui| {\n\
         \x20   for log in &logs {\n\
         \x20       ui.text(log).dim();\n\
         \x20   }\n\
         });",
    );

    md_p(
        ui,
        "And that's it. Run it with `cargo run --example demo_dashboard` to see the \
              full version with simulated live data and animated values.",
    );

    md_blockquote(
        ui,
        "The full example is 120 lines including simulated data.\n50 lines is just the UI code.",
    );
}

// ── Blog Post: u32 Coordinates ──

fn render_post_u32(ui: &mut Context) {
    md_h1(ui, "The Case for u32 Coordinates");
    md_meta(ui, "2025-03-01", "3 min read");
    ui.text("");

    md_p(
        ui,
        "Every major TUI library uses u16 for terminal coordinates. Ratatui, Cursive, \
              tui-rs, even crossterm's raw types. The maximum value is 65,535. That seems \
              like more than enough — no terminal is 65K columns wide. So why does SLT use u32?",
    );

    md_h2(ui, "The Overflow Bug");
    md_p(
        ui,
        "The problem isn't the terminal size. It's arithmetic. When you compute layouts, \
              you add widths, subtract padding, multiply by column counts. These intermediate \
              values can exceed u16::MAX even when the final result fits in the terminal.",
    );

    md_code_block(
        ui,
        "rust",
        "// Ratatui-style u16 arithmetic:\n\
         let total_width: u16 = col_count * col_width + (col_count - 1) * gap;\n\
         //                     ^^^^^^^^^^^^^^^^^^^\n\
         //                     This overflows with 200 cols x 400 width\n\
         \n\
         // SLT u32 arithmetic: same code, no overflow\n\
         let total_width: u32 = col_count * col_width + (col_count - 1) * gap;",
    );

    md_h2(ui, "Real-World Triggers");
    md_p(ui, "This isn't theoretical. It triggers in practice:");
    md_bullet(
        ui,
        &[
            "Scrollable containers with large content (1000+ rows)",
            "Tables with many columns and wide data",
            "Nested layouts that accumulate padding and borders",
            "Animation interpolation between large values",
        ],
    );

    md_h2(ui, "Why Not Just Use i32 or usize?");
    md_p(ui, "We considered all options:");
    md_bullet(ui, &[
        "i32: Negative coordinates are meaningless for layout. Wastes a sign bit and allows invalid states.",
        "usize: 64-bit on most platforms. Wastes memory in the character buffer (millions of cells).",
        "u32: 4 billion max. More than enough for intermediate arithmetic. Same size as u16 after padding on most structs.",
    ]);

    md_p(
        ui,
        "u32 is the Goldilocks choice: large enough for safe arithmetic, small enough for \
              efficient buffers, and semantically correct (coordinates are never negative).",
    );
}

// ── Blog Post: Flexbox for Terminals ──

fn render_post_flexbox(ui: &mut Context) {
    md_h1(ui, "Flexbox for Terminals");
    md_meta(ui, "2025-02-25", "6 min read");
    ui.text("");

    md_p(
        ui,
        "SLT's layout engine is inspired by CSS Flexbox. If you've ever written \
              `display: flex; flex-direction: column; gap: 8px`, you already know how \
              SLT layout works. This post maps CSS concepts to SLT API calls.",
    );

    md_h2(ui, "The Mapping");

    ui.bordered(Border::Rounded).pad(1).col(|ui| {
        ui.row(|ui| {
            ui.styled(
                format!("{:<32}", "CSS"),
                Style::new().bold().fg(Color::Cyan),
            );
            ui.text("SLT").bold().fg(Color::Green);
        });
        ui.separator();
        let mappings = [
            ("display: flex", "implicit (all containers)"),
            ("flex-direction: column", "ui.col(|ui| { ... })"),
            ("flex-direction: row", "ui.row(|ui| { ... })"),
            ("gap: 8px", ".gap(1)"),
            ("flex-grow: 1", ".grow(1)"),
            ("flex-shrink: 0", ".shrink(0)"),
            ("padding: 8px", ".pad(1)"),
            ("padding: 8px 16px", ".padding(Padding::xy(2, 1))"),
            ("margin: 4px", ".margin(Margin::all(1))"),
            ("align-items: center", ".align(Align::Center)"),
            ("min-width: 20px", ".min_width(20)"),
            ("max-height: 10px", ".max_height(10)"),
            ("overflow: scroll", "ui.scrollable(&mut s)"),
            ("overflow: hidden", "automatic on containers"),
            ("border: 1px solid", ".border(Border::Single)"),
            ("border-radius: 4px", ".border(Border::Rounded)"),
        ];
        for (css, slt_api) in &mappings {
            ui.row(|ui| {
                ui.styled(format!("{:<32}", css), Style::new().fg(Color::Indexed(252)));
                ui.text(*slt_api).fg(Color::Green);
            });
        }
    });
    ui.text("");

    md_h2(ui, "How Layout Works Internally");
    md_p(ui, "SLT's layout algorithm runs in two passes:");

    md_numbered(ui, &[
        "Measure pass: each node computes its minimum size. Text measures its string width. Containers sum their children (column = sum heights, row = sum widths).",
        "Layout pass: starting from the root (terminal size), each container distributes space to children. Fixed-size children get their minimum. Remaining space goes to children with grow > 0, proportional to their grow factor.",
    ]);

    md_h2(ui, "Where We Diverge from CSS");
    md_p(
        ui,
        "SLT intentionally simplifies Flexbox in places that don't matter for terminals:",
    );

    md_bullet(
        ui,
        &[
            "No flex-wrap: terminal rows don't wrap. Use nested row/col instead.",
            "No order property: children render in declaration order. Always.",
            "No flex-basis: grow starts from the minimum size, not a basis.",
            "No justify-content: use spacer() for space-between/space-around effects.",
        ],
    );

    md_p(
        ui,
        "These simplifications keep the layout engine under 400 lines of code while \
              covering 95% of real-world terminal layouts.",
    );

    md_h2(ui, "A Complete Example");
    md_p(
        ui,
        "Here's a typical dashboard layout using the flexbox primitives:",
    );

    md_code_block(
        ui,
        "rust",
        "ui.col(|ui| {\n\
         \x20   // Top bar: logo left, nav right\n\
         \x20   ui.row(|ui| {\n\
         \x20       ui.text(\"MyApp\").bold();\n\
         \x20       ui.spacer();\n\
         \x20       ui.tabs(&mut nav);\n\
         \x20   });\n\
         \n\
         \x20   // Main content: sidebar + body\n\
         \x20   ui.row(|ui| {\n\
         \x20       ui.container().min_width(20).col(|ui| {\n\
         \x20           ui.list(&mut menu);\n\
         \x20       });\n\
         \x20       ui.container().grow(1).col(|ui| {\n\
         \x20           // body content\n\
         \x20       });\n\
         \x20   });\n\
         \n\
         \x20   // Footer\n\
         \x20   ui.help(&[(\"q\", \"quit\")]);\n\
         });",
    );

    md_p(
        ui,
        "No CSS files. No class names. No style props. The layout is the code.",
    );
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// PRICING
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn render_pricing(ui: &mut Context, toasts: &mut ToastState, tick: u64) {
    ui.container().padding(Padding::xy(2, 1)).col(|ui| {
        md_h1(ui, "Pricing");
        md_p_dim(
            ui,
            "SLT is free and open source forever. Sponsorship helps us ship faster.",
        );

        ui.row(|ui| {
            price_card(ui, "Open Source", "Free", "forever", &[
                "Full library access",
                "All 14 widgets",
                "MIT License",
                "Community support",
                "All examples included",
            ], Color::Green, false, toasts, tick);

            price_card(ui, "Sponsor", "$5", "/month", &[
                "Everything in Free",
                "Priority issue response",
                "Logo on README",
                "Early access to features",
                "Private Discord channel",
            ], Color::Cyan, true, toasts, tick);

            price_card(ui, "Enterprise", "Custom", "pricing", &[
                "Everything in Sponsor",
                "Dedicated support",
                "Custom widget development",
                "SLA guarantee",
                "Architecture consulting",
            ], Color::Magenta, false, toasts, tick);
        });

        ui.text("");
        md_h2(ui, "FAQ");
        faq_item(
            ui,
            "Is SLT really free?",
            "Yes. MIT licensed. Use it in commercial products, modify it, redistribute it. No strings.",
        );
        faq_item(
            ui,
            "What does sponsorship fund?",
            "Full-time development, CI infrastructure, documentation, and community management. \
             Every dollar goes directly into making SLT better.",
        );
        faq_item(
            ui,
            "Can I use SLT in production?",
            "Yes. The API is pre-1.0 so breaking changes may happen, but the core is stable \
             and well-tested. Pin your version and you'll be fine.",
        );
    });
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// CONTACT
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn render_contact(ui: &mut Context, nav_target: &mut Option<usize>) {
    ui.container().padding(Padding::xy(2, 1)).col(|ui| {
        md_h1(ui, "Contact & Community");

        md_p(ui, "SLT is built in the open. Here's how to get involved:");

        md_h2(ui, "Get Help");
        md_bullet(
            ui,
            &[
                "GitHub Issues — Bug reports and feature requests",
                "GitHub Discussions — Questions and community help",
                "Discord — Real-time chat with maintainers and users",
            ],
        );

        md_h2(ui, "Links");
        md_link(ui, "GitHub Repository", "github.com/user/superlighttui");
        md_link(ui, "API Documentation", "docs.rs/superlighttui");
        md_link(ui, "Crates.io", "crates.io/crates/superlighttui");
        md_link(ui, "Discord Server", "discord.gg/slt");
        ui.text("");
        ui.row(|ui| {
            if ui.button("View Docs") {
                *nav_target = Some(1);
            }
            if ui.button("View Pricing") {
                *nav_target = Some(3);
            }
        });

        ui.text("");
        md_h2(ui, "Contributing");
        md_p(
            ui,
            "We welcome contributions of all kinds: bug fixes, new widgets, documentation \
                   improvements, and example code. Here's how to get started:",
        );

        md_numbered(
            ui,
            &[
                "Fork the repository on GitHub",
                "Create a feature branch: git checkout -b feat/my-widget",
                "Make your changes with tests",
                "Run: cargo test && cargo clippy",
                "Submit a pull request",
            ],
        );

        md_h2(ui, "Code of Conduct");
        md_p(
            ui,
            "We follow the Rust community's Code of Conduct. Be kind, be constructive, \
                   and assume good intent. We're all here to build great terminal UIs.",
        );

        md_h2(ui, "Maintainers");
        ui.bordered(Border::Rounded).pad(1).col(|ui| {
            ui.row(|ui| {
                ui.text("@subinium").bold().fg(Color::Cyan);
                ui.text(" — ").dim();
                ui.text("Creator & lead maintainer");
            });
        });

        ui.text("");
        md_h2(ui, "Acknowledgements");
        md_p(ui, "SLT wouldn't exist without the Rust TUI ecosystem:");
        md_bullet(
            ui,
            &[
                "crossterm — The rock-solid terminal abstraction we build on",
                "ratatui — Proved that Rust TUIs can be production-quality",
                "egui — Inspiration for the immediate-mode API design",
                "Ink — Showed that declarative terminal UIs are possible",
                "Tailwind CSS — Influenced our utility-first styling approach",
            ],
        );
    });
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Shared Components
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn stat_block(ui: &mut Context, value: &str, label: &str) {
    ui.container().grow(1).center().col(|ui| {
        ui.text(value).bold().fg(Color::Cyan);
        ui.text(label).dim();
    });
}

fn feature_card(ui: &mut Context, title: &str, desc: &str) {
    ui.container().rounded().p(1).grow(1).col(|ui| {
        ui.text(title).bold().fg(Color::Cyan);
        ui.text_wrap(desc).dim();
    });
}

fn price_card(
    ui: &mut Context,
    tier: &str,
    price: &str,
    period: &str,
    features: &[&str],
    color: Color,
    highlight: bool,
    toasts: &mut ToastState,
    tick: u64,
) {
    let border = if highlight {
        Border::Double
    } else {
        Border::Rounded
    };
    ui.bordered(border).pad(1).grow(1).col(|ui| {
        ui.text(tier).bold().fg(color);
        ui.row(|ui| {
            ui.text(price).bold().fg(color);
            ui.text(format!(" {period}")).dim();
        });
        ui.separator();
        for feat in features {
            ui.row(|ui| {
                ui.text(" + ").fg(Color::Green);
                ui.text(*feat);
            });
        }
        ui.text("");
        let label = if highlight { "Sponsor" } else { "Select" };
        if ui.button(label) {
            toasts.success(format!("Selected: {tier} plan"), tick);
        }
    });
}

fn faq_item(ui: &mut Context, question: &str, answer: &str) {
    ui.container().padding(Padding::new(0, 1, 0, 0)).col(|ui| {
        ui.text(format!("Q: {question}")).bold();
        ui.text_wrap(format!("A: {answer}")).dim();
    });
    ui.text("");
}
