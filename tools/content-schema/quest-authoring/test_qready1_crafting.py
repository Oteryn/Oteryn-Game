"""M20's tool identity, positive material cost and normalized Items remain reproducible."""

import json
from pathlib import Path
import sys
import unittest

import qready1_craft_callbacks as crafts

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tools/content-schema/world-authoring"))
import world_region_codec as codec


class Qready1CraftBindings(unittest.TestCase):
    def test_registered_recipe_matches_generator_and_chosen_s2(self):
        self.assertEqual(crafts.build(), json.loads(crafts.OUTPUT.read_bytes()))
        rows = json.loads(
            (
                ROOT
                / "tools/content-schema/quest-authoring/samples/completion242/recipes.json"
            ).read_bytes()
        )["records"]
        quest = next(
            r for r in rows if r["identity"]["key"] == "oteryn:quest.the_cursed_crystal"
        )
        step = next(s for s in quest["recipe"]["stages"] if s["key"] == "s2")
        self.assertEqual(["Green Crystal Splinter", "Wooden Grinder"], step["targets"])
        row = crafts.build()["callbacks"][0]
        self.assertEqual("oteryn:item.tibia.i16122", row["item_on_target"]["item"])
        self.assertEqual(1, row["item_on_target"]["count"])
        self.assertFalse(row["item_on_target"]["reusable"])
        self.assertEqual(
            "oteryn:item.tibia.i21507", row["effects"]["item_reward"]["item"]
        )
        self.assertEqual(
            row["storage"][0]["track"], row["effects"]["item_reward_cycle"]
        )
        self.assertTrue(row["storage"][0]["increment"])

    def test_source_qualified_material_and_immovable_station(self):
        packet = json.loads(
            (
                ROOT
                / "tools/content-schema/native-gameplay/qready1-craft-item-profiles.json"
            ).read_bytes()
        )
        rows = {
            r["production_definition"]["production_key"]: r for r in packet["records"]
        }
        self.assertEqual(
            {"oteryn:item.tibia.i21507", "oteryn:item.tibia.i21573"}, set(rows)
        )
        dust = rows["oteryn:item.tibia.i21507"]
        self.assertEqual(60, dust["semantics"]["physical"]["value"]["weight"]["value"])
        station = rows["oteryn:item.tibia.i21573"]
        self.assertEqual(["Ground"], station["admission"]["legal_destinations"])
        self.assertFalse(
            station["semantics"]["physical"]["value"]["pickupable"]["value"]
        )
        self.assertFalse(station["semantics"]["physical"]["value"]["movable"]["value"])

    def test_grinder_is_an_existing_unmodified_native_placement(self):
        index = json.loads((ROOT / "content/world/placements/index.json").read_bytes())
        raw = (ROOT / "content/world/placements/region-z07-x125-y128.b3").read_bytes()
        tile = next(
            t
            for _, tiles in codec.decode_region(raw)[3]
            for t in tiles
            if t[:2] == (32012, 32908)
        )
        roots = [
            index["palette"][i]["source_item_id"]
            for i, depth, _ in tile[5]
            if depth == 0
        ]
        self.assertEqual([4404, 4572, 15858, 21573], roots)
        self.assertEqual(21573, roots[3])
        self.assertEqual(0, tile[3], "public source tool is not inside a player house")
