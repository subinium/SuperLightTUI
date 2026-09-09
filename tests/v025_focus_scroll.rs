use slt::context::ModalOptions;
use slt::{
    AppState, Backend, Buffer, Context, Event, FormField, FormFieldResponse, FormState, KeyCode,
    Rect, Response, RunConfig, ScreenState, ScrollState, TextInputState,
};

struct Target(Buffer);
impl Backend for Target {
    fn size(&self) -> (u32, u32) {
        (self.0.area.width, self.0.area.height)
    }
    fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.0
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct App {
    target: Target,
    state: AppState,
}
impl App {
    fn new(width: u32, height: u32) -> Self {
        Self {
            target: Target(Buffer::empty(Rect::new(0, 0, width, height))),
            state: AppState::new(),
        }
    }
    fn frame(&mut self, events: Vec<Event>, mut draw: impl FnMut(&mut Context)) {
        self.target.0.reset();
        assert!(
            slt::frame_owned(
                &mut self.target,
                &mut self.state,
                &RunConfig::default(),
                events,
                &mut draw
            )
            .unwrap()
        );
    }
    fn text(&self) -> String {
        let mut text = String::new();
        for y in 0..self.target.0.area.height {
            for x in 0..self.target.0.area.width {
                text.push_str(self.target.0.get(x, y).symbol.as_str());
            }
            text.push('\n');
        }
        text
    }
}

fn form() -> FormState {
    (0..10).fold(FormState::new(), |state, index| {
        let mut field = FormField::new(format!("Label {index:02}"));
        field.input.value = format!("FIELD_{index:02}");
        state.field(field)
    })
}

#[test]
fn focus_transitions_withhold_stale_caret_privacy_until_the_target_renders() {
    let mut app = App::new(20, 8);
    let mut fields = [TextInputState::new(), TextInputState::new()];
    fields[1].masked = true;
    let mut draw = |ui: &mut Context| {
        for field in &mut fields {
            let _ = ui.text_input(field);
        }
    };
    app.frame(vec![], &mut draw);
    assert!(app.target.0.cursor_position().is_some());
    assert!(!app.target.0.cursor_is_masked());
    app.frame(vec![Event::key(KeyCode::Tab)], &mut draw);
    assert_eq!(app.target.0.cursor_position(), None);
    app.frame(vec![], &mut draw);
    assert!(app.target.0.cursor_position().is_some());
    assert!(app.target.0.cursor_is_masked());
    app.frame(vec![Event::key(KeyCode::BackTab)], &mut draw);
    assert_eq!(app.target.0.cursor_position(), None);
    app.frame(vec![], &mut draw);
    assert!(app.target.0.cursor_position().is_some());
    assert!(!app.target.0.cursor_is_masked());
    app.frame(vec![], |ui| {
        draw(ui);
        ui.set_focus_index(2);
    });
    assert!(
        app.target.0.cursor_position().is_some(),
        "equivalent modulo requests are not transitions"
    );
    app.frame(vec![], |ui| {
        draw(ui);
        ui.set_focus_index(1);
    });
    assert_eq!(app.target.0.cursor_position(), None);
    app.frame(vec![], &mut draw);
    assert!(app.target.0.cursor_is_masked());
}

fn draw_form(
    ui: &mut Context,
    state: &mut FormState,
    scroll: &mut ScrollState,
    modal: bool,
    height: u32,
    responses: &mut Vec<FormFieldResponse>,
) {
    responses.clear();
    let mut body = |ui: &mut Context| {
        let _ = ui.scrollable(scroll).w(28).h(height).col(|ui| {
            ui.form(state, |ui, state| {
                for field in &mut state.fields {
                    responses.push(ui.form_field_response(field));
                }
            });
        });
    };
    if modal {
        let _ = ui.modal_with(ModalOptions { tab_trap: true }, body);
    } else {
        body(ui);
    }
}

#[test]
fn tab_and_reverse_tab_reveal_fields_and_wrap_in_normal_and_modal_forms() {
    for modal in [false, true] {
        let mut app = App::new(40, 12);
        let mut fields = form();
        let mut scroll = ScrollState::new();
        let mut responses = vec![];
        for _ in 0..2 {
            app.frame(vec![], |ui| {
                draw_form(ui, &mut fields, &mut scroll, modal, 6, &mut responses)
            });
        }
        for index in 1..10 {
            app.frame(vec![Event::key(KeyCode::Tab)], |ui| {
                draw_form(ui, &mut fields, &mut scroll, modal, 6, &mut responses)
            });
            for _ in 0..2 {
                app.frame(vec![], |ui| {
                    draw_form(ui, &mut fields, &mut scroll, modal, 6, &mut responses)
                });
            }
            assert!(responses[index].focused);
            assert!(
                app.text().contains(&format!("FIELD_{index:02}")),
                "focus {index} not revealed"
            );
            assert!(app.target.0.cursor_position().is_some());
        }
        assert!(scroll.offset > 0);
        app.frame(vec![Event::key(KeyCode::Tab)], |ui| {
            draw_form(ui, &mut fields, &mut scroll, modal, 6, &mut responses)
        });
        for _ in 0..2 {
            app.frame(vec![], |ui| {
                draw_form(ui, &mut fields, &mut scroll, modal, 6, &mut responses)
            });
        }
        assert!(responses[0].focused);
        assert!(app.text().contains("FIELD_00"));
        assert!(app.text().contains("Label 00"));
        app.frame(vec![Event::key(KeyCode::BackTab)], |ui| {
            draw_form(ui, &mut fields, &mut scroll, modal, 6, &mut responses)
        });
        for _ in 0..2 {
            app.frame(vec![], |ui| {
                draw_form(ui, &mut fields, &mut scroll, modal, 6, &mut responses)
            });
        }
        assert!(responses[9].focused);
        assert!(app.text().contains("FIELD_09"));
    }
}

#[test]
fn initial_input_is_unique_and_real_tab_edges_fire_once() {
    let mut app = App::new(40, 12);
    let mut a = TextInputState::new();
    let mut b = TextInputState::new();
    let mut responses = [Response::none(), Response::none()];
    app.frame(vec![Event::key(KeyCode::Char('x'))], |ui| {
        responses = [ui.text_input(&mut a), ui.text_input(&mut b)];
    });
    assert!(responses[0].focused);
    assert!(!responses[1].focused);
    assert_eq!(a.value, "x");
    assert_eq!(b.value, "");
    app.frame(vec![Event::key(KeyCode::Tab)], |ui| {
        responses = [ui.text_input(&mut a), ui.text_input(&mut b)];
    });
    app.frame(vec![], |ui| {
        responses = [ui.text_input(&mut a), ui.text_input(&mut b)];
    });
    assert!(responses[0].lost_focus);
    assert!(responses[1].gained_focus);
    app.frame(vec![], |ui| {
        responses = [ui.text_input(&mut a), ui.text_input(&mut b)];
    });
    assert!(!responses[0].lost_focus);
    assert!(!responses[1].gained_focus);
}

#[test]
fn focus_edges_use_the_rendered_slot_for_late_and_modulo_requests() {
    let mut app = App::new(30, 5);
    let mut responses = [Response::none(), Response::none()];
    for _ in 0..2 {
        app.frame(vec![], |ui| {
            responses = [ui.button("A"), ui.button("B")];
        });
    }
    app.frame(vec![], |ui| {
        responses = [ui.button("A"), ui.button("B")];
        ui.set_focus_index(3);
    });
    app.frame(vec![], |ui| {
        responses = [ui.button("A"), ui.button("B")];
    });
    assert!(responses[0].lost_focus && responses[1].gained_focus);
    app.frame(vec![], |ui| {
        responses = [ui.button("A"), ui.button("B")];
    });
    assert!(responses[1].focused && !responses[1].gained_focus);
    app.frame(vec![], |ui| {
        ui.set_focus_index(0);
        responses = [ui.button("A"), ui.button("B")];
    });
    assert!(responses[0].gained_focus && responses[1].lost_focus);
}

#[test]
fn manual_scrolling_and_opt_out_do_not_get_pulled_back_on_idle_frames() {
    for follow_focus in [false, true] {
        let mut app = App::new(40, 12);
        let mut fields = form();
        let mut scroll = ScrollState::new();
        scroll.follow_focus = follow_focus;
        let mut responses = vec![];
        for _ in 0..3 {
            app.frame(vec![], |ui| {
                draw_form(ui, &mut fields, &mut scroll, false, 6, &mut responses)
            });
        }
        scroll.set_offset(34);
        for _ in 0..3 {
            app.frame(vec![], |ui| {
                draw_form(ui, &mut fields, &mut scroll, false, 6, &mut responses)
            });
        }
        assert_eq!(scroll.offset, 34);
        assert!(app.text().contains("FIELD_09"));
        // Appending below an unchanged focus must not cancel manual browsing.
        fields.fields.push(FormField::new("Extra"));
        for _ in 0..3 {
            app.frame(vec![], |ui| {
                draw_form(ui, &mut fields, &mut scroll, false, 6, &mut responses)
            });
        }
        assert_eq!(scroll.offset, 34);
        if !follow_focus {
            scroll.set_offset(0);
            for _ in 0..3 {
                app.frame(vec![], |ui| {
                    ui.set_focus_index(9);
                    draw_form(ui, &mut fields, &mut scroll, false, 6, &mut responses);
                });
            }
            assert_eq!(scroll.offset, 0);
            assert!(!app.text().contains("FIELD_09"));
        }
    }
}

#[test]
fn logical_geometry_remains_available_for_clipped_form_fields() {
    let mut app = App::new(40, 12);
    let mut fields = form();
    let mut scroll = ScrollState::new();
    scroll.follow_focus = false;
    let mut responses = vec![];
    let mut focus_rect = None;
    let mut group_rect = None;
    let mut visible_group = None;
    for _ in 0..3 {
        app.frame(vec![], |ui| {
            ui.set_focus_index(9);
            let _ = ui.scrollable(&mut scroll).w(28).h(6).col(|ui| {
                responses.clear();
                for (index, field) in fields.fields.iter_mut().enumerate() {
                    let _ = ui.group(&format!("field-{index}")).col(|ui| {
                        responses.push(ui.form_field_response(field));
                    });
                }
            });
            focus_rect = ui.focused_layout_rect();
            group_rect = ui.measured_layout_rect("field-9");
            visible_group = ui.measured_rect("field-9");
        });
    }
    assert_eq!(responses[9].rect, Rect::default());
    assert!(responses[9].layout_rect.unwrap().y > 6);
    assert!(focus_rect.unwrap().y > 6);
    assert!(group_rect.unwrap().y > 6);
    assert!(visible_group.is_none());
    let before = focus_rect;
    scroll.set_offset(34);
    for _ in 0..2 {
        app.frame(vec![], |ui| {
            ui.set_focus_index(9);
            let _ = ui.scrollable(&mut scroll).w(28).h(6).col(|ui| {
                for (index, field) in fields.fields.iter_mut().enumerate() {
                    let _ = ui.group(&format!("field-{index}")).col(|ui| {
                        ui.form_field(field);
                    });
                }
            });
            focus_rect = ui.focused_layout_rect();
        });
    }
    assert_eq!(focus_rect, before);
    assert!(app.target.0.cursor_position().is_some());
}

#[test]
fn overlay_and_raw_scrollers_do_not_corrupt_scroll_state_bindings() {
    let mut app = App::new(50, 20);
    let mut fields = form();
    let mut modal = ScrollState::new();
    let mut background = ScrollState::new();
    let mut responses = vec![];
    for _ in 0..3 {
        app.frame(vec![], |ui| {
            draw_form(ui, &mut fields, &mut modal, true, 6, &mut responses);
            let _ = ui.container().scroll_offset(0).h(2).col(|ui| {
                for _ in 0..8 {
                    ui.text("raw");
                }
            });
            let _ = ui.scrollable(&mut background).h(3).col(|ui| {
                for _ in 0..100 {
                    ui.text("background");
                }
            });
        });
    }
    assert_eq!(modal.content_height(), 40);
    assert_eq!(modal.viewport_height(), 6);
    assert_eq!(background.content_height(), 100);
    assert_eq!(background.viewport_height(), 3);
}

#[test]
fn current_layout_clamps_bound_offsets_after_content_shrink() {
    let mut app = App::new(40, 12);
    let mut fields = form();
    let mut scroll = ScrollState::new();
    scroll.follow_focus = false;
    let mut responses = vec![];
    for _ in 0..3 {
        app.frame(vec![], |ui| {
            draw_form(ui, &mut fields, &mut scroll, false, 8, &mut responses)
        });
    }
    scroll.set_offset(32);
    app.frame(vec![], |ui| {
        draw_form(ui, &mut fields, &mut scroll, false, 8, &mut responses)
    });
    fields.fields.truncate(2);
    app.frame(vec![], |ui| {
        draw_form(ui, &mut fields, &mut scroll, false, 8, &mut responses)
    });
    assert!(app.text().contains("FIELD_00"));
    app.frame(vec![], |ui| {
        draw_form(ui, &mut fields, &mut scroll, false, 8, &mut responses)
    });
    assert_eq!(scroll.offset, 0);
}

#[test]
fn nested_axes_reveal_the_focused_child_from_inside_out() {
    let mut app = App::new(40, 15);
    let mut fields = form();
    let mut horizontal = ScrollState::new();
    let mut vertical = ScrollState::new();
    for _ in 0..3 {
        app.frame(vec![], |ui| {
            ui.set_focus_index(9);
            let _ = ui.scrollable(&mut horizontal).w(20).h(6).row(|ui| {
                ui.text("prefix").w(30);
                let _ = ui.scrollable(&mut vertical).w(20).h(6).col(|ui| {
                    for field in &mut fields.fields {
                        let _ = ui.text_input(&mut field.input);
                    }
                });
            });
        });
    }
    assert_eq!(horizontal.offset_x, 30);
    assert_eq!(vertical.offset, 24);
    assert!(app.text().contains("FIELD_09"));
    let (x, y) = app.target.0.cursor_position().unwrap();
    assert!(x < 20 && y < 6);
}

#[test]
fn tiny_viewport_reveals_the_caret_instead_of_only_the_input_border() {
    let mut app = App::new(30, 5);
    let mut input = TextInputState::new();
    input.value = "value".into();
    let mut scroll = ScrollState::new();
    for _ in 0..3 {
        app.frame(vec![], |ui| {
            let _ = ui.scrollable(&mut scroll).w(20).h(1).col(|ui| {
                let _ = ui.text_input(&mut input);
            });
        });
    }
    assert_eq!(scroll.offset, 1);
    assert_eq!(app.target.0.cursor_position().map(|(_, y)| y), Some(0));
    assert!(app.text().contains("value"));
}

#[test]
fn resize_and_validation_growth_keep_the_same_focused_input_visible() {
    let mut app = App::new(40, 12);
    let mut fields = form();
    let mut scroll = ScrollState::new();
    let mut responses = vec![];
    for _ in 0..3 {
        app.frame(vec![], |ui| {
            ui.set_focus_index(9);
            draw_form(ui, &mut fields, &mut scroll, false, 6, &mut responses);
        });
    }
    fields.fields[0].error = Some("Validation error above focus".into());
    app.frame(vec![], |ui| {
        draw_form(ui, &mut fields, &mut scroll, false, 6, &mut responses)
    });
    assert!(app.text().contains("FIELD_09"));
    app.target.0.resize(Rect::new(0, 0, 35, 8));
    for _ in 0..2 {
        app.frame(vec![], |ui| {
            draw_form(ui, &mut fields, &mut scroll, false, 3, &mut responses)
        });
    }
    assert!(app.text().contains("FIELD_09"));
    assert!(app.target.0.cursor_position().is_some());
}

#[test]
fn screen_tab_and_mouse_focus_are_not_overwritten_by_saved_state() {
    let mut app = App::new(30, 8);
    let mut screens = ScreenState::new("main");
    let mut responses = [Response::none(), Response::none()];
    for _ in 0..2 {
        app.frame(vec![], |ui| {
            ui.screen("main", &mut screens, |ui| {
                responses = [ui.button("A"), ui.button("B")];
            });
        });
    }
    app.frame(vec![Event::key(KeyCode::Tab)], |ui| {
        ui.screen("main", &mut screens, |ui| {
            responses = [ui.button("A"), ui.button("B")];
        });
    });
    app.frame(vec![], |ui| {
        ui.screen("main", &mut screens, |ui| {
            responses = [ui.button("A"), ui.button("B")];
        });
    });
    assert!(responses[0].lost_focus && responses[1].gained_focus);
    app.frame(vec![], |ui| {
        ui.screen("main", &mut screens, |ui| {
            responses = [ui.button("A"), ui.button("B")];
        });
    });
    assert!(responses[1].focused && !responses[1].gained_focus);
    let rect = responses[0].rect;
    app.frame(vec![Event::mouse_click(rect.x, rect.y)], |ui| {
        ui.screen("main", &mut screens, |ui| {
            responses = [ui.button("A"), ui.button("B")];
        });
    });
    assert!(responses[0].focused && responses[0].clicked);
}

#[test]
fn returning_to_a_larger_screen_never_activates_the_wrong_saved_slot() {
    let mut app = App::new(30, 8);
    let mut screens = ScreenState::new("a");
    let mut responses = vec![];
    for _ in 0..2 {
        app.frame(vec![], |ui| {
            ui.screen("a", &mut screens, |ui| {
                ui.set_focus_index(2);
                responses = vec![ui.button("A0"), ui.button("A1"), ui.button("A2")];
            });
        });
    }
    screens.push("b");
    app.frame(vec![], |ui| {
        ui.screen("b", &mut screens, |ui| {
            let _ = ui.button("B0");
        });
    });
    screens.pop();
    app.frame(vec![Event::key(KeyCode::Enter)], |ui| {
        ui.screen("a", &mut screens, |ui| {
            responses = vec![ui.button("A0"), ui.button("A1"), ui.button("A2")];
        });
    });
    assert!(!responses[0].clicked && !responses[1].clicked);
    assert!(responses[2].focused && responses[2].clicked);
}

#[test]
fn screen_focus_survives_a_hidden_frame_and_clone_without_borrowed_pointers() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<ScreenState>();
    let mut app = App::new(30, 8);
    let mut screens = ScreenState::new("a");
    let mut responses = vec![];
    for _ in 0..2 {
        app.frame(vec![], |ui| {
            ui.screen("a", &mut screens, |ui| {
                responses = vec![ui.button("A0"), ui.button("A1")];
            });
        });
    }
    app.frame(vec![Event::key(KeyCode::Tab)], |ui| {
        ui.screen("a", &mut screens, |ui| {
            responses = vec![ui.button("A0"), ui.button("A1")];
        });
    });
    let mut cloned = screens.clone();
    app.frame(vec![], |ui| {
        ui.text("temporarily hidden");
    });
    app.frame(vec![], |ui| {
        ui.screen("a", &mut screens, |ui| {
            responses = vec![ui.button("A0"), ui.button("A1")];
        });
    });
    assert!(responses[1].focused);
    let mut other = App::new(30, 8);
    other.frame(vec![], |ui| {
        ui.screen("a", &mut cloned, |ui| {
            responses = vec![ui.button("A0"), ui.button("A1")];
        });
    });
    assert!(responses[1].focused);
}

