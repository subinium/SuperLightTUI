#!/usr/bin/env bash
# api_audit.sh — heuristic API consistency audit for SuperLightTUI.
#
# Reports (does not fix) the high-frequency violations called out in
# docs/DESIGN_PRINCIPLES.md plus three v0.20 demo-bug regressions:
#
#   V1 — Two paths to the same thing
#         Same method on Context AND ContainerBuilder.
#   V2 — Mixed verbs in same family
#         Immediate `widget(args) -> Response` next to a builder type
#         `<Widget><Lifetime>` for the same family in the same file.
#   V3 — Naming length mismatch (informational)
#         An impl block where one public method is > 20 chars and
#         another is <= 6 chars. Often correct, but worth a glance.
#   V4 — Public API missing rustdoc
#         pub fn / pub struct / pub enum without /// directly above.
#   V5 — Outer container missing grow/fill (v0.20+ demos)
#         examples/v020_*.rs whose entry-point function ships an
#         outermost `.bordered(...)` / `.container()` chain that lacks
#         `.grow(N)` or `.fill()` before the `.col(|ui|`/`.row(|ui|`
#         closure. Without one of those, the inner area collapses to
#         intrinsic widget width and inputs render as 1-cell strips.
#   V6 — Fallback path container nesting divergence (informational)
#         Pattern: an `if let Some(...) { ui.line(|ui| inner(ui)) }
#         else { inner(ui) }` shape where one branch wraps the inner
#         call in `ui.line(`/`ui.row(`/`ui.col(` and the other does
#         not, producing inconsistent indentation/wrapping. v0.20 ships
#         this as informational only — see the section below for the
#         v0.21 dylint-based plan.
#   V7 — Demo title wide-character drift (v0.20+ demos)
#         `.title("…")` strings inside examples/v020_*.rs that contain
#         non-ASCII codepoints not in the per-check allowlist. Wide
#         glyphs (em-dash U+2014, en-dash U+2013, ideographs, etc.)
#         cause border-misalignment in terminals that report a
#         single-cell width for them.
#
# Historical bugs each new check defends against:
#   V5 → examples/v020_named_focus.rs shipped without `.grow(1)`,
#        making the email/name/city inputs render as a single column.
#   V6 → src/context/widgets_display/status.rs `code_block_lang`
#        previously wrapped the tree-sitter branch in `ui.line(...)`
#        but called `render_highlighted_line(...)` bare in the
#        non-highlight fallback, producing inconsistent line wrapping.
#   V7 → examples like `"SLT v0.20 — Density presets"` shipped with
#        an em-dash (U+2014) in the title, breaking border alignment
#        on terminals that render it as a 1-cell glyph.
#
# Output is human-readable plus an exit code. In v0.23, CI gates on V1, V2,
# V4, V5, and V7; V3 and V6 remain informational.
#
# Run from repo root:
#   scripts/api_audit.sh
#   scripts/api_audit.sh --strict   # exit 1 on V1/V2/V4/V5/V7
#
# This is intentionally heuristic — false positives are expected and
# allowlisted via the per-check "allowlist" sections below. The point is
# a regular signal, not perfection.

set -uo pipefail

REPO_ROOT="${SLT_AUDIT_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
SRC="${REPO_ROOT}/src"
EXAMPLES="${REPO_ROOT}/examples"

if [[ ! -d "${SRC}" ]]; then
    echo "api_audit: src/ not found at ${SRC}" >&2
    exit 2
fi

STRICT=0
if [[ "${1:-}" == "--strict" ]]; then
    STRICT=1
fi

violations=0
warnings=0

# --- V1: Two-path methods ----------------------------------------------------

echo "── V1: Two-path methods (Context vs ContainerBuilder) ──"

# Allowlist — methods intentionally overloaded between Context's immediate
# text/widget modifiers and ContainerBuilder's deferred container modifiers.
# Any new overlap must be reviewed and added explicitly.
V1_ALLOWLIST=(
    "align"
    "bg"
    "col"
    "col_gap"
    "color"
    "group_hover_bg"
    "grow"
    "h"
    "m"
    "max_h"
    "max_w"
    "mb"
    "min_h"
    "min_w"
    "ml"
    "mr"
    "mt"
    "mx"
    "my"
    "row"
    "row_gap"
    "text"           # Context: unbordered shortcut; Builder: inside-builder form
    "theme"          # Context: getter; Builder: per-subtree override
    "width"          # Context: terminal width; Builder: w() (different name already)
    "height"         # same as width
    "push_container" # internal helper used in both layers
    "line"           # Context: row-shorthand; ContainerBuilder: not present (false pos)
    "w"
    "with"
    "with_if"
    "wrap"
)

