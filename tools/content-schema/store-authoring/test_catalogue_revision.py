import json
from pathlib import Path
import unittest

from catalogue_revision import revision


class CatalogueRevisionTest(unittest.TestCase):
    def test_rust_fixture_revision(self):
        root = Path(__file__).parent / "samples"
        document = json.loads((root / "revision-utf8.json").read_text(encoding="utf-8"))
        self.assertEqual(revision(document), (root / "revision-utf8.sha256").read_text().strip())

    def test_rejects_floats_recursively(self):
        for number in (1.0, 1.5, float("inf"), float("nan")):
            with self.assertRaises(ValueError):
                revision({"nested": [{"number": number}]})
