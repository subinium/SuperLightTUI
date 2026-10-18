//! Issue #466: chunking and highlight-cache tests for [`CodeEditorState`].

use super::*;

fn lines(text: &str) -> Vec<String> {
    text.split('\n').map(str::to_string).collect()
}

#[test]
fn chunks_split_before_unindented_lines_after_a_blank() {
    let doc = lines("fn a() {\n    1;\n}\n\nfn b() {\n    2;\n}\n\nfn c() {\n    3;\n}");
    assert_eq!(chunk_ranges(&doc, 0..doc.len()), vec![0..4, 4..8, 8..11]);
    // The chunking of a row does not depend on the requested window.
    assert_eq!(chunk_ranges(&doc, 5..6), vec![4..8]);
    assert_eq!(chunk_ranges(&doc, 3..9), vec![0..4, 4..8, 8..11]);
    assert_eq!(
        chunk_ranges(&doc, 11..20),
        Vec::<std::ops::Range<usize>>::new()
    );
}

#[test]
fn indented_lines_and_closers_do_not_start_chunks() {
    let doc = lines("impl A {\n    fn a() {}\n\n    fn b() {}\n}\n\n}\nx");
    // `    fn b` is indented and `}` is a closer, so only row 0 starts one
    // until `x` (which follows `}`, not a blank line).
    assert_eq!(chunk_ranges(&doc, 0..doc.len()), vec![0..doc.len()]);
}

#[test]
fn long_runs_without_boundaries_are_capped() {
    let mut text: Vec<String> = Vec::new();
    text.push("fn big() {".into());
    for i in 0..300 {
        text.push(format!("    let v{i} = {i};"));
        if i % 50 == 49 {
            text.push(String::new());
        }
    }
    text.push("}".into());
    let chunks = chunk_ranges(&text, 0..text.len());
    assert_eq!(chunks.first().map(|c| c.start), Some(0));
    assert_eq!(chunks.last().map(|c| c.end), Some(text.len()));
    for pair in chunks.windows(2) {
        assert_eq!(pair[0].end, pair[1].start, "chunks are contiguous");
    }
    for chunk in &chunks {
        assert!(chunk.len() <= MAX_CHUNK_LINES, "{chunk:?} exceeds the cap");
    }
    // Forced splits prefer the line after a blank line.
    assert!(is_blank(&text[chunks[1].start - 1]));
}

#[test]
fn indent_dedent_respect_tab_width_and_hard_tabs() {
    let cfg = IndentConfig {
        tab_width: 2,
        hard_tabs: false,
        tab_indents: true,
        auto_indent: true,
    };
    let mut ta = TextareaState::new();
    ta.set_value("a\nb");
    ta.set_selection((0, 0), (1, 1));
    cfg.indent(&mut ta);
    assert_eq!(ta.value(), "  a\n  b");
    // Selection anchored at column 0 stays there; the cursor shifts.
    assert_eq!(ta.selection(), Some((0, 0)..(1, 3)));
    cfg.dedent(&mut ta);
    assert_eq!(ta.value(), "a\nb");

    let hard = IndentConfig {
        hard_tabs: true,
        ..cfg
    };
    ta.set_value("\t\tx");
    ta.cursor_col = 3;
    hard.dedent(&mut ta);
    assert_eq!(ta.value(), "\tx");
    assert_eq!(ta.cursor_col, 2);
}

#[test]
fn multi_line_indent_respects_max_length() {
    let cfg = IndentConfig {
        tab_width: 4,
        hard_tabs: false,
        tab_indents: true,
        auto_indent: true,
    };
    let mut ta = TextareaState::new().max_length(8);
    ta.set_value("a\nb");
    ta.set_selection((0, 0), (1, 1));
    cfg.indent(&mut ta);
    assert_eq!(ta.value(), "a\nb", "4 + 4 extra graphemes exceed the cap");
}

#[test]
fn display_cols_expand_tabs() {
    assert_eq!(display_cols("\tx", 1, 4), 4);
    assert_eq!(display_cols("ab\tx", 3, 4), 4);
    assert_eq!(display_cols("abcd\tx", 5, 4), 8);
    assert_eq!(display_cols("한x", 2, 4), 3);
}

#[cfg(feature = "syntax-rust")]
mod highlighted {
    use super::*;
    use proptest::prelude::*;

    const SOURCE: &str = "use std::fmt;\n\n/// Doc.\nfn main() {\n    let s = \"hi\";\n    \
        println!(\"{s}\");\n}\n\nstruct P {\n    x: i32,\n}\n\nimpl P {\n    fn get(&self) -> i32 \
        {\n        self.x // c\n    }\n}\n";

    #[derive(Debug, Clone)]
    enum Edit {
        Insert(usize, usize, char),
        Delete(usize, usize),
        Newline(usize, usize),
        JoinLine(usize),
    }

