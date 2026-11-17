import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


class RealmTrialBindingTests(unittest.TestCase):
    def test_native_realm_controls_bind_entry_and_all_guardian_floors(self):
        rows = json.loads((ROOT / 'content/quests/world-callbacks/m07-realm-trial.json').read_text())['callbacks']
        begin, trial = rows
        self.assertEqual(begin['quest'], 'oteryn:quest.chayenne_realm')
        self.assertEqual(begin['minimum_level'], 40)
        self.assertEqual((begin['storage'][0]['minimum'], begin['storage'][0]['set']), (-1, 1))
        self.assertEqual((trial['storage'][0]['minimum'], trial['storage'][0]['set']), (1, 2))
        for callback in rows:
            self.assertTrue(callback['requires_world_admission'])
            self.assertEqual(callback['item_on_target']['item'], 'oteryn:item.tibia.i14682')
            self.assertEqual(callback['item_on_target']['count'], 1)
            self.assertTrue(callback['item_on_target']['reusable'])
        area = trial['effects']['clear_area']
        self.assertEqual(area['minimum']['floor'], -3)
        self.assertEqual(area['additional_floors'], [-6, -5, -4])
        self.assertEqual(set(area['creatures']), {'oteryn:creature.dragon', 'oteryn:creature.dragon_lord',
                                                'oteryn:creature.fire_elemental'})
        self.assertEqual(trial['effects']['reset_after_ms'], 300000)
        self.assertEqual(trial['effects']['passages_reset_after_ms'], 60000)
        self.assertEqual(trial['selector']['native']['appearance'], 2124)
        self.assertEqual(trial['effects']['transform'], 2773)


if __name__ == '__main__':
    unittest.main()
