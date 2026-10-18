//! Delta undo history for [`TextareaState`] and [`TextInputState`] (#415).
//!
//! Earlier versions stored every undo step as a full copy of the document,
//! so a 1 MB textarea at the default cap of 100 steps retained about
//! 156 MB. History now keeps **one** materialized copy of the document (the
//! *base*, taken at the first recorded edit) plus one edit record per step.
//! An edit record holds only the changed region (rows for a textarea, bytes
//! for a text input) before and after the edit, so retained memory scales
//! with edit size, not document size. Records are additionally bounded by a
//! byte budget.
//!
//! Records are computed by diffing the base against the live document when a
//! step is recorded, never from what an editing primitive *claims* to have
//! changed. `TextareaState::lines` and `TextInputState::value` are public, so
//! direct mutation between recordings is captured exactly like before.
//!
//! The engine ([`DeltaHistory`]) stores a linear sequence of states
//! `S_0 .. S_{n-1}`: per-state metadata (cursor, selection) plus edit
//! `deltas[i]` turning the content of `S_i` into that of `S_{i+1}`. The base
//! is the content of one state and walks along the sequence by applying
//! deltas forward or backward. [`TextareaHistory`] and [`TextInputHistory`]
//! map the two widgets' original snapshot-stack semantics onto it unchanged.

use super::*;
use std::collections::VecDeque;

/// Default byte budget for retained undo edit records
/// ([`TextareaState::history_max_bytes`]). Excludes the one materialized
/// copy of the document that history keeps once an edit is recorded.
pub(crate) const DEFAULT_HISTORY_MAX_BYTES: usize = 8 * 1024 * 1024;

/// Approximate retained heap bytes of one `String` beyond its text: the
/// `String` header in its containing `Vec`.
const STRING_OVERHEAD: usize = std::mem::size_of::<String>();

/// A document type the delta history can store.
pub(crate) trait HistoryDoc: Clone + PartialEq {
    /// One edit between two documents.
    type Delta: Clone + std::fmt::Debug;

    /// The edit turning `from` into `to`.
    fn diff(from: &Self, to: &Self) -> Self::Delta;

    /// Apply `delta` forward (its `from` side to its `to` side) or backward.
    /// `self` must equal the side being replaced.
    fn apply(&mut self, delta: &Self::Delta, forward: bool);

    /// Approximate retained heap bytes of `delta`.
    fn delta_bytes(delta: &Self::Delta) -> usize;
}

/// Length in bytes of the longest common prefix of `a` and `b`, at a char
/// boundary.
fn common_prefix(a: &str, b: &str) -> usize {
    let mut len = a
        .as_bytes()
        .iter()
        .zip(b.as_bytes())
        .take_while(|(x, y)| x == y)
        .count();
    // Equal bytes up to `len`: a boundary in `a` is a boundary in `b`.
    while !a.is_char_boundary(len) {
        len -= 1;
    }
    len
}

/// Length in bytes of the longest common suffix of `a` and `b`, at most
/// `limit`, at a char boundary.
fn common_suffix(a: &str, b: &str, limit: usize) -> usize {
    let mut len = a
        .as_bytes()
        .iter()
        .rev()
        .zip(b.as_bytes().iter().rev())
        .take(limit)
        .take_while(|(x, y)| x == y)
        .count();
    while !a.is_char_boundary(a.len() - len) {
        len -= 1;
    }
    len
}

/// Edit between two textarea documents: rows `row .. row + old.len()` of the
/// `from` side become rows `row .. row + new.len()` of the `to` side.
///
/// When both sides are non-empty, the first changed row of both sides starts
/// with the same `prefix` bytes and the last changed row of both sides ends
/// with the same `suffix` bytes; those shared bytes stay in the document and
/// are trimmed from `old` / `new`. Typing into a 1 MB single-line document
/// therefore records a few bytes, not the line.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct RowsDelta {
    row: usize,
    prefix: usize,
    suffix: usize,
    old: Vec<String>,
    new: Vec<String>,
}