    fn edit_strategy() -> impl Strategy<Value = Edit> {
        let ch = prop::sample::select(vec!['a', ' ', '{', '}', '"', '/', '*', '\n', '(', ';']);
        prop_oneof![
            (any::<usize>(), any::<usize>(), ch).prop_map(|(r, c, ch)| Edit::Insert(r, c, ch)),
            (any::<usize>(), any::<usize>()).prop_map(|(r, c)| Edit::Delete(r, c)),
            (any::<usize>(), any::<usize>()).prop_map(|(r, c)| Edit::Newline(r, c)),
            any::<usize>().prop_map(Edit::JoinLine),
        ]
    }

    fn apply(doc: &mut Vec<String>, edit: &Edit) {
        let row_of = |r: usize, doc: &Vec<String>| r % doc.len();
        match *edit {
            Edit::Insert(r, c, ch) => {
                let r = row_of(r, doc);
                let at = c % (doc[r].len() + 1);
                if ch == '\n' {
                    let tail = doc[r].split_off(at);
                    doc.insert(r + 1, tail);
                } else {
                    doc[r].insert(at, ch);
                }
            }
            Edit::Delete(r, c) => {
                let r = row_of(r, doc);
                if !doc[r].is_empty() {
                    let at = c % doc[r].len();
                    doc[r].remove(at);
                }
            }
            Edit::Newline(r, c) => {
                let r = row_of(r, doc);
                let at = c % (doc[r].len() + 1);
                let tail = doc[r].split_off(at);
                doc.insert(r + 1, tail);
            }
            Edit::JoinLine(r) => {
                if doc.len() > 1 {
                    let r = r % (doc.len() - 1);
                    let next = doc.remove(r + 1);
                    doc[r].push_str(&next);
                }
            }
        }
    }

    fn spans_of(cache: &HighlightCache, rows: std::ops::Range<usize>) -> Vec<Vec<(String, Style)>> {
        rows.map(|row| cache.line_spans(row).map(<[_]>::to_vec).unwrap_or_default())
            .collect()
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        /// Random edits: the cached spans of the visible window always
        /// equal a from-scratch highlight of the same window.
        #[test]
        fn cached_spans_equal_a_fresh_highlight(
            edits in prop::collection::vec(edit_strategy(), 1..24),
            top in 0usize..12,
        ) {
            let theme = Theme::dark();
            let mut doc = lines(SOURCE);
            let mut cache = HighlightCache::default();
            for edit in &edits {
                apply(&mut doc, edit);
                let top = top.min(doc.len() - 1);
                let window = top..(top + 8).min(doc.len());
                cache.prepare(&doc, window.clone(), Some("rust"), &theme);
                let mut fresh = HighlightCache::default();
                fresh.prepare(&doc, window.clone(), Some("rust"), &theme);
                prop_assert_eq!(spans_of(&cache, window.clone()), spans_of(&fresh, window.clone()));
                // And each chunk equals highlighting its text directly.
                for range in chunk_ranges(&doc, window.clone()) {
                    let code = doc[range.clone()].join("\n");
                    let direct = crate::syntax::highlight_lines(&code, "rust", &theme)
                        .expect("rust is supported");
                    prop_assert_eq!(spans_of(&cache, range.clone()), direct);
                }
            }
        }
    }

    #[test]
    fn cache_reuses_unchanged_chunks_and_stays_bounded() {
        let theme = Theme::dark();
        let mut doc = lines(SOURCE);
        let mut cache = HighlightCache::default();
        cache.prepare(&doc, 0..doc.len(), Some("rust"), &theme);
        let chunks = chunk_ranges(&doc, 0..doc.len()).len() as u64;
        assert_eq!(cache.stats.runs, chunks);

        // Edit the `struct P` chunk: one miss, the rest hit.
        doc[9].push_str(" // y");
        let before = cache.stats;
        cache.prepare(&doc, 0..doc.len(), Some("rust"), &theme);
        assert_eq!(cache.stats.runs - before.runs, 1);
        assert_eq!(cache.stats.hits - before.hits, chunks - 1);

        // A theme change invalidates everything.
        let before = cache.stats;
        cache.prepare(&doc, 0..doc.len(), Some("rust"), &Theme::light());
        assert_eq!(cache.stats.runs - before.runs, chunks);

        // Many distinct edits never grow the cache past the cap + window.
        for i in 0..200 {
            doc[9] = format!("    x: i32, // {i}");
            cache.prepare(&doc, 8..11, Some("rust"), &theme);
        }
        assert!(cache.cached_chunks() <= MAX_CACHED_CHUNKS + 1);
        // Unsupported language: plain text.
        cache.prepare(&doc, 0..doc.len(), Some("nope"), &theme);
        assert!(cache.line_spans(0).is_none());
    }
}
