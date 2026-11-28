"""The live Item provider must retain independent shot overrides and caps."""

import json
import unittest

import ranged_item_profiles as ranged

NEWLY_EQUIPPABLE = (3448, 3449, 6528, 14251, 14252, 15793, 16141, 16142, 16143)


class AmmunitionHitChanceTests(unittest.TestCase):
    def test_native_profiles_match_source_override_and_cap(self):
        source, _, _ = ranged.source_profiles(ranged.ROOT)
        records = json.loads((ranged.ROOT / ranged.PROFILES).read_bytes())["records"]
        native = {
            row["production_definition"]["production_key"]: row["authoring"]["ranged"]
            for row in records
            if "ranged" in row["authoring"]
        }
        for item_id in NEWLY_EQUIPPABLE:
            with self.subTest(item_id=item_id):
                actual = native[f"oteryn:item.tibia.i{item_id}"]
                self.assertEqual(
                    (actual["hit_chance"], actual["max_hit_chance"]),
                    (source[item_id]["hit_chance"], source[item_id]["max_hit_chance"]),
                )


if __name__ == "__main__":
    unittest.main()
