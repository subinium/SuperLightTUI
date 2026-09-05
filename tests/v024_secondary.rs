use slt::chart::{ChartRenderer, RenderedLine};
use slt::{ChartBuilder, Color, LegendPosition, LoopMode, Sequence, Spring, Style, TestBackend};
use std::cell::Cell;
use std::rc::Rc;
use unicode_width::UnicodeWidthStr;

fn bars(data: &[(f64, f64)], height: u32) -> Vec<RenderedLine> {
    let mut builder = ChartBuilder::new(12, height, Style::new(), Style::new());
    builder
        .grid(false)
        .legend(LegendPosition::None)
        .x_axis_visible(false)
        .y_axis_visible(false)
        .ylim(-1.0, 1.0)
        .yticks(&[-1.0, 0.0, 1.0]);
    builder.bar(data).color(Color::Red);
    ChartRenderer::new(builder.build()).render()
}

#[test]
fn zero_bars_are_empty_but_quantized_nonzero_bars_remain_visible() {
    for height in [1, 6] {
        assert_eq!(bars(&[(0.0, 0.0), (1.0, -0.0)], height), bars(&[], height));
        for value in [-1.0, -1e-12, 1e-12, 1.0] {
            let rows = bars(&[(0.0, value), (1.0, 0.0)], height);
            assert!(rows.iter().any(|(line, _)| line.contains('\u{2588}')));
            assert!(
                rows.iter()
                    .all(|(line, _)| line.chars().skip(6).all(|c| c == ' '))
            );
        }
    }
}

#[test]
fn bar_placement_is_ordinal_not_numeric() {
    let ordered = bars(&[(0.0, 1.0), (1.0, 0.0), (100.0, -1.0)], 6);
    assert_eq!(ordered, bars(&[(100.0, 1.0), (-50.0, 0.0), (0.0, -1.0)], 6));
    assert_ne!(ordered, bars(&[(100.0, -1.0), (1.0, 0.0), (0.0, 1.0)], 6));
}

#[test]
fn empty_histogram_bin_has_no_blocks() {
    let mut backend = TestBackend::new(36, 10);
    backend.render(|ui| {
        let _ = ui.histogram_with(
            &[0.0, 10.0],
            |h| {
                h.bins(3);
            },
            36,
            10,
        );
    });
    assert!((0..9).all(|y| backend.buffer().get(18, y).symbol.as_str() != "\u{2588}"));
    backend.assert_contains("\u{2588}");
}

#[test]
fn histogram_keeps_final_edge_label() {
    for bins in [8, 9, 10, 11] {
        let mut backend = TestBackend::new(60, 10);
        backend.render(|ui| {
            let _ = ui.histogram_with(
                &[0.0, 9.0],
                |h| {
                    h.bins(bins);
                },
                60,
                10,
            );
        });
        assert!(
            backend.line(9).trim_end().ends_with('9')
                || backend.line(9).trim_end().ends_with("9.00"),
            "bins={bins}: {}",
            backend.line(9)
        );
    }
}

#[test]
fn narrow_histogram_keeps_whole_endpoint_labels() {
    for width in [14, 20] {
        let mut backend = TestBackend::new(width, 10);
        backend.render(|ui| {
            let _ = ui.histogram_with(
                &[-0.75, 2.25],
                |h| {
                    h.bins(11);
                },
                width,
                10,
            );
        });
        assert!(
            backend.line(9).trim_end().ends_with("2.25"),
            "{}",
            backend.line(9)
        );
        assert!(backend.line(9).width() <= width as usize);
    }
}

#[test]
fn legends_keep_graphemes_width_borders_and_color_spans() {
    for position in [
        LegendPosition::TopLeft,
        LegendPosition::BottomLeft,
        LegendPosition::TopRight,
        LegendPosition::BottomRight,
    ] {
        for name in [
            "AB",
            "\u{d55c}\u{ae00}",
            "e\u{301}",
            "\u{1f469}\u{200d}\u{1f4bb}",
        ] {
            for width in [8, 24] {
                let style = Style::new().fg(Color::White);
                let mut builder = ChartBuilder::new(width, 8, style, style);
                builder
                    .grid(false)
                    .frame(true)
                    .x_axis_visible(false)
                    .y_axis_visible(false)
                    .legend(position);
                builder.scatter(&[]).label(name).color(Color::Red);
                let rows = ChartRenderer::new(builder.build()).render();
                for (line, spans) in &rows {
                    assert_eq!(line.width(), width as usize, "{position:?}: {line:?}");
                    assert!(
                        spans
                            .iter()
                            .all(|(start, end, _)| start <= end && *end <= width as usize)
                    );
                }
                if width == 24 {
                    let (line, spans) = rows
                        .iter()
                        .find(|(line, _)| line.contains(name))
                        .expect("complete legend grapheme");
                    assert!(line.starts_with('\u{2502}') && line.ends_with('\u{2502}'));
                    let marker = line.find('\u{28ff}').unwrap();
                    let col = line[..marker].width();
                    assert!(spans.iter().any(|(start, end, color)| *start <= col
                        && col < *end
                        && *color == Color::Red));
                }
            }
        }
    }
}

