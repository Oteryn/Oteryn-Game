"""Pinned Grizzly extraction: exact named storage mapping and fail-closed facts."""

import json
import unittest

import grizzly_source


class GrizzlySourceTests(unittest.TestCase):
    def test_named_storage_uses_existing_quest_track_spelling(self):
        self.assertEqual(
            grizzly_source.canonical_storage("AltKillCount.QuaramMntassinScoutCount"),
            "oteryn:quest-progress/quest/u8_5/killing_in_the_name_of/"
            "alt_kill_count/quaram_mntassin_scout_count",
        )

    def test_unknown_reward_kind_cannot_be_silently_dropped(self):
        with self.assertRaisesRegex(ValueError, "unsupported reward"):
            grizzly_source.rewards([{"type": "unmapped", "value": [1]}], {})

    def test_storage_expressions_must_resolve_to_pinned_numeric_values(self):
        with self.assertRaisesRegex(ValueError, "unknown storage"):
            grizzly_source.storage_number({"expr": "Storage.Unknown"}, {})
        self.assertEqual(grizzly_source.storage_number(41300, {}), 41300)

    def test_position_constructor_lowering_preserves_source_lines(self):
        text = "local bosses = {\n {playerPosition = Position(1, 2, 3)},\n}\n"
        lowered = grizzly_source.lower_positions(text)
        self.assertEqual(lowered.count("\n"), text.count("\n"))
        self.assertIn("{ x = 1, y = 2, z = 3 }", lowered)

    def test_shop_rejects_unknown_storage_authority(self):
        text = 'npcConfig.shop = {{clientId=5907,buy=35000,itemName="slingshot",storageKey=UNKNOWN,storageValue=20}}'
        with self.assertRaisesRegex(ValueError, "unknown Grizzly shop gate"):
            grizzly_source.shop_offers(text)

    def test_shop_keeps_source_prices_and_canonical_eye_identity(self):
        text = '''npcConfig.shop = {
          {clientId=11491,sell=500,itemName="quara bone",storageKey=POINTSSTORAGE,storageValue=10},
          {clientId=11491,sell=350,itemName="quara eye",storageKey=POINTSSTORAGE,storageValue=10},
        }'''
        offers = grizzly_source.shop_offers(text)
        self.assertEqual([(x["item"]["key"], x["unit_price"]) for x in offers], [
            ("oteryn:item.tibia.i11491", 500), ("oteryn:item.tibia.i11488", 350)])
        self.assertEqual([x["donor_line"] for x in offers], [2, 3])

    def test_shop_item_definitions_exist_at_the_selected_identity(self):
        catalogue = json.loads(grizzly_source.OUTPUT.read_text())
        definitions = {}
        for path in (grizzly_source.ROOT / "content/items/definitions").glob("*.json"):
            for record in json.loads(path.read_text()).get("records", []):
                if "definition" not in record:
                    continue
                identity = record["definition"]["identity"]
                definitions[identity["key"]] = identity["revision"]
        self.assertEqual(len(catalogue["shop"]), 56)
        for offer in catalogue["shop"]:
            self.assertEqual(definitions[offer["item"]["key"]], offer["item"]["revision"])


if __name__ == "__main__":
    unittest.main()
