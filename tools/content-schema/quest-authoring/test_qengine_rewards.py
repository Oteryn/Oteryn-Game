"""Offline donor identity, schema and deterministic materialization checks."""
import copy
import json
import unittest
from jsonschema import Draft202012Validator, ValidationError
import qengine_rewards as q


class QuestRewards(unittest.TestCase):
    def test_royal_costume_completes_only_after_crown_exchange(self):
        rows = {row['key'].split('/')[-1]: row for row in self.callbacks['callbacks']}
        for name, before, after in [('base', -1, 1), ('shield', 1, 2), ('crown', 2, 3)]:
            row = rows['l4-royal-costume-' + name]
            with self.subTest(exchange=name):
                self.assertEqual(row.get('completes', False), name == 'crown')
                self.assertEqual(row['storage'][0]['minimum'], before)
                self.assertEqual(row['storage'][0]['set'], after)

    def test_literal_reward_text_is_bounded_and_leaf_only(self):
        document=copy.deepcopy(self.callbacks)
        reward={'item':'oteryn:item.tibia.i2814','count':1,'contents':[],
                'text':'Literal café.\n'}
        document['callbacks'][0]['give']=reward
        q.validate_callbacks(document)
        for body in ['x'*4000,'é'*3998]:
            reward['text']=body
            q.validate_callbacks(document)
        for body in ['', 'bad\0text', 'x'*4001, 'é'*4000]:
            reward['text']=body
            with self.subTest(body_length=len(body)), self.assertRaises((ValidationError,ValueError)):
                q.validate_callbacks(document)
        reward['text']='Text'
        reward['count']=2
        with self.assertRaises(ValidationError):
            q.validate_callbacks(document)
        reward['count']=1
        reward['contents']=[{'item':'oteryn:item.tibia.i3031','count':1}]
        with self.assertRaises(ValidationError):
            q.validate_callbacks(document)

    def test_atomic_primary_storage_capacity_is_sixteen(self):
        document=copy.deepcopy(self.callbacks)
        document.pop('tracks',None)
        row=document['callbacks'][0]
        row.pop('participants',None)
        row.pop('state_ops',None)
        row['storage']=[{'track':f'oteryn:quest-progress/atomic/{i}','minimum':0,'maximum':0,'set':1} for i in range(16)]
        document['callbacks']=[row]
        q.validate_callbacks(document)
        row['storage'].append({'track':'oteryn:quest-progress/atomic/16','minimum':0,'maximum':0,'set':1})
        with self.assertRaises((ValueError,ValidationError)):
            q.validate_callbacks(document)

    def test_native_trial_bindings_require_full_goals_and_one_shot_grants(self):
        document=json.loads(q.build()['content/quests/shared-reward-callbacks.json'])
        rows={c['key'].split('/')[-1]:c for c in document['callbacks']}
        for label,goals in [('dragon-slayer',[20,20]),('cave-explorer',[20]),('fire-fighter',[10])]:
            start=rows[f'l12-{label}-start'];base=rows[f'l12-{label}-base']
            self.assertEqual(start['storage'][0]['minimum'],0)
            self.assertEqual(start['storage'][0]['set'],1)
            self.assertEqual([s['minimum'] for s in base['storage'][1:]],goals)
            self.assertEqual(base['storage'][0]['set'],2)
            self.assertTrue(base['requires_premium'])
            self.assertEqual(base['forbids_outfits'],[base['outfit']['outfit']])
        self.assertEqual([rows[f'l12-cave-explorer-{stage}']['fee']['gold'] for stage in ['base','first','second']],[1000,2000,3000])
        appearance=json.loads(q.build()['content/cosmetics/outfits/authoring.json'])['document']
        dragon=next(o for o in appearance['outfits'] if o['key']=='oteryn:outfit.dragon_slayer')
        self.assertEqual((dragon['female']['id'],dragon['male']['id']),(1289,1288))
        self.assertTrue(any(m['grant_key']=='oteryn:outfit.dragon_slayer/quest-grant/0' for m in appearance['grant_mappings']))

    @classmethod
    def setUpClass(cls):
        cls.outputs=q.build()
        cls.callbacks=json.loads(cls.outputs['content/quests/shared-reward-callbacks.json'])
        cls.author=json.loads(cls.outputs['content/cosmetics/outfits/authoring.json'])
        cls.schema=json.loads((q.ROOT/'tools/content-schema/quest-authoring/shared-reward-callbacks.schema.json').read_text())

    def test_l11_levels_one_shot_bases_and_selected_desks(self):
        rows={c['key'].rsplit('/',1)[-1]:c for c in self.callbacks['callbacks']}
        for label,level in [('dream-warrior',250),('issavi',250),('makeshift',100)]:
            for stage in ['base','first','second']:
                row=rows[f'l11-{label}-{stage}'];self.assertEqual(row['minimum_level'],level);self.assertTrue(row['requires_premium'])
            if label!='makeshift':
                row=rows[f'l11-{label}-base'];self.assertEqual(row['storage'],[]);self.assertEqual(row['forbids_outfits'],[row['outfit']['outfit']])
            self.assertEqual(rows[f'l11-{label}-second']['requires_addons'][0]['addons'],1)
        for stage,item,count in [('first',30169,5),('second',30168,1)]:
            self.assertEqual(rows[f'l11-dream-warrior-{stage}']['take'],[{'item':f'oteryn:item.tibia.i{item}','count':count}])
        for field,value in [('minimum_level',0),('forbids_outfits',[]),('forbids_outfits',['oteryn:outfit.knight'])]:
            bad=copy.deepcopy(self.callbacks)
            next(c for c in bad['callbacks'] if c['key']=='oteryn:quest-reward/l11-dream-warrior-base')[field]=value
            with self.assertRaises((ValueError,ValidationError)):q.validate_callbacks(bad)

    def test_l10_distinct_order_documents_and_exact_carried_binding(self):
        rows={c['key'].rsplit('/',1)[-1]:c for c in self.callbacks['callbacks']}
        for family in ['brotherhood','nightmare']:
            for stage,item,price in [('join',768,0),('officer',769,500),('leader',770,1000)]:
                row=rows[f'l10-{family}-{stage}']
                self.assertEqual(row['give']['item'],f'oteryn:item.tibia.i{item}')
                self.assertEqual(row['take'],[] if price==0 else [{'item':'oteryn:item.tibia.i6499','count':price}])
                self.assertEqual(row['storage'][0]['set'],{'join':1,'officer':2,'leader':3}[stage])
            for stage,item,bit in [('follower',768,0),('officer',769,1),('leader',770,2)]:
                row=rows[f'l10-{family}-use-{stage}']
                self.assertEqual(row['carried_item'],f'oteryn:item.tibia.i{item}')
                self.assertEqual(row['take'],[{'item':row['carried_item'],'count':1}])
                self.assertEqual(row['outfit'],{'outfit':'oteryn:outfit.'+family,'addons':bit})
                self.assertTrue(row['requires_premium'])
                self.assertEqual(row['forbids_outfits'] if bit==0 else row['requires_outfits'],['oteryn:outfit.'+family])
        for field,value in [('carried_item','oteryn:item.tibia.i769'),('take',[]),('give',{'item':'oteryn:item.tibia.i768','count':1,'contents':[]})]:
            bad=copy.deepcopy(self.callbacks)
            next(c for c in bad['callbacks'] if c['key']=='oteryn:quest-reward/l10-brotherhood-use-follower')[field]=value
            with self.assertRaises((ValueError,ValidationError)):q.validate_callbacks(bad)

    def test_l10_golden_chosen_prices_and_order(self):
        rows={c['key'].rsplit('/',1)[-1]:c for c in self.callbacks['callbacks']}
        for suffix,before,after,bit,gold in [('base',0,1,0,100000),('boots',1,2,1,50000),('helmet',2,3,2,50000)]:
            row=rows['l10-golden-'+suffix]
            self.assertEqual(row['fee'],{'gold':gold,'include_bank':True})
            self.assertEqual(row['outfit'],{'outfit':'oteryn:outfit.golden','addons':bit})
            self.assertEqual([(r['minimum'],r['maximum'],r['set']) for r in row['storage']],[(before,before,after)])
            self.assertFalse(row.get('requires_premium',False))

    def test_l9_repeatable_source_trades_and_canonical_item_grants(self):
        rows={c['key'].rsplit('/',1)[-1]:c for c in self.callbacks['callbacks']}
        trade=rows['l9-afflicted-pouch']
        self.assertEqual(trade['storage'],[])
        self.assertEqual(trade['take'],[{'item':'oteryn:item.tibia.i12517','count':1}])
        self.assertEqual(trade['give']['item'],'oteryn:item.tibia.i12413')
        self.assertEqual(trade['achievement_progress'],[{'key':'oteryn:achievement/doctor_doctor','target':100}])
        for field,value in [('take',[]),('give',None),('achievement_progress',[{'key':'oteryn:achievement/doctor_doctor','target':0}])]:
            invalid=copy.deepcopy(self.callbacks)
            next(c for c in invalid['callbacks'] if c['key']==trade['key'])[field]=value
            with self.assertRaises((ValueError,ValidationError)):q.validate_callbacks(invalid)
        for family,looks in [('afflicted','430-431'),('soil_guardian','514-516'),('crystal_warlord','512-513')]:
            mappings={r['grant_key']:r for r in self.author['document']['grant_mappings']}
            for addon in (0,1,2):
                key=f'oteryn:item-grant.outfit.{looks}.'+('base' if addon==0 else f'addon{addon}')
                self.assertEqual(mappings[key]['target'],{'kind':'Outfit','outfit_key':'oteryn:outfit.'+family} if addon==0 else {'kind':'Addon','outfit_key':'oteryn:outfit.'+family,'addon':addon})
                self.assertEqual(mappings[key]['compatible_provenance'],['crystal-item-cosmetics-201189f-v1'])

    def test_l8_source_targets_and_atomic_ten_wand_recipe(self):
        rows={c['key'].rsplit('/',1)[-1]:c for c in self.callbacks['callbacks']}
        self.assertEqual(rows['l8-hunter-hat']['outfits'],[
            {'outfit':'oteryn:outfit.hunter','addons':1,'sex':'male'},
            {'outfit':'oteryn:outfit.hunter','addons':2,'sex':'female'}])
        self.assertEqual(rows['l8-mage-male-wands']['take'],[
            {'item':f'oteryn:item.tibia.i{i}','count':1} for i in (3065,3066,3067,3069,3070,3071,3072,3073,3074,3075)])
        hat=rows['l8-mage-male-hat']
        self.assertEqual(hat['storage'],[])
        self.assertTrue(hat['exact_take'])
        self.assertEqual(hat['creation_sex'],'male')
        self.assertEqual(hat['forbids_addons'],[{'outfit':'oteryn:outfit.mage','addons':2,'sex':'male'}])
        invalid=copy.deepcopy(self.callbacks)
        next(c for c in invalid['callbacks'] if c['key']==hat['key'])['forbids_addons']=[]
        with self.assertRaises(ValueError):q.validate_callbacks(invalid)
        for name in ('mask','claw'):self.assertTrue(rows['l8-dream-warden-'+name]['requires_premium'])

    def test_norseman_reward_continues_with_next_shards_on_published_topic(self):
        callbacks={c['key'].split('/')[-1]:c for c in self.callbacks['callbacks']}
        reward=callbacks['l5-norseman-first-reward']
        second=callbacks['l5-norseman-second-shards']
        phase=next(s['set'] for s in reward['storage'] if s['track'].endswith('/questline'))
        offers=json.loads(self.outputs['content/quests/shared-reward-offers.json'])['offers']
        reachable=[o for o in offers if o['npc']==reward['npc'] and o['topic']==reward['next_topic']
            and 'shard' in o['keywords'] and all(s['minimum']<=phase<=s['maximum'] for s in o['storage'])]
        self.assertEqual(len(reachable),1,'the next source delivery must be reachable without resetting the NPC conversation')
        self.assertEqual(reachable[0]['next_topic'],second['topic'])
        self.assertTrue(all(s['minimum']<=phase<=s['maximum'] for s in second['storage']))

    def test_l7_source_paths_keep_complete_recipes_gifts_and_live_guards(self):
        rows={c['key'].rsplit('/',1)[-1]:c for c in self.callbacks['callbacks']}
        for family,name,item,count in [('knight','fangs',5893,100),('knight','helmet',5924,1),('knight','sweat',5885,1),('knight','steel',5887,1),('druid','griffinclaw',5937,1),('druid','water',5939,1),('druid','dust',5906,100),('druid','chain',5940,1),('assassin','yarn',5886,10)]:
            self.assertEqual(rows[f'l7-{family}-{name}']['take'],[{'item':f'oteryn:item.tibia.i{item}','count':count}])
        for name,item in [('hat-start',4867),('water-start',5938)]:
            self.assertEqual(rows['l7-druid-'+name]['give'],{'item':f'oteryn:item.tibia.i{item}','count':1,'contents':[]})
        for name,suffix in [('l7-knight-fangs','ramsays_helmet_door'),('l7-druid-chain-start','druid_amulet_door')]:
            assigned=next(e for e in rows[name]['storage'] if e['track'].endswith(suffix))
            self.assertEqual((assigned['minimum'],assigned['maximum'],assigned['set']),(-1,1,1))
            declaration=next(t for t in self.callbacks['tracks'] if t['key']==assigned['track'])
            self.assertEqual((declaration['initial'],declaration['quest']),(0,'oteryn:quest.outfit_and_addon_quests'))
        self.assertEqual(rows['l7-knight-helmet-reward']['state_ops'][0]['from'],{'kind':'elapsed_at_least','seconds':7201})
        self.assertTrue(rows['l7-knight-helmet-start']['requires_premium'])
        self.assertEqual(rows['knight-sword']['forbids_addons'],[{'outfit':'oteryn:outfit.knight','addons':1}])
        self.assertEqual(rows['l7-assassin-base-start']['requires_actor_conditions'],['drunk'])
        for name in ('second-start','second'):
            self.assertEqual(rows['l7-assassin-'+name]['requires_actor_conditions'],['fire'])
        for field,value in [('requires_actor_conditions',['fire','fire']),('requires_actor_conditions',['invented']),('forbids_addons',[{'outfit':'oteryn:outfit.knight','addons':0}])]:
            bad=copy.deepcopy(self.callbacks);bad['callbacks'][0][field]=value
            with self.assertRaises(ValidationError):q.validate_callbacks(bad)

    def test_beggar_preserves_source_costs_clocks_and_native_guards(self):
        rows={c['key'].rsplit('/',1)[-1]:c for c in self.callbacks['callbacks']}
        for name,item,count in [('cloth',5913,20),('leather',5878,50),('wings',5894,10),('blossoms',5921,30),('first-materials',5883,100),('staff',6107,1)]:
            self.assertEqual(rows['l6-beggar-'+name]['take'],[{'item':f'oteryn:item.tibia.i{item}','count':count}])
        self.assertEqual(rows['l6-beggar-base']['state_ops'][0]['from'],{'kind':'elapsed_at_least','seconds':86401})
        self.assertEqual(rows['l6-beggar-beard']['state_ops'][0]['from'],{'kind':'elapsed_at_least','seconds':432000})
        self.assertEqual(rows['l6-beggar-dress']['creation_sex'],'female')
        self.assertEqual(rows['l6-beggar-beard']['creation_sex'],'male')
        self.assertEqual(rows['l6-beggar-first-start']['requires_outfits'],['oteryn:outfit.beggar'])
        for name in ('start','base','first-materials','staff'):
            self.assertTrue(rows['l6-beggar-'+name]['requires_premium'])
        self.assertEqual(rows['l6-beggar-first-materials']['fee'],{'gold':20000,'include_bank':True})
        for guard in ('requires_premium','requires_outfits'):
            bad=copy.deepcopy(self.callbacks)
            bad['callbacks'][0][guard]='untrusted'
            with self.assertRaises(ValidationError):q.validate_callbacks(bad)

    def test_citizen_rook_timer_and_shared_counter_are_explicit_source_operations(self):
        callbacks={c['key'].split('/')[-1]:c for c in self.callbacks['callbacks']}
        start=callbacks['l6-citizen-rook-backpack-start']
        self.assertEqual(start['participants'][0]['quest'],'oteryn:quest.outfit_and_addon_quests')
        self.assertEqual(start['participants'][0]['state_ops'][0]['effect'],{'kind':'add','value':1})
        material=callbacks['l6-citizen-rook-backpack-materials']
        self.assertEqual(material['take'],[{'item':'oteryn:item.tibia.i5878','count':100}])
        self.assertEqual(material['state_ops'][0]['effect'],{'kind':'set_now'})
        claim=callbacks['l6-citizen-rook-backpack']
        self.assertEqual(claim['state_ops'][0]['from'],{'kind':'elapsed_at_least','seconds':7201})
        self.assertEqual(callbacks['l6-citizen-rook-hat']['take'],[
            {'item':'oteryn:item.tibia.i3374','count':1},{'item':'oteryn:item.tibia.i5902','count':50},
            {'item':'oteryn:item.tibia.i5890','count':100}])
        declaration=next(t for t in self.callbacks['tracks'] if t.get('writable'))
        self.assertEqual(declaration['quest'],'oteryn:quest.outfit_and_addon_quests')
        altered=copy.deepcopy(self.callbacks)
        next(t for t in altered['tracks'] if t.get('writable'))['quest']='oteryn:quest.citizen_outfits_quest_on_rook'
        with self.assertRaises(ValueError): q.validate_callbacks(altered)

    def test_oriental_source_costs_gender_and_foreign_door_owner(self):
        callbacks={c['key'].split('/')[-1]:c for c in self.callbacks['callbacks']}
        for sex in ('female','male'):
            start=callbacks['l5-oriental-'+sex+'-first-start']
            self.assertEqual(start['creation_sex'],sex)
            self.assertEqual(start['participants'],[{'quest':'oteryn:quest.oriental_outfits_quest','storage':[{
                'track':'oteryn:quest-progress/quest/u7_8/oriental_outfits/oriental_door',
                'minimum':0,'maximum':1,'set':1}]}])
            self.assertEqual(start['requires'][0]['minimum'],35)
            for name,item,count in [('comb',5945,1),('ape-fur',5883,100),('fish-fins',5895,100),('wings',5891,2),('blue-cloth',5912,100)]:
                c=callbacks['l5-oriental-'+sex+'-'+name]
                self.assertEqual(c['take'],[{'item':f'oteryn:item.tibia.i{item}','count':count}])
                self.assertEqual(c['creation_sex'],sex)

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
            if not row['key'].startswith('oteryn:outfit.'):
                continue
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
        source=next(c for c in self.callbacks['callbacks'] if c['key']=='oteryn:quest-reward/l4-nobleman-first')
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
        self.assertEqual({c['key'].split('/')[-1] for c in callbacks if not c['key'].split('/')[-1].startswith(('l1-','l2-','l3-','l4-','l5-','l6-','l7-','l8-','l9-','l10-','l11-','l12-'))},
                         {'pirate-sabre','knight-sword','deepling-helmet','hallowed-holy-water','outfox-helmet','sam-backpack'})
        self.assertEqual(sum(c['outfit'] is not None for c in callbacks),83)
        self.assertEqual(sum(c['outfit'] is None for c in callbacks),119)
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

if __name__=='__main__':
    unittest.main()
