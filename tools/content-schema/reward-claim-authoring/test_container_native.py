"""W15b: only complete source-backed recipes reach the real container mint."""
import copy
import hashlib
import json
import unittest

import charge_native
import container_native
import reward_claim_authoring as plain


class ContainerNativeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.items = plain.load_items()
        cls.rows = [r['definition'] for p in (plain.ROOT / plain.CONTENT_DIR).glob('reward-claim-variants-*.json')
                    for r in json.loads(p.read_text())['records']]
        cls.bear = next(d for d in cls.rows if d['identity']['key'] == 'oteryn:reward-claim.kv.bear_chest1')

    def test_goblin_temple_bag_keeps_all_children_and_selected_milk(self):
        claim = next(d for d in self.rows if d['identity']['key'] ==
                     'oteryn:reward-claim.kv.goblintemple2')
        held_items = copy.deepcopy(self.items)
        held_items['oteryn:item.tibia.i2874']['materializable'] = False
        items = container_native.claim_items(claim['provenance']['pilot_key'], held_items)
        recipe = container_native.derive(claim, items, plain.stack_problem)
        self.assertIsNotNone(recipe)
        self.assertEqual(recipe['placements'][0]['unique_id'],6119)
        self.assertEqual(recipe['placements'][0]['completion'], {'kind':'quest_kv','key':'goblintemple2'})
        self.assertEqual([(c['item']['key'],c['count'],c.get('fluid_subtype'))
                          for c in recipe['placements'][0]['contents']],
                         [('oteryn:item.tibia.i3466',1,None),('oteryn:item.tibia.i2992',4,None),
                          ('oteryn:item.tibia.i2874',1,9)])
        self.assertFalse(held_items['oteryn:item.tibia.i2874']['materializable'])
        changed = copy.deepcopy(claim)
        changed['placements'][0]['reward']['items'][2]['count'] = 15
        self.assertIsNone(container_native.derive(changed, items, plain.stack_problem))

    def test_draconia_bag_keeps_the_one_charge_ice_rapier_and_serpent_sword(self):
        claim = next(d for d in self.rows if d['identity']['key'] ==
                     'oteryn:reward-claim.kv.draconia1')
        held_items = copy.deepcopy(self.items)
        held_items['oteryn:item.tibia.i3284']['materializable'] = False
        items = container_native.claim_items(claim['provenance']['pilot_key'], held_items)
        recipe = container_native.derive(claim, items, plain.stack_problem)
        self.assertIsNotNone(recipe)
        self.assertEqual(recipe['placements'][0]['unique_id'], 6023)
        self.assertEqual([(c['item']['key'], c['count']) for c in recipe['placements'][0]['contents']],
                         [('oteryn:item.tibia.i3284', 1), ('oteryn:item.tibia.i3297', 1)])
        key = 'oteryn:item.tibia.i3284'
        self.assertFalse(held_items[key]['materializable'])
        profiles = json.loads((plain.ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())
        profile = next(p for p in profiles['records'] if p['production_definition']['production_key'] == key)
        self.assertTrue(container_native.native_charge_item(held_items[key], profile)['materializable'])
        changed = copy.deepcopy(profile)
        changed['semantics']['weapon']['value']['attack']['value'] += 1
        self.assertIs(container_native.native_charge_item(held_items[key], changed), held_items[key])
        changed = copy.deepcopy(profile)
        changed['semantics']['charges']['value']['count']['value'] = 2
        self.assertIs(container_native.native_charge_item(held_items[key], changed), held_items[key])

    def test_fire_axe_bag_preserves_the_full_definition_charge_necklace(self):
        claim = next(d for d in self.rows if d['identity']['key'] ==
                     'oteryn:reward-claim.kv.fireaxe1')
        held_items = copy.deepcopy(self.items)
        held_items['oteryn:item.tibia.i3085']['materializable'] = False
        items = container_native.claim_items(claim['provenance']['pilot_key'], held_items)
        recipe = container_native.derive(claim, items, plain.stack_problem)
        self.assertIsNotNone(recipe)
        self.assertEqual(recipe['placements'][0]['unique_id'], 6067)
        self.assertEqual([(c['item']['key'], c['count']) for c in recipe['placements'][0]['contents']],
                         [('oteryn:item.tibia.i3098', 1), ('oteryn:item.tibia.i3085', 1),
                          ('oteryn:item.tibia.i3028', 7), ('oteryn:item.tibia.i3320', 1)])
        self.assertFalse(held_items['oteryn:item.tibia.i3085']['materializable'])
        self.assertEqual(items['oteryn:item.tibia.i3085']['semantics']['charges']['value']['count']['value'], 200)
        changed = copy.deepcopy(claim)
        changed['placements'][0]['reward']['items'][1]['count'] = 199
        self.assertIsNone(container_native.derive(changed, items, plain.stack_problem))

    def test_dreamer_wine_keeps_selected_wine_in_all_alternatives(self):
        claim = next(d for d in self.rows if d['identity']['key'] ==
                     'oteryn:reward-claim.quest.u7_9.dreamers_challenge.chests_wine')
        held_items = copy.deepcopy(self.items)
        held_items['oteryn:item.tibia.i2874']['materializable'] = False
        items = container_native.claim_items(claim['provenance']['pilot_key'], held_items)
        recipe = container_native.derive(claim, items, plain.stack_problem)
        self.assertIsNotNone(recipe)
        self.assertEqual([p['unique_id'] for p in recipe['placements']], [6197, 6198, 6199, 6200])
        for placement in recipe['placements']:
            self.assertEqual([c.get('fluid_subtype') for c in placement['contents']], [None, 2, None])
            self.assertEqual([c['count'] for c in placement['contents']], [1, 1, 1])
        self.assertFalse(held_items['oteryn:item.tibia.i2874']['materializable'])
        changed = copy.deepcopy(claim)
        changed['placements'][0]['reward']['items'][1]['count'] = 15
        self.assertIsNone(container_native.derive(changed, items, plain.stack_problem))

    def test_water_vial_requires_an_agreeing_native_profile_and_exact_claim(self):
        key = 'oteryn:item.tibia.i2874'
        original = copy.deepcopy(self.items[key])
        original['materializable'] = False
        profiles = json.loads((plain.ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())
        profile = next(p for p in profiles['records'] if p['production_definition']['production_key'] == key)
        self.assertTrue(container_native.native_water_vial(original, profile)['materializable'])
        self.assertIs(container_native.claim_items('canary:reward-claim/other', self.items), self.items)
        for change in ('weight', 'admission', 'fluid'):
            changed = copy.deepcopy(profile)
            if change == 'weight': changed['semantics']['physical']['value']['weight']['value'] += 1
            elif change == 'admission': changed['admission']['materializable'] = False
            else: changed['semantics']['fluid']['value']['fluid_type'] = {'state': 'KNOWN', 'value': 1}
            self.assertIs(container_native.native_water_vial(original, changed), original)

    def test_poison_daggers_target_has_two_distinct_nonstackable_children(self):
        definition = next(d for d in self.rows
                          if d['identity']['key'] == 'oteryn:reward-claim.kv.poisondaggers')
        recipe = container_native.derive(definition, self.items, plain.stack_problem)
        self.assertIsNotNone(recipe)
        self.assertEqual([(q['item']['key'], q['count'])
                          for q in recipe['placements'][0]['contents']],
                         [('oteryn:item.tibia.i3448', 30),
                          ('oteryn:item.tibia.i3299', 1),
                          ('oteryn:item.tibia.i3299', 1)])

    def test_bear_room_bag_keeps_both_rewards_and_source_storage(self):
        recipe = container_native.derive(self.bear, self.items, plain.stack_problem)
        self.assertEqual(recipe['revision'], '201189f871d23597b82caf88fd611454a2ddefb8')
        placement = recipe['placements'][0]
        self.assertEqual(placement['root'], self.items['oteryn:item.tibia.i2853']['identity'])
        self.assertEqual(placement['capacity'], 8)
        self.assertEqual(placement['unique_id'], 6114)
        self.assertEqual([(q['item']['key'], q['count']) for q in placement['contents']],
                         [('oteryn:item.tibia.i3447', 12), ('oteryn:item.tibia.i3031', 40)])
        self.assertEqual(placement['completion'], {'kind': 'quest_kv', 'key': 'bearChest1'})

    def fluid_items(self):
        # Copy only the vessels changed by negative cases; positive admission
        # must use the actual generated Item facts.
        items = self.items.copy()
        for number in (2874, 2881, 2882):
            key = f'oteryn:item.tibia.i{number}'
            items[key] = copy.deepcopy(items[key])
        return items

    def test_wiki_milk_wine_and_present_donor_fills_preserve_source(self):
        cases = (
            ('kv.goblintemple2', 'WIKI_MILK_OVERRIDES_DONOR_WATER', [6119], [(2874, 9)]),
            ('quest.u7_9.dreamers_challenge.chests_wine',
             'WIKI_WINE_OVERRIDES_DONOR_WATER', [6197, 6198, 6199, 6200], [(2874, 2)]),
            ('kv.present_box', 'CRYSTAL_PRESENT_WATER_SUBTYPES', [6113], [(2882, 1), (2881, 1)]),
        )
        for suffix, choice, uids, fills in cases:
            definition = next(d for d in self.rows if d['identity']['key'].endswith(suffix))
            before = copy.deepcopy(definition)
            with self.subTest(claim=suffix):
                native = container_native.derive(definition, self.fluid_items(), plain.stack_problem)
                self.assertIsNotNone(native)
                self.assertEqual(native['source_difference'], choice)
                self.assertEqual([p['unique_id'] for p in native['placements']], uids)
                for source, placement in zip(definition['placements'], native['placements']):
                    self.assertEqual([{k: v for k, v in child.items() if k != 'fluid_subtype'}
                                      for child in placement['contents']], source['reward']['items'])
                    self.assertEqual([(container_native.item_id(child['item']), child['fluid_subtype'])
                                      for child in placement['contents'] if 'fluid_subtype' in child], fills)
                self.assertEqual(definition, before, 'historical SOURCE tuples and holds stay intact')
                if 'chests_wine' in suffix:
                    self.assertTrue(all(p['completion'] == {
                        'kind': 'storage', 'key': 'Storage.Quest.U7_9.DreamersChallenge.ChestsWine'}
                        for p in native['placements']))

    def test_fluid_choice_never_clears_sibling_holds_or_unproven_item_facts(self):
        definition = next(d for d in self.rows if d['identity']['key'].endswith('kv.goblintemple2'))
        for mutation in ('hold', 'source', 'quantity', 'position', 'uid', 'identity', 'fluid', 'charges', 'unadmitted'):
            changed = copy.deepcopy(definition)
            items = self.fluid_items()
            if mutation == 'hold':
                changed['data_holds'].append({'category': 'source', 'code': 'UNRESOLVED'})
            elif mutation == 'source':
                changed['source_variant']['source_data']['status'] = 'WAITING_SOURCE'
            elif mutation == 'quantity':
                changed['placements'][0]['reward']['items'][2]['count'] = 9
            elif mutation == 'position':
                changed['placements'][0]['source_binding']['project_position']['x'] += 1
            elif mutation == 'uid':
                changed['placements'][0]['source_binding']['legacy_unique_ids'][1]['unique_id'] = 6118
            elif mutation == 'identity':
                changed['identity']['key'] = 'oteryn:reward-claim.kv.invented'
            elif mutation == 'fluid':
                items['oteryn:item.tibia.i2874']['semantics']['fluid'] = {'state': 'UNKNOWN'}
            elif mutation == 'charges':
                items['oteryn:item.tibia.i2874']['semantics']['charges'] = {
                    'state': 'KNOWN', 'value': {'count': {'state': 'KNOWN', 'value': 9}}}
            else:
                items['oteryn:item.tibia.i2874']['materializable'] = False
            with self.subTest(mutation=mutation):
                self.assertIsNone(container_native.derive(changed, items, plain.stack_problem))

    def test_medusa_uses_complete_crystal_reward_without_canary_fox_text(self):
        definition = next(d for d in self.rows
                          if d['identity']['key'].endswith('quest.u7_3.the_medusa_quest'))
        before = copy.deepcopy(definition)
        native = container_native.derive(definition, self.items, plain.stack_problem)
        self.assertIsNotNone(native)
        self.assertEqual(native['source_difference'], 'CRYSTAL_MEDUSA_WITHOUT_CANARY_TEXT')
        placement = native['placements'][0]
        self.assertEqual(placement['unique_id'], 6183)
        self.assertEqual(placement['root']['key'], 'oteryn:item.tibia.i2853')
        self.assertEqual([(child['item']['key'], child['count']) for child in placement['contents']],
                         [(f'oteryn:item.tibia.i{i}', 1) for i in (3436, 3567, 3324)])
        self.assertNotIn('root_text', placement)
        self.assertTrue(all('text' not in child for child in placement['contents']))
        self.assertEqual(definition, before, 'the original Canary SOURCE hold stays intact')

    def test_medusa_reconciliation_cannot_clear_a_sibling_hold_or_invent_a_carrier(self):
        definition = next(d for d in self.rows
                          if d['identity']['key'].endswith('quest.u7_3.the_medusa_quest'))
        for mutation in ('hold', 'carrier', 'quantity'):
            changed = copy.deepcopy(definition)
            if mutation == 'hold':
                changed['data_holds'].append({'category': 'item', 'code': 'ITEM_SEMANTICS_MISSING'})
            elif mutation == 'carrier':
                changed['placements'][0]['reward']['written_text']['item'] = self.items[
                    'oteryn:item.tibia.i3436']['identity']
            else:
                changed['placements'][0]['reward']['items'][0]['count'] = 2
            self.assertIsNone(container_native.derive(changed, self.items, plain.stack_problem), mutation)

    def test_no_recipe_for_unsourced_quantity_position_uid_or_container(self):
        for mutation in ('quantity', 'position', 'unique', 'root'):
            d = copy.deepcopy(self.bear)
            p = d['placements'][0]
            if mutation == 'quantity': p['reward']['items'][0]['count'] += 1
            elif mutation == 'position': p['source_binding']['project_position']['x'] += 1
            elif mutation == 'unique':
                next(u for u in p['source_binding']['legacy_unique_ids'] if u['server'] == 'crystalserver')['unique_id'] += 1
            else: p['reward']['container'] = self.items['oteryn:item.tibia.i2854']['identity']
            with self.subTest(mutation=mutation):
                self.assertIsNone(container_native.derive(d, self.items, plain.stack_problem))

    def test_unadmitted_or_overfull_items_do_not_silently_disappear(self):
        for mutation in ('unadmitted', 'capacity', 'unknown_stack'):
            items = copy.deepcopy(self.items)
            if mutation == 'unadmitted': items['oteryn:item.tibia.i3447']['materializable'] = False
            elif mutation == 'capacity': items['oteryn:item.tibia.i2853']['semantics']['container']['value']['capacity']['value'] = 1
            else: items['oteryn:item.tibia.i3447']['stack_class'] = 'Unknown'
            with self.subTest(mutation=mutation):
                self.assertIsNone(container_native.derive(self.bear, items, plain.stack_problem))

    def test_held_data_and_unimplemented_reward_shapes_are_not_admitted(self):
        for mutate in ('hold', 'cooldown', 'text', 'achievement'):
            d = copy.deepcopy(self.bear)
            if mutate == 'hold': d['data_holds'] = [{'category': 'source', 'code': 'SOURCE_CHARGE_MISMATCH'}]
            elif mutate == 'cooldown': d['claim']['repeat'] = {'kind': 'cooldown', 'hours': 24}
            elif mutate == 'text': d['placements'][0]['reward']['written_text'] = {}
            else: d['placements'][0]['achievement_grant'] = {}
            with self.subTest(mutation=mutate):
                self.assertIsNone(container_native.derive(d, self.items, plain.stack_problem))

    def test_key_action_applies_only_to_the_donor_key_item(self):
        d = next(d for d in self.rows if any(u['unique_id'] == 5004 and u['server'] == 'crystalserver'
                for p in d['placements'] for u in p['source_binding']['legacy_unique_ids']))
        recipe = container_native.derive(d, self.items, plain.stack_problem)
        self.assertIsNotNone(recipe)
        key, cookies = recipe['placements'][0]['contents']
        self.assertEqual(key['key_number'], 3801)
        self.assertNotIn('key_number', cookies)

    def test_accepted_two_stack_rulings_keep_both_stacks(self):
        for uid, selected in container_native.ACCEPTED_TWO_STACKS.items():
            d = next(d for d in self.rows if any(u['unique_id'] == uid and u['server'] == 'crystalserver'
                    for p in d['placements'] for u in p['source_binding']['legacy_unique_ids']))
            recipe = container_native.derive(d, self.items, plain.stack_problem)
            quantities = [q['count'] for q in recipe['placements'][0]['contents']
                          if q['item']['key'] == f'oteryn:item.tibia.i{selected}']
            self.assertEqual(quantities, [100, 100])
            changed = copy.deepcopy(d)
            changed['placements'][0]['reward']['items'].pop()
            self.assertIsNone(container_native.derive(changed, self.items, plain.stack_problem))

    def test_annihilator_choices_share_one_storage_and_exact_achievement(self):
        d = next(d for d in self.rows if d['identity']['key'].endswith('the_annihilator.reward'))
        recipe = container_native.derive(d, self.items, plain.stack_problem)
        rows = recipe['placements']
        self.assertEqual([p['unique_id'] for p in rows], [6085, 6086, 6087, 6088])
        self.assertEqual([len(p['contents']) for p in rows], [0, 0, 0, 1])
        self.assertTrue(all(p['completion'] == {'kind': 'storage', 'key': 'Storage.Quest.U7_24.TheAnnihilator.Reward'} for p in rows))
        self.assertTrue(all(p['achievement']['key'] == 'oteryn:achievement/annihilator' for p in rows))
        changed = copy.deepcopy(d)
        changed['placements'][3].pop('achievement_grant')
        self.assertIsNone(container_native.derive(changed, self.items, plain.stack_problem))

    def test_definition_capacity_is_preserved_with_existing_direct_child_bound(self):
        for suffix in ('the_pits_of_inferno.reward_chest_bp', 'wrath_of_the_emperor.chest_bag'):
            d = next(d for d in self.rows if d['identity']['key'].endswith(suffix))
            recipe = container_native.derive(d, self.items, plain.stack_problem)
            self.assertIsNotNone(recipe)
            row = recipe['placements'][0]
            self.assertLessEqual(len(row['contents']), min(row['capacity'], 20))
        native = [d for d in self.rows if container_native.derive(d, self.items, plain.stack_problem)]
        ornament = next(d for d in self.rows
                        if d['identity']['key'] == 'oteryn:reward-claim.kv.ornamentedshield')
        self.assertIsNone(container_native.derive(ornament, self.items, plain.stack_problem))
        recipe = charge_native.derive(ornament, self.items, plain.stack_problem)
        self.assertEqual(recipe, ornament['native_contents'])
        self.assertEqual(recipe['placements'][0]['contents'][2]['charges_remaining'], 1)
        # Count the migrated exact-charge recipe alongside the generic recipes.
        native.append(ornament)
        self.assertEqual(len(native), 77)  # Includes the milk, wine, Present and Poison Daggers recipes.
        self.assertEqual(sum(any('container' in p['reward'] for p in d['placements'])
                             for d in native), 64)

    def test_all_twelve_text_leaves_keep_the_established_lf_normalization(self):
        evidence, _ = container_native.source()
        texts = {r['unique_id']: r for r in evidence['text_entries'] if r['unique_id'] != 6195}
        count = trimmed = 0
        for d in self.rows:
            ids = [u['unique_id'] for p in d['placements'] for u in p['source_binding']['legacy_unique_ids']
                   if u['server'] == 'crystalserver']
            if len(ids) != 1 or ids[0] not in texts:
                continue
            recipe = container_native.derive(d, self.items, plain.stack_problem)
            self.assertIsNotNone(recipe)
            row = recipe['placements'][0]
            body = texts[ids[0]]['text']
            self.assertEqual(row['root_text'], body.strip('\n'))
            self.assertEqual(row['contents'], [])
            self.assertIsNone(row['capacity'])
            self.assertEqual(row['root']['key'], f"oteryn:item.tibia.i{texts[ids[0]]['item_id']}")
            count += 1
            trimmed += body != row['root_text']
            changed = copy.deepcopy(d)
            changed['placements'][0]['reward']['written_text']['text_ref']['sha256'] = '0' * 64
            self.assertIsNone(container_native.derive(changed, self.items, plain.stack_problem))
        self.assertEqual((count, trimmed), (12, 5))

    def test_crystal_key3002_uses_the_exact_existing_leaf_key_owner(self):
        d = next(d for d in self.rows if d['identity']['key'].endswith('quest.key.id3002'))
        recipe = container_native.derive(d, self.items, plain.stack_problem)
        self.assertIsNotNone(recipe)
        row = recipe['placements'][0]
        self.assertEqual((row['unique_id'], row['root']['key'], row['contents']),
                         (6093, 'oteryn:item.tibia.i2969', []))
        self.assertEqual(row['completion'], {'kind': 'storage', 'key': 'Storage.Quest.Key.ID3002'})
        for mutation in ('number', 'count', 'position'):
            changed = copy.deepcopy(d)
            p = changed['placements'][0]
            if mutation == 'number': p['reward']['key_binding'] = 'crystalserver:door-key/3003'
            elif mutation == 'count': p['reward']['items'][0]['count'] = 2
            else: p['source_binding']['project_position']['x'] += 1
            self.assertIsNone(container_native.derive(changed, self.items, plain.stack_problem))

    def test_full_charge_leaf_never_admits_a_partial_or_held_definition(self):
        item = self.items['oteryn:item.tibia.i3081']
        key = 'canary:reward-claim/kv/madmageroom3'
        position = {'x': 32572, 'y': 32195, 'z': 14}
        proof = plain.load_source_charge_evidence()
        self.assertEqual(plain.charged_source_reward(item, key, position,
                         item['identity']['key'], 5, proof), (1, None))
        for mutation in ('count', 'charges', 'admission', 'position', 'claim'):
            changed = copy.deepcopy(item); point = dict(position); raw = 5; claim = key
            if mutation == 'count': raw = 1
            elif mutation == 'charges': changed['semantics']['charges']['value']['count']['value'] = 6
            elif mutation == 'admission': changed['materializable'] = False
            elif mutation == 'position': point['x'] += 1
            else: claim += 'forged'
            self.assertFalse(container_native.full_charge_leaf(changed, claim, point,
                             item['identity']['key'], raw), mutation)

    def test_every_admitted_default_matching_charge_leaf_uses_one_instance(self):
        proof = plain.load_source_charge_evidence()
        for source in proof:
            item = self.items[source['item']]
            if (not item['materializable'] or source['source_count_argument']
                    != source['source_default_charges']):
                continue
            with self.subTest(claim=source['pilot_key']):
                self.assertEqual(plain.charged_source_reward(
                    item, source['pilot_key'], source['project_position'], source['item'],
                    source['source_count_argument'], proof), (1, None))

    def test_authored_text_is_bound_only_to_the_exact_source_book(self):
        d = next(d for d in self.rows if d['identity']['key'].endswith('dreamers_challenge.bplong_sword'))
        recipe = container_native.derive(d, self.items, plain.stack_problem)
        children = recipe['placements'][0]['contents']
        self.assertEqual([q['item']['key'] for q in children],
                         [f'oteryn:item.tibia.i{i}' for i in (2816, 3285, 3352, 3558)])
        self.assertEqual([i for i, q in enumerate(children) if 'text' in q], [0])
        body = children[0]['text']
        self.assertEqual(len(body.encode()), 478)
        self.assertEqual(hashlib.sha256(body.encode()).hexdigest(),
                         '484f70e524471dc25e2dc695deae9e697aa11497a1d6d7d6d32b2e79f3d7f476')
        for mutation in ('missing', 'hash', 'carrier', 'quantity', 'revision'):
            changed = copy.deepcopy(d)
            reward = changed['placements'][0]['reward']
            if mutation == 'missing': reward.pop('written_text')
            elif mutation == 'hash': reward['written_text']['text_ref']['sha256'] = '0' * 64
            elif mutation == 'carrier': reward['written_text']['item'] = reward['items'][1]['item']
            elif mutation == 'quantity': reward['items'][0]['count'] = 2
            else: reward['written_text']['item']['revision'] = 'invented'
            with self.subTest(mutation=mutation):
                self.assertIsNone(container_native.derive(changed, self.items, plain.stack_problem))


if __name__ == '__main__':
    unittest.main()