#[test]
fn header_and_nested_independent_screen_states_share_unique_global_focus() {
    let mut app = App::new(30, 8);
    let mut outer = ScreenState::new("same");
    let mut inner = ScreenState::new("same");
    let mut responses = vec![];
    for step in 0..8 {
        let events = if step > 1 {
            vec![Event::key(KeyCode::Tab)]
        } else {
            vec![]
        };
        app.frame(events, |ui| {
            responses = vec![ui.button("header")];
            ui.screen("same", &mut outer, |ui| {
                responses.push(ui.button("outer"));
                ui.screen("same", &mut inner, |ui| {
                    responses.push(ui.button("inner"));
                });
            });
            responses.push(ui.button("footer"));
        });
        assert_eq!(
            responses.iter().filter(|response| response.focused).count(),
            1
        );
        let expected = if step < 2 { 0 } else { (step - 2) % 4 };
        assert!(responses[expected].focused, "step {step}");
    }
}

#[test]
fn modal_focus_does_not_jump_when_background_slots_are_suppressed() {
    let mut app = App::new(40, 12);
    let mut modal = false;
    let mut responses = vec![];
    for frame in 0..7 {
        if frame == 2 {
            modal = true;
        }
        app.frame(vec![], |ui| {
            let _ = ui.button("background A");
            let _ = ui.button("background B");
            if modal {
                let _ = ui.modal_with(ModalOptions { tab_trap: true }, |ui| {
                    responses = (0..4).map(|i| ui.button(format!("modal {i}"))).collect();
                });
            }
        });
        if frame >= 3 {
            assert!(responses[0].focused, "focus jumped on frame {frame}");
            assert_eq!(
                responses.iter().filter(|response| response.focused).count(),
                1
            );
        }
    }
}

