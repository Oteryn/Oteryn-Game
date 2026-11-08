"""M01b keeps native entry selectors and separate once-only visit counters exact."""
import unittest
import json
import sys
from unittest import mock
from pathlib import Path

import questpc_explore_bindings as explore

ROOT = Path(__file__).resolve().parents[3]


class ExploreBindingsTests(unittest.TestCase):
    def test_unimplemented_required_guard_is_not_silently_dropped(self):
        guarded = (('oteryn:quest.authored.asura_palace_quest', 's1',
                    'oteryn:area.hunting_place.asura_palace', 'authored/asura_palace_quest'),)
        with mock.patch.object(explore, 'BINDINGS', guarded):
            with self.assertRaisesRegex(ValueError, 'additional authoritative guards'):
                explore.callbacks(ROOT)

    def test_declared_area_point_really_has_the_retained_native_ground(self):
        sys.path.insert(0, str(ROOT / 'tools/content-schema/world-authoring'))
        import world_region_codec as codec
        callback, = explore.callbacks(ROOT)
        native = callback['selector']['native']
        at = native['position']
        index = json.loads((ROOT / 'content/world/placements/index.json').read_text())
        source = ROOT / 'content/world/placements/region-z07-x127-y124.b3'
        raw = source.read_bytes()
        floor, rx, ry, table = codec.parse_region(raw)
        self.assertEqual((floor, rx, ry), (7, 127, 124))
        found = []
        for local, offset, length in table:
            sx, sy = rx * 8 + local % 8, ry * 8 + local // 8
            if (sx, sy) != (at['x'] // 32, at['y'] // 32):
                continue
            payload = codec.zstandard.ZstdDecompressor().decompress(
                raw[offset:offset + length], max_output_size=codec.MAX_SECTOR_BYTES)
            for x, y, _, house, _, items in codec.decode_sector(payload, sx, sy):
                if (x, y) == (at['x'], at['y']):
                    self.assertEqual(house, 0)
                    found = [index['palette'][palette]['source_item_id']
                             for palette, depth, _ in items if depth == 0]
        self.assertEqual(found, [native['appearance']])

    def test_native_anchor_has_no_value_or_movement_effect(self):
        callback, = explore.callbacks(ROOT)
        self.assertEqual(callback['quest'], 'oteryn:quest.longsword_quest')
        self.assertEqual(callback['trigger'], 'step_in')
        self.assertEqual(callback['selector']['native'], {
            'position': {'x': 32663, 'y': 31970, 'floor': -7},
            'ordinal': 0, 'appearance': 4525})
        self.assertTrue(callback['requires_world_admission'])
        self.assertEqual(callback['offerings'], [])
        self.assertIsNone(callback['teleport'])
        seen, counter = callback['storage']
        self.assertEqual((seen['minimum'], seen['maximum'], seen['set']), (0, 0, 1))
        self.assertEqual(counter['track'],
                         'oteryn:quest-progress/chosen-source/longsword_quest/s1')
        self.assertEqual((counter['minimum'], counter['maximum'], counter['set']), (0, 0, 1))
        self.assertTrue(counter['increment'])
