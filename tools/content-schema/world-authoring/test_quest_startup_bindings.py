"""Native selectors must exist on exact source Items, not just PG fixture tiles."""
import json
import unittest
from pathlib import Path

import world_region_codec as codec
import quest_startup_bindings as startup

ROOT = Path(__file__).resolve().parents[3]
EXPECTED = (
    ((32966, 32626, 5), 4634, 'action', 50022),
    ((32968, 32630, 7), 1720, 'action', 50022),
    ((32968, 32629, 7), 1949, 'action', 50022),
    ((32967, 32630, 8), 1949, 'action', 50022),
    ((33190, 31684, 14), 5102, 'action', 1070),
    ((33195, 31684, 14), 5102, 'action', 1070),
    ((32573, 32121, 7), 2772, 'unique', 30034),
)


class QuestStartupBindingsTests(unittest.TestCase):
    def outputs(self):
        index = json.loads((ROOT / startup.doors.INDEX).read_bytes())
        document = json.loads((ROOT / startup.CAPTURE).read_bytes())
        paths = [startup.doors.INDEX.as_posix(), startup.SUMMARY,
                 *(r['path'] for r in startup.affected(index, document))]
        return {p: (ROOT / p).read_bytes() for p in paths}

    def test_startup_overlay_is_idempotent_and_rejects_ambiguous_items(self):
        outputs = self.outputs()
        before = outputs.copy()
        startup.overlay_outputs(outputs, require_all=True)
        self.assertEqual(outputs, before)
        name = 'content/world/placements/region-z07-x127-y125.b3'
        z, rx, ry, sectors = codec.decode_region(outputs[name])
        for _, tiles in sectors:
            for x, y, _, _, _, items in tiles:
                if (x, y) == (32573, 32121):
                    items.extend([entry for entry in items if (entry[2] or {}).get('unique') == 30034])
        outputs[name] = codec.encode_region(z, rx, ry, {
            local: codec.encode_sector_tiles(rx * 8 + local % 8, ry * 8 + local // 8, tiles)
            for local, tiles in sectors})
        with self.assertRaisesRegex(ValueError, 'missing or ambiguous'):
            startup.overlay_outputs(outputs, require_all=True)

    def test_capture_reproduces_the_pinned_donor_values(self):
        retained = ROOT / '.questpc4/evidence/crystal-201189f'
        expected = json.loads((ROOT / startup.CAPTURE).read_bytes())
        self.assertEqual(startup.capture(retained), expected)

    def test_native_startup_selectors_match_exact_source_items(self):
        index = json.loads((ROOT / 'content/world/placements/index.json').read_bytes())
        for (x, y, z), appearance, attribute, expected in EXPECTED:
            with self.subTest(position=(x, y, z)):
                raw = (ROOT / f'content/world/placements/region-z{z:02}-x{x // 256:03}-y{y // 256:03}.b3').read_bytes()
                _, _, _, sectors = codec.decode_region(raw)
                tiles = [t for _, tiles in sectors for t in tiles if t[:2] == (x, y)]
                self.assertEqual(len(tiles), 1)
                self.assertEqual(tiles[0][3], 0)
                items = [attrs for palette, depth, attrs in tiles[0][5]
                         if depth == 0 and index['palette'][palette]['source_item_id'] == appearance]
                self.assertEqual(len(items), 1)
                self.assertEqual((items[0] or {}).get(attribute), expected)


if __name__ == '__main__':
    unittest.main()
