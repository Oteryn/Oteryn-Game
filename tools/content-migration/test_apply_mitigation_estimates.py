import copy
import unittest

import apply_mitigation_estimates as adoption


class BalanceAdoptionTests(unittest.TestCase):
    def test_completed_value_removes_stale_unknown_and_keeps_other_flags(self):
        flags = adoption.completed_flags({'completion_flags': ['MITIGATION_UNKNOWN', 'UNSUPPORTED_CALLBACK']},
                                         {'out_of_distribution': True})
        self.assertNotIn('MITIGATION_UNKNOWN', flags)
        self.assertIn(adoption.QUALIFICATION, flags)
        self.assertIn('UNSUPPORTED_CALLBACK', flags)
        self.assertIn('MITIGATION_ESTIMATE_OUT_OF_DISTRIBUTION', flags)

    def setUp(self):
        folder = adoption.population.MONSTERS / 'samples/canary-47dfd51f/rat'
        self.documents = [adoption.read(folder / f) for f in adoption.population.FILES]
        self.documents[0]['creature']['stats'].pop('mitigation_percent', None)
        self.estimate = {'monster': 'rat', 'identity': self.documents[0]['creature']['identity'],
                         'qualification': adoption.QUALIFICATION, 'evidence_kind': 'DERIVED',
                         'value_percent': 0.05, 'value_ratio': {'numerator': 1, 'denominator': 20},
                         'method': 'regression fixture', 'policy': 'bounded estimate'}

    def test_fill_only_absent_value_and_keep_honest_provenance(self):
        before = copy.deepcopy(self.documents)
        result = adoption.patch(self.documents, self.estimate, 'a' * 64, 1)
        self.assertEqual(before, self.documents)
        restored = copy.deepcopy(result[0])
        del restored['creature']['stats']['mitigation_percent']
        self.assertEqual(before[0], restored)
        self.assertEqual(before[1:3], result[1:3])
        source = result[3]['sources'][-1]
        self.assertFalse(source['global_parity'])
        self.assertEqual(source['evidence_classification'], 'DERIVED')
        self.assertEqual(source['qualification'], adoption.QUALIFICATION)

    def test_existing_zero_is_preserved(self):
        self.documents[0]['creature']['stats']['mitigation_percent'] = {'numerator': 0, 'denominator': 1}
        with self.assertRaisesRegex(ValueError, 'existing'):
            adoption.patch(self.documents, self.estimate, 'a' * 64, 1)

    def test_wrong_identity_unaccepted_or_inconsistent_value_rejected(self):
        for mutation in ('identity', 'qualification', 'value'):
            estimate = copy.deepcopy(self.estimate)
            if mutation == 'identity':
                estimate['identity']['key'] = 'canary:creature/other'
            elif mutation == 'qualification':
                estimate['qualification'] = 'GLOBAL_VERIFIED'
            else:
                estimate['value_percent'] = 0.06
            with self.assertRaises(ValueError):
                adoption.patch(self.documents, estimate, 'a' * 64, 1)

    def test_source_schema_rejects_claim_of_global_parity(self):
        result = adoption.patch(self.documents, self.estimate, 'a' * 64, 1)
        result[3]['sources'][-1]['global_parity'] = True
        self.assertTrue(adoption.population.validator.validate(*result))


if __name__ == '__main__':
    unittest.main()
