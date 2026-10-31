"""M06Q: exercise the committed production placement and lossless binding import."""
import json
import unittest

import quest_door_bindings as doors
import world_region_codec as codec


class QuestDoorBindingsTests(unittest.TestCase):
    def test_m06q_placed_banshee_door_carries_its_storage_action(self):
        index = json.loads((doors.ROOT / doors.INDEX).read_bytes())
        raw = (doors.ROOT / 'content/world/placements/region-z14-x125-y124.b3').read_bytes()
        _, _, _, sectors = codec.decode_region(raw)
        items = [attrs for _, tiles in sectors for x, y, _, _, _, entries in tiles
                 if (x, y) == (32246, 31861) for palette, depth, attrs in entries
                 if not depth and index['palette'][palette]['source_item_id'] == 5113]
        self.assertEqual(len(items), 1)
        self.assertEqual((items[0] or {}).get('action'), 10089)

    def test_m06q_one_action_cannot_authorize_two_different_tracks(self):
        document = json.loads((doors.ROOT / doors.CAPTURE).read_bytes())
        tracks = {}
        for row in document['bindings']:
            self.assertEqual(tracks.setdefault(row['action'], row['track']), row['track'])
        masks = [row for row in document['bindings'] if
                 row.get('decision', {}).get('category') == 'behaviour']
        self.assertEqual(len(masks), 6)
        self.assertEqual({row['action'] for row in masks}, {46906, 46907, 46908, 46909})

    def test_m06q_overlay_preserves_unrelated_facts_and_is_idempotent(self):
        capture = {'bindings': [{'position': [1, 2, 7], 'action': 123, 'item': 5745}],
                   'pairs': [{'closed': 5745, 'open': 5746}]}
        palette = [{'source_item_id': 5745}, {'source_item_id': 3031}]
        tiles = [(1, 2, 0, 0, (), [(0, 0, None), (1, 0, {'count': 3})]),
                 (2, 2, 0, 0, (), [(0, 0, None)])]
        raw = codec.encode_region(7, 0, 0, {0: codec.encode_sector_tiles(0, 0, tiles)})
        result, rows = doors.overlay_region(raw, palette, capture)
        self.assertEqual(rows, [{'position': [1, 2, 7], 'item': 5745, 'action': 123}])
        tiles[0][5][0] = (0, 0, {'action': 123})
        self.assertEqual(codec.decode_region(result)[3][0][1], tiles)
        self.assertEqual(doors.overlay_region(result, palette, capture)[0], result)
        capture['bindings'][0]['action'] = 456
        with self.assertRaisesRegex(ValueError, 'action conflict'):
            doors.overlay_region(result, palette, capture)


if __name__ == '__main__':
    unittest.main()
