#!/usr/bin/env python3
"""Classify every Rust fence in user-facing Markdown and compile the maintained ones.

Every fence whose info string starts with ``rust`` must be preceded by exactly
one marker line:

* ``<!-- slt-check: {"name": ..., "features": [...], "context": "module"|"ui"} -->``
  marks a maintained recipe. It is compiled against the local crate with
  ``#![deny(warnings)]`` (only ``dead_code`` is allowed, because recipe helpers
  may be unused), so a discarded ``#[must_use]`` value, an unused variable or
  import, or a deprecated call fails the check. ``"context": "ui"`` wraps the
  body in ``fn recipe(ui: &mut Context)``; ``"module"`` compiles it as items.
  An optional ``"setup"`` string is compiled before the body to declare the
  app variables a fragment refers to; keep it short, since readers copy only
  the fence itself.
* ``<!-- slt-check-skip: <reason> -->`` marks an illustrative fragment. The
  reason is required and must say why the fence cannot compile on its own.

Contributor-only pages under ``docs/internals/`` and ``docs/archive/`` may
instead declare one file-level default
(``<!-- slt-check-default: illustrative fragments require application context -->``);
user-facing pages may not.

With no arguments the checker covers ``README.md``, ``docs/**/*.md`` and both
agent skill trees.
"""

import argparse
import collections
import dataclasses
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parents[1]

DEFAULT_MARKER = (
    "<!-- slt-check-default: illustrative fragments require application context -->"
)
# Repository-relative prefixes whose pages may use the file-level default.
DEFAULT_ALLOWED_PREFIXES = ("docs/internals/", "docs/archive/")

# The generated crate denies every warning. Recipe-level helpers (an unused
# `fn main`, a struct only shown for its shape) are the one allowed lint.
LIB_PRELUDE = "#![deny(warnings)]\n#![allow(dead_code)]\n"
MIN_SKIP_REASON = 8


@dataclasses.dataclass
class Example:
    name: str
    features: tuple[str, ...]
    context: str
    tokio: bool
    code: str
    setup: str
    source: str
    line: int


def default_paths(root=ROOT):
    """Return every Markdown file the checker covers when given no arguments."""
    paths = [root / "README.md"]
    for pattern_root in ("docs", ".agents/skills", ".claude/skills"):
        base = root / pattern_root
        if base.is_dir():
            paths.extend(sorted(base.rglob("*.md")))
    return paths


def default_allowed(path, root=ROOT):
    """Whether `path` may use the file-level illustrative default."""
    try:
        relative = Path(path).resolve().relative_to(root).as_posix()
    except ValueError:
        return True  # ad-hoc fixtures outside the repository
    return relative.startswith(DEFAULT_ALLOWED_PREFIXES)


def is_rust(info):
    return info.split(",", 1)[0].strip() == "rust"


def extract(path, allow_default=True):
    """Parse one Markdown file into (maintained examples, illustrative count)."""
    lines = Path(path).read_text(encoding="utf-8").splitlines()
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
                if is_rust(fence[1]):
                    if pending is None:
                        if not classified_default:
                            raise ValueError(
                                f"{path}:{start}: Rust fence lacks classification "
                                "(add <!-- slt-check: {...} --> or <!-- slt-check-skip: reason -->)"
                            )
                        illustrative += 1
                    elif pending == "skip":
                        illustrative += 1
                    else:
                        if fence[1] != "rust":
                            raise ValueError(
                                f"{path}:{start}: maintained fence must be plain ```rust"
                            )
                        name = pending["name"]
                        if not re.fullmatch(r"[a-z][a-z0-9_]*", name) or name in names:
                            raise ValueError(
                                f"{path}:{start}: invalid/duplicate name: {name}"
                            )
                        names.add(name)
                        features = pending["features"]
                        if not isinstance(features, list) or not all(
                            isinstance(f, str) for f in features
                        ):
                            raise ValueError(
                                f"{path}:{start}: features must be a string list"
                            )
                        if pending["context"] not in ("module", "ui"):
                            raise ValueError(
                                f"{path}:{start}: context must be module or ui"
                            )
                        setup = pending.get("setup", "")
                        if not isinstance(setup, str):
                            raise ValueError(f"{path}:{start}: setup must be a string")
                        examples.append(
                            Example(
                                name,
                                tuple(sorted(features)),
                                pending["context"],
                                pending.get("tokio", False),
                                "\n".join(body),
                                setup,
                                str(path),
                                start,
                            )
                        )
                elif pending is not None:
                    raise ValueError(
                        f"{path}:{start}: slt-check marker must precede a Rust fence"
                    )
                pending = None
                fence = None
                body = []
            else:
                body.append(line)
            continue
        stripped = line.strip()
        if stripped.startswith("<!-- slt-check-default:"):
            if stripped != DEFAULT_MARKER:
                raise ValueError(f"{path}:{number}: unknown default classification")
            if not allow_default:
                raise ValueError(
                    f"{path}:{number}: file-level illustrative default is only allowed under "
                    + ", ".join(DEFAULT_ALLOWED_PREFIXES)
                    + "; classify each fence"
                )
            classified_default = True
        elif stripped.startswith("<!-- slt-check-skip:"):
            if pending is not None:
                raise ValueError(f"{path}:{number}: marker without fence")
            if not stripped.endswith("-->"):
                raise ValueError(f"{path}:{number}: malformed marker")
            reason = stripped[len("<!-- slt-check-skip:") : -len("-->")].strip()
            if len(reason) < MIN_SKIP_REASON:
                raise ValueError(f"{path}:{number}: slt-check-skip needs a reason")
            pending = "skip"
        elif stripped.startswith("<!-- slt-check:"):
            if pending is not None:
                raise ValueError(f"{path}:{number}: marker without fence")
            if not stripped.endswith(" -->"):
                raise ValueError(f"{path}:{number}: malformed marker")
            pending = json.loads(stripped[len("<!-- slt-check:") : -len(" -->")])
            if set(pending) - {"name", "features", "context", "tokio", "setup"}:
                raise ValueError(f"{path}:{number}: unknown marker fields")
            for key in ("name", "features", "context"):
                if key not in pending:
                    raise ValueError(f"{path}:{number}: missing {key}")
        elif stripped.startswith("<!-- slt-check"):
            raise ValueError(f"{path}:{number}: unknown slt-check marker")
        else:
            match = re.fullmatch(r"\s*(`{3,})([^`]*)", line)
            if match:
                fence = (len(match[1]), match[2].strip())
                start = number + 1
            elif pending is not None and stripped:
                raise ValueError(
                    f"{path}:{number}: marker must immediately precede its fence"
                )
    if fence is not None or pending is not None:
        raise ValueError(f"{path}: unterminated fence or marker")
    return examples, illustrative


