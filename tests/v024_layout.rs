use slt::widgets::ScrollState;
use slt::{Align, Buffer, Color, Rect, Style, TestBackend};

#[test]
fn nested_cross_axis_alignment_moves_descendants() {
    for (align, x, y) in [
        (Align::Start, 0, 0),
        (Align::Center, 8, 2),
        (Align::End, 16, 4),
    ] {
        let mut tb = TestBackend::new(20, 5);
        tb.render(|ui| {
            let _ = ui.container().grow(1).align(align).col(|ui| {
                let _ = ui.container().w(4).h(1).col(|ui| {
                    let _ = ui.col(|ui| {
                        ui.text("X");
                    });
                });
            });
        });
        assert_eq!(tb.buffer().get(x, 0).symbol.as_str(), "X", "{align:?}");
        tb.render(|ui| {
            let _ = ui.container().h(5).align(align).row(|ui| {
                let _ = ui.container().w(4).h(1).col(|ui| {
                    ui.text("Y");
                });
            });
        });
        assert_eq!(tb.buffer().get(0, y).symbol.as_str(), "Y", "{align:?}");
    }
}

#[test]
fn nested_wrapped_height_reserves_space_for_siblings() {
    let mut tb = TestBackend::new(4, 4);
    tb.render(|ui| {
        let _ = ui.col(|ui| {
            let _ = ui.col(|ui| {
                ui.text("aaaa bbbb").wrap();
            });
        });
        ui.text("NEXT");
    });
    assert_eq!(tb.line(0), "aaaa");
    assert_eq!(tb.line(1), "bbbb");
    assert_eq!(tb.line(2), "NEXT");
}

#[test]
fn horizontally_clipped_wide_grapheme_keeps_display_columns() {
    for text in ["\u{65e5}AB", "\u{1f469}\u{200d}\u{1f4bb}AB"] {
        for offset in 0..=2 {
            let mut tb = TestBackend::new(3, 2);
            let mut scroll = ScrollState::new();
            scroll.offset_x = offset;
            tb.render(|ui| {
                let _ = ui.scrollable(&mut scroll).w(3).h(2).row(|ui| {
                    ui.text(text);
                });
            });
            if offset == 1 {
                assert_eq!(tb.buffer().get(0, 0).symbol.as_str(), " ");
                assert_eq!(tb.buffer().get(1, 0).symbol.as_str(), "A");
                assert_eq!(tb.buffer().get(2, 0).symbol.as_str(), "B");
            }
        }
    }
}

#[test]
fn precomputed_source_is_cropped_on_both_axes() {
    let mut tb = TestBackend::new(2, 2);
    let mut vertical = ScrollState::new();
    vertical.offset = 1;
    let mut horizontal = ScrollState::new();
    horizontal.offset_x = 1;
    tb.render(|ui| {
        let _ = ui.scrollable(&mut vertical).w(2).h(2).col(|ui| {
            let _ = ui.scrollable(&mut horizontal).w(2).h(3).row(|ui| {
                ui.container()
                    .w(3)
                    .h(3)
                    .draw_precomputed(3, 3, |buffer, _| {
                        for (y, text) in ["ABC", "DEF", "GHI"].into_iter().enumerate() {
                            buffer.set_string_linked(
                                0,
                                y as u32,
                                text,
                                Style::new().fg(Color::Cyan),
                                "https://example.com",
                            );
                        }
                    })
                    .unwrap();
            });
        });
    });
    assert_eq!(tb.line(0), "EF");
    assert_eq!(tb.line(1), "HI");
    assert_eq!(tb.buffer().get(0, 0).style.fg, Some(Color::Cyan));
    assert_eq!(
        tb.buffer().get(0, 0).hyperlink.as_deref(),
        Some("https://example.com")
    );
}

#[test]
fn invalid_zero_width_suffix_coordinates_never_mutate() {
    for origin in [0, 7, 100] {
        for width in 0..=4 {
            for height in 0..=2 {
                let area = Rect::new(origin, origin, width, height);
                let mut buffer = Buffer::empty(area);
                if width > 0 && height > 0 {
                    buffer.set_string(origin, origin, "ABCD", Style::new());
                }
                let before = buffer.content.clone();
                for x in [0, origin, area.right().saturating_add(1), u32::MAX] {
                    for y in [origin, area.bottom(), u32::MAX] {
                        buffer.set_char(x, y, '\u{301}', Style::new());
                    }
                }
                assert_eq!(buffer.content, before, "{area:?}");
            }
        }
    }
}

