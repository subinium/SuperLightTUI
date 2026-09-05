"""Offline checks for native smoke generation, provenance and nonempty execution."""

import contextlib
import io
import json
from pathlib import Path
import subprocess
import tomllib
import unittest
from unittest.mock import patch

import smoke_native_release as smoke


class NativeSmokeTests(unittest.TestCase):
    def execute(self, *, source="registry+https://github.com/rust-lang/crates.io-index", count=6):
        calls = []

        def command(argv, *, cwd, **_):
            calls.append(argv)
            manifest = tomllib.loads((Path(cwd) / "Cargo.toml").read_text())
            self.assertEqual(manifest["dependencies"]["superlighttui"], {
                "version": "=0.24.0", "features": ["async"],
            })
            self.assertEqual(manifest["features"]["default"], ["crossterm", "async"])
            self.assertTrue((Path(cwd) / "tests/v024_burst.rs").is_file())
            self.assertTrue((Path(cwd) / "tests/support/burst_pty.py").is_file())
            if argv[1] == "metadata":
                return subprocess.CompletedProcess(argv, 0, stdout=json.dumps({"packages": [{
                    "name": "superlighttui", "version": "0.24.0", "source": source,
                    "manifest_path": str(Path(cwd) / "Cargo.toml"),
                }]}))
            self.assertEqual(argv[1:5], ["test", "--locked", "--test", "v024_burst"])
            return subprocess.CompletedProcess(argv, 0, stdout=f"test result: ok. {count} passed; 0 failed; 0 ignored\n", stderr="")

        with patch("sys.argv", ["smoke_native_release.py", "0.24.0"]), \
                patch.object(smoke, "verify_registry") as registry, \
                patch.object(smoke.subprocess, "run", side_effect=command), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            smoke.main()
            registry.assert_called_once_with("0.24.0", 60)
        return calls

    def test_exact_consumer_enables_and_runs_the_native_suite(self):
        self.assertEqual(len(self.execute()), 2)

    def test_empty_or_incomplete_suite_is_not_a_pass(self):
        for count in [0, 1, 5]:
            with self.subTest(count=count), self.assertRaisesRegex(RuntimeError, "not all executed"):
                self.execute(count=count)

    def test_path_or_git_override_is_rejected(self):
        for source in [None, "git+https://example.test/repository"]:
            with self.subTest(source=source), self.assertRaisesRegex(RuntimeError, "override"):
                self.execute(source=source)


if __name__ == "__main__":
    unittest.main()