/// Trim `prefix` bytes from the first row and `suffix` bytes from the last
/// row of `rows` (non-empty).
fn trimmed(rows: &[String], prefix: usize, suffix: usize) -> Vec<String> {
    let last = rows.len() - 1;
    rows.iter()
        .enumerate()
        .map(|(i, row)| {
            let start = if i == 0 { prefix } else { 0 };
            let end = if i == last {
                row.len() - suffix
            } else {
                row.len()
            };
            row[start..end].to_string()
        })
        .collect()
}

/// Replace the trimmed rows `remove` at `row` with `insert` (see
/// [`RowsDelta`]).
fn splice_rows(
    doc: &mut Vec<String>,
    row: usize,
    prefix: usize,
    suffix: usize,
    remove: &[String],
    insert: &[String],
) {
    if remove.is_empty() || insert.is_empty() {
        debug_assert!(prefix == 0 && suffix == 0);
        doc.splice(row..row + remove.len(), insert.iter().cloned());
        return;
    }
    let last = row + remove.len() - 1;
    if remove.len() == 1 && insert.len() == 1 {
        let line = &mut doc[row];
        let end = line.len() - suffix;
        line.replace_range(prefix..end, &insert[0]);
        return;
    }
    let tail = {
        let line = &mut doc[last];
        line.split_off(line.len() - suffix)
    };
    let first = &mut doc[row];
    first.truncate(prefix);
    first.push_str(&insert[0]);
    doc.splice(row + 1..=last, insert[1..].iter().cloned());
    doc[row + insert.len() - 1].push_str(&tail);
}

impl HistoryDoc for Vec<String> {
    type Delta = RowsDelta;

    fn diff(from: &Self, to: &Self) -> RowsDelta {
        let common = from.len().min(to.len());
        let top = from.iter().zip(to).take_while(|(a, b)| a == b).count();
        if top == from.len() && top == to.len() {
            return RowsDelta::default();
        }
        let bottom = from[top..]
            .iter()
            .rev()
            .zip(to[top..].iter().rev())
            .take(common - top)
            .take_while(|(a, b)| a == b)
            .count();
        let old = &from[top..from.len() - bottom];
        let new = &to[top..to.len() - bottom];
        if old.is_empty() || new.is_empty() {
            return RowsDelta {
                row: top,
                prefix: 0,
                suffix: 0,
                old: old.to_vec(),
                new: new.to_vec(),
            };
        }
        let prefix = common_prefix(&old[0], &new[0]);
        // Room left for the suffix in each side's last changed row; a
        // single changed row shares it with the prefix.
        let room = |rows: &[String]| {
            let last = rows[rows.len() - 1].len();
            if rows.len() == 1 { last - prefix } else { last }
        };
        let suffix = common_suffix(
            &old[old.len() - 1],
            &new[new.len() - 1],
            room(old).min(room(new)),
        );
        RowsDelta {
            row: top,
            prefix,
            suffix,
            old: trimmed(old, prefix, suffix),
            new: trimmed(new, prefix, suffix),
        }
    }

    fn apply(&mut self, delta: &RowsDelta, forward: bool) {
        let (remove, insert) = if forward {
            (&delta.old, &delta.new)
        } else {
            (&delta.new, &delta.old)
        };
        splice_rows(self, delta.row, delta.prefix, delta.suffix, remove, insert);
    }

    fn delta_bytes(delta: &RowsDelta) -> usize {
        std::mem::size_of::<RowsDelta>()
            + delta
                .old
                .iter()
                .chain(&delta.new)
                .map(|row| row.len() + STRING_OVERHEAD)
                .sum::<usize>()
    }
}

/// Edit between two single-line values: `old` at byte `at` becomes `new`.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct TextDelta {
    at: usize,
    old: String,
    new: String,
}

impl HistoryDoc for String {
    type Delta = TextDelta;

    fn diff(from: &Self, to: &Self) -> TextDelta {
        let at = common_prefix(from, to);
        let room = from.len().min(to.len()) - at;
        let suffix = common_suffix(from, to, room);
        TextDelta {
            at,
            old: from[at..from.len() - suffix].to_string(),
            new: to[at..to.len() - suffix].to_string(),
        }
    }