#[test]
fn suffix_at_exclusive_right_edge_extends_the_last_grapheme() {
    let mut buffer = Buffer::empty(Rect::new(5, 7, 3, 1));
    buffer.set_string(5, 7, "ABC", Style::new());
    buffer.set_char(8, 7, '\u{301}', Style::new());
    assert_eq!(buffer.get(7, 7).symbol.as_str(), "C\u{301}");
}

#[test]
fn canvas_retains_requested_coordinates_under_parent_clipping() {
    let mut tb = TestBackend::new(10, 5);
    let mut dimensions = (0, 0);
    tb.render(|ui| {
        let _ = ui.container().w(3).h(2).col(|ui| {
            let _ = ui.canvas(20, 10, |canvas| {
                dimensions = (canvas.width(), canvas.height());
                canvas.dot(usize::MAX, usize::MAX);
                canvas.print(usize::MAX, usize::MAX, "OUTSIDE");
                canvas.print(0, 0, "ABCDE");
                canvas.print(0, 4, "FGHIJ");
                canvas.print(0, 8, "KLMNO");
            });
        });
    });
    assert_eq!(dimensions, (40, 40));
    assert_eq!(tb.line(0), "ABC");
    assert_eq!(tb.line(1), "FGH");
    assert!(!tb.to_string().contains("OUTSIDE"));
    assert!(!tb.to_string().contains('K'));
    let mut calls = 0;
    tb.render(|ui| {
        let _ = ui.canvas(0, 10, |_| calls += 1);
    });
    assert_eq!(calls, 0);
}

#[test]
fn aligned_borders_margins_raw_draw_and_hit_rect_move_together() {
    use std::cell::Cell;
    use std::rc::Rc;
    let mut tb = TestBackend::new(20, 8);
    let drawn = Rc::new(Cell::new(Rect::default()));
    let mut hit = Rect::default();
    for _ in 0..2 {
        tb.render(|ui| {
            let _ = ui.container().grow(1).col(|ui| {
                let _ = ui
                    .container()
                    .w(6)
                    .h(5)
                    .ml(1)
                    .mr(3)
                    .align_self(Align::End)
                    .border(slt::Border::Single)
                    .p(1)
                    .col(|ui| {
                        let drawn = Rc::clone(&drawn);
                        let response =
                            ui.container().w(2).h(1).draw_interactive(move |buf, rect| {
                                drawn.set(rect);
                                buf.set_string(rect.x, rect.y, "XY", Style::new());
                            });
                        hit = response.rect;
                    });
            });
        });
    }
    assert_eq!(drawn.get(), Rect::new(13, 2, 2, 1));
    assert_eq!(hit, drawn.get());
    assert_eq!(tb.buffer().get(11, 0).symbol.as_str(), "\u{250c}");
    assert_eq!(tb.buffer().get(13, 2).symbol.as_str(), "X");
}

#[test]
fn nested_wrap_respects_borders_padding_margins_and_height_constraints() {
    let mut tb = TestBackend::new(14, 12);
    for width in [10, 14, 10] {
        tb.render(|ui| {
            let _ = ui
                .container()
                .w(width)
                .border(slt::Border::Single)
                .p(1)
                .m(1)
                .col(|ui| {
                    let _ = ui.col(|ui| {
                        ui.text("aaaa bbbb").wrap();
                    });
                });
            ui.text("NEXT");
        });
        let lines = if width == 10 { 2 } else { 1 };
        assert_eq!(tb.buffer().get(3, 3).symbol.as_str(), "a");
        if lines == 2 {
            assert_eq!(tb.buffer().get(3, 4).symbol.as_str(), "b");
        }
        assert_eq!(tb.line(6 + lines), "NEXT");
    }
    for (minimum, maximum, expected_next) in [(0, 1, 1), (4, 6, 4)] {
        tb.render(|ui| {
            let _ = ui.container().w(4).min_h(minimum).max_h(maximum).col(|ui| {
                ui.text("aaaa bbbb").wrap();
            });
            ui.text("NEXT");
        });
        assert_eq!(tb.line(expected_next), "NEXT");
        if maximum == 1 {
            assert!(!tb.to_string().contains("bbbb"));
        }
    }
}

