"""Keep the native no-UID reward selector tied to the real source map cell."""
import json
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools/content-schema/world-authoring'))
from world_region_codec import decode_region


class HydraEggSource(unittest.TestCase):
    def test_native_nest_has_the_exact_source_items_and_no_invented_binding(self):
        packet = json.loads((ROOT / 'tools/content-schema/quest-authoring/samples/native-rewards/hydra-egg.json').read_bytes())
        palette = json.loads((ROOT / 'content/world/placements/index.json').read_bytes())['palette']
        _, _, _, sectors = decode_region((ROOT / packet['native_map']['path']).read_bytes())
        position = packet['selector']['position']
        found = [tile for _, tiles in sectors for tile in tiles
                 if tile[:2] == (position['x'], position['y'])]
        self.assertEqual(len(found), 1)
        tile = found[0]
        self.assertEqual(tile[3], 0, 'quest source is not a house tile')
        roots = [entry for entry in tile[5] if entry[1] == 0]
        self.assertEqual([palette[index]['source_item_id'] for index, _, _ in roots],
                         packet['native_map']['top_level_source_items'])
        index, _, attributes = roots[packet['native_map']['nest_top_level_ordinal']]
        self.assertEqual(palette[index]['source_item_id'], packet['selector']['appearance'])
        self.assertEqual(attributes, packet['native_map']['nest_attributes'])
        self.assertIsNone(packet['selector']['unique'])
        self.assertIsNone(packet['selector']['action'])
        self.assertFalse(packet['hydra_kill_required'])


if __name__ == '__main__':
    unittest.main()
