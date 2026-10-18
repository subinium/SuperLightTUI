//! Unified-diff parser and writer for [`super::DiffState`].
//!
//! Hand-written, dependency-free and panic-free: every slice uses
//! `strip_prefix` / `split_once` / `get`, and every count is checked.

use super::{
    DiffFile, DiffFileStatus, DiffHunk, DiffLine, DiffLineKind, DiffParseError, DiffParseErrorKind,
};

/// Parser bookkeeping for the file currently being filled.
struct FileBuilder {
    file: DiffFile,
    /// Started by a `diff --git` header (extended headers may follow).
    git: bool,
    /// A `---` / `+++` pair was already applied to this file.
    saw_paths: bool,
}

impl FileBuilder {
    fn new(git: bool) -> Self {
        Self {
            file: DiffFile::empty(),
            git,
            saw_paths: false,
        }
    }
}

fn err(line_idx: usize, kind: DiffParseErrorKind) -> DiffParseError {
    DiffParseError::new(line_idx + 1, kind)
}

/// Parse unified-diff text into files. See [`super::DiffState::from_unified`].
pub(super) fn parse_unified(input: &str) -> Result<Vec<DiffFile>, DiffParseError> {
    let mut lines: Vec<&str> = input
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .collect();
    if input.ends_with('\n') {
        lines.pop();
    }

    let mut files = Vec::new();
    let mut current: Option<FileBuilder> = None;
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];

        if let Some(rest) = line.strip_prefix("diff --git ") {
            finish(&mut files, current.take());
            let mut builder = FileBuilder::new(true);
            if let Some((old, new)) = split_git_paths(rest) {
                builder.file.old_path = Some(strip_prefix_dir(old, "a/"));
                builder.file.new_path = Some(strip_prefix_dir(new, "b/"));
            }
            current = Some(builder);
            i += 1;
            continue;
        }

        if let (Some(old), Some(new)) = (
            line.strip_prefix("--- "),
            lines.get(i + 1).and_then(|next| next.strip_prefix("+++ ")),
        ) {
            let reuse = current
                .as_ref()
                .is_some_and(|c| c.git && !c.saw_paths && c.file.hunks.is_empty());
            if !reuse {
                finish(&mut files, current.take());
                current = Some(FileBuilder::new(false));
            }
            if let Some(builder) = current.as_mut() {
                apply_header_paths(builder, old, new);
            }
            i += 2;
            continue;
        }

        if line.starts_with("@@") {
            let builder = current.get_or_insert_with(|| FileBuilder::new(false));
            let (hunk, next) = parse_hunk(&lines, i)?;
            builder.file.hunks.push(hunk);
            i = next;
            continue;
        }

        let in_git_header = current
            .as_ref()
            .is_some_and(|c| c.git && !c.saw_paths && c.file.hunks.is_empty());

        if is_binary_notice(line) {
            if in_git_header {
                if let Some(builder) = current.as_mut() {
                    builder.file.binary = true;
                }
            } else {
                // `diff -r` style notice: a file section of its own.
                finish(&mut files, current.take());
                let mut builder = FileBuilder::new(false);
                builder.file.binary = true;
                if let Some((old, new)) = line
                    .strip_prefix("Binary files ")
                    .and_then(|rest| rest.strip_suffix(" differ"))
                    .and_then(|rest| rest.split_once(" and "))
                {
                    set_old_path(&mut builder.file, old, "a/");
                    set_new_path(&mut builder.file, new, "b/");
                }
                current = Some(builder);
            }
            i += 1;
            continue;
        }

        if in_git_header && let Some(builder) = current.as_mut() {
            apply_extended_header(&mut builder.file, line);
            if line == "GIT binary patch" {
                // Skip the base85 literal / delta payload up to the next file.
                i += 1;
                while i < lines.len() && !lines[i].starts_with("diff --git ") {
                    i += 1;
                }
                continue;
            }
        }
        // Anything else (commit message, `Index:`, trailers) is ignored.
        i += 1;
    }
    finish(&mut files, current.take());

    if files.is_empty() && !input.trim().is_empty() {
        return Err(err(0, DiffParseErrorKind::NotADiff));
    }
    Ok(files)
}

fn finish(files: &mut Vec<DiffFile>, builder: Option<FileBuilder>) {
    if let Some(builder) = builder {
        files.push(builder.file);
    }
}

fn is_binary_notice(line: &str) -> bool {
    line.starts_with("Binary files ") && line.ends_with(" differ")
}

