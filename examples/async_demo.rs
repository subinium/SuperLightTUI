use slt::{Border, Color, Context};
use tokio::time::{sleep, Duration};

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
    let mut updates: Vec<String> = Vec::new();
    let mut total_updates: u64 = 0;

    let tx = slt::run_async(move |ui: &mut Context, messages: &mut Vec<String>| {
        if ui.key_mod('q', slt::KeyModifiers::CONTROL) || ui.key_code(slt::KeyCode::Esc) {
            ui.quit();
        }

        for message in messages.drain(..) {
            total_updates = total_updates.wrapping_add(1);
            updates.push(message);
        }

        const MAX_UPDATES: usize = 6;
        if updates.len() > MAX_UPDATES {
            let overflow = updates.len() - MAX_UPDATES;
            updates.drain(0..overflow);
        }

        let _ = ui
            .bordered(Border::Rounded)
            .title("Async Demo")
            .pad(1)
            .gap(1)
            .col(|ui| {
                ui.text("Background updates from tokio task")
                    .bold()
                    .fg(Color::Cyan);
                ui.text(format!("Total updates: {total_updates}"));

                if updates.is_empty() {
                    ui.text("Waiting for updates...").dim();
                } else {
                    for update in &updates {
                        ui.text(update);
                    }
                }

                ui.text("q = quit").dim();
            });
    })?;

    let producer = tokio::spawn(async move {
        let mut counter: u64 = 1;
        loop {
            let message = format!("Status update #{counter}");
            if tx.send(message).await.is_err() {
                break;
            }
            counter = counter.wrapping_add(1);
            sleep(Duration::from_secs(2)).await;
        }
    });

    let _ = producer.await;
    Ok(())
}
