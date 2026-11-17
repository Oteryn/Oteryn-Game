"""A scoped full-charge reward needs exact canonical, native and source agreement."""
import copy
import json
import unittest

import container_native as native
import reward_claim_authoring as claims

PILOT = 'canary:reward-claim/kv/desert2'
KEY = 'oteryn:item.tibia.i3084'


class SourceChargedAdmission(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.items = claims.load_items()
        # Replay the historical held Item independently of its live admission.
        cls.items[KEY]['materializable'] = False
        rows = json.loads((claims.ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())['records']
        cls.profile = next(r for r in rows if r['production_definition']['production_key'] == KEY)

    def test_projection_is_scoped_and_never_rewrites_the_global_item(self):
        before = copy.deepcopy(self.items[KEY])
        projected = native.claim_items(PILOT, self.items)
        self.assertFalse(before['materializable'])
        self.assertTrue(projected[KEY]['materializable'])
        self.assertEqual(native.fact(projected[KEY], 'charges', 'count'), 250)
        self.assertEqual(self.items[KEY], before)
        self.assertIsNot(projected[KEY], self.items[KEY])
        self.assertEqual(native.CHARGED_QUEST_CLAIMS, {PILOT: 3084,
            'canary:reward-claim/kv/timering3': 3082,
            'canary:reward-claim/kv/triangletower': 3083,
            'canary:reward-claim/kv/trollcave1': 3083,
            'canary:reward-claim/kv/weddingring1': 3085})
        self.assertIs(native.claim_items(PILOT + '/foreign', self.items), self.items)

    def test_only_exact_full_source_charges_become_one_instance(self):
        reward = [[3084, 250], [3098, 1], [3046, 1], [3077, 1]]
        self.assertEqual(native.source_charged_reward(PILOT, reward, self.items),
                         [[3084, 1], [3098, 1], [3046, 1], [3077, 1]])
        self.assertEqual(reward[0], [3084, 250])
        for count in (0, 1, 249, 251):
            with self.subTest(count=count), self.assertRaisesRegex(ValueError, 'CHARGE_EXACT_SOURCE'):
                native.source_charged_reward(PILOT, [[3084, count]], self.items)
        for changed in ([], [[3084, 250], [3084, 250]]):
            with self.assertRaisesRegex(ValueError, 'CHARGE_SOURCE_CARDINALITY'):
                native.source_charged_reward(PILOT, changed, self.items)

    def test_each_native_binding_and_custody_fact_must_match(self):
        for change in ('charges', 'target', 'definition', 'stack', 'destinations', 'pickup'):
            profile = copy.deepcopy(self.profile)
            if change == 'charges':
                profile['semantics']['charges']['value']['count']['value'] = 249
            elif change == 'target':
                profile['production_binding']['target']['revision'] = 'foreign'
            elif change == 'definition':
                profile['production_definition']['family'] = 'Creature'
            elif change == 'stack':
                profile['admission']['stack_class'] = 'Stackable'
            elif change == 'destinations':
                profile['admission']['legal_destinations'] = ['Ground']
            else:
                profile['semantics']['physical']['value']['pickupable']['value'] = False
            with self.subTest(change=change):
                self.assertIs(native.native_charge_item(self.items[KEY], profile), self.items[KEY])
        changed = copy.deepcopy(self.items[KEY])
        changed['semantics']['charges']['value']['count']['value'] = 249
        self.assertIs(native.native_charge_item(changed, self.profile), changed)
        with self.assertRaisesRegex(ValueError, 'CHARGE_NATIVE_MATCH'):
            native.source_charged_reward(PILOT, [[3084, 250]], {**self.items, KEY: changed})

    def test_source_recipe_keeps_all_four_children_and_the_real_bag_capacity(self):
        definitions = [r['definition'] for p in (claims.ROOT / claims.CONTENT_DIR).glob('reward-claim-variants-*.json')
                       for r in json.loads(p.read_bytes())['records']]
        definition = next(d for d in definitions if d['identity']['key'] == 'oteryn:reward-claim.kv.desert2')
        self.assertEqual(definition['data_holds'], [])
        recipe = native.derive(definition, native.claim_items(PILOT, self.items), claims.stack_problem)
        self.assertEqual(recipe, definition['native_contents'])
        placement = recipe['placements'][0]
        self.assertEqual((placement['unique_id'], placement['capacity']), (6158, 8))
        self.assertEqual([(r['item']['key'], r['count']) for r in placement['contents']],
                         [(f'oteryn:item.tibia.i{i}', 1) for i in (3084, 3098, 3046, 3077)])


if __name__ == '__main__':
    unittest.main()
