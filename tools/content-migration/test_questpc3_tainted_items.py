"""Strict talisman evidence and idempotent materialization."""
import copy
import importlib.util
import json
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / 'tools/content-migration/questpc3_tainted_items.py'


class TalismanAdmissionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location('tainted', SOURCE)
        cls.module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.module)
        cls.rows = []
        for shard in json.loads((ROOT / 'content/items/index.json').read_bytes())['shards']:
            for row in json.loads((ROOT / shard).read_bytes())['records']:
                if row['definition']['identity']['key'] in ('oteryn:item.tibia.i22008', 'oteryn:item.tibia.i22009'):
                    cls.rows.append(row['definition'])

    def test_exact_source_facts_and_idempotence(self):
        rows = copy.deepcopy(self.rows)
        self.module.apply_admissions(rows)
        first = copy.deepcopy(rows)
        self.module.apply_admissions(rows)
        self.assertEqual(rows, first)
        for row in rows:
            self.assertTrue(row['materializable'])
            self.assertEqual(row['semantics']['physical'], next(r for r in self.rows if r['identity'] == row['identity'])['semantics']['physical'])
            self.assertEqual(row['stack_class'], 'StackCapable' if row['identity']['key'].endswith('i22008') else 'NonStackable')

    def test_refuses_changed_physical_fact(self):
        rows = copy.deepcopy(self.rows)
        rows[0]['semantics']['physical']['value']['weight']['value'] = 241
        with self.assertRaisesRegex(ValueError, 'canonical facts changed'):
            self.module.apply_admissions(rows)

    def test_refuses_changed_source_digest(self):
        facts = json.loads((ROOT / 'imports/tibiawiki/facts/items-stats.json').read_bytes())
        facts['records']['oteryn:item.tibia.i22008']['observations'][0]['content_sha256'] = '0' * 64
        with patch.object(self.module.json, 'loads', return_value=facts):
            with self.assertRaisesRegex(ValueError, 'source changed'):
                self.module.apply_admissions(copy.deepcopy(self.rows))

    def test_refuses_changed_classification(self):
        rows = copy.deepcopy(self.rows)
        rows[0]['semantics']['classification'] = {'state': 'KNOWN', 'value': {'item_type': {'state': 'KNOWN', 'value': 'MAGIC_FIELD'}}}
        with self.assertRaisesRegex(ValueError, 'classification changed'):
            self.module.apply_admissions(rows)

    def test_refuses_changed_lifetime(self):
        rows = copy.deepcopy(self.rows)
        rows[0]['semantics']['temporal'] = {'state': 'KNOWN', 'value': {}}
        with self.assertRaisesRegex(ValueError, 'canonical lifetime changed'):
            self.module.apply_admissions(rows)

    def test_refuses_duplicate_identity(self):
        rows = copy.deepcopy(self.rows)
        rows.append(copy.deepcopy(rows[0]))
        with self.assertRaisesRegex(ValueError, 'identity changed'):
            self.module.apply_admissions(rows)


if __name__ == '__main__':
    unittest.main()
