#!/usr/bin/env bash
#
# ghostty_demos.sh — interactive launcher for registered SuperLightTUI demos.
#
# Each chosen demo is opened in a fresh Ghostty.app window via
#   open -na Ghostty.app -e "<command>"
# so multiple demos run side-by-side without clobbering the current terminal.
#
# Usage examples:
#   ./scripts/ghostty_demos.sh                    # interactive picker
#   ./scripts/ghostty_demos.sh all                # every registered example
#   ./scripts/ghostty_demos.sh showcase_tour      # explicit target names
#   ./scripts/ghostty_demos.sh --showcase         # integration tours
#   ./scripts/ghostty_demos.sh --features         # feature tours
#   ./scripts/ghostty_demos.sh --build-first all  # pre-build, then launch all
#   ./scripts/ghostty_demos.sh --help             # full help
#
set -euo pipefail

# --- paths ---------------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
EXAMPLES_DIR="${REPO_ROOT}/examples"
GHOSTTY_APP="/Applications/Ghostty.app"

# --- preset playlists ----------------------------------------------------

# Integration tours that exercise composed application layouts.
SHOWCASE_PRESET=(
    "showcase_tour"
    "cookbook_tour"
    "demo"
)

# Registered tours that cover feature families without advertising source-only
# modules as standalone Cargo targets.
FEATURES_PRESET=(
    "v020_tour"
    "v0210_widgets"
    "v0211_tour"
    "canvas_tour"
    "text_tour"
    "system_tour"
)

# --- helpers -------------------------------------------------------------

print_usage() {
    cat <<'EOF'
ghostty_demos.sh — launch registered SuperLightTUI examples in fresh Ghostty windows.

USAGE:
    scripts/ghostty_demos.sh [FLAGS] [DEMOS...]

FLAGS:
    -h, --help          Show this help and exit.
    --build-first       Run `cargo build --examples --all-features` before
                        launching, so each Ghostty window skips the long
                        first-compile output.
    --showcase          Launch the integration tours.
    --features          Launch the feature tours.
    --list              Print registered Cargo example targets and exit.

POSITIONAL:
    all                 Launch every registered Cargo example.
    <name> [<name>...]  Launch only the named Cargo example targets.

INTERACTIVE MODE:
    With no arguments, prints a numbered menu of all v020_* demos and
    prompts:
        select demos (e.g. 1 3 5 or 'all'):

EXAMPLES:
    # Interactive picker
    scripts/ghostty_demos.sh

    # Pre-build once, then launch everything
    scripts/ghostty_demos.sh --build-first all

    # Integration tours
    scripts/ghostty_demos.sh --showcase

    # Feature tours
    scripts/ghostty_demos.sh --features

    # Specific registered targets
    scripts/ghostty_demos.sh showcase_tour v020_tour

NOTES:
    - Requires Ghostty.app at /Applications/Ghostty.app. If missing, the
      script falls back to AppleScript / new Terminal tabs and warns.
    - Each launched window runs:
          cd <repo> && cargo run --example <name>
    - Ctrl-C in a Ghostty window stops only that demo.
EOF
}

err() {
    printf 'ghostty_demos: %s\n' "$*" >&2
}

# Print every explicit [[example]] target from Cargo.toml, one per line.
discover_demos() {
    if [[ ! -f "${REPO_ROOT}/Cargo.toml" ]]; then
        err "Cargo.toml not found: ${REPO_ROOT}"
        return 1
    fi
    awk '
        /^\[\[example\]\]$/ { in_example = 1; next }
        /^\[\[/ { in_example = 0 }
        in_example && /^name = "/ {
            name = $0
            sub(/^name = "/, "", name)
            sub(/".*$/, "", name)
            print name
        }
    ' "${REPO_ROOT}/Cargo.toml" | LC_ALL=C sort -u
}

# Verify a name is an explicit Cargo example target.
demo_exists() {
    local name="$1"
    local candidate
    for candidate in "${ALL_DEMOS[@]}"; do
        [[ "${candidate}" == "${name}" ]] && return 0
    done
    return 1
}

# Open one demo in a fresh Ghostty window if available, else fall back.
launch_demo() {
    local name="$1"
    local repo_q name_q cmd
    printf -v repo_q '%q' "${REPO_ROOT}"
    printf -v name_q '%q' "${name}"
    cmd="cd ${repo_q} && cargo run --all-features --example ${name_q}"

    if [[ -d "${GHOSTTY_APP}" ]]; then
        printf '  -> Ghostty: %s\n' "${name}"
        open -na "Ghostty.app" -e "${cmd}"
        return 0
    fi

    # Fallback 1: macOS Terminal.app via AppleScript.
    if command -v osascript >/dev/null 2>&1; then
        printf '  -> Terminal.app (Ghostty not found): %s\n' "${name}"
        local script
        printf -v script 'tell application "Terminal" to do script "%s"' \
            "${cmd//\"/\\\"}"
        osascript -e "${script}" >/dev/null
        return 0
    fi

    # Fallback 2: just print what we *would* have run.
    err "Ghostty.app and osascript both unavailable; please run manually:"
    err "    ${cmd}"
    return 1
}

# Build all examples once so each Ghostty window opens straight into the
# running TUI instead of a multi-second cargo compile log.
build_all_examples() {
    printf 'Building all examples (cargo build --examples --all-features)...\n'
    (
        cd "${REPO_ROOT}"
        cargo build --examples --all-features
    )
    printf 'Build complete.\n\n'
}

# --- argument parsing ----------------------------------------------------

build_first=0
preset=""

# Scan flags first so they can appear in any position.
positional=()
while (( $# > 0 )); do
    case "$1" in
        -h|--help)
            print_usage
            exit 0
            ;;
        --build-first)
            build_first=1
            shift
            ;;
        --showcase)
            preset="showcase"
            shift
            ;;
        --features)
            preset="features"
            shift
            ;;
        --list)
            discover_demos
            exit 0
            ;;
        --)
            shift
            while (( $# > 0 )); do
                positional+=("$1")
                shift
            done
            ;;
        -*)
            err "unknown flag: $1"
            err "run with --help for usage"
            exit 2
            ;;
        *)
            positional+=("$1")
            shift
            ;;
    esac