/// Apply a git extended-header line (`new file mode`, `rename from`, ...).
fn apply_extended_header(file: &mut DiffFile, line: &str) {
    if line.starts_with("new file mode ") {
        file.status = DiffFileStatus::Added;
        file.old_path = None;
    } else if line.starts_with("deleted file mode ") {
        file.status = DiffFileStatus::Deleted;
        file.new_path = None;
    } else if let Some(path) = line.strip_prefix("rename from ") {
        file.status = DiffFileStatus::Renamed;
        file.old_path = Some(unquote(path));
    } else if let Some(path) = line.strip_prefix("rename to ") {
        file.status = DiffFileStatus::Renamed;
        file.new_path = Some(unquote(path));
    } else if let Some(path) = line.strip_prefix("copy from ") {
        file.status = DiffFileStatus::Copied;
        file.old_path = Some(unquote(path));
    } else if let Some(path) = line.strip_prefix("copy to ") {
        file.status = DiffFileStatus::Copied;
        file.new_path = Some(unquote(path));
    } else if line == "GIT binary patch" {
        file.binary = true;
    }
}

fn apply_header_paths(builder: &mut FileBuilder, old: &str, new: &str) {
    builder.saw_paths = true;
    set_old_path(&mut builder.file, header_path(old), "a/");
    set_new_path(&mut builder.file, header_path(new), "b/");
}

fn set_old_path(file: &mut DiffFile, raw: &str, prefix: &str) {
    if raw == "/dev/null" {
        file.old_path = None;
        if file.status == DiffFileStatus::Modified {
            file.status = DiffFileStatus::Added;
        }
    } else {
        file.old_path = Some(strip_prefix_dir(raw, prefix));
    }
}

fn set_new_path(file: &mut DiffFile, raw: &str, prefix: &str) {
    if raw == "/dev/null" {
        file.new_path = None;
        if file.status == DiffFileStatus::Modified {
            file.status = DiffFileStatus::Deleted;
        }
    } else {
        file.new_path = Some(strip_prefix_dir(raw, prefix));
    }
}

/// Path part of a `---` / `+++` line: drops a tab-separated timestamp.
fn header_path(raw: &str) -> &str {
    if raw.starts_with('"') {
        return raw;
    }
    raw.split_once('\t').map_or(raw, |(path, _)| path)
}

fn strip_prefix_dir(raw: &str, prefix: &str) -> String {
    let path = unquote(raw);
    match path.strip_prefix(prefix) {
        Some(rest) if !rest.is_empty() => rest.to_string(),
        _ => path,
    }
}

/// Split `a/old b/new` from a `diff --git` header.
fn split_git_paths(rest: &str) -> Option<(&str, &str)> {
    if rest.starts_with('"') {
        let end = quoted_end(rest)?;
        let old = rest.get(..end)?;
        let new = rest.get(end..)?.strip_prefix(' ')?;
        return Some((old, new));
    }
    // Same path on both sides: `a/P b/P` — split exactly in the middle so
    // paths containing spaces survive.
    let bytes = rest.as_bytes();
    if bytes.len() % 2 == 1 {
        let mid = bytes.len() / 2;
        if bytes[mid] == b' ' && rest.is_char_boundary(mid) {
            let (old, new) = (&rest[..mid], &rest[mid + 1..]);
            if old
                .strip_prefix("a/")
                .is_some_and(|o| Some(o) == new.strip_prefix("b/"))
            {
                return Some((old, new));
            }
        }
    }
    if let Some(at) = rest.find(" b/") {
        return Some((rest.get(..at)?, rest.get(at + 1..)?));
    }
    if let Some(at) = rest.find(" \"") {
        return Some((rest.get(..at)?, rest.get(at + 1..)?));
    }
    rest.rsplit_once(' ')
}

/// Byte index one past the closing quote of a leading C-style quoted string.
fn quoted_end(s: &str) -> Option<usize> {
    let mut escaped = false;
    for (idx, ch) in s.char_indices().skip(1) {
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some(idx + 1);
        }
    }
    None
}

