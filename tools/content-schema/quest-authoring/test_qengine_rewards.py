"""Offline donor identity, schema and deterministic materialization checks."""
import copy
import json
import unittest
from jsonschema import Draft202012Validator
import qengine_rewards as q


class QuestRewards(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.outputs=q.build()
        cls.callbacks=json.loads(cls.outputs['content/quests/shared-reward-callbacks.json'])
        cls.author=json.loads(cls.outputs['content/cosmetics/outfits/authoring.json'])
        cls.schema=json.loads((q.ROOT/'tools/content-schema/quest-authoring/shared-reward-callbacks.schema.json').read_text())

    def test_qengine_rewards_outputs_reproduce_and_validate(self):
        q.validate_callbacks(self.callbacks)
        for path,raw in self.outputs.items():
            self.assertEqual((q.ROOT/path).read_bytes(),raw,path)

    def test_quest_catalogue_pairs_both_sexes_and_explicit_addon_bits(self):
        outfits={r['key']:r for r in self.author['document']['outfits']}
        for name,looks in [('nightmare',(269,268)),('nobleman',(140,132)),('norseman',(252,251)),('oriental',(150,146)),('pirate',(155,151)),('rascoohan',(1372,1371)),('revenant',(1323,1322)),('rift_warrior',(845,846)),('deepling',(464,463))]:
            row=outfits['oteryn:outfit.'+name]
            self.assertEqual((row['female']['id'],row['male']['id']),looks)
            self.assertEqual(row['addon_count'],2)
        mappings=self.author['document']['grant_mappings']
        for row in outfits.values():
            for addon in range(row['addon_count']+1):
                self.assertEqual(sum(m['grant_key']==f"{row['key']}/quest-grant/{addon}" for m in mappings),1)

    def test_cosmetics_revision_matches_the_character_gameplay_content(self):
        gameplay=json.loads(self.outputs['content/spells.manifest.json'])
        self.assertEqual(self.author['document']['content_revision'],
                         gameplay['build_training']['content_revision'])

    def test_unknown_fields_and_invalid_masks_counts_are_rejected(self):
        validator=Draft202012Validator(self.schema)
        for field,value in [('unknown',1),('outfit',{'outfit':'oteryn:outfit.pirate','addons':4}),('take',[{'item':'oteryn:item.tibia.i139','count':0}])]:
            bad=copy.deepcopy(self.callbacks)
            bad['callbacks'][0][field]=value
            self.assertTrue(list(validator.iter_errors(bad)),field)

    def test_repeatable_or_duplicate_value_grants_are_rejected(self):
        for duplicate in (False,True):
            bad=copy.deepcopy(self.callbacks)
            if duplicate:
                bad['callbacks'].append(copy.deepcopy(bad['callbacks'][0]))
            else:
                for storage in bad['callbacks'][0]['storage']:
                    storage['set']=storage['minimum']
            with self.assertRaises(ValueError):
                q.validate_callbacks(bad)

    def test_original_six_and_eight_l1_source_compositions_exist(self):
        callbacks=self.callbacks['callbacks']
        self.assertEqual({c['key'].split('/')[-1] for c in callbacks if not c['key'].split('/')[-1].startswith('l1-')},
                         {'pirate-sabre','knight-sword','deepling-helmet','hallowed-holy-water','outfox-helmet','sam-backpack'})
        self.assertEqual(sum(c['outfit'] is not None for c in callbacks),11)
        self.assertEqual(sum(c['outfit'] is None for c in callbacks),3)
        self.assertTrue(any(len(c['take'])>1 and any(i['count']==100 for i in c['take']) for c in callbacks))
        self.assertTrue(all(q.PIN in c['sources'][0] for c in callbacks))

    def test_l1_handins_preserve_source_crest_identity_and_exclusive_sides(self):
        callbacks={c['key'].split('/')[-1]:c for c in self.callbacks['callbacks']}
        for name,side,bit in [('sage',1,1),('villain',2,2)]:
            c=callbacks['l1-yalaharian-'+name]
            self.assertEqual(c['take'],[])
            self.assertEqual(c['present'],[{'item':'oteryn:item.tibia.i9041','count':1}])
            self.assertEqual(c['outfit']['addons'],bit)
            self.assertTrue(any(s['track'].endswith('/side_decision') and s['minimum']==side and s['maximum']==side and s['preserve'] for s in c['storage']))
        for name,item in [('crest',10199),('tribal',10198)]:
            c=callbacks['l1-warmaster-'+name]
            self.assertEqual(c['take'],[{'item':f'oteryn:item.tibia.i{item}','count':1}])
            self.assertTrue(any(s['track'].endswith('/questline') and s['minimum']==27 and s['preserve'] for s in c['storage']))
        for name,item in [('cape',11701),('hat',11700)]:
            self.assertEqual(callbacks['l1-wayfarer-'+name]['take'],[{'item':f'oteryn:item.tibia.i{item}','count':1}])

    def test_every_exchange_item_has_native_custody_facts(self):
        rows=json.loads((q.ROOT/'content/items/definitions/spell-native-profiles.json').read_bytes())['records']
        native={r['production_definition']['production_key']:r for r in rows}
        for callback in self.callbacks['callbacks']:
            items=callback['take']+callback.get('present',[])+([callback['give']] if callback['give'] else [])
            for item in items:
                self.assertTrue(item['item'] in native,item['item'])
                row=native[item['item']]
                self.assertTrue(row['admission']['materializable'])
                self.assertIn('CharacterInventory',row['admission']['legal_destinations'])

if __name__=='__main__':
    unittest.main()
