import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


class LionTearsBindingTests(unittest.TestCase):
    def test_native_tears_offering_consumes_exact_vessel_and_resets_pagoda(self):
        callbacks = json.loads((ROOT / 'content/quests/world-callbacks/m07-lion-offerings.json').read_text())['callbacks']
        callback = next((c for c in callbacks if c['key'] == 'oteryn:quest-callback/m07/lion-tears'), None)
        self.assertIsNotNone(callback, 'native Lion Tears offering missing')
        self.assertEqual(callback['quest'], 'oteryn:quest.tibia_tales')
        # Crystal actions_lions_rock.lua:35-36, including Lua's \z whitespace join.
        self.assertEqual(callback['success_message'],
                         "You have purified a sacret pedestal with holy water. You have now passed the last testto enter thge inner sanctum.")
        self.assertTrue(callback['requires_world_admission'])
        self.assertEqual(callback['selector']['native'], {
            'position': {'x': 33154, 'y': 32279, 'floor': -8}, 'ordinal': 2, 'appearance': 21391})
        self.assertEqual(callback['item_on_target']['item'], 'oteryn:item.tibia.i21466')
        self.assertEqual(callback['item_on_target']['count'], 1)
        self.assertFalse(callback['item_on_target'].get('reusable', False))
        self.assertEqual(callback['effects']['passages_reset_after_ms'], 15000)
        self.assertEqual(callback['passages'], [{'position': {'x': 33156, 'y': 32279, 'floor': -8},
                                                'appearance': 2074, 'replacement': 2075}])
        self.assertEqual(callback['storage'][0]['track'],
                         'oteryn:quest-progress/crystalserver/quest/u10_70/lions_rock/outer_sanctum/lions_tears')
        self.assertEqual(callback['storage'][0]['set'], 1)
        self.assertEqual(callback['state_ops'][0]['effect'], {'kind': 'add_at_least', 'value': 1, 'minimum': 2})


    def test_native_beauty_offering_owns_exact_flower_and_brazier(self):
        callbacks = json.loads((ROOT / 'content/quests/world-callbacks/m07-lion-offerings.json').read_text())['callbacks']
        callback = next((c for c in callbacks if c['key'] == 'oteryn:quest-callback/m07/lion-beauty'), None)
        self.assertIsNotNone(callback, 'native Lion Beauty offering missing')
        self.assertEqual(callback['selector']['native'], {
            'position': {'x': 33138, 'y': 32369, 'floor': -8}, 'ordinal': 3, 'appearance': 2110})
        self.assertEqual(callback['item_on_target']['item'], 'oteryn:item.tibia.i21389')
        self.assertEqual(callback['item_on_target']['count'], 1)
        self.assertEqual(callback['storage'][0]['track'].rsplit('/', 1)[1], 'lions_beauty')
        self.assertEqual(callback['effects']['passages_reset_after_ms'], 15000)

    def test_both_flower_plants_are_native_renewable_and_guard_the_current_bloom(self):
        callbacks = json.loads((ROOT / 'content/quests/world-callbacks/m07-lion-offerings.json').read_text())['callbacks']
        plants = [c for c in callbacks if c['key'].startswith('oteryn:quest-callback/m07/lion-flower-')]
        self.assertEqual(len(plants), 2, 'both captured native flowers need a producer')
        self.assertEqual({(c['selector']['native']['position']['x'], c['selector']['native']['position']['y'])
                          for c in plants}, {(33133, 32359), (33157, 32364)})
        for callback in plants:
            native = callback['selector']['native']
            self.assertEqual((native['ordinal'], native['appearance'], native['position']['floor']), (2, 21388, -6))
            self.assertTrue(callback['requires_world_admission'])
            self.assertTrue(callback['effects']['item_reward_per_use'])
            self.assertEqual(callback['effects']['item_reward']['item'], 'oteryn:item.tibia.i21389')
            self.assertEqual(callback['effects']['item_reward']['count'], 1)
            self.assertEqual((callback['effects']['transform'], callback['effects']['reset_after_ms']), (21935, 60000))
            self.assertEqual(callback['world_requirements'], [{'position': native['position'], 'appearance': 21388}])


if __name__ == '__main__':
    unittest.main()
