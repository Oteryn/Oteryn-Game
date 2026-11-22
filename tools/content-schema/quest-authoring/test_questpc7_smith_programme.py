"""The selected cache must exist on real clear native terrain, not only in PG geometry."""
import json
from pathlib import Path
import sys
import unittest
import questpc7_smith_programme as smith

ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'tools/content-schema/world-authoring'))
import world_region_codec as codec

class SmithProductionPlacement(unittest.TestCase):
    def test_three_source_boxes_are_the_only_smith_reward_selectors(self):
        index=json.loads((ROOT/'content/world/placements/index.json').read_bytes())
        callbacks,_=smith.build(ROOT)
        rewards = [row for row in callbacks if row.get('effects', {}).get('item_reward')]
        found = {
            (r['selector']['position']['x'], r['selector']['position']['y']):
            (r['effects']['item_reward']['item'], r['effects']['item_reward']['count'])
            for r in rewards
        }
        self.assertEqual(found, {
            (32774, 32253): ('oteryn:item.tibia.i3034', 1),
            (32776, 32253): ('oteryn:item.tibia.i3208', 1),
            (32778, 32253): ('oteryn:item.tibia.i3031', 100),
        })
        cells={(x,y):entries for _,sector in codec.decode_region((ROOT/smith.MAP).read_bytes())[3]
               for x,y,_,_,_,entries in sector}
        for cell in found:
            self.assertIn(2472, [index['palette'][item]['source_item_id']
                                 for item, depth, _ in cells[cell] if depth == 0])
        self.assertFalse(any(r['path'].endswith('authored-questpc7-smith.json')
                             for r in index['authored_props']))

if __name__=='__main__':unittest.main()
