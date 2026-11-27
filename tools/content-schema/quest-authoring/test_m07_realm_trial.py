import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


class RealmTrialBindingTests(unittest.TestCase):
    def test_native_realm_controls_bind_entry_and_all_guardian_floors(self):
        rows = json.loads(
            (ROOT / "content/quests/world-callbacks/m07-realm-trial.json").read_text()
        )["callbacks"]
        by_key = {row["key"]: row for row in rows}
        prefix = "oteryn:quest-callback/m07/realm-"
        self.assertCountEqual(
            [row["key"] for row in rows],
            [
                prefix + stage
                for stage in (
                    "begin",
                    "draconia-book",
                    "entry",
                    "exit",
                    "reward",
                    "trial",
                )
            ],
        )
        begin = by_key[prefix + "begin"]
        trial = by_key[prefix + "trial"]
        self.assertEqual(begin["quest"], "oteryn:quest.chayenne_realm")
        self.assertEqual(begin["minimum_level"], 40)
        self.assertEqual(
            (begin["storage"][0]["minimum"], begin["storage"][0]["set"]), (-1, 1)
        )
        self.assertEqual(
            (trial["storage"][0]["minimum"], trial["storage"][0]["set"]), (1, 2)
        )
        for callback in (begin, trial):
            self.assertTrue(callback["requires_world_admission"])
        key = "oteryn:item.tibia.i14682"
        self.assertEqual(begin["item_on_target"]["item"], key)
        self.assertEqual(begin["item_on_target"]["count"], 1)
        self.assertTrue(begin["item_on_target"]["reusable"])
        self.assertEqual(trial["present"], [{"item": key, "count": 1}])
        self.assertNotIn("item_on_target", trial)
        area = trial["effects"]["clear_area"]
        self.assertEqual(area["minimum"]["floor"], -3)
        self.assertEqual(area["additional_floors"], [-6, -5, -4])
        self.assertEqual(
            set(area["creatures"]),
            {
                "oteryn:creature.dragon",
                "oteryn:creature.dragon_lord",
                "oteryn:creature.fire_elemental",
            },
        )
        self.assertEqual(trial["effects"]["reset_after_ms"], 300000)
        self.assertEqual(trial["effects"]["passages_reset_after_ms"], 60000)
        self.assertEqual(begin["selector"]["native"]["appearance"], 2124)
        self.assertEqual(trial["selector"]["action"], 55021)
        self.assertEqual(trial["selector"]["appearance"], 2772)
        self.assertEqual(trial["lever"], [2772, 2773])


if __name__ == '__main__':
    unittest.main()
