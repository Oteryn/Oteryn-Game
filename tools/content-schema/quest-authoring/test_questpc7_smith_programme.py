"""The selected cache must exist on real clear native terrain, not only in PG geometry."""
import hashlib
import json
from pathlib import Path
import sys
import unittest
import questpc7_smith_programme as smith

ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'tools/content-schema/world-authoring'))
import world_region_codec as codec

class SmithProductionPlacement(unittest.TestCase):
    def test_production_prop_is_pinned_and_matches_runtime_reward_selector(self):
        index='content/world/placements/index.json'
        generated={index:(ROOT/index).read_bytes()}
        smith.placements(ROOT,generated)
        path='content/world/placements/authored-questpc7-smith.json'
        descriptor=next(r for r in json.loads(generated[index])['authored_props'] if r['path']==path)
        self.assertEqual(descriptor['sha256'],hashlib.sha256(generated[path]).hexdigest())
        prop=json.loads(generated[path])['props'][0]
        callbacks,_=smith.build(ROOT)
        self.assertEqual(prop['position'],callbacks[1]['selector']['position'])
        self.assertEqual(prop['item'],'oteryn:item.tibia.i2472')
        cells={(x,y):entries for _,sector in codec.decode_region((ROOT/smith.MAP).read_bytes())[3] for x,y,_,_,_,entries in sector}
        for cell in ((32775,32254),(32776,32254),(32777,32254)):
            self.assertEqual(cells[cell],[(173,0,None)],'guardian, entrance and authored chest need real clear source cells')

if __name__=='__main__':unittest.main()
