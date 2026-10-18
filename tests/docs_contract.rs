//! Repository-level documentation and release-contract checks.

use std::collections::{BTreeMap, BTreeSet};
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

/// Source paths (relative to the repository root) of every `[[example]]`
/// target in the manifest.
fn example_target_paths(manifest: &str) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    let mut in_example = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line == "[[example]]" {
            in_example = true;
            continue;
        }
        if line.starts_with('[') {
            in_example = false;
        }
        if in_example
            && let Some(path) = line
                .strip_prefix("path = \"")
                .and_then(|value| value.strip_suffix('"'))
        {
            paths.insert(path.to_string());
        }
    }
    paths
}

/// Files a Rust source pulls in through `#[path = "..."] mod ...;` and plain
/// out-of-line `mod name;` declarations. Paths resolve the way rustc resolves
/// top-level module declarations: `#[path]` relative to the including file's
/// directory, `mod name;` to `name.rs` / `name/mod.rs` next to a
/// `main.rs` / `mod.rs` root, or under `<stem>/` for any other file.
fn module_files(file: &Path) -> Vec<PathBuf> {
    let dir = file.parent().expect("source file has a parent directory");
    let stem = file
        .file_stem()
        .and_then(|value| value.to_str())
        .expect("UTF-8 source file name");
    let mod_root = matches!(stem, "main" | "mod" | "lib");
    let mut out = Vec::new();
    let mut pending_path = false;
    for line in read(file).lines() {
        let line = line.trim();
        if let Some(target) = line
            .strip_prefix("#[path = \"")
            .and_then(|rest| rest.strip_suffix("\"]"))
        {
            out.push(dir.join(target));
            pending_path = true;
            continue;
        }
        let decl = line
            .strip_prefix("pub(crate) ")
            .or_else(|| line.strip_prefix("pub(super) "))
            .or_else(|| line.strip_prefix("pub "))
            .unwrap_or(line);
        let Some(name) = decl
            .strip_prefix("mod ")
            .and_then(|rest| rest.strip_suffix(';'))
        else {
            continue;
        };
        if std::mem::take(&mut pending_path) {
            continue;
        }
        let base = if mod_root {
            dir.to_path_buf()
        } else {
            dir.join(stem)
        };
        out.push(base.join(format!("{name}.rs")));
        out.push(base.join(name).join("mod.rs"));
    }
    out
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
fn every_example_source_is_compiled_by_some_target() {
    let root = repo_root();
    let manifest = read(root.join("Cargo.toml"));

    // Compilation roots: registered examples, integration-test crates
    // (`tests/*.rs`, `tests/*/main.rs`), and benches.
    let mut queue: Vec<PathBuf> = example_target_paths(&manifest)
        .into_iter()
        .map(|path| root.join(path))
        .collect();
    for dir in ["tests", "benches"] {
        for entry in fs::read_dir(root.join(dir)).expect("target directory") {
            let path = entry.expect("directory entry").path();
            let main = path.join("main.rs");
            if path.extension().and_then(|value| value.to_str()) == Some("rs") {
                queue.push(path);
            } else if main.is_file() {
                queue.push(main);
            }
        }
    }

    let mut reached = BTreeSet::new();
    while let Some(path) = queue.pop() {
        let Ok(path) = path.canonicalize() else {
            continue;
        };
        if reached.insert(path.clone()) {
            queue.extend(module_files(&path));
        }
    }

    let mut examples = Vec::new();
    for entry in fs::read_dir(root.join("examples")).expect("examples directory") {
        let path = entry.expect("examples entry").path();
        if path.extension().and_then(|value| value.to_str()) == Some("rs") {
            examples.push(path);
        }
    }
    let mut unreachable: Vec<String> = examples
        .iter()
        .filter(|path| !reached.contains(&path.canonicalize().expect("example path")))
        .map(|path| path.strip_prefix(&root).unwrap().display().to_string())
        .collect();
    unreachable.sort();

    assert!(
        unreachable.is_empty(),
        "examples not compiled by any [[example]] target or #[path] include:\n{}",
        unreachable.join("\n")
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

/// Pre-0.26 flat `docs/*.md` pages. Each one is now a one-line redirect stub
/// so links from published rustdoc, READMEs and external posts keep working.
const MOVED_DOCS: &[&str] = &[
    "AI_GUIDE",
    "ANIMATION",
    "API_DESIGN",
    "ARCHITECTURE",
    "BACKENDS",
    "COMPETITIVE_ANALYSIS",
    "COMPLETE_REFERENCE",
    "COOKBOOK",
    "DEBUGGING",
    "DEMO_GUIDE",
    "DESIGN_PRINCIPLES",
    "EXAMPLES",
    "FEATURES",
    "INTERACTION",
    "MIGRATION",
    "NAMING",
    "PATTERNS",
    "PERFORMANCE",
    "POSITIONING",
    "PREVIOUS_FRAME_GUIDE",
    "QUICK_START",
    "RUSTDOC_GUIDE",
    "STATE_APIS",
    "TERMINAL_COMPATIBILITY",
    "TESTING",
    "THEMING",
    "WASM",
    "WEB_DEVELOPERS",
    "WIDGETS",
];

/// The one page that explains the previous-frame rule; every other mention
/// links here (#480).
const PREVIOUS_FRAME_ANCHOR: &str = "02-state.md#the-previous-frame-rule";

/// Target of a redirect stub: a file whose entire content is the single line
/// `This page moved to [path](path).`
fn stub_target(text: &str) -> Option<&str> {
    let line = text.strip_suffix('\n').unwrap_or(text);
    if line.contains('\n') {
        return None;
    }
    let rest = line.strip_prefix("This page moved to [")?;
    let (label, rest) = rest.split_once("](")?;
    let target = rest.strip_suffix(").")?;
    (label == target && !target.is_empty()).then_some(target)
}

/// Lines outside fenced code blocks, with 1-based line numbers.
fn prose_lines(text: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut fence: Option<usize> = None;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        let ticks = trimmed.bytes().take_while(|byte| *byte == b'`').count();
        match fence {
            Some(open) if ticks >= open && trimmed[ticks..].trim().is_empty() => fence = None,
            Some(_) => {}
            None if ticks >= 3 => fence = Some(ticks),
            None => out.push((index + 1, line)),
        }
    }
    out
}

/// Relative link targets on one prose line: inline `](target)` links and
/// reference definitions `[label]: target`. Inline code spans are ignored.
fn relative_links(line: &str) -> Vec<String> {
    let mut cleaned = String::with_capacity(line.len());
    let mut in_code = false;
    for ch in line.chars() {
        if ch == '`' {
            in_code = !in_code;
        } else if !in_code {
            cleaned.push(ch);
        }
    }
    let mut targets = Vec::new();
    let trimmed = cleaned.trim_start();
    if trimmed.starts_with('[')
        && let Some((_, rest)) = trimmed.split_once("]: ")
    {
        targets.push(rest.split_whitespace().next().unwrap_or("").to_string());
    }
    let mut rest = cleaned.as_str();
    while let Some(start) = rest.find("](") {
        let tail = &rest[start + 2..];
        let end = tail.find(')').unwrap_or(tail.len());
        targets.push(
            tail[..end]
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string(),
        );
        rest = &tail[end..];
    }
    targets.retain(|target| {
        !target.is_empty()
            && !target.starts_with('#')
            && !target.starts_with('<')
            && !target.contains("://")
            && !target.starts_with("mailto:")
    });
    targets
}

/// GitHub-style heading anchor: lowercase, keep alphanumerics, `-` and `_`,
/// turn spaces into `-`, drop everything else.
fn heading_anchor(heading: &str) -> String {
    heading
        .chars()
        .filter_map(|ch| {
            if ch.is_alphanumeric() || ch == '-' || ch == '_' {
                Some(ch.to_lowercase().next().unwrap_or(ch))
            } else if ch == ' ' {
                Some('-')
            } else {
                None
            }
        })
        .collect()
}

/// Every anchor a Markdown file defines through its headings (with GitHub's
/// `-1`, `-2` suffixes for repeats) or explicit `id="..."` attributes.
fn anchors(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for (_, line) in prose_lines(text) {
        let trimmed = line.trim_start();
        let level = trimmed.bytes().take_while(|byte| *byte == b'#').count();
        if (1..=6).contains(&level) && trimmed[level..].starts_with(' ') {
            let base = heading_anchor(trimmed[level..].trim());
            let mut anchor = base.clone();
            let mut suffix = 1;
            while out.contains(&anchor) {
                anchor = format!("{base}-{suffix}");
                suffix += 1;
            }
            out.insert(anchor);
        }
        let mut rest = line;
        while let Some(start) = rest.find("id=\"") {
            let tail = &rest[start + 4..];
            let end = tail.find('"').unwrap_or(tail.len());
            out.insert(tail[..end].to_string());
            rest = &tail[end..];
        }
    }
    out
}

/// Markdown files that link into the docs tree: README*, AGENTS.md,
/// CONTRIBUTING.md, `docs/**` (plus `llms.txt`) and both skill trees.
fn linked_doc_files(root: &Path) -> Vec<PathBuf> {
    let mut files = vec![
        root.join("README.md"),
        root.join("AGENTS.md"),
        root.join("CONTRIBUTING.md"),
        root.join("docs/llms.txt"),
    ];
    collect_files(&root.join("docs"), "md", &mut files);
    collect_files(&root.join(".agents/skills"), "md", &mut files);
    collect_files(&root.join(".claude/skills"), "md", &mut files);
    files
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}

/// Broken relative links in `text` (a file in `dir`): missing files, and
/// `#anchors` that the target Markdown file does not define.
fn broken_links(text: &str, dir: &Path, self_anchors: &BTreeSet<String>) -> Vec<String> {
    let mut broken = Vec::new();
    for (number, line) in prose_lines(text) {
        for target in relative_links(line) {
            let (path, anchor) = target.split_once('#').unwrap_or((target.as_str(), ""));
            let file = dir.join(path);
            if !path.is_empty() && !file.exists() {
                broken.push(format!("{number}: missing {target}"));
                continue;
            }
            if anchor.is_empty() {
                continue;
            }
            let defined = if path.is_empty() {
                self_anchors.clone()
            } else if path.ends_with(".md") && file.is_file() {
                anchors(&read(&file))
            } else {
                continue;
            };
            if !defined.contains(anchor) {
                broken.push(format!("{number}: missing anchor {target}"));
            }
        }
    }
    broken
}

#[test]
fn link_checker_rejects_missing_files_and_anchors() {
    let dir = repo_root().join("docs");
    let none = BTreeSet::new();
    assert!(broken_links("[ok](guide/01-quick-start.md)\n", &dir, &none).is_empty());
    assert_eq!(
        broken_links("[gone](NO_SUCH_PAGE.md)\n", &dir, &none).len(),
        1
    );
    assert_eq!(
        broken_links("[bad](guide/02-state.md#no-such-heading)\n", &dir, &none).len(),
        1
    );
    assert!(broken_links("```\n[x](missing.md)\n```\n", &dir, &none).is_empty());
    assert_eq!(
        heading_anchor("8b. Theming deep dive"),
        "8b-theming-deep-dive"
    );
    assert_eq!(
        heading_anchor("Derived state with `use_memo`"),
        "derived-state-with-use_memo"
    );
}

#[test]
fn relative_doc_links_resolve() {
    let root = repo_root();
    let mut broken = Vec::new();
    for path in linked_doc_files(&root) {
        let text = read(&path);
        let dir = path.parent().expect("doc file has a parent directory");
        for problem in broken_links(&text, dir, &anchors(&text)) {
            broken.push(format!("{}:{problem}", relative(&root, &path)));
        }
    }
    assert!(
        broken.is_empty(),
        "broken relative documentation links:\n{}",
        broken.join("\n")
    );
}

#[test]
fn moved_doc_paths_are_redirect_stubs() {
    let root = repo_root();
    assert_eq!(
        stub_target("This page moved to [guide/a.md](guide/a.md).\n"),
        Some("guide/a.md")
    );
    assert_eq!(stub_target("# Title\n\nThis page moved to [a](a).\n"), None);
    assert_eq!(stub_target("This page moved to [a.md](b.md).\n"), None);

    for name in MOVED_DOCS {
        let path = root.join(format!("docs/{name}.md"));
        let text = read(&path);
        let target = stub_target(&text)
            .unwrap_or_else(|| panic!("docs/{name}.md must be a one-line redirect stub"));
        let file = target.split('#').next().unwrap_or(target);
        let resolved = root.join("docs").join(file);
        assert!(
            resolved.is_file(),
            "docs/{name}.md redirects to missing {target}"
        );
        assert!(
            stub_target(&read(&resolved)).is_none(),
            "docs/{name}.md redirects to another stub ({target})"
        );
    }
}

#[test]
fn docs_index_lists_every_maintained_guide() {
    let root = repo_root();
    let docs = root.join("docs");
    let index = read(docs.join("README.md"));
    let mut pages = Vec::new();
    collect_files(&docs, "md", &mut pages);

    let mut missing = Vec::new();
    for path in pages {
        let link = relative(&docs, &path);
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .expect("UTF-8 documentation filename");
        if name == "README.md" || name.starts_with("README.") || link.starts_with("archive/") {
            continue;
        }
        if stub_target(&read(&path)).is_some() {
            continue;
        }
        if !index.contains(&format!("({link})")) && !index.contains(&format!("({link}#")) {
            missing.push(link);
        }
    }
    assert!(
        missing.is_empty(),
        "docs/README.md must link every maintained page:\n{}",
        missing.join("\n")
    );
}

/// Prose lines that mention the previous-frame rule without linking the
/// canonical explanation. Headings are exempt so their anchors stay stable.
fn unlinked_previous_frame_mentions(text: &str) -> Vec<usize> {
    prose_lines(text)
        .into_iter()
        .filter(|(_, line)| {
            let lower = line.to_lowercase();
            (lower.contains("previous frame") || lower.contains("previous-frame"))
                && !line.trim_start().starts_with('#')
                && !line.contains(PREVIOUS_FRAME_ANCHOR)
        })
        .map(|(number, _)| number)
        .collect()
}

#[test]
fn previous_frame_rule_is_explained_once() {
    assert_eq!(
        unlinked_previous_frame_mentions("Rects come from the previous frame.\n"),
        vec![1]
    );
    assert!(
        unlinked_previous_frame_mentions(
            "See the [previous-frame rule](guide/02-state.md#the-previous-frame-rule).\n"
        )
        .is_empty()
    );

    let root = repo_root();
    let docs = root.join("docs");
    let canonical = docs.join("guide/02-state.md");
    assert!(
        anchors(&read(&canonical)).contains("the-previous-frame-rule"),
        "docs/guide/02-state.md must keep the canonical previous-frame section"
    );
    let mut files = vec![root.join("README.md"), docs.join("llms.txt")];
    collect_files(&docs, "md", &mut files);
    let mut unlinked = Vec::new();
    for path in files {
        let link = relative(&docs, &path);
        if path == canonical || link.starts_with("archive/") {
            continue;
        }
        for number in unlinked_previous_frame_mentions(&read(&path)) {
            unlinked.push(format!("{}:{number}", relative(&root, &path)));
        }
    }
    assert!(
        unlinked.is_empty(),
        "previous-frame mentions must link {PREVIOUS_FRAME_ANCHOR} instead of re-explaining it:\n{}",
        unlinked.join("\n")
    );
}

#[test]
fn maintained_recipes_use_encapsulated_state_accessors() {
    let root = repo_root();
    let files = [
        "docs/reference/cookbook.md",
        "docs/reference/complete-reference.md",
        "docs/reference/state.md",
        "docs/reference/widgets.md",
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
        "docs/reference/cookbook.md",
        "docs/guide/03-patterns.md",
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
        "docs/internals/design-principles.md",
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
    let guide = read(root.join("docs/reference/state.md"));
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
                    "docs/reference/state.md must document public widget state {name}"
                );
            }
        }
    }
}

