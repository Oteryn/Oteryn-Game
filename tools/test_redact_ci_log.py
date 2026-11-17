#!/usr/bin/env python3

from pathlib import Path
import importlib.util
import tempfile
import unittest


MODULE = Path(__file__).with_name("redact_ci_log.py")
SPEC = importlib.util.spec_from_file_location("redact_ci_log", MODULE)
assert SPEC is not None and SPEC.loader is not None
redact_ci_log = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(redact_ci_log)
redact = redact_ci_log.redact


class RedactCiLogTests(unittest.TestCase):
    def test_drops_sensitive_lines_case_insensitively(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source.log"
            destination = root / "redacted.log"
            source.write_text(
                "safe first line\n"
                "PASSWORD=secret\n"
                "database=postgres://user:secret@localhost/db\n"
                "request token=secret\n"
                "GRANT SELECT ON table TO role\n"
                "safe final line\n",
                encoding="utf-8",
            )

            redact(source, destination)

            self.assertEqual(
                destination.read_text(encoding="utf-8"),
                "safe first line\nsafe final line\n",
            )

    def test_replaces_invalid_utf8(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source.log"
            destination = root / "redacted.log"
            source.write_bytes(b"diagnostic: \xff\n")

            redact(source, destination)

            self.assertEqual(destination.read_text(encoding="utf-8"), "diagnostic: �\n")


if __name__ == "__main__":
    unittest.main()
