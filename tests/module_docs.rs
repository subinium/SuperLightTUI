//! Every source file under `src/` opens with a `//!` module doc (#479), so a
//! reader landing in any file learns what it owns before reading code.

use std::fs;
use std::path::{Path, PathBuf};

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|err| panic!("read {}: {err}", dir.display()));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|err| panic!("entry in {}: {err}", dir.display()))
            .path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn every_src_file_has_a_module_doc() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    assert!(
        files.len() > 100,
        "expected the full source tree under {}",
        src.display()
    );

    let mut missing: Vec<String> = files
        .iter()
        .filter(|path| {
            let text = fs::read_to_string(path)
                .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
            !text
                .lines()
                .any(|line| line.trim_start().starts_with("//!"))
        })
        .map(|path| {
            path.strip_prefix(&src)
                .unwrap_or(path)
                .display()
                .to_string()
        })
        .collect();
    missing.sort();
    assert!(
        missing.is_empty(),
        "src files without a `//!` module doc:\n  {}",
        missing.join("\n  ")
    );
}
