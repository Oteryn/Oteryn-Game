"""Inventory grants must resolve to profiles accepted by the native Item owner."""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "content-migration"))
import forge_sql_admission
import inventory_grant_profiles as inventory


class InventoryGrantProfiles(unittest.TestCase):
    def test_fixture_and_content_grants_have_admissible_item_profiles(self):
        self.assertEqual(inventory.missing_inventory_profiles(), {})

    def test_adventurers_stone_has_a_nonstackable_inventory_profile(self):
        index, profiles = inventory.profile_index(inventory.ROOT)
        key = "oteryn:item.tibia.i16277"
        self.assertEqual(index[key], "definition-r1")
        profile = profiles.get((key, index[key]))
        self.assertTrue(inventory.inventory_admissible(profile))
        self.assertEqual(profile["admission"]["stack_class"], "NonStackable")
        self.assertEqual(profile["semantics"]["physical"]["value"]["weight"], {
            "state": "KNOWN", "value": 30,
        })

    def test_inventory_grant_profiles_preserve_the_forge_sql_projection(self):
        actual, expected = forge_sql_admission.checked_projection()
        self.assertEqual(actual, expected)
