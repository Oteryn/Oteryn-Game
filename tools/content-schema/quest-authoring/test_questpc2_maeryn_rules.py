import json
from pathlib import Path
import unittest

import questpc2_maeryn_rules as source

ROOT = Path(__file__).resolve().parents[3]


class MaerynRules(unittest.TestCase):
    def test_source_pin_reproduces_runtime_program(self):
        for path, text in source.outputs(ROOT).items():
            self.assertEqual((ROOT / path).read_text(), text)

    def test_six_exact_choices_and_no_invented_item_or_quest_gate(self):
        doc = json.loads(source.outputs(ROOT)[source.OUTPUT])
        self.assertEqual(len(doc['rules']), 9)
        self.assertEqual(doc['tracks'][0]['initial'], -1)
        choices = {r['keywords'][0]: r for r in doc['rules']
                   if any(e['kind'] == 'set' for e in r['effects'])}
        self.assertEqual({name: r['effects'][0]['value'] for name, r in choices.items()},
                         dict(sorcerer=0, druid=1, paladin=2, club=3, axe=4, sword=5))
        for name, rule in choices.items():
            self.assertEqual(rule['keyword_mode'], 'exact')
            self.assertEqual(rule['topics'], [3 if name in ('club', 'axe', 'sword') else 2])
        self.assertTrue(all(r['guards'] == [] for r in doc['rules']))
        self.assertEqual({e['kind'] for r in doc['rules'] for e in r['effects']},
                         {'set', 'set_topic'})
        self.assertTrue(all(set(source.PRIOR) == set(r['exclude_keywords']) for r in doc['rules']))
        self.assertTrue(all(not r.get('continue_matching', False) for r in doc['rules']))