is_allowlisted_v1() {
    local m="$1"
    for allow in "${V1_ALLOWLIST[@]}"; do
        [[ "$m" == "$allow" ]] && return 0
    done
    return 1
}

# Collect Context-layer pub fn names. Context lives in core.rs, runtime.rs,
# and the various widgets_*/*.rs files (those add `impl Context { ... }`
# blocks).
ctx_files=(
    "${SRC}/context/core.rs"
    "${SRC}/context/runtime.rs"
)
for d in widgets_display widgets_input widgets_interactive widgets_viz; do
    if [[ -d "${SRC}/context/${d}" ]]; then
        while IFS= read -r f; do
            ctx_files+=("$f")
        done < <(find "${SRC}/context/${d}" -name '*.rs' -type f)
    fi
done

ctx_methods=""
for f in "${ctx_files[@]}"; do
    [[ -f "$f" ]] || continue
    ctx_methods+=$(grep -hE '^[[:space:]]*pub[[:space:]]+fn[[:space:]]+[[:alnum:]_]+' "$f" 2>/dev/null \
        | sed -E 's/.*pub[[:space:]]+fn[[:space:]]+([[:alnum:]_]+).*/\1/' || true)
    ctx_methods+=$'\n'
done
ctx_methods=$(printf '%s' "${ctx_methods}" | sort -u)

# ContainerBuilder lives in container.rs.
cb_methods=$(grep -hE '^[[:space:]]*pub[[:space:]]+fn[[:space:]]+[[:alnum:]_]+' "${SRC}/context/container.rs" 2>/dev/null \
    | sed -E 's/.*pub[[:space:]]+fn[[:space:]]+([[:alnum:]_]+).*/\1/' \
    | sort -u || true)

shared=$(comm -12 <(echo "${ctx_methods}") <(echo "${cb_methods}") || true)
v1_count=0
if [[ -n "${shared}" ]]; then
    while IFS= read -r m; do
        [[ -z "$m" ]] && continue
        if is_allowlisted_v1 "$m"; then
            continue
        fi
        echo "  V1: ${m} defined on both Context and ContainerBuilder"
        v1_count=$((v1_count + 1))
    done <<< "${shared}"
fi
if [[ "${v1_count}" -eq 0 ]]; then
    echo "  ✅ none (allowlist size: ${#V1_ALLOWLIST[@]})"
fi
violations=$((violations + v1_count))

# --- V2: Mixed verbs (immediate + builder for same widget) -------------------

echo
echo "── V2: Mixed verbs in same widget file ──"

# Heuristic: a widget file with both
#   pub fn <name>(...) -> Response   (immediate)
# and a corresponding
#   pub struct <Name><Lifetime>      (builder)
# for the SAME widget name.

v2_count=0
for f in "${SRC}/context/widgets_display/"*.rs \
         "${SRC}/context/widgets_input/"*.rs \
         "${SRC}/context/widgets_interactive/"*.rs; do
    [[ -f "$f" ]] || continue

    # Immediate fns: pub fn returning Response on `&mut self`.
    immediate_fns=$(grep -E '^[[:space:]]*pub[[:space:]]+fn[[:space:]]+[[:alnum:]_]+\([^)]*&mut[[:space:]]+self' "$f" 2>/dev/null \
        | grep -E '\)[[:space:]]*->[[:space:]]*Response' \
        | sed -E 's/.*pub[[:space:]]+fn[[:space:]]+([[:alnum:]_]+).*/\1/' || true)

    # Builder structs in same file.
    builder_types=$(grep -E '^[[:space:]]*pub[[:space:]]+struct[[:space:]]+[A-Z][[:alnum:]_]*<' "$f" 2>/dev/null \
        | sed -E 's/.*pub[[:space:]]+struct[[:space:]]+([A-Z][[:alnum:]_]*).*/\1/' || true)

    [[ -z "${immediate_fns}" || -z "${builder_types}" ]] && continue

    while IFS= read -r fn; do
        [[ -z "$fn" ]] && continue
        # Convert snake_case to PascalCase.
        bt=$(echo "$fn" | awk -F_ '{
            for(i=1;i<=NF;i++) printf "%s%s", toupper(substr($i,1,1)), substr($i,2)
        }')
        if echo "${builder_types}" | grep -qx "$bt"; then
            echo "  V2: ${fn}() (immediate) coexists with ${bt}<'_> (builder) in $(basename "$f")"
            v2_count=$((v2_count + 1))
        fi
    done <<< "${immediate_fns}"
