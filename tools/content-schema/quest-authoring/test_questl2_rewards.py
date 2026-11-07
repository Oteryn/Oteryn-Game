"""L2 source hand-ins must survive ordinary shared-registry regeneration."""
import json
import re
import unittest

import qengine_rewards as q


class LaneL2Rewards(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.outputs = q.build()
        cls.callbacks = {
            c['key'].removeprefix('oteryn:quest-reward/'): c
            for c in json.loads(cls.outputs['content/quests/shared-reward-callbacks.json'])['callbacks']
        }

    def test_white_raven_uses_real_brooch_diary_and_paid_friendship(self):
        root = 'oteryn:quest-progress/quest/u7_24/the_white_raven_monastery/'
        brooch = self.callbacks['l2-white-raven-brooch']
        self.assertEqual(brooch['take'], [{'item': 'oteryn:item.tibia.i3205', 'count': 1}])
        self.assertEqual(brooch['topic'], 2)
        self.assertEqual(brooch['storage'], [
            {'track': root+'passage', 'minimum': 0, 'maximum': 1, 'set': 2},
            {'track': root+'quest_log', 'minimum': 0, 'maximum': 0, 'set': 1}])
        diary = self.callbacks['l2-white-raven-diary']
        self.assertEqual(diary['take'], [{'item': 'oteryn:item.tibia.i3212', 'count': 1}])
        self.assertEqual(diary['give']['item'], 'oteryn:item.tibia.i3214')
        self.assertEqual(diary['requires'], [{'track': root+'quest_log', 'minimum': 1, 'maximum': 1}])
        routes = json.loads((q.ROOT/'crates/gameplay/content/src/content/npc_catalogue/travel_routes.json').read_bytes())
        route = next(r for r in routes if r['key'] == 'oteryn:service.travel.dalbrect/isle_of_kings')
        self.assertEqual((route['price'], route['premium'], route['held']), (10, False, False))
        self.assertEqual(route['quest_gate'], root+'quest_log')

    def test_cael_incremental_unlocks_costed_xp_and_real_portable_shop_kits(self):
        prefix='oteryn:quest-progress/quest/u8_54/the_new_frontier/'
        for number in range(1,13):
            callback=self.callbacks[f'l2-cael-tome-{number}']
            self.assertEqual(callback['quest'],'oteryn:quest.the_new_frontier_quest')
            self.assertEqual(callback['take'],[{'item':'oteryn:item.tibia.i10217','count':1}])
            self.assertEqual(callback['storage'][0],{'track':prefix+'tomeof_knowledge','minimum':number-1,'maximum':number-1,'set':number})
            self.assertEqual(len(callback['requires']),1+int(number>=7))
        for number,flag in [(7,'snake_head_teleport'),(8,'corruption_hole'),(10,'zao_palace_doors')]:
            self.assertEqual(self.callbacks[f'l2-cael-tome-{number}']['storage'][1]['track'],prefix+flag)
            self.assertEqual(self.callbacks[f'l2-cael-tome-{number}']['storage'][1]['set'],1)
        repeat=self.callbacks['l2-cael-tome-13']
        self.assertTrue(repeat['repeatable']);self.assertEqual(repeat['experience'],5000)
        self.assertTrue(repeat['storage'][0]['preserve']);self.assertIsNone(repeat['give'])
        shop=json.loads(self.outputs['content/quests/shared-quest-shops.json'])['shops'][0]
        self.assertEqual([(o['item']['key'],o['unit_price'],o['requirements'][0]['minimum']) for o in shop['offers']],
            [(f'oteryn:item.tibia.i{i}',p,n) for i,p,n in [(2965,5000,6),(2966,1000,6),(10216,2500,11),(10288,5000,12)]])
        worlds=json.loads((q.ROOT/'content/quests/world-callbacks.json').read_bytes())['callbacks']
        books=[c for c in worlds if c['key'].startswith('oteryn:quest-callback/l2-tome-')]
        self.assertEqual(len(books),10)
        self.assertEqual(len({(c['selector']['position']['x'],c['selector']['position']['y'],c['selector']['position']['floor']) for c in books}),10)
        self.assertTrue(all(c['effects']['item_reward']['item']=='oteryn:item.tibia.i10217' for c in books))

    def test_botany_source_issue_order_and_atomic_hat_gold(self):
        start=self.callbacks['l2-botany-start'];finish=self.callbacks['l2-botany-return']
        self.assertEqual(start['quest'],'oteryn:quest.tibia_tales')
        states={x['track'].rsplit('/',1)[-1]:x for x in start['storage']}
        self.assertEqual((states['questline']['minimum'],states['questline']['maximum'],states['questline']['set']),(-1,0,1))
        self.assertEqual(states['chest_door']['set'],0)
        self.assertEqual(finish['take'],[{'item':'oteryn:item.tibia.i11699','count':1}])
        self.assertEqual(finish['give'],{'item':'oteryn:item.tibia.i11700','count':1,'contents':[],
            'backpack_outputs':[{'item':'oteryn:item.tibia.i3035','count':10}]})
        self.assertEqual(finish['experience'],3000)
        worlds=json.loads((q.ROOT/'content/quests/world-callbacks.json').read_bytes())['callbacks']
        for index,appearance in [(1,10735),(2,10697)]:
            c=next(c for c in worlds if c['key'].endswith(f'/l2-botany-sample-{index}'))
            self.assertEqual(c['selector']['appearance'],appearance)
            self.assertEqual((c['storage'][0]['minimum'],c['storage'][0]['maximum'],c['storage'][0]['set']),(index,index,index+1))
            self.assertEqual(c['effects']['present'],[{'item':'oteryn:item.tibia.i11699','count':1},{'item':'oteryn:item.tibia.i5908','count':1}])

    def test_botany_uses_served_native_door_and_untagged_book_chest(self):
        import sys
        sys.path.insert(0,str(q.ROOT/'tools/content-schema/world-authoring'))
        from world_region_codec import decode_region
        tiles={t[:2]:t for _,sector in decode_region((q.ROOT/'content/world/placements/region-z10-x128-y123.b3').read_bytes())[3] for t in sector if t[:2] in ((33007,31536),(33004,31530))}
        palette=json.loads((q.ROOT/'content/world/placements/index.json').read_bytes())['palette']
        self.assertEqual([(palette[i]['source_item_id'],a) for i,d,a in tiles[(33007,31536)][-1] if d==0][-1],(7721,{'action':42965}))
        self.assertEqual([(palette[i]['source_item_id'],a) for i,d,a in tiles[(33004,31530)][-1] if d==0][-1],(2469,None))

    def test_bone_pit_selector_resolves_the_served_source_ground(self):
        import sys
        sys.path.insert(0,str(q.ROOT/'tools/content-schema/world-authoring'))
        from world_region_codec import decode_region
        callbacks=json.loads((q.ROOT/'content/quests/world-callbacks.json').read_bytes())['callbacks']
        dig=next(c for c in callbacks if c['key'].endswith('/l2-bone-dig'))
        self.assertEqual(dig['selector'],{'position':{'x':32408,'y':32107,'floor':-10},'unique':None,'action':None,'appearance':103})
        region=q.ROOT/'content/world/placements/region-z10-x126-y125.b3'
        tiles=[t for _,sector in decode_region(region.read_bytes())[3] for t in sector if t[:2]==(32408,32107)]
        self.assertEqual(len(tiles),1)
        self.assertEqual(tiles[0][3],0)
        palette=json.loads((q.ROOT/'content/world/placements/index.json').read_bytes())['palette']
        self.assertEqual([(palette[i]['source_item_id'],depth,attrs) for i,depth,attrs in tiles[0][-1]],[(103,0,None)])
        self.assertEqual(dig['effects']['temporary_item'],{'appearance':3133,'action':4206,'lifetime_us':60000000})
        self.assertEqual(dig['effects']['present'],[{'item':'oteryn:item.tibia.i3457','count':1}])

    def test_bone_pit_issues_container_and_atomic_completion(self):
        start = self.callbacks['l2-bone-start']
        finish = self.callbacks['l2-bone-return']
        self.assertEqual(start['give']['item'], 'oteryn:item.tibia.i4852')
        self.assertEqual(start['minimum_level'], 35)
        self.assertEqual((start['storage'][0]['minimum'],start['storage'][0]['maximum']),(-1,0))
        self.assertEqual(finish['minimum_level'], 35)
        self.assertEqual(finish['take'], [{'item':'oteryn:item.tibia.i131','count':1}])
        self.assertEqual(finish['give']['item'], 'oteryn:item.tibia.i6299')
        self.assertEqual(finish['experience'], 1000)
        self.assertEqual((start['storage'][0]['set'],finish['storage'][0]['minimum'],finish['storage'][0]['set']), (1,2,3))

    def test_draconia_greeting_matches_exact_source_payment_and_releases_focus(self):
        donor = q.donor_inputs()['data-global/npc/an_old_dragon_lord.lua'].decode()
        self.assertIn('player:removeItem(3723, 1)', donor)
        self.assertIn('player:addItem(3206, 1)', donor)
        actual = [c for c in self.callbacks.values()
                  if c['npc'] == 'oteryn:npc.an_old_dragonlord']
        self.assertEqual(len(actual), 2)
        self.assertEqual([(c['storage'][0]['minimum'], c['storage'][0]['maximum'])
                          for c in actual], [(-1, 0), (2, 9223372036854775807)])
        for callback in actual:
            self.assertTrue(callback['close_after'])
            self.assertTrue(callback['repeatable'])
            self.assertEqual(callback['take'], [{'item': 'oteryn:item.tibia.i3723', 'count': 1}])
            self.assertEqual(callback['give'], {'item': 'oteryn:item.tibia.i3206', 'count': 1, 'contents': []})
            self.assertEqual(callback['storage'][0]['set'], 1)
            self.assertIn(callback['reply'][0], donor)
            self.assertIn(callback['failure_reply'][0], donor)
            self.assertIn(':sha256:85016cc08856f49d94c507f43bc84f54801d3a2844946ae391015a0d8c84a052', callback['sources'][0])

    def test_all_jorge_souvenirs_match_the_pinned_menu(self):
        donor = q.donor_inputs()['data-global/npc/jorge.lua'].decode()
        expected = {(name.lower(), int(item)) for name, item in
                    re.findall(r'\{ name = "([^"]+)", id = (\d+) \}', donor)}
        actual = {(c['keywords'][0], int(c['give']['item'].rsplit('i', 1)[1]))
                  for c in self.callbacks.values() if c['npc'] == 'oteryn:npc.jorge'}
        self.assertEqual(len(expected), 37)
        self.assertEqual(actual, expected)
        for c in self.callbacks.values():
            if c['npc'] == 'oteryn:npc.jorge':
                self.assertTrue(c['repeatable'])
                self.assertEqual(c['take'], [{'item': 'oteryn:item.tibia.i19083', 'count': 20}])
                self.assertEqual(c['give']['count'], 1)

    def test_souvenir_profiles_preserve_accepted_weights(self):
        profiles = json.loads(self.outputs['content/items/definitions/spell-native-profiles.json'])
        by_key = {r['production_definition']['production_key']: r for r in profiles['records']}
        for item, weight in [(23677, 2700), (34210, 6660)]:
            semantics = by_key[f'oteryn:item.tibia.i{item}']['semantics']
            self.assertEqual(semantics['physical']['value']['weight']['value'], weight)

    def test_promoted_souvenirs_keep_readability_and_equipment_facts(self):
        profiles = json.loads(self.outputs['content/items/definitions/spell-native-profiles.json'])
        by_key = {r['production_definition']['production_key']: r for r in profiles['records']}
        for item in (31266, 31268):
            row = by_key[f'oteryn:item.tibia.i{item}']
            self.assertTrue(row['admission']['materializable'])
            self.assertEqual(row['admission']['legal_destinations'], ['CharacterInventory', 'Ground'])
            text = row['semantics']['readable_writeable']['value']
            self.assertEqual(text['max_text_length']['value'], 99)
            self.assertTrue(text['readable']['value'])
        equipment = by_key['oteryn:item.tibia.i31268']['semantics']['equipment']['value']
        self.assertEqual(equipment['patterns']['value'][0]['primary_slot']['value'], 'AMULET')

    def test_souvenir_admission_cannot_replace_a_live_native_provider(self):
        import qengine_reward_item_profiles as items
        generated = dict(self.outputs)
        profiles = json.loads(generated[items.PROFILE])
        row = next(r for r in profiles['records']
                   if r['production_definition']['production_key'] == 'oteryn:item.tibia.i31266')
        row['attributes']['speed_bonus'] = 1
        generated[items.PROFILE] = q.encoded(profiles)
        with self.assertRaisesRegex(ValueError, 'QENGINE_REWARD_ITEM_NATIVE_CONFLICT'):
            items.admit(q.ROOT, generated, items.PROFILE, 'content/spells.manifest.json',
                        lambda r: r['production_definition']['production_key'], q.encoded)

    def test_source_equipment_extension_preserves_the_native_provider(self):
        import qengine_reward_item_profiles as items
        profiles = json.loads(self.outputs[items.PROFILE])
        row = next(r for r in profiles['records'] if r['production_definition']['production_key'] == 'oteryn:item.tibia.i3079')
        pattern = row['semantics']['equipment']['value']['patterns']['value'][0]
        self.assertEqual(pattern['primary_slot'], {'state': 'KNOWN', 'value': 'FEET'})
        self.assertEqual(pattern['level'], {'state': 'KNOWN', 'value': 0})
        self.assertEqual(pattern['vocations'], {'state': 'KNOWN', 'value': []})
        self.assertEqual(row['attributes']['speed_bonus'], 20)
        proposed = next(r for r in json.loads(self.outputs[items.PROOF])['records'] if r['production_definition']['production_key'] == 'oteryn:item.tibia.i3079')
        import copy
        constrained = copy.deepcopy(row)
        constrained['semantics']['equipment']['value']['patterns']['value'][0]['level'] = {'state': 'KNOWN', 'value': 42}
        with self.assertRaisesRegex(ValueError, 'EQUIPMENT_CONFLICT'):
            items.extend_equipment(constrained, proposed)
        refined = items.extend_equipment(row, proposed)
        self.assertEqual(refined['attributes'], row['attributes'])
        self.assertEqual(refined['authoring'], row['authoring'])
        for section in row['semantics']:
            if section != 'equipment':
                self.assertEqual(refined['semantics'][section], row['semantics'][section])

    def test_native_manifest_uses_the_repository_canonical_serializer(self):
        import register_spell_families as native
        raw = self.outputs['content/spells.manifest.json']
        self.assertEqual(raw, native.spell_import.canonical_bytes(json.loads(raw)))

    def test_native_weight_cannot_override_the_accepted_canonical_weight(self):
        from pathlib import Path
        from unittest.mock import patch
        import qengine_reward_item_profiles as items
        profiles = json.loads(self.outputs[items.PROFILE])
        row = next(r for r in profiles['records']
                   if r['production_definition']['production_key'] == 'oteryn:item.tibia.i23677')
        row['semantics']['physical']['value']['weight']['value'] = 1
        original = Path.read_bytes
        def read(path):
            return q.encoded(profiles) if path == q.ROOT / items.PROFILE else original(path)
        with patch.object(Path, 'read_bytes', read):
            packet = items.capture()
        record = next(r for r in packet['records']
                      if r['production_definition']['production_key'] == 'oteryn:item.tibia.i23677')
        self.assertEqual(record['semantics']['physical']['value']['weight']['value'], 2700)

    def test_soul_orb_bulk_template_is_source_bound_and_closed(self):
        import copy
        from jsonschema.exceptions import ValidationError
        row = self.callbacks['exchange-cyclops-soul-orbs']
        self.assertEqual(row['bulk'], {'item':'oteryn:item.tibia.i5944',
                         'output':'oteryn:item.tibia.i6528',
                         'base':100, 'bonus':100, 'chance_percent':1})
        self.assertIn(':197:sha256:bc694bf7d62f6a0daae1eda9f6542c6bd79fb749805fe58c23275189ce1e5d0a',
                      row['sources'][0])
        for field, value in [('chance_percent',0), ('chance_percent',101), ('bonus',0),
                             ('base',101), ('output','oteryn:item.tibia.i5944')]:
            bad = copy.deepcopy(row)
            bad['bulk'][field] = value
            with self.assertRaises((ValueError,ValidationError)):
                q.validate_callbacks({'schema':1, 'callbacks':[bad]})
        for update in [{'repeatable':False}, {'take':[]}, {'fee':{'gold':1,'include_bank':True}}]:
            bad = copy.deepcopy(row)
            bad.update(update)
            with self.assertRaises((ValueError,ValidationError)):
                q.validate_callbacks({'schema':1, 'callbacks':[bad]})

    def test_outfox_issue_and_return_use_the_native_door_track(self):
        key = 'oteryn:quest-progress/quest/u8_1/to_outfox_afox_quest'
        start = self.callbacks['outfox-start']
        finish = self.callbacks['outfox-helmet']
        self.assertEqual(start['storage'][-1]['track'],key)
        self.assertEqual(start['storage'][-1]['set'],1)
        self.assertEqual(finish['storage'][0]['track'],key)
        self.assertEqual(finish['storage'][0]['set'],2)
        self.assertEqual(finish['take'],[{'item':'oteryn:item.tibia.i139','count':1}])
        self.assertEqual(finish['give']['item'],'oteryn:item.tibia.i875')

    def test_sam_kroox_opens_both_source_tracks_without_minting_armor(self):
        callback = self.callbacks['sam-kroox']
        self.assertEqual(callback['npc'], 'oteryn:npc.kroox')
        self.assertIsNone(callback['give'])
        self.assertEqual(
            [(s['track'].rsplit('/', 1)[-1], s['set']) for s in callback['storage']],
            [('sams_old_backpack_npc', 2), ('sams_old_backpack_door', 1)],
        )

    def test_bone_pit_inventory_branches_are_disjoint_and_level_gated(self):
        issue = self.callbacks['l2-bone-start']
        filled = self.callbacks['l2-bone-return']
        lost = self.callbacks['bone-pit-lost-sample']
        self.assertEqual(issue['give']['item'], 'oteryn:item.tibia.i4852')
        self.assertEqual(filled['take'], [{'item': 'oteryn:item.tibia.i131', 'count': 1}])
        self.assertEqual(filled['give']['item'], 'oteryn:item.tibia.i6299')
        self.assertEqual(filled['experience'], 1000)
        self.assertIsNone(lost['give'])
        self.assertFalse(lost.get('take'))
        self.assertFalse(lost.get('experience'))
        for callback in (issue, filled, lost):
            self.assertEqual(callback['minimum_level'], 35)
            self.assertTrue(callback['failure_reply'])
            self.assertIn('data-global/npc/muriel.lua:', callback['sources'][0])
        self.assertEqual(filled['inventory'], [
            {'item': 'oteryn:item.tibia.i131', 'minimum': 1, 'maximum': 4294967295}])
        self.assertEqual(lost['inventory'], [
            {'item': 'oteryn:item.tibia.i131', 'minimum': 0, 'maximum': 0}])

    def test_outfox_issue_sets_both_native_owner_tracks_together(self):
        start = self.callbacks['outfox-start']
        self.assertEqual(start['quest'], 'oteryn:quest.tibia_tales')
        self.assertEqual([(r['track'], r['set']) for r in start['storage']], [
            ('oteryn:quest-progress/quest/u8_1/tibia_tales/default_start', 1),
            ('oteryn:quest-progress/quest/u8_1/to_outfox_afox_quest', 1)])

    def test_thieves_deliveries_burn_the_exact_source_items(self):
        stages = [
            ('tusks', 3044, 10, 1, 2), ('vase', 227, 1, 2, 3),
            ('invitation', 7933, 1, 2, 3), ('bridge', 7871, 1, 7, 8),
            ('goblet', 7870, 1, 1, 2), ('fish', 7936, 1, 3, 4),
            ('letter', 7935, 1, 1, 2),
        ]
        for index, (name, item, count, before, after) in enumerate(stages, 1):
            callback = self.callbacks['thieves-' + name]
            self.assertEqual(callback['take'], [{'item': f'oteryn:item.tibia.i{item}', 'count': count}])
            self.assertEqual(callback['storage'][0]['minimum'], before)
            self.assertEqual(callback['storage'][0]['maximum'], before)
            self.assertEqual(callback['storage'][0]['set'], after)
            self.assertEqual(callback['storage'][1]['minimum'], index)
            self.assertEqual(callback['storage'][1]['set'], index + 1)
            self.assertIsNone(callback['give'])

    def test_ape_city_handins_keep_source_progress_and_secondary_guards(self):
        for name, item, before, after in [
            ('moss', 4827, 1, 2), ('syrup', 4828, 3, 4),
            ('parchment', 4831, 5, 6), ('egg', 4839, 9, 10),
            ('mushroom', 4829, 11, 12), ('hair', 4832, 15, 16),
        ]:
            callback = self.callbacks['ape-' + name]
            self.assertEqual(callback['take'], [{'item': f'oteryn:item.tibia.i{item}', 'count': 1}])
            self.assertEqual(callback['storage'][0]['minimum'], before)
            self.assertEqual(callback['storage'][0]['set'], after)
        for name, track in [('moss', 'whisper_moss'), ('hair', 'holy_ape_hair')]:
            guard = self.callbacks['ape-' + name]['storage'][1]
            self.assertTrue(guard['track'].endswith('/' + track))
            self.assertEqual((guard['minimum'], guard['maximum']), (1, 1))
            self.assertTrue(guard['preserve'])

    def test_witches_cap_keeps_its_absolute_deadline(self):
        profiles = json.loads(self.outputs['content/items/definitions/spell-native-profiles.json'])
        cap = next(r for r in profiles['records']
                   if r['production_definition']['production_key'] == 'oteryn:item.tibia.i4829')
        temporal = cap['semantics']['temporal']['value']
        self.assertEqual(temporal['consumption_mode']['value'], 'DURABLE_ABSOLUTE_DEADLINE')
        self.assertEqual(temporal['duration']['value'], 86400000)
        self.assertFalse(temporal['stop_duration']['value'])

    def test_jorge_repeat_is_twenty_tokens_for_one_source_abacus(self):
        callback = self.callbacks['token-abacus']
        self.assertTrue(callback['repeatable'])
        self.assertEqual(callback['take'], [{'item': 'oteryn:item.tibia.i19083', 'count': 20}])
        self.assertEqual(callback['give'], {'item': 'oteryn:item.tibia.i19151', 'count': 1, 'contents': []})
        self.assertTrue(callback['storage'][0]['preserve'])
        self.assertIn('data-global/npc/jorge.lua:128:sha256:', callback['sources'][0])
        free = json.loads(json.dumps(callback))
        free['take'] = []
        with self.assertRaises(ValueError):
            q.validate_callbacks({'schema': 1, 'callbacks': [free]})


if __name__ == '__main__':
    unittest.main()
