//! Delta history tests (#415): edit records round-trip, the delta engine
//! matches the original full-snapshot implementation step for step, and
//! retained memory scales with edit size.

use super::*;
use proptest::prelude::*;

fn cases() -> u32 {
    std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(256)
}

// ── reference: the pre-#415 snapshot implementations ──────────────────

/// Retained bytes the delta engine charges for a run of stored documents.
fn chain_bytes<D: HistoryDoc>(docs: impl Iterator<Item = D>) -> usize {
    let docs: Vec<D> = docs.collect();
    docs.windows(2)
        .map(|pair| D::delta_bytes(&D::diff(&pair[0], &pair[1])))
        .sum()
}

/// `TextareaState`'s former `Vec<TextareaSnapshot>` + `redo_tip` history,
/// plus the byte budget evaluated on full snapshots.
struct RefTextarea {
    history: Vec<(Vec<String>, TextareaMeta)>,
    index: usize,
    tip: Option<(Vec<String>, TextareaMeta)>,
    max: usize,
    max_bytes: usize,
}

impl RefTextarea {
    fn bytes(&self) -> usize {
        chain_bytes(self.history.iter().map(|(doc, _)| doc.clone()))
    }

    fn push(&mut self, lines: &[String], meta: TextareaMeta) {
        if self.max == 0 {
            return;
        }
        if self.index < self.history.len() {
            self.history.truncate(self.index);
        }
        self.tip = None;
        self.history.push((lines.to_vec(), meta));
        while self.history.len() > self.max {
            self.history.remove(0);
        }
        while self.history.len() > 1 && self.bytes() > self.max_bytes {
            self.history.remove(0);
        }
        self.index = self.history.len();
    }

    fn undo(&mut self, lines: &mut Vec<String>, live: TextareaMeta) -> Option<TextareaMeta> {
        if self.history.is_empty() || self.index == 0 {
            return None;
        }
        if self.index == self.history.len() {
            self.tip = Some((lines.clone(), live));
        }
        self.index -= 1;
        let (doc, meta) = &self.history[self.index];
        *lines = doc.clone();
        Some(*meta)
    }

    fn redo(&mut self, lines: &mut Vec<String>) -> Option<TextareaMeta> {
        if self.index < self.history.len().saturating_sub(1) {
            self.index += 1;
            let (doc, meta) = &self.history[self.index];
            *lines = doc.clone();
            return Some(*meta);
        }
        if self.index + 1 != self.history.len() {
            return None;
        }
        let (doc, meta) = self.tip.as_ref()?;
        self.index = self.history.len();
        *lines = doc.clone();
        Some(*meta)
    }
}

/// `TextInputState`'s former undo / redo snapshot stacks.
#[derive(Default)]
struct RefTextInput {
    undo: Vec<(String, TextInputMeta)>,
    redo: Vec<(String, TextInputMeta)>,
}

impl RefTextInput {
    fn push(&mut self, value: &str, meta: TextInputMeta, max: usize, max_bytes: usize) {
        self.redo.clear();
        self.undo.push((value.to_string(), meta));
        while self.undo.len() > max {
            self.undo.remove(0);
        }
        while self.undo.len() > 1
            && chain_bytes(self.undo.iter().map(|(doc, _)| doc.clone())) > max_bytes
        {
            self.undo.remove(0);
        }
    }

    fn undo(&mut self, value: &mut String, live: TextInputMeta) -> Option<TextInputMeta> {
        let (doc, meta) = self.undo.pop()?;
        self.redo.push((std::mem::replace(value, doc), live));
        Some(meta)
    }

    fn redo(&mut self, value: &mut String, live: TextInputMeta) -> Option<TextInputMeta> {
        let (doc, meta) = self.redo.pop()?;
        self.undo.push((std::mem::replace(value, doc), live));
        Some(meta)
    }
}

// ── generators ─────────────────────────────────────────────────────────

/// Short strings over an alphabet with multi-byte scalars, a ZWJ sequence
/// part, CR and an embedded newline (rows of the public `lines` field may
/// contain anything).
fn text() -> impl Strategy<Value = String> {
    proptest::collection::vec(
        prop::sample::select(vec![
            "a",
            "b",
            " ",
            "\u{d55c}",
            "\u{e9}",
            "\u{200d}",
            "\r",
            "\n",
            "\u{1f469}",
        ]),
        0..6,
    )
    .prop_map(|parts| parts.concat())
}

fn rows() -> impl Strategy<Value = Vec<String>> {
    proptest::collection::vec(text(), 0..5)
}