#[test]
fn row_intrinsic_height_uses_grow_and_fixed_child_widths() {
    let mut tb = TestBackend::new(9, 8);
    tb.render(|ui| {
        let _ = ui.container().gap(1).row(|ui| {
            let _ = ui.container().grow(1).col(|ui| {
                ui.text("aaaa bbbb").wrap();
            });
            let _ = ui.container().w(4).col(|ui| {
                ui.text("cccc dddd").wrap();
            });
        });
        ui.text("NEXT");
    });
    assert_eq!(tb.line(0), "aaaa cccc");
    assert_eq!(tb.line(1), "bbbb dddd");
    assert_eq!(tb.line(2), "NEXT");
}

#[test]
fn flex_wrap_measures_wrapped_descendants_per_line() {
    let mut tb = TestBackend::new(7, 8);
    tb.render(|ui| {
        let _ = ui.container().wrap().row(|ui| {
            for text in ["aaaa bbbb", "cccc dddd"] {
                let _ = ui.container().w(4).col(|ui| {
                    ui.text(text).wrap();
                });
            }
        });
        ui.text("NEXT");
    });
    for (y, text) in ["aaaa", "bbbb", "cccc", "dddd", "NEXT"]
        .into_iter()
        .enumerate()
    {
        assert_eq!(tb.line(y as u32), text);
    }
}

#[test]
fn nonzero_origin_partial_wide_clip_preserves_link_and_style() {
    let mut tb = TestBackend::new(10, 3);
    let mut scroll = ScrollState::new();
    scroll.offset_x = 1;
    tb.render(|ui| {
        let _ = ui.scrollable(&mut scroll).ml(3).w(3).h(2).row(|ui| {
            ui.link("\u{65e5}AB", "https://example.com").fg(Color::Red);
        });
    });
    assert_eq!(tb.buffer().get(3, 0).symbol.as_str(), " ");
    assert_eq!(tb.buffer().get(4, 0).symbol.as_str(), "A");
    assert_eq!(tb.buffer().get(4, 0).style.fg, Some(Color::Red));
    assert_eq!(
        tb.buffer().get(4, 0).hyperlink.as_deref(),
        Some("https://example.com")
    );
    assert_eq!(tb.buffer().get(5, 0).symbol.as_str(), "B");
}

#[test]
fn precomputed_wide_cells_crop_without_shifting_adjacent_regions() {
    let mut tb = TestBackend::new(8, 3);
    let mut scroll = ScrollState::new();
    scroll.offset_x = 1;
    let source = String::from("\u{65e5}ABC");
    tb.render(|ui| {
        let _ = ui.row(|ui| {
            let _ = ui.scrollable(&mut scroll).w(3).h(2).row(|ui| {
                ui.container()
                    .w(5)
                    .h(2)
                    .draw_precomputed(5, 2, |buf, _| {
                        buf.set_string(0, 0, &source, Style::new().fg(Color::Green));
                    })
                    .unwrap();
            });
            ui.container()
                .w(2)
                .h(1)
                .draw_precomputed(2, 1, |buf, _| {
                    buf.set_string(0, 0, "OK", Style::new());
                })
                .unwrap();
        });
    });
    assert_eq!(tb.line(0), " ABOK");
    assert_eq!(tb.buffer().get(1, 0).style.fg, Some(Color::Green));
}

#[test]
fn precomputed_copy_revalidates_directly_mutated_hyperlinks() {
    let mut tb = TestBackend::new(2, 1);
    tb.render(|ui| {
        ui.container()
            .w(2)
            .h(1)
            .draw_precomputed(2, 1, |buf, _| {
                buf.set_string(0, 0, "OK", Style::new());
                buf.get_mut(0, 0).hyperlink = Some("https://example.com\x1b]52;c;payload".into());
            })
            .unwrap();
    });
    assert_eq!(tb.line(0), "OK");
    assert!(tb.buffer().get(0, 0).hyperlink.is_none());
}

#[test]
fn zero_width_suffix_respects_entire_wide_grapheme_clip() {
    let mut buf = Buffer::empty(Rect::new(5, 7, 3, 1));
    buf.set_string(5, 7, "\u{65e5}X", Style::new());
    let before = buf.content.clone();
    buf.push_clip(Rect::new(5, 7, 1, 1));
    buf.set_char(7, 7, '\u{301}', Style::new());
    assert_eq!(buf.content, before);
    buf.push_clip(Rect::new(6, 7, 1, 1));
    buf.set_char(7, 7, '\u{301}', Style::new());
    assert_eq!(buf.content, before);
    buf.pop_clip();
    buf.pop_clip();
    buf.set_char(7, 7, '\u{301}', Style::new());
    assert_eq!(buf.get(5, 7).symbol.as_str(), "\u{65e5}\u{301}");
}

