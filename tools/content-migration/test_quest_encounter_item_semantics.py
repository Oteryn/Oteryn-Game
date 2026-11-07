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

    def test_exact_devil_corpse_chains_and_complete_loot_closure(self):
        definitions = {d['identity']['key']: d for d in self.definitions()}
        chains = [(5973,12,10000,4025),(4025,10,300000,4026),
                  (4026,10,300000,4027),(4027,8,60000,None),
                  (5983,10,10000,4057),(4057,10,300000,4058),
                  (4058,10,300000,4013),(4013,10,60000,None)]
        for item,capacity,duration,target in chains:
            with self.subTest(item=item):
                d = definitions[f'oteryn:item.tibia.i{item}']
                self.assertTrue(d['materializable'])
                self.assertEqual(d['stack_class'],'NonStackable')
                self.assertEqual(d['semantics']['container'], {'state':'KNOWN','value':{'capacity':{'state':'KNOWN','value':capacity}}} if capacity else {'state':'NOT_APPLICABLE'})
                temporal=d['semantics']['temporal']['value']
                self.assertEqual(temporal['duration']['value'],duration)
                self.assertEqual(temporal['decay_target'],{'state':'KNOWN','value':{'key':f'oteryn:item.tibia.i{target}','revision':'definition-r1'}} if target else {'state':'NOT_APPLICABLE'})
        for creature,count in [('dragon',22),('minotaur_guard',11)]:
            loot = next(r['definition'] for p in (encounter.ROOT/'content/loot').glob('*.json')
                        for r in json.loads(p.read_bytes()).get('records',[])
                        if r.get('definition',{}).get('identity',{}).get('key') == f'oteryn:loot.creature.{creature}')
            self.assertEqual(len(loot['entries']),count)
            self.assertTrue(all(definitions[e['item']['key']]['materializable'] for e in loot['entries']))

    def test_troll_chain_loot_and_exact_preservation_of_prior_encounters(self):
        import hashlib
        definitions={d['identity']['key']:d for d in self.definitions()}
        for item,capacity,duration,target in [(5960,7,10000,3987),(3987,10,300000,3991),
                                              (3991,10,300000,3992),(3992,None,60000,None)]:
            d=definitions[f'oteryn:item.tibia.i{item}']
            self.assertTrue(d['materializable'])
            self.assertEqual(d['semantics']['container'],{'state':'KNOWN','value':{'capacity':{'state':'KNOWN','value':capacity}}} if capacity else {'state':'NOT_APPLICABLE'})
            self.assertEqual(d['semantics']['temporal']['value']['duration']['value'],duration)
            self.assertEqual(d['semantics']['temporal']['value']['decay_target'],{'state':'KNOWN','value':{'key':f'oteryn:item.tibia.i{target}','revision':'definition-r1'}} if target else {'state':'NOT_APPLICABLE'})
        loot=next(r['definition'] for p in (encounter.ROOT/'content/loot').glob('*.json')
                  for r in json.loads(p.read_bytes()).get('records',[])
                  if r.get('definition',{}).get('identity',{}).get('key')=='oteryn:loot.creature.troll')
        self.assertEqual(len(loot['entries']),12)
        self.assertTrue(all(definitions[e['item']['key']]['materializable'] for e in loot['entries']))
        capture=json.loads((encounter.ROOT/encounter.PATH).read_bytes())['records']
        added={3277,3336,3355,3987,3991,3992,5960,9689}
        prior=[r for r in capture if int(r['production_binding']['external_id']) not in added]
        self.assertEqual(len(prior),37)
        self.assertEqual(hashlib.sha256(json.dumps(prior,sort_keys=True,separators=(',',':')).encode()).hexdigest(),'8e335341e591aa3b6f752f9738768c0bd166f7a63532a51b99e0ef38517b07b5')

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

    def assert_known(self, original, actual):
        if isinstance(original,dict):
            if original.get('state') == 'UNKNOWN':
                return
            self.assertIsInstance(actual,dict)
            for key,value in original.items():
                self.assertIn(key,actual)
                self.assert_known(value,actual[key])
        elif isinstance(original,list):
            self.assertEqual(len(original),len(actual))
            for expected,observed in zip(original,actual):
                self.assert_known(expected,observed)
        else:
            self.assertEqual(original,actual)

    def test_every_previously_known_canonical_fact_is_retained(self):
        legacy=json.loads((encounter.ROOT/'content/world/definitions/reference.json').read_bytes())
        original={d['identity']['key']:d for d in legacy['records']}
        actual={d['identity']['key']:d for d in self.definitions()}
        for key in encounter.profiles():
            with self.subTest(item=key):
                self.assert_known(original[key].get('semantics',{}),actual[key].get('semantics',{}))

    def test_canonical_admission_retains_existing_semantic_group_owners(self):
        original = encounter.legacy_definitions()
        actual = {d['identity']['key']:d for d in self.definitions()}
        materialization = {'physical','stack','container','temporal'}
        for key in encounter.profiles():
            for group in encounter.profiles()[key]['semantics']:
                previous = original[key].get('semantics',{}).get(group,{'state':'UNKNOWN'})
                if group not in materialization or previous.get('state') != 'UNKNOWN':
                    with self.subTest(item=key,group=group):
                        self.assertEqual(actual[key]['semantics'][group],previous)

    def test_canonical_stack_maximum_hold_is_not_promoted_by_native_admission(self):
        original = encounter.legacy_definitions()
        actual = {d['identity']['key']:d for d in self.definitions()}
        for key,profile in encounter.profiles().items():
            prior = original[key].get('semantics',{}).get('stack',{}).get('value',{}).get('stack_max',{'state':'UNKNOWN'})
            if profile['semantics']['stack']['value']['stackable'] == {'state':'KNOWN','value':True} and prior.get('state') == 'UNKNOWN':
                self.assertEqual(actual[key]['semantics']['stack']['value']['stack_max'],{'state':'UNKNOWN'})
                self.assertEqual(profile['semantics']['stack']['value']['stack_max'],{'state':'KNOWN','value':100})

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
        selected = set(encounter.profiles())
        for row in before:
            key = row['production_definition']['production_key']
            if key not in selected:
                self.assertEqual(existing[key], row)
            else:
                self.assertEqual({k:v for k,v in existing[key].items() if k!='semantics'},
                                 {k:v for k,v in row.items() if k!='semantics'})
                self.assert_known(row['semantics'],existing[key]['semantics'])
        for item,slots in [(3366,2),(3414,1),(7382,3)]:
            self.assertEqual(existing[f'oteryn:item.tibia.i{item}']['semantics']['imbuement']['value']['slot_count'],
                             {'state':'KNOWN','value':slots})

if __name__ == '__main__':
    unittest.main()
