"""Cover the quest and equipment producers' shared native Item identities."""
import copy
import json
import unittest
from pathlib import Path

import crystal_equipment_slice as equipment
from register_spell_families import provider_order

ROOT = Path(__file__).resolve().parents[2]


class QuestEquipmentCompositionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        provider = ROOT / "content/items/definitions/spell-native-profiles.json"
        cls.records = {
            row["production_definition"]["production_key"]: row
            for row in json.loads(provider.read_bytes())["records"]
        }

    def quest_provider(self):
        def item(source_id):
            return copy.deepcopy(self.records[equipment.item_key(source_id)])

        pairs = [item(source_id) for source_id in (23474, 23477)]
        for row in pairs:
            row["semantics"]["use_transform"] = equipment.UNKNOWN
            row["semantics"]["protection"] = equipment.UNKNOWN
        # Quest custody does not require the protection or imbuement projection.
        rewards = [item(source_id) for source_id in (19356, 19362, 19363, 19366, 19373, 19391)]
        for row in rewards:
            row["semantics"]["protection"] = equipment.UNKNOWN
            row["semantics"]["imbuement"] = equipment.UNKNOWN
        return {"records": pairs + rewards}

    def test_quest_reward_equipment_extends_the_single_custody_owner(self):
        provider = self.quest_provider()
        rewards = provider["records"][2:]
        owners = copy.deepcopy(rewards)
        equipment.admit(ROOT, provider, provider_order)
        for owner, row in zip(owners, rewards):
            key = owner["production_definition"]["production_key"]
            with self.subTest(key=key):
                self.assertEqual(
                    [
                        candidate
                        for candidate in provider["records"]
                        if candidate["production_definition"] == owner["production_definition"]
                    ],
                    [row],
                )
                for field in ("admission", "authoring", "production_binding"):
                    self.assertEqual(row[field], owner[field])
                self.assertEqual(row["semantics"]["protection"]["state"], "KNOWN")

    def test_quest_reward_conflicting_custody_is_rejected(self):
        provider = self.quest_provider()
        reward = provider["records"][2]
        reward["admission"]["legal_destinations"] = ["Ground"]
        external_id = reward["production_binding"]["external_id"]
        with self.assertRaisesRegex(ValueError, f"equipment source identity collision {external_id}"):
            equipment.admit(ROOT, provider, provider_order)


if __name__ == "__main__":
    unittest.main()