#[test]
fn adding_modal_widgets_never_duplicates_the_focused_slot() {
    let mut app = App::new(40, 12);
    let mut responses = vec![];
    for count in [2, 2, 3, 3] {
        app.frame(vec![], |ui| {
            let _ = ui.modal(|ui| {
                responses = (0..count)
                    .map(|i| ui.button(format!("modal {i}")))
                    .collect();
            });
        });
        assert_eq!(
            responses.iter().filter(|response| response.focused).count(),
            1
        );
    }
}

#[test]
fn failed_screen_scope_restores_focus_and_discards_phantom_names() {
    let mut app = App::new(40, 10);
    let mut screens = ScreenState::new("bad");
    for _ in 0..2 {
        app.frame(vec![], |ui| {
            let _ = ui.button("stable");
        });
    }
    app.frame(vec![], |ui| {
        let _ = ui.button("stable");
        ui.error_boundary(|ui| {
            ui.screen("bad", &mut screens, |ui| {
                ui.set_focus_index(7);
                ui.focus_by_name("phantom");
                ui.register_focusable_named("phantom");
                let _ = ui.button("rolled back");
                panic!("fixture rollback");
            });
        });
        assert_eq!(ui.focus_index(), 0);
        assert!(!ui.focus_by_name("phantom"));
    });
}

