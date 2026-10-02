import copy
import unittest

from quest_rollout_authoring import build


class RolloutTests(unittest.TestCase):
    def setUp(self):
        self.source = {'identity': {'key': 'canary:quest/example', 'revision': 'source-r1'}}
        self.row = {'wiki_title': 'Example Quest', 'source': {'pageid': 1, 'revid': 2},
                    'authored_candidates': [self.source], 'family_representation': None,
                    'fresh_sources': []}
        self.bundle = {'quests': [self.source], 'quest_gaps': [{'quest': 'canary:quest/example', 'gaps': []}]}
        self.specs = {'titles': 1, 'no_runtime_promotion': True, 'source_definition_complete': False,
                      'runtime_readiness': 'UNKNOWN',
                      'entries': [{'wiki_title': 'Example Quest', 'source_refs': [], 'unresolved': [],
                                   'definition_complete': False, 'runtime_readiness': 'UNKNOWN',
                                   'source_field_listing_complete': False}],
                      'coverage': [{'wiki_title': 'Example Quest', 'proof_level': 'PINNED_STRUCTURED_FIELDS_ONLY',
                                    'full_walkthrough_complete': False, 'raw_body_rechecked': False}]}
        self.definition = {'identity': {'key': 'oteryn:quest.example', 'revision': 'quest-r1'},
                           'source_refs': {'quest': {'key': 'canary:quest/example'}},
                           'readiness': 'definition_ready', 'missing_data': []}

    def result(self, row=None, definitions=None):
        return build({'quests': [row or self.row]}, self.bundle,
                     [self.definition] if definitions is None else definitions, self.specs)['records'][0]

    def test_field_ready_never_enables_runtime_or_claims_smoke(self):
        row = self.result()
        self.assertTrue(row['definition_fields_ready'])
        self.assertFalse(row['runtime_enabled'])
        self.assertIn('needs_runtime', row['flags'])
        self.assertEqual(set(row['smoke_verification'].values()), {'NOT_RUN'})
        self.assertFalse(row['approximation_applied'])

    def test_unbound_title_retained_with_honest_gaps(self):
        row = copy.deepcopy(self.row)
        row['authored_candidates'] = []
        result = self.result(row, [])
        self.assertEqual(result['binding_scope'], 'unbound')
        self.assertIn('needs_source', result['flags'])
        self.assertNotIn('imported', result['flags'])
        self.assertEqual(result['canonical_quest_refs'], [])

    def test_family_is_partial_even_when_owner_definition_ready(self):
        row = copy.deepcopy(self.row)
        row['authored_candidates'] = []
        row['family_representation'] = {'target_key': 'canary:quest/example'}
        result = self.result(row)
        self.assertEqual(result['binding_scope'], 'family')
        self.assertIn('partial', result['flags'])
        self.assertEqual(result['known_gaps'][0]['code'], 'partial_mission_family')

    def test_definition_and_source_gaps_both_preserved(self):
        self.bundle['quest_gaps'][0]['gaps'] = [{'kind': 'source_progress_unknown'}]
        self.definition['readiness'] = 'waiting_data'
        self.definition['missing_data'] = [{'code': 'claim_native_lowering_missing'}]
        result = self.result()
        self.assertFalse(result['definition_fields_ready'])
        self.assertEqual([g['code'] for g in result['known_gaps']], ['source_gap', 'definition_gap'])

    def test_missing_source_or_duplicate_binding_rejected(self):
        self.bundle['quests'] = []
        with self.assertRaisesRegex(ValueError, 'absent source'):
            self.result()
        with self.assertRaisesRegex(ValueError, 'duplicate canonical'):
            self.result(definitions=[self.definition, self.definition])

    def test_source_holds_and_proof_level_are_preserved(self):
        hold = {'field_group': 'Source requirement not parsed', 'classification': 'UNKNOWN'}
        self.specs['entries'][0]['unresolved'] = [hold]
        result = self.result()
        self.assertEqual(result['source_specification']['unresolved'], [hold])
        self.assertEqual(result['known_gaps'][0]['code'], 'source_specification_gap')
        self.assertIn('partial', result['flags'])
        self.assertTrue(result['definition_fields_ready'])

    def test_missing_duplicate_or_false_complete_source_rejected(self):
        for mutate in [lambda s: s['entries'].clear(), lambda s: s['entries'].append(s['entries'][0]),
                       lambda s: s['coverage'][0].update(full_walkthrough_complete=True),
                       lambda s: s['coverage'][0].update(raw_body_rechecked=True)]:
            previous = copy.deepcopy(self.specs)
            mutate(self.specs)
            with self.assertRaises(ValueError):
                self.result()
            self.specs = previous

    def test_duplicate_title_rejected(self):
        with self.assertRaisesRegex(ValueError, 'duplicate wiki'):
            build({'quests': [self.row, self.row]}, self.bundle, [], self.specs)


if __name__ == '__main__':
    unittest.main()
