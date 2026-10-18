//! v0.26 widget builders (#460) and API consolidation (#463).
//!
//! * `button(..).variant(..).colors(..)` composes (impossible before 0.26).
//! * Every deprecated `_with` / `_colored` forward renders a buffer identical
//!   to its builder replacement, in the plain, focused and hovered states.
//! * `ContainerBuilder::{fg, w_ratio}` match `text_color` / `Constraints`.
//! * `slt::prelude` does not export `crossterm`.
#![allow(deprecated)]

use slt::widgets::SliderOpts;
use slt::{
    Border, ButtonVariant, Color, Constraints, Context, EventBuilder, GridColumn, KeyCode,
    Response, Style, TestBackend, WidgetColors,
};

const W: u32 = 48;
const H: u32 = 8;

/// Every cell of a `W`x`H` buffer after two frames of `f` (the second frame
/// sees the first frame's hit map), with focus on slot 0 or nowhere and an
/// optional hover at (3, 0).
fn cells(focused: bool, hover: bool, f: &dyn Fn(&mut Context)) -> Vec<(String, Style)> {
    let focus = if focused { 0 } else { usize::MAX };
    let mut tb = TestBackend::new(W, H);
    tb.render_with_events(vec![], focus, 4, |ui| f(ui));
    let events = if hover {
        EventBuilder::new().mouse_move(3, 0).build()
    } else {
        vec![]
    };
    tb.render_with_events(events, focus, 4, |ui| f(ui));
    let buf = tb.buffer();
    let mut out = Vec::with_capacity((W * H) as usize);
    for y in 0..H {
        for x in 0..W {
            let cell = buf.get(x, y);
            out.push((cell.symbol.to_string(), cell.style));
        }
    }
    out
}

/// Assert `old` and `new` render identically in every interaction state.
fn assert_parity(name: &str, old: &dyn Fn(&mut Context), new: &dyn Fn(&mut Context)) {
    for focused in [false, true] {
        for hover in [false, true] {
            assert_eq!(
                cells(focused, hover, old),
                cells(focused, hover, new),
                "{name}: focused={focused} hover={hover}"
            );
        }
    }
}

fn custom_colors() -> WidgetColors {
    WidgetColors::new()
        .fg(Color::Rgb(250, 240, 230))
        .bg(Color::Rgb(120, 10, 30))
        .accent(Color::Rgb(255, 120, 0))
        .border(Color::Rgb(90, 90, 90))
}

const VARIANTS: [ButtonVariant; 4] = [
    ButtonVariant::Default,
    ButtonVariant::Primary,
    ButtonVariant::Danger,
    ButtonVariant::Outline,
];

#[test]
fn danger_variant_composes_with_custom_colors() {
    let colors = custom_colors();
    let mut tb = TestBackend::new(30, 3);
    tb.render(|ui| {
        let r = ui
            .button("Delete")
            .variant(ButtonVariant::Danger)
            .colors(&colors)
            .show();
        assert!(!r.clicked);
    });
    // Danger layout: no brackets, label padded by one filled space.
    tb.assert_contains("Delete");
    tb.assert_not_contains("[ Delete ]");
    let (x, y) = tb.find_text("Delete").expect("label rendered");
    // The only button holds focus: custom label fg, bold, on the custom fill,
    // padding included.
    let label = Style::new()
        .fg(Color::Rgb(250, 240, 230))
        .bg(Color::Rgb(120, 10, 30))
        .bold();
    tb.assert_style_at(x, y, label);
    tb.assert_style_at(x - 1, y, label);
    tb.assert_style_at(x + 6, y, label);

    // The same call without colors keeps the theme's Danger fill.
    let mut plain = TestBackend::new(30, 3);
    plain.render(|ui| {
        let _ = ui.button("Delete").variant(ButtonVariant::Danger);
    });
    let theme = slt::Theme::dark();
    let (x, y) = plain.find_text("Delete").expect("label rendered");
    plain.assert_style_at(x, y, Style::new().fg(theme.bg).bg(theme.error).bold());
}

#[test]
fn variant_and_colors_compose_in_either_order() {
    let colors = custom_colors();
    for variant in VARIANTS {
        assert_parity(
            &format!("{variant:?}"),
            &|ui| {
                let _ = ui.button("Save").variant(variant).colors(&colors);
            },
            &|ui| {
                let _ = ui.button("Save").colors(&colors).variant(variant);
            },
        );
    }
}

#[test]
fn deprecated_button_forwards_match_builders() {
    let colors = custom_colors();
    for variant in VARIANTS {
        assert_parity(
            &format!("button_with {variant:?}"),
            &|ui| {
                let _ = ui.button_with("Save", variant);
            },
            &|ui| {
                let _ = ui.button("Save").variant(variant);
            },
        );
    }
    assert_parity(
        "button_colored",
        &|ui| {
            let _ = ui.button_colored("Save", &colors);
        },
        &|ui| {
            let _ = ui.button("Save").colors(&colors);
        },
    );
}