#[test]
fn canvas_rejects_budget_requests_without_changing_existing_content() {
    use slt::context::{CanvasContext, CanvasError};
    for (cols, rows) in [(u32::MAX, 1), (1, u32::MAX), (513, 512)] {
        assert!(matches!(
            CanvasContext::try_new(cols, rows),
            Err(CanvasError::GeometryBudgetExceeded)
        ));
    }
    let mut canvas = CanvasContext::try_new(2, 1).unwrap();
    for _ in 1..CanvasContext::MAX_LAYERS {
        canvas.try_layer().unwrap();
    }
    assert_eq!(canvas.try_layer(), Err(CanvasError::LayerBudgetExceeded));
    let huge = "a".repeat(CanvasContext::MAX_LABEL_BYTES + 1);
    assert_eq!(
        canvas.try_print(0, 0, &huge),
        Err(CanvasError::LabelBudgetExceeded)
    );
    assert_eq!(canvas.try_print(usize::MAX, usize::MAX, &huge), Ok(()));
    let mut tb = TestBackend::new(8, 2);
    let mut called = false;
    tb.render(|ui| {
        assert!(matches!(
            ui.try_canvas(u32::MAX, 2, |_| called = true),
            Err(CanvasError::GeometryBudgetExceeded)
        ));
        ui.text("OK");
    });
    assert!(!called);
    assert_eq!(tb.line(0), "OK");
}

#[test]
fn canvas_reuse_clears_old_layers_and_labels_and_resets_color() {
    let mut canvas = slt::CanvasContext::try_new(4, 2).unwrap();
    let mut tb = TestBackend::new(4, 2);
    tb.render(|ui| {
        let _ = ui.canvas_with(&mut canvas, |canvas| {
            canvas.layer();
            canvas.set_color(Color::Red);
            canvas.filled_rect(0, 0, 8, 8);
            canvas.print(0, 0, "OLD");
        });
    });
    tb.render(|ui| {
        let _ = ui.canvas_with(&mut canvas, |canvas| {
            assert_eq!(canvas.color(), Color::Reset);
            canvas.print(0, 0, "NEW");
        });
    });
    assert!(!tb.to_string().contains("OLD"));
    assert_eq!(tb.buffer().get(0, 0).symbol.as_str(), "N");
    assert_eq!(tb.buffer().get(0, 1).symbol.as_str(), "\u{2800}");
}

