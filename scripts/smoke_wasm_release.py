"""Verify exact published packages through a registry-only compiled browser app."""

import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import time
import urllib.error
import urllib.request


def run(command, cwd, **kwargs):
    return subprocess.run(command, cwd=cwd, check=True, text=True, **kwargs)


def verify_docs(version, attempts):
    pending = {"superlighttui": "slt", "slt-wasm": "slt_wasm"}
    for attempt in range(attempts):
        for package, library in list(pending.items()):
            url = f"https://docs.rs/{package}/{version}/{library}/"
            request = urllib.request.Request(url, headers={"User-Agent": "SuperLightTUI release smoke"})
            try:
                with urllib.request.urlopen(request, timeout=30) as response:
                    page = response.read().decode("utf-8")
                    if response.status == 200 and f"/{version}/{library}/" in response.url and f"<title>{library} - Rust</title>" in page:
                        del pending[package]
                        print(f"Verified exact docs: {url}", flush=True)
            except (urllib.error.URLError, TimeoutError):
                pass
        if not pending:
            return
        if attempt + 1 < attempts:
            time.sleep(10)
    raise RuntimeError(f"Exact docs are not ready: {sorted(pending)}")


def verify_registry(version, attempts):
    pending = {"superlighttui": "su/pe/superlighttui", "slt-wasm": "sl/t-/slt-wasm"}
    for attempt in range(attempts):
        for package, index_path in list(pending.items()):
            request = urllib.request.Request(
                f"https://index.crates.io/{index_path}",
                headers={"User-Agent": "SuperLightTUI release smoke"},
            )
            try:
                with urllib.request.urlopen(request, timeout=30) as response:
                    records = [json.loads(line) for line in response.read().decode("utf-8").splitlines()]
                record = next((record for record in records if record["vers"] == version), None)
                if record is not None:
                    if record["yanked"]:
                        raise RuntimeError(f"Published version is yanked: {package} {version}")
                    del pending[package]
                    print(f"Verified registry version: {package} {version}", flush=True)
            except (urllib.error.URLError, TimeoutError):
                pass
        if not pending:
            return
        if attempt + 1 < attempts:
            time.sleep(10)
    raise RuntimeError(f"Exact registry versions are not ready: {sorted(pending)}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("version")
    parser.add_argument("--expect-commit")
    parser.add_argument("--docs-attempts", type=int, default=60)
    args = parser.parse_args()
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", args.version) or args.docs_attempts < 1:
        parser.error("use X.Y.Z and a positive docs-attempts count")
    verify_registry(args.version, args.docs_attempts)
    source = Path(__file__).resolve().parents[1] / "crates/slt-wasm/examples/browser"
    environment = os.environ.copy()
    if "PLAYWRIGHT_MODULE" not in environment:
        environment["PLAYWRIGHT_MODULE"] = run(
            ["node", "-p", "require.resolve('playwright')"], source, capture_output=True
        ).stdout.strip()
    with tempfile.TemporaryDirectory(prefix="slt-wasm-registry-smoke-") as directory:
        consumer = Path(directory)
        shutil.copytree(source / "src", consumer / "src")
        (consumer / "tests").mkdir()
        shutil.copy2(source.parents[1] / "tests/browser_runtime.rs", consumer / "tests/browser_runtime.rs")
        for name in ("index.html", "test.html", "browser.test.cjs"):
            shutil.copy2(source / name, consumer / name)
        (consumer / "Cargo.toml").write_text(f'''[package]
name = "slt-browser-example"
version = "0.0.0"
edition = "2024"
publish = false
[workspace]
[lib]
crate-type = ["cdylib", "rlib"]
[dependencies]
superlighttui = {{ version = "={args.version}", default-features = false }}
slt-wasm = "={args.version}"
wasm-bindgen = "0.2"
js-sys = "0.3"
web-sys = {{ version = "0.3", features = ["Window", "Document", "HtmlElement", "Event", "EventTarget"] }}
[features]
browser-tests = []
[target.'cfg(target_arch = "wasm32")'.dev-dependencies]
wasm-bindgen-test = "0.3"
wasm-bindgen-futures = "0.4"
[package.metadata.wasm-pack.profile.release]
wasm-opt = false
''', encoding="utf-8")
        metadata = json.loads(run(
            ["cargo", "metadata", "--format-version", "1", "--filter-platform", "wasm32-unknown-unknown"],
            consumer, capture_output=True,
        ).stdout)
        nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
        for name in ("superlighttui", "slt-wasm"):
            packages = [package for package in metadata["packages"] if package["name"] == name]
            if len(packages) != 1 or packages[0]["version"] != args.version:
                raise RuntimeError(f"Wrong package resolution: {name}")
            package = packages[0]
            if not (package["source"] or "").startswith("registry+"):
                raise RuntimeError(f"Non-registry override detected: {name}")
            if name == "superlighttui" and "crossterm" in nodes[package["id"]]["features"]:
                raise RuntimeError("Native terminal features leaked into browser consumer")
            if args.expect_commit:
                vcs = json.loads((Path(package["manifest_path"]).parent / ".cargo_vcs_info.json").read_text())
                if vcs["git"]["sha1"] != args.expect_commit or vcs["git"].get("dirty", False):
                    raise RuntimeError(f"Published source provenance mismatch: {name}")
        run(["wasm-pack", "build", "--target", "web", "--dev", "--", "--features", "browser-tests", "--locked"], consumer)
        run(["node", "browser.test.cjs"], consumer, env=environment)
        run(["wasm-pack", "test", "--headless", "--chrome"], consumer, env=environment)
    verify_docs(args.version, args.docs_attempts)
    print(f"Both {args.version} registry packages, browser runtime and exact docs verified")


if __name__ == "__main__":
    main()
