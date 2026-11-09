"""Exact source non-corpse Item admission and immutable physical semantics."""
import json
from pathlib import Path
import unittest
ROOT = Path(__file__).resolve().parents[3]
IDS = {33793,33877,34005,33950,33854,34006,34007}

class SoulWarSourceItems(unittest.TestCase):
    def test_native_provider_really_contains_seven_required_source_items(self):
        native=json.loads((ROOT/'content/items/definitions/spell-native-profiles.json').read_bytes())
        keys={row['production_definition']['production_key'] for row in native['records']}
        missing={f'oteryn:item.tibia.i{i}' for i in IDS}-keys
        self.assertFalse(missing, f'actual native source Item profiles missing: {sorted(missing)}')

    def test_source_profiles_have_exact_chain_and_no_callback_decay(self):
        doc=json.loads((Path(__file__).with_name('soul-war-source-item-profiles.json')).read_bytes())
        rows={int(row['production_definition']['production_key'].rsplit('i',1)[1]):row for row in doc['records']}
        self.assertEqual(set(rows),IDS)
        for item,row in rows.items():
            self.assertTrue(row['admission']['materializable'])
            self.assertEqual(row['admission']['legal_destinations'],['CharacterInventory','Ground'] if item in {33793,33950} else ['Ground'])
        for item in (33793,33877,34005,33950):
            self.assertEqual(rows[item]['semantics']['temporal'],{'state':'NOT_APPLICABLE'})
        for item,duration,target in ((33854,600000,34006),(34006,300000,34007),(34007,300000,None)):
            value=rows[item]['semantics']['temporal']['value']
            self.assertEqual(value['duration'],{'state':'KNOWN','value':duration})
            self.assertEqual(value['decay_target'],{'state':'KNOWN','value':{'key':f'oteryn:item.tibia.i{target}','revision':'definition-r1'}} if target else {'state':'NOT_APPLICABLE'})

if __name__=='__main__': unittest.main()
