"""The source catalogue's full Item closure reaches the real active provider."""
import copy
import hashlib
import json
import unittest
from pathlib import Path

import hireling_item_profiles as hireling

ROOT = Path(__file__).resolve().parents[2]
PROFILES = "content/items/definitions/spell-native-profiles.json"


class HirelingItemProfiles(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.overlay = json.loads((ROOT / hireling.PATH).read_bytes())
        cls.raw = (ROOT / PROFILES).read_bytes()
        cls.profiles = {row["production_definition"]["production_key"]: row
                        for row in json.loads(cls.raw)["records"]}

    def test_all_source_trade_food_and_package_operands_are_profiled(self):
        self.assertEqual(self.overlay["source_revision"], hireling.REVISION)
        self.assertEqual(self.overlay["catalogue"]["trade_rows"], 1675)
        self.assertEqual(self.overlay["catalogue"]["trader_items"], 1409)
        self.assertEqual(self.overlay["catalogue"]["packages"], 23)
        self.assertEqual(self.overlay["catalogue"]["package_ingredients"], 69)
        for source in self.overlay["records"]:
            key = source["production_definition"]["production_key"]
            row = self.profiles[key]
            self.assertEqual(row["production_definition"], source["production_definition"])
            self.assertEqual(row["production_binding"], source["production_binding"])
            self.assertTrue(row["admission"]["materializable"])
            destinations = row["admission"]["legal_destinations"]
            if key == hireling.LAMP:
                self.assertEqual(destinations, ["CharacterInbox"])
            elif key == "oteryn:item.tibia.i23721":
                self.assertEqual(destinations, ["CharacterInventory"], "existing gold-pouch owner stays closed")
            elif "CharacterInventory" in source["admission"]["legal_destinations"]:
                self.assertIn("CharacterInventory", destinations, key)
            else:
                self.assertEqual(destinations, ["Ground"], key)
            self.assertEqual(row["admission"]["stack_class"], source["admission"]["stack_class"])
            qualified = hireling.qualified_source_values(row, source)
            for group in hireling.VALUE_GROUPS:
                # Known facts agree except the five asserted Q14A weights and
                # the one virtual GoldPouch movable pair approved in Q15A.
                self.assertEqual(hireling.merge_fact(row["semantics"][group],
                    qualified["semantics"][group], f"{key}:{group}"), row["semantics"][group])
        manifest = json.loads((ROOT / "content/spells.manifest.json").read_bytes())
        self.assertEqual(manifest["item_profiles"]["sha256"], hashlib.sha256(self.raw).hexdigest())
        print(f"HIRELING_NATIVE_PROFILE_CLOSURE items={len(self.overlay['required_ids'])} "
              f"active_profiles={len(self.profiles)} active_bytes={len(self.raw)}")

    def test_lamp_is_immovable_and_admitted_to_inbox_only(self):
        row = self.profiles[hireling.LAMP]
        self.assertEqual(row["admission"], {"materializable": True,
            "stack_class": "NonStackable", "legal_destinations": ["CharacterInbox"]})
        physical = row["semantics"]["physical"]["value"]
        self.assertEqual(physical["weight"], {"state": "KNOWN", "value": 1000})
        self.assertEqual(physical["movable"], {"state": "KNOWN", "value": False})
        for key, other in self.profiles.items():
            if key != hireling.LAMP:
                self.assertNotIn("CharacterInbox", other["admission"]["legal_destinations"])
        for number in range(29408, 29417):
            self.assertIn(f"oteryn:item.tibia.i{number}", self.profiles)
        self.assertEqual(self.overlay["catalogue"]["cook_items"], list(range(29408, 29417)))

    def test_complete_active_profiles_fit_the_unchanged_native_input_bound(self):
        # native_gameplay.rs::MAX_PROFILES remains 8 MiB. The former indented
        # 2,104-record output reached 9,074,560 bytes and could not activate.
        self.assertLessEqual(len(self.raw), 8 * 1024 * 1024,
            "the full active Item closure must activate without widening its resource bound")
        self.assertEqual(len(self.profiles), len(json.loads(self.raw)["records"]),
            "serialization cannot hide duplicate canonical Item profiles")

    def test_merge_preserves_richer_authoring_and_restricted_destinations(self):
        source = next(row for row in self.overlay["records"]
                      if "CharacterInventory" in row["admission"]["legal_destinations"]
                      and row["production_definition"]["production_key"] not in hireling.SHARED_NATIVE_WEIGHTS)
        existing = copy.deepcopy(source)
        existing["attributes"]["speed_bonus"] = 83
        existing["admission"]["legal_destinations"] = ["CharacterInventory"]
        existing["semantics"]["physical"]["value"]["weight"] = {"state": "UNKNOWN"}
        merged = hireling.merge_record(existing, source)
        self.assertEqual(merged["attributes"], existing["attributes"])
        self.assertEqual(merged["authoring"], existing["authoring"])
        self.assertEqual(merged["admission"]["legal_destinations"], ["CharacterInventory"])
        self.assertEqual(merged["semantics"]["physical"], source["semantics"]["physical"])

    def test_nonportable_offer_never_gains_inventory_admission(self):
        source = next(row for row in self.overlay["records"]
                      if row["production_definition"]["production_key"] == "oteryn:item.tibia.i2352")
        self.assertEqual(source["admission"]["legal_destinations"], ["Ground"])
        existing = copy.deepcopy(source)
        existing["admission"]["materializable"] = False
        self.assertEqual(hireling.merge_record(existing, source)["admission"], source["admission"])
        wrapped = {row["offered_item_id"]: row for row in self.overlay["wrapped_trade_items"]}
        self.assertEqual(wrapped[2352], {"offered_item_id": 2352,
            "delivered_item_id": 23398, "unwrap_item_id": 2352})
        self.assertIn("oteryn:item.tibia.i23398", self.profiles)
        self.assertEqual(set(wrapped), {2352, 2426, *range(32473, 32481), *range(32482, 32490)})

    def test_answer_q11_preserves_three_stale_sell_rows_without_a_carry_grant(self):
        # Crystal Lua IDs remain literal; the retained names do not override
        # the engine's pickup facts or create a second item-ID mapping.
        for number in (3657, 5526, 25686):
            row = self.profiles[f"oteryn:item.tibia.i{number}"]
            self.assertEqual(row["admission"]["legal_destinations"], ["Ground"])
            self.assertEqual(row["semantics"]["physical"]["value"]["pickupable"],
                {"state": "KNOWN", "value": False})
            self.assertIn(number, self.overlay["nonportable_ids"])

    def test_answer_q14_preserves_only_the_five_asserted_shared_weights(self):
        sources = {row["production_definition"]["production_key"]: row for row in self.overlay["records"]}
        self.assertEqual(len(hireling.SHARED_NATIVE_WEIGHTS), 5)
        for key, (native_weight, donor_weight, line) in hireling.SHARED_NATIVE_WEIGHTS.items():
            source, active = sources[key], self.profiles[key]
            path = lambda row: row["semantics"]["physical"]["value"]["weight"]
            self.assertEqual(path(source), {"state": "KNOWN", "value": donor_weight})
            self.assertEqual(path(active), {"state": "KNOWN", "value": native_weight})
            self.assertEqual(self.overlay["source_lines"][key.removeprefix("oteryn:item.tibia.i")],
                f"data/items/items.xml:{line}")
            before = copy.deepcopy(source)
            self.assertEqual(path(hireling.merge_record(active, source)), path(active))
            self.assertEqual(source, before, "the captured donor proof remains untouched")
            drifted = copy.deepcopy(source)
            path(drifted)["value"] += 1
            with self.assertRaisesRegex(ValueError, "HIRELING_SHARED_WEIGHT_DRIFT"):
                hireling.merge_record(active, drifted)
            changed_active = copy.deepcopy(active)
            path(changed_active)["value"] += 1
            with self.assertRaisesRegex(ValueError, "HIRELING_SHARED_WEIGHT_DRIFT"):
                hireling.merge_record(changed_active, source)
            other_fact = copy.deepcopy(source)
            other_fact["semantics"]["physical"]["value"]["pickupable"]["value"] = False
            with self.assertRaisesRegex(ValueError, "HIRELING_NATIVE_SOURCE_CONFLICT"):
                hireling.merge_record(active, other_fact)

    def test_known_source_conflict_never_silently_overwrites_existing_value(self):
        source = self.profiles[hireling.LAMP]
        existing = copy.deepcopy(source)
        existing["semantics"]["physical"]["value"]["weight"]["value"] += 1
        before = copy.deepcopy(existing)
        with self.assertRaisesRegex(ValueError, "HIRELING_NATIVE_SOURCE_CONFLICT"):
            hireling.merge_record(existing, source)
        self.assertEqual(existing, before)

    def test_answer_q15_preserves_only_the_virtual_pouch_movable_pair(self):
        key = "oteryn:item.tibia.i23721"
        self.assertEqual(hireling.SHARED_NATIVE_MOVABLE, {key: (True, False, 47233)})
        source = next(row for row in self.overlay["records"]
                      if row["production_definition"]["production_key"] == key)
        active = self.profiles[key]
        fact = lambda row: row["semantics"]["physical"]["value"]["movable"]
        self.assertEqual(fact(source), {"state": "KNOWN", "value": False})
        self.assertEqual(fact(active), {"state": "KNOWN", "value": True})
        self.assertEqual(source["admission"]["legal_destinations"], ["Ground"])
        self.assertEqual(active["admission"]["legal_destinations"], ["CharacterInventory"])
        self.assertEqual(self.overlay["source_lines"]["23721"], "data/items/items.xml:47233")
        original = copy.deepcopy(source)
        self.assertEqual(hireling.merge_record(active, source), active,
                         "the virtual Hireling operand cannot change its shared GoldPouch owner")
        self.assertEqual(source, original, "raw donor facts are not rewritten")
        for change_old, replacement in ((True, False), (False, True), (True, 1), (False, 0)):
            before, donor = copy.deepcopy(active), copy.deepcopy(source)
            fact(before if change_old else donor)["value"] = replacement
            with self.assertRaisesRegex(ValueError, "HIRELING_SHARED_MOVABLE_DRIFT"):
                hireling.merge_record(before, donor)
        other = copy.deepcopy(source)
        other["semantics"]["container"]["value"]["capacity"]["value"] += 1
        with self.assertRaisesRegex(ValueError, "HIRELING_NATIVE_SOURCE_CONFLICT"):
            hireling.merge_record(active, other)
        coin = next(row for row in self.overlay["records"]
                    if row["production_definition"]["production_key"] == "oteryn:item.tibia.i3031")
        other = copy.deepcopy(coin)
        fact(other)["value"] = False
        with self.assertRaisesRegex(ValueError, "HIRELING_NATIVE_SOURCE_CONFLICT"):
            hireling.merge_record(self.profiles["oteryn:item.tibia.i3031"], other)


if __name__ == "__main__":
    unittest.main()
