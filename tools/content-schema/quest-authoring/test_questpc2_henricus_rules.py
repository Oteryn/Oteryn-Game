import json
from pathlib import Path
import unittest

import questpc2_henricus_rules as source

ROOT = Path(__file__).resolve().parents[3]


class HenricusRules(unittest.TestCase):
    def test_pinned_source_reproduces_the_exact_runtime_program(self):
        for path, text in source.outputs(ROOT).items():
            self.assertEqual((ROOT / path).read_text(), text)

    def test_report_requires_every_source_guard_and_cannot_complete_the_quest(self):
        document = json.loads(source.outputs(ROOT)[source.OUTPUT])
        rule = next(r for r in document['rules'] if r['key'].endswith('/report-accept'))
        self.assertEqual({g['track'].removeprefix(source.PREFIX) for g in rule['guards']},
                         {'walter_guard', 'kulag_guard', 'grof_guard', 'miles_guard', 'tim_guard'})
        self.assertTrue(all(g['op'] == 'eq' and g['value'] == 1 for g in rule['guards']))
        writes = {e['track']: e['value'] for e in rule['effects'] if e['kind'] == 'set'}
        self.assertEqual(writes, {source.PREFIX+'questline': 3, source.PREFIX+'mission01': 7})
        self.assertFalse(any(e.get('value') == 25 for r in document['rules'] for e in r['effects']))
