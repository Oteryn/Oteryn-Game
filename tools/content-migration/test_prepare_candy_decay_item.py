import copy
import unittest
from prepare_candy_decay_item import CORPSE, MISSING, SCHEMA, correct_bundle


class CandyCorrectionTests(unittest.TestCase):
    def setUp(self):
        self.proof = {'schema': SCHEMA, 'new_item_identity_authorized': False,
                      'sources': {'canary': {}, 'crystal': {}}, 'corpse_exact_bindings': [{}]}
        self.bundle = {'monster': {'creature': {'identity': {'key': 'canary:creature/candy_horror'},
                         'corpse_item': {'key': CORPSE}, 'stats': {'max_health': 3100}},
                         'loot': {'entries': [{'item': {'key': 'canary:item/3031'}}]}},
                       'dependencies': {'items': [{'identity': {'key': CORPSE},
                           'temporal': {'decay_action': 'transform', 'duration_ms': 5000,
                                        'decay_target': {'key': MISSING}}, 'container': {'capacity': 8}},
                           {'identity': {'key': MISSING}}]},
                       'catalog': {'assets': ['canary.appearance:object/48267', 'canary.appearance:object/48296'],
                                   'definitions': [{'key': MISSING}]},
                       'manifest': {'entries': [{'destination': '/monster/creature/corpse_item', 'resolution': 'original'}]}}

    def test_keeps_original_stats_loot_corpse_and_capacity_without_mutating_input(self):
        original = copy.deepcopy(self.bundle)
        output = correct_bundle(self.bundle, self.proof)
        self.assertEqual(original, self.bundle)
        self.assertEqual(output['monster'], original['monster'])
        self.assertEqual(output['dependencies']['items'][0]['container'], {'capacity': 8})
        self.assertEqual(output['dependencies']['items'][0]['temporal'], {'decay_action': 'none', 'stop_duration': False})
        self.assertEqual(output['catalog']['assets'], ['canary.appearance:object/48267'])

    def test_rejects_hidden_loot_use_of_missing_target(self):
        self.bundle['monster']['loot']['entries'][0]['item']['key'] = MISSING
        with self.assertRaises(ValueError):
            correct_bundle(self.bundle, self.proof)

    def test_rejects_different_corpse_and_timer(self):
        for mutation in ('corpse', 'timer'):
            with self.subTest(mutation=mutation):
                bundle = copy.deepcopy(self.bundle)
                if mutation == 'corpse':
                    bundle['monster']['creature']['corpse_item']['key'] = 'canary:item/48268'
                else:
                    bundle['dependencies']['items'][0]['temporal']['duration_ms'] = 6000
                with self.assertRaises(ValueError):
                    correct_bundle(bundle, self.proof)

    def test_rejects_incomplete_or_identity_minting_proof(self):
        for field, value in [('sources', {}), ('corpse_exact_bindings', []), ('new_item_identity_authorized', True)]:
            with self.subTest(field=field):
                proof = copy.deepcopy(self.proof)
                proof[field] = value
                with self.assertRaises(ValueError):
                    correct_bundle(self.bundle, proof)


if __name__ == '__main__':
    unittest.main()
