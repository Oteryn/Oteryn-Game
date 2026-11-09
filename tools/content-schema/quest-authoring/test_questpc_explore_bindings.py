"""M01b keeps native entry selectors and separate once-only visit counters exact."""
import unittest
import json
import sys
from unittest import mock
from pathlib import Path

import questpc_explore_bindings as explore

ROOT = Path(__file__).resolve().parents[3]


class ExploreBindingsTests(unittest.TestCase):
    def test_qm55_chakoya_entry_preserves_premium_and_once_only_progress(self):
        callback = next(c for c in explore.callbacks(ROOT)
                        if c['quest'] == 'oteryn:quest.authored.chakoya_iceberg_mini_world_change')
        self.assertEqual(callback['admission'], {'premium': True})
        self.assertEqual(callback['selector']['native'], {
            'position': {'x': 32623, 'y': 32584, 'floor': -7},
            'ordinal': 0, 'appearance': 799})
        self.assertTrue(callback['requires_world_admission'])
        visit, counter = callback['storage']
        track = 'oteryn:quest-progress/authored/chakoya_iceberg_mini_world_change/s1'
        self.assertEqual(visit['track'], track + '/visited/entry')
        self.assertEqual((visit['minimum'], visit['maximum'], visit['set']), (0, 0, 1))
        self.assertEqual(counter['track'], track)
        self.assertEqual((counter['minimum'], counter['maximum'], counter['set']), (0, 0, 1))
        self.assertTrue(counter['increment'])
        self.assertEqual(callback['offerings'], [])
        self.assertIsNone(callback['teleport'])

    def test_non_explore_and_unknown_stages_are_rejected(self):
        for stage in ('s2', 'missing'):
            binding = tuple((q, stage, a, t) for q, _, a, t in explore.BINDINGS)
            with self.subTest(stage=stage), mock.patch.object(explore, 'BINDINGS', binding):
                with self.assertRaisesRegex(ValueError, 'one EXPLORE stage'):
                    explore.callbacks(ROOT)

    def test_counter_owner_must_match_the_canonical_quest(self):
        binding = tuple((q, s, a, 'chosen-source/foreign')
                        for q, s, a, _ in explore.BINDINGS)
        with mock.patch.object(explore, 'BINDINGS', binding):
            with self.assertRaisesRegex(ValueError, 'counter owner'):
                explore.callbacks(ROOT)

    def test_unimplemented_required_guard_is_not_silently_dropped(self):
        guarded = (('oteryn:quest.fury_gates_mini_world_change', 's1',
                    'oteryn:area.hunting_place.fury_gate', 'chosen-source/fury_gates_mini_world_change'),)
        with mock.patch.object(explore, 'BINDINGS', guarded):
            with self.assertRaisesRegex(ValueError, 'additional authoritative guards'):
                explore.callbacks(ROOT)

    def test_declared_area_point_really_has_the_retained_native_ground(self):
        sys.path.insert(0, str(ROOT / 'tools/content-schema/world-authoring'))
        import world_region_codec as codec
        index = json.loads((ROOT / 'content/world/placements/index.json').read_text())
        for callback in explore.callbacks(ROOT):
            with self.subTest(quest=callback['quest']):
                native = callback['selector']['native']
                at = native['position']
                z, rx, ry = -at['floor'], at['x'] // 256, at['y'] // 256
                source = ROOT / f'content/world/placements/region-z{z:02}-x{rx}-y{ry}.b3'
                raw = source.read_bytes()
                floor, region_x, region_y, table = codec.parse_region(raw)
                self.assertEqual((floor, region_x, region_y), (z, rx, ry))
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
                # The retained Chakoya tile has a ground root plus its ice overlay.
                expected = ([799, 7152] if callback['quest'] ==
                            'oteryn:quest.authored.chakoya_iceberg_mini_world_change'
                            else [native['appearance']])
                self.assertEqual(found, expected)
                self.assertEqual(found[native['ordinal']], native['appearance'])

    def test_geomancer_uses_its_exact_canonical_area_and_counter(self):
        callback = next(c for c in explore.callbacks(ROOT)
                        if c['quest'] == 'oteryn:quest.geomancer_quest')
        self.assertEqual(callback['key'], 'oteryn:quest-callback/m01b/geomancer/s1/entry')
        self.assertEqual(callback['selector']['native'], {
            'position': {'x': 32418, 'y': 32017, 'floor': -7},
            'ordinal': 0, 'appearance': 4529})
        self.assertIn('https://tibia.fandom.com/index.php?curid=20335&oldid=1114489',
                      callback['sources'])
        self.assertEqual(callback['storage'][1]['track'],
                         'oteryn:quest-progress/chosen-source/geomancer_quest/s1')

    def test_native_anchor_has_no_value_or_movement_effect(self):
        callback = explore.callbacks(ROOT)[0]
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
