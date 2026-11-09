"""M20's tool identity, positive material cost and normalized Items remain reproducible."""

import json
import sys
import unittest
from pathlib import Path

import qready1_craft_callbacks as crafts

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tools/content-schema/world-authoring"))
import world_region_codec as codec

sys.path.insert(0, str(ROOT / "tools/content-schema/native-gameplay"))
import build_qready1_cradle_items as cradle


class Qready1CraftBindings(unittest.TestCase):
    def test_cradle_uses_three_prepared_inputs_and_one_antimutagen(self):
        row = next(r for r in crafts.build()["callbacks"] if r["key"].endswith("cradle-brew"))
        primary = row["item_on_target"]
        self.assertEqual("oteryn:quest.cradle_of_monsters", row["quest"])
        self.assertEqual("oteryn:item.tibia.i40601", primary["item"])
        self.assertEqual(
            [{"item": "oteryn:item.tibia.i40598", "count": 1},
             {"item": "oteryn:item.tibia.i40016", "count": 1}],
            primary["ingredients"],
        )
        self.assertEqual("oteryn:item.tibia.i40603", row["effects"]["item_reward"]["item"])
        self.assertEqual(1, row["effects"]["item_reward"]["count"])

    def test_blender_spends_the_five_literal_materials_on_its_native_tile(self):
        row = next(r for r in crafts.build()["callbacks"] if r["key"].endswith("scatter-blend"))
        self.assertIsNone(row["item_on_target"])
        self.assertEqual(
            {f"oteryn:item.tibia.i{i}" for i in [3603, 3492, 3723, 3606, 3738]},
            {r["item"] for r in row["offerings"]},
        )
        self.assertTrue(all(r["count"] == 1 for r in row["offerings"]))
        self.assertTrue(all(r["position"] == row["selector"]["native"]["position"] for r in row["offerings"]))
        self.assertEqual("oteryn:item.tibia.i9109", row["effects"]["item_reward"]["item"])

    def test_cradle_bindings_reproduce_named_official_and_explicit_wiki_evidence(self):
        packet = cradle.build()
        self.assertEqual(packet, json.loads(cradle.OUTPUT.read_bytes()))
        self.assertEqual(4, len(packet["records"]))
        for row, evidence in zip(packet["records"], packet["evidence"], strict=True):
            identity = row["production_binding"]
            self.assertEqual("tibiawiki/item_id", identity["identity_namespace"])
            self.assertEqual(evidence["appearance_id"], evidence["wiki_item_id"])
            self.assertEqual(str(evidence["appearance_id"]), identity["external_id"])
            self.assertEqual(f"fandom-r{evidence['wiki_fact']['revision_id']}", identity["source_revision"])
            self.assertEqual(evidence["official_name"].casefold(), evidence["wiki_fact"]["wiki_title"].casefold())
    def test_first_mixture_reuses_existing_native_qn4_crafting_owner(self):
        recipes = json.loads((ROOT / "content/quests/carried-recipes.json").read_bytes())
        recipe = next(r for r in recipes if r["key"] == "oteryn:quest-craft/cursed-crystal/first-ointment")
        self.assertEqual([11466, 21504], recipe["ingredients"])
        self.assertEqual([[21505, 1]], recipe["outputs"])
        final = json.loads((ROOT / "content/quests/npc-callbacks/m20-cursed-crystal-recipes.json").read_bytes())["callbacks"]
        self.assertEqual(2, len(final))
        self.assertTrue(all(row["storage"][0]["at_least"] for row in final))
        self.assertTrue(all(row["give"]["item"] == "oteryn:item.tibia.i21506" for row in final))

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
            {f"oteryn:item.tibia.i{i}" for i in [8997, 9108, 9109, 21506, 21507, 21573]}, set(rows)
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