#[test]
fn deprecated_display_forwards_match_builders() {
    let data = [1.0, 3.0, 2.0, 5.0, 4.0, 6.0, 3.0, 7.0];
    let bindings = [("q", "quit"), ("?", "help"), ("t", "theme")];
    assert_parity(
        "badge_colored",
        &|ui| {
            ui.badge_colored("NEW", Color::Magenta);
        },
        &|ui| {
            ui.badge("NEW").color(Color::Magenta);
        },
    );
    assert_parity(
        "stat_colored",
        &|ui| {
            ui.stat_colored("CPU", "72%", Color::Green);
        },
        &|ui| {
            ui.stat("CPU", "72%").color(Color::Green);
        },
    );
    assert_parity(
        "separator_colored",
        &|ui| {
            ui.separator_colored(Color::Cyan);
        },
        &|ui| {
            ui.separator().color(Color::Cyan);
        },
    );
    assert_parity(
        "progress_bar_colored",
        &|ui| {
            ui.progress_bar_colored(0.4, 30, Color::Yellow);
        },
        &|ui| {
            ui.progress_bar(0.4, 30).color(Color::Yellow);
        },
    );
    assert_parity(
        "help_colored",
        &|ui| {
            ui.help_colored(&bindings, Color::Red, Color::Blue);
        },
        &|ui| {
            ui.help(&bindings)
                .key_color(Color::Red)
                .text_color(Color::Blue);
        },
    );
    assert_parity(
        "line_chart_colored",
        &|ui| {
            ui.line_chart_colored(&data, 40, 4, Color::Cyan);
        },
        &|ui| {
            ui.line_chart(&data, 40, 4).color(Color::Cyan);
        },
    );
    assert_parity(
        "area_chart_colored",
        &|ui| {
            ui.area_chart_colored(&data, 40, 4, Color::Green);
        },
        &|ui| {
            ui.area_chart(&data, 40, 4).color(Color::Green);
        },
    );
    assert_parity(
        "grid_with",
        &|ui| {
            let _ = ui.grid_with(&[GridColumn::Fixed(8), GridColumn::Grow(1)], |ui| {
                ui.text("a");
                ui.text("b");
                ui.text("c");
            });
        },
        &|ui| {
            let _ = ui.grid_columns(&[GridColumn::Fixed(8), GridColumn::Grow(1)], |ui| {
                ui.text("a");
                ui.text("b");
                ui.text("c");
            });
        },
    );
}

#[test]
fn default_colors_match_the_plain_widgets() {
    // `.color(theme default)` is a no-op restyle of the plain widget.
    let theme = slt::Theme::dark();
    assert_parity(
        "badge",
        &|ui| {
            ui.badge("NEW");
        },
        &|ui| {
            ui.badge("NEW").color(theme.primary);
        },
    );
    assert_parity(
        "progress",
        &|ui| {
            ui.progress(0.5);
        },
        &|ui| {
            ui.progress_bar(0.5, 20).color(theme.primary);
        },
    );
    assert_parity(
        "help",
        &|ui| {
            ui.help(&[("q", "quit"), ("?", "help")]);
        },
        &|ui| {
            ui.help(&[("q", "quit"), ("?", "help")])
                .key_color(theme.primary)
                .text_color(theme.text_dim);
        },
    );
}

/// Press Right `presses` times on a focused slider built by `f`; return the
/// value and whether any frame reported `changed`.
fn press_right(presses: usize, f: &dyn Fn(&mut Context, &mut f64) -> Response) -> (f64, bool) {
    let mut tb = TestBackend::new(60, 3);
    let mut value = 30.0;
    let mut changed = false;
    tb.render_with_events(vec![], 0, 1, |ui| {
        f(ui, &mut value);
    });
    for _ in 0..presses {
        let events = EventBuilder::new().key_code(KeyCode::Right).build();
        tb.render_with_events(events, 0, 1, |ui| {
            changed |= f(ui, &mut value).changed;
        });
    }
    (value, changed)
}

#[test]
fn slider_step_matches_deprecated_forms() {
    let builder = press_right(2, &|ui, v| {
        ui.slider("Vol", v, 0.0..=100.0).step(7.0).show()
    });
    let opts = press_right(2, &|ui, v| {
        ui.slider_with(v, SliderOpts::new("Vol", 0.0..=100.0).step(7.0))
    });
    let step = press_right(2, &|ui, v| ui.slider_with_step("Vol", v, 0.0..=100.0, 7.0));
    assert_eq!(builder, (44.0, true));
    assert_eq!(opts, builder);
    assert_eq!(step, builder);
    // Default step is span / 20.
    let default = press_right(1, &|ui, v| ui.slider("Vol", v, 0.0..=100.0).show());
    assert_eq!(default, (35.0, true));
    assert_parity(
        "slider_with",
        &|ui| {
            let mut v = 30.0;
            let _ = ui.slider_with(&mut v, SliderOpts::new("Vol", 0.0..=100.0).step(7.0));
        },
        &|ui| {
            let mut v = 30.0;
            ui.slider("Vol", &mut v, 0.0..=100.0).step(7.0);
        },
    );
}

