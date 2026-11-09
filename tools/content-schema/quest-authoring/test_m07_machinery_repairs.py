import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools/content-schema/world-authoring'))
import world_region_codec


class MachineryRepairTests(unittest.TestCase):
    def rows(self):
        return json.loads((ROOT / 'content/quests/world-callbacks/m07-machinery-repairs.json').read_text())['callbacks']

    def test_six_atomic_sites_use_exactly_forty_four_kits(self):
        rows = self.rows()
        self.assertEqual([r['item_on_target']['count'] for r in rows], [7, 7, 7, 7, 8, 8])
        self.assertEqual(sum(r['item_on_target']['count'] for r in rows), 44)
        self.assertEqual(len({r['storage'][0]['track'] for r in rows}), 6)
        positions = []
        for row in rows:
            self.assertEqual(row['item_on_target']['item'], 'oteryn:item.tibia.i7785')
            self.assertFalse(row['item_on_target']['reusable'])
            self.assertEqual(len(row['passages']), row['item_on_target']['count'])
            self.assertEqual(row['minimum_level'], 50)
            self.assertEqual(row['quest'], 'oteryn:quest.machinery_of_war_quest')
            self.assertTrue(all(p['replacement_passable'] is False for p in row['passages']))
            self.assertTrue(any(p['position'] == row['selector']['position'] and p['replacement'] == row['effects']['transform'] for p in row['passages']))
            self.assertEqual(row['state_ops'][0]['from']['maximum'], 5)
            self.assertEqual(row['state_ops'][1]['from']['maximum'], 44 - row['item_on_target']['count'])
            positions.extend((p['position']['x'], p['position']['y']) for p in row['passages'])
        self.assertEqual(len(set(positions)), 44)

    def test_all_44_parts_and_control_ordinals_match_the_native_map(self):
        palette = json.loads((ROOT / 'content/world/placements/index.json').read_text())['palette']
        tiles = {}
        for name in ['region-z07-x128-y123.b3', 'region-z07-x128-y124.b3']:
            _, _, _, sectors = world_region_codec.decode_region((ROOT / 'content/world/placements' / name).read_bytes())
            tiles.update({(t[0], t[1]): [palette[i]['source_item_id'] for i, depth, _ in t[-1] if depth == 0] for _, group in sectors for t in group})
        for row in self.rows():
            source = row['selector']['native']
            self.assertEqual(tiles[(source['position']['x'], source['position']['y'])][source['ordinal']], source['appearance'])
            for part in row['passages']:
                ids = tiles[(part['position']['x'], part['position']['y'])]
                self.assertEqual(ids.count(part['appearance']), 1)
                self.assertEqual(part['replacement'], 1271 if part['appearance'] in [6280, 6282] else 1277)
                self.assertIn({'position': part['position'], 'appearance': part['appearance']}, row['world_requirements'])


if __name__ == '__main__':
    unittest.main()
