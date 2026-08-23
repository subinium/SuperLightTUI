#!/usr/bin/env bash
# Compile and run an exact-version downstream consumer from crates.io.

set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "usage: scripts/smoke_release.sh X.Y.Z" >&2
    exit 2
fi

VERSION="$1"
if [[ ! "${VERSION}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "invalid version: ${VERSION}" >&2
    exit 2
fi

SMOKE_DIR="$(mktemp -d "${TMPDIR:-/tmp}/slt-release-smoke.XXXXXX")"
cleanup() {
    rm -rf -- "${SMOKE_DIR}"
}
trap cleanup EXIT

cargo init --quiet --bin --name slt-release-smoke "${SMOKE_DIR}"
(
    cd "${SMOKE_DIR}"
    cargo add --quiet "superlighttui@=${VERSION}"
    cat > src/main.rs <<'RS'
use slt::TestBackend;

fn main() {
    let mut backend = TestBackend::new(20, 2);
    backend.render(|ui| {
        ui.text("release smoke");
    });
    backend.assert_contains("release smoke");
}
RS
    cargo run --quiet --locked
)

echo "superlighttui ${VERSION} downstream smoke passed"
