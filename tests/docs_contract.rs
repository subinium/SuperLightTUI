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

#[test]
fn docs_index_lists_every_maintained_guide() {
    let root = repo_root();
    let index = read(root.join("docs/README.md"));

    for entry in fs::read_dir(root.join("docs")).expect("docs directory") {
        let path = entry.expect("docs entry").path();
        if path.extension().and_then(|value| value.to_str()) != Some("md") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .expect("UTF-8 documentation filename");
        if name == "README.md" || name.starts_with("README.") {
            continue;
        }
        assert!(
            index.contains(&format!("({name})")),
            "docs/README.md must link maintained guide {name}"
        );
    }
}

#[test]
fn maintained_recipes_use_encapsulated_state_accessors() {
    let root = repo_root();
    let files = [
        "docs/COOKBOOK.md",
        "docs/COMPLETE_REFERENCE.md",
        "docs/STATE_APIS.md",
        "docs/WIDGETS.md",
    ];
    let forbidden = [
        "table.rows.",
        "table.rows[",
        "table.headers[",
        "list.items[",
        "palette.commands[",
    ];

    for relative in files {
        let text = read(root.join(relative));
        for pattern in forbidden {
            assert!(
                !text.contains(pattern),
                "{relative} must use a public state accessor instead of `{pattern}`"
            );
        }
    }
}

#[test]
fn maintained_recipes_avoid_deprecated_aliases() {
    let root = repo_root();
    let files = [
        "docs/COOKBOOK.md",
        "docs/PATTERNS.md",
        ".agents/skills/slt-migration/SKILL.md",
    ];

    for relative in files {
        let text = read(root.join(relative));
        assert!(
            !text.contains(".pad("),
            "{relative} must use the canonical .p(...) builder"
        );
        assert!(
            !text.contains("key_seq("),
            "{relative} must use key_chord(...)"
        );
    }
}

#[test]
fn agent_reference_tracks_example_inventory() {
    let root = repo_root();
    let manifest = read(root.join("Cargo.toml"));
    let target_count = example_targets(&manifest).len();
    let mut files = Vec::new();
    collect_files(&root.join("examples"), "rs", &mut files);

    let reference = read(root.join(".agents/skills/slt/REFERENCES.md"));
    let inventory = format!(
        "{} Rust files, {} Cargo-listed targets",
        files.len(),
        target_count
    );
    assert!(
        reference.contains(&inventory),
        "SLT agent reference must track the current example inventory: {inventory}"
    );
}

#[test]
fn project_docs_point_to_agents_instructions() {
    let root = repo_root();
    for relative in [
        "docs/DESIGN_PRINCIPLES.md",
        ".agents/skills/slt/REFERENCES.md",
        ".agents/skills/slt-migration/SKILL.md",
    ] {
        let text = read(root.join(relative));
        assert!(
            !text.contains("CLAUDE.md") && !text.contains(".Codex/skills"),
            "{relative} must point to AGENTS.md and .agents/skills"
        );
    }
}

#[test]
fn state_api_guide_lists_every_public_widget_state() {
    let root = repo_root();
    let guide = read(root.join("docs/STATE_APIS.md"));
    let mut sources = Vec::new();
    collect_files(&root.join("src/widgets"), "rs", &mut sources);

    for path in sources {
        for line in read(&path).lines() {
            let Some(rest) = line.trim().strip_prefix("pub struct ") else {
                continue;
            };
            let name = rest
                .split(|ch: char| ch == '<' || ch == '{' || ch.is_whitespace())
                .next()
                .expect("public struct name");
            if name.ends_with("State") {
                assert!(
                    guide.contains(&format!("## {name}")),
                    "docs/STATE_APIS.md must document public widget state {name}"
                );
            }
        }
    }
}
