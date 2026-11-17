import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools/content-schema/world-authoring'))
import world_region_codec

SITES = [(32593, 31841), (32601, 31841), (32610, 31841)]


class TooHotColumnTests(unittest.TestCase):
    def rows(self):
        return json.loads((ROOT / 'content/quests/world-callbacks/m07-too-hot-columns.json').read_text())['callbacks']

    def test_three_distinct_repairs_keep_chisel_and_do_not_grant_completion(self):
        rows = self.rows()
        self.assertEqual(len(rows), 3)
        self.assertEqual(len({r['storage'][0]['track'] for r in rows}), 3)
        for row, (x, y) in zip(rows, SITES):
            self.assertEqual(row['quest'], 'oteryn:quest.too_hot_to_handle_quest')
            self.assertEqual(row['minimum_level'], 80)
            self.assertEqual(row['admission'], {'premium': True})
            self.assertEqual(row['selector']['native'], {'position': {'x': x, 'y': y, 'floor': -9}, 'ordinal': 1, 'appearance': 36541})
            self.assertEqual(row['item_on_target'], {'item': 'oteryn:item.tibia.i36543', 'count': 1, 'replacement': None, 'target_replacement': None, 'reusable': True})
            self.assertEqual(row['effects'], {})
            self.assertNotIn('completion', row)
            self.assertEqual(row['state_ops'][0]['from'], {'kind': 'between', 'minimum': -1, 'maximum': 2})
            self.assertEqual(row['state_ops'][0]['effect'], {'kind': 'add_at_least', 'value': 1, 'minimum': 1})

    def test_exact_column_source_ordinals(self):
        palette = json.loads((ROOT / 'content/world/placements/index.json').read_text())['palette']
        _, _, _, sectors = world_region_codec.decode_region((ROOT / 'content/world/placements/region-z09-x127-y124.b3').read_bytes())
        tiles = {(t[0], t[1]): t[-1] for _, group in sectors for t in group}
        for row, site in zip(self.rows(), SITES):
            self.assertEqual(palette[tiles[site][1][0]]['source_item_id'], 36541)
            self.assertEqual(row['world_requirements'], [{'position': row['selector']['position'], 'appearance': 36541}])


if __name__ == '__main__':
    unittest.main()
