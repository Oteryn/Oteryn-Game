"""Chosen Pits entrance is guarded at its retained native source anchor."""
import json
from pathlib import Path
import sys
import unittest
import qengine_access

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools/content-schema/world-authoring'))
import world_region_codec

class PitsAdmissionTests(unittest.TestCase):
    def test_pits_retains_chosen_level_and_live_premium(self):
        rows = [r for r in qengine_access.document()['callbacks']
                if r['key'] == 'oteryn:quest-callback/qm03/pits']
        self.assertEqual(len(rows), 1)
        row = rows[0]
        self.assertEqual(row['quest'], 'oteryn:quest.the_pits_of_inferno_quest')
        self.assertEqual(row['admission'], {'premium': True})
        self.assertEqual(row['minimum_level'], 80)
        self.assertEqual(row['trigger'], 'step_in')
        self.assertTrue(row['return_on_failure'])
        self.assertTrue(row['requires_world_admission'])
        self.assertIsNone(row['teleport'])
        self.assertEqual(row['storage'][0]['set'], 1)
        self.assertEqual(row['storage'][0]['minimum'], 0)
        self.assertEqual(row['track_initials'], {row['storage'][0]['track']: 0})
        self.assertTrue(row['storage'][0]['at_least'])
        index = json.loads((ROOT/'content/world/placements/index.json').read_text())
        region = world_region_codec.decode_region((ROOT/'content/world/placements/region-z10-x128-y126.b3').read_bytes())
        tiles = [tile for _, sector in region[3] for tile in sector if tile[:2] == (32791,32331)]
        self.assertEqual(len(tiles),1)
        palette, depth, attrs = tiles[0][5][0]
        self.assertEqual(index['palette'][palette]['source_item_id'],354)
        self.assertEqual(depth,0)
        self.assertIsNone(attrs)
        self.assertEqual(row['selector']['native'], {'position':{'x':32791,'y':32331,'floor':-10},'ordinal':0,'appearance':354})

if __name__ == '__main__':
    unittest.main()
