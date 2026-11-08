"""Pinned startup tables must agree with physical World objects, before compilation."""
import json
import unittest

import questpc5_startup_projection as author
import world_region_codec as codec


class QuestStartupBindings(unittest.TestCase):
    def test_source_projection_is_reproducible_and_uses_the_global_donor(self):
        doc = author.document()
        self.assertEqual(json.loads(author.OUTPUT.read_text()), doc)
        self.assertTrue(all(row['source'].startswith('data-global/startup/tables/')
                            for row in doc['bindings']))
        skeleton, = [row for row in doc['bindings'] if row['value'] == 20003]
        self.assertEqual((skeleton['position'], skeleton['item'], skeleton['attribute']),
                         ([32211, 31273, 10], 4285, 'unique'))
        hoists = [row for row in doc['bindings'] if row['value'] in (3059, 3060, 3061)]
        self.assertEqual([(row['position'], row['item'], row['value']) for row in hoists],
                         [([32157, 31126, z], 1949, 3050 + z) for z in (9, 10, 11)])

    def test_every_selector_has_one_exact_unhoused_top_level_source_object(self):
        doc = author.document()
        root = author.ROOT / 'content/world/placements'
        palette = json.loads((root / 'index.json').read_text())['palette']
        regions = {}
        for row in doc['bindings']:
            x, y, z = row['position']
            key = (z, x // 256, y // 256)
            if key not in regions:
                path = root / codec.region_name(*key)
                regions[key] = codec.decode_region(path.read_bytes())[3]
            tiles = [tile for _, sector in regions[key] for tile in sector
                     if tile[:2] == (x, y)]
            self.assertEqual(len(tiles), 1, row)
            self.assertEqual(tiles[0][3], 0, row)
            objects = [(depth, attrs) for index, depth, attrs in tiles[0][5]
                       if depth == 0 and palette[index]['source_item_id'] == row['item']]
            self.assertEqual(len(objects), 1, row)
            value = (objects[0][1] or {}).get(row['attribute'])
            self.assertIn(value, (None, row['value']), row)

    def test_legion_helmet_retains_the_exact_donor_corpse_selector(self):
        row,=[r for r in author.document()['bindings'] if r['attribute']=='unique' and r['value']==6006]
        self.assertEqual((row['position'],row['item']),([32143,31910,8],4240))
        self.assertEqual(row['source'],'data-global/startup/tables/chest.lua')
        self.assertGreater(row['line'],0)

    def test_hoist_direction_lever_is_the_real_upper_floor_default(self):
        root = author.ROOT / 'content/world/placements'
        palette = json.loads((root / 'index.json').read_text())['palette']
        x, y, z = 32156, 31125, 10
        sectors = codec.decode_region((root / codec.region_name(z, x // 256, y // 256)).read_bytes())[3]
        tile, = [tile for _, sector in sectors for tile in sector if tile[:2] == (x, y)]
        self.assertEqual(tile[3], 0)
        top = [(palette[index]['source_item_id'], attrs) for index, depth, attrs in tile[5] if depth == 0]
        self.assertEqual(top, [(5770, None), (5772, None), (2773, None)])


if __name__ == '__main__':
    unittest.main()
