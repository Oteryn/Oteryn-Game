"""A field-qualified Wiki observation must not authorize unrelated data changes."""
import copy
import unittest

import canary_batch as cb


class MitigationAdoptionScope(unittest.TestCase):
    def setUp(self):
        self.record = {
            'status': 'COMPARED', 'qualified_fields': ['mitigation_percent'],
            'wiki_title': 'Orc Warlord', 'page_id': 123, 'cut_revision_id': 456,
            'cut_content_sha256': 'a' * 64,
            'rows': [{'field': 'mitigation_percent', 'status': 'DIFF', 'canary': None,
                      'wiki': 2.31, 'wiki_raw': '2.31', 'wiki_line': 8}],
        }
        self.monster = {'creature': {'stats': {'max_health': 950, 'experience': 670},
                                    'summoning': {'summonable': False, 'convinceable': False}},
                        'behavior': {}, 'presentation': {}}
        self.rows = [{'source_index': 0, 'source_file': 'monster/quest_orc.lua',
                      'source_field': 'maxHealth', 'status': 'mapped'}]
        self.sources = [{'repository': cb.REPOSITORY, 'revision': cb.REVISION}]

    def adopt(self):
        converter = object.__new__(cb.Converter)
        converter.wiki = {'quest_orc': self.record}
        converter.adopt_wiki('quest_orc', self.monster, self.rows, self.sources, set(), lambda _: 1)

    def rejected_without_mutation(self):
        before = copy.deepcopy((self.monster, self.rows, self.sources))
        with self.assertRaises(ValueError):
            self.adopt()
        self.assertEqual(before, (self.monster, self.rows, self.sources))

    def test_scoped_value_updates_only_mitigation(self):
        before = copy.deepcopy(self.monster)
        self.adopt()
        before['creature']['stats']['mitigation_percent'] = {'numerator': 231, 'denominator': 100}
        self.assertEqual(before, self.monster)
        mapped = [r for r in self.rows if r.get('source_index') == 1]
        self.assertEqual(['/monster/creature/stats/mitigation_percent'],
                         [r['destination'] for r in mapped])

    def test_explicit_zero_is_preserved_as_a_value(self):
        self.record['rows'][0].update(wiki=0, wiki_raw='0.00')
        self.adopt()
        self.assertEqual({'numerator': 0, 'denominator': 1},
                         self.monster['creature']['stats']['mitigation_percent'])

    def test_extra_health_row_cannot_escalate_field_scope(self):
        self.record['rows'].append({'field': 'max_health', 'status': 'DIFF', 'wiki': 10000})
        self.rejected_without_mutation()

    def test_replaced_field_scope_is_rejected(self):
        self.record['qualified_fields'] = ['max_health']
        self.rejected_without_mutation()

    def test_loot_and_ability_payloads_cannot_escalate_field_scope(self):
        for key in ('loot_chances', 'loot_statistics', 'abilities'):
            with self.subTest(key=key):
                self.record[key] = [{'unqualified': True}]
                self.rejected_without_mutation()
                self.record.pop(key)

    def test_boolean_nonfinite_and_uncertain_numeric_values_are_rejected(self):
        for value in (True, False, float('nan'), float('inf'), -0.01, 100.01, '1-2%', None):
            with self.subTest(value=value):
                self.record['rows'][0]['wiki'] = value
                self.rejected_without_mutation()


if __name__ == '__main__':
    unittest.main()
