import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools/content-schema/world-authoring'))
import world_region_codec

ROOTS = [(32909, 31733, 1), (32905, 31733, 1), (32905, 31736, 2), (32909, 31736, 2),
         (32905, 31739, 2), (32909, 31739, 2), (32905, 31742, 2), (32909, 31742, 1)]


class CatapultBurnBindingTests(unittest.TestCase):
    def callbacks(self):
        path = ROOT / 'content/quests/world-callbacks/m07-machinery-catapults.json'
        return json.loads(path.read_text())['callbacks']

    def test_eight_distinct_catapults_keep_the_tool_and_separate_credit(self):
        rows = self.callbacks()
        self.assertEqual(len(rows), 8)
        self.assertEqual(len({r['storage'][0]['track'] for r in rows}), 8)
        for row, (x, y, ordinal) in zip(rows, ROOTS):
            self.assertEqual(row['quest'], 'oteryn:quest.machinery_of_war_quest')
            self.assertEqual(row['minimum_level'], 50)
            self.assertTrue(row['requires_world_admission'])
            self.assertFalse(row['reversible'])
            self.assertEqual(row['selector']['native'], {
                'position': {'x': x, 'y': y, 'floor': -7}, 'ordinal': ordinal, 'appearance': 5608})
            self.assertIsNone(row['selector']['unique'])
            self.assertIsNone(row['selector']['action'])
            self.assertEqual(row['item_on_target']['item'], 'oteryn:item.tibia.i5467')
            self.assertTrue(row['item_on_target']['reusable'])
            self.assertEqual(row['effects'], {'transform': 2113, 'reset_after_ms': 60000})
            self.assertTrue(row['state_ops'][0]['track'].endswith('/catapults_burned'))
            self.assertTrue(row['storage'][0]['track'].rsplit('/', 1)[-1].startswith('catapult_'))

    def test_all_four_catapult_parts_and_source_ordinals_match_the_b3(self):
        palette = json.loads((ROOT / 'content/world/placements/index.json').read_text())['palette']
        region = ROOT / 'content/world/placements/region-z07-x128-y123.b3'
        _, _, _, sectors = world_region_codec.decode_region(region.read_bytes())
        tiles = {(tile[0], tile[1]): tile[-1] for _, group in sectors for tile in group}
        for row, (x, y, ordinal) in zip(self.callbacks(), ROOTS):
            self.assertEqual(len(row['world_requirements']), 4)
            expected = {(x, y, 5608), (x, y - 1, 5606), (x - 1, y, 5607), (x - 1, y - 1, 5605)}
            actual = {(r['position']['x'], r['position']['y'], r['appearance'])
                      for r in row['world_requirements']}
            self.assertEqual(actual, expected)
            for tx, ty, appearance in expected:
                self.assertIn(appearance, [palette[i]['source_item_id']
                                          for i, depth, _ in tiles[(tx, ty)] if depth == 0])
            i, depth, _ = tiles[(x, y)][ordinal]
            self.assertEqual((palette[i]['source_item_id'], depth), (5608, 0))


if __name__ == '__main__':
    unittest.main()
