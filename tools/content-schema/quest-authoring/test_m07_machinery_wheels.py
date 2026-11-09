import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools/content-schema/world-authoring'))
import world_region_codec

WHEELS = [(32964, 31814, 1), (32961, 31817, 1), (32964, 31817, 2),
          (32964, 31820, 1), (32961, 31823, 1), (32964, 31823, 2)]


class MachineryWheelBindingTests(unittest.TestCase):
    def callbacks(self):
        path = ROOT / 'content/quests/world-callbacks/m07-machinery-wheels.json'
        return json.loads(path.read_text())['callbacks']

    def test_six_personal_wheels_keep_the_exact_tool_and_bounded_counter(self):
        rows = self.callbacks()
        self.assertEqual(len(rows), 6)
        self.assertEqual(len({r['storage'][0]['track'] for r in rows}), 6)
        for row, (x, y, ordinal) in zip(rows, WHEELS):
            self.assertEqual(row['quest'], 'oteryn:quest.machinery_of_war_quest')
            self.assertEqual(row['minimum_level'], 50)
            self.assertTrue(row['requires_world_admission'])
            self.assertFalse(row['reversible'])
            self.assertEqual(row['selector']['native'], {
                'position': {'x': x, 'y': y, 'floor': -7}, 'ordinal': ordinal, 'appearance': 7775})
            self.assertEqual(row['item_on_target']['item'], 'oteryn:item.tibia.i5467')
            self.assertEqual(row['item_on_target']['count'], 1)
            self.assertTrue(row['item_on_target']['reusable'])
            flag = row['storage'][0]
            self.assertEqual((flag['minimum'], flag['maximum'], flag['set']), (-1, 0, 1))
            self.assertEqual(row['effects'], {'transform': 2113, 'reset_after_ms': 60000})
            counter = row['state_ops'][0]
            self.assertTrue(counter['track'].endswith('/tower_wheels_burned'))
            self.assertEqual(counter['from'], {'kind': 'between', 'minimum': -1, 'maximum': 5})
            self.assertEqual(counter['effect'], {'kind': 'add_at_least', 'value': 1, 'minimum': 1})

    def test_all_twelve_wheel_parts_match_the_captured_native_map(self):
        palette = json.loads((ROOT / 'content/world/placements/index.json').read_text())['palette']
        region = ROOT / 'content/world/placements/region-z07-x128-y124.b3'
        _, _, _, sectors = world_region_codec.decode_region(region.read_bytes())
        tiles = {(tile[0], tile[1]): tile[-1] for _, group in sectors for tile in group}
        for row, (x, y, ordinal) in zip(self.callbacks(), WHEELS):
            self.assertEqual(len(row['world_requirements']), 2)
            self.assertEqual({(r['position']['x'], r['position']['y'], r['appearance'])
                              for r in row['world_requirements']}, {(x, y, 7775), (x + 1, y, 7776)})
            for tx, item in [(x, 7775), (x + 1, 7776)]:
                self.assertIn(item, [palette[i]['source_item_id'] for i, depth, _ in tiles[(tx, y)] if depth == 0])
            i, depth, _ = tiles[(x, y)][ordinal]
            self.assertEqual((palette[i]['source_item_id'], depth), (7775, 0))


if __name__ == '__main__':
    unittest.main()