#[test]
fn slider_response_reads_record_once_in_place() {
    // Reading a field records the slider; the statement form records it at
    // the end of the statement. Both land before the following text.
    let mut tb = TestBackend::new(60, 4);
    tb.render(|ui| {
        let mut a = 10.0;
        let mut b = 20.0;
        if ui.slider("A", &mut a, 0.0..=100.0).changed {
            ui.text("never");
        }
        ui.slider("B", &mut b, 0.0..=100.0).step(1.0);
        ui.text("after");
    });
    assert!(tb.line(0).starts_with("A"), "{}", tb.to_string_trimmed());
    assert!(tb.line(1).starts_with("B"), "{}", tb.to_string_trimmed());
    assert!(
        tb.line(2).starts_with("after"),
        "{}",
        tb.to_string_trimmed()
    );
    tb.assert_not_contains("never");
}

#[test]
fn builders_keep_response_access_and_chains() {
    let mut tb = TestBackend::new(40, 6);
    let mut tips = 0;
    tb.render(|ui| {
        // Field access through Deref, `.on_hover(ui, ..)` through `Response: Copy`.
        let clicked = ui.button("Save").clicked;
        assert!(!clicked);
        let r = ui.button("Help").on_hover(ui, "tip");
        assert!(!r.clicked);
        ui.badge("NEW").on_hover(ui, "badge tip");
        let r: Response = ui.stat("CPU", "1%").show();
        tips += usize::from(r.hovered);
        let r = ui.separator().show();
        tips += usize::from(r.hovered);
    });
    assert_eq!(tips, 0);
}

#[test]
fn container_fg_matches_text_color() {
    let render = |use_fg: bool| {
        let mut tb = TestBackend::new(30, 3);
        tb.render(|ui| {
            let builder = ui.container().w_ratio(1, 3);
            let builder = if use_fg {
                builder.fg(Color::Red)
            } else {
                builder.text_color(Color::Red)
            };
            let _ = builder.col(|ui| {
                ui.text("abc");
            });
        });
        tb
    };
    let fg = render(true);
    let text_color = render(false);
    fg.assert_style_at(0, 0, Style::new().fg(Color::Red));
    assert_eq!(fg.snapshot(), text_color.snapshot());
    for x in 0..30 {
        assert_eq!(fg.buffer().get(x, 0), text_color.buffer().get(x, 0));
    }
}

#[test]
fn container_w_ratio_matches_constraints_w_ratio() {
    let probe = |via_builder: bool| {
        let mut tb = TestBackend::new(30, 3);
        let mut rect = slt::Rect::default();
        for _ in 0..2 {
            tb.render(|ui| {
                let builder = ui.container().border(Border::Single);
                let builder = if via_builder {
                    builder.w_ratio(1, 3).h_ratio(2, 3)
                } else {
                    builder.constraints(Constraints::default().w_ratio(1, 3).h_ratio(2, 3))
                };
                rect = builder
                    .col(|ui| {
                        ui.text("x");
                    })
                    .rect;
            });
        }
        (rect, tb.snapshot())
    };
    let (rect, snapshot) = probe(true);
    assert_eq!((rect.width, rect.height), (10, 2));
    assert_eq!((rect, snapshot), probe(false));
}

#[test]
fn context_line_returns_response() {
    let mut tb = TestBackend::new(20, 2);
    let mut width = 0;
    for _ in 0..2 {
        tb.render(|ui| {
            let r: Response = ui.line(|ui| {
                ui.text("ab");
                ui.text("cd");
            });
            width = r.rect.width;
        });
    }
    tb.assert_contains("abcd");
    assert!(width >= 4, "line response carries its rect: {width}");
}

/// Stand-in for an application's own `crossterm` dependency.
mod app_deps {
    pub mod crossterm {
        pub struct UserCrate;
    }
}

mod prelude_probe {
    // Two glob imports: if `slt::prelude` also exported `crossterm`, the name
    // below would be ambiguous (E0659) instead of resolving to the
    // application's own crate.
    use super::app_deps::*;
    use slt::prelude::*;

    pub fn resolves_to_user_crossterm(_ui: &mut Context) -> crossterm::UserCrate {
        crossterm::UserCrate
    }
}

#[test]
fn prelude_does_not_shadow_user_crossterm() {
    let mut tb = TestBackend::new(10, 1);
    tb.render(|ui| {
        let _marker = prelude_probe::resolves_to_user_crossterm(ui);
    });
    // The re-export stays reachable by path (#278).
    let _ = slt::crossterm::event::KeyCode::Enter;
}
