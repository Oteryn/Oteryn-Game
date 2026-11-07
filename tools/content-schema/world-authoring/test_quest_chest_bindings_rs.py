"""R–S startup UID import must preserve the committed source map and fail closed."""
import json
import unittest

import quest_chest_bindings_rs as chests
import world_region_codec as codec


class QuestChestBindingsTests(unittest.TestCase):
    def test_all_committed_rs_chests_have_the_exact_source_uid_and_appearance(self):
        # Before the overlay this fails on Rapier: its placed chest has no UID.
        selected, _ = chests.bindings()
        index = json.loads((chests.ROOT / chests.INDEX).read_bytes())
        regions = {row['path'].split('/')[-1]: row for row in index['regions']}
        cache = {}
        for (x, y, z), expected in selected.items():
            name = codec.region_name(z, x // 256, y // 256)
            if name not in cache:
                cache[name] = codec.decode_region((chests.ROOT / regions[name]['path']).read_bytes())[3]
            matches = [attrs for _, tiles in cache[name]
                       for tx, ty, _, house, _, entries in tiles if (tx, ty) == (x, y) and not house
                       for palette, depth, attrs in entries
                       if not depth and index['palette'][palette]['source_item_id'] == expected['appearance']]
            self.assertEqual(len(matches), 1, expected['claim'])
            self.assertEqual((matches[0] or {}).get('unique'), expected['unique'], expected['claim'])

    def fixture(self, items=None, house=0):
        palette = [{'source_item_id': 2473}, {'source_item_id': 3031}]
        entries = items if items is not None else [(0, 0, {'text': 'retained'}), (1, 1, {'count': 3})]
        tiles = [(1, 2, 8, house, (4,), entries), (2, 2, 0, 0, (), [(0, 0, None)])]
        raw = codec.encode_region(9, 0, 0, {0: codec.encode_sector_tiles(0, 0, tiles)})
        selected = {(1, 2, 9): {'appearance': 2473, 'unique': 14042}}
        return raw, palette, selected, tiles

    def test_overlay_preserves_every_other_attribute_nested_item_and_tile(self):
        raw, palette, selected, tiles = self.fixture()
        result, imported, added = chests.overlay_region(raw, palette, selected)
        self.assertEqual(imported, {(1, 2, 9)})
        self.assertEqual(added, 1)
        tiles[0][5][0] = (0, 0, {'text': 'retained', 'unique': 14042})
        self.assertEqual(codec.decode_region(result)[3][0][1], tiles)
        self.assertEqual(chests.overlay_region(result, palette, selected), (result, imported, 0))

    def test_conflicting_uid_duplicate_appearance_and_house_refuse_whole_region(self):
        for entries, house in [([(0, 0, {'unique': 7})], 0),
                               ([(0, 0, None), (0, 0, None)], 0),
                               ([(1, 0, None)], 0), (None, 1)]:
            with self.subTest(entries=entries, house=house):
                raw, palette, selected, _ = self.fixture(entries, house)
                with self.assertRaises(ValueError):
                    chests.overlay_region(raw, palette, selected)


if __name__ == '__main__':
    unittest.main()
