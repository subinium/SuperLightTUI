#[test]
fn run_static_signature_compiles() {
    let mut output = slt::StaticOutput::new();
    output.println("Preparing...");

    let result = slt::run_static(&mut output, 3, |ui| {
        ui.text("dynamic");
        ui.quit();
    });

    assert!(result.is_ok());
}
