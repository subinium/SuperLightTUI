#!/usr/bin/env python3
"""Parser classification tests; --compile-negative verifies rustc rejects drift."""

import argparse
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from check_doc_examples import (
    ROOT,
    default_allowed,
    default_paths,
    extract,
    render_module,
)

DEFAULT = (
    "<!-- slt-check-default: illustrative fragments require application context -->\n"
)


class ClassificationTests(unittest.TestCase):
    def extract_text(self, text, allow_default=True):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "fixture.md"
            path.write_text(text, encoding="utf-8")
            return extract(path, allow_default=allow_default)

    def test_rejects_unclassified_rust(self):
        with self.assertRaisesRegex(ValueError, "lacks classification"):
            self.extract_text("```rust\nlet x = 1;\n```\n")

    def test_rejects_unclassified_rust_ignore(self):
        with self.assertRaisesRegex(ValueError, "lacks classification"):
            self.extract_text("```rust,ignore\nlet x = 1;\n```\n")

    def test_non_rust_fences_need_no_marker(self):
        examples, count = self.extract_text(
            "```toml\n[dependencies]\n```\n```text\nx\n```\n"
        )
        self.assertEqual((examples, count), ([], 0))

    def test_skip_marker_requires_reason(self):
        with self.assertRaisesRegex(ValueError, "needs a reason"):
            self.extract_text("<!-- slt-check-skip: -->\n```rust\nx\n```\n")
        examples, count = self.extract_text(
            "<!-- slt-check-skip: ratatui code shown for comparison -->\n```rust\nframe.render_widget(w, area);\n```\n"
        )
        self.assertEqual((examples, count), ([], 1))

    def test_skip_marker_must_precede_rust(self):
        with self.assertRaisesRegex(ValueError, "must precede a Rust fence"):
            self.extract_text(
                "<!-- slt-check-skip: not rust at all here -->\n```toml\nx = 1\n```\n"
            )

    def test_unknown_marker_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "unknown slt-check marker"):
            self.extract_text("<!-- slt-check-ignore: why -->\n```rust\nx\n```\n")

    def test_maintained_fence_must_be_plain_rust(self):
        with self.assertRaisesRegex(ValueError, "plain"):
            self.extract_text(
                '<!-- slt-check: {"name":"x", "features":[], "context":"ui"} -->\n```rust,ignore\nui.text("x");\n```\n'
            )

    def test_default_is_explicit_and_not_applied_to_maintained_fence(self):
        examples, count = self.extract_text(
            DEFAULT
            + "```rust\napplication_specific();\n```\n"
            + '<!-- slt-check: {"name":"example", "features":[], "context":"ui"} -->\n'
            + '```rust\nui.text("hello");\n```\n'
        )
        self.assertEqual(count, 1)
        self.assertEqual(examples[0].name, "example")
        self.assertEqual(examples[0].code, 'ui.text("hello");')

    def test_default_is_rejected_on_user_facing_pages(self):
        with self.assertRaisesRegex(ValueError, "only allowed under"):
            self.extract_text(DEFAULT + "```rust\nx\n```\n", allow_default=False)
        self.assertFalse(default_allowed(ROOT / "docs/guide/01-quick-start.md"))
        self.assertFalse(default_allowed(ROOT / "README.md"))
        self.assertTrue(default_allowed(ROOT / "docs/internals/architecture.md"))
        self.assertTrue(default_allowed(ROOT / "docs/archive/old.md"))

    def test_default_paths_cover_readme_docs_and_skills(self):
        paths = {path.relative_to(ROOT).as_posix() for path in default_paths()}
        self.assertIn("README.md", paths)
        self.assertIn("docs/guide/01-quick-start.md", paths)
        self.assertIn("docs/reference/cookbook.md", paths)
        self.assertIn(".agents/skills/slt/SKILL.md", paths)
        self.assertIn(".claude/skills/slt/SKILL.md", paths)

    def test_setup_is_compiled_before_the_body(self):
        examples, _ = self.extract_text(
            '<!-- slt-check: {"name":"with_setup", "features":[], "context":"ui", "setup":"let n = 1;"} -->\n'
            '```rust\nui.text(format!("{n}"));\n```\n'
        )
        module = render_module(examples[0])
        self.assertLess(
            module.index("let n = 1;"), module.index('ui.text(format!("{n}"));')
        )

    def test_rejects_duplicate_names(self):
        example = '<!-- slt-check: {"name":"same", "features":[], "context":"ui"} -->\n```rust\nui.text("x");\n```\n'
        with self.assertRaisesRegex(ValueError, "duplicate"):
            self.extract_text(example * 2)

    def test_rejects_unterminated_fence(self):
        with self.assertRaisesRegex(ValueError, "unterminated"):
            self.extract_text("```rust\n")


def expect_rejected(toolchain, fixture, needles, description):
    result = subprocess.run(
        [
            sys.executable,
            str(ROOT / "scripts/check_doc_examples.py"),
            "--toolchain",
            toolchain,
            str(ROOT / fixture),
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    output = result.stdout + result.stderr
    if result.returncode == 0 or not all(needle in output for needle in needles):
        raise SystemExit(
            f"negative contract failed: {description} was not rejected:\n{output}"
        )
    print(f"Negative compile contract: {description} rejected.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compile-negative", action="store_true")
    parser.add_argument("--toolchain", default="stable")
    args = parser.parse_args()
    result = unittest.TextTestRunner().run(
        unittest.defaultTestLoader.loadTestsFromTestCase(ClassificationTests)
    )
    if not result.wasSuccessful():
        raise SystemExit(1)
    if args.compile_negative:
        expect_rejected(
            args.toolchain,
            "tests/fixtures/doc_examples/invalid.md",
            ["x_range", "E0599"],
            "invalid ChartBuilder.x_range (E0599)",
        )
        expect_rejected(
            args.toolchain,
            "tests/fixtures/doc_examples/must_use.md",
            ["unused_must_use", "Button"],
            "discarded #[must_use] Button builder (unused_must_use denied)",
        )
