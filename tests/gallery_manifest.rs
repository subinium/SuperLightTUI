//! README ↔ VHS-tape gallery parity (issue #274).
//!
//! Pure string assertions — no VHS, no `vhs` binary, no feature gate, so this
//! runs in plain `cargo test`. It guards two invariants:
//!
//!  1. Every gallery asset advertised in `README.md` (`assets/<name>.png|gif`)
//!     has a sibling `<name>.tape` that can regenerate it. A drifted/stale GIF
//!     is detectable because the tape that produces it is version-controlled
//!     next to the README that ships it.
//!  2. Every `.tape` in the repo declares an `Output assets/<name>.<ext>` that
//!     the README actually references — so we never accumulate orphan tapes
//!     that render assets nobody links.
//!
//! Together these replace the manual "tmux verification for visual demos
//! before release" step with a CI-checkable contract; the VHS `gallery` job in
//! `.github/workflows/ci.yml` then proves the tapes actually render.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Repository root (the crate manifest dir).
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Extract every `assets/<name>.<ext>` reference from the README, returning the
/// bare `<name>` stems (no directory, no extension) for `png`/`gif` assets.
fn readme_asset_stems(readme: &str) -> BTreeSet<String> {
    let mut stems = BTreeSet::new();
    let needle = "assets/";
    let mut rest = readme;
    while let Some(idx) = rest.find(needle) {
        let after = &rest[idx + needle.len()..];
        // Take chars up to the first character that cannot be part of a path.
        let end = after
            .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-' || c == '.' || c == '/'))
            .unwrap_or(after.len());
        let path = &after[..end];
        rest = &after[end..];

        // Only consider image assets (png/gif) that live directly under assets/.
        if let Some(stem) = path
            .strip_suffix(".png")
            .or_else(|| path.strip_suffix(".gif"))
        {
            // Skip nested paths like `blackpink/foo` — gallery tapes are flat.
            if !stem.contains('/') {
                stems.insert(stem.to_string());
            }
        }
    }
    stems
}

/// Parse a `.tape` file's `Output <path>` line into the asset stem it renders.
fn tape_output_stem(tape: &str) -> Option<String> {
    for line in tape.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("Output ") {
            let path = rest.trim();
            let path = path.strip_prefix("assets/").unwrap_or(path);
            let stem = path
                .strip_suffix(".png")
                .or_else(|| path.strip_suffix(".gif"))
                .unwrap_or(path);
            return Some(stem.to_string());
        }
    }
    None
}

/// List every `<name>.tape` at the repo root.
fn repo_tape_stems(root: &Path) -> BTreeSet<String> {
    let mut stems = BTreeSet::new();
    for entry in fs::read_dir(root).expect("read repo root") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("tape") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                stems.insert(stem.to_string());
            }
        }
    }
    stems
}

#[test]
fn every_readme_asset_has_a_tape() {
    let root = repo_root();
    let readme = fs::read_to_string(root.join("README.md")).expect("read README.md");
    let assets = readme_asset_stems(&readme);
    let tapes = repo_tape_stems(&root);

    assert!(
        !assets.is_empty(),
        "no `assets/<name>.png|gif` references found in README.md — parser likely broke"
    );

    let missing: Vec<&String> = assets.difference(&tapes).collect();
    assert!(
        missing.is_empty(),
        "README advertises gallery assets with no `<name>.tape` to (re)generate them: {missing:?}\n\
         Add a tape mirroring an existing one (see demo_spreadsheet.tape) so the gallery has a \
         living regeneration source and the VHS CI job can render it."
    );
}

#[test]
fn every_tape_output_is_referenced_by_readme() {
    let root = repo_root();
    let readme = fs::read_to_string(root.join("README.md")).expect("read README.md");
    let assets = readme_asset_stems(&readme);

    for entry in fs::read_dir(&root).expect("read repo root") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("tape") {
            continue;
        }
        let tape = fs::read_to_string(&path).expect("read tape");
        let stem = tape_output_stem(&tape).unwrap_or_else(|| {
            panic!("tape {path:?} has no `Output <asset>` line");
        });
        assert!(
            assets.contains(&stem),
            "tape {path:?} renders `assets/{stem}.*` but README.md does not reference it — \
             orphan tape; either link the asset in the gallery or remove the tape"
        );
    }
}

#[test]
fn tape_parser_extracts_known_assets() {
    // Guard the parser itself against regressions.
    let sample = "<img src=\"assets/demo_fire.gif\" /> and assets/demo.png plus assets/sub/x.png";
    let stems = readme_asset_stems(sample);
    assert!(stems.contains("demo_fire"));
    assert!(stems.contains("demo"));
    // Nested asset paths are intentionally ignored (gallery tapes are flat).
    assert!(!stems.iter().any(|s| s.contains('/')));
}

#[test]
fn tape_output_stem_parses_output_line() {
    let tape = "# comment\nOutput assets/demo_dashboard.png\nSet Shell \"bash\"\n";
    assert_eq!(tape_output_stem(tape).as_deref(), Some("demo_dashboard"));
}
