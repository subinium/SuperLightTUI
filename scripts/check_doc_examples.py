#!/usr/bin/env python3
"""Compile explicitly maintained Markdown recipes, never arbitrary Markdown."""

import argparse
import collections
import dataclasses
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile


@dataclasses.dataclass
class Example:
    name: str
    features: tuple[str, ...]
    context: str
    tokio: bool
    code: str
    source: str
    line: int


def extract(path):
    lines = path.read_text(encoding="utf-8").splitlines()
    examples = []
    illustrative = 0
    classified_default = False
    pending = None
    fence = None
    body = []
    start = 0
    names = set()
    for number, line in enumerate(lines, 1):
        if fence is not None:
            if re.fullmatch(r"\s*`{%d,}\s*" % fence[0], line):
                if fence[1] == "rust":
                    if pending is None:
                        if not classified_default:
                            raise ValueError(f"{path}:{start}: Rust fence lacks classification")
                        illustrative += 1
                    else:
                        name = pending["name"]
                        if not re.fullmatch(r"[a-z][a-z0-9_]*", name) or name in names:
                            raise ValueError(f"{path}:{start}: invalid/duplicate name: {name}")
                        names.add(name)
                        features = pending["features"]
                        if not isinstance(features, list) or not all(isinstance(f, str) for f in features):
                            raise ValueError(f"{path}:{start}: features must be a string list")
                        if pending["context"] not in ("module", "ui"):
                            raise ValueError(f"{path}:{start}: context must be module or ui")
                        examples.append(Example(name, tuple(sorted(features)), pending["context"], pending.get("tokio", False), "\n".join(body), str(path), start))
                elif pending is not None:
                    raise ValueError(f"{path}:{start}: maintained fence must be Rust")
                pending = None
                fence = None
                body = []
            else:
                body.append(line)
            continue
        if line.startswith("<!-- slt-check-default:"):
            if line != "<!-- slt-check-default: illustrative fragments require application context -->":
                raise ValueError(f"{path}:{number}: unknown default classification")
            classified_default = True
        elif line.startswith("<!-- slt-check:"):
            if pending is not None:
                raise ValueError(f"{path}:{number}: marker without fence")
            if not line.endswith(" -->"):
                raise ValueError(f"{path}:{number}: malformed marker")
            pending = json.loads(line[len("<!-- slt-check:"):-len(" -->")])
            if set(pending) - {"name", "features", "context", "tokio"}:
                raise ValueError(f"{path}:{number}: unknown marker fields")
            for key in ("name", "features", "context"):
                if key not in pending:
                    raise ValueError(f"{path}:{number}: missing {key}")
        else:
            match = re.fullmatch(r"\s*(`{3,})([^`]*)", line)
            if match:
                fence = (len(match[1]), match[2].strip())
                start = number + 1
            elif pending is not None and line.strip():
                raise ValueError(f"{path}:{number}: marker must immediately precede its fence")
    if fence is not None or pending is not None:
        raise ValueError(f"{path}: unterminated fence or marker")
    return examples, illustrative


def check(args):
    root = Path(__file__).resolve().parents[1]
    paths = args.docs or [root / "docs/COMPLETE_REFERENCE.md"]
    groups = collections.defaultdict(list)
    total = 0
    for path in paths:
        examples, illustrative = extract(Path(path))
        print(f"{path}: {len(examples)} maintained, {illustrative} illustrative Rust fences", flush=True)
        for example in examples:
            groups[(example.features, example.tokio)].append(example)
        total += len(examples)
    if not total:
        raise ValueError("no maintained examples found")
    failures = 0
    with tempfile.TemporaryDirectory(prefix="slt-doc-examples-") as temporary:
        for index, ((features, tokio), examples) in enumerate(groups.items()):
            project = Path(temporary) / str(index)
            source = project / "src"
            source.mkdir(parents=True)
            manifest = (
                '[workspace]\n[package]\nname = "slt_doc_examples"\nversion = "0.0.0"\nedition = "2024"\n'
                '[dependencies]\nslt = { package = "superlighttui", path = ' + json.dumps(str(root))
                + ', default-features = false, features = ' + json.dumps(list(features)) + ' }\n'
            )
            if tokio:
                manifest += 'tokio = { version = "1", features = ["rt-multi-thread", "macros", "signal"] }\n'
            (project / "Cargo.toml").write_text(manifest, encoding="utf-8")
            modules = []
            for example in examples:
                code = "use slt::*;\n" + example.code
                if example.context == "ui":
                    code = "use slt::*;\nfn recipe(ui: &mut Context) {\n" + example.code + "\n}\n"
                (source / (example.name + ".rs")).write_text(code, encoding="utf-8")
                modules.append(f"mod {example.name};")
                print(f"  {example.name}: {example.source}:{example.line}; features={list(features)}", flush=True)
            (source / "lib.rs").write_text("#![allow(dead_code, unused_imports, unused_variables, unused_must_use)]\n" + "\n".join(modules), encoding="utf-8")
            command = ["cargo", "+" + args.toolchain, "check", "--manifest-path", str(project / "Cargo.toml"), "--lib"]
            if args.release:
                command.append("--release")
            env = os.environ.copy()
            env.setdefault("CARGO_TARGET_DIR", str(root / "target/doc-examples"))
            failures += subprocess.run(command, env=env, check=False).returncode != 0
    if failures:
        raise SystemExit(f"{failures} maintained feature group(s) failed compilation")
    print(f"Compiled {total} maintained recipes in {len(groups)} feature groups.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("docs", nargs="*", type=Path)
    parser.add_argument("--toolchain", default="stable")
    parser.add_argument("--release", action="store_true")
    args = parser.parse_args()
    try:
        check(args)
    except (ValueError, KeyError, TypeError) as error:
        raise SystemExit(str(error)) from error


if __name__ == "__main__":
    main()