#[test]
fn static_parent_clip_limits_reveal_and_scroll_bounds() {
    let mut app = App::new(30, 10);
    let mut scroll = ScrollState::new();
    for _ in 0..3 {
        app.frame(vec![], |ui| {
            ui.set_focus_index(9);
            let _ = ui.container().h(3).col(|ui| {
                let _ = ui.scrollable(&mut scroll).h(6).col(|ui| {
                    for index in 0..10 {
                        let _ = ui.button(format!("button-{index}"));
                    }
                });
            });
        });
    }
    assert!(app.text().contains("button-9"));
    assert_eq!(scroll.viewport_height(), 3);
    assert!(scroll.offset > 0);
}

#[test]
fn inserting_unrelated_containers_never_rebinds_other_scroll_states() {
    let mut app = App::new(30, 12);
    let mut a = ScrollState::new();
    let mut b = ScrollState::new();
    a.offset = 80;
    b.offset = 2;
    for inserted in [false, false, true, true] {
        app.frame(vec![], |ui| {
            if inserted {
                let _ = ui.container().h(0).col(|_| {});
            }
            let _ = ui.scrollable(&mut a).h(5).col(|ui| {
                for _ in 0..100 {
                    ui.text("A");
                }
            });
            let _ = ui.scrollable(&mut b).h(2).col(|ui| {
                for _ in 0..4 {
                    ui.text("B");
                }
            });
        });
        assert_eq!(a.offset, 80);
        assert_eq!(b.offset, 2);
    }
    assert_eq!(a.content_height(), 100);
    assert_eq!(b.content_height(), 4);
}

