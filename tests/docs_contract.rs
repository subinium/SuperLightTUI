//! Repository-level documentation and release-contract checks.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path.as_ref())
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.as_ref().display()))
}

fn package_version(manifest: &str) -> &str {
    manifest
        .lines()
        .skip_while(|line| line.trim() != "[package]")
        .skip(1)
        .find_map(|line| {
            line.trim()
                .strip_prefix("version = \"")
                .and_then(|value| value.strip_suffix('"'))
        })
        .expect("Cargo.toml package version")
}

fn example_targets(manifest: &str) -> BTreeSet<String> {
    let mut targets = BTreeSet::new();
    let mut in_example = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line == "[[example]]" {
            in_example = true;
            continue;
        }
        if line.starts_with("[[") {
            in_example = false;
        }
        if in_example
            && let Some(name) = line
                .strip_prefix("name = \"")
                .and_then(|value| value.strip_suffix('"'))
        {
            targets.insert(name.to_string());
        }
    }
    targets
}

fn collect_files(dir: &Path, extension: &str, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("failed to read directory {}: {error}", dir.display()))
    {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            collect_files(&path, extension, out);
        } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
            out.push(path);
        }
    }
}

fn advertised_examples(text: &str) -> impl Iterator<Item = &str> {
    const NEEDLE: &str = "cargo run --example ";
    text.match_indices(NEEDLE).filter_map(|(start, _)| {
        let tail = &text[start + NEEDLE.len()..];
        let len = tail
            .bytes()
            .take_while(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
            .count();
        (len > 0).then_some(&tail[..len])
    })
}

#[test]
fn every_advertised_example_is_a_registered_target() {
    let root = repo_root();
    let manifest = read(root.join("Cargo.toml"));
    let targets = example_targets(&manifest);
    let mut files = vec![root.join("README.md"), root.join("CONTRIBUTING.md")];
    collect_files(&root.join("docs"), "md", &mut files);
    collect_files(&root.join("examples"), "rs", &mut files);
    files.push(root.join("scripts/ghostty_demos.sh"));

    let mut invalid = Vec::new();
    for path in files {
        let text = read(&path);
        for name in advertised_examples(&text) {
            if !targets.contains(name) {
                invalid.push(format!(
                    "{}: {name}",
                    path.strip_prefix(&root).unwrap().display()
                ));
            }
        }
    }

    assert!(
        invalid.is_empty(),
        "documentation advertises non-target examples:\n{}",
        invalid.join("\n")
    );
}

#[test]
fn release_documents_track_the_package_minor() {
    let root = repo_root();
    let manifest = read(root.join("Cargo.toml"));
    let version = package_version(&manifest);
    let minor = version.rsplit_once('.').map_or(version, |(minor, _)| minor);

    let readme = read(root.join("README.md"));
    assert!(
        readme.contains(&format!("version = \"{version}\"")),
        "README dependency snippet must use exact package version {version}"
    );

    let security = read(root.join("SECURITY.md"));
    assert!(
        security.contains(&format!("| {minor}.x")),
        "SECURITY.md must support the current {minor}.x line"
    );
}

#[test]
fn release_smoke_uses_an_exact_registry_version() {
    let script = read(repo_root().join("scripts/smoke_release.sh"));
    assert!(script.contains("superlighttui@=${VERSION}"));
    assert!(!script.contains("path ="));
}
