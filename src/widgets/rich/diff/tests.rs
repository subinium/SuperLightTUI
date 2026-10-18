//! Unit and property tests for unified-diff parsing.

use super::*;
use proptest::prelude::*;

const MODIFY: &str = "\
diff --git a/src/lib.rs b/src/lib.rs
index 83db48f..bf269f4 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,4 +1,5 @@ fn main() {
 fn main() {
-    println!(\"old\");
+    println!(\"new\");
+    println!(\"extra\");
     let x = 1;
 }
";

const ADD_ONLY: &str = "\
diff --git a/new.txt b/new.txt
new file mode 100644
index 0000000..3b18e51
--- /dev/null
+++ b/new.txt
@@ -0,0 +1,2 @@
+hello
+world
";

const DELETE_ONLY: &str = "\
diff --git a/gone.txt b/gone.txt
deleted file mode 100644
index 3b18e51..0000000
--- a/gone.txt
+++ /dev/null
@@ -1,2 +0,0 @@
-hello
-world
";

const RENAME: &str = "\
diff --git a/old/name.rs b/new/name.rs
similarity index 90%
rename from old/name.rs
rename to new/name.rs
index 1111111..2222222 100644
--- a/old/name.rs
+++ b/new/name.rs
@@ -3 +3 @@
-let a = 1;
+let a = 2;
";

const PURE_RENAME: &str = "\
diff --git a/a.txt b/b.txt
similarity index 100%
rename from a.txt
rename to b.txt
";

const BINARY: &str = "\
diff --git a/logo.png b/logo.png
index 1111111..2222222 100644
Binary files a/logo.png and b/logo.png differ
diff --git a/img.bin b/img.bin
new file mode 100644
index 0000000..1234567
GIT binary patch
literal 12
TcmZQzU|?WiU|?WiVE_X800

literal 0
HcmV?d00001

diff --git a/after.txt b/after.txt
--- a/after.txt
+++ b/after.txt
@@ -1 +1 @@
-x
+y
";

const NO_NEWLINE: &str = "\
--- a/f.txt
+++ b/f.txt
@@ -1,2 +1,2 @@
 keep
-old
\\ No newline at end of file
+new
\\ No newline at end of file
";

#[test]
fn parses_modified_file_with_line_numbers() {
    let diff = DiffState::from_unified(MODIFY).unwrap();
    assert_eq!(diff.files().len(), 1);
    let file = &diff.files()[0];
    assert_eq!(file.old_path.as_deref(), Some("src/lib.rs"));
    assert_eq!(file.new_path.as_deref(), Some("src/lib.rs"));
    assert_eq!(file.status, DiffFileStatus::Modified);
    assert_eq!(file.lang_hint(), "rs");
    let hunk = &file.hunks[0];
    assert_eq!(
        (
            hunk.old_start,
            hunk.old_count,
            hunk.new_start,
            hunk.new_count
        ),
        (1, 4, 1, 5)
    );
    assert_eq!(hunk.section, "fn main() {");
    let summary: Vec<_> = hunk
        .lines
        .iter()
        .map(|l| (l.kind, l.old_line, l.new_line))
        .collect();
    assert_eq!(
        summary,
        vec![
            (DiffLineKind::Context, Some(1), Some(1)),
            (DiffLineKind::Removed, Some(2), None),
            (DiffLineKind::Added, None, Some(2)),
            (DiffLineKind::Added, None, Some(3)),
            (DiffLineKind::Context, Some(3), Some(4)),
            (DiffLineKind::Context, Some(4), Some(5)),
        ]
    );
    assert_eq!(hunk.lines[1].text, "    println!(\"old\");");
    assert_eq!((hunk.added(), hunk.removed()), (2, 1));
}

#[test]
fn parses_add_only_file() {
    let diff = DiffState::from_unified(ADD_ONLY).unwrap();
    let file = &diff.files()[0];
    assert_eq!(file.status, DiffFileStatus::Added);
    assert_eq!(file.old_path, None);
    assert_eq!(file.new_path.as_deref(), Some("new.txt"));
    let lines = &file.hunks[0].lines;
    assert_eq!(lines.len(), 2);
    assert!(lines.iter().all(|l| l.kind == DiffLineKind::Added));
    assert_eq!(lines[1].new_line, Some(2));
}

#[test]
fn parses_delete_only_file() {
    let diff = DiffState::from_unified(DELETE_ONLY).unwrap();
    let file = &diff.files()[0];
    assert_eq!(file.status, DiffFileStatus::Deleted);
    assert_eq!(file.old_path.as_deref(), Some("gone.txt"));
    assert_eq!(file.new_path, None);
    assert_eq!(file.path(), "gone.txt");
    assert!(
        file.hunks[0]
            .lines
            .iter()
            .all(|l| l.kind == DiffLineKind::Removed)
    );
}

#[test]
fn parses_multi_file_patch_in_order() {
    let patch = format!("{MODIFY}{ADD_ONLY}{DELETE_ONLY}");
    let diff = DiffState::from_unified(&patch).unwrap();
    let paths: Vec<_> = diff.files().iter().map(DiffFile::path).collect();
    assert_eq!(paths, ["src/lib.rs", "new.txt", "gone.txt"]);
    assert_eq!(diff.hunk_count(), 3);
    let ids: Vec<_> = diff.hunk_ids().collect();
    assert_eq!(ids[2], HunkId { file: 2, hunk: 0 });
}

#[test]
fn parses_renames_with_and_without_hunks() {
    let diff = DiffState::from_unified(RENAME).unwrap();
    let file = &diff.files()[0];
    assert_eq!(file.status, DiffFileStatus::Renamed);
    assert_eq!(file.old_path.as_deref(), Some("old/name.rs"));
    assert_eq!(file.new_path.as_deref(), Some("new/name.rs"));
    assert_eq!(file.hunks[0].lines[0].old_line, Some(3));

    let diff = DiffState::from_unified(PURE_RENAME).unwrap();
    let file = &diff.files()[0];
    assert_eq!(file.status, DiffFileStatus::Renamed);
    assert_eq!(file.old_path.as_deref(), Some("a.txt"));
    assert_eq!(file.new_path.as_deref(), Some("b.txt"));
    assert!(file.hunks.is_empty());
}

#[test]
fn reports_binary_files_without_content() {
    let diff = DiffState::from_unified(BINARY).unwrap();
    assert_eq!(diff.files().len(), 3);
    assert!(diff.files()[0].binary);
    assert!(diff.files()[0].hunks.is_empty());
    let literal = &diff.files()[1];
    assert!(literal.binary);
    assert_eq!(literal.status, DiffFileStatus::Added);
    assert!(
        literal.hunks.is_empty(),
        "base85 payload must not parse as hunks"
    );
    let after = &diff.files()[2];
    assert!(!after.binary);
    assert_eq!(after.hunks.len(), 1);
}

#[test]
fn diff_r_binary_notice_is_its_own_file() {
    let patch = "--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\nBinary files a/y.bin and b/y.bin differ\n";
    let diff = DiffState::from_unified(patch).unwrap();
    assert_eq!(diff.files().len(), 2);
    assert!(!diff.files()[0].binary);
    assert!(diff.files()[1].binary);
    assert_eq!(diff.files()[1].path(), "y.bin");
}

#[test]
fn parses_no_newline_markers_on_both_sides() {
    let diff = DiffState::from_unified(NO_NEWLINE).unwrap();
    let lines = &diff.files()[0].hunks[0].lines;
    assert_eq!(lines.len(), 3);
    assert!(!lines[0].no_newline);
    assert!(lines[1].no_newline && lines[1].kind == DiffLineKind::Removed);
    assert!(lines[2].no_newline && lines[2].kind == DiffLineKind::Added);
}

#[test]
fn plain_diff_u_with_timestamps_and_preamble() {
    let patch = "\
From 1234 Mon Sep 17 00:00:00 2001
Subject: [PATCH] tweak

---
 f.c | 2 +-

--- f.c.orig\t2024-01-01 00:00:00.000000000 +0000
+++ f.c\t2024-01-02 00:00:00.000000000 +0000
@@ -1 +1 @@
-int a;
+int b;
--
2.40.1
";
    let diff = DiffState::from_unified(patch).unwrap();
    assert_eq!(diff.files().len(), 1);
    assert_eq!(diff.files()[0].old_path.as_deref(), Some("f.c.orig"));
    assert_eq!(diff.files()[0].new_path.as_deref(), Some("f.c"));
    assert_eq!(diff.files()[0].hunks[0].lines.len(), 2);
}

#[test]
fn crlf_patches_parse_like_lf() {
    let crlf = MODIFY.replace('\n', "\r\n");
    let a = DiffState::from_unified(MODIFY).unwrap();
    let b = DiffState::from_unified(&crlf).unwrap();
    assert_eq!(a.files(), b.files());
}

#[test]
fn quoted_paths_are_unquoted() {
    let patch = "diff --git \"a/sp ace\\303\\244.txt\" \"b/sp ace\\303\\244.txt\"\n--- \"a/sp ace\\303\\244.txt\"\n+++ \"b/sp ace\\303\\244.txt\"\n@@ -1 +1 @@\n-a\n+b\n";
    let diff = DiffState::from_unified(patch).unwrap();
    assert_eq!(diff.files()[0].path(), "sp aceä.txt");
}

#[test]
fn git_header_with_spaces_splits_in_the_middle() {
    let patch = "diff --git a/my file.txt b/my file.txt\nnew file mode 100644\n";
    let diff = DiffState::from_unified(patch).unwrap();
    assert_eq!(diff.files()[0].path(), "my file.txt");
    assert_eq!(diff.files()[0].status, DiffFileStatus::Added);
}

#[test]
fn bare_hunks_without_headers_parse() {
    let diff = DiffState::from_unified("@@ -1 +1 @@\n-a\n+b\n").unwrap();
    assert_eq!(diff.files().len(), 1);
    assert_eq!(diff.files()[0].path(), "");
    assert_eq!(diff.to_unified(), "@@ -1,1 +1,1 @@\n-a\n+b\n");
}

#[test]
fn empty_line_inside_hunk_is_empty_context() {
    let diff = DiffState::from_unified("@@ -1,3 +1,3 @@\n a\n\n-b\n+c\n").unwrap();
    let lines = &diff.files()[0].hunks[0].lines;
    assert_eq!(lines[1].kind, DiffLineKind::Context);
    assert_eq!(lines[1].text, "");
}

#[test]
fn empty_input_is_an_empty_diff() {
    assert!(DiffState::from_unified("").unwrap().is_empty());
    assert!(DiffState::from_unified("\n  \n").unwrap().is_empty());
}

fn parse_err(input: &str) -> DiffParseError {
    DiffState::from_unified(input).expect_err("input must be rejected")
}

#[test]
fn malformed_input_returns_errors() {
    let e = parse_err("just some text\nwith no diff\n");
    assert_eq!(e.kind(), DiffParseErrorKind::NotADiff);

    let e = parse_err("--- a/x\n+++ b/x\n@@ -1,x +1 @@\n-a\n");
    assert_eq!(
        (e.kind(), e.line()),
        (DiffParseErrorKind::InvalidHunkHeader, 3)
    );

    let e = parse_err("@@@ -1,2 -1,2 +1,3 @@@\n");
    assert_eq!(e.kind(), DiffParseErrorKind::InvalidHunkHeader);

    let e = parse_err("@@ -1,2 +1,2 @@\n a\n");
    assert_eq!((e.kind(), e.line()), (DiffParseErrorKind::TruncatedHunk, 1));

    let e = parse_err("@@ -1,2 +1,2 @@\n a\n?b\n");
    assert_eq!(
        (e.kind(), e.line()),
        (DiffParseErrorKind::UnexpectedLine, 3)
    );

    // More removed lines than the header announced.
    let e = parse_err("@@ -1 +1,2 @@\n-a\n-b\n+c\n");
    assert_eq!(
        (e.kind(), e.line()),
        (DiffParseErrorKind::UnexpectedLine, 3)
    );

    let e = parse_err("@@ -1 +1 @@\n\\ No newline at end of file\n-a\n+b\n");
    assert_eq!(e.kind(), DiffParseErrorKind::OrphanNoNewlineMarker);

    let e = parse_err("@@ -4294967295,2 +1 @@\n");
    assert_eq!(e.kind(), DiffParseErrorKind::LineNumberOverflow);

    let e = parse_err("@@ -+1 +1 @@\n-a\n+b\n");
    assert_eq!(e.kind(), DiffParseErrorKind::InvalidHunkHeader);
    assert_eq!(e.to_string(), "invalid hunk header at line 1");
}

#[test]
fn fixtures_round_trip_through_to_unified() {
    for fixture in [
        MODIFY,
        ADD_ONLY,
        DELETE_ONLY,
        RENAME,
        PURE_RENAME,
        BINARY,
        NO_NEWLINE,
    ] {
        let first = DiffState::from_unified(fixture).unwrap();
        let again = DiffState::from_unified(&first.to_unified()).unwrap();
        assert_eq!(first.files(), again.files(), "fixture:\n{fixture}");
    }
}

#[test]
fn review_state_accessors() {
    let patch = format!("{MODIFY}{ADD_ONLY}");
    let mut diff = DiffState::from_unified(&patch).unwrap();
    let id = HunkId { file: 1, hunk: 0 };
    let bad = HunkId { file: 1, hunk: 9 };
    assert_eq!(diff.decision(id), Some(HunkDecision::Pending));
    assert_eq!(diff.decision(bad), None);
    assert!(diff.set_decision(id, HunkDecision::Rejected));
    assert!(!diff.set_decision(bad, HunkDecision::Rejected));
    assert_eq!(diff.decision(id), Some(HunkDecision::Rejected));
    assert!(diff.set_collapsed(id, true));
    assert!(diff.is_collapsed(id));
    diff.expand_all();
    assert!(!diff.is_collapsed(id));
    diff.collapse_all();
    assert!(diff.is_collapsed(HunkId { file: 0, hunk: 0 }));
    assert_eq!(diff.focused_hunk(), None, "no rows before first render");
    assert!(diff.focus_hunk(id));
    assert_eq!(diff.focused_hunk(), Some(id));
    assert_eq!(diff.hunk(id).map(DiffHunk::added), Some(2));
}

#[test]
fn split_rows_align_removed_and_added_runs() {
    let mut diff = DiffState::from_unified(MODIFY).unwrap();
    diff.ensure_rows(true);
    let pairs: Vec<_> = diff
        .rows
        .rows
        .iter()
        .filter_map(|row| match row {
            DiffRow::Pair { old, new, .. } => Some((*old, *new)),
            _ => None,
        })
        .collect();
    assert_eq!(
        pairs,
        vec![
            (Some(0), Some(0)),
            (Some(1), Some(2)),
            (None, Some(3)),
            (Some(4), Some(4)),
            (Some(5), Some(5)),
        ]
    );
}

// ── proptest: model -> to_unified -> parse round trip ─────────────────────

fn arb_text() -> impl Strategy<Value = String> {
    // Anything except line breaks — including diff markers and tabs.
    proptest::string::string_regex("[ -~\t가-힣é]{0,12}").unwrap()
}

fn arb_path() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[a-z0-9_.-]{1,6}(/[a-z0-9_.-]{1,6}){0,2}").unwrap()
}

fn arb_hunk_lines() -> impl Strategy<Value = Vec<(DiffLineKind, String)>> {
    let kind = prop_oneof![
        Just(DiffLineKind::Context),
        Just(DiffLineKind::Added),
        Just(DiffLineKind::Removed),
    ];
    proptest::collection::vec((kind, arb_text()), 1..8)
}

fn build_hunk(
    lines: Vec<(DiffLineKind, String)>,
    old_start: u32,
    new_start: u32,
    section: String,
    no_newline_last: bool,
) -> DiffHunk {
    let old_count = lines
        .iter()
        .filter(|(k, _)| *k != DiffLineKind::Added)
        .count() as u32;
    let new_count = lines
        .iter()
        .filter(|(k, _)| *k != DiffLineKind::Removed)
        .count() as u32;
    let (mut old_no, mut new_no) = (old_start, new_start);
    let count = lines.len();
    let lines = lines
        .into_iter()
        .enumerate()
        .map(|(i, (kind, text))| {
            let old_line = (kind != DiffLineKind::Added).then(|| {
                old_no += 1;
                old_no - 1
            });
            let new_line = (kind != DiffLineKind::Removed).then(|| {
                new_no += 1;
                new_no - 1
            });
            DiffLine {
                kind,
                text,
                old_line,
                new_line,
                no_newline: no_newline_last && i + 1 == count,
            }
        })
        .collect();
    DiffHunk {
        old_start,
        old_count,
        new_start,
        new_count,
        section,
        lines,
    }
}

fn arb_file() -> impl Strategy<Value = DiffFile> {
    let status = prop_oneof![
        Just(DiffFileStatus::Modified),
        Just(DiffFileStatus::Added),
        Just(DiffFileStatus::Deleted),
        Just(DiffFileStatus::Renamed),
        Just(DiffFileStatus::Copied),
    ];
    let hunk = (
        arb_hunk_lines(),
        1u32..500,
        1u32..500,
        proptest::string::string_regex("[a-z (){}]{0,10}").unwrap(),
    );
    (
        status,
        arb_path(),
        arb_path(),
        any::<bool>(),
        proptest::collection::vec(hunk, 0..3),
        any::<bool>(),
    )
        .prop_map(|(status, a, b, binary, hunks, no_newline)| {
            let (old_path, new_path) = match status {
                DiffFileStatus::Added => (None, Some(a)),
                DiffFileStatus::Deleted => (Some(a), None),
                DiffFileStatus::Modified => (Some(a.clone()), Some(a)),
                _ => {
                    let b = if a == b { format!("{b}.moved") } else { b };
                    (Some(a), Some(b))
                }
            };
            let hunk_total = hunks.len();
            let hunks = if binary {
                Vec::new()
            } else {
                hunks
                    .into_iter()
                    .enumerate()
                    .map(|(i, (lines, old_start, new_start, section))| {
                        build_hunk(
                            lines,
                            old_start,
                            new_start,
                            section.trim_start().to_string(),
                            no_newline && i + 1 == hunk_total,
                        )
                    })
                    .collect()
            };
            DiffFile {
                old_path,
                new_path,
                status,
                binary,
                hunks,
            }
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn render_then_parse_round_trips(files in proptest::collection::vec(arb_file(), 1..4)) {
        let text = parse::write_unified(&files);
        let parsed = DiffState::from_unified(&text)
            .map_err(|e| TestCaseError::fail(format!("{e}\n{text}")))?;
        prop_assert_eq!(parsed.files(), files.as_slice(), "patch:\n{}", text);
    }

    #[test]
    fn parser_never_panics_on_arbitrary_text(input in ".{0,400}") {
        let _ = DiffState::from_unified(&input);
    }

    #[test]
    fn parser_never_panics_on_diff_like_lines(lines in proptest::collection::vec(
        proptest::string::string_regex(
            "(diff --git a/x b/y|--- a/x|\\+\\+\\+ b/x|--- /dev/null|\\+\\+\\+ /dev/null\
|@@ -[0-9]{1,3}(,[0-9]{1,2})? \\+[0-9]{1,3}(,[0-9]{1,2})? @@[ a-z]{0,4}\
|[ +\\-\\\\][ -~]{0,6}|rename (from|to) [a-z]{1,3}|new file mode 100644\
|deleted file mode 100644|Binary files a/x and b/x differ|GIT binary patch|\"a/q\\\\\"|)"
        ).unwrap(),
        0..40,
    )) {
        let input = lines.join("\n");
        if let Ok(mut diff) = DiffState::from_unified(&input) {
            // Row building must cope with whatever the parser accepted.
            diff.ensure_rows(false);
            diff.ensure_rows(true);
            let _ = diff.to_unified();
        }
    }
}
