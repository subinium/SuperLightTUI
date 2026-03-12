use slt::anim::ease_in_out_cubic;
use slt::{Border, Context, KeyCode, Spring, Tween};

fn main() -> std::io::Result<()> {
    let mut progress_target = 0.2;
    let mut progress_tween =
        Tween::new(progress_target, progress_target, 1).easing(ease_in_out_cubic);
    progress_tween.reset(0);

    let mut spring_target = 0.0;
    let mut spring = Spring::new(0.0, 0.15, 0.85);

    slt::run(|ui: &mut Context| {
        if ui.key('q') {
            ui.quit();
        }

        if ui.key(' ') {
            let current = progress_tween.value(ui.tick());
            progress_target = if progress_target < 0.5 { 0.9 } else { 0.1 };
            progress_tween = Tween::new(current, progress_target, 12).easing(ease_in_out_cubic);
            progress_tween.reset(ui.tick());
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

        ui.bordered(Border::Rounded)
            .title("Animation Primitives")
            .pad(1)
            .gap(1)
            .col(|ui| {
                ui.bordered(Border::Single)
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

                ui.bordered(Border::Single)
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

                ui.text("space tween | up/down spring | q quit").dim();
            });
    })
}
