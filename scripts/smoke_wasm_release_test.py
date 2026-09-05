"""Offline checks for the exact-version release verification gates."""
import json
import unittest
from unittest.mock import patch
import urllib.error

import smoke_wasm_release as smoke


class Response:
    def __init__(self, url, text):
        self.url, self.text, self.status = url, text, 200

    def __enter__(self):
        return self

    def __exit__(self, *_):
        return False

    def read(self):
        return self.text.encode("utf-8")


class VerificationTests(unittest.TestCase):
    def test_registry_requires_both_exact_unyanked_versions(self):
        def fetch(request, **_):
            return Response(request.full_url, json.dumps({"vers": "0.24.0", "yanked": False}))
        with patch.object(smoke.urllib.request, "urlopen", side_effect=fetch) as fetch:
            smoke.verify_registry("0.24.0", 1)
            self.assertEqual(fetch.call_count, 2)

    def test_registry_does_not_accept_another_version(self):
        with patch.object(smoke.urllib.request, "urlopen", return_value=Response("", '{"vers":"0.23.0","yanked":false}')):
            with self.assertRaisesRegex(RuntimeError, "not ready"):
                smoke.verify_registry("0.24.0", 1)

    def test_registry_rejects_yanked_version(self):
        with patch.object(smoke.urllib.request, "urlopen", return_value=Response("", '{"vers":"0.24.0","yanked":true}')):
            with self.assertRaisesRegex(RuntimeError, "yanked"):
                smoke.verify_registry("0.24.0", 1)

    def test_network_failure_is_not_a_success(self):
        with patch.object(smoke.urllib.request, "urlopen", side_effect=urllib.error.URLError("offline")):
            with self.assertRaises(RuntimeError):
                smoke.verify_registry("0.24.0", 1)

    def test_docs_require_exact_url_and_rustdoc_title(self):
        def fetch(request, **_):
            library = "slt_wasm" if "slt-wasm" in request.full_url else "slt"
            return Response(request.full_url, f"<title>{library} - Rust</title>")
        with patch.object(smoke.urllib.request, "urlopen", side_effect=fetch):
            smoke.verify_docs("0.24.0", 1)

    def test_docs_do_not_accept_latest_redirect_or_error_page(self):
        for url, body in [("https://docs.rs/superlighttui/latest/slt/", "<title>slt - Rust</title>"),
                          ("https://docs.rs/superlighttui/0.24.0/slt/", "not built")]:
            with self.subTest(url=url, body=body):
                with patch.object(smoke.urllib.request, "urlopen", return_value=Response(url, body)):
                    with self.assertRaises(RuntimeError):
                        smoke.verify_docs("0.24.0", 1)


if __name__ == "__main__":
    unittest.main()