def module_slug(path, root=ROOT):
    try:
        relative = Path(path).resolve().relative_to(root).as_posix()
    except ValueError:
        relative = Path(path).name
    return re.sub(r"[^a-z0-9]+", "_", relative.lower().removesuffix(".md")).strip("_")


def render_module(example):
    """Wrap one recipe in a compilable module body."""
    setup = example.setup + "\n" if example.setup else ""
    if example.context == "ui":
        return (
            "use slt::*;\nfn recipe(ui: &mut Context) {\n"
            + setup
            + example.code
            + "\n}\n"
        )
    if re.search(r"^\s*use slt\b", example.code, re.M):
        return setup + example.code + "\n"
    # Harness convenience import; the recipe's own imports are still linted.
    return "#[allow(unused_imports)]\nuse slt::*;\n" + setup + example.code + "\n"


def check(args):
    paths = args.docs or default_paths()
    groups = collections.defaultdict(list)
    seen = {}
    total = 0
    for path in paths:
        path = Path(path)
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        allow = default_allowed(path)
        examples, illustrative = extract(path, allow_default=allow)
        if digest in seen and examples:
            print(
                f"{path}: identical to {seen[digest]}, classification checked, compiled once",
                flush=True,
            )
            continue
        seen.setdefault(digest, path)
        if examples or illustrative:
            print(
                f"{path}: {len(examples)} maintained, {illustrative} illustrative Rust fences",
                flush=True,
            )
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
                '[dependencies]\nslt = { package = "superlighttui", path = '
                + json.dumps(str(ROOT))
                + ", default-features = false, features = "
                + json.dumps(list(features))
                + " }\n"
            )
            if tokio:
                manifest += 'tokio = { version = "1", features = ["rt-multi-thread", "macros", "signal"] }\n'
            (project / "Cargo.toml").write_text(manifest, encoding="utf-8")
            # Resolve against the repository lockfile so every run (and CI)
            # checks recipes with the same dependency versions as the crate.
            if (ROOT / "Cargo.lock").is_file():
                shutil.copyfile(ROOT / "Cargo.lock", project / "Cargo.lock")
            modules = []
            modules_seen = set()
            for example in examples:
                module = f"{module_slug(example.source)}_{example.name}"
                if module in modules_seen:
                    raise ValueError(
                        f"{example.source}:{example.line}: module name collision: {module}"
                    )
                modules_seen.add(module)
                (source / (module + ".rs")).write_text(
                    render_module(example), encoding="utf-8"
                )
                modules.append(f"mod {module};")
                print(
                    f"  {module}: {example.source}:{example.line}; features={list(features)}",
                    flush=True,
                )
            (source / "lib.rs").write_text(
                LIB_PRELUDE + "\n".join(modules) + "\n", encoding="utf-8"
            )
            command = [
                "cargo",
                "+" + args.toolchain,
                "check",
                "--manifest-path",
                str(project / "Cargo.toml"),
                # The lib's test target: compiles `#[test]` fns in recipes too,
                # so test snippets keep their attribute and their imports count.
                "--tests",
            ]
            if args.release:
                command.append("--release")
            env = os.environ.copy()
            env.setdefault("CARGO_TARGET_DIR", str(ROOT / "target/doc-examples"))
            failures += subprocess.run(command, env=env, check=False).returncode != 0
    if failures:
        raise SystemExit(f"{failures} maintained feature group(s) failed compilation")
    print(f"Compiled {total} maintained recipes in {len(groups)} feature groups.")


def main():
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
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
