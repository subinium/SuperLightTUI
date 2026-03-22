# Widget Guide

This is the high-level widget map.
Use it to decide which part of the API to reach for before opening docs.rs or source files.

## Core rules

- Display-oriented methods usually return `&mut Context` for style chaining.
- Interactive widgets usually return `Response` and keep their own `*State` in `src/widgets.rs`.
- Layout is built with `row()`, `col()`, `grid()`, `spacer()`, `grow()`, and container builders.

## Text and display

```rust
ui.text("Hello").bold().fg(Color::Cyan);
ui.styled("inline", Style::new().underline());
ui.link("Docs", "https://docs.rs/superlighttui");
ui.markdown("# Heading\n\n**Bold** text");
ui.code_block_lang("fn main() {}", "rust");
ui.big_text("SLT");
ui.timer_display(elapsed);
```

Use these when you mostly need output, formatting, and rich text.

## Layout and containers

```rust
ui.row(|ui| {});
ui.col(|ui| {});
ui.grid(3, |ui| {});
ui.scrollable(&mut scroll).col(|ui| {});
ui.bordered(Border::Rounded).title("Panel").p(1).col(|ui| {});
ui.modal(|ui| {});
ui.overlay(|ui| {});
ui.screen("home", &screens, |ui| {});
```

These are the pieces that define structure, not data.

## Actions and form input

```rust
if ui.button("Submit").clicked {}
ui.checkbox("Dark mode", &mut dark);
ui.toggle("Notifications", &mut enabled);
ui.text_input(&mut input);
ui.textarea(&mut notes, 5);
ui.slider("Volume", &mut volume, 0.0..=100.0);
ui.form_field(&mut field);
ui.confirm("Delete?", &mut yes);
```

These are the widgets you reach for first in CRUD-style or command-driven UIs.

## Choice and navigation

```rust
ui.tabs(&mut tabs);
ui.list(&mut list);
ui.select(&mut select);
ui.radio(&mut radio);
ui.multi_select(&mut multi);
ui.tree(&mut tree);
ui.directory_tree(&mut directory_tree);
ui.calendar(&mut calendar);
ui.command_palette(&mut palette);
ui.breadcrumb(&["Home", "Settings"]);
```

These widgets help users move through data or switch views.

## Data and feedback

```rust
ui.table(&mut table);
ui.virtual_list(&mut list, 20, |ui, i| {});
ui.progress(0.75);
ui.spinner(&spinner);
ui.toast(&mut toasts);
ui.alert("Saved!", AlertLevel::Success);
ui.stat("Users", "1,234");
ui.empty_state("No results", "Try a different search");
ui.help(&[("q", "quit")]);
```

Use these for dashboards, tools, and status-heavy UIs.

## Visualization

```rust
ui.chart(|c| {
    c.line(&data);
    c.grid(true);
}, 50, 16);
ui.line_chart(&data, 50, 16);
ui.area_chart(&data, 50, 16);
ui.scatter(&points, 50, 16);
ui.histogram(&values, 40, 12);
ui.bar_chart(&bars, 24);
ui.bar_chart_grouped(&groups, 24);
ui.sparkline(&values, 16);
ui.heatmap(&grid, 40, 10, lo, hi);
ui.canvas(40, 10, |cv| {
    cv.circle(20, 20, 15);
});
ui.candlestick(&candles, up_color, down_color);
```

This is the most visualization-heavy part of the library.

## AI-native and rich terminal output

```rust
ui.streaming_text(&mut stream);
ui.streaming_markdown(&mut md_stream);
ui.tool_approval(&mut tool);
ui.context_bar(&items);
ui.image(&img);
ui.sixel_image(&rgba, w, h, cols, rows);
ui.kitty_image(&rgba, pw, ph, cols, rows);
ui.qr_code("https://example.com");
```

These APIs support terminals that need richer output than plain text widgets.

## Where to look in the codebase

- `src/context/widgets_display.rs` - display/layout facade
- `src/context/widgets_display/` - text, rich output, status, layout/container subfiles
- `src/context/widgets_input.rs` - input facade
- `src/context/widgets_input/` - text input, feedback, textarea/progress subfiles
- `src/context/widgets_interactive.rs` - interactive facade
- `src/context/widgets_interactive/` - collections, selection, rich markdown, events, tree widget subfiles
- `src/context/widgets_viz.rs` - chart and visualization widgets
- `src/widgets.rs` - public state catalog facade
- `src/widgets/` - grouped `*State` definitions

If you are contributing a new widget, read `CONTRIBUTING.md`, `docs/DESIGN_PRINCIPLES.md`, and `docs/ARCHITECTURE.md` first.
