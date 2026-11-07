"""SRV7 Cake bridge: canonical Terrain identity, never an invented Item alias."""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


class CakeBridgeGroundTest(unittest.TestCase):
    def test_bridge_appearance_is_canonical_terrain_in_the_real_placement_palette(self):
        index = json.loads((ROOT / "content/world/placements/index.json").read_bytes())
        entries = [row for row in index["palette"] if row.get("source_item_id") == 48227]
        self.assertEqual(len(entries), 1)
        self.assertEqual(entries[0]["key"], "oteryn:terrain.tibia.i48227")
        self.assertFalse(entries[0]["provisional"])
        bindings = json.loads((ROOT / "imports/crystalserver/bindings/items.json").read_bytes())
        self.assertFalse(any(int(row["external_id"]) == 48227 for row in bindings["bindings"]))
        records = []
        for path in (ROOT / "content/world/terrain").glob("terrain-*.json"):
            records.extend(json.loads(path.read_bytes())["records"])
        matches = [row for row in records if row["identity"]["key"] == entries[0]["key"]]
        self.assertEqual(len(matches), 1)
        self.assertEqual(matches[0]["ground_speed"], {"state": "KNOWN", "value": 150})
        self.assertEqual(matches[0]["client_projection"], {"state": "KNOWN", "value": {"appearance_id": 48227}})


if __name__ == "__main__":
    unittest.main()