    fn apply(&mut self, delta: &TextDelta, forward: bool) {
        let (remove, insert) = if forward {
            (&delta.old, &delta.new)
        } else {
            (&delta.new, &delta.old)
        };
        self.replace_range(delta.at..delta.at + remove.len(), insert);
    }

    fn delta_bytes(delta: &TextDelta) -> usize {
        std::mem::size_of::<TextDelta>() + delta.old.len() + delta.new.len()
    }
}

/// Linear sequence of document states stored as one materialized base plus
/// edit records (see the module docs).
#[derive(Clone)]
pub(crate) struct DeltaHistory<D: HistoryDoc, M> {
    /// Per-state metadata (cursor, selection).
    metas: VecDeque<M>,
    /// `deltas[i]` turns the content of state `i` into that of `i + 1`.
    deltas: VecDeque<D::Delta>,
    /// Sum of [`HistoryDoc::delta_bytes`] over `deltas`.
    delta_bytes: usize,
    /// Materialized content of state `base_index`; `None` when empty.
    base: Option<D>,
    base_index: usize,
}

impl<D: HistoryDoc, M> Default for DeltaHistory<D, M> {
    fn default() -> Self {
        Self {
            metas: VecDeque::new(),
            deltas: VecDeque::new(),
            delta_bytes: 0,
            base: None,
            base_index: 0,
        }
    }
}

impl<D: HistoryDoc, M> std::fmt::Debug for DeltaHistory<D, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeltaHistory")
            .field("states", &self.metas.len())
            .field("delta_bytes", &self.delta_bytes)
            .finish_non_exhaustive()
    }
}

impl<D: HistoryDoc, M: Clone> DeltaHistory<D, M> {
    /// Number of stored states.
    pub(crate) fn len(&self) -> usize {
        self.metas.len()
    }