done

# --- environment sanity --------------------------------------------------

if [[ ! -d "${EXAMPLES_DIR}" ]]; then
    err "examples dir not found: ${EXAMPLES_DIR}"
    err "is this script being run from a SuperLightTUI checkout?"
    exit 1
fi

if [[ ! -d "${GHOSTTY_APP}" ]]; then
    err "warning: ${GHOSTTY_APP} not found; will fall back to Terminal.app"
fi

# --- discover --------------------------------------------------------------

ALL_DEMOS=()
while IFS= read -r demo; do
    [[ -n "${demo}" ]] && ALL_DEMOS+=("${demo}")
done < <(discover_demos || true)

if (( ${#ALL_DEMOS[@]} == 0 )); then
    err "no registered examples found in Cargo.toml"
    err "v0.20 demos may not have landed yet on this branch."
    exit 1
fi

# --- resolve which demos to launch ---------------------------------------

selected=()

if [[ -n "${preset}" ]]; then
    if (( ${#positional[@]} > 0 )); then
        err "cannot combine preset (--${preset}) with positional demo names"
        exit 2
    fi
    case "${preset}" in
        showcase) selected=("${SHOWCASE_PRESET[@]}") ;;
        features) selected=("${FEATURES_PRESET[@]}") ;;
    esac
elif (( ${#positional[@]} == 1 )) && [[ "${positional[0]}" == "all" ]]; then
    selected=("${ALL_DEMOS[@]}")
elif (( ${#positional[@]} > 0 )); then
    for raw in "${positional[@]}"; do
        selected+=("${raw}")
    done
else
    # Interactive picker.
    printf 'SuperLightTUI v0.20 demo launcher\n'
    printf '=================================\n\n'
    printf 'Available demos (%d):\n' "${#ALL_DEMOS[@]}"
    i=0
    for name in "${ALL_DEMOS[@]}"; do
        i=$((i + 1))
        printf '  %2d) %s\n' "${i}" "${name}"
    done
    printf '\n'
    printf "select demos (e.g. 1 3 5 or 'all'): "
    IFS= read -r reply || true
    reply="${reply## }"
    reply="${reply%% }"

    if [[ -z "${reply}" ]]; then
        err "no selection; nothing to do"
        exit 0
    fi

    if [[ "${reply}" == "all" ]]; then
        selected=("${ALL_DEMOS[@]}")
    else
        # Parse space-separated indices.
        for tok in ${reply}; do
            if [[ "${tok}" =~ ^[0-9]+$ ]]; then
                idx=$((tok - 1))
                if (( idx < 0 || idx >= ${#ALL_DEMOS[@]} )); then
                    err "index out of range: ${tok}"
                    exit 2
                fi
                selected+=("${ALL_DEMOS[${idx}]}")
            else
                # Allow names alongside numbers, just in case.
                selected+=("${tok}")
            fi
        done
    fi
fi

if (( ${#selected[@]} == 0 )); then
    err "nothing selected"
    exit 0
fi

# --- validate -------------------------------------------------------------

missing=()
for name in "${selected[@]}"; do
    if ! demo_exists "${name}"; then
        missing+=("${name}")
    fi
done

if (( ${#missing[@]} > 0 )); then
    err "the following demos do not exist on this branch:"
    for name in "${missing[@]}"; do
        err "  - ${name}"
    done
    err "run 'scripts/ghostty_demos.sh --list' to see available demos."
    exit 1
fi

# --- optional pre-build ---------------------------------------------------

if (( build_first == 1 )); then
    build_all_examples
fi

# --- launch ---------------------------------------------------------------

printf 'Launching %d demo(s) in fresh Ghostty windows...\n' "${#selected[@]}"
for name in "${selected[@]}"; do
    launch_demo "${name}"
done
printf 'Done. %d Ghostty window(s) requested.\n' "${#selected[@]}"