/// Floor `byte` to a char boundary of `s`.
fn floor_boundary(s: &str, byte: usize) -> usize {
    let mut byte = byte.min(s.len());
    while !s.is_char_boundary(byte) {
        byte -= 1;
    }
    byte
}

/// One mutation of the live textarea document, chosen and positioned by
/// raw numbers so shrinking stays meaningful.
#[derive(Debug, Clone)]
enum RowsEdit {
    InsertText(usize, usize, String),
    DeleteBytes(usize, usize, usize),
    InsertRows(usize, Vec<String>),
    RemoveRows(usize, usize),
    SplitRow(usize, usize),
    JoinRows(usize),
    Replace(Vec<String>),
}

fn rows_edit() -> impl Strategy<Value = RowsEdit> {
    prop_oneof![
        4 => (any::<usize>(), any::<usize>(), text()).prop_map(|(r, b, t)| RowsEdit::InsertText(r, b, t)),
        3 => (any::<usize>(), any::<usize>(), 0usize..8).prop_map(|(r, b, n)| RowsEdit::DeleteBytes(r, b, n)),
        2 => (any::<usize>(), rows()).prop_map(|(r, v)| RowsEdit::InsertRows(r, v)),
        2 => (any::<usize>(), 0usize..4).prop_map(|(r, n)| RowsEdit::RemoveRows(r, n)),
        2 => (any::<usize>(), any::<usize>()).prop_map(|(r, b)| RowsEdit::SplitRow(r, b)),
        2 => any::<usize>().prop_map(RowsEdit::JoinRows),
        1 => rows().prop_map(RowsEdit::Replace),
    ]
}

fn apply_rows_edit(doc: &mut Vec<String>, edit: &RowsEdit) {
    let row = |r: usize, doc: &Vec<String>| r % doc.len().max(1);
    match edit {
        RowsEdit::InsertText(r, b, t) => {
            if doc.is_empty() {
                doc.push(String::new());
            }
            let r = row(*r, doc);
            let at = floor_boundary(&doc[r], *b % (doc[r].len() + 1));
            doc[r].insert_str(at, t);
        }
        RowsEdit::DeleteBytes(r, b, n) => {
            if doc.is_empty() {
                return;
            }
            let r = row(*r, doc);
            let start = floor_boundary(&doc[r], *b % (doc[r].len() + 1));
            let end = floor_boundary(&doc[r], start + n);
            doc[r].replace_range(start..end, "");
        }
        RowsEdit::InsertRows(r, rows) => {
            let r = *r % (doc.len() + 1);
            doc.splice(r..r, rows.iter().cloned());
        }
        RowsEdit::RemoveRows(r, n) => {
            if doc.is_empty() {
                return;
            }
            let r = row(*r, doc);
            let end = (r + n).min(doc.len());
            doc.drain(r..end);
        }
        RowsEdit::SplitRow(r, b) => {
            if doc.is_empty() {
                return;
            }
            let r = row(*r, doc);
            let at = floor_boundary(&doc[r], *b % (doc[r].len() + 1));
            let tail = doc[r].split_off(at);
            doc.insert(r + 1, tail);
        }
        RowsEdit::JoinRows(r) => {
            if doc.len() < 2 {
                return;
            }
            let r = *r % (doc.len() - 1);
            let next = doc.remove(r + 1);
            doc[r].push_str(&next);
        }
        RowsEdit::Replace(rows) => *doc = rows.clone(),
    }
}

#[derive(Debug, Clone)]
enum Step<E> {
    /// Mutate the live document without recording (typing inside a burst,
    /// or app code writing the public field).
    Edit(E),
    /// Record an undo step, then mutate (what every editing primitive does).
    RecordedEdit(E),
    /// Move the cursor (metadata only).
    Cursor(usize),
    Undo,
    Redo,
}

fn steps<E: std::fmt::Debug + Clone, S: Strategy<Value = E>>(
    edit: impl Fn() -> S,
) -> impl Strategy<Value = Vec<Step<E>>> {
    proptest::collection::vec(
        prop_oneof![
            2 => edit().prop_map(Step::Edit),
            5 => edit().prop_map(Step::RecordedEdit),
            1 => any::<usize>().prop_map(Step::Cursor),
            3 => Just(Step::Undo),
            2 => Just(Step::Redo),
        ],
        0..40,
    )
}

