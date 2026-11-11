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
        cls.merged_items = plain.load_items()
        # Replay main's historical Item holds independently of QUESTPC8's
        # subsequent source-backed global admission of these same identities.
        cls.items = copy.deepcopy(cls.merged_items)
        for number in container_native.CHARGED_QUEST_CLAIMS.values():
            cls.items[f'oteryn:item.tibia.i{number}']['materializable'] = False
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
        before = copy.deepcopy(self.items)
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
        self.assertEqual(self.items, before, 'source catalogue must remain unchanged')

    def test_native_charge_admission_requires_exact_profile_and_all_known_facts(self):
        profiles = plain.load_json(plain.ROOT / 'content/items/definitions/spell-native-profiles.json')
        key = 'oteryn:item.tibia.i3083'
        item = copy.deepcopy(self.items[key])
        # Test admission of a held item even when the merged canonical item is ready.
        item['materializable'] = False
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

    def test_merged_charge_admissions_are_idempotent_and_never_mutate_sources(self):
        before = copy.deepcopy(self.merged_items)
        for claim in container_native.CHARGED_QUEST_CLAIMS:
            once = container_native.claim_items(claim, self.merged_items)
            self.assertEqual(container_native.claim_items(claim, once), once)
        self.assertEqual(self.merged_items, before)
        profiles = plain.load_json(plain.ROOT / 'content/items/definitions/spell-native-profiles.json')
        key = 'oteryn:item.tibia.i3083'
        profile = copy.deepcopy(next(p for p in profiles['records']
                                    if p['production_definition']['production_key'] == key))
        profile['semantics']['charges']['value']['count']['value'] = 149
        self.assertTrue(self.merged_items[key]['materializable'])
        self.assertFalse(container_native.native_charge_item(self.merged_items[key], profile)['materializable'])
        self.assertEqual(self.merged_items, before)

    def test_qualification_changes_only_the_merged_owned_claims(self):
        held_items = copy.deepcopy(self.items)
        for item in container_native.CHARGED_QUEST_CLAIMS.values():
            held_items[f'oteryn:item.tibia.i{item}']['materializable'] = False
        # This historical fixture holds charge Items only; the current empty
        # vial remains unchanged while exact source-scoped fluid proofs apply.
        self.assertEqual(held_items['oteryn:item.tibia.i2874'],
                         self.merged_items['oteryn:item.tibia.i2874'])
        after = self.build(held_items)
        variants_after = variant.derive(plain.ROOT, held_items, plain.stack_problem)
        with patch.object(container_native, 'claim_items', side_effect=lambda _key, items: items):
            before = self.build(held_items)
            variants_before = variant.derive(plain.ROOT, held_items, plain.stack_problem)
        self.assertEqual({key for key in before if before[key] != after[key]}, {
            'oteryn:reward-claim.kv.timering3', 'oteryn:reward-claim.kv.trollcave1',
            'oteryn:reward-claim.kv.weddingring1',
        })
        self.assertEqual([r['definition']['identity']['key']
                          for r, old in zip(variants_after, variants_before) if r != old],
                         ['oteryn:reward-claim.kv.desert2', 'oteryn:reward-claim.kv.draconia1',
                          'oteryn:reward-claim.kv.fireaxe1', 'oteryn:reward-claim.kv.goblintemple2',
                          'oteryn:reward-claim.kv.triangletower',
                          'oteryn:reward-claim.quest.u7_9.dreamers_challenge.chests_wine'])


if __name__ == '__main__':
    unittest.main()
