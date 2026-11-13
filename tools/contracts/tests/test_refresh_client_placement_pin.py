"""The pin updater must preserve hash validation and unrelated Rust declarations."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import refresh_client_placement_pin as tool  # noqa: E402


class PlacementPinTests(unittest.TestCase):
    def fixture(self, root):
        region = root / "content/world/placements/region-z07-x000-y000.b3"
        region.parent.mkdir(parents=True)
        region.write_bytes(b"bounded pinned region fixture")
        document = {
            "schema": "OTERYN_FAMILY_INDEX/v1", "family": "WorldPlacement.Base",
            "codec": "OTERYN_WORLD_REGION_B3/v1",
            "palette": [{"source_item_id": 104}],
            "regions": [{"path": region.relative_to(root).as_posix(),
                         "sha256": hashlib.sha256(region.read_bytes()).hexdigest()}],
        }
        index = root / tool.INDEX
        index.write_text(json.dumps(document))
        consumer = root / tool.CONSUMER
        consumer.parent.mkdir(parents=True)
        consumer.write_text('pub const MANIFEST_SHA256: &str = "unchanged";\n'
                            'pub const PLACEMENT_INDEX_SHA256: &str =\n    "' + '0' * 64 + '";\n')
        return region, index, consumer

    def test_check_refuses_stale_then_refresh_changes_only_derived_pin(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            _, index, consumer = self.fixture(root)
            before = consumer.read_text()
            with self.assertRaisesRegex(ValueError, "stale client placement pin"):
                tool.refresh(root, check=True)
            self.assertEqual(consumer.read_text(), before)
            digest = hashlib.sha256(index.read_bytes()).hexdigest()
            self.assertEqual(tool.refresh(root), digest)
            self.assertEqual(consumer.read_text(), before.replace('0' * 64, digest))
            self.assertEqual(tool.refresh(root, check=True), digest)

    def test_region_tamper_cannot_be_blessed_by_refresh(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            region, _, consumer = self.fixture(root)
            before = consumer.read_text()
            region.write_bytes(b"substituted region")
            with self.assertRaisesRegex(ValueError, "region SHA mismatch"):
                tool.refresh(root)
            self.assertEqual(consumer.read_text(), before)

    def test_invalid_index_or_ambiguous_consumer_never_changes_pin(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            _, index, consumer = self.fixture(root)
            before = consumer.read_text()
            document = json.loads(index.read_text())
            document["regions"][0]["path"] = "content/world/placements/../outside.b3"
            index.write_text(json.dumps(document))
            with self.assertRaisesRegex(ValueError, "invalid or duplicate pinned region"):
                tool.refresh(root)
            self.assertEqual(consumer.read_text(), before)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            _, _, consumer = self.fixture(root)
            consumer.write_text(consumer.read_text() * 2)
            before = consumer.read_text()
            with self.assertRaisesRegex(ValueError, "exactly one"):
                tool.refresh(root)
            self.assertEqual(consumer.read_text(), before)


if __name__ == "__main__":
    unittest.main()
