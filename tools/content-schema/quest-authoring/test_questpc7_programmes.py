"""Offline donor identity, schema and deterministic materialization checks."""
import copy
import json
import unittest
from jsonschema import Draft202012Validator
import qengine_rewards as q


class QuestPc7QuestRewards(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.outputs=q.build()
        cls.callbacks=json.loads(cls.outputs['content/quests/shared-reward-callbacks.json'])
        cls.author=json.loads(cls.outputs['content/cosmetics/outfits/authoring.json'])
        cls.schema=json.loads((q.ROOT/'tools/content-schema/quest-authoring/shared-reward-callbacks.schema.json').read_text())
    def test_golden_confirmation_topics_do_not_collide_with_other_king_routes(self):
        own=[c for c in self.callbacks['callbacks'] if c['key'].startswith('oteryn:quest-reward/questpc7-golden-') and 'fee' in c]
        others=[c for c in self.callbacks['callbacks'] if c['npc']=='oteryn:npc.king_tibianus' and not c['key'].startswith('oteryn:quest-reward/questpc7-golden-')]
        self.assertEqual({c['topic'] for c in own},{101,102,103})
        self.assertFalse({c['topic'] for c in own}&{c['topic'] for c in others})

    def test_out_of_order_prayers_have_an_explicit_reply_without_transition(self):
        offers=json.loads(self.outputs['content/quests/shared-reward-offers.json'])['offers']
        prayers=[o for o in offers if o['keywords']==['stake prayer']]
        self.assertEqual(len(prayers),10)
        for offer in prayers:
            self.assertEqual((offer['topic'],offer['next_topic']),(0,0))
            self.assertEqual(len(offer['storage']),1)
            self.assertTrue(offer['storage'][0]['preserve'])
            self.assertFalse({'give','take','fee','outfit','transition'}&offer.keys())

    def test_formal_exhibition_requires_current_premium_and_consumed_button(self):
        worlds=[c for c in json.loads(self.outputs['content/quests/questpc7-world-callbacks.json'])['callbacks'] if c['key'].startswith('oteryn:quest-callback/questpc7-formal-')]
        self.assertEqual(len(worlds),2)
        for callback in worlds:
            self.assertEqual(callback['requirements'], {'min_level':20, 'premium':True, 'completed_quests':[]})
        entry=next(c for c in worlds if c['key'].endswith('-entry'))
        display=next(c for c in worlds if c['key'].endswith('-display'))
        self.assertEqual(entry['selector']['position'], {'x':32314,'y':32170,'floor':-6})
        self.assertEqual(entry['selector']['appearance'],464)
        self.assertEqual(display['selector']['appearance'],2110)
        self.assertEqual(display['effects']['outfit_reward'],{'outfit':'oteryn:outfit.formal_dress','addons':0})
        plume=next(c for c in self.callbacks['callbacks'] if c['key']=='oteryn:quest-reward/questpc7-formal-plume')
        self.assertEqual(plume['requirements']['owned_cosmetics'],[{'outfit':'oteryn:outfit.formal_dress','addons':2}])
    def test_ferumbras_guardians_and_passages_use_native_clear_source_cells(self):
        codec=q.load_module('questpc7_ferumbras_test_codec','tools/content-schema/world-authoring/world_region_codec.py')
        import sys
        sys.path.insert(0,str(q.ROOT/'tools/content-schema/native-gameplay'))
        import build_source_item_profiles as source
        client=q.ROOT/'content/assets/files/appearances-2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.dat'
        objects=source.engine_items.load_appearance_objects(client.read_bytes())
        palette=json.loads((q.ROOT/'content/world/placements/index.json').read_bytes())['palette']
        cache={}
        def tile(p):
            path=f'content/world/placements/region-z{-p["floor"]:02}-x{p["x"]//256}-y{p["y"]//256}.b3'
            if path not in cache:
                region=codec.decode_region((q.ROOT/path).read_bytes())
                cache[path]={(x,y):(flags,house,entries) for _,cells in region[3] for x,y,flags,house,_,entries in cells}
            return cache[path][(p['x'],p['y'])]
        rows=[r for r in json.loads(self.outputs['content/quests/questpc7-world-callbacks.json'])['callbacks'] if r['quest']=='oteryn:quest.ferumbras_ascension_quest']
        self.assertEqual(len(rows),11)
        for row in rows:
            selector=row['selector'];flags,house,entries=tile(selector['position'])
            self.assertEqual(house,0)
            self.assertIn(selector['appearance'],[palette[i]['source_item_id'] for i,depth,_ in entries if depth==0])
            self.assertLessEqual(len(row['storage']),8)
        objectives=[r for r in json.loads(self.outputs['content/quests/questpc7-death-objectives.json'])['objectives'] if r['quest']=='oteryn:quest.ferumbras_ascension_quest']
        self.assertEqual(len(objectives),8)
        for objective in objectives:
            flags,house,entries=tile(objective['guardian']['position'])
            self.assertEqual((flags&1,house,len(entries)),(0,0,1))
            client_flags=objects[palette[entries[0][0]]['source_item_id']]['flags']
            self.assertTrue(client_flags.get('flags.bank'))
            self.assertFalse(client_flags.get('flags.unpass',False))
    def test_grave_twelve_sites_and_both_sirs_have_exact_native_bindings(self):
        programme=q.load_module('questpc7_grave_test_programme','tools/content-schema/quest-authoring/questpc7_grave_programme.py')
        self.assertEqual(len(programme.GRAVES),12)
        codec=q.load_module('questpc7_grave_test_codec','tools/content-schema/world-authoring/world_region_codec.py')
        palette=json.loads((q.ROOT/'content/world/placements/index.json').read_bytes())['palette']
        cache={}
        for name,p,appearance,arrival,_ in programme.GRAVES:
            path=f'content/world/placements/region-z{-p[2]:02}-x{p[0]//256}-y{p[1]//256}.b3'
            if path not in cache:
                region=codec.decode_region((q.ROOT/path).read_bytes())
                cache[path]={(x,y):(house,[palette[i]['source_item_id'] for i,d,_ in es if d==0]) for _,cells in region[3] for x,y,_,house,_,es in cells}
            house,items=cache[path][p[:2]]
            self.assertEqual(house,0,name)
            self.assertIn(appearance,items,name)
            self.assertLessEqual(max(abs(arrival[0]-p[0]),abs(arrival[1]-p[1])),1,name)
        rows=[r for r in json.loads(self.outputs['content/quests/questpc7-world-callbacks.json'])['callbacks'] if r['quest']==programme.QUEST]
        self.assertEqual(len(rows),22)
        self.assertTrue(all(len(r['storage'])<=8 for r in rows))
        pair=next(r for r in rows if r['key'].endswith('knight-passage-4'))
        required={r['track'] for r in pair['storage'] if r.get('preserve') and r['minimum']==1}
        self.assertTrue({programme.counter(4),programme.counter(5)}<=required)
        cobra=next(r for r in rows if r['key'].endswith('-cobra'))
        self.assertEqual(cobra['trigger'],'use')
        self.assertIn({'track':programme.COBRA,'minimum':1,'maximum':1,'set':1,'preserve':True},cobra['storage'])
    def test_hero_selected_bindings_exist_in_unchanged_source_map(self):
        rows=json.loads(self.outputs['content/quests/questpc7-world-callbacks.json'])['callbacks']
        rows=[r for r in rows if r['quest']=='oteryn:quest.hero_of_rathleton_quest']
        codec=q.load_module('questpc7_hero_test_codec','tools/content-schema/world-authoring/world_region_codec.py')
        palette=json.loads((q.ROOT/'content/world/placements/index.json').read_bytes())['palette']
        region=codec.decode_region((q.ROOT/'content/world/placements/region-z14-x131-y124.b3').read_bytes())
        tiles={(x,y):(flags,house,[palette[p]['source_item_id'] for p,depth,_ in entries if not depth])
            for _,cells in region[3] for x,y,flags,house,_,entries in cells}
        self.assertEqual(len(rows),4)
        for row in rows:
            selector=row['selector'];position=selector['position']
            flags,house,appearances=tiles[(position['x'],position['y'])]
            self.assertEqual((flags,house),(0,0))
            self.assertIn(selector['appearance'],appearances)
        objectives=json.loads(self.outputs['content/quests/questpc7-death-objectives.json'])['objectives']
        boss=next(o for o in objectives if o['key']=='oteryn:quest-death/questpc7-hero-maxxen')
        construct=next(o for o in objectives if o['key']=='oteryn:quest-death/questpc7-hero-construct')
        self.assertEqual((boss['stage'],boss['stage_value']),(construct['counter'],construct['required']))
    def test_hidden_native_objectives_share_quota_and_use_admitted_empty_ground(self):
        rows=json.loads(self.outputs['content/quests/questpc7-death-objectives.json'])['objectives']
        rows=[r for r in rows if r['quest']=='oteryn:quest.hidden_threats_quest']
        self.assertEqual({r['creature'] for r in rows},{'oteryn:creature.corym_'+s for s in ['charlatan','skirmisher','vanguard']})
        self.assertEqual({r['counter'] for r in rows},{'oteryn:quest-progress/questpc7/hidden/kills'})
        self.assertTrue(all(r['required']==5 for r in rows))
        self.assertEqual(sum(r['guardian'] is not None for r in rows),1)
        codec=q.load_module('questpc7_test_world_codec','tools/content-schema/world-authoring/world_region_codec.py')
        palette=json.loads((q.ROOT/'content/world/placements/index.json').read_bytes())['palette']
        region=codec.decode_region((q.ROOT/'content/world/placements/region-z13-x129-y125.b3').read_bytes())
        tiles={(x,y):[palette[p]['source_item_id'] for p,depth,_ in entries if not depth]
               for _,cells in region[3] for x,y,_,_,_,entries in cells}
        for position in [(33078,32015),(33079,32015),(33080,32015)]:
            self.assertEqual(tiles[position],[10113],position)
        self.assertIn(2469,tiles[(33080,32014)],'the reward binds an existing physical chest')
    def test_hydra_uses_existing_nest_and_personal_native_pickup_instead_of_a_claim(self):
        worlds=json.loads(self.outputs['content/quests/questpc7-world-callbacks.json'])['callbacks']
        rows=[r for r in worlds if r['quest']=='oteryn:quest.hydra_egg_quest']
        self.assertEqual(len(rows),2)
        nest=next(r for r in rows if r['trigger']=='use')
        self.assertTrue(nest['completes'])
        self.assertNotIn('item_reward',nest)
        self.assertEqual(nest['requirements']['owned_items'],[{'item':'oteryn:item.tibia.i4839','count':1,'map_source':(33004<<32)|(32664<<16)|(3<<8)|2}])
        objectives=json.loads(self.outputs['content/quests/questpc7-death-objectives.json'])['objectives']
        objective=next(r for r in objectives if r['quest']==nest['quest'])
        self.assertEqual((objective['creature'],objective['required']),('oteryn:creature.hydra',1))
        codec=q.load_module('questpc7_hydra_test_codec','tools/content-schema/world-authoring/world_region_codec.py')
        palette=json.loads((q.ROOT/'content/world/placements/index.json').read_bytes())['palette']
        region=codec.decode_region((q.ROOT/'content/world/placements/region-z03-x128-y127.b3').read_bytes())
        tiles={(x,y):[(palette[p]['source_item_id'],attrs) for p,depth,attrs in entries if not depth]
            for _,cells in region[3] for x,y,_,_,_,entries in cells}
        self.assertEqual(tiles[(33004,32664)],[(4402,None),(1063,None),(4839,{'count':0})])
        self.assertEqual(tiles[(33003,32665)],[(1019,None)])
        self.assertEqual(tiles[(33005,32665)],[(1019,None)])
    def test_hidden_valuables_are_one_exact_once_only_backpack_plan(self):
        rows=json.loads(self.outputs['content/quests/questpc7-world-callbacks.json'])['callbacks']
        chest=next(r for r in rows if r['key']=='oteryn:quest-callback/questpc7-hidden-valuables')
        self.assertTrue(chest['completes'])
        self.assertEqual(chest['trigger'],'use')
        self.assertEqual(chest['activation'],{'track':chest['storage'][0]['track'],'minimum':1,'maximum':1})
        self.assertEqual(chest['storage'][1]['minimum'],5)
        self.assertTrue(chest['storage'][1]['preserve'])
        self.assertEqual(chest['item_reward'],{'item':'oteryn:item.tibia.i3040','count':2,'contents':[],
            'backpack_outputs':[{'item':f'oteryn:item.tibia.i{i}','count':1} for i in (3030,3032,3033)]})
    def test_selected_jorge_replica_consumes_exact_twenty_and_completes_once(self):
        rows=[c for c in self.callbacks['callbacks'] if c['key'].startswith('oteryn:quest-reward/questpc7-tokens-')]
        self.assertEqual(len(rows),3)
        exchange=next(c for c in rows if c['give'])
        self.assertEqual(exchange['take'],[{'item':'oteryn:item.tibia.i19083','count':20}])
        self.assertEqual(exchange['give'],{'item':'oteryn:item.tibia.i19151','count':1,'contents':[]})
        self.assertEqual(sum(c['completes'] for c in rows),1)
        self.assertTrue(all(c['npc']=='oteryn:npc.jorge' for c in rows))
        self.assertEqual([c['storage'][0]['minimum'] for c in rows if c['completes']],[1])
    def test_hoist_binds_real_native_objects_without_donor_uids(self):
        callbacks=json.loads(self.outputs['content/quests/questpc7-world-callbacks.json'])['callbacks']
        rows=[c for c in callbacks if c['key'].startswith('oteryn:quest-callback/questpc7-hoist-')]
        self.assertEqual(len(rows),4)
        self.assertEqual(sum(c.get('completes',False) for c in rows),1)
        bindings=json.loads(self.outputs['content/quests/native-world-callback-bindings.json'])['bindings']
        self.assertEqual([b['expected']['unique'] for b in bindings],[3059,3060,3061])
        codec=q.load_module('questpc7_hoist_test_codec','tools/content-schema/world-authoring/world_region_codec.py')
        palette=json.loads((q.ROOT/'content/world/placements/index.json').read_bytes())['palette']
        for floor in [9,10,11]:
            region=codec.decode_region((q.ROOT/f'content/world/placements/region-z{floor:02}-x125-y121.b3').read_bytes())
            tiles={(x,y):[(palette[p]['source_item_id'],attrs) for p,depth,attrs in entries if not depth]
                   for _,cells in region[3] for x,y,_,_,_,entries in cells}
            self.assertEqual(sum(i==5770 for i,_ in tiles[(32157,31125)]),1)
            self.assertTrue(all(not (attrs or {}).get('unique') for _,attrs in tiles[(32157,31125)]))
            for callback in rows:
                position=callback['selector']['position']
                if position['floor']==-floor:
                    self.assertIn(callback['selector']['appearance'],[i for i,_ in tiles[(position['x'],position['y'])]])
        self.assertTrue(all(c['requirements']['completed_quests']==['oteryn:quest.the_ice_islands_quest'] for c in rows))
    def test_friends_services_preserve_programme_and_have_bounded_distinct_counters(self):
        from jsonschema import ValidationError
        selected=[c for c in self.callbacks['callbacks'] if c['key'].split('/')[-1].startswith('questpc7-friends-')]
        self.assertEqual(len(selected),18)
        services=[c for c in selected if c.get('counter')]
        self.assertEqual(len(services),2)
        self.assertEqual(len({c['counter']['track'] for c in services}),2)
        self.assertTrue(all(c['counter']['maximum']==1000000000000 and all(s['preserve'] for s in c['storage']) for c in services))
        self.assertEqual(sum(c.get('completes',False) for c in selected),1)
        for mutation in ('zero','collision','completion'):
            bad=copy.deepcopy(self.callbacks)
            callback=next(c for c in bad['callbacks'] if c['key']==services[0]['key'])
            if mutation=='zero':callback['counter']['maximum']=0
            elif mutation=='collision':callback['counter']['track']=callback['storage'][0]['track']
            else:callback['completes']=True
            with self.assertRaises((ValueError,ValidationError)):q.validate_callbacks(bad)
    def test_selected_reward_requirements_are_bounded_and_use_canonical_parents(self):
        from jsonschema import ValidationError
        selected={c['key'].split('/')[-1]:c for c in self.callbacks['callbacks'] if c.get('requirements')}
        self.assertEqual(selected['questpc7-hunter-intro']['requirements'],{'min_level':0,'premium':True,'completed_quests':[]})
        self.assertEqual(selected['questpc7-illuminated-intro']['requirements'],{'min_level':1000,'premium':True,'completed_quests':['oteryn:quest.authored.make_believe_quest']})
        for mutation in ('level','duplicate','foreign'):
            bad=copy.deepcopy(self.callbacks)
            callback=next(c for c in bad['callbacks'] if c['key']==selected['questpc7-illuminated-intro']['key'])
            if mutation=='level':callback['requirements']['min_level']=1000001
            elif mutation=='duplicate':callback['requirements']['completed_quests']*=2
            else:callback['requirements']['completed_quests']=['unscoped:parent']
            with self.assertRaises((ValueError,ValidationError)):q.validate_callbacks(bad)
    def test_golden_selected_prices_and_fee_conservation(self):
        from jsonschema import ValidationError
        selected=[c for c in self.callbacks['callbacks'] if c['key'].split('/')[-1].startswith('questpc7-golden-')]
        self.assertEqual(len(selected),5)
        fees={c['key'].split('-')[-1]:c['fee'] for c in selected if c.get('fee')}
        self.assertEqual(fees, {name:{'gold':amount,'include_bank':True} for name,amount in [('base',100000),('gloves',50000),('helmet',50000)]})
        source=next(c for c in selected if c.get('fee'))
        for mutation in ('zero','double'):
            bad=copy.deepcopy(self.callbacks)
            callback=next(c for c in bad['callbacks'] if c['key']==source['key'])
            if mutation=='zero':callback['fee']['gold']=0
            else:callback['take']=[{'item':'oteryn:item.tibia.i3043','count':10}]
            with self.assertRaises((ValueError,ValidationError)):q.validate_callbacks(bad)
        self.assertEqual(sum(c.get('completes',False) for c in selected),1)
    def test_selected_consumptive_grants_keep_explicit_pairs_and_real_ownership_guards(self):
        from jsonschema import ValidationError
        document = self.author['document']
        formal = next(o for o in document['outfits'] if o['key'] == 'oteryn:outfit.formal_dress')
        self.assertEqual((formal['female']['id'], formal['male']['id'], formal['addon_count']), (1461, 1460, 2))
        self.assertTrue(formal['premium'])
        self.assertFalse(formal['starter_female'] or formal['starter_male'])
        targets = {'oteryn:outfit.formal_dress', 'oteryn:outfit.fiend_slayer', 'oteryn:outfit.hand_of_the_inquisition'}
        item_aliases = [m for m in document['grant_mappings'] if m['grant_key'].startswith('oteryn:item-grant.')]
        self.assertEqual(len(item_aliases),47)
        aliases = [m for m in item_aliases if m['target'].get('outfit_key') in targets]
        self.assertEqual(len(aliases), 9)
        self.assertEqual({m['target']['outfit_key'] for m in aliases}, targets)
        self.assertTrue(all(m['compatible_provenance'] == ['crystal-item-cosmetics-201189f-v1'] for m in aliases))
        callbacks = {c['key'].split('/')[-1]: c for c in self.callbacks['callbacks']}
        for slug in ('fiend', 'inquisition'):
            self.assertEqual(callbacks[f'questpc7-{slug}-second-token']['requirements']['owned_cosmetics'][0]['addons'], 1)
            self.assertEqual(callbacks[f'questpc7-{slug}-complete']['requirements']['owned_cosmetics'][0]['addons'], 3)
            self.assertTrue(callbacks[f'questpc7-{slug}-complete']['completes'])
        bad = copy.deepcopy(self.callbacks)
        callback = next(c for c in bad['callbacks'] if c['key'].endswith('questpc7-fiend-second-token'))
        callback['requirements']['owned_cosmetics'][0]['addons'] = 4
        with self.assertRaises((ValueError, ValidationError)):
            q.validate_callbacks(bad)


class GrimvaleProgramme(unittest.TestCase):
    def test_personal_harvesting_and_exact_trap_input_keep_global_event_separate(self):
        import questpc7_grimvale_programme as grim
        worlds, objectives, contacts=grim.build(q.ROOT)
        plots=[c for c in worlds if '-nightshade-' in c['key']]
        self.assertEqual(len(plots),5)
        self.assertEqual(len({tuple(c['selector']['position'].values()) for c in plots}),5)
        for i,c in enumerate(plots):
            self.assertEqual(c['item_reward'],{'item':'oteryn:item.tibia.i22082','count':1,'contents':[]})
            self.assertEqual(c['activation'],{'track':grim.FLOWERS,'minimum':i,'maximum':i})
            self.assertTrue(c['requirements']['premium'])
        trap=next(c for c in worlds if c['key'].endswith('/questpc7-grimvale-deploy-trap'))
        self.assertEqual(trap['item_on_target'],{'item':'oteryn:item.tibia.i22074','count':1,'replacement':None,'target_replacement':None})
        afflicted=[o for o in objectives if o['counter'].endswith('/afflicted')]
        self.assertEqual({o['creature'] for o in afflicted},{'oteryn:creature.'+n for n in ['werewolf','werebear','wereboar']})
        self.assertTrue(all(o['required']==10 for o in afflicted))
        self.assertEqual(next(o for o in objectives if o['creature']=='oteryn:creature.werehyaena')['required'],20)
        self.assertEqual({c['npc'] for c in contacts},{'oteryn:npc.rhargu','oteryn:npc.domizian'})

    def test_real_native_sites_and_once_only_rewards_have_two_earned_travel_directions(self):
        import questpc7_grimvale_programme as grim
        import sys
        sys.path.insert(0,str(q.ROOT/'tools/content-schema/world-authoring'))
        import world_region_codec as codec
        worlds,_,_=grim.build(q.ROOT)
        palette=json.loads((q.ROOT/'content/world/placements/index.json').read_bytes())['palette']
        cache={}
        for c in worlds:
            p=c['selector']['position'];x,y,z=p['x'],p['y'],p['floor']
            path=f'content/world/placements/region-z{-z:02}-x{x//256}-y{y//256}.b3'
            if path not in cache:cache[path]=codec.decode_region((q.ROOT/path).read_bytes())[3]
            native=[palette[i]['source_item_id'] for _,cells in cache[path] for tx,ty,_,_,_,items in cells if (tx,ty)==(x,y) for i,depth,_ in items if depth==0]
            self.assertIn(c['selector']['appearance'],native,c['key'])
        reward=next(c for c in worlds if c.get('completes'))
        self.assertEqual(reward['item_reward']['item'],'oteryn:item.tibia.i27565')
        self.assertEqual(set(reward['achievements']),{'oteryn:achievement/over_the_moon','oteryn:achievement/drama_in_darama'})
        travel=[c for c in worlds if '/questpc7-grimvale-earned-access-' in c['key']]
        self.assertEqual(len(travel),2)
        self.assertTrue(all(c['storage']==[{'track':grim.ACCESS,'minimum':1,'maximum':1,'set':1,'preserve':True}] and c['teleport'] and c['requirements']['premium'] for c in travel))


class HeartProgramme(unittest.TestCase):
    def test_six_independent_source_arenas_and_exact_yana_reward(self):
        import questpc7_heart_programme as heart
        worlds, objectives, rewards=heart.build(q.ROOT,q.donor_inputs(),q.PIN)
        self.assertEqual([o['creature'] for o in objectives],['oteryn:creature.'+n for n in ['anomaly','rupture','realityquake','eradicator','outburst','world_devourer']])
        self.assertEqual(len({o['counter'] for o in objectives}),6)
        self.assertTrue(all(o['required']==1 and o['guardian']['respawn_ms']==60000 for o in objectives))
        lever=next(c for c in worlds if c['key'].endswith('/questpc7-heart-destabilizer'))
        self.assertEqual(lever['selector']['position'],{'x':32245,'y':31244,'floor':-14})
        self.assertEqual(lever['lever'],[8911,8912])
        self.assertEqual(rewards[0]['npc'],'oteryn:npc.yana')
        self.assertEqual(rewards[0]['give'],{'item':'oteryn:item.tibia.i22721','count':5,'contents':[]})
        self.assertTrue(rewards[0]['completes'])
        self.assertEqual(rewards[0]['achievement'],'oteryn:achievement/ender_of_the_end')
        self.assertEqual(rewards[0]['requirements'],{'min_level':150,'premium':True,'completed_quests':[]})

    def test_powerful_learning_binds_only_the_source_named_families_to_native_completion(self):
        import questpc7_knowledge
        rules=[r for r in questpc7_knowledge.build(q.ROOT,q.donor_inputs(),q.PIN)['rules'] if 'oteryn:quest.heart_of_destruction_quest' in r['any_completed_quests']]
        self.assertEqual([(r['family'],r['tier']) for r in rules],[(1,3),(11,3),(12,3)])
        for rule in rules:
            self.assertEqual(rule['any_completed_quests'],['oteryn:quest.heart_of_destruction_quest','oteryn:quest.forgotten_knowledge_quest'])
            self.assertTrue(any('data-global/npc/yana.lua:185-187:sha256:' in src for src in rule['sources']))
            self.assertTrue(any(src.startswith('canonical:') for src in rule['sources']))


class ForgottenProgramme(unittest.TestCase):
    def test_exact_selected_stages_source_terminal_bosses_and_tome_exchange(self):
        import questpc7_forgotten_programme as f
        worlds,objectives,rewards=f.build(q.ROOT,q.donor_inputs(),q.PIN)
        self.assertEqual(len(worlds),11)
        self.assertEqual([r['creature'] for r in objectives],['oteryn:creature.'+n for n in ['lloyd','lady_tenebris','the_enraged_thorn_knight','soul_of_dragonking_zyrtarch','the_time_guardian','the_last_lore_keeper']])
        self.assertEqual([r['stage_value'] for r in objectives],[2,3,5,7,8,9])
        self.assertEqual(len({r['counter'] for r in objectives}),6)
        self.assertTrue(all(r['required']==1 for r in objectives))
        tome=next(r for r in worlds if r['key'].endswith('/questpc7-forgotten-tome'))
        self.assertEqual(tome['item_reward'],{'item':'oteryn:item.tibia.i23986','count':1,'contents':[]})
        take=next(r for r in rewards if r['key'].endswith('/questpc7-forgotten-tome'))
        self.assertEqual(take['take'],[{'item':'oteryn:item.tibia.i23986','count':1}])
        self.assertEqual(len(rewards),10)
        self.assertEqual(sum(r['npc']=='oteryn:npc.ivalisse' for r in rewards),4)
        self.assertEqual([r['keywords'][0] for r in rewards if r['npc']=='oteryn:npc.a_strange_chalice'],['chalice','daughter','story','disguised','thirsty','learned'])
        self.assertEqual([r['key'] for r in rewards if r.get('completes')],['oteryn:quest-reward/questpc7-forgotten-complete'])
        self.assertTrue(all(r['requirements']=={'min_level':0,'premium':True,'completed_quests':[]} for r in worlds+rewards))

    def test_every_world_selector_exists_in_retained_native_map(self):
        import questpc7_forgotten_programme as f
        import sys
        sys.path.insert(0,str(q.ROOT/'tools/content-schema/world-authoring'))
        import world_region_codec as codec
        palette=json.loads((q.ROOT/'content/world/placements/index.json').read_bytes())['palette'];cache={}
        worlds,_,_=f.build(q.ROOT,q.donor_inputs(),q.PIN)
        for c in worlds:
            p=c['selector']['position'];x,y,z=p['x'],p['y'],p['floor']
            path=f'content/world/placements/region-z{-z:02}-x{x//256}-y{y//256}.b3'
            if path not in cache:cache[path]=codec.decode_region((q.ROOT/path).read_bytes())[3]
            native=[palette[i]['source_item_id'] for _,cells in cache[path] for tx,ty,_,_,_,items in cells if (tx,ty)==(x,y) for i,depth,_ in items if depth==0]
            self.assertIn(c['selector']['appearance'],native,c['key'])
        egg=next(r for r in worlds if r['key'].endswith('/questpc7-forgotten-egg'))
        self.assertEqual(egg['selector']['appearance'],4839)

    def test_native_learning_matches_source_families_and_preserves_heart_alternatives(self):
        import questpc7_knowledge
        rules=questpc7_knowledge.build(q.ROOT,q.donor_inputs(),q.PIN)['rules']
        self.assertEqual([(r['family'],r['tier']) for r in rules],[(f,3) for f in [1,3,4,5,6,7,8,10,11,12,14,15,16,17,18,19,20]])
        for rule in rules:
            expected=['oteryn:quest.forgotten_knowledge_quest']
            if rule['family'] in [1,11,12]:expected.insert(0,'oteryn:quest.heart_of_destruction_quest')
            self.assertEqual(rule['any_completed_quests'],expected)
            self.assertTrue(any('data/XML/imbuements.xml:' in s for s in rule['sources']))


class FeasterPrefix(unittest.TestCase):
    def test_personal_quota_tool_custody_and_exact_potion_inputs(self):
        import questpc7_feaster_programme as f
        worlds,objectives,rewards=f.build(q.ROOT)
        self.assertEqual((len(worlds),len(objectives),len(rewards)),(13,0,4))
        self.assertFalse(any(r.get('completes') for r in worlds+rewards))
        repeated=[r for r in worlds if any(s.get('add') for s in r['storage'])]
        self.assertEqual({r['key'].rsplit('-',1)[-1] for r in repeated},{'sample','soul','purify'})
        for row in repeated:
            counters=[s for s in row['storage'] if s.get('add')]
            self.assertEqual(len(counters),1)
            state=counters[0]
            self.assertEqual((state['minimum'],state['maximum'],state['set'],state['add']),(0,19,20,1))
            self.assertEqual(row['activation'],{'track':state['track'],'minimum':0,'maximum':19})
            self.assertTrue(row['requirements']['owned_items'])
        apparatus=next(r for r in worlds if r['key'].endswith('-apparatus'))
        self.assertEqual(apparatus['item_on_target']['item'],'oteryn:item.tibia.i32602')
        self.assertEqual(apparatus['offerings'][0]['item'],'oteryn:item.tibia.i3661')
        self.assertEqual(apparatus['item_reward']['item'],'oteryn:item.tibia.i32584')
        self.assertEqual([r['give']['item'] for r in rewards if r.get('give')],['oteryn:item.tibia.i32582','oteryn:item.tibia.i32225','oteryn:item.tibia.i32226'])
        self.assertTrue(all(r['requirements']['min_level']==250 and r['requirements']['premium'] for r in worlds+rewards))

    def test_anchored_props_have_ground_only_source_constructors(self):
        import questpc7_props
        props=questpc7_props.build(q.ROOT)['props']
        packet=json.loads((q.ROOT/'tools/content-schema/native-gameplay/qengine-reward-item-profiles.json').read_bytes())
        profiles={r['production_definition']['production_key']:r for r in packet['records']}
        self.assertEqual(len(props),6)
        for prop in props:
            row=profiles[prop['item']]
            self.assertEqual(row['admission'],{'materializable':True,'stack_class':'NonStackable','legal_destinations':['Ground']})
            for field in ['movable','pickupable']:
                self.assertEqual(row['semantics']['physical']['value'][field],{'state':'KNOWN','value':False})
