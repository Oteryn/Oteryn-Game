"""Encounter admission preserves existing owners and refuses conflicting partial facts."""
import copy
import json
from pathlib import Path
import unittest
import quest_encounter_item_semantics as encounter

class EncounterItemTests(unittest.TestCase):
    def definitions(self):
        return [r['definition'] for p in (encounter.ROOT / 'content/items/definitions').glob('items-*.json')
                for r in json.loads(p.read_bytes())['records']]

    def test_exact_demon_corpse_chain_and_complete_loot_closure(self):
        definitions = {d['identity']['key']: d for d in self.definitions()}
        for item, capacity, duration, target in [(5995,15,10000,4097),(4097,24,300000,4098),
                                                  (4098,24,300000,4099),(4099,None,60000,None)]:
            d = definitions[f'oteryn:item.tibia.i{item}']
            self.assertTrue(d['materializable'])
            self.assertEqual(d['stack_class'], 'NonStackable')
            self.assertEqual(d['semantics']['container'], {'state':'KNOWN','value':{'capacity':{'state':'KNOWN','value':capacity}}}
                             if capacity else {'state':'NOT_APPLICABLE'})
            temporal = d['semantics']['temporal']['value']
            self.assertEqual(temporal['duration']['value'], duration)
            self.assertEqual(temporal['decay_target'], {'state':'KNOWN','value':{'key':f'oteryn:item.tibia.i{target}','revision':'definition-r1'}}
                             if target else {'state':'NOT_APPLICABLE'})
        loot = next(r['definition'] for p in (encounter.ROOT / 'content/loot').glob('*.json')
                    for r in json.loads(p.read_bytes()).get('records', [])
                    if r.get('definition', {}).get('identity', {}).get('key') == 'oteryn:loot.creature.demon')
        self.assertEqual(len(loot['entries']), 34)
        self.assertTrue(all(definitions[e['item']['key']]['materializable'] for e in loot['entries']))

    def test_canonical_projection_is_idempotent_and_preserves_unrelated_items(self):
        definitions = self.definitions()
        before = copy.deepcopy(definitions)
        encounter.apply_admissions(definitions)
        self.assertEqual(definitions, before)
        self.assertTrue(all(p.read_bytes() == raw for p, raw in encounter.outputs().items()))

    def test_conflicting_admitted_fact_refuses_before_any_mutation(self):
        definitions = self.definitions()
        selected = next(d for d in definitions if d['identity']['key'] == 'oteryn:item.tibia.i5995')
        selected['semantics']['physical']['value']['weight']['value'] = 1
        before = copy.deepcopy(definitions)
        with self.assertRaisesRegex(ValueError, 'CANONICAL_CONFLICT'):
            encounter.apply_admissions(definitions)
        self.assertEqual(definitions, before)

    def test_native_projection_appends_only_absent_decay_stages(self):
        path = 'content/items/definitions/spell-native-profiles.json'
        manifest = 'content/spells.manifest.json'
        document = json.loads((encounter.ROOT / path).read_bytes())
        absent = {'oteryn:item.tibia.i4097', 'oteryn:item.tibia.i4099'}
        document['records'] = [r for r in document['records'] if r['production_definition']['production_key'] not in absent]
        before = copy.deepcopy(document['records'])
        canonical = lambda v: (json.dumps(v,sort_keys=True,separators=(',',':'))+'\n').encode()
        generated = {path:canonical(document), manifest:(encounter.ROOT / manifest).read_bytes()}
        encounter.admit(encounter.ROOT, generated, path, manifest, lambda r:r['production_definition']['production_key'], canonical)
        after = json.loads(generated[path])['records']
        self.assertEqual(len(after),len(before)+2)
        existing = {r['production_definition']['production_key']:r for r in after}
        self.assertTrue(all(existing[r['production_definition']['production_key']] == r for r in before))

if __name__ == '__main__':
    unittest.main()
