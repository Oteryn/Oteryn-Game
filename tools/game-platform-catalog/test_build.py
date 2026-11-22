from __future__ import annotations

import hashlib
import subprocess
import tempfile
import unittest
from collections import Counter
from pathlib import Path

from producer import build_snapshot, canonical_json_bytes, load_json_file, verify_snapshot, write_snapshot_files


class PublicViewArtifactTests(unittest.TestCase):
    def test_rust_adapter_input_matches_the_physical_validator(self):
        root = Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary) / "source.json"
            with source.open("wb") as output:
                subprocess.run(
                    ["cargo", "run", "--locked", "--quiet", "-p", "oteryn-gameplay-content",
                     "--bin", "oteryn-game-platform-catalog", "--", "content/world",
                     "d34807f5bd234ee8318e57ba454ba39be0df113d", "2026-11-07T00:00:00Z"],
                    cwd=root, stdout=output, check=True,
                )
            snapshot = build_snapshot(load_json_file(source))
            verify_snapshot(snapshot)
            support = {row["capability_id"]: row["support"] for row in snapshot["capability_manifest"]}
            completeness = {row["capability_id"]: row["state"] for row in snapshot["completeness_manifest"]}
            records = snapshot["entities"] + snapshot["relations"]
            counts = Counter(row["capability_id"] for row in records)
            self.assertEqual(set(counts), {"item", "creature", "npc", "npc_shop", "creature_loot"})
            self.assertEqual(len(support), 8)
            self.assertEqual({cap for cap in support if support[cap] == "supported"}, set(counts))
            self.assertEqual(completeness["item"], "complete")
            self.assertEqual(completeness["creature_loot"], "partial")
            self.assertTrue(all(completeness[cap] == "unknown" for cap in support.keys() - counts.keys()))
            self.assertTrue(all(row["type"] == row["capability_id"] for row in records))
            self.assertTrue(all(row["data"]["payload_revision"] == 1 for row in records))
            item = next(row for row in snapshot["entities"] if row["content_key"] == "oteryn:item.tibia.i1000")
            self.assertEqual(item["data"], {
                "payload_revision": 1, "definition_revision": "definition-r1",
                "materializable": False, "stack_class": "Unknown", "name": "dry grass",
            })
            self.assertEqual(snapshot["tombstones"], [])

    def test_same_content_produces_identical_bytes_and_transport_digest(self):
        source = load_json_file(Path(__file__).parent / "fixtures/unsupported-native-input.json")
        with tempfile.TemporaryDirectory() as temporary:
            first = Path(temporary) / "first.json"
            second = Path(temporary) / "second.json"
            for output in (first, second):
                write_snapshot_files(build_snapshot(source), output)
            self.assertEqual(first.read_bytes(), second.read_bytes())
            digest = hashlib.sha256(first.read_bytes()).hexdigest()
            self.assertTrue(first.with_suffix(".json.sha256").read_text().startswith(digest))
            self.assertEqual(
                canonical_json_bytes(build_snapshot(source)) + b"\n", first.read_bytes()
            )


if __name__ == "__main__":
    unittest.main()