#[test]
fn application_offset_edits_override_pending_reveal_feedback() {
    let mut app = App::new(40, 12);
    let mut fields = form();
    let mut scroll = ScrollState::new();
    let mut responses = vec![];
    for _ in 0..2 {
        app.frame(vec![], |ui| {
            draw_form(ui, &mut fields, &mut scroll, false, 6, &mut responses)
        });
    }
    app.frame(vec![], |ui| {
        ui.set_focus_index(9);
        draw_form(ui, &mut fields, &mut scroll, false, 6, &mut responses);
    });
    assert!(app.text().contains("FIELD_09"));
    scroll.offset = 10;
    for _ in 0..2 {
        app.frame(vec![], |ui| {
            draw_form(ui, &mut fields, &mut scroll, false, 6, &mut responses)
        });
    }
    assert_eq!(scroll.offset, 10);
}

#[test]
fn scroll_arithmetic_saturates_even_for_public_extreme_offsets() {
    let mut app = App::new(30, 12);
    let mut scroll = ScrollState::new();
    for _ in 0..2 {
        app.frame(vec![], |ui| {
            let _ = ui.scrollable(&mut scroll).h(5).col(|ui| {
                for _ in 0..20 {
                    ui.text("row");
                }
            });
        });
    }
    scroll.offset = usize::MAX;
    assert!(!scroll.can_scroll_down());
    scroll.scroll_down(usize::MAX);
    assert_eq!(scroll.offset, 15);
    scroll.scroll_up(usize::MAX);
    assert_eq!(scroll.offset, 0);
    let mut horizontal = ScrollState::new();
    for _ in 0..2 {
        app.frame(vec![], |ui| {
            let _ = ui.scrollable(&mut horizontal).w(5).row(|ui| {
                ui.text("01234567890123456789");
            });
        });
    }
    horizontal.offset_x = usize::MAX;
    assert!(!horizontal.can_scroll_right());
    horizontal.scroll_right(usize::MAX);
    assert_eq!(horizontal.offset_x, 15);
}

#[test]
fn closing_modal_restores_the_background_slot_and_only_top_modal_is_active() {
    let mut app = App::new(40, 12);
    let mut background = vec![];
    let mut lower = vec![];
    let mut upper = vec![];
    for frame in 0..9 {
        app.frame(vec![], |ui| {
            if frame < 2 {
                ui.set_focus_index(1);
            }
            background = vec![ui.button("A"), ui.button("B")];
            if (2..6).contains(&frame) {
                let _ = ui.modal(|ui| {
                    lower = vec![ui.button("low0"), ui.button("low1")];
                });
                let _ = ui.modal(|ui| {
                    upper = vec![ui.button("top0"), ui.button("top1")];
                });
            }
        });
        if (3..6).contains(&frame) {
            assert!(lower.iter().all(|response| !response.focused));
            assert_eq!(upper.iter().filter(|response| response.focused).count(), 1);
            assert!(background.iter().all(|response| !response.focused));
        }
        if frame >= 7 {
            assert!(background[1].focused);
        }
    }
}