mod measurements {
    use super::*;
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};

    struct CountingAllocator;
    static ENABLED: AtomicBool = AtomicBool::new(false);
    static LIVE: AtomicUsize = AtomicUsize::new(0);
    static BASE: AtomicUsize = AtomicUsize::new(0);
    static PEAK: AtomicUsize = AtomicUsize::new(0);
    static REQUESTED: AtomicUsize = AtomicUsize::new(0);
    static ALLOCS: AtomicUsize = AtomicUsize::new(0);

    fn record(size: usize) {
        let live = LIVE.fetch_add(size, Relaxed) + size;
        if ENABLED.load(Relaxed) {
            PEAK.fetch_max(live, Relaxed);
            REQUESTED.fetch_add(size, Relaxed);
            ALLOCS.fetch_add(1, Relaxed);
        }
    }

    unsafe impl GlobalAlloc for CountingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let pointer = unsafe { System.alloc(layout) };
            if !pointer.is_null() {
                record(layout.size());
            }
            pointer
        }
        unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
            LIVE.fetch_sub(layout.size(), Relaxed);
            unsafe {
                System.dealloc(pointer, layout);
            }
        }
        unsafe fn realloc(&self, pointer: *mut u8, old: Layout, size: usize) -> *mut u8 {
            let result = unsafe { System.realloc(pointer, old, size) };
            if !result.is_null() {
                LIVE.fetch_sub(old.size(), Relaxed);
                record(size);
            }
            result
        }
    }

    #[global_allocator]
    static ALLOCATOR: CountingAllocator = CountingAllocator;

    fn start() {
        let base = LIVE.load(Relaxed);
        BASE.store(base, Relaxed);
        PEAK.store(base, Relaxed);
        REQUESTED.store(0, Relaxed);
        ALLOCS.store(0, Relaxed);
        ENABLED.store(true, Relaxed);
    }

    fn stop() -> (usize, usize, usize, usize) {
        ENABLED.store(false, Relaxed);
        (
            ALLOCS.load(Relaxed),
            REQUESTED.load(Relaxed),
            PEAK.load(Relaxed).saturating_sub(BASE.load(Relaxed)),
            LIVE.load(Relaxed).saturating_sub(BASE.load(Relaxed)),
        )
    }

    #[test]
    #[ignore = "release measurement; run alone with --ignored --nocapture --test-threads=1"]
    fn canvas_release_measurement() {
        for (width, height, layers) in [(80, 24, 1), (80, 24, 8), (240, 80, 8)] {
            let mut tb = TestBackend::new(width, height);
            let draw = |ui: &mut slt::Context| {
                let _ = ui.canvas(width, height, |canvas| {
                    for layer in 0..layers {
                        if layer > 0 {
                            canvas.layer();
                        }
                        canvas.set_color(if layer % 2 == 0 {
                            Color::Red
                        } else {
                            Color::Green
                        });
                        canvas.line(0, layer, canvas.width() - 1, canvas.height() - 1 - layer);
                        canvas.print(0, layer * 4, "Canvas benchmark");
                    }
                });
            };
            for _ in 0..10 {
                tb.render(draw);
            }
            start();
            tb.render(draw);
            let allocation = stop();
            let start = std::time::Instant::now();
            for _ in 0..100 {
                tb.render(draw);
                std::hint::black_box(tb.buffer());
            }
            let elapsed = start.elapsed().as_micros() / 100;
            use std::hash::{Hash, Hasher};
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            tb.to_string().hash(&mut hash);
            println!(
                "canvas {width}x{height} layers={layers}: {elapsed} us/frame allocs={} requested={} peak_extra={} retained_extra={} hash={:016x}",
                allocation.0,
                allocation.1,
                allocation.2,
                allocation.3,
                hash.finish()
            );
        }
    }

    #[test]
    #[ignore = "release measurement; run alone with --ignored --nocapture --test-threads=1"]
    fn canvas_reuse_release_measurement() {
        for (width, height, layers) in [(80, 24, 1), (80, 24, 8), (240, 80, 8)] {
            let mut tb = TestBackend::new(width, height);
            let paint = |canvas: &mut slt::CanvasContext| {
                for layer in 0..layers {
                    if layer > 0 {
                        canvas.layer();
                    }
                    canvas.set_color(if layer % 2 == 0 {
                        Color::Red
                    } else {
                        Color::Green
                    });
                    canvas.line(0, layer, canvas.width() - 1, canvas.height() - 1 - layer);
                    canvas.print(0, layer * 4, "Canvas benchmark");
                }
            };
            for _ in 0..10 {
                tb.render(|ui| {
                    let _ = ui.canvas(width, height, paint);
                });
            }
            let expected = tb.buffer().content.clone();
            let before_canvas = LIVE.load(Relaxed);
            let mut canvas = slt::CanvasContext::try_new(width, height).unwrap();
            for _ in 0..10 {
                tb.render(|ui| {
                    let _ = ui.canvas_with(&mut canvas, paint);
                });
            }
            let retained = LIVE.load(Relaxed).saturating_sub(before_canvas);
            assert_eq!(
                tb.buffer().content,
                expected,
                "reused text/style/link output differs"
            );
            start();
            tb.render(|ui| {
                let _ = ui.canvas_with(&mut canvas, paint);
            });
            let allocation = stop();
            let start = std::time::Instant::now();
            for _ in 0..100 {
                tb.render(|ui| {
                    let _ = ui.canvas_with(&mut canvas, paint);
                });
                std::hint::black_box(tb.buffer());
            }
            let elapsed = start.elapsed().as_micros() / 100;
            println!(
                "canvas reuse {width}x{height} layers={layers}: {elapsed} us/frame allocs={} requested={} peak_extra={} retained_extra={} backing_retained={retained}",
                allocation.0, allocation.1, allocation.2, allocation.3
            );
        }
    }

    fn nested(ui: &mut slt::Context, depth: usize) {
        if depth == 0 {
            ui.text("aaaa bbbb cccc dddd").wrap();
        } else {
            let _ = ui.col(|ui| nested(ui, depth - 1));
        }
    }

    #[test]
    #[ignore = "release measurement; run alone with --ignored --nocapture --test-threads=1"]
    fn deep_layout_release_measurement() {
        for depth in [16, 64, 128] {
            let mut tb = TestBackend::new(4, 8);
            for _ in 0..5 {
                tb.render(|ui| nested(ui, depth));
            }
            let start = std::time::Instant::now();
            for _ in 0..200 {
                tb.render(|ui| nested(ui, depth));
            }
            println!(
                "deep layout depth={depth}: {} us/frame lines={:?}",
                start.elapsed().as_micros() / 200,
                tb.to_string()
            );
        }
    }
}
