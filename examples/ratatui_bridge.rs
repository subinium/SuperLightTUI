//! Render stock ratatui widgets inside SLT containers (`ratatui` feature).
//!
//! Run with: `cargo run --example ratatui_bridge --features ratatui`
//!
//! SLT owns the layout, theme, input and scrolling; ratatui widgets paint
//! into the laid-out regions. Up/Down (or the mouse wheel) scroll the lower
//! pane, which is a single ratatui `Paragraph` taller than its viewport.

use ratatui_widgets::block::Block;
use ratatui_widgets::gauge::Gauge;
use ratatui_widgets::paragraph::Paragraph;
use ratatui_widgets::sparkline::Sparkline;
use slt::ratatui_compat::ratatui_core::style::{Color as RatColor, Style as RatStyle};
use slt::widgets::ScrollState;
use slt::{Border, Color, Context, KeyCode, RunConfig};

const SAMPLES: usize = 48;
const LOG_LINES: usize = 40;

fn main() -> std::io::Result<()> {
    let mut scroll = ScrollState::new();
    slt::run_with(RunConfig::default(), move |ui: &mut Context| {
        if ui.key('q') || ui.key_code(KeyCode::Esc) {
            ui.quit();
            return;
        }
        if ui.key_code(KeyCode::Up) {
            scroll.scroll_up(1);
        }
        if ui.key_code(KeyCode::Down) {
            scroll.scroll_down(1);
        }
        let tick = ui.tick();

        ui.bordered(Border::Rounded)
            .title("ratatui widgets inside SLT")
            .p(1)
            .gap(1)
            .col(|ui| {
                ui.text("SLT layout + ratatui-core Widget rendering. q to quit.")
                    .fg(Color::Cyan);

                ui.container().gap(1).row(|ui| {
                    let percent = (tick % 101) as u16;
                    ui.container().grow(1).h(3).ratatui_widget(
                        Gauge::default()
                            .block(Block::bordered().title("Gauge"))
                            .gauge_style(RatStyle::new().fg(RatColor::Green))
                            .percent(percent),
                    );

                    let data: Vec<u64> = (0..SAMPLES as u64)
                        .map(|i| ((i + tick) * 7 % 13) + 1)
                        .collect();
                    ui.container().grow(1).h(3).ratatui(move |area, buf| {
                        use slt::ratatui_compat::ratatui_core::widgets::Widget;
                        Sparkline::default()
                            .block(Block::bordered().title("Sparkline"))
                            .style(RatStyle::new().fg(RatColor::Yellow))
                            .data(&data)
                            .render(area, buf);
                    });
                });

                let log: String = (1..=LOG_LINES)
                    .map(|i| format!("line {i:02}: ratatui Paragraph, scrolled by SLT\n"))
                    .collect();
                ui.scrollable(&mut scroll).grow(1).col(|ui| {
                    ui.container().h(LOG_LINES as u32 + 2).ratatui_widget(
                        Paragraph::new(log).block(Block::bordered().title("Scroll me")),
                    );
                });
            });
    })
}
