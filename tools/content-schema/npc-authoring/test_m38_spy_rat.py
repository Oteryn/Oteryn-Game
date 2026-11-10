"""The retained Spy Rat needs a real normal-package NPC, not a synthetic test actor."""
import json
import unittest
from pathlib import Path

import convert_placements as cp

KEY='oteryn:npc.eustacios_spy_rat'


class SpyRatContactTest(unittest.TestCase):
    def test_normal_package_contains_the_source_bound_passive_contact(self):
        records=json.loads((cp.ROOT/'content/world/definitions/declarations.json').read_text())['records']
        selected=[r for r in records if r.get('kind')=='NPC' and r['identity']['key']==KEY]
        self.assertEqual(len(selected),1,'Spy Rat is missing from the real normal package')
        npc=selected[0]
        self.assertIsNone(npc['dialogue'])
        self.assertEqual(npc['services'],[])
        self.assertEqual(npc['behavior']['key'],'oteryn:behavior.npc.a_swan')
        self.assertEqual(npc['presentation']['key'],'oteryn:presentation.npc.rata_mari')
        sources=json.loads((cp.ROOT/'content/world/provenance/sources.json').read_text())
        binding=[b for b in sources['source_identity_bindings'] if b['target']['family']=='NPC' and b['target']['key']==KEY]
        self.assertEqual(len(binding),1)
        self.assertEqual(binding[0]['external_id'],'50622')
        self.assertEqual(binding[0]['source_revision'],'tibiawiki-br-npc-0773232ddd356be2')

    def test_retained_spy_rat_cell_is_published_without_other_authority(self):
        rows=[r['declaration'] for name,text in cp.convert().items() if name.startswith('placements-')
              for r in json.loads(text)['records'] if r['declaration']['npc']==KEY]
        self.assertEqual([r['cell'] for r in rows],[{'x':33857,'y':31333,'floor':9}])
        self.assertFalse(rows[0].get('block_solid_contact',False))
        self.assertFalse(rows[0].get('non_walkable_contact',False))