/// Undo git's C-style path quoting (`"a/sp\303\244ce"`). Unquoted input is
/// returned unchanged; malformed escapes are kept literally.
fn unquote(raw: &str) -> String {
    let Some(inner) = raw
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
    else {
        return raw.to_string();
    };
    let mut out: Vec<u8> = Vec::with_capacity(inner.len());
    let bytes = inner.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b != b'\\' || i + 1 >= bytes.len() {
            out.push(b);
            i += 1;
            continue;
        }
        let next = bytes[i + 1];
        let simple = match next {
            b'n' => Some(b'\n'),
            b't' => Some(b'\t'),
            b'r' => Some(b'\r'),
            b'a' => Some(0x07),
            b'b' => Some(0x08),
            b'f' => Some(0x0c),
            b'v' => Some(0x0b),
            b'"' => Some(b'"'),
            b'\\' => Some(b'\\'),
            _ => None,
        };
        if let Some(byte) = simple {
            out.push(byte);
            i += 2;
            continue;
        }
        let octal = bytes
            .get(i + 1..i + 4)
            .filter(|digits| digits.iter().all(|d| (b'0'..=b'7').contains(d)) && digits[0] <= b'3');
        if let Some(digits) = octal {
            out.push((digits[0] - b'0') * 64 + (digits[1] - b'0') * 8 + (digits[2] - b'0'));
            i += 4;
        } else {
            out.push(b);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Parse `N` or `N,M` (digits only).
fn parse_range(raw: &str) -> Option<(u32, u32)> {
    fn number(digits: &str) -> Option<u32> {
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        digits.parse().ok()
    }
    match raw.split_once(',') {
        Some((start, count)) => Some((number(start)?, number(count)?)),
        None => Some((number(raw)?, 1)),
    }
}

/// Parse `@@ -a,b +c,d @@ section`.
fn parse_hunk_header(line: &str) -> Option<(u32, u32, u32, u32, String)> {
    let rest = line.strip_prefix("@@ -")?;
    let (old, rest) = rest.split_once(' ')?;
    let rest = rest.strip_prefix('+')?;
    let (new, rest) = rest.split_once(' ')?;
    let rest = rest.strip_prefix("@@")?;
    let section = rest.strip_prefix(' ').unwrap_or(rest);
    let (old_start, old_count) = parse_range(old)?;
    let (new_start, new_count) = parse_range(new)?;
    Some((
        old_start,
        old_count,
        new_start,
        new_count,
        section.to_string(),
    ))
}

/// Parse one hunk starting at the header on `lines[start]`. Returns the hunk
/// and the index of the first line after it.
fn parse_hunk(lines: &[&str], start: usize) -> Result<(DiffHunk, usize), DiffParseError> {
    let (old_start, old_count, new_start, new_count, section) = parse_hunk_header(lines[start])
        .ok_or_else(|| err(start, DiffParseErrorKind::InvalidHunkHeader))?;
    if old_start.checked_add(old_count).is_none() || new_start.checked_add(new_count).is_none() {
        return Err(err(start, DiffParseErrorKind::LineNumberOverflow));
    }

    let mut hunk = DiffHunk {
        old_start,
        old_count,
        new_start,
        new_count,
        section,
        lines: Vec::new(),
    };
    let (mut rem_old, mut rem_new) = (old_count, new_count);
    let (mut old_no, mut new_no) = (old_start, new_start);
    let mut i = start + 1;

    while rem_old > 0 || rem_new > 0 {
        let Some(&line) = lines.get(i) else {
            return Err(err(start, DiffParseErrorKind::TruncatedHunk));
        };
        let (kind, text) = match line.as_bytes().first() {
            // GNU patch treats an empty line as an empty context line (some
            // editors strip the trailing space).
            None => (DiffLineKind::Context, ""),
            Some(b' ') => (DiffLineKind::Context, &line[1..]),
            Some(b'+') => (DiffLineKind::Added, &line[1..]),
            Some(b'-') => (DiffLineKind::Removed, &line[1..]),
            Some(b'\\') => {
                let last = hunk
                    .lines
                    .last_mut()
                    .ok_or_else(|| err(i, DiffParseErrorKind::OrphanNoNewlineMarker))?;
                last.no_newline = true;
                i += 1;
                continue;
            }
            Some(_) => return Err(err(i, DiffParseErrorKind::UnexpectedLine)),
        };
        let (old_line, new_line) = match kind {
            DiffLineKind::Context if rem_old > 0 && rem_new > 0 => {
                rem_old -= 1;
                rem_new -= 1;
                (Some(old_no), Some(new_no))
            }
            DiffLineKind::Removed if rem_old > 0 => {
                rem_old -= 1;
                (Some(old_no), None)
            }
            DiffLineKind::Added if rem_new > 0 => {
                rem_new -= 1;
                (None, Some(new_no))
            }
            _ => return Err(err(i, DiffParseErrorKind::UnexpectedLine)),
        };
        if old_line.is_some() {
            old_no = old_no.saturating_add(1);
        }
        if new_line.is_some() {
            new_no = new_no.saturating_add(1);
        }
        hunk.lines.push(DiffLine {
            kind,
            text: text.to_string(),
            old_line,
            new_line,
            no_newline: false,
        });
        i += 1;
    }

    // A trailing `\ No newline at end of file` belongs to the last line.
    if let Some(line) = lines.get(i)
        && line.starts_with('\\')
    {
        let last = hunk
            .lines
            .last_mut()
            .ok_or_else(|| err(i, DiffParseErrorKind::OrphanNoNewlineMarker))?;
        last.no_newline = true;
        i += 1;
    }

    Ok((hunk, i))
}

/// Quote a path the way git does when it contains special characters.
fn quote_path(prefix: &str, path: &str) -> String {
    let needs_quote = path
        .chars()
        .any(|c| c == '"' || c == '\\' || c.is_control())
        || path.starts_with(' ')
        || path.ends_with(' ');
    if !needs_quote {
        return format!("{prefix}{path}");
    }
    let mut out = String::with_capacity(path.len() + prefix.len() + 2);
    out.push('"');
    out.push_str(prefix);
    for byte in path.bytes() {
        match byte {
            b'"' => out.push_str("\\\""),
            b'\\' => out.push_str("\\\\"),
            b'\n' => out.push_str("\\n"),
            b'\t' => out.push_str("\\t"),
            b'\r' => out.push_str("\\r"),
            0x00..=0x1f | 0x7f..=0xff => out.push_str(&format!("\\{byte:03o}")),
            _ => out.push(byte as char),
        }
    }
    out.push('"');
    out
}

/// Write files as a git-style unified diff. See
/// [`super::DiffState::to_unified`].
pub(super) fn write_unified(files: &[DiffFile]) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();
    for file in files {
        if file.old_path.is_none() && file.new_path.is_none() {
            // Bare hunks without any file header are written back bare.
            write_hunks(&mut out, &file.hunks);
            continue;
        }
        let old_name = file.old_path.as_deref().or(file.new_path.as_deref());
        let new_name = file.new_path.as_deref().or(file.old_path.as_deref());
        let (old_name, new_name) = (old_name.unwrap_or("unknown"), new_name.unwrap_or("unknown"));
        let _ = writeln!(
            out,
            "diff --git {} {}",
            quote_path("a/", old_name),
            quote_path("b/", new_name)
        );
        match file.status {
            DiffFileStatus::Added => out.push_str("new file mode 100644\n"),
            DiffFileStatus::Deleted => out.push_str("deleted file mode 100644\n"),
            DiffFileStatus::Renamed => {
                let _ = writeln!(out, "rename from {}", quote_path("", old_name));
                let _ = writeln!(out, "rename to {}", quote_path("", new_name));
            }
            DiffFileStatus::Copied => {
                let _ = writeln!(out, "copy from {}", quote_path("", old_name));
                let _ = writeln!(out, "copy to {}", quote_path("", new_name));
            }
            DiffFileStatus::Modified => {}
        }
        let old_header = file
            .old_path
            .as_deref()
            .map_or_else(|| "/dev/null".to_string(), |p| quote_path("a/", p));
        let new_header = file
            .new_path
            .as_deref()
            .map_or_else(|| "/dev/null".to_string(), |p| quote_path("b/", p));
        if file.binary {
            let _ = writeln!(out, "Binary files {old_header} and {new_header} differ");
        }
        if file.hunks.is_empty() {
            continue;
        }
        let _ = writeln!(out, "--- {old_header}");
        let _ = writeln!(out, "+++ {new_header}");
        write_hunks(&mut out, &file.hunks);
    }
    out
}

fn write_hunks(out: &mut String, hunks: &[DiffHunk]) {
    use std::fmt::Write as _;

    for hunk in hunks {
        let _ = write!(
            out,
            "@@ -{},{} +{},{} @@",
            hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count
        );
        if !hunk.section.is_empty() {
            out.push(' ');
            out.push_str(&hunk.section);
        }
        out.push('\n');
        for line in &hunk.lines {
            out.push(match line.kind {
                DiffLineKind::Context => ' ',
                DiffLineKind::Added => '+',
                DiffLineKind::Removed => '-',
            });
            out.push_str(&line.text);
            out.push('\n');
            if line.no_newline {
                out.push_str("\\ No newline at end of file\n");
            }
        }
    }
}