#[test]
fn completion_keeps_plain_tab_but_shift_tab_still_navigates_focus() {
    let mut app = App::new(40, 12);
    let mut input = TextInputState::new();
    input.value = "he".into();
    input.cursor = 2;
    input.suggestions = vec!["hello".into()];
    input.show_suggestions = true;
    let mut focused = false;
    for _ in 0..2 {
        app.frame(vec![], |ui| {
            let _ = ui.text_input(&mut input);
            focused = ui.button("other").focused;
        });
    }
    app.frame(vec![Event::key(KeyCode::Tab)], |ui| {
        let _ = ui.text_input(&mut input);
        let _ = ui.button("other");
    });
    app.frame(vec![], |ui| {
        let _ = ui.text_input(&mut input);
        focused = ui.button("other").focused;
    });
    assert_eq!(input.value, "hello");
    assert!(!focused);
    input.value = "he".into();
    input.cursor = 2;
    input.show_suggestions = true;
    let events = slt::EventBuilder::new()
        .key_with(KeyCode::Tab, slt::KeyModifiers::SHIFT)
        .build();
    app.frame(events, |ui| {
        let _ = ui.text_input(&mut input);
        let _ = ui.button("other");
    });
    app.frame(vec![], |ui| {
        let _ = ui.text_input(&mut input);
        focused = ui.button("other").focused;
    });
    assert_eq!(input.value, "he");
    assert!(focused);
}

#[test]
fn unrelated_widget_insertion_does_not_drop_active_modal_input() {
    let mut app = App::new(40, 12);
    let mut input = TextInputState::new();
    for index in 0..4 {
        let events = if index == 3 {
            vec![Event::key(KeyCode::Char('Z'))]
        } else {
            vec![]
        };
        app.frame(events, |ui| {
            if index == 3 {
                let _ = ui.container().h(0).col(|_| {});
            }
            let _ = ui.modal(|ui| {
                let _ = ui.text_input(&mut input);
            });
        });
    }
    assert_eq!(input.value, "Z");
}

#[test]
fn cross_axis_parent_clipping_reduces_the_inner_viewport() {
    let mut app = App::new(30, 10);
    let mut outer = ScrollState::new();
    let mut inner = ScrollState::new();
    for _ in 0..3 {
        app.frame(vec![], |ui| {
            ui.set_focus_index(19);
            let _ = ui.scrollable(&mut outer).w(20).h(3).row(|ui| {
                let _ = ui.scrollable(&mut inner).w(20).h(6).col(|ui| {
                    for index in 0..20 {
                        let _ = ui.button(format!("button-{index}"));
                    }
                });
            });
        });
    }
    assert_eq!(inner.viewport_height(), 3);
    assert_eq!(inner.offset, 17);
    assert!(app.text().contains("button-19"));
}

proptest::proptest! {
    #[test]
    fn batched_navigation_and_text_match_an_ordered_model(count in 1usize..10, actions in proptest::collection::vec(0u8..5, 1..50)) {
        let mut app = App::new(40, 32);
        let mut fields = vec![TextInputState::new(); count];
        let mut values = vec![String::new(); count];
        let mut expected_focus = 0;
        let mut observed_focus = 0;
        let mut events = vec![];
        for action in actions {
            let key = match action {
                0 => { expected_focus = (expected_focus + 1) % count; KeyCode::Tab }
                1 => { expected_focus = (expected_focus + count - 1) % count; KeyCode::BackTab }
                other => { let ch = char::from(b'a' + other); values[expected_focus].push(ch); KeyCode::Char(ch) }
            };
            events.push(Event::key(key));
        }
        let mut draw = |ui: &mut Context| {
            for (index, field) in fields.iter_mut().enumerate() {
                if ui.text_input(field).focused { observed_focus = index; }
            }
        };
        for _ in 0..2 { app.frame(vec![], &mut draw); }
        app.frame(events, &mut draw);
        while app.state.has_pending_input() { app.frame(vec![], &mut draw); }
        app.frame(vec![], &mut draw);
        proptest::prop_assert_eq!(observed_focus, expected_focus);
        proptest::prop_assert_eq!(fields.iter().map(|field| field.value.clone()).collect::<Vec<_>>(), values);
    }

    #[test]
    fn clipped_scroll_end_stays_visible(top in 0u32..10, prefix in 0u32..5, overlap in 0u32..10) {
        let start = top.saturating_add(prefix).saturating_sub(overlap);
        let end = (start + 6).min(top + 6);
        proptest::prop_assume!(end > start.max(top));
        let mut app = App::new(30, 24);
        let mut scroll = ScrollState::new();
        scroll.follow_focus = false;
        let mut draw = |ui: &mut Context| {
            let _ = ui.container().mt(top).h(6).gap_overlap(overlap).col(|ui| {
                ui.text("prefix").h(prefix);
                let _ = ui.scrollable(&mut scroll).h(6).col(|ui| {
                    for index in 0..20 { ui.text(format!("row-{index}")); }
                });
            });
        };
        for _ in 0..3 { app.frame(vec![], &mut draw); }
        let expected = 20u32.saturating_sub(end - start) as usize;
        proptest::prop_assert_eq!(scroll.max_offset(), expected);
        scroll.set_offset(usize::MAX);
        for _ in 0..2 { app.frame(vec![], |ui| {
            let _ = ui.container().mt(top).h(6).gap_overlap(overlap).col(|ui| {
                ui.text("prefix").h(prefix);
                let _ = ui.scrollable(&mut scroll).h(6).col(|ui| {
                    for index in 0..20 { ui.text(format!("row-{index}")); }
                });
            });
        }); }
        proptest::prop_assert_eq!(scroll.offset, expected);
        proptest::prop_assert!(app.text().contains("row-19"));
    }

    #[test]
    fn leading_horizontal_clip_keeps_the_last_cell_reachable(left in 0u32..10, prefix in 0u32..5, overlap in 0u32..10) {
        let start = left.saturating_add(prefix).saturating_sub(overlap);
        let end = (start + 6).min(left + 6);
        proptest::prop_assume!(end > start.max(left));
        let mut app = App::new(30, 4);
        let mut scroll = ScrollState::new();
        scroll.follow_focus = false;
        for frame in 0..5 {
            if frame == 3 { scroll.scroll_right(usize::MAX); }
            app.frame(vec![], |ui| {
                let _ = ui.container().ml(left).w(6).h(1).gap_overlap(overlap).row(|ui| {
                    ui.text("prefix").w(prefix);
                    let _ = ui.scrollable(&mut scroll).w(6).h(1).row(|ui| {
                        for index in 0..20 { ui.text((index % 10).to_string()); }
                    });
                });
            });
        }
        proptest::prop_assert_eq!(scroll.max_offset_x(), 20u32.saturating_sub(end - start) as usize);
        proptest::prop_assert_eq!(app.target.0.get(end - 1, 0).symbol.as_str(), "9");
    }
}

