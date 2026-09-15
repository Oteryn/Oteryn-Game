from __future__ import annotations

import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from producer import (
    CatalogValidationError,
    build_snapshot,
    verify_snapshot,
)
from test_producer import valid_source


class ProvenanceTests(unittest.TestCase):
    def test_generated_at_is_integrity_protected(self) -> None:
        snapshot = build_snapshot(valid_source())
        snapshot["generated_at"] = "2026-08-22T18:56:00Z"
        with self.assertRaisesRegex(CatalogValidationError, "payload_digest mismatch"):
            verify_snapshot(snapshot)

    def test_generation_time_changes_snapshot_identity(self) -> None:
        first = valid_source()
        second = valid_source()
        second["generated_at"] = "2026-08-22T18:56:00Z"
        self.assertNotEqual(
            build_snapshot(first)["payload_digest"],
            build_snapshot(second)["payload_digest"],
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
