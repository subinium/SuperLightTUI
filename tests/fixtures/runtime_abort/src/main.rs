fn main() {
    if std::env::var("SLT_RUNTIME_TEST_CASE").as_deref() == Ok("abort_boundary") {
        slt::run_with(
            slt::RunConfig::default().mouse(true).handle_suspend(false),
            |ui| {
                ui.error_boundary(|_| panic!("abort-mode boundary"));
            },
        )
        .unwrap();
    } else {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let mut task = None;
            let run = slt::run_async_with::<()>(
                slt::RunConfig::default().mouse(true).handle_suspend(false),
                move |ui, _| {
                    if task.is_none() {
                        task = Some(ui.spawn(async {
                            panic!("abort-mode supervised task");
                        }));
                    }
                },
            )
            .unwrap();
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            run.cancel_and_join().await.unwrap();
        });
    }
}
