"""M38 static source collision closure, without Item/use admission."""
import copy
import json
import sys
import tempfile
from pathlib import Path
import unittest

import npcfix_travel_static as travel
import world_objects


class NpcTravelStaticTests(unittest.TestCase):
    def test_exact_source_flags_and_collision_are_preserved(self):
        rows = json.loads(travel.outputs()[travel.SHARD])["records"]
        by_id = {row["provenance"]["source_item_id"]: row for row in rows}
        self.assertEqual(set(by_id), travel.IDS)
        self.assertEqual(by_id[20661]["kind"]["value"], "border")
        self.assertFalse(by_id[20661]["walkable"]["value"])
        for item in (34434, 51921):
            self.assertTrue(by_id[item]["walkable"]["value"])
        for row in rows:
            self.assertNotIn("item_pointer", row["provenance"])
            self.assertEqual(world_objects.schema_errors("Terrain", row), [])
            self.assertEqual(world_objects.validate_record(row), [])
        forged = copy.deepcopy(by_id[20661])
        forged["walkable"]["value"] = True
        self.assertTrue(world_objects.validate_record(forged))

    def test_palette_slots_and_unrelated_unknowns_are_preserved(self):
        current = json.loads((travel.ROOT / "content/world/placements/index.json").read_bytes())
        generated = json.loads(travel.outputs()["content/world/placements/index.json"])
        self.assertEqual(len(current["palette"]), len(generated["palette"]))
        for before, after in zip(current["palette"], generated["palette"]):
            if before["source_item_id"] not in travel.IDS:
                self.assertEqual(before, after)
            else:
                self.assertEqual(after["key"], f"oteryn:terrain.tibia.i{before['source_item_id']}")
                self.assertFalse(after["provisional"])
        self.assertTrue(any(row["provisional"] for row in generated["palette"]))

    def test_real_travel_anchor_geometry_has_no_omitted_source_identity(self):
        import world_region_codec as codec
        index = json.loads(travel.outputs()["content/world/placements/index.json"])
        anchors = ((32552, 32497), (31938, 31928), (32534, 32513))
        selected = [r for r in index["regions"] if any(
            f"z07-x{x // 256}-y{y // 256}.b3" in r["path"] for x, y in anchors
        )]
        self.assertEqual(len(selected), 3)
        seen = set()
        for region in selected:
            raw = (travel.ROOT / region["path"]).read_bytes()
            for _, tiles in codec.decode_region(raw)[3]:
                for x, y, _, _, _, items in tiles:
                    if not any(abs(x - ax) <= 3 and abs(y - ay) <= 3 for ax, ay in anchors):
                        continue
                    for palette_index, _, _ in items:
                        row = index["palette"][palette_index]
                        self.assertFalse(row["provisional"], row)
                        if row["source_item_id"] in travel.IDS:
                            seen.add(row["source_item_id"])
        self.assertEqual(seen, travel.IDS)

    def test_historical_marker_strips_only_verified_addition(self):
        sys.path.insert(0, str(travel.ROOT / "tools/content-schema/item-authoring"))
        import historical_world_owner_context as historical
        generated = travel.outputs()
        marker = generated[historical.MARKER]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            shard = root / travel.SHARD
            shard.parent.mkdir(parents=True)
            shard.write_bytes(generated[travel.SHARD])
            stripped = historical.without_npcfix(root, marker)
            self.assertNotIn(travel.MARKER_NOTE, json.loads(stripped)["notes"])
            shard.write_bytes(generated[travel.SHARD] + b" ")
            with self.assertRaisesRegex(ValueError, "terrain witness"):
                historical.without_npcfix(root, marker)


if __name__ == "__main__":
    unittest.main()
