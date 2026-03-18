use slt::anim::{ease_in_out_cubic, ease_out_bounce, ease_out_quad};
use slt::{Border, Color, Context, KeyCode, Keyframes, LoopMode, Sequence, Spring, Stagger, Tween};

fn main() -> std::io::Result<()> {
    let mut progress_target = 0.2;
    let mut progress_tween =
        Tween::new(progress_target, progress_target, 1).easing(ease_in_out_cubic);
    progress_tween.reset(0);

    let mut spring_target = 0.0;
    let mut spring = Spring::new(0.0, 0.15, 0.85);

    let mut kf = Keyframes::new(120)
        .stop(0.0, 0.0)
        .stop(0.3, 100.0)
        .stop(0.7, 20.0)
        .stop(1.0, 80.0)
        .loop_mode(LoopMode::PingPong);

    let mut seq = Sequence::new()
        .then(0.0, 80.0, 40, ease_out_quad)
        .then(80.0, 20.0, 30, ease_in_out_cubic)
        .then(20.0, 60.0, 20, ease_out_bounce)
        .loop_mode(LoopMode::Repeat);

    let mut stagger = Stagger::new(0.0, 1.0, 30)
        .delay(6)
        .easing(ease_out_quad)
        .items(5)
        .loop_mode(slt::LoopMode::Repeat);

    let mut anim_started = false;
    let mut cb_tween = Tween::new(0.0, 100.0, 120);
    let mut cb_fired = false;

    slt::run_with(
        slt::RunConfig {
            mouse: true,
            ..Default::default()
        },
        |ui: &mut Context| {
            if ui.key_mod('q', slt::KeyModifiers::CONTROL) || ui.key_code(slt::KeyCode::Esc) {
                ui.quit();
            }

            if ui.key(' ') {
                let current = progress_tween.value(ui.tick());
                progress_target = if progress_target < 0.5 { 0.9 } else { 0.1 };
                progress_tween = Tween::new(current, progress_target, 12).easing(ease_in_out_cubic);
                progress_tween.reset(ui.tick());
            }

            if ui.key('r') {
                let t = ui.tick();
                kf.reset(t);
                seq.reset(t);
                stagger.reset(t);
                anim_started = true;
                progress_tween = Tween::new(0.1, 0.9, 12).easing(ease_in_out_cubic);
                progress_tween.reset(t);
                spring_target = 0.0;
                spring = Spring::new(0.0, 0.15, 0.85);
            }

            if !anim_started {
                let t = ui.tick();
                kf.reset(t);
                seq.reset(t);
                stagger.reset(t);
                anim_started = true;
            }

            if ui.key_code(KeyCode::Up) || ui.key('k') {
                spring_target += 10.0;
                spring.set_target(spring_target);
            }
            if ui.key_code(KeyCode::Down) || ui.key('j') {
                spring_target -= 10.0;
                spring.set_target(spring_target);
            }

            spring.tick();
            let progress = progress_tween.value(ui.tick());

            let _ = ui
                .bordered(Border::Rounded)
                .title("Animation Primitives — SLT v0.5.0")
                .pad(1)
                .gap(1)
                .col(|ui| {
                    let _ = ui
                        .bordered(Border::Single)
                        .title("Tween")
                        .pad(1)
                        .gap(1)
                        .col(|ui| {
                            ui.text("Press Space to retarget");
                            ui.progress(progress);
                            ui.text(format!(
                                "value {:.2} -> target {:.2} | done {}",
                                progress,
                                progress_target,
                                progress_tween.is_done()
                            ));
                        });

                    let _ = ui
                        .bordered(Border::Single)
                        .title("Spring")
                        .pad(1)
                        .gap(1)
                        .col(|ui| {
                            ui.text("Up/k +10, Down/j -10");
                            ui.text(format!(
                                "value {:.2} | target {:.2} | settled {}",
                                spring.value(),
                                spring_target,
                                spring.is_settled()
                            ));
                        });

                    let _ = ui
                        .bordered(Border::Single)
                        .title("Keyframes")
                        .pad(1)
                        .gap(1)
                        .col(|ui| {
                            let kf_val = kf.value(ui.tick());
                            ui.progress(kf_val / 100.0);
                            ui.text(format!(
                                "value {:.1} | done {} | mode PingPong",
                                kf_val,
                                kf.is_done()
                            ));
                            ui.text("4 stops: 0→100→20→80").dim();
                        });

                    let _ = ui
                        .bordered(Border::Single)
                        .title("Sequence")
                        .pad(1)
                        .gap(1)
                        .col(|ui| {
                            let seq_val = seq.value(ui.tick());
                            ui.progress(seq_val / 100.0);
                            ui.text(format!(
                                "value {:.1} | done {} | mode Repeat",
                                seq_val,
                                seq.is_done()
                            ));
                            ui.text("3 chained: 0→80→20→60").dim();
                        });

                    let _ = ui
                        .bordered(Border::Single)
                        .title("Stagger")
                        .pad(1)
                        .gap(1)
                        .col(|ui| {
                            let labels = ["Item A", "Item B", "Item C", "Item D", "Item E"];
                            for (i, label) in labels.iter().enumerate() {
                                let val = stagger.value(ui.tick(), i);
                                let _ = ui.row(|ui| {
                                    ui.text(format!("{label}:"));
                                    ui.progress(val);
                                });
                            }
                            ui.text("5 items, 6-tick delay each").dim();
                        });

                    let accent = ui.theme().accent;
                    ui.text("Callback").bold().fg(accent);
                    let val = cb_tween.value(ui.tick());
                    ui.progress(val / 100.0);
                    if cb_tween.is_done() && !cb_fired {
                        cb_fired = true;
                    }
                    let _ = ui.row_gap(1, |ui| {
                        if cb_fired {
                            ui.text("on_complete fired!").fg(Color::Green);
                        }
                        if ui.button("Restart").clicked {
                            cb_tween.reset(ui.tick());
                            cb_fired = false;
                        }
                    });

                    ui.text("space tween | up/down spring | r restart all | ^Q quit")
                        .dim()
                        .fg(Color::Cyan);
                });
        },
    )
}