fn meta(cursor: usize) -> TextareaMeta {
    TextareaMeta {
        cursor_row: cursor % 7,
        cursor_col: cursor % 11,
        navigation: None,
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(cases()))]

    #[test]
    fn rows_delta_round_trips(from in rows(), to in rows()) {
        let delta = <Vec<String>>::diff(&from, &to);
        let mut doc = from.clone();
        doc.apply(&delta, true);
        prop_assert_eq!(&doc, &to);
        doc.apply(&delta, false);
        prop_assert_eq!(&doc, &from);
    }

    #[test]
    fn text_delta_round_trips(from in text(), to in text()) {
        let delta = String::diff(&from, &to);
        let mut doc = from.clone();
        doc.apply(&delta, true);
        prop_assert_eq!(&doc, &to);
        doc.apply(&delta, false);
        prop_assert_eq!(&doc, &from);
    }

    /// The delta engine reproduces the snapshot implementation: same live
    /// document, same restored cursor and same history length after every
    /// step, including byte-budget eviction.
    #[test]
    fn textarea_history_matches_snapshot_reference(
        initial in rows(),
        max in 0usize..6,
        max_bytes in prop_oneof![Just(usize::MAX), 0usize..600],
        script in steps(rows_edit),
    ) {
        let mut history = TextareaHistory::new(max);
        history.max_bytes = max_bytes;
        let mut reference = RefTextarea { history: Vec::new(), index: 0, tip: None, max, max_bytes };
        let mut lines = initial.clone();
        let mut ref_lines = initial;
        let mut cursor = 0usize;
        for step in &script {
            match step {
                Step::Edit(edit) => {
                    apply_rows_edit(&mut lines, edit);
                    apply_rows_edit(&mut ref_lines, edit);
                }
                Step::RecordedEdit(edit) => {
                    history.push(&lines, meta(cursor));
                    reference.push(&ref_lines, meta(cursor));
                    apply_rows_edit(&mut lines, edit);
                    apply_rows_edit(&mut ref_lines, edit);
                }
                Step::Cursor(c) => cursor = *c,
                Step::Undo => {
                    let got = history.undo(&mut lines, meta(cursor));
                    let want = reference.undo(&mut ref_lines, meta(cursor));
                    prop_assert_eq!(got, want);
                    if let Some(m) = got {
                        cursor = m.cursor_row * 1000 + m.cursor_col;
                    }
                }
                Step::Redo => {
                    let got = history.redo(&mut lines);
                    let want = reference.redo(&mut ref_lines);
                    prop_assert_eq!(got, want);
                    if let Some(m) = got {
                        cursor = m.cursor_row * 1000 + m.cursor_col;
                    }
                }
            }
            prop_assert_eq!(&lines, &ref_lines, "step {:?}", step);
            prop_assert_eq!(history.len(), reference.history.len());
            prop_assert_eq!(history.bytes() >= reference.bytes(), true);
        }
    }

    /// Same for the text input's undo / redo stacks.
    #[test]
    fn text_input_history_matches_snapshot_reference(
        initial in text(),
        max in 1usize..6,
        max_bytes in prop_oneof![Just(usize::MAX), 0usize..300],
        script in steps(|| (any::<usize>(), 0usize..4, text())),
    ) {
        let edit = |value: &mut String, (at, len, insert): &(usize, usize, String)| {
            let start = floor_boundary(value, *at % (value.len() + 1));
            let end = floor_boundary(value, start + len);
            value.replace_range(start..end, insert);
        };
        let meta = |cursor: usize| TextInputMeta { cursor: cursor % 13, anchor: cursor.is_multiple_of(3).then_some(cursor % 5) };
        let mut history = TextInputHistory::default();
        let mut reference = RefTextInput::default();
        let mut value = initial.clone();
        let mut ref_value = initial;
        let mut cursor = 0usize;
        for step in &script {
            match step {
                Step::Edit(e) => {
                    edit(&mut value, e);
                    edit(&mut ref_value, e);
                }
                Step::RecordedEdit(e) => {
                    history.push(&value, meta(cursor), max, max_bytes);
                    reference.push(&ref_value, meta(cursor), max, max_bytes);
                    edit(&mut value, e);
                    edit(&mut ref_value, e);
                }
                Step::Cursor(c) => cursor = *c,
                Step::Undo => {
                    let got = history.undo(&mut value, meta(cursor));
                    prop_assert_eq!(got, reference.undo(&mut ref_value, meta(cursor)));
                }
                Step::Redo => {
                    let got = history.redo(&mut value, meta(cursor));
                    prop_assert_eq!(got, reference.redo(&mut ref_value, meta(cursor)));
                }
            }
            prop_assert_eq!(&value, &ref_value, "step {:?}", step);
            prop_assert_eq!(history.undo_len(), reference.undo.len());
        }
    }
}

