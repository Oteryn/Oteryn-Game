import json
from pathlib import Path
import unittest

import questpc2_gerimor_rules as source

ROOT = Path(__file__).resolve().parents[3]


class GerimorRules(unittest.TestCase):
    def test_source_pin_reproduces_runtime_assignment_program(self):
        for path, text in source.outputs(ROOT).items():
            self.assertEqual((ROOT / path).read_text(), text)

    def test_all_seven_choices_are_exact_and_confirm_only_the_selected_cult(self):
        doc = json.loads(source.outputs(ROOT)[source.OUTPUT])
        choices = [r for r in doc['rules'] if '/offer-' in r['key']]
        self.assertEqual({r['keywords'][0] for r in choices}, set(source.CULTS))
        self.assertEqual(len({r['effects'][0]['value'] for r in choices}), 7)
        for rule in choices:
            self.assertEqual(rule['keyword_mode'], 'exact')
            self.assertEqual(rule['topics'], [2])
        accepts = [r for r in doc['rules'] if '/accept-' in r['key']]
        self.assertEqual(len(accepts), 14)
        for rule in accepts:
            writes = [e for e in rule['effects'] if e['kind'] == 'set']
            self.assertEqual(len(writes), 2 if rule['key'].endswith('-first') else 1)
            self.assertTrue(all(e['value'] == 1 for e in writes))
            self.assertTrue(all('final_boss' not in e['track'] for e in writes))
            self.assertTrue(any(g['op'] == 'lt' and g['value'] == 1
                                and g['track'].endswith('/mission') for g in rule['guards']))
        assignment_rules = [r for r in doc['rules'] if all(
            part not in r['key'] for part in ('/report-', '/reported-', '/finale-'))]
        self.assertEqual({e['kind'] for r in assignment_rules for e in r['effects']},
                         {'set', 'set_topic'})

        reports = [r for r in doc['rules'] if '/report-' in r['key']]
        self.assertEqual({r['keywords'][0] for r in reports}, set(source.CULTS))
        self.assertEqual(sum(r['effects'][1]['amount'] for r in reports), 275_000)
        self.assertTrue(all(r['topics'] == [2] and r['keyword_mode'] == 'exact'
                            for r in reports))
        rewards = [r for r in doc['rules'] if '/finale-reward-' in r['key']]
        self.assertEqual(len(rewards), 6)
        self.assertTrue(all(r['topics'] == [4] and r['guards'][0]['value'] == 2
                            for r in rewards))
        decline = next(r for r in doc['rules'] if r['key'].endswith('/decline'))
        self.assertNotIn(2, decline['topics'])
        self.assertEqual(decline['exclude_keywords'], ['mission', 'yes'])