#[test]
fn spring_same_target_does_not_rearm_or_change_trajectory() {
    let hits = Rc::new(Cell::new(0));
    let count = Rc::clone(&hits);
    let mut repeated = Spring::new(0.0, 0.2, 0.85).on_settle(move || count.set(count.get() + 1));
    let mut once = Spring::new(0.0, 0.2, 0.85);
    for target in [1.0, -1.0, 2.0] {
        once.set_target(target);
        for _ in 0..500 {
            repeated.set_target(target);
            repeated.tick();
            once.tick();
            assert_eq!(repeated.value(), once.value());
        }
    }
    assert_eq!(hits.get(), 3);
}

#[test]
fn spring_near_target_change_already_settled_needs_no_callback() {
    let hits = Rc::new(Cell::new(0));
    let count = Rc::clone(&hits);
    let mut spring = Spring::new(1.0, 0.2, 0.85).on_settle(move || count.set(count.get() + 1));
    spring.set_target(1.001);
    for _ in 0..100 {
        spring.set_target(1.001);
        spring.tick();
    }
    assert_eq!(hits.get(), 0);
    spring.set_target(1.1);
    for _ in 0..500 {
        spring.tick();
    }
    assert_eq!(hits.get(), 1);
}

#[test]
fn sequence_duration_handles_append_zero_overflow_and_loops() {
    let linear = slt::anim::ease_linear;
    let mut sequence = Sequence::new().then(0.0, 10.0, 10, linear);
    assert_eq!(sequence.value(10), 10.0);
    sequence = sequence
        .then(10.0, 10.0, 0, linear)
        .then(10.0, 20.0, 10, linear);
    sequence.reset(0);
    assert_eq!(sequence.value(15), 15.0);
    for (mode, expected) in [(LoopMode::Repeat, 5.0), (LoopMode::PingPong, 15.0)] {
        sequence = sequence.loop_mode(mode);
        sequence.reset(0);
        assert_eq!(sequence.value(25), expected);
    }
    let mut huge = Sequence::new()
        .then(0.0, 1.0, u64::MAX, linear)
        .then(1.0, 2.0, 1, linear);
    assert_eq!(huge.value(u64::MAX), 2.0);
}

#[cfg(feature = "syntax-cpp")]
#[test]
fn cpp_feature_styles_base_c_tokens_and_preserves_source() {
    let theme = slt::Theme::dark();
    let source = "// cbase\nint main(){ return 0; }\nconst char* s = \"text\";";
    let lines = slt::syntax::highlight_code(source, "cpp", &theme).unwrap();
    assert_eq!(
        lines
            .iter()
            .map(|line| line
                .iter()
                .map(|(text, _)| text.as_str())
                .collect::<String>())
            .collect::<Vec<_>>()
            .join("\n"),
        source
    );
    for token in ["// cbase", "int", "return", "0", "\"text\""] {
        assert!(
            lines
                .iter()
                .flatten()
                .any(|(text, style)| text.contains(token) && style.fg != Some(theme.text)),
            "unstyled token: {token}; {lines:?}"
        );
    }
    for alias in ["c++", "cxx", "cc", "hpp"] {
        assert_eq!(
            slt::syntax::highlight_code(source, alias, &theme).unwrap(),
            lines
        );
    }
    #[cfg(not(feature = "syntax-c"))]
    assert!(slt::syntax::highlight_code(source, "c", &theme).is_none());
}

#[test]
fn cached_is_diagnostics_only_and_executes_every_frame() {
    let mut cached = TestBackend::new(30, 8);
    let mut plain = TestBackend::new(30, 8);
    let mut calls = 0;
    let mut hits = Vec::new();
    for frame in 0..4 {
        cached.render(|ui| {
            let _ = ui.container().cached(7 + frame / 2, |ui| {
                calls += 1;
                ui.text("static");
            });
            hits.push(ui.region_cache_hits());
            ui.text(format!("frame {frame}"));
        });
        plain.render(|ui| {
            let _ = ui.container().col(|ui| {
                ui.text("static");
            });
            ui.text(format!("frame {frame}"));
        });
        assert_eq!(cached.to_string(), plain.to_string());
    }
    assert_eq!(calls, 4);
    assert_eq!(hits, [0, 1, 0, 1]);
}
