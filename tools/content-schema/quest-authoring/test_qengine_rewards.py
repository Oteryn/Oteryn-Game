"""Offline donor identity, schema and deterministic materialization checks."""
import copy
import json
import unittest
from jsonschema import Draft202012Validator
from jsonschema.exceptions import ValidationError
import qengine_rewards as q


class QuestRewards(unittest.TestCase):
    def test_inventory_predicates_reject_ambiguous_and_invalid_bounds(self):
        source = next(c for c in self.callbacks['callbacks'] if c.get('inventory'))
        for mutation in ('duplicate', 'inverted', 'negative', 'overflow', 'boolean'):
            bad = copy.deepcopy(source)
            if mutation == 'duplicate':
                bad['inventory'].append(copy.deepcopy(bad['inventory'][0]))
            elif mutation == 'inverted':
                bad['inventory'][0].update(minimum=2, maximum=1)
            elif mutation == 'negative':
                bad['inventory'][0]['minimum'] = -1
            elif mutation == 'overflow':
                bad['inventory'][0]['maximum'] = 4294967296
            else:
                bad['inventory'][0]['minimum'] = True
            with self.assertRaises((ValueError, ValidationError), msg=mutation):
                q.validate_callbacks({'schema': 1, 'callbacks': [bad]})

    def test_exact_conditional_storage_rejects_overloaded_or_out_of_range_guards(self):
        document=copy.deepcopy(self.callbacks)
        callback=document['callbacks'][0]
        callback['storage'].append({
            'track':'oteryn:quest-progress/conditional-reward/flag',
            'minimum':-1, 'maximum':2, 'set':0, 'set_if':1})
        q.validate_callbacks(document)
        for patch in [{'set_if':3}, {'set_if':True}, {'preserve':True}]:
            invalid=copy.deepcopy(document)
            invalid['callbacks'][0]['storage'][-1].update(patch)
            with self.assertRaises((ValueError, ValidationError)):
                q.validate_callbacks(invalid)

    def test_shared_participant_tracks_require_their_declared_owner(self):
        document=copy.deepcopy(self.callbacks)
        declaration=next(d for d in document['tracks'] if d.get('writable'))
        declaration['quest']='oteryn:quest.wrong_owner'
        with self.assertRaises(ValueError):
            q.validate_callbacks(document)

    def test_shared_operations_reject_duplicate_atomic_tracks(self):
        document=copy.deepcopy(self.callbacks)
        callback=next(c for c in document['callbacks'] if c.get('state_ops'))
        callback['state_ops'].append(copy.deepcopy(callback['state_ops'][0]))
        with self.assertRaises(ValueError):
            q.validate_callbacks(document)

    def test_vampire_source_tiers_preserve_exact_costs_xp_and_final_outputs(self):
        tiers=[c for c in self.callbacks['callbacks'] if '/l2-vampire-tier-' in c['key']]
        self.assertEqual([c['take'][0]['count'] for c in tiers],[1,4,5,10,30,50])
        self.assertEqual([c['experience'] for c in tiers],[1000,5000,10000,20000,50000,100000])
        self.assertTrue(all(c['requires'][0]['minimum']==11 for c in tiers))
        self.assertTrue(all(c['give'] is None and c['achievement'] is None for c in tiers[:-1]))
        self.assertEqual(tiers[-1]['give']['item'],'oteryn:item.tibia.i8191')
        self.assertEqual(tiers[-1]['achievement'],'oteryn:achievement/razing')
        self.assertEqual(tiers[-1]['storage'][-1]['set'],1)

    def test_carried_use_rejects_wrong_input_and_duplicate_ownership_guards(self):
        original=next(c for c in self.callbacks['callbacks'] if c.get('carried_item'))
        for patch in [{'take':[]},{'take':[{'item':original['carried_item'],'count':2}]},{'fee':{'gold':1,'include_bank':False}},{'forbids_outfits':['oteryn:outfit.brotherhood','oteryn:outfit.brotherhood']}]:
            document=copy.deepcopy(self.callbacks)
            row=next(c for c in document['callbacks'] if c['key']==original['key'])
            row.update(patch)
            with self.assertRaises((ValueError, ValidationError)):
                q.validate_callbacks(document)

    @classmethod
    def setUpClass(cls):
        cls.outputs=q.build()
        cls.callbacks=json.loads(cls.outputs['content/quests/shared-reward-callbacks.json'])
        cls.author=json.loads(cls.outputs['content/cosmetics/outfits/authoring.json'])
        cls.schema=json.loads((q.ROOT/'tools/content-schema/quest-authoring/shared-reward-callbacks.schema.json').read_text())

    def test_peer_outputs_reject_mixed_layouts_and_omitted_deadlines(self):
        row = copy.deepcopy(next(c for c in self.callbacks['callbacks']
                                 if c['give'] and not c['give'].get('decay')))
        row['give']['contents'] = []
        row['give']['backpack_outputs'] = [{'item': 'oteryn:item.tibia.i3035', 'count': 10}]
        q.validate_callbacks({'schema': 1, 'callbacks': [row]})
        for update in [
            {'contents': [{'item': 'oteryn:item.tibia.i3035', 'count': 1}]},
            {'backpack_outputs': [{'item': 'oteryn:item.tibia.i3035', 'count': 101}]},
            {'backpack_outputs': [{'item': 'oteryn:item.tibia.i3035', 'count': 1}] * 20},
            {'decay': {'duration_millis': 36000000, 'target': None}},
        ]:
            bad = copy.deepcopy(row)
            bad['give'].update(update)
            with self.assertRaises((ValueError, ValidationError)):
                q.validate_callbacks({'schema': 1, 'callbacks': [bad]})

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

    def test_readonly_prerequisites_reject_overlap_duplicates_and_inverted_bounds(self):
        source=next(c for c in self.callbacks['callbacks'] if c.get('requires'))
        for mutation in ('overlap','duplicate','inverted'):
            bad=copy.deepcopy(self.callbacks)
            callback=next(c for c in bad['callbacks'] if c['key']==source['key'])
            if mutation=='overlap':
                callback['requires'][0]['track']=callback['storage'][0]['track']
            elif mutation=='duplicate':
                callback['requires'].append(copy.deepcopy(callback['requires'][0]))
            else:
                callback['requires'][0]['maximum']=callback['requires'][0]['minimum']-1
            with self.assertRaises(ValueError,msg=mutation):
                q.validate_callbacks(bad)

    def test_declared_parent_track_is_readonly_used_and_has_valid_initial(self):
        for mutation in ('duplicate','inverted','unused','written'):
            bad=copy.deepcopy(self.callbacks)
            track=bad['tracks'][0]
            if mutation=='duplicate': bad['tracks'].append(copy.deepcopy(track))
            elif mutation=='inverted':track['initial']=track['max']+1
            elif mutation=='unused':track['key']='oteryn:unused'
            else:
                source=next(c for c in bad['callbacks'] if any(r['track']==track['key'] for r in c.get('requires',[])))
                other=next(c for c in bad['callbacks'] if c['key']!=source['key'] and not c.get('requires'))
                other['storage'][0]['track']=track['key']
            with self.assertRaises(ValueError,msg=mutation): q.validate_callbacks(bad)

    def test_glooth_source_elif_awards_second_addon_first_at_six_votes(self):
        callbacks={c['key'].split('/')[-1]:c for c in self.callbacks['callbacks']}
        low=callbacks['l3-glooth-engineer-first-low']
        high=callbacks['l3-glooth-engineer-first-high']
        second=callbacks['l3-glooth-engineer-second']
        self.assertEqual((low['requires'][1]['minimum'],low['requires'][1]['maximum']),(3,5))
        self.assertEqual(high['requires'][-1]['track'].split('/')[-1],'addon2')
        self.assertEqual((high['requires'][-1]['minimum'],high['requires'][-1]['maximum']),(1,1))
        self.assertEqual(second['requires'][1]['minimum'],6)
        self.assertEqual(second['outfit']['addons'],2)

    def test_monetary_source_price_disallows_zero_or_double_coin_consumption(self):
        source=next(c for c in self.callbacks['callbacks']
                    if c['key']=='oteryn:quest-reward/l4-nobleman-first')
        for mutation in ('zero','double'):
            bad=copy.deepcopy(self.callbacks)
            callback=next(c for c in bad['callbacks'] if c['key']==source['key'])
            if mutation=='zero':callback['fee']['gold']=0
            else:callback['take']=[{'item':'oteryn:item.tibia.i3043','count':15}]
            with self.assertRaises((ValueError,ValidationError)):q.validate_callbacks(bad)
        self.assertEqual(source['fee'],{'gold':150000,'include_bank':False})

    def test_royal_costume_quest_admits_only_its_source_store_tag_exception(self):
        outfits={r['key']:r for r in self.author['document']['outfits']}
        royal=outfits['oteryn:outfit.royal_costume']
        self.assertEqual((royal['female']['id'],royal['male']['id'],royal['addon_count']),(1456,1457,2))
        callbacks={c['key'].split('/')[-1]:c for c in self.callbacks['callbacks']}
        for name,silver,gold,bit in [('base',15000,12500,0),('shield',7500,6250,1),('crown',7500,6250,2)]:
            c=callbacks['l4-royal-costume-'+name]
            self.assertEqual(c['take'],[{'item':'oteryn:item.tibia.i22516','count':silver},{'item':'oteryn:item.tibia.i22721','count':gold}])
            self.assertEqual(c['outfit']['addons'],bit)
            self.assertEqual(c['present'] if 'present' in c else [],[])

    def test_pirate_full_source_stages_retain_the_original_firebird_owner(self):
        callbacks={c['key'].split('/')[-1]:c for c in self.callbacks['callbacks']}
        self.assertNotIn('l4-pirate-sabre',callbacks)
        self.assertEqual(callbacks['pirate-sabre']['storage'][0]['minimum'],4)
        for name,item,before,after in [('eye-patches',6098,1,2),('peg-legs',6126,2,3),('hooks',6097,3,4)]:
            c=callbacks['l4-pirate-'+name]
            self.assertEqual(c['take'],[{'item':f'oteryn:item.tibia.i{item}','count':100}])
            self.assertEqual((c['storage'][0]['minimum'],c['storage'][0]['set']),(before,after))
            self.assertEqual(c['quest'],'oteryn:quest.outfit_and_addon_quests')
        self.assertEqual([r['item'] for r in callbacks['l4-pirate-hat']['take']],
                         ['oteryn:item.tibia.i6101','oteryn:item.tibia.i6102','oteryn:item.tibia.i6100','oteryn:item.tibia.i6099'])

    def test_original_six_and_eight_l1_source_compositions_exist(self):
        callbacks=self.callbacks['callbacks']
        original = {c['key'].split('/')[-1] for c in callbacks
                    if not c['key'].split('/')[-1].startswith(('l1-', 'l2-', 'l3-', 'l4-', 'l6-', 'token-', 'ape-', 'thieves-', 'draconia-', 'exchange-', 'bone-pit-'))
                    and c['key'].split('/')[-1] not in ('sam-kroox', 'outfox-start')}
        self.assertEqual(original, {'pirate-sabre', 'knight-sword', 'deepling-helmet',
                                   'hallowed-holy-water', 'outfox-helmet', 'sam-backpack'})
        self.assertEqual(sum(c['outfit'] is not None for c in callbacks),37)
        # Published QENGINE2 has13 non-outfit branches; L2 adds the pinned
        # Jorge37, Ape6, Thieves7, Kroox1, dragon2 and Quentin/Gamon2 source branches.
        self.assertEqual(sum(c['outfit'] is None for c in callbacks),13 + 37 + 6 + 7 + 1 + 2 + 2 + 10 + 13 + 3 + 2 + 3 + 6 + 1)
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

    def test_draconia_greeting_barter_uses_canonical_npc_and_excludes_only_flag_one(self):
        rows = [c for c in self.callbacks['callbacks'] if c['key'].startswith('oteryn:quest-reward/draconia-fetish-')]
        self.assertEqual(len(rows), 2)
        for row in rows:
            self.assertEqual(row['npc'], 'oteryn:npc.an_old_dragonlord')
            self.assertEqual(row['take'], [{'item': 'oteryn:item.tibia.i3723', 'count': 1}])
            self.assertEqual(row['give'], {'item': 'oteryn:item.tibia.i3206', 'count': 1, 'contents': []})
            self.assertTrue(row['repeatable'])
            self.assertEqual(row['storage'][0]['set'], 1)
            self.assertIn('data-global/npc/an_old_dragon_lord.lua:72:sha256:85016cc0', row['sources'][0])
        for value in [-1, 0, 1, 2, 9223372036854775807]:
            matches = [r for r in rows if r['storage'][0]['minimum'] <= value <= r['storage'][0]['maximum']]
            self.assertEqual(len(matches), 0 if value == 1 else 1)
        self.assertEqual(q.questl2_rewards.source_path(rows[0]['npc']), 'data-global/npc/an_old_dragon_lord.lua')


    def test_source_stake_price_composes_paid_repeat_without_a_token_cost(self):
        row = next(c for c in self.callbacks['callbacks'] if c['key'].endswith('exchange-gamon-stake'))
        self.assertEqual(row['fee'], {'gold': 5000, 'include_bank': True})
        self.assertTrue(row['repeatable'])
        self.assertEqual(row['take'], [])
        self.assertEqual(row['give'], {'item': 'oteryn:item.tibia.i5941', 'count': 1, 'contents': []})
        self.assertTrue(all(s['preserve'] for s in row['storage']))
        q.validate_callbacks({'schema': 1, 'callbacks': [row]})
        for edit in range(5):
            bad = copy.deepcopy(row)
            if edit == 0:
                bad['fee']['gold'] = 0
            elif edit == 1:
                bad['fee']['gold'] = 1000000000001
            elif edit == 2:
                bad['fee']['include_bank'] = 'true'
            elif edit == 3:
                bad['take'] = [{'item': 'oteryn:item.tibia.i3031', 'count': 1}]
            else:
                bad.pop('fee')
            with self.subTest(edit=edit), self.assertRaises((ValueError, ValidationError)):
                q.validate_callbacks({'schema': 1, 'callbacks': [bad]})


if __name__=='__main__':
    unittest.main()
