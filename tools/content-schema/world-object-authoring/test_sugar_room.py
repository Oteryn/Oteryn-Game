"""Real source room identity and physical collision closure; no synthetic resolver."""

import copy
import json
import unittest

import sugar_room
import world_objects
import world_region_codec as codec


class SugarRoomTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.generated = sugar_room.outputs()
        cls.records = json.loads(cls.generated[sugar_room.SHARD])["records"]

    def test_exact_79_source_flags_and_native_geometry(self):
        self.assertEqual(
            {r["provenance"]["source_item_id"] for r in self.records}, sugar_room.IDS
        )
        by_id = {r["provenance"]["source_item_id"]: r for r in self.records}
        self.assertEqual(by_id[48229]["ground_speed"]["value"], 150)
        self.assertTrue(by_id[48229]["walkable"]["value"])
        for item in (45179, 45181, 45182, 45015, 45031):
            self.assertEqual(by_id[item]["kind"]["value"], "wall")
            self.assertFalse(by_id[item]["walkable"]["value"])
        for item in (
            45015,
            45031,
            45176,
            45177,
            45178,
            45179,
            45180,
            45181,
            45182,
            45250,
        ):
            self.assertFalse(by_id[item]["blocks_projectile"]["value"])
        for item in (45196, 48121):
            self.assertTrue(by_id[item]["blocks_projectile"]["value"])
            self.assertEqual(by_id[item]["ground_speed"]["value"], 0)
            self.assertFalse(by_id[item]["walkable"]["value"])
        for row in self.records:
            self.assertEqual(world_objects.schema_errors("Terrain", row), [])
            self.assertEqual(world_objects.validate_record(row), [])
        witness = json.loads((sugar_room.ROOT / sugar_room.CAPTURE).read_bytes())
        self.assertTrue(witness["flags"]["45015"]["flags.usable"])
        self.assertTrue(witness["flags"]["45031"]["flags.usable"])
        foreign = copy.deepcopy(by_id[45179])
        foreign["provenance"]["source_item_id"] = 999999
        self.assertTrue(world_objects.schema_errors("Terrain", foreign))
        forged = copy.deepcopy(by_id[45179])
        forged["walkable"]["value"] = True
        self.assertTrue(world_objects.validate_record(forged))

    def test_real_b3_room_has_no_provisional_omissions_and_preserves_indices(self):
        index = json.loads(
            (sugar_room.ROOT / "content/world/placements/index.json").read_bytes()
        )
        palette = index["palette"]
        anchors = {(33339, y) for y in range(32221, 32226)} | {(33397, 32201)}
        paths = [
            sugar_room.ROOT / r["path"]
            for r in index["regions"]
            if "z09-x130-y125" in r["path"]
        ]
        self.assertEqual(len(paths), 1)
        seen = set()
        occurrences = 0
        for _, tiles in codec.decode_region(paths[0].read_bytes())[3]:
            for x, y, _, _, _, items in tiles:
                if not (
                    (33360 <= x <= 33382 and 32215 <= y <= 32246) or (x, y) in anchors
                ):
                    continue
                for palette_index, _, _ in items:
                    row = palette[palette_index]
                    self.assertFalse(
                        row["provisional"], f"room source identity omitted: {row}"
                    )
                    if row["source_item_id"] in sugar_room.IDS:
                        seen.add(row["source_item_id"])
                        occurrences += 1
                        self.assertEqual(
                            row["key"], f"oteryn:terrain.tibia.i{row['source_item_id']}"
                        )
        self.assertEqual(seen, sugar_room.IDS)
        self.assertEqual(occurrences, 1119)
        self.assertTrue(
            any(r["provisional"] for r in palette),
            "unrelated unknown donor keys stay refused",
        )


if __name__ == "__main__":
    unittest.main()
