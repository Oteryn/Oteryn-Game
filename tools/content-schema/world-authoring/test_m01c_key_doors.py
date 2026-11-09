"""Exercise the actual M01c locked-door placements before production overlay."""
import json
import unittest

import m01c_key_doors as doors
import quest_door_bindings as base
import world_region_codec as codec


class M01cKeyDoorsTests(unittest.TestCase):
    def test_source_positions_bind_exactly_six_native_locked_doors(self):
        index = json.loads((base.ROOT / base.INDEX).read_bytes())
        capture = json.loads((base.ROOT / doors.CAPTURE).read_bytes())
        count = 0
        for region in index['regions']:
            if not any(region['path'].endswith(doors.region_name(row['position']))
                       for row in capture['bindings']):
                continue
            raw = (base.ROOT / region['path']).read_bytes()
            result, rows = doors.overlay_region(raw, index['palette'], capture)
            count += len(rows)
            self.assertEqual(doors.overlay_region(result, index['palette'], capture)[0], result)
            original = codec.decode_region(raw)
            changed = codec.decode_region(result)
            self.assertEqual(original[:3], changed[:3])
            for (_, before), (_, after) in zip(original[3], changed[3], strict=True):
                for left, right in zip(before, after, strict=True):
                    self.assertEqual(left[:5], right[:5])
                    self.assertEqual(len(left[5]), len(right[5]))
                    for a, b in zip(left[5], right[5], strict=True):
                        self.assertEqual(a[:2], b[:2])
                        attrs = dict(b[2] or {})
                        if a != b:
                            self.assertIn(attrs.pop('action'), (3012, 3520, 5010))
                            self.assertEqual(attrs, a[2] or {})
        self.assertEqual(count, 6)

    def test_missing_or_duplicate_native_target_refuses(self):
        capture = {'bindings': [{'position': [1, 2, 7], 'key_number': 3012, 'item': None}]}
        palette = [{'source_item_id': 5097}, {'source_item_id': 3031}]
        for items in [[(1, 0, None)], [(0, 0, None), (0, 0, None)]]:
            raw = codec.encode_region(7, 0, 0, {0: codec.encode_sector_tiles(
                0, 0, [(1, 2, 0, 0, (), items)])})
            with self.assertRaisesRegex(ValueError, 'M01C_KEY_DOOR_TARGET'):
                doors.overlay_region(raw, palette, capture)

    def test_existing_key_number_conflict_is_never_replaced(self):
        capture = {'bindings': [{'position': [1, 2, 7], 'key_number': 3012, 'item': None}]}
        palette = [{'source_item_id': 5097}]
        raw = codec.encode_region(7, 0, 0, {0: codec.encode_sector_tiles(
            0, 0, [(1, 2, 0, 0, (), [(0, 0, {'action': 3520})])])})
        with self.assertRaisesRegex(ValueError, 'action conflict'):
            doors.overlay_region(raw, palette, capture)
        capture['bindings'][0]['key_number'] = 3013
        with self.assertRaisesRegex(ValueError, 'M01C_KEY_DOOR_NUMBER'):
            doors.overlay_region(raw, palette, capture)


if __name__ == '__main__':
    unittest.main()
