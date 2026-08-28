from __future__ import annotations

import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from producer import CatalogValidationError, build_snapshot
from test_producer import valid_source


class TaxonomyTests(unittest.TestCase):
    def test_unknown_v1_capability_is_rejected(self) -> None:
        source = valid_source()
        source["capability_manifest"].append(
            {"capability_id": "made_up", "support": "supported"}
        )
        source["completeness_manifest"].append(
            {"capability_id": "made_up", "state": "complete"}
        )
        with self.assertRaisesRegex(CatalogValidationError, "unknown v1 capability"):
            build_snapshot(source)


if __name__ == "__main__":
    unittest.main(verbosity=2)
