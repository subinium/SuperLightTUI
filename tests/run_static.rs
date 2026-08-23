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

#[test]
fn zero_height_inline_modes_fail_before_terminal_access() {
    let inline_err = slt::run_inline(0, |_| {}).unwrap_err();
    assert_eq!(inline_err.kind(), std::io::ErrorKind::InvalidInput);

    let mut output = slt::StaticOutput::new();
    let static_err = slt::run_static(&mut output, 0, |_| {}).unwrap_err();
    assert_eq!(static_err.kind(), std::io::ErrorKind::InvalidInput);
}
