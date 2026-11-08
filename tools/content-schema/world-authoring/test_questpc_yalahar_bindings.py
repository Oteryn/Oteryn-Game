"""Original Yalahar startup selectors must survive the native map import."""
import json
import unittest
import sys

import quest_access_bindings as access


class YalaharStartupBindings(unittest.TestCase):
    def test_original_item_unique_selectors_are_captured(self):
        capture = json.loads((access.ROOT / access.CAPTURE).read_bytes())
        rows = {row['value']: row for row in capture['bindings']
                if row['source'] == 'data-global/startup/tables/item.lua'}
        expected = {1018, 1019, *range(3071, 3075), *range(3091, 3103)}
        self.assertEqual(set(rows), expected)
        self.assertEqual(rows[3091]['position'], [32743, 31164, 5])
        self.assertEqual(rows[3091]['item'], 8618)
        self.assertEqual(rows[1018]['position'], [32775, 31118, 7])
        self.assertTrue(all(row['attribute'] == 'unique' for row in rows.values()))

    def test_native_source_items_already_carry_every_original_uid(self):
        capture = json.loads((access.ROOT / access.CAPTURE).read_bytes())
        rows = [row for row in capture['bindings']
                if row['source'] == 'data-global/startup/tables/item.lua']
        index = json.loads((access.ROOT / access.INDEX).read_bytes())
        paths = {f"content/world/placements/region-z{z:02}-x{x // 256:03}-y{y // 256:03}.b3"
                 for x, y, z in (row['position'] for row in rows)}
        imported = {}
        for path in sorted(paths):
            raw = (access.ROOT / path).read_bytes()
            overlaid, selected = access.overlay_region(raw, index['palette'], rows)
            self.assertEqual(overlaid, raw, f"native startup selectors missing: {path}")
            imported.update(selected)
        self.assertEqual(len(imported), 18)

    def test_all_original_mechanisms_have_executable_native_bindings(self):
        sys.path.insert(0, str(access.ROOT / 'tools/content-schema/quest-authoring/batch3_a'))
        from capture_world_in_service_of_yalahar import document
        from questpc_yalahar_publication import restored
        callbacks = document()['callbacks']
        self.assertEqual(len(callbacks), 14)
        for row in callbacks:
            self.assertTrue(restored('in_service_of_yalahar', row), row['key'])
            self.assertFalse(restored('another_quest', row))


if __name__ == '__main__':
    unittest.main()