    /// Retained bytes of the edit records (excludes the base copy).
    pub(crate) fn delta_bytes(&self) -> usize {
        self.delta_bytes
    }

    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }

    /// Walk the base to state `target`, applying the same steps to `live`
    /// when `live` is `Some` (it must equal the base).
    fn walk(&mut self, target: usize, mut live: Option<&mut D>) {
        let Some(base) = self.base.as_mut() else {
            return;
        };
        while self.base_index < target {
            let delta = &self.deltas[self.base_index];
            base.apply(delta, true);
            if let Some(live) = live.as_deref_mut() {
                live.apply(delta, true);
            }
            self.base_index += 1;
        }
        while self.base_index > target {
            self.base_index -= 1;
            let delta = &self.deltas[self.base_index];
            base.apply(delta, false);
            if let Some(live) = live.as_deref_mut() {
                live.apply(delta, false);
            }
        }
    }

    /// Drop every state from index `len` on.
    pub(crate) fn truncate(&mut self, len: usize) {
        if len >= self.metas.len() {
            return;
        }
        if len == 0 {
            self.clear();
            return;
        }
        if self.base_index >= len {
            self.walk(len - 1, None);
        }
        self.metas.truncate(len);
        while self.deltas.len() > len - 1 {
            if let Some(delta) = self.deltas.pop_back() {
                self.delta_bytes -= D::delta_bytes(&delta);
            }
        }
    }

    /// Append `live` as the newest state. Afterwards the base equals `live`.
    pub(crate) fn push_back(&mut self, live: &D, meta: M) {
        if self.metas.is_empty() {
            self.base = Some(live.clone());
            self.base_index = 0;
            self.metas.push_back(meta);
            return;
        }
        let last = self.metas.len() - 1;
        self.walk(last, None);
        let Some(base) = self.base.as_mut() else {
            return;
        };
        let delta = D::diff(base, live);
        base.apply(&delta, true);
        debug_assert!(base == live);
        self.delta_bytes += D::delta_bytes(&delta);
        self.deltas.push_back(delta);
        self.metas.push_back(meta);
        self.base_index = last + 1;
    }

    /// Drop the oldest state.
    pub(crate) fn pop_front(&mut self) {
        if self.metas.len() <= 1 {
            self.clear();
            return;
        }
        if self.base_index == 0 {
            self.walk(1, None);
        }
        self.metas.pop_front();
        if let Some(delta) = self.deltas.pop_front() {
            self.delta_bytes -= D::delta_bytes(&delta);
        }
        self.base_index -= 1;
    }

    /// Replace `live` with the content of state `target` and return its
    /// metadata. `live_is_base` promises that `live` equals the base, which
    /// skips the comparison.
    pub(crate) fn load(&mut self, target: usize, live: &mut D, live_is_base: bool) -> M {
        let in_sync = live_is_base || self.base.as_ref() == Some(&*live);
        if in_sync {
            self.walk(target, Some(live));
        } else {
            // `live` was changed outside the history (or never matched):
            // restore the stored content exactly, as a snapshot would.
            self.walk(target, None);
            if let Some(base) = &self.base {
                live.clone_from(base);
            }
        }
        self.metas[target].clone()
    }

    /// Exchange `live` (with `meta`) and stored state `index`: `live` gets
    /// the stored content, the state gets `live`'s. Returns the state's old
    /// metadata.
    pub(crate) fn swap_with_live(&mut self, index: usize, live: &mut D, meta: M) -> M {
        self.walk(index, None);
        let Some(base) = self.base.as_mut() else {
            return meta;
        };
        // `live` now holds the stored content exactly, so the neighbouring
        // deltas can be replayed on it to rebuild both links.
        std::mem::swap(base, live);
        if index > 0 {
            let old = &self.deltas[index - 1];
            live.apply(old, false);
            let left = D::diff(live, base);
            live.apply(old, true);
            self.delta_bytes = self.delta_bytes - D::delta_bytes(old) + D::delta_bytes(&left);
            self.deltas[index - 1] = left;
        }
        if index + 1 < self.metas.len() {
            let old = &self.deltas[index];
            live.apply(old, true);
            let right = D::diff(base, live);
            live.apply(old, false);
            self.delta_bytes = self.delta_bytes - D::delta_bytes(old) + D::delta_bytes(&right);
            self.deltas[index] = right;
        }
        std::mem::replace(&mut self.metas[index], meta)
    }
}

/// Cursor state stored with each textarea undo step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TextareaMeta {
    pub(crate) cursor_row: usize,
    pub(crate) cursor_col: usize,
    pub(crate) navigation: Option<TextareaNavigation>,
}

/// Undo history of a [`TextareaState`].
///
/// Semantics are those of the original snapshot stack: past states
/// `S_0 .. S_{n-1}`, an index `p` (`n` means "at the live tip") and a redo
/// tip captured by the first undo from the tip. The tip is stored as an
/// extra state at index `n` so it costs an edit record, not a document.
#[derive(Debug, Clone)]
pub(crate) struct TextareaHistory {
    states: DeltaHistory<Vec<String>, TextareaMeta>,
    /// Next undo target is `index - 1`; `index == past_len()` is the tip.
    index: usize,
    /// The redo tip is stored as the last state.
    has_tip: bool,
    pub(crate) max: usize,
    pub(crate) max_bytes: usize,
}

impl TextareaHistory {
    pub(crate) fn new(max: usize) -> Self {
        Self {
            states: DeltaHistory::default(),
            index: 0,
            has_tip: false,
            max,
            max_bytes: DEFAULT_HISTORY_MAX_BYTES,
        }
    }

    /// Number of past (undoable) states, excluding the redo tip.
    pub(crate) fn len(&self) -> usize {
        self.states.len() - usize::from(self.has_tip)
    }

    /// Retained bytes of the edit records.
    pub(crate) fn bytes(&self) -> usize {
        self.states.delta_bytes()
    }

    pub(crate) fn clear(&mut self) {
        self.states.clear();
        self.index = 0;
        self.has_tip = false;
    }

