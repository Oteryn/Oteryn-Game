import copy
import json
from pathlib import Path
import unittest
import audit


class ReadinessTests(unittest.TestCase):
    def test_chosen_flags_are_independent_of_source_gaps(self):
        d = dict(readiness='waiting_data', missing_data=[dict(code='reported_source_gap')],
                 oteryn_recipe=dict(profile='chosen_source_completion_v1', chosen_data_complete=True,
                                     runtime_enabled=False, readiness='waiting_native_bindings'))
        original = copy.deepcopy(d)
        r = audit.profile_readiness(d)
        self.assertEqual(r['original_source_readiness'], 'waiting_data')
        self.assertEqual(r['chosen_data_readiness'], 'waiting_native_bindings')
        self.assertFalse(r['runtime_enabled'])
        self.assertFalse(r['native_admission'])
        self.assertEqual(d, original)
        for field, bad in [('profile', 'unknown'), ('chosen_data_complete', False), ('runtime_enabled', True), ('readiness', 'ready')]:
            changed = copy.deepcopy(d)
            changed['oteryn_recipe'][field] = bad
            self.assertFalse(audit.profile_readiness(changed)['chosen_data_complete'])

    def test_existing_track_artifact_cannot_admit_execution(self):
        result = audit.classify_issue(dict(code='quest_native_lowering_missing'), {}, dict(tracks=['present']))
        self.assertTrue(result['artifact_present'])
        self.assertEqual(result['derived_status'], 'TRACK_ARTIFACT_PRESENT_EXECUTION_UNPROVEN')
        self.assertEqual(result['category'], 'execution_binding')

    def test_source_discrepancy_not_silently_closed(self):
        for code in audit.SOURCE:
            result = audit.classify_issue(dict(code=code), {}, None)
            self.assertEqual(result['category'], 'preserved_source')
            self.assertEqual(result['derived_status'], 'SOURCE_HOLD_NOT_CHOSEN_DATA_HOLE')

    def test_item_hold_live_vs_stale(self):
        issue = dict(code='claim_item_semantics_missing', source_key='source:key')
        result = audit.classify_issue(issue, {'source:key':dict(readiness='ready')}, None)
        self.assertEqual(result['derived_status'], 'STALE_OR_UNPROVEN_ITEM_HOLD')
        self.assertFalse(result['actual_data_hole'])
        for claim in [dict(readiness='waiting_item_semantics'), dict(readiness='waiting_data', data_holds=[dict(category='item')])]:
            self.assertTrue(audit.classify_issue(issue, {'source:key':claim}, None)['actual_data_hole'])

    def test_unknown_code_cannot_be_complete(self):
        self.assertTrue(audit.classify_issue(dict(code='new_unknown_gap'), {}, None)['actual_data_hole'])

    def test_qualified_all242_and_counts(self):
        report = json.loads(Path(__file__).with_name('audit.json').read_text())
        self.assertEqual(len(report['records']), 242)
        self.assertEqual(len({r['quest_key'] for r in report['records']}), 242)
        self.assertEqual(report['counts']['chosen_data_complete'], 242)
        self.assertEqual(report['counts']['chosen_recipe_data_holes'], 0)
        self.assertEqual(report['counts']['item_or_unclassified_hold_quests'], 21)
        self.assertEqual(report['counts']['item_or_unclassified_hold_refs'], 22)
        for record in report['records']:
            self.assertEqual(len(record['original_missing_data']), len(record['gap_resolution']))
            self.assertEqual(record['original_missing_data'], [r['issue'] for r in record['gap_resolution']])
            self.assertFalse(record['runtime_enabled'])
            self.assertFalse(record['source_completion_claimed'])


if __name__ == '__main__':
    unittest.main()