#[test]
fn nested_screen_prefix_change_rebases_the_original_slot_only_once() {
    let mut app = App::new(40, 12);
    let mut outer = ScreenState::new("outer");
    let mut inner = ScreenState::new("inner");
    let mut responses = vec![];
    for frame in 0..5 {
        app.frame(vec![], |ui| {
            if frame == 0 {
                ui.set_focus_index(2);
            }
            responses.clear();
            if frame >= 3 {
                responses.push(ui.button("new header"));
            }
            responses.push(ui.button("header"));
            ui.screen("outer", &mut outer, |ui| {
                responses.push(ui.button("outer"));
                ui.screen("inner", &mut inner, |ui| {
                    responses.push(ui.button("inner 0"));
                    responses.push(ui.button("inner 1"));
                });
            });
            responses.push(ui.button("footer"));
        });
        let expected = if frame >= 3 { 3 } else { 2 };
        assert!(responses[expected].focused, "frame {frame}");
        assert_eq!(
            responses.iter().filter(|response| response.focused).count(),
            1
        );
        if frame >= 3 {
            assert!(!responses[expected].gained_focus);
        }
    }
}

#[test]
fn returning_to_a_shrunken_screen_does_not_activate_its_footer() {
    let mut app = App::new(40, 10);
    let mut screens = ScreenState::new("a");
    let mut responses = vec![];
    let mut footer = Response::none();
    for _ in 0..2 {
        app.frame(vec![], |ui| {
            ui.screen("a", &mut screens, |ui| {
                ui.set_focus_index(2);
                responses = (0..3).map(|i| ui.button(format!("A{i}"))).collect();
            });
            footer = ui.button("footer");
        });
    }
    screens.push("b");
    app.frame(vec![], |ui| {
        ui.screen("b", &mut screens, |ui| {
            for i in 0..4 {
                let _ = ui.button(format!("B{i}"));
            }
        });
        footer = ui.button("footer");
    });
    screens.pop();
    app.frame(vec![Event::key(KeyCode::Enter)], |ui| {
        ui.screen("a", &mut screens, |ui| {
            responses = (0..2).map(|i| ui.button(format!("A{i}"))).collect();
        });
        footer = ui.button("footer");
    });
    assert!(!footer.clicked);
    assert!(responses.iter().all(|response| !response.clicked));
    app.frame(vec![], |ui| {
        ui.screen("a", &mut screens, |ui| {
            responses = (0..2).map(|i| ui.button(format!("A{i}"))).collect();
        });
        footer = ui.button("footer");
    });
    assert!(responses[1].focused && !footer.focused);
}

#[test]
fn pruning_inactive_screen_history_cancels_deferred_writes() {
    for retain in [false, true] {
        let mut app = App::new(30, 8);
        let mut screens = ScreenState::new("home");
        screens.push("detail");
        app.frame(vec![], |ui| {
            ui.screen("detail", &mut screens, |ui| {
                ui.set_focus_index(1);
                let _ = ui.button("detail 0");
                let _ = ui.button("detail 1");
                ui.pop_screen();
            });
            if retain {
                assert_eq!(screens.retain_inactive(|_| false), 1);
            } else {
                assert!(screens.remove_inactive("detail"));
            }
        });
        app.frame(vec![], |ui| {
            ui.screen("home", &mut screens, |ui| {
                let _ = ui.button("home");
            });
        });
        assert!(!screens.remove_inactive("detail"));
        assert_eq!(screens.focus_state_count(), 1);
    }
}

