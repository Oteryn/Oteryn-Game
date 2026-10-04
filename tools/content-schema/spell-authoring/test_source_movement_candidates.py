import copy
import json
from pathlib import Path
import unittest

import source_movement_candidates as adapter
import validate_spell

REPO = Path(__file__).resolve().parents[3]


class MovementCandidateTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.values, cls.capture = adapter.build(REPO, '/workspace/spell-sources')

    def test_both_exact_headers_validate_and_have_no_dependencies(self):
        records = self.values['import-summary.json']['records_index']
        self.assertEqual(2, len(records))
        for record in records:
            row = 'canary-main-current/' + record['candidate_key'].rsplit('/', 1)[-1] + '/'
            spell = self.values[row + 'spell.json']
            self.assertEqual([], validate_spell.validate(spell, self.values[row + 'dependencies.json'], self.values[row + 'catalog.json']))
            self.assertEqual(adapter.CANDIDATE_REVISION, spell['spell']['identity']['revision'])
            header = self.values[row + 'source-header.json']['spell']
            for field in ('presentation', 'requirements', 'costs', 'targeting'):
                for key, value in header[field].items():
                    self.assertEqual(value, spell['spell'][field][key])
            receipt = self.values[row + 'receipt.json']
            validate_spell.Draft202012Validator(adapter.base.receipt_schema(), registry=validate_spell.REGISTRY).validate(receipt)
            self.assertFalse(receipt['native_execution_qualified'])
            self.assertEqual({'abilities': 0, 'effects': 0, 'formulas': 0}, receipt['dependencies'])
            self.assertTrue(receipt['remaining_mechanics'])

    def test_descriptors_equal_existing_crystal_source_templates(self):
        original = json.loads((REPO / adapter.ORIGINAL / 'import-summary.json').read_text())
        for record in self.values['import-summary.json']['records_index']:
            crystal_key = record['registration_key'].replace('canary-main-current/', 'crystal-summer-current/', 1)
            archived = [row for row in original['records_index'] if row['registration_key'] == crystal_key]
            self.assertEqual(1, len(archived))
            crystal_id = adapter.sha(crystal_key.encode())[:16]
            crystal = json.loads((REPO / adapter.ORIGINAL / 'crystal-summer-current' / crystal_id / 'spell.json').read_text())
            current_id = record['candidate_key'].rsplit('/', 1)[-1]
            spell = self.values['canary-main-current/' + current_id + '/spell.json']
            self.assertEqual(crystal['spell']['execution'], spell['spell']['execution'])

    def test_exact_full_capture_preserved_and_only_two_overlay_records(self):
        self.assertEqual((REPO / adapter.ORIGINAL / 'source-callback-facts.jsonl.gz').read_bytes(), self.capture)
        summary = self.values['import-summary.json']
        self.assertEqual(483, summary['captured_records'])
        self.assertEqual(2, summary['records'])
        self.assertEqual(2, summary['native_descriptor_count'])
        for field in ('runtime_activation', 'external_sources_used', 'native_execution_qualified', 'canonical_selection_changed', 'native_identity_allocation', 'full_source_mechanics_1_to_1_complete'):
            self.assertIs(False, summary[field])

    def test_capture_mutations_and_truncated_body_are_not_qualification(self):
        facts = [json.loads(line) for line in adapter.gzip.decompress(self.capture).splitlines()]
        for name in adapter.FACT_SHAS:
            reg = 'canary-main-current/' + adapter.native.FILES[name] + '#1'
            fact = next(value for value in facts if value['registration_key'] == reg)
            source = adapter.closure.read('/workspace/spell-sources/canary', adapter.REVISION, adapter.native.FILES[name])
            adapter.qualify_fact(name, fact, source)
            for mutation in ('body', 'registrar', 'source'):
                changed = copy.deepcopy(fact)
                if mutation == 'body':
                    changed['source_callback_facts']['cast']['body'] += ' return true'
                elif mutation == 'registrar':
                    changed['source_callback_facts']['registrar']['mana'] += 1
                else:
                    changed['source_revision'] = adapter.closure.OLD_REVISION
                with self.subTest(name=name, mutation=mutation), self.assertRaises(ValueError):
                    adapter.qualify_fact(name, changed, source)


if __name__ == '__main__':
    unittest.main()