done
if [[ "${v2_count}" -eq 0 ]]; then
    echo "  ✅ none"
fi
violations=$((violations + v2_count))

# --- V3: Naming length mismatch (informational) ------------------------------

echo
echo "── V3: Naming length mismatch (informational) ──"

# Heuristic: per-impl-block, list public method names. Flag when one is
# > 20 chars and another is <= 6 chars in the same file. Often correct
# (different categories), but worth a human glance.
v3_files=()
for f in "${SRC}/context/container.rs" \
         "${SRC}/context/runtime.rs" \
         "${SRC}/context/core.rs"; do
    [[ -f "$f" ]] || continue
    methods=$(grep -E '^[[:space:]]*pub[[:space:]]+fn[[:space:]]+[[:alnum:]_]+' "$f" \
        | sed -E 's/.*pub[[:space:]]+fn[[:space:]]+([[:alnum:]_]+).*/\1/' \
        | sort -u)
    long=$(echo "${methods}" | awk 'length > 20')
    short=$(echo "${methods}" | awk 'length <= 6 && length >= 1')
    if [[ -n "${long}" && -n "${short}" ]]; then
        v3_files+=("$f")
    fi
done

if [[ "${#v3_files[@]}" -gt 0 ]]; then
    for f in "${v3_files[@]}"; do
        echo "  V3 (info): $(basename "$f") has both long (>20) and short (<=6) public method names"
    done
    echo "    (informational — see NAMING.md \"Length Conventions\")"
    warnings=$((warnings + ${#v3_files[@]}))
else
    echo "  ✅ none"
fi

# --- V4: Public API missing rustdoc ------------------------------------------

echo
echo "── V4: Public API missing rustdoc ──"

# Heuristic: every pub fn / pub struct / pub enum should have a /// line
# directly above (allowing #[derive(...)] / #[must_use] / blank attrs in
# between). Skip pub(crate) and pub(super).
v4_count=0
while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    file="${line%%:*}"
    rest="${line#*:}"
    lineno="${rest%%:*}"
    [[ "$lineno" -lt 1 ]] && continue

    # Walk upward past complete attributes (including multiline attributes)
    # and blank lines, looking for the nearest preceding non-blank line.
    seek=$((lineno - 1))
    in_multiline_attr=0
    while [[ "$seek" -ge 1 ]]; do
        prev_line=$(sed -n "${seek}p" "$file" 2>/dev/null || echo "")
        if [[ -z "${prev_line//[[:space:]]/}" ]]; then
            seek=$((seek - 1))
            continue
        fi
        if [[ "$prev_line" =~ ^[[:space:]]*//[^/] ]]; then
            seek=$((seek - 1))
            continue
        fi

        if [[ "$in_multiline_attr" -eq 1 ]]; then
            if [[ "$prev_line" =~ ^[[:space:]]*\#\[ ]]; then
                in_multiline_attr=0
            fi
            seek=$((seek - 1))
            continue
        fi

        # A single-line attribute starts with `#[` and ends with `]`. A
        # multiline attribute is encountered backwards at its closing `]` /
        # `)]`; keep walking until its opening `#[` line is consumed.
        if [[ "$prev_line" =~ ^[[:space:]]*\#\[ ]]; then
            seek=$((seek - 1))
            continue
        fi
        if [[ "$prev_line" =~ \][[:space:]]*$ ]]; then
            in_multiline_attr=1
            seek=$((seek - 1))
            continue
        fi
        break
    done

    prev_line=$(sed -n "${seek}p" "$file" 2>/dev/null || echo "")
    if ! [[ "$prev_line" =~ ^[[:space:]]*/// ]]; then
        echo "  V4: ${file}:${lineno} — missing rustdoc"
        v4_count=$((v4_count + 1))
    fi
done < <(grep -rnE '^[[:space:]]*pub[[:space:]]+(fn|struct|enum)[[:space:]]+[A-Za-z_]' "${SRC}" 2>/dev/null \
    | grep -v 'pub(' || true)

if [[ "${v4_count}" -eq 0 ]]; then
    echo "  ✅ none"
fi
violations=$((violations + v4_count))

# --- V5: Outer container missing grow/fill (v0.20+ demos) --------------------

echo
echo "── V5: Outer container missing grow/fill (v020_*.rs) ──"

# Heuristic. For each examples/v020_*.rs file:
#   1. Locate the entry-point function: prefer `pub fn render(`, then
#      `fn body(`, then `fn main(` (with `slt::run` inside).
#   2. Extract its body by tracking `{`/`}` depth from the opening line.
#   3. Within that body, look for the first chain anchored on
#      `.bordered(` or `.container()` and ending at `.col(|ui|` /
#      `.row(|ui|`.
#   4. Flag when the chain does NOT contain `.grow(` or `.fill()`.
#
# Files that delegate to a helper (no chain in the entry-point body)
# are skipped — V5 does not chase across function boundaries. The
# v0.21 dylint-based rule will handle inter-procedural cases.
#
# Allowlist: demos that intentionally use a non-growing top-level chain
# (e.g. width-bounded showcase with an outer scroll container). Add
# basenames here to suppress flagging.
V5_ALLOWLIST=(
    "v020_test_utils.rs"   # not a runnable demo — exercises test harness only
)

is_allowlisted_v5() {
    local b="$1"
    for allow in "${V5_ALLOWLIST[@]}"; do
        [[ "$b" == "$allow" ]] && return 0
    done
    return 1
}

v5_count=0
if [[ -d "${EXAMPLES}" ]]; then
    while IFS= read -r f; do
        [[ -f "$f" ]] || continue
        base=$(basename "$f")
        is_allowlisted_v5 "$base" && continue

        # Pick the first matching entry-point line by priority.
        start=""
        for pat in '^pub fn render\(' '^fn body\(' '^fn main\('; do
            line=$(grep -nE "$pat" "$f" 2>/dev/null | head -1 | cut -d: -f1 || true)
            if [[ -n "$line" ]]; then
                # For `fn main(`, require a `slt::run` somewhere in the file.
                if [[ "$pat" == '^fn main\(' ]] \
                    && ! grep -q 'slt::run' "$f" 2>/dev/null; then
                    continue
                fi
                start="$line"
                break
            fi
        done
        [[ -z "$start" ]] && continue

        # Walk forward from `start`, tracking { / } depth across the
        # function. Capture lines up to (and including) the matching
        # closing brace into `body`.
        body=$(awk -v s="$start" '
            NR < s { next }
            NR == s { depth = 0 }
            { print }
            {
                for (i = 1; i <= length($0); i++) {
                    c = substr($0, i, 1)
                    if (c == "{") depth++
                    else if (c == "}") {
                        depth--
                        if (depth == 0 && NR > s) exit
                    }
                }
            }
        ' "$f")
        [[ -z "$body" ]] && continue

        # Within the body, capture the first chain that begins at a
        # line containing `.bordered(` or `.container()` and ends at a
        # line containing `.col(|ui|` or `.row(|ui|`. This ignores
        # later inner chains.
        chain=$(echo "$body" | awk '
            /\.bordered\(|\.container\(\)/ { capture = 1 }
            capture {
                print
                if ($0 ~ /\.col\(\|ui\||\.row\(\|ui\|/) exit
            }
        ')
        [[ -z "$chain" ]] && continue

        if ! echo "$chain" | grep -qE '\.grow\(|\.fill\(\)'; then
            echo "  V5: ${base} — outer chain lacks .grow(N) or .fill() before .col(|ui|/.row(|ui|"
            v5_count=$((v5_count + 1))
        fi
    done < <(find "${EXAMPLES}" -maxdepth 1 -name 'v020_*.rs' -type f 2>/dev/null | sort)
fi

if [[ "${v5_count}" -eq 0 ]]; then
    echo "  ✅ none (allowlist size: ${#V5_ALLOWLIST[@]})"
fi
violations=$((violations + v5_count))

# --- V6: Fallback path container nesting divergence (informational) ----------

echo
echo "── V6: Fallback path container nesting divergence ──"

# Catching this reliably with grep is impractical: the bug requires
# matching a primary `if let Some(...) { ... ui.line(|ui| inner(ui)) }`
# against an `else { inner(ui) }` and confirming that the inner
# function is the same in both branches. Plain regex flags hundreds of
# false positives across src/.
#
# Future plan: replace this section with a syntax-aware AST rule that
# walks each `if let`/`match` expression, identifies its mirror branch,
# normalizes wrap-call shapes (line/row/col/styled-line), and flags
# only when wrap depth differs for an otherwise-identical body. Until
# then, this section is a manual-review reminder so the principle
# stays visible in CI logs.
echo "  V6 (informational): manual review required for fallback parity"
echo "    (see status.rs::code_block_lang for the canonical pattern fixed in v0.20.x;"
echo "     a future syntax-aware rule should mechanize this check)"

# --- V7: Demo title wide-character drift (v0.20+ demos) ----------------------

echo
echo "── V7: Demo title wide-character drift (v020_*.rs) ──"

# Heuristic. For each `.title("…")` call inside examples/v020_*.rs,
# inspect the literal between the first pair of double quotes and
# flag any byte ≥ 0x80 (UTF-8 lead/cont byte) that isn't part of an
# allowlisted multi-byte sequence.
#
# Implementation note: we rely on `LC_ALL=C` plus bash `$'\x80'`-style
# byte literals so the byte range is consistent across BSD grep
# (macOS) and GNU grep (Linux). Plain `\x` inside a regex is NOT
# portable across grep implementations — the `$'…'` form expands the
# escape in the shell before grep sees it, sidestepping the issue.
#
# Allowlisted UTF-8 byte sequences (regex alternation literal). The
# bug we're catching is specifically em-dash (U+2014 → e2 80 94),
# en-dash (U+2013 → e2 80 93), ideographic glyphs, etc., so the
# default is empty — flag any non-ASCII byte. Add to this list (e.g.
# $'\xc3\xb1' for `ñ`) only when a demo legitimately needs it.
V7_ALLOWED_PATTERN=""

v7_count=0
if [[ -d "${EXAMPLES}" ]]; then
    while IFS= read -r f; do
        [[ -f "$f" ]] || continue
        base=$(basename "$f")

        # Grab raw .title("…") occurrences, then test the inner
        # literal for non-ASCII bytes.
        while IFS= read -r hit; do
            [[ -z "$hit" ]] && continue
            lineno="${hit%%:*}"
            rest="${hit#*:}"
            # Extract the first `"…"` literal after `.title(`.
            literal=$(LC_ALL=C printf '%s' "$rest" \
                | LC_ALL=C sed -nE 's/.*\.title\("([^"]*)".*/\1/p')
            [[ -z "$literal" ]] && continue

            # Strip allowlisted byte sequences before scanning.
            scan="$literal"
            if [[ -n "$V7_ALLOWED_PATTERN" ]]; then
                scan=$(LC_ALL=C printf '%s' "$scan" \
                    | LC_ALL=C sed -E "s/(${V7_ALLOWED_PATTERN})//g")
            fi

            # Use bash $'…' to expand the byte range BEFORE grep
            # parses the regex. Anything ≥ 0x80 is non-ASCII.
            if LC_ALL=C printf '%s' "$scan" \
                | LC_ALL=C grep -q $'[\x80-\xff]'; then
                # Show the literal (truncated) for quick triage.
                snippet=$(LC_ALL=C printf '%s' "$literal" | head -c 80)
                echo "  V7: ${base}:${lineno} — non-ASCII char in .title(\"${snippet}\")"
                v7_count=$((v7_count + 1))
            fi
        done < <(LC_ALL=C grep -nE '\.title\("' "$f" 2>/dev/null || true)
    done < <(find "${EXAMPLES}" -maxdepth 1 -name 'v020_*.rs' -type f 2>/dev/null | sort)
fi

if [[ "${v7_count}" -eq 0 ]]; then
    echo "  ✅ none"
fi
violations=$((violations + v7_count))

# --- Summary -----------------------------------------------------------------

echo
echo "── Summary ──"
echo "Violations (V1+V2+V4+V5+V7):  ${violations}"
echo "Warnings   (V3+V6 info):     ${warnings}"
echo

if [[ "${violations}" -eq 0 ]]; then
    echo "✅ Clean."
    exit 0
fi

echo "⚠️  Reported ${violations} violation(s)."
if [[ "${STRICT}" -eq 1 ]]; then
    echo "    --strict mode: exiting 1 (CI gates V1/V2/V4/V5/V7)."
    exit 1
fi
echo "    Run 'scripts/api_audit.sh --strict' to apply the CI gate locally."
exit 0
