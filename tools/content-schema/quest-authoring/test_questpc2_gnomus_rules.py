import json
from pathlib import Path
import unittest

import questpc2_gnomus_rules as source

ROOT = Path(__file__).resolve().parents[3]


class GnomusRules(unittest.TestCase):
    def test_source_pin_reproduces_runtime_program(self):
        for path, text in source.outputs(ROOT).items():
            self.assertEqual((ROOT / path).read_text(), text)

    def test_acceptance_resets_both_counters_without_reward_or_cooldown_credit(self):
        doc = json.loads(source.outputs(ROOT)[source.OUTPUT])
        rule = next(r for r in doc['rules'] if r['key'].endswith('/accept-first'))
        self.assertEqual(rule['topics'], [22])
        self.assertIn('ordnance', rule['exclude_keywords'])
        writes = {e['track'].removeprefix(source.PREFIX): e['value']
                  for e in rule['effects'] if e['kind'] == 'set'}
        self.assertEqual(writes, {'questline': 1, 'gnomes/ordnance': 1,
                                 'gnomes/gnomes_count': 0, 'gnomes/crawlers_count': 0})
        self.assertEqual({e['kind'] for r in doc['rules'] for e in r['effects']},
                         {'set', 'set_topic'})
        self.assertFalse(any(r.get('continue_matching', False) for r in doc['rules']))
        source_rules = [r for r in doc['rules'] if '/unsupported-task-' not in r['key']]
        self.assertEqual(len(source_rules), 7)
        self.assertTrue(all(set(source.UNOWNED) <= set(r['exclude_keywords']) for r in source_rules))

    def test_unowned_task_clears_confirmation_without_progress_or_reward(self):
        doc = json.loads(source.outputs(ROOT)[source.OUTPUT])
        boundaries = [r for r in doc['rules'] if '/unsupported-task-' in r['key']]
        self.assertEqual({r['keywords'][0] for r in boundaries}, set(source.UNOWNED))
        for rule in boundaries:
            self.assertEqual(rule['effects'], [dict(kind='set_topic', value=0)])
            self.assertEqual(rule['reply'], [])
            self.assertEqual(rule['exclude_keywords'], [])
