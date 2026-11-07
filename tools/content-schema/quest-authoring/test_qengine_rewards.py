"""Offline donor identity, schema and deterministic materialization checks."""
import copy
import json
import unittest
from jsonschema import Draft202012Validator
from jsonschema.exceptions import ValidationError
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

    def test_original_fourteen_and_present_source_compositions_exist(self):
        callbacks=[c for c in self.callbacks['callbacks'] if not c['key'].startswith('oteryn:quest-reward/newhaven-')]
        self.assertEqual({c['key'].split('/')[-1] for c in callbacks if not c['key'].split('/')[-1].startswith('l1-')},
                         {'pirate-sabre','knight-sword','deepling-helmet','hallowed-holy-water','outfox-helmet','sam-backpack','present-box'})
        self.assertEqual(sum(c['outfit'] is not None for c in callbacks),11)
        self.assertEqual(sum(c['outfit'] is None for c in callbacks),4)
        self.assertTrue(any(len(c['take'])>1 and any(i['count']==100 for i in c['take']) for c in callbacks))
        self.assertTrue(all(q.PIN in c['sources'][0] for c in callbacks))

    def test_newhaven_is_exactly_four_additive_selected_recipe_callbacks(self):
        import hashlib
        old=[c for c in self.callbacks['callbacks'] if not c['key'].startswith('oteryn:quest-reward/newhaven-')]
        self.assertEqual(hashlib.sha256(json.dumps(old,sort_keys=True,separators=(',',':')).encode()).hexdigest(),
                         '0d6f65810f316f88810dfc6d67ed7b7f2d8c456657215f5f2afe49dc1a8d0e5a')
        new={c['key'].split('/')[-1]:c for c in self.callbacks['callbacks'] if c['key'].startswith('oteryn:quest-reward/newhaven-')}
        self.assertEqual(set(new),{'newhaven-welcome','newhaven-training-ready','newhaven-guardian','newhaven-wisdom'})
        self.assertTrue(all(c['minimum_level']==2 and c['npc']=='oteryn:npc.gustavo_the_guard' for c in new.values()))
        self.assertTrue(all(c['sources'][0].startswith('oteryn:selected-recipe:') for c in new.values()))
        self.assertEqual(sum(c.get('experience',0) for c in new.values()),1600)
        self.assertEqual(new['newhaven-guardian']['give'],{'item':'oteryn:item.tibia.i52732','count':1})
        self.assertEqual(new['newhaven-wisdom']['give'],{'item':'oteryn:item.tibia.i3572','count':1})
        self.assertEqual(new['newhaven-wisdom']['take'],[{'item':'oteryn:item.tibia.i51977','count':1}])
        self.assertTrue(new['newhaven-wisdom']['completes'])
        for field,values in [('experience',[0,-1,100000001]),('minimum_level',[0,65536])]:
            for value in values:
                bad=copy.deepcopy(self.callbacks)
                bad['callbacks'][-1][field]=value
                with self.assertRaises(ValidationError):
                    q.validate_callbacks(bad)

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

    def test_palimuth_base_completion_does_not_require_his_side(self):
        callbacks={c['key'].split('/')[-1]:c for c in self.callbacks['callbacks']}
        palimuth=callbacks['l1-yalaharian-base-palimuth']
        self.assertFalse(any(s['track'].endswith('/side_decision') for s in palimuth['storage']))
        yalahari=callbacks['l1-yalaharian-base-yalahari']
        self.assertTrue(any(s['track'].endswith('/side_decision') and s['minimum']==2
                            and s['maximum']==2 and s['preserve'] for s in yalahari['storage']))
        for callback in (palimuth,yalahari):
            for suffix,maximum in (('/door_to_reward',1),('/mission10',4)):
                state=next(s for s in callback['storage'] if s['track'].endswith(suffix))
                self.assertEqual((state['minimum'],state['maximum'],state['set']),(-1,maximum,maximum))

    def test_wardrobe_rewards_use_exact_native_source_objects_and_once_guards(self):
        import quest_batch4
        worlds=json.loads(self.outputs['content/quests/shared-outfit-callbacks.json'])['callbacks']
        self.assertEqual(len(worlds),2)
        for world,expected in zip(worlds,[(33053,31020,-7,5862,'warmaster'),(33073,31173,-8,11431,'wayfarer')]):
            x,y,z,item,outfit=expected
            self.assertEqual(world['selector'],{'position':{'x':x,'y':y,'floor':z},'unique':None,'action':None,'appearance':item})
            self.assertEqual(world['effects']['outfit_reward'],{'outfit':'oteryn:outfit.'+outfit,'addons':0})
            import sys
            sys.path.insert(0,str(q.ROOT/'tools/content-schema/world-authoring'))
            import world_region_codec as codec
            index=json.loads((q.ROOT/'content/world/placements/index.json').read_bytes())
            region=q.ROOT/f'content/world/placements/region-z{-z:02}-x{x//256:03}-y{y//256:03}.b3'
            matches=[attrs for _,tiles in codec.decode_region(region.read_bytes())[3]
                     for tx,ty,_,_,_,items in tiles if (tx,ty)==(x,y)
                     for palette,depth,attrs in items
                     if depth==0 and index['palette'][palette]['source_item_id']==item]
            self.assertEqual(len(matches),1,'exact native wardrobe target')
            quest_batch4.provenance(world['sources'],world['key'],outfit=True)
            quest_batch4.validate_effects(world,world['key'])
            with self.assertRaises(ValueError):quest_batch4.provenance(world['sources'],world['key'])
            bad=copy.deepcopy(world)
            for state in bad['storage']:state['set']=state['minimum']
            with self.assertRaises(ValueError):quest_batch4.validate_effects(bad,bad['key'])

    def test_accepted_legacy_base_entitlements_keep_their_exact_mapping_and_provenance(self):
        mappings={r['grant_key']:r for r in self.author['document']['grant_mappings']}
        for path,outfit,provenance in [
            ('soul-war/revenant-base','revenant','crystal-201189f-soul-war-revenant-v1'),
            ('rotten-blood/decaying-defender-base','decaying_defender','owner-pr2784-decaying-defender-base-v1')]:
            row=mappings['oteryn:appearance-grant/quest/'+path]
            self.assertEqual(row['target'],{'kind':'Outfit','outfit_key':'oteryn:outfit.'+outfit})
            self.assertEqual(row['compatible_provenance'],[provenance])
        promotion=next(p for p in self.author['promotions'] if p['path']=='/grant_mappings')
        self.assertEqual(promotion['source_kind'],'OWNER_DECISION')

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
