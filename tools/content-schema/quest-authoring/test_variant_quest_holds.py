"""Registered variant references must not turn into executable quest readiness."""
import unittest

from quest_tree_authoring import build_records
from test_quest_tree_authoring import source_claim, source_quest


class VariantQuestHolds(unittest.TestCase):
    def test_canonical_registration_keeps_independent_execution_source_item_holds(self):
        for categories in ([], ['source'], ['item'], ['source', 'item']):
            with self.subTest(categories=categories):
                claim = source_claim()
                claim.update(definition_profile='authored_variant_v1',
                             readiness='waiting_data' if categories else 'waiting_implementation',
                             data_holds=[{'category': c} for c in categories])
                result = build_records([source_quest()], [claim])[0]['definition']
                self.assertEqual(result['unresolved_claims'], [])
                self.assertEqual(result['claims'][0]['key'], claim['identity']['key'])
                self.assertEqual(result['readiness'], 'waiting_data')
                codes = {i['code'] for i in result['missing_data']}
                self.assertIn('claim_native_lowering_missing', codes)
                self.assertEqual('claim_source_data_missing' in codes, 'source' in categories)
                self.assertEqual('claim_item_semantics_missing' in codes, 'item' in categories)


if __name__ == '__main__':
    unittest.main()
