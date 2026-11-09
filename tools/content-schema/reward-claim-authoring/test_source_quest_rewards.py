"""Exact T..Z charge rewards and their finite native Item closure."""
import copy
import unittest
from unittest.mock import patch

import container_native
import reward_claim_authoring as plain
import reward_claim_variant_authoring as variant


class SourceQuestRewards(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.items = plain.load_items()
        # Exercise historical held inputs explicitly: another owner may already
        # qualify a ring in today's catalogue. Keep every source fact intact.
        cls.items = copy.deepcopy(cls.items)
        for item_id in container_native.CHARGED_QUEST_CLAIMS.values():
            cls.items[f'oteryn:item.tibia.i{item_id}']['materializable'] = False
        cls.claims = plain.load_json(plain.ROOT / plain.CLAIMS_REL)['claims']
        cls.manifest = plain.load_json(plain.ROOT / plain.MANIFEST_REL)

    def build(self, items):
        records, _ = plain.build_records(self.claims, self.manifest, items)
        return {r['definition']['identity']['key']: r['definition'] for r in records}

    def test_draconia_rapier_qualifies_only_its_exact_existing_native_charge(self):
        key = 'oteryn:item.tibia.i3284'
        claim = 'canary:reward-claim/kv/draconia1'
        qualified = container_native.claim_items(claim, self.items)
        self.assertTrue(qualified[key]['materializable'])
        self.assertFalse(self.items[key]['materializable'])
        changed = copy.deepcopy(self.items)
        changed[key]['semantics']['charges']['value']['count']['value'] = 2
        self.assertFalse(container_native.claim_items(claim, changed)[key]['materializable'])
        self.assertFalse(container_native.claim_items('canary:reward-claim/kv/unrelated', self.items)[key]['materializable'])
        rows = variant.derive(plain.ROOT, self.items, plain.stack_problem)
        row = next(r['definition'] for r in rows
                   if r['definition']['identity']['key'] == 'oteryn:reward-claim.kv.draconia1')
        self.assertEqual(row['data_holds'], [])
        self.assertEqual(row['placements'][0]['reward']['items'], [
            {'count':1, 'item':plain.item_ref(key)},
            {'count':1, 'item':plain.item_ref('oteryn:item.tibia.i3297')},
        ])

    def test_full_charge_quest_leaves_are_one_physical_item(self):
        rows = self.build(self.items)
        for suffix, item in [('timering3', 3082), ('trollcave1', 3083), ('weddingring1', 3085)]:
            row = rows['oteryn:reward-claim.kv.' + suffix]
            with self.subTest(claim=suffix):
                self.assertEqual(row['readiness'], 'ready')
                reward = row['placements'][0]['reward']['items']
                self.assertEqual(reward, [{'count': 1, 'item': plain.item_ref(f'oteryn:item.tibia.i{item}')}])
        # Troll Cave is a source-shared choice, not two independent rewards.
        self.assertEqual(len(rows['oteryn:reward-claim.kv.trollcave1']['placements']), 2)

    def test_triangle_projection_retains_necklace_and_every_sibling(self):
        rows = variant.derive(plain.ROOT, self.items, plain.stack_problem)
        row = next(r['definition'] for r in rows
                   if r['definition']['identity']['key'] == 'oteryn:reward-claim.kv.triangletower')
        self.assertEqual(row['data_holds'], [])
        self.assertEqual(row['native_contents']['placements'][0]['contents'], [
            {'count': 1, 'item': plain.item_ref('oteryn:item.tibia.i3083')},
            {'count': 2, 'item': plain.item_ref('oteryn:item.tibia.i3029')},
            {'count': 1, 'item': plain.item_ref('oteryn:item.tibia.i3097')},
        ])

    def test_desert_projection_retains_full_protection_amulet_and_every_sibling(self):
        rows = variant.derive(plain.ROOT, self.items, plain.stack_problem)
        row = next(r['definition'] for r in rows
                   if r['definition']['identity']['key'] == 'oteryn:reward-claim.kv.desert2')
        self.assertEqual(row['data_holds'], [])
        self.assertEqual(row['native_contents']['placements'][0]['contents'], [
            {'count': 1, 'item': plain.item_ref('oteryn:item.tibia.i3084')},
            {'count': 1, 'item': plain.item_ref('oteryn:item.tibia.i3098')},
            {'count': 1, 'item': plain.item_ref('oteryn:item.tibia.i3046')},
            {'count': 1, 'item': plain.item_ref('oteryn:item.tibia.i3077')},
        ])
        self.assertFalse(self.items['oteryn:item.tibia.i3084']['materializable'])

    def test_charged_mismatch_stays_held_and_unrelated_quests_keep_their_facts(self):
        changed = copy.deepcopy(self.items)
        changed['oteryn:item.tibia.i3083']['semantics']['charges']['value']['count']['value'] = 149
        rows = self.build(changed)
        self.assertEqual(rows['oteryn:reward-claim.kv.trollcave1']['readiness'], 'waiting_item_semantics')
        original = variant.derive(plain.ROOT, self.items, plain.stack_problem)
        life_ring = next(r['definition'] for r in original
                         if r['definition']['identity']['key'] == 'oteryn:reward-claim.kv.lifering')
        self.assertNotIn('native_contents', life_ring)
        # Full-charge admission must not mutate the source-wide Item catalogue.
        self.assertFalse(self.items['oteryn:item.tibia.i3083']['materializable'])

    def test_native_charge_admission_requires_exact_profile_and_all_known_facts(self):
        profiles = plain.load_json(plain.ROOT / 'content/items/definitions/spell-native-profiles.json')
        key = 'oteryn:item.tibia.i3083'
        item = self.items[key]
        original = next(p for p in profiles['records'] if p['production_definition']['production_key'] == key)
        self.assertTrue(container_native.native_charge_item(item, original)['materializable'])
        for mutation in ('identity', 'binding', 'admission', 'charges', 'physical', 'container', 'temporal'):
            profile = copy.deepcopy(original)
            if mutation == 'identity': profile['production_definition']['revision_ref'] = 'other'
            elif mutation == 'binding': profile['production_binding']['disposition'] = 'HELD'
            elif mutation == 'admission': profile['admission']['materializable'] = False
            elif mutation == 'charges': profile['semantics']['charges']['value']['count']['value'] = 149
            elif mutation == 'physical': profile['semantics']['physical']['value']['pickupable']['value'] = False
            else: profile['semantics'][mutation] = {'state': 'UNKNOWN'}
            with self.subTest(mutation=mutation):
                self.assertFalse(container_native.native_charge_item(item, profile)['materializable'])
        self.assertFalse(item['materializable'], 'qualification must not rewrite canonical source facts')

    def test_full_charge_leaf_requires_exact_source_position_and_default(self):
        for suffix, item_id, charges, position in [
            ('timering3', 3082, 50, {'x': 33040, 'y': 32171, 'z': 9}),
            ('trollcave1', 3083, 150, {'x': 33143, 'y': 31719, 'z': 10}),
            ('weddingring1', 3085, 200, {'x': 33158, 'y': 31621, 'z': 15}),
        ]:
            claim = 'canary:reward-claim/kv/' + suffix
            key = f'oteryn:item.tibia.i{item_id}'
            item = container_native.claim_items(claim, self.items)[key]
            with self.subTest(claim=claim):
                self.assertTrue(container_native.full_charge_leaf(item, claim, position, key, charges))
                self.assertFalse(container_native.full_charge_leaf(item, claim, position, key, charges - 1))
                self.assertFalse(container_native.full_charge_leaf(item, claim, {**position, 'x': position['x'] + 1}, key, charges))
                self.assertFalse(container_native.full_charge_leaf(item, claim + '_other', position, key, charges))

    def test_qualification_changes_only_the_six_owned_claims(self):
        after = self.build(self.items)
        variants_after = variant.derive(plain.ROOT, self.items, plain.stack_problem)
        with patch.object(container_native, 'claim_items', side_effect=lambda _key, items: items):
            before = self.build(self.items)
            variants_before = variant.derive(plain.ROOT, self.items, plain.stack_problem)
        self.assertEqual({key for key in before if before[key] != after[key]}, {
            'oteryn:reward-claim.kv.timering3', 'oteryn:reward-claim.kv.trollcave1',
            'oteryn:reward-claim.kv.weddingring1',
        })
        self.assertEqual([r['definition']['identity']['key']
                          for r, old in zip(variants_after, variants_before) if r != old],
                         ['oteryn:reward-claim.kv.desert2', 'oteryn:reward-claim.kv.draconia1',
                          'oteryn:reward-claim.kv.triangletower'])


if __name__ == '__main__':
    unittest.main()
