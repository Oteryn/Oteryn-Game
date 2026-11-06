"""Standalone quest aliases still select the exact native source objects."""
import json
import hashlib
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools/content-schema/world-authoring'))
from world_region_codec import decode_region


class StandaloneChestsSource(unittest.TestCase):
    def test_native_reward_roots_retain_the_pinned_container_facts(self):
        sys.path[:0] = [str(ROOT / 'tools/content-schema/item-authoring'),
                        str(ROOT / 'tools/content-migration')]
        from engine_items import load_appearance_objects
        from fathers_burden_item_profiles import CLIENT
        raw = (ROOT / CLIENT).read_bytes()
        self.assertEqual(hashlib.sha256(raw).hexdigest(), CLIENT.rsplit('-', 1)[1][:-4])
        appearances = load_appearance_objects(raw)
        flags = appearances[2472]['flags']
        self.assertTrue(flags['flags.container'])
        self.assertTrue(flags['flags.take'])
        self.assertFalse(flags.get('flags.unmove', False))
        self.assertTrue(appearances[1983]['flags']['flags.unmove'])
        drawer = appearances[2434]['flags']
        self.assertTrue(drawer['flags.container'])
        self.assertFalse(drawer.get('flags.take', False))
        self.assertNotIn('flags.unmove', drawer)
        self.assertTrue(drawer['flags.usable'])

    def test_fixed_native_roots_have_no_legacy_script_attributes(self):
        palette = json.loads((ROOT / 'content/world/placements/index.json').read_bytes())['palette']
        for x, y, z, appearance in [
            (32546, 31522, 11, 1983),
            (32774, 32253, 8, 2472),
            (32776, 32253, 8, 2472),
            (32778, 32253, 8, 2472),
            (32778, 32232, 8, 2772),
            (32780, 32231, 8, 389),
            (32591, 31647, 3, 2472),
            (32590, 31647, 3, 2472),
            (32588, 31645, 3, 2434),
            (32588, 31644, 3, 2434),
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
