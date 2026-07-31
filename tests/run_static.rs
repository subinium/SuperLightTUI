#[test]
fn run_static_signature_compiles() {
    let _signature_check = || -> std::io::Result<()> {
        let mut output = slt::StaticOutput::new();
        slt::run_static(&mut output, 3, |ui| {
            ui.text("dynamic");
            ui.quit();
        })
    };
}