#[test]
fn tab_then_text_in_one_batch_never_edits_the_previous_field() {
    for backwards in [false, true] {
        let mut app = App::new(40, 12);
        let mut a = TextInputState::new();
        let mut b = TextInputState::new();
        for _ in 0..2 {
            app.frame(vec![], |ui| {
                let _ = ui.text_input(&mut a);
                let _ = ui.text_input(&mut b);
            });
        }
        let navigation = if backwards {
            KeyCode::BackTab
        } else {
            KeyCode::Tab
        };
        app.frame(
            vec![Event::key(navigation), Event::key(KeyCode::Char('x'))],
            |ui| {
                let _ = ui.text_input(&mut a);
                let _ = ui.text_input(&mut b);
            },
        );
        assert_eq!(a.value, "");
        assert_eq!(b.value, "");
        assert!(app.state.has_pending_input());
        app.frame(vec![], |ui| {
            let _ = ui.text_input(&mut a);
            let _ = ui.text_input(&mut b);
        });
        assert!(!app.state.has_pending_input());
        assert_eq!(a.value, "");
        assert_eq!(b.value, "x");
    }
}

#[test]
fn tab_then_enter_activates_the_new_button_and_keeps_new_input_order() {
    let mut app = App::new(30, 8);
    let mut count = [0; 2];
    for _ in 0..2 {
        app.frame(vec![], |ui| {
            let _ = ui.button("A");
            let _ = ui.button("B");
        });
    }
    app.frame(
        vec![Event::key(KeyCode::Tab), Event::key(KeyCode::Enter)],
        |ui| {
            if ui.button("A").clicked {
                count[0] += 1;
            }
            if ui.button("B").clicked {
                count[1] += 1;
            }
        },
    );
    assert_eq!(count, [0, 0]);
    app.frame(vec![Event::key(KeyCode::BackTab)], |ui| {
        if ui.button("A").clicked {
            count[0] += 1;
        }
        if ui.button("B").clicked {
            count[1] += 1;
        }
    });
    assert_eq!(count, [0, 1]);
    let mut focus = false;
    app.frame(vec![], |ui| {
        focus = ui.button("A").focused;
        let _ = ui.button("B");
    });
    assert!(focus);
}

#[test]
fn a_new_screen_item_cannot_steal_footer_activation() {
    for mouse in [false, true] {
        let mut app = App::new(40, 10);
        let mut screens = ScreenState::new("a");
        let mut buttons = vec![];
        let mut footer = Response::none();
        for _ in 0..3 {
            app.frame(vec![], |ui| {
                ui.set_focus_index(2);
                ui.screen("a", &mut screens, |ui| {
                    buttons = (0..2).map(|i| ui.button(format!("A{i}"))).collect();
                });
                footer = ui.button("footer");
            });
        }
        let events = if mouse {
            vec![Event::mouse_click(footer.rect.x, footer.rect.y)]
        } else {
            vec![Event::key(KeyCode::Enter)]
        };
        app.frame(events, |ui| {
            ui.screen("a", &mut screens, |ui| {
                buttons = (0..3).map(|i| ui.button(format!("A{i}"))).collect();
            });
            footer = ui.button("footer");
        });
        assert!(buttons.iter().all(|button| !button.clicked));
        if !mouse {
            assert!(footer.clicked);
        }
    }
}

#[test]
fn nested_screen_prefix_removal_does_not_normalize_counts_twice() {
    let mut app = App::new(40, 12);
    let mut outer = ScreenState::new("outer");
    let mut inner = ScreenState::new("inner");
    let mut buttons = vec![];
    for frame in 0..4 {
        app.frame(
            if frame == 3 {
                vec![Event::key(KeyCode::Enter)]
            } else {
                vec![]
            },
            |ui| {
                if frame == 0 {
                    ui.set_focus_index(4);
                }
                buttons.clear();
                if frame < 3 {
                    let _ = ui.button("H0");
                    let _ = ui.button("H1");
                }
                ui.screen("outer", &mut outer, |ui| {
                    let _ = ui.button("outer");
                    ui.screen("inner", &mut inner, |ui| {
                        buttons = vec![ui.button("inner0"), ui.button("inner1")];
                    });
                });
                let _ = ui.button("footer");
            },
        );
    }
    assert!(buttons[1].focused && buttons[1].clicked);
    assert!(!buttons[0].clicked);
    assert!(!buttons[1].gained_focus);
}

#[test]
fn changing_mailbox_name_cannot_resurrect_pruned_screen_history() {
    for retain in [false, true] {
        let mut app = App::new(30, 8);
        let mut screens = ScreenState::new("home");
        screens.push("detail");
        app.frame(vec![], |ui| {
            ui.screen("detail", &mut screens, |ui| {
                let _ = ui.button("detail");
            });
            screens.pop();
            ui.screen("home", &mut screens, |ui| {
                ui.text("home");
            });
            if retain {
                assert_eq!(screens.retain_inactive(|_| false), 1);
            } else {
                assert!(screens.remove_inactive("detail"));
            }
        });
        app.frame(vec![], |ui| {
            ui.screen("home", &mut screens, |ui| {
                ui.text("home");
            });
        });
        assert!(!screens.remove_inactive("detail"));
        assert_eq!(screens.focus_state_count(), 1);
    }
}
