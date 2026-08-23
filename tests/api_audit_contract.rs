//! Regression coverage for the repository API audit parser.

#![cfg(not(windows))]

use std::fs;
use std::path::PathBuf;
use std::process::Command;

static AUDIT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
static FIXTURE_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn fixture_root() -> PathBuf {
    let unique = format!(
        "slt-api-audit-{}-{}",
        std::process::id(),
        FIXTURE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    );
    std::env::temp_dir().join(unique)
}

fn run_audit(root: &PathBuf) -> std::process::Output {
    let _guard = AUDIT_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Command::new("bash")
        .arg("scripts/api_audit.sh")
        .arg("--strict")
        .env("SLT_AUDIT_ROOT", root)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run api audit")
}

#[test]
fn multiline_attributes_do_not_hide_rustdoc_or_missing_docs() {
    let root = fixture_root();
    fs::create_dir_all(root.join("src/context")).expect("create fixture src");
    fs::create_dir_all(root.join("examples")).expect("create fixture examples");
    fs::write(
        root.join("src/api.rs"),
        r#"
/// Documented API.
#[deprecated(
    since = "0.1.0",
    note = "fixture"
)]
pub fn documented() {}

pub fn missing() {}
"#,
    )
    .expect("write fixture");

    let failed = run_audit(&root);
    let failed_stdout = String::from_utf8_lossy(&failed.stdout);
    assert!(!failed.status.success());
    assert!(failed_stdout.contains("api.rs:9"), "{failed_stdout}");
    assert!(!failed_stdout.contains("api.rs:6"), "{failed_stdout}");

    fs::write(
        root.join("src/api.rs"),
        r#"
/// Documented API.
#[deprecated(
    since = "0.1.0",
    note = "fixture"
)]
pub fn documented() {}

/// Also documented.
pub fn missing() {}
"#,
    )
    .expect("rewrite fixture");
    let passed = run_audit(&root);
    assert!(
        passed.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&passed.stdout),
        String::from_utf8_lossy(&passed.stderr)
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn duplicate_method_parser_is_portable_across_grep_and_sed_variants() {
    let root = fixture_root();
    fs::create_dir_all(root.join("src/context")).expect("create fixture context");
    fs::create_dir_all(root.join("examples")).expect("create fixture examples");
    fs::write(
        root.join("src/context/core.rs"),
        "impl Context {\n    /// Fixture.\n    pub fn collision(&self) {}\n}\n",
    )
    .expect("write context fixture");
    fs::write(
        root.join("src/context/container.rs"),
        "impl ContainerBuilder {\n    /// Fixture.\n    pub fn collision(self) {}\n}\n",
    )
    .expect("write builder fixture");

    let failed = run_audit(&root);
    let stdout = String::from_utf8_lossy(&failed.stdout);
    assert!(!failed.status.success(), "{stdout}");
    assert!(stdout.contains("collision defined on both"), "{stdout}");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn repository_strict_api_audit_is_clean() {
    let output = run_audit(&PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
