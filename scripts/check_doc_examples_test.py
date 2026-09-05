#!/usr/bin/env python3
"""Parser classification tests; --compile-negative verifies rustc rejects drift."""

import argparse
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from check_doc_examples import extract


class ClassificationTests(unittest.TestCase):
    def extract_text(self, text):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "fixture.md"
            path.write_text(text, encoding="utf-8")
            return extract(path)

    def test_rejects_unclassified_rust(self):
        with self.assertRaisesRegex(ValueError, "lacks classification"):
            self.extract_text("```rust\nlet x = 1;\n```\n")

    def test_default_is_explicit_and_not_applied_to_maintained_fence(self):
        examples, count = self.extract_text(
            '<!-- slt-check-default: illustrative fragments require application context -->\n'
            '```rust\napplication_specific();\n```\n'
            '<!-- slt-check: {"name":"example", "features":[], "context":"ui"} -->\n'
            '```rust\nui.text("hello");\n```\n'
        )
        self.assertEqual(count, 1)
        self.assertEqual(examples[0].name, "example")
        self.assertEqual(examples[0].code, 'ui.text("hello");')

    def test_rejects_duplicate_names(self):
        example = '<!-- slt-check: {"name":"same", "features":[], "context":"ui"} -->\n```rust\nui.text("x");\n```\n'
        with self.assertRaisesRegex(ValueError, "duplicate"):
            self.extract_text(example * 2)

    def test_rejects_unterminated_fence(self):
        with self.assertRaisesRegex(ValueError, "unterminated"):
            self.extract_text("```rust\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compile-negative", action="store_true")
    parser.add_argument("--toolchain", default="stable")
    args = parser.parse_args()
    result = unittest.TextTestRunner().run(unittest.defaultTestLoader.loadTestsFromTestCase(ClassificationTests))
    if not result.wasSuccessful():
        raise SystemExit(1)
    if args.compile_negative:
        root = Path(__file__).resolve().parents[1]
        result = subprocess.run([sys.executable, str(root / "scripts/check_doc_examples.py"), "--toolchain", args.toolchain, str(root / "tests/fixtures/doc_examples/invalid.md")], capture_output=True, text=True, check=False)
        if result.returncode == 0 or "x_range" not in result.stderr or "E0599" not in result.stderr:
            raise SystemExit("negative contract failed to produce the expected missing-method compiler diagnostic:\n" + result.stdout + result.stderr)
        print("Negative compile contract: invalid ChartBuilder.x_range rejected (E0599).")
