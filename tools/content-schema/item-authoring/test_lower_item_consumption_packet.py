"""ITEM-SEM-USE-2: the consumption lowering is reproducible and every row names its source."""

import json
import unittest

import lower_item_consumption_packet as lane


class Consumption(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.snapshot = json.loads(lane.SNAPSHOT.read_bytes())
        cls.facts, cls.admission = lane.build(cls.snapshot)

    def test_two_runs_and_the_committed_packets_are_identical(self):
        again = lane.build(self.snapshot)
        self.assertEqual(lane.encode(again[0]), lane.encode(self.facts))
        self.assertEqual(lane.encode(again[1]), lane.encode(self.admission))
        self.assertEqual(lane.FACTS.read_bytes(), lane.encode(self.facts))
        self.assertEqual(lane.ADMISSION.read_bytes(), lane.encode(self.admission))

    def test_every_row_names_its_source(self):
        for row in self.facts["rows"]:
            source = row["source"]
            if row["consumption"]["kind"] == "FOOD":
                self.assertIn(source["kind"], {"TIBIAWIKI", "OTS_HYPOTHESIS_ONLY"})
                if source["kind"] == "TIBIAWIKI":
                    self.assertTrue(source["tibiawiki"]["revision_id"])
                else:
                    self.assertTrue(source["canary"]["line"])
            else:
                self.assertEqual(source["canary"]["path"], lane.POTIONS_LUA)
                self.assertTrue(source["canary"]["line"])

    def test_food_is_within_the_model_and_ham_matches_tibiawiki(self):
        food = {
            r["target"]["key"]: r
            for r in self.facts["rows"]
            if r["consumption"]["kind"] == "FOOD"
        }
        for row in food.values():
            self.assertTrue(
                1 <= row["consumption"]["value"]["regeneration_seconds"] <= 1199
            )
        ham = food["oteryn:item.tibia.i3582"]
        self.assertEqual(ham["consumption"]["value"]["regeneration_seconds"], 360)
        self.assertEqual(ham["source"]["tibiawiki"]["regenseconds"], "360")
        self.assertTrue(ham["source"]["canary_agrees"])

    def test_the_floor_potions_are_lowered_with_flasks_and_requirements(self):
        potions = {
            int(r["target"]["key"][len(lane.KEY) :]): r
            for r in self.facts["rows"]
            if r["consumption"]["kind"] == "POTION"
        }
        for item_id in (266, 236, 239, 7643, 23375, 268, 237, 238, 23373, 7642, 23374):
            self.assertIn(item_id, potions)
        for item_id in (7642, 23374):
            restores = potions[item_id]["consumption"]["value"]["restores"]
            self.assertEqual([r["resource"] for r in restores], ["HEALTH", "MANA"])
        for row in potions.values():
            for restore in row["consumption"]["value"]["restores"]:
                self.assertTrue(1 <= restore["min"] <= restore["max"] <= 10000)
            flask = row["consumption"]["value"]["empty_flask"]["value"]["key"]
            self.assertIn(flask, {lane.KEY + str(i) for i in lane.FLASKS.values()})
        # Wiki and Canary agree: the health potion leaves the small flask i285.
        flask = potions[266]["consumption"]["value"]["empty_flask"]["value"]["key"]
        self.assertEqual(flask, lane.KEY + "285")
        self.assertNotIn("use_requirements", potions[266])
        self.assertEqual(potions[23375]["use_requirements"]["min_level"]["value"], 200)

    def test_unsupported_rows_are_reported_not_lowered(self):
        lowered = {r["target"]["key"] for r in self.facts["rows"]}
        for entry in self.facts["reported"]:
            if "item_id" in entry:
                self.assertNotIn(lane.KEY + str(entry["item_id"]), lowered)
        reasons = {(e["kind"], e["reason"]) for e in self.facts["reported"]}
        self.assertIn(("POTION", "attribute_or_condition_potion"), reasons)
        self.assertIn(("FOOD", "regeneration_1200_seconds_or_more"), reasons)

    def test_admissions_are_identity_only_baseline_items_with_both_sources(self):
        world = self.snapshot["world_baseline"]
        for row in self.admission["admissions"]:
            item_id = row["item_key"][len(lane.KEY) :]
            self.assertFalse(world[item_id]["materializable"])
            self.assertEqual(world[item_id]["stack_class"], "Unknown")
            self.assertEqual(
                row["stack_max"],
                lane.STACK_MAX if row["stack_class"] == "STACK_CAPABLE" else None,
            )
            self.assertIn(row["stack_class"], {"STACK_CAPABLE", "NON_STACKABLE"})
            self.assertIn(row["evidence"]["tibiawiki"]["stackable"], {"yes", "no"})
            self.assertEqual(row["evidence"]["canary"]["sha256"], lane.ITEMS_XML_SHA256)

    def test_canary_parsers(self):
        foods = lane.parse_foods(
            [{"line": 3, "text": '\t[3582] = { 30, "Chomp." }, -- ham'}]
        )
        self.assertEqual(foods, {3582: {"value": 30, "line": 3}})
        potions = lane.parse_potions(
            [
                {
                    "line": 9,
                    "text": "\t[7642] = { health = { 250, 350 }, mana = { 100, 200 }, vocations = "
                    "{ VOCATION.BASE_ID.PALADIN, VOCATION.BASE_ID.MONK }, level = 80, flask = 284, "
                    'description = "x" },',
                },
                {
                    "line": 10,
                    "text": "\t[7439] = { vocations = { VOCATION.BASE_ID.KNIGHT }, condition = berserk },",
                },
            ]
        )
        self.assertEqual(
            potions[7642],
            {
                "line": 9,
                "health": [250, 350],
                "mana": [100, 200],
                "level": 80,
                "vocations": ["MONK", "PALADIN"],
                "flask": 284,
            },
        )
        self.assertEqual(potions[7439]["unsupported"], ["condition"])


if __name__ == "__main__":
    unittest.main()
