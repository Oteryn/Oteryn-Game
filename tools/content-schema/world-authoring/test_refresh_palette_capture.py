"""Merged capture summaries are recounted from hash-checked admitted regions."""

import hashlib
import json
import tempfile
import unittest
from pathlib import Path

import refresh_palette_capture as capture


class RefreshCapture(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(dir=capture.base.ROOT / ".lane")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.xml = self.root / "items.xml"
        self.xml.write_bytes(b'<items><item id="100" name="ground" /></items>')
        region = "content/world/placements/region-z07-x000-y000.b3"
        tiles = [(1, 2, 0, 0, (), [(0, 0, {"action": 123})])]
        raw = capture.codec.encode_region(
            7,
            0,
            0,
            {
                0: capture.codec.encode_sector_tiles(0, 0, tiles),
            },
        )
        self.index = {
            "source": {
                "files": [
                    {
                        "path": capture.base.ITEMS_XML,
                        "sha256": hashlib.sha256(self.xml.read_bytes()).hexdigest(),
                    }
                ],
                "peer_overlay": {"path": "peer.json", "sha256": "retained"},
            },
            "regions": [
                {
                    "path": region,
                    "sha256": hashlib.sha256(raw).hexdigest(),
                    "tiles": 1,
                    "items": 1,
                }
            ],
            "totals": {"regions": 1, "sectors": 1, "tiles": 1, "items": 1},
            "palette": [
                {
                    "source_item_id": 100,
                    "key": "oteryn:terrain.tibia.i100",
                    "provisional": False,
                }
            ],
        }
        self.summary_path = (
            "tools/content-schema/world-authoring/samples/world-base-capture-v1.json"
        )
        self.summary = {
            "source": {"files": self.index["source"]["files"]},
            "map": {"width": 256, "height": 256},
            "totals": {"items": 999},
            "palette": {},
            "excluded_teleports": {"unset_destination": 1576},
            "fill": {"retained_donor_fact": True},
        }
        for name, document in [
            ("content/world/placements/index.json", self.index),
            (self.summary_path, self.summary),
        ]:
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(capture.base.canonical(document))
        path = self.root / region
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(raw)
        self.region = path

    def test_recounts_merged_facts_preserves_donor_facts_and_is_idempotent(self):
        output = capture.outputs(self.xml, self.root, refresh_summary=True)
        summary = json.loads(output[self.summary_path])
        self.assertEqual(summary["totals"], self.index["totals"])
        self.assertEqual(summary["source"], self.index["source"])
        self.assertEqual(summary["item_attributes"], {"action": 1})
        self.assertEqual(summary["tiles_by_floor"], {"7": 1})
        self.assertEqual(summary["bytes_on_disk"], self.region.stat().st_size)
        self.assertEqual(
            (summary["tiles_with_house"], summary["tiles_with_zone"]), (0, 0)
        )
        self.assertEqual(
            summary["palette"]["families"]["terrain"], {"entries": 1, "occurrences": 1}
        )
        for key in ("excluded_teleports", "fill", "map"):
            self.assertEqual(summary[key], self.summary[key])
        (self.root / self.summary_path).write_bytes(output[self.summary_path])
        self.assertEqual(
            capture.outputs(self.xml, self.root, refresh_summary=True), output
        )

    def test_region_drift_is_rejected(self):
        self.region.write_bytes(self.region.read_bytes() + b"drift")
        with self.assertRaisesRegex(ValueError, "sha256 differs"):
            capture.outputs(self.xml, self.root, refresh_summary=True)

    def test_refreshes_catalogue_ownership_without_changing_indices_or_regions(self):
        bindings = capture.base.canonical({"bindings": []})
        path = self.root / "imports/crystalserver/bindings/items.json"
        path.parent.mkdir(parents=True)
        path.write_bytes(bindings)
        self.index["item_bindings"] = {"sha256": hashlib.sha256(bindings).hexdigest()}
        self.index["palette"][0]["key"] = capture.base.donor_key(100)
        self.index["palette"][0]["provisional"] = True
        path = self.root / "content/world/placements/index.json"
        path.write_bytes(capture.base.canonical(self.index))
        terrain = self.root / "content/world/terrain/terrain-00000-00000.json"
        terrain.parent.mkdir(parents=True)
        terrain.write_bytes(
            capture.base.canonical(
                {"records": [{"identity": {"key": "oteryn:terrain.tibia.i100"}}]}
            )
        )
        region_before = self.region.read_bytes()
        output = capture.outputs(
            self.xml, self.root, refresh_summary=True, refresh_palette=True
        )
        index = json.loads(output["content/world/placements/index.json"])
        self.assertEqual(
            index["palette"],
            [
                {
                    "source_item_id": 100,
                    "key": "oteryn:terrain.tibia.i100",
                    "provisional": False,
                }
            ],
        )
        self.assertEqual(index["regions"], self.index["regions"])
        self.assertEqual(self.region.read_bytes(), region_before)
        self.assertEqual(
            json.loads(output[self.summary_path])["palette"]["families"]["terrain"],
            {"entries": 1, "occurrences": 1},
        )
        for name, raw in output.items():
            (self.root / name).write_bytes(raw)
        self.assertEqual(
            capture.outputs(
                self.xml, self.root, refresh_summary=True, refresh_palette=True
            ),
            output,
        )

    def test_index_totals_drift_is_rejected(self):
        self.index["totals"]["tiles"] += 1
        (self.root / "content/world/placements/index.json").write_bytes(
            capture.base.canonical(self.index)
        )
        with self.assertRaisesRegex(ValueError, "totals differ"):
            capture.outputs(self.xml, self.root, refresh_summary=True)

    def test_donor_input_pin_drift_is_rejected(self):
        self.index["source"]["files"][0]["sha256"] = "0" * 64
        (self.root / "content/world/placements/index.json").write_bytes(
            capture.base.canonical(self.index)
        )
        with self.assertRaisesRegex(ValueError, "donor input pins differ"):
            capture.outputs(self.xml, self.root, refresh_summary=True)


if __name__ == "__main__":
    unittest.main()
