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


if __name__ == '__main__':
    unittest.main()
