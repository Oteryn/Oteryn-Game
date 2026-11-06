"""Standalone quest aliases still select the exact native source objects."""
import json
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools/content-schema/world-authoring'))
from world_region_codec import decode_region


class StandaloneChestsSource(unittest.TestCase):
    def test_beregar_and_smithhammer_roots_have_no_legacy_script_attributes(self):
        palette = json.loads((ROOT / 'content/world/placements/index.json').read_bytes())['palette']
        for x, y, z, appearance in [
            (32546, 31522, 11, 1983),
            (32774, 32253, 8, 2472),
            (32776, 32253, 8, 2472),
            (32778, 32253, 8, 2472),
        ]:
            path = ROOT / f'content/world/placements/region-z{z:02}-x{x // 256:03}-y{y // 256:03}.b3'
            _, _, _, sectors = decode_region(path.read_bytes())
            tiles = [tile for _, tiles in sectors for tile in tiles if tile[:2] == (x, y)]
            self.assertEqual(len(tiles), 1)
            self.assertEqual(tiles[0][3], 0)
            roots = [entry for entry in tiles[0][5]
                     if entry[1] == 0 and palette[entry[0]]['source_item_id'] == appearance]
            self.assertEqual(len(roots), 1)
            self.assertIsNone(roots[0][2])


if __name__ == '__main__':
    unittest.main()
