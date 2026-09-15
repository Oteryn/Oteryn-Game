from __future__ import annotations

import copy
import hashlib
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from producer import (
    CatalogValidationError,
    build_snapshot,
    canonical_json_bytes,
    verify_snapshot,
)


def valid_source() -> dict:
    return {
        "authority_epoch": "native-epoch-1",
        "source_revision": "a" * 40,
        "generated_at": "2026-08-22T17:55:00Z",
        "ruleset_id": "oteryn:ruleset.reference_v1",
        "content_profile_id": "oteryn:profile.reference_v1",
        "required_capabilities": ["item"],
        "capability_manifest": [
            {"capability_id": "item", "support": "supported"},
            {"capability_id": "npc", "support": "unsupported"},
        ],
        "completeness_manifest": [
            {"capability_id": "item", "state": "complete"},
            {"capability_id": "npc", "state": "unknown"},
        ],
        "entities": [
            {
                "type": "item",
                "content_key": "oteryn:item.training_sword",
                "capability_id": "item",
                "data": {"name": "Training Sword", "attack": 7},
            }
        ],
        "relations": [],
        "tombstones": [],
    }


class ProducerTests(unittest.TestCase):
    def test_determinism_ignores_input_collection_order(self) -> None:
        first = valid_source()
        second = copy.deepcopy(first)
        second["capability_manifest"].reverse()
        second["completeness_manifest"].reverse()
        second["entities"][0]["data"] = {"attack": 7, "name": "Training Sword"}
        a = build_snapshot(first)
        b = build_snapshot(second)
        self.assertEqual(a, b)
        self.assertEqual(canonical_json_bytes(a), canonical_json_bytes(b))

    def test_payload_digest_is_independently_verifiable(self) -> None:
        snapshot = build_snapshot(valid_source())
        semantic = {
            key: value
            for key, value in snapshot.items()
            if key not in {"snapshot_id", "payload_digest"}
        }
        expected = hashlib.sha256(canonical_json_bytes(semantic)).hexdigest()
        self.assertEqual(snapshot["payload_digest"], f"sha256:{expected}")
        self.assertEqual(snapshot["snapshot_id"], f"sha256:{expected}")
        verify_snapshot(snapshot)

    def test_duplicate_entity_identity_fails_closed(self) -> None:
        source = valid_source()
        source["entities"].append(copy.deepcopy(source["entities"][0]))
        with self.assertRaisesRegex(CatalogValidationError, "duplicate entity"):
            build_snapshot(source)

    def test_dangling_relation_fails_closed(self) -> None:
        source = valid_source()
        source["relations"] = [
            {
                "type": "item_upgrade",
                "relation_key": "oteryn:relation.training_upgrade",
                "capability_id": "item",
                "source": "oteryn:item.training_sword",
                "target": "oteryn:item.missing",
                "data": {},
            }
        ]
        with self.assertRaisesRegex(CatalogValidationError, "dangling relation target"):
            build_snapshot(source)

    def test_tombstone_requires_complete_capability(self) -> None:
        source = valid_source()
        source["completeness_manifest"][0]["state"] = "partial"
        source["tombstones"] = [
            {
                "content_key": "oteryn:item.retired_blade",
                "capability_id": "item",
                "reason": "removed",
            }
        ]
        with self.assertRaisesRegex(
            CatalogValidationError, "tombstone requires complete"
        ):
            build_snapshot(source)

    def test_contradictory_tombstone_fails_closed(self) -> None:
        source = valid_source()
        source["tombstones"] = [
            {
                "content_key": "oteryn:item.training_sword",
                "capability_id": "item",
                "reason": "removed",
            }
        ]
        with self.assertRaisesRegex(CatalogValidationError, "contradictory tombstone"):
            build_snapshot(source)

    def test_unsupported_required_capability_fails_closed(self) -> None:
        source = valid_source()
        source["required_capabilities"] = ["npc"]
        with self.assertRaisesRegex(
            CatalogValidationError, "required capability.*unsupported"
        ):
            build_snapshot(source)

    def test_numeric_legacy_identity_is_rejected(self) -> None:
        source = valid_source()
        source["entities"][0]["content_key"] = "2516"
        with self.assertRaisesRegex(CatalogValidationError, "content_key"):
            build_snapshot(source)

    def test_source_revision_must_be_exact_lowercase_git_sha(self) -> None:
        source = valid_source()
        source["source_revision"] = "ABC123"
        with self.assertRaisesRegex(CatalogValidationError, "source_revision"):
            build_snapshot(source)

    def test_oversized_text_is_rejected(self) -> None:
        source = valid_source()
        source["entities"][0]["data"]["name"] = "x" * 2049
        with self.assertRaisesRegex(CatalogValidationError, "UTF-8 string limit"):
            build_snapshot(source)

    def test_too_deep_payload_is_rejected(self) -> None:
        source = valid_source()
        nested: dict = {"leaf": "ok"}
        for _ in range(20):
            nested = {"next": nested}
        source["entities"][0]["data"] = nested
        with self.assertRaisesRegex(CatalogValidationError, "nesting depth"):
            build_snapshot(source)

    def test_modified_snapshot_digest_is_rejected(self) -> None:
        snapshot = build_snapshot(valid_source())
        snapshot["entities"][0]["data"]["attack"] = 999
        with self.assertRaisesRegex(CatalogValidationError, "payload_digest mismatch"):
            verify_snapshot(snapshot)


if __name__ == "__main__":
    unittest.main(verbosity=2)
