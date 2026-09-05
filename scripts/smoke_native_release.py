"""Run the native PTY acceptance suite against an exact registry-only consumer."""

import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile

from smoke_wasm_release import verify_registry


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("version")
    parser.add_argument("--expect-commit")
    args = parser.parse_args()
    if os.name != "posix":
        parser.error("native PTY verification requires Unix")
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", args.version):
        parser.error("use an exact X.Y.Z version")
    verify_registry(args.version, 60)
    root = Path(__file__).resolve().parents[1]
    with tempfile.TemporaryDirectory(prefix="slt-native-registry-smoke-") as directory:
        consumer = Path(directory)
        (consumer / "src").mkdir()
        (consumer / "src/lib.rs").write_text("// Registry-only native acceptance consumer.\n", encoding="utf-8")
        (consumer / "tests/support").mkdir(parents=True)
        shutil.copy2(root / "tests/v024_burst.rs", consumer / "tests/v024_burst.rs")
        shutil.copy2(root / "tests/support/burst_pty.py", consumer / "tests/support/burst_pty.py")
        (consumer / "Cargo.toml").write_text(f'''[package]
name = "slt-native-registry-smoke"
version = "0.0.0"
edition = "2024"
publish = false
[workspace]
[dependencies]
superlighttui = {{ version = "={args.version}", features = ["async"] }}
rustix = {{ version = "1", features = ["fs", "stdio"] }}
tokio = {{ version = "1", features = ["rt", "sync", "time"] }}
[features]
default = ["crossterm", "async"]
crossterm = []
async = []
''', encoding="utf-8")
        metadata = json.loads(subprocess.run(
            ["cargo", "metadata", "--format-version", "1"], cwd=consumer,
            check=True, capture_output=True, text=True,
        ).stdout)
        packages = [package for package in metadata["packages"] if package["name"] == "superlighttui"]
        if len(packages) != 1 or packages[0]["version"] != args.version:
            raise RuntimeError("Wrong superlighttui resolution")
        package = packages[0]
        if not (package["source"] or "").startswith("registry+"):
            raise RuntimeError("Native consumer resolved a path/git override")
        if args.expect_commit:
            vcs = json.loads((Path(package["manifest_path"]).parent / ".cargo_vcs_info.json").read_text())
            if vcs["git"]["sha1"] != args.expect_commit or vcs["git"].get("dirty", False):
                raise RuntimeError("Published native source provenance mismatch")
        result = subprocess.run(
            ["cargo", "test", "--locked", "--test", "v024_burst", "--", "--test-threads=1", "--nocapture"],
            cwd=consumer, capture_output=True, text=True,
        )
        print(result.stdout, end="", flush=True)
        print(result.stderr, end="", file=sys.stderr, flush=True)
        result.check_returncode()
        summaries = re.findall(r"test result: ok\. (\d+) passed; 0 failed; 0 ignored", result.stdout)
        if not summaries or max(map(int, summaries)) < 6:
            raise RuntimeError("Native acceptance tests were not all executed")
    print(f"superlighttui {args.version} exact registry native PTY acceptance passed", flush=True)


if __name__ == "__main__":
    main()
