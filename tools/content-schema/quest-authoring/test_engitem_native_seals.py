"""The paint routes refer to bare native map decorations, not invented Creatures."""
import json
import sys
import unittest
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'tools/content-schema/world-authoring'))
from world_region_codec import decode_region

class NativeSeals(unittest.TestCase):
    def test_source_seal_stacks_are_bare_and_distinct(self):
        index=json.loads((ROOT/'content/world/placements/index.json').read_bytes())
        _,_,_,sectors=decode_region((ROOT/'content/world/placements/region-z07-x126-y121.b3').read_bytes())
        positions={(32380,31157),(32380,31158),(32381,31158)}
        found=set()
        for _,tiles in sectors:
            for tile in tiles:
                if tile[:2] not in positions: continue
                items=[(i,a) for i,d,a in tile[5] if d==0]
                self.assertEqual(index['palette'][items[2][0]]['source_item_id'],7178)
                self.assertFalse(items[2][1])
                found.add(tile[:2])
        self.assertEqual(found,positions)
