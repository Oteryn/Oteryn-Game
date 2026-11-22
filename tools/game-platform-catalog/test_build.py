from __future__ import annotations

import hashlib
import subprocess
import sys
import tempfile
import unittest
from collections import Counter
from pathlib import Path

from producer import build_snapshot, canonical_json_bytes, load_json_file, verify_snapshot, write_snapshot_files


class PublicViewArtifactTests(unittest.TestCase):
    def test_rust_adapter_input_matches_the_physical_validator(self):
        root = Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "catalog.json"
            subprocess.run(
                [sys.executable, "tools/game-platform-catalog/build.py", "--preproduction", output],
                cwd=root, check=True,
            )
            snapshot = load_json_file(output)
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

    def test_default_build_preserves_published_unsupported_profile(self):
        from build import build

        root = Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "catalog.json"
            build(root, output)
            snapshot = load_json_file(output)
            verify_snapshot(snapshot)
            self.assertEqual(len(snapshot["capability_manifest"]), 8)
            self.assertEqual({row["support"] for row in snapshot["capability_manifest"]}, {"unsupported"})
            self.assertEqual({row["state"] for row in snapshot["completeness_manifest"]}, {"unknown"})
            self.assertEqual([snapshot[key] for key in ("entities", "relations", "tombstones")], [[], [], []])

    def test_build_uses_pinned_projection_and_producer(self):
        from unittest.mock import patch

        from build import build

        root = Path(__file__).resolve().parents[2]
        projection = "crates/gameplay/content/src/content/project/public_view.rs"
        producer = "tools/game-platform-catalog/producer.py"
        expected_ruleset = "oteryn:ruleset.public-catalog-candidate"
        changed_ruleset = "oteryn:ruleset.catalog-provenance-regression"

        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            checkout = directory / "checkout"
            subprocess.run(["git", "worktree", "add", "--quiet", "--detach", checkout, "HEAD"],
                           cwd=root, check=True)
            try:
                git_output = subprocess.check_output

                def git(*args):
                    return git_output(["git", *args], cwd=checkout, text=True).strip()

                before = git("rev-parse", "HEAD")
                projection_path = checkout / projection
                original_projection = projection_path.read_text(encoding="utf-8")
                self.assertIn(expected_ruleset, original_projection)
                projection_path.write_text(original_projection.replace(expected_ruleset, changed_ruleset),
                                           encoding="utf-8")
                producer_path = checkout / producer
                original_producer = producer_path.read_text(encoding="utf-8")
                producer_path.write_text(
                    original_producer.replace(
                        'CONTENT_AUTHORITY_ID = "oteryn-native"',
                        'CONTENT_AUTHORITY_ID = "oteryn-provenance-regression"',
                    ),
                    encoding="utf-8",
                )
                dirty = directory / "dirty.json"
                subprocess.run(
                    [sys.executable, "tools/game-platform-catalog/build.py", dirty],
                    cwd=checkout, check=True, capture_output=True,
                )
                dirty_snapshot = load_json_file(dirty)
                with self.subTest(source="dirty Rust projection"):
                    self.assertEqual(dirty_snapshot["source_revision"], before)
                    self.assertEqual(dirty_snapshot["ruleset_id"], expected_ruleset)
                with self.subTest(source="dirty Python producer"):
                    self.assertEqual(dirty_snapshot["content_authority_id"], "oteryn-native")
                producer_path.write_text(original_producer, encoding="utf-8")

                git("add", projection)
                git("-c", "user.name=Catalogue regression", "-c", "user.email=regression@example.invalid",
                    "commit", "--quiet", "-m", "Change projection metadata")
                after = git("rev-parse", "HEAD")
                git("switch", "--quiet", "--detach", before)

                def switch_after_revision(args, **kwargs):
                    result = git_output(args, **kwargs)
                    if args == ["git", "rev-parse", "HEAD"]:
                        git("switch", "--quiet", "--detach", after)
                    return result

                switched = directory / "switched.json"
                with patch("build.subprocess.check_output", side_effect=switch_after_revision):
                    build(checkout, switched)
                switched_snapshot = load_json_file(switched)
                with self.subTest(source="clean checkout switched after revision capture"):
                    self.assertEqual(git("rev-parse", "HEAD"), after)
                    self.assertEqual(switched_snapshot["source_revision"], before)
                    self.assertEqual(switched_snapshot["ruleset_id"], expected_ruleset)

            finally:
                subprocess.run(["git", "worktree", "remove", "--force", checkout], cwd=root, check=True)

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