// ── memory bounds ──────────────────────────────────────────────────────

fn big_document() -> Vec<String> {
    (0..25_000)
        .map(|i| format!("{i:08} {}", "a".repeat(31)))
        .collect()
}

/// 100 one-byte edits of a ~1 MB document retain edit records of a few
/// kilobytes, not 100 documents.
#[test]
fn textarea_history_memory_scales_with_edit_size() {
    let mut lines = big_document();
    let mut history = TextareaHistory::new(100);
    for i in 0..150 {
        history.push(&lines, meta(i));
        let row = (i * 97) % lines.len();
        lines[row].insert(3, 'x');
    }
    assert_eq!(history.len(), 100);
    assert!(
        history.bytes() < 100 * 256,
        "edit records retain {} bytes",
        history.bytes()
    );
    // Every step still undoes exactly.
    let mut expected = lines.clone();
    for i in (50..150).rev() {
        history.undo(&mut lines, meta(0)).expect("undo step");
        let row = (i * 97) % expected.len();
        expected[row].remove(3);
        assert_eq!(lines, expected);
    }
    assert!(history.undo(&mut lines, meta(0)).is_none());
}

/// Typing into one huge line records the typed bytes, not the line.
#[test]
fn single_line_edits_record_bytes_not_lines() {
    let mut lines = vec!["\u{d55c}".repeat(300_000)];
    let mut history = TextareaHistory::new(100);
    for i in 0..10 {
        history.push(&lines, meta(i));
        let at = floor_boundary(&lines[0], lines[0].len() / 2);
        lines[0].insert_str(at, "xy");
    }
    assert!(history.bytes() < 10 * 256, "{}", history.bytes());

    let mut value = "a".repeat(1_000_000);
    let mut input = TextInputHistory::default();
    for i in 0..100 {
        input.push(
            &value,
            TextInputMeta {
                cursor: i,
                anchor: None,
            },
            100,
            usize::MAX,
        );
        value.pop();
    }
    assert!(input.bytes() < 100 * 128, "{}", input.bytes());
}

/// The byte budget evicts the oldest steps but always keeps the newest one.
#[test]
fn byte_budget_evicts_oldest_and_keeps_newest() {
    let mut lines = vec![String::new()];
    let mut history = TextareaHistory::new(100);
    history.max_bytes = 50_000;
    for i in 0..10 {
        history.push(&lines, meta(i));
        lines.push("p".repeat(20_000));
    }
    // Each step after the first is a ~20 KB record: two fit the budget.
    assert_eq!(history.len(), 3);
    assert!(history.bytes() <= 50_000);
    for _ in 0..3 {
        assert!(history.undo(&mut lines, meta(0)).is_some());
    }
    assert_eq!(lines.len(), 8, "only the three newest steps are undoable");
    assert!(history.undo(&mut lines, meta(0)).is_none());

    // A single step larger than the budget is still recorded.
    let mut history = TextareaHistory::new(100);
    history.max_bytes = 10;
    let mut lines = vec![String::new()];
    history.push(&lines, meta(0));
    lines[0] = "q".repeat(1_000);
    history.push(&lines, meta(1));
    lines[0].clear();
    assert_eq!(history.len(), 1);
    assert!(history.undo(&mut lines, meta(2)).is_some());
    assert_eq!(lines, vec!["q".repeat(1_000)]);
}

/// Changing the public document outside the history after an undo still
/// restores the recorded states exactly on redo, like the snapshots did.
#[test]
fn external_mutation_after_undo_restores_recorded_states() {
    let mut lines = vec!["one".to_string()];
    let mut history = TextareaHistory::new(10);
    history.push(&lines, meta(0));
    lines[0].push_str(" two");
    history.push(&lines, meta(1));
    lines[0].push_str(" three");
    history.undo(&mut lines, meta(2));
    assert_eq!(lines, vec!["one two"]);
    lines = vec!["app".into(), "rewrote".into(), "it".into()];
    history.undo(&mut lines, meta(0));
    assert_eq!(lines, vec!["one"]);
    lines.push("again".into());
    history.redo(&mut lines);
    assert_eq!(lines, vec!["one two"]);
    history.redo(&mut lines);
    assert_eq!(lines, vec!["one two three"]);
}
