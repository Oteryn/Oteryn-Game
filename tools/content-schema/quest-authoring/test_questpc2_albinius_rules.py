import json
from pathlib import Path
import unittest

import questpc2_albinius_rules as source

ROOT = Path(__file__).resolve().parents[3]


class AlbiniusRules(unittest.TestCase):
    def test_source_pin_reproduces_exact_runtime_program(self):
        for path, text in source.outputs(ROOT).items():
            self.assertEqual((ROOT / path).read_text(), text)

    def test_only_five_exact_tomes_can_earn_one_progress_flag(self):
        doc = json.loads(source.outputs(ROOT)[source.OUTPUT])
        self.assertEqual([t['key'] for t in doc['tracks']], [source.TRACK])
        contribution = next(r for r in doc['rules'] if r['key'].endswith('/contribute'))
        self.assertEqual(contribution['topics'], [1])
        self.assertEqual(contribution['guards'][1],
                         dict(kind='item', item=source.ITEM, count=5, op='ge'))
        self.assertEqual(contribution['effects'], [
            dict(kind='take_item', item=source.ITEM, count=5, optional=False),
            dict(kind='set', track=source.TRACK, value=1)])
        self.assertTrue(all(r['continue_matching'] for r in doc['rules']))
        self.assertTrue(all(set(source.UNOWNED) <= set(r['exclude_keywords']) for r in doc['rules']))
        self.assertFalse(any(e['kind'] == 'give_item' for r in doc['rules'] for e in r['effects']))
        missing = next(r for r in doc['rules'] if r['key'].endswith('/missing'))
        self.assertEqual(missing['effects'], [])
        self.assertEqual(missing['guards'][1]['op'], 'lt')
