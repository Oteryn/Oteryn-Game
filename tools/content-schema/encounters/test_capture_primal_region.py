"""Reject substituted donor bytes before any source tile can be captured."""
import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location("capture_primal_region", Path(__file__).with_name("capture_primal_region.py"))
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class SourceCustody(unittest.TestCase):
    def test_rejects_substituted_item_sources_before_parsing(self):
        with self.assertRaisesRegex(ValueError, "pinned Crystal appearances blob mismatch"):
            MODULE.attach_item_semantics({"tiles": []}, b"substitute", b"substitute")

    def test_rejects_substituted_map_before_parsing(self):
        for raw in (b"", b"OTBM", b"\x00\x00\x00\x00\xfe"):
            with self.subTest(raw=raw), self.assertRaisesRegex(ValueError, "pinned Crystal map blob mismatch"):
                MODULE.capture(raw)


if __name__ == "__main__":
    unittest.main()