    /// Record `lines` / `meta` as an undo step, dropping the redo branch and
    /// evicting the oldest steps beyond the count cap or byte budget. The
    /// newest step is always kept.
    pub(crate) fn push(&mut self, lines: &Vec<String>, meta: TextareaMeta) {
        if self.max == 0 {
            return;
        }
        self.states.truncate(self.index);
        self.has_tip = false;
        self.states.push_back(lines, meta);
        while self.states.len() > self.max {
            self.states.pop_front();
        }
        while self.states.delta_bytes() > self.max_bytes && self.states.len() > 1 {
            self.states.pop_front();
        }
        self.index = self.states.len();
    }

    /// Step back one state, restoring it into `lines`. Returns its metadata.
    pub(crate) fn undo(
        &mut self,
        lines: &mut Vec<String>,
        live: TextareaMeta,
    ) -> Option<TextareaMeta> {
        let past = self.len();
        if past == 0 || self.index == 0 {
            return None;
        }
        let mut live_is_base = false;
        if self.index == past {
            // Keep the live tip outside the bounded past states so the first
            // undo stays redoable even when `max == 1`.
            self.states.truncate(past);
            self.states.push_back(lines, live);
            self.has_tip = true;
            live_is_base = true;
        }
        self.index -= 1;
        Some(self.states.load(self.index, lines, live_is_base))
    }

    /// Step forward one state, restoring it into `lines`.
    pub(crate) fn redo(&mut self, lines: &mut Vec<String>) -> Option<TextareaMeta> {
        let past = self.len();
        if self.index + 1 < past {
            self.index += 1;
            return Some(self.states.load(self.index, lines, false));
        }
        if self.index + 1 != past || !self.has_tip {
            return None;
        }
        self.index = past;
        Some(self.states.load(past, lines, false))
    }
}

/// Cursor and selection stored with each text-input undo step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TextInputMeta {
    pub(crate) cursor: usize,
    pub(crate) anchor: Option<usize>,
}

/// Undo/redo history of a [`TextInputState`]: the original undo and redo
/// stacks laid out as one sequence `undo[0..] ++ reverse(redo)`, with the
/// live value sitting between them at `boundary`.
#[derive(Debug, Clone, Default)]
pub(crate) struct TextInputHistory {
    states: DeltaHistory<String, TextInputMeta>,
    /// Number of undo states; the rest are redo states.
    boundary: usize,
}

impl TextInputHistory {
    pub(crate) fn clear(&mut self) {
        self.states.clear();
        self.boundary = 0;
    }

    /// Record `value` / `meta` as an undo step, dropping the redo branch.
    pub(crate) fn push(
        &mut self,
        value: &String,
        meta: TextInputMeta,
        max: usize,
        max_bytes: usize,
    ) {
        self.states.truncate(self.boundary);
        self.states.push_back(value, meta);
        while self.states.len() > max {
            self.states.pop_front();
        }
        while self.states.delta_bytes() > max_bytes && self.states.len() > 1 {
            self.states.pop_front();
        }
        self.boundary = self.states.len();
    }

    /// Swap the live value with the newest undo state.
    pub(crate) fn undo(
        &mut self,
        value: &mut String,
        live: TextInputMeta,
    ) -> Option<TextInputMeta> {
        if self.boundary == 0 {
            return None;
        }
        self.boundary -= 1;
        Some(self.states.swap_with_live(self.boundary, value, live))
    }

    /// Swap the live value with the nearest redo state.
    pub(crate) fn redo(
        &mut self,
        value: &mut String,
        live: TextInputMeta,
    ) -> Option<TextInputMeta> {
        if self.boundary == self.states.len() {
            return None;
        }
        let meta = self.states.swap_with_live(self.boundary, value, live);
        self.boundary += 1;
        Some(meta)
    }

    /// Number of undo steps.
    #[cfg(test)]
    pub(crate) fn undo_len(&self) -> usize {
        self.boundary
    }

    /// Retained bytes of the edit records.
    #[cfg(test)]
    pub(crate) fn bytes(&self) -> usize {
        self.states.delta_bytes()
    }
}

#[cfg(test)]
mod tests;
