"""Goblin Merchant's contact uses the retained agreeing donor placement."""
import json
import unittest
import sys
from pathlib import Path

import convert_placements as cp


class RafzanPlacementTest(unittest.TestCase):
    def test_source_cell_and_both_source_attestations(self):
        rows = [record['declaration']
                for name, text in cp.convert().items()
                if name.startswith('placements-')
                for record in json.loads(text)['records']
                if record['declaration']['npc'] == 'oteryn:npc.rafzan']
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]['cell'], {'x': 33052, 'y': 32095, 'floor': 6})
        self.assertEqual(rows[0]['direction'], 'north')
        self.assertEqual({p['origin'] for p in rows[0]['provenance']},
                         {'canary', 'crystal'})
        crystal = next(p for p in rows[0]['provenance'] if p['origin'] == 'crystal')
        self.assertEqual(crystal['definition_sha256'],
                         '9b5f6c4439671329c654c4333f73559235d63d7d61059e152de65ac28c066f34')

    def test_retained_native_cell_exists_outside_a_house(self):
        sys.path.insert(0, str(cp.ROOT / 'tools/content-schema/world-authoring'))
        import world_region_codec as codec
        raw = (cp.ROOT / 'content/world/placements/region-z06-x129-y125.b3').read_bytes()
        cells = [(flags, house, entries)
                 for _, sector in codec.decode_region(raw)[3]
                 for x, y, flags, house, _, entries in sector
                 if (x, y) == (33052, 32095)]
        self.assertEqual(len(cells), 1)
        flags, house, entries = cells[0]
        self.assertEqual((flags, house), (0, 0))
        palette = json.loads((cp.ROOT / 'content/world/placements/index.json').read_text())['palette']
        self.assertEqual(palette[entries[0][0]]['key'], 'oteryn:item.tibia.i17518')


if __name__ == '__main__':
    unittest.main()
