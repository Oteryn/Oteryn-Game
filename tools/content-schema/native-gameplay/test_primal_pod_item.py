import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[3]


class PrimalPodItemTests(unittest.TestCase):
    def test_primal_pod_is_materializable_with_source_nonpickupable_flags(self):
        source = json.loads((ROOT / "tools/content-schema/native-gameplay/primal-pod-item-profile.json").read_bytes())
        self.assertEqual(source["source_revision"], "201189f871d23597b82caf88fd611454a2ddefb8")
        self.assertEqual(len(source["records"]), 1)
        row = source["records"][0]
        self.assertEqual(row["production_definition"], {"family": "Item", "production_key": "oteryn:item.tibia.i39176", "revision_ref": "definition-r1"})
        item = row["admission"]
        self.assertTrue(item["materializable"])
        self.assertEqual(item["stack_class"], "NonStackable")
        self.assertEqual(item["legal_destinations"], ["Ground"])
        for attribute in ("blocks_movement", "blocks_projectile", "immovable_block_solid"):
            self.assertIs(row["attributes"][attribute], False)
        physical = row["semantics"]["physical"]["value"]
        self.assertEqual(physical["movable"], {"state": "KNOWN", "value": False})
        self.assertEqual(physical["pickupable"], {"state": "KNOWN", "value": False})
        self.assertEqual(row["semantics"]["container"], {"state": "NOT_APPLICABLE"})
        self.assertEqual(row["semantics"]["temporal"], {"state": "NOT_APPLICABLE"})


if __name__ == "__main__":
    unittest.main()