/// `required-features` of every `[[example]]` target that declares them.
fn example_required_features(manifest: &str) -> BTreeMap<String, Vec<String>> {
    let mut out = BTreeMap::new();
    let mut name: Option<String> = None;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            name = None;
            if line == "[[example]]" {
                name = Some(String::new());
            }
            continue;
        }
        let Some(current) = name.as_mut() else {
            continue;
        };
        if let Some(value) = line
            .strip_prefix("name = \"")
            .and_then(|value| value.strip_suffix('"'))
        {
            *current = value.to_string();
        } else if let Some(list) = line
            .strip_prefix("required-features = [")
            .and_then(|value| value.strip_suffix(']'))
        {
            let features = list
                .split(',')
                .map(|feature| feature.trim().trim_matches('"').to_string())
                .filter(|feature| !feature.is_empty())
                .collect();
            out.insert(current.clone(), features);
        }
    }
    out
}

/// Features a single command line enables with `--features a,b` /
/// `--features "a b"` / `-F a`; `None` means `--all-features`.
fn enabled_features(command: &str) -> Option<BTreeSet<String>> {
    if command.contains("--all-features") {
        return None;
    }
    let mut enabled = BTreeSet::new();
    let words: Vec<&str> = command.split_whitespace().collect();
    for (index, word) in words.iter().enumerate() {
        let value = if let Some(value) = word.strip_prefix("--features=") {
            Some(value.to_string())
        } else if *word == "--features" || *word == "-F" {
            let mut value = String::new();
            for next in &words[index + 1..] {
                value.push_str(next);
                value.push(' ');
                let opens_quote = value.starts_with(['"', '\'']);
                let closed = value.trim_end().len() > 1 && value.trim_end().ends_with(['"', '\'']);
                if !opens_quote || closed {
                    break;
                }
            }
            Some(value)
        } else {
            None
        };
        if let Some(value) = value {
            for feature in value
                .trim_matches(|ch: char| ch == '"' || ch == '\'' || ch == '`' || ch.is_whitespace())
                .split(|ch: char| ch == ',' || ch.is_whitespace())
            {
                let feature = feature.trim_start_matches(['"', '\'', '`']);
                let len = feature
                    .bytes()
                    .take_while(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
                    .count();
                let feature = &feature[..len];
                if !feature.is_empty() {
                    enabled.insert(feature.to_string());
                }
            }
        }
    }
    Some(enabled)
}

/// `cargo run --example X` lines that omit a required feature of `X`.
fn missing_example_features(text: &str, required: &BTreeMap<String, Vec<String>>) -> Vec<String> {
    let mut missing = Vec::new();
    for (index, line) in text.lines().enumerate() {
        for (start, _) in line.match_indices("cargo run") {
            let command = &line[start..];
            // A line may hold several commands (`a && b`, table cells, prose).
            let command = command
                .split(['|', ';'])
                .next()
                .unwrap_or(command)
                .split("&&")
                .next()
                .unwrap_or(command)
                .split('`')
                .next()
                .unwrap_or(command);
            let Some(name) = advertised_examples(command).next() else {
                continue;
            };
            let Some(features) = required.get(name) else {
                continue;
            };
            let Some(enabled) = enabled_features(command) else {
                continue;
            };
            let absent: Vec<&str> = features
                .iter()
                .filter(|feature| !enabled.contains(*feature) && !enabled.contains("full"))
                .map(String::as_str)
                .collect();
            if !absent.is_empty() {
                missing.push(format!(
                    "{}: `{}` needs --features {}",
                    index + 1,
                    command.trim(),
                    absent.join(",")
                ));
            }
        }
    }
    missing
}

/// Files that tell users how to run examples or depend on the crate.
fn user_facing_text_files(root: &Path) -> Vec<PathBuf> {
    let mut files = vec![
        root.join("README.md"),
        root.join("CONTRIBUTING.md"),
        root.join("AGENTS.md"),
        root.join("docs/llms.txt"),
    ];
    collect_files(&root.join("docs"), "md", &mut files);
    collect_files(&root.join(".agents/skills"), "md", &mut files);
    collect_files(&root.join(".claude/skills"), "md", &mut files);
    files
}

#[test]
fn advertised_examples_enable_required_features() {
    let required = BTreeMap::from([("system_tour".to_string(), vec!["async".to_string()])]);
    assert_eq!(
        missing_example_features("cargo run --example system_tour\n", &required).len(),
        1
    );
    for ok in [
        "cargo run --example system_tour --features async",
        "cargo run --example system_tour --features \"async image\"",
        "cargo run --example system_tour --features=serde,async",
        "cargo run --example system_tour --all-features",
        "`cargo run --example system_tour -F async` | `cargo run --example hello`",
        "ui.text(\"cargo run --example system_tour --features async\").bold();",
    ] {
        assert!(
            missing_example_features(ok, &required).is_empty(),
            "{ok} should be accepted"
        );
    }

    let root = repo_root();
    let manifest = read(root.join("Cargo.toml"));
    let required = example_required_features(&manifest);
    assert!(
        required
            .get("system_tour")
            .is_some_and(|features| features.iter().any(|f| f == "async")),
        "manifest parser must see system_tour's required-features"
    );
    let mut files = user_facing_text_files(&root);
    collect_files(&root.join("examples"), "rs", &mut files);
    let mut missing = Vec::new();
    for path in files {
        for problem in missing_example_features(&read(&path), &required) {
            missing.push(format!("{}:{problem}", relative(&root, &path)));
        }
    }
    assert!(
        missing.is_empty(),
        "advertised examples must enable their required-features:\n{}",
        missing.join("\n")
    );
}

/// `major.minor` of a version requirement such as `0.25`, `=0.25.0`, `^0.26`.
fn requirement_minor(requirement: &str) -> Option<String> {
    let version = requirement.trim_start_matches(['=', '^', '~', ' ']);
    let mut parts = version.split('.');
    let major: u64 = parts.next()?.parse().ok()?;
    let minor: u64 = parts.next()?.parse().ok()?;
    Some(format!("{major}.{minor}"))
}

/// Dependency pins on `superlighttui` whose minor is neither the package
/// minor nor the next minor (pages written for the unreleased version
/// before the release bump). `"..."` placeholders are allowed.
fn stale_pins(text: &str, package: &str) -> Vec<String> {
    let current = requirement_minor(package).expect("package version");
    let (major, minor) = current.split_once('.').expect("major.minor");
    let next = format!("{major}.{}", minor.parse::<u64>().expect("minor") + 1);
    let mut stale = Vec::new();
    for (index, line) in text.lines().enumerate() {
        for needle in ["superlighttui = \"", "superlighttui = { version = \""] {
            for (start, _) in line.match_indices(needle) {
                let tail = &line[start + needle.len()..];
                let requirement = &tail[..tail.find('"').unwrap_or(tail.len())];
                if requirement == "..." {
                    continue;
                }
                match requirement_minor(requirement) {
                    Some(found) if found == current || found == next => {}
                    _ => stale.push(format!("{}: superlighttui {requirement}", index + 1)),
                }
            }
        }
    }
    stale
}

#[test]
fn dependency_pins_track_the_package_minor() {
    assert_eq!(
        stale_pins("superlighttui = \"0.23.0\"\n", "0.25.0").len(),
        1
    );
    assert!(stale_pins("superlighttui = \"0.25\"\n", "0.25.0").is_empty());
    assert!(
        stale_pins(
            "superlighttui = { version = \"0.26\", features = [\"ratatui\"] }\n",
            "0.25.0"
        )
        .is_empty()
    );
    assert_eq!(stale_pins("superlighttui = \"0.25\"\n", "0.26.0").len(), 1);

    let root = repo_root();
    let manifest = read(root.join("Cargo.toml"));
    let version = package_version(&manifest);
    let mut stale = Vec::new();
    for path in user_facing_text_files(&root) {
        let link = relative(&root, &path);
        // Version-by-version upgrade notes and dated snapshots name old
        // releases on purpose.
        if link == "docs/reference/migration.md" || link.starts_with("docs/archive/") {
            continue;
        }
        for problem in stale_pins(&read(&path), version) {
            stale.push(format!("{link}:{problem}"));
        }
    }
    assert!(
        stale.is_empty(),
        "superlighttui dependency pins must use the {version} minor (or the next, unreleased one):\n{}",
        stale.join("\n")
    );
}

/// Relative file paths under `dir`, mapped to their bytes.
fn tree_bytes(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir)
            .unwrap_or_else(|error| panic!("failed to read directory {}: {error}", dir.display()))
        {
            let path = entry.expect("directory entry").path();
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                let bytes = fs::read(&path).expect("skill file");
                out.insert(relative(base, &path), bytes);
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

/// Differences between two skill trees: files missing on either side or
/// with different bytes.
fn tree_differences(
    left: &BTreeMap<String, Vec<u8>>,
    right: &BTreeMap<String, Vec<u8>>,
) -> Vec<String> {
    let mut out = Vec::new();
    for (path, bytes) in left {
        match right.get(path) {
            None => out.push(format!("only in .agents/skills: {path}")),
            Some(other) if other != bytes => out.push(format!("differs: {path}")),
            Some(_) => {}
        }
    }
    for path in right.keys().filter(|path| !left.contains_key(*path)) {
        out.push(format!("only in .claude/skills: {path}"));
    }
    out
}

#[test]
fn agent_skill_copies_are_identical() {
    let a = BTreeMap::from([("slt/SKILL.md".to_string(), b"one".to_vec())]);
    let b = BTreeMap::from([("slt/SKILL.md".to_string(), b"two".to_vec())]);
    assert_eq!(tree_differences(&a, &b), vec!["differs: slt/SKILL.md"]);
    assert_eq!(tree_differences(&a, &BTreeMap::new()).len(), 1);

    let root = repo_root();
    let agents = tree_bytes(&root.join(".agents/skills"));
    let claude = tree_bytes(&root.join(".claude/skills"));
    assert!(!agents.is_empty(), ".agents/skills must not be empty");
    let differences = tree_differences(&agents, &claude);
    assert!(
        differences.is_empty(),
        ".agents/skills and .claude/skills must be byte-identical:\n{}",
        differences.join("\n")
    );
}
