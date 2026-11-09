import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools/content-schema/world-authoring'))
import world_region_codec

ROOTS = [(32824, 31708), (32828, 31709), (32832, 31709), (32836, 31709),
         (32827, 31713), (32830, 31713), (32833, 31712), (32836, 31712)]


class MachineryBurnBindingTests(unittest.TestCase):
    def callbacks(self):
        path = ROOT / 'content/quests/world-callbacks/m07-machinery-burn.json'
        return json.loads(path.read_text())['callbacks']

    def test_eight_distinct_personal_sites_keep_the_exact_fire_bug(self):
        rows = self.callbacks()
        self.assertEqual(len(rows), 8)
        self.assertEqual(len({r['storage'][0]['track'] for r in rows}), 8)
        for row, (x, y) in zip(rows, ROOTS):
            self.assertEqual(row['quest'], 'oteryn:quest.machinery_of_war_quest')
            self.assertEqual(row['minimum_level'], 50)
            self.assertTrue(row['requires_world_admission'])
            self.assertFalse(row['reversible'])
            self.assertEqual(row['trigger'], 'use')
            self.assertEqual(row['selector']['native'], {
                'position': {'x': x, 'y': y, 'floor': -7}, 'ordinal': 1, 'appearance': 5697})
            self.assertIsNone(row['selector']['unique'])
            self.assertIsNone(row['selector']['action'])
            self.assertEqual(row['item_on_target'], {
                'item': 'oteryn:item.tibia.i5467', 'count': 1, 'reusable': True,
                'replacement': None, 'target_replacement': None})
            flag = row['storage'][0]
            self.assertEqual((flag['minimum'], flag['maximum'], flag['set']), (-1, 0, 1))
            self.assertEqual(row['effects'], {'transform': 2113, 'reset_after_ms': 60000})
            counter = row['state_ops'][0]
            self.assertEqual(counter['from'], {'kind': 'between', 'minimum': -1, 'maximum': 7})
            self.assertEqual(counter['effect'], {'kind': 'add_at_least', 'value': 1, 'minimum': 1})

    def test_all_three_machine_parts_are_witnessed_in_the_native_map(self):
        palette = json.loads((ROOT / 'content/world/placements/index.json').read_text())['palette']
        region = ROOT / 'content/world/placements/region-z07-x128-y123.b3'
        _, _, _, sectors = world_region_codec.decode_region(region.read_bytes())
        tiles = {(tile[0], tile[1]): tile[-1] for _, group in sectors for tile in group}
        for row, (x, y) in zip(self.callbacks(), ROOTS):
            self.assertEqual(len(row['world_requirements']), 3)
            expected = {(x, y, 5697), (x, y - 1, 5695), (x - 1, y, 5696)}
            actual = {(r['position']['x'], r['position']['y'], r['appearance'])
                      for r in row['world_requirements']}
            self.assertEqual(actual, expected)
            for tx, ty, appearance in expected:
                self.assertIn(appearance, [palette[i]['source_item_id']
                                          for i, depth, _ in tiles[(tx, ty)] if depth == 0])
            i, depth, _ = tiles[(x, y)][row['selector']['native']['ordinal']]
            self.assertEqual((palette[i]['source_item_id'], depth), (5697, 0))


if __name__ == '__main__':
    unittest.main()
