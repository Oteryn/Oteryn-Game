"""Selected leaves retain complete qualification and original source evidence."""
import copy
import json
import unittest

import container_native
import reward_claim_authoring as plain
import selected_native_leaf


class SelectedNativeLeafTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.items = plain.load_items()
        cls.rows = [r['definition'] for p in (plain.ROOT / plain.CONTENT_DIR).glob('reward-claim-variants-*.json')
                    for r in json.loads(p.read_bytes())['records']]
        cls.source = next(d for d in cls.rows if d['identity']['key'] == 'oteryn:reward-claim.kv.ornamentedshield')

    def test_selected_leaf_keeps_source_siblings_and_claim(self):
        before = copy.deepcopy(self.source)
        original = container_native.derive(self.source, self.items, plain.stack_problem)
        result = selected_native_leaf.derive(self.source, self.items, plain.stack_problem,
                                            source_unique_id=6063, path=[3])
        recipe = result['placements'][0]
        self.assertEqual(recipe['contents'], original['placements'][0]['contents'])
        self.assertEqual(len(recipe['contents']), 5)
        self.assertEqual(recipe['selected_leaf']['item']['key'], 'oteryn:item.tibia.i3035')
        self.assertEqual(recipe['selected_leaf']['count'], 5)
        self.assertEqual(self.source, before)
        recipe['selected_leaf']['item']['key'] = 'changed'
        self.assertEqual(recipe['contents'][3]['item']['key'], 'oteryn:item.tibia.i3035')
        self.assertEqual(self.source, before)

    def test_uid6062_source_holds_cannot_be_bypassed(self):
        held = next(d for d in self.rows if d['identity']['key'] ==
                    'oteryn:reward-claim.quest.pre_u6_0.ornamented_shield.rewards.ornamented_shield')
        self.assertTrue(any(h['code'] == 'SOURCE_CHARGE_MISMATCH' for h in held['data_holds']))
        with self.assertRaises(ValueError):
            selected_native_leaf.derive(held, self.items, plain.stack_problem,
                                        source_unique_id=6062, path=[3])

    def test_rejects_invalid_source_uid_path_and_missing_siblings(self):
        for uid, path in [(6062, [3]), (True, [3]), (6063, []), (6063, [3, 0]),
                          (6063, [-1]), (6063, [True]), (6063, [19]), (6063, None)]:
            with self.subTest(uid=uid, path=path), self.assertRaises(ValueError):
                selected_native_leaf.derive(self.source, self.items, plain.stack_problem,
                                            source_unique_id=uid, path=path)
        bad = copy.deepcopy(self.source)
        bad['placements'][0]['reward']['items'].pop()
        with self.assertRaises(ValueError):
            selected_native_leaf.derive(bad, self.items, plain.stack_problem,
                                        source_unique_id=6063, path=[3])


if __name__ == '__main__':
    unittest.main()
