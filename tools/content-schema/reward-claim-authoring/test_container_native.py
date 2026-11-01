"""W15b: only complete source-backed recipes reach the real container mint."""
import copy
import hashlib
import json
import unittest

import container_native
import reward_claim_authoring as plain


class ContainerNativeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.items = plain.load_items()
        cls.rows = [r['definition'] for p in (plain.ROOT / plain.CONTENT_DIR).glob('reward-claim-variants-*.json')
                    for r in json.loads(p.read_text())['records']]
        cls.bear = next(d for d in cls.rows if d['identity']['key'] == 'oteryn:reward-claim.kv.bear_chest1')

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
        self.assertEqual(len(native), 67)
        self.assertEqual(sum(any('container' in p['reward'] for p in d['placements'])
                             for d in native), 54)

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
