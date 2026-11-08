"""The selected daily harvest binds existing native plants, never invented props."""
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools/content-schema/world-authoring'))
from world_region_codec import decode_region


class OramondNativePlants(unittest.TestCase):
    def test_marked_plants_exist_at_the_exact_native_stack_ordinal(self):
        index = json.loads((ROOT / 'content/world/placements/index.json').read_text())
        document = json.loads((ROOT / 'content/quests/world-callbacks/oramond-native-roots.json').read_text())
        seen = set()
        for callback in document['callbacks']:
            native = callback['selector']['native']
            pos = native['position']
            self.assertEqual(callback['selector']['position'], pos)
            self.assertEqual(callback['selector']['appearance'], native['appearance'])
            z = -pos['floor']
            path = ROOT / f"content/world/placements/region-z{z:02}-x{pos['x']//256:03}-y{pos['y']//256:03}.b3"
            _, _, _, sectors = decode_region(path.read_bytes())
            matches = [tile for _, tiles in sectors for tile in tiles
                       if tile[0:2] == (pos['x'], pos['y'])]
            self.assertEqual(len(matches), 1)
            items = [(i, attrs) for i, depth, attrs in matches[0][5] if depth == 0]
            palette, attrs = items[native['ordinal']]
            self.assertEqual(index['palette'][palette]['source_item_id'], native['appearance'])
            self.assertIn(native['appearance'], (21104, 21105))
            self.assertEqual(sum(index['palette'][i]['source_item_id'] == native['appearance'] for i, _ in items), 1)
            self.assertFalse(attrs, 'native bare-object admission rejects attributed props')
            self.assertNotIn((pos['x'], pos['y'], z, native['ordinal']), seen)
            seen.add((pos['x'], pos['y'], z, native['ordinal']))
        self.assertEqual(len(seen), 5)


if __name__ == '__main__':
    unittest.main()
