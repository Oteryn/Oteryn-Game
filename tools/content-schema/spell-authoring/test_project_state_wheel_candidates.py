import copy
import gzip
import json
from pathlib import Path
import unittest

import project_state_wheel_candidates as adapter
import validate_spell

REPO = Path(__file__).resolve().parents[3]


class StateWheelProjectionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.candidates, cls.audit = adapter.build(REPO)

    def test_exact_population_and_safe_base_count(self):
        self.assertEqual(len(self.audit), 64)
        self.assertEqual(len({row['registration_key'] for row in self.audit}), 64)
        self.assertEqual(len(self.candidates), 21)
        self.assertEqual(sum('/attack/' in row['registration_key'] and 'S21' in row['normalization_policy'] for row in self.candidates), 15)
        self.assertEqual(sum('S6' in row['normalization_policy'] for row in self.candidates), 6)

    def test_every_bundle_validates_actual_reader_shape(self):
        for row in self.candidates:
            files = row['files']
            self.assertEqual(validate_spell.validate(files['spell.json'], files['dependencies.json'], files['catalog.json']), [])
            self.assertTrue(adapter.controls.validate_reader_shape(REPO, files['spell.json'], files['dependencies.json']))
            self.assertIn('ability', files['spell.json']['spell']['execution'])
            self.assertNotIn('native_behavior', files['spell.json']['spell']['execution'])
            self.assertFalse(files['receipt.json']['native_execution_qualified'])

    def test_raw_headers_exact_and_current_source_identity(self):
        for row in self.candidates:
            expected = (adapter.base_row(REPO, row['registration_key']) / 'source-header.json').read_bytes()
            self.assertEqual(row['source_header_bytes'], expected)
            self.assertEqual(adapter.sha(expected), row['source_header_sha256'])
            receipt = row['files']['receipt.json']
            self.assertEqual(receipt['source_sha256'], row['source_sha256'])
            self.assertTrue(receipt['remaining_mechanics'])
            self.assertTrue(any('Raw source header copied byte-exact' in note for note in receipt['conversion_notes']))

    def test_s21_counterpart_evidence_and_identity_rebound(self):
        for row in self.candidates:
            if 'S21' not in row['normalization_policy']: continue
            self.assertTrue(row['registration_key'].startswith('crystal'))
            self.assertTrue(row['selected_mechanics_registration'].startswith('canary'))
            self.assertNotEqual(row['selected_mechanics_source_sha256'], row['source_sha256'])
            for family in row['files']['dependencies.json'].values():
                for definition in family:
                    self.assertEqual(definition['identity']['revision'], adapter.REVISION)
                    self.assertTrue(definition['identity']['key'].startswith(row['candidate_key']))

    def test_true_wheel_stage_rows_never_silently_demoted(self):
        for row in self.audit:
            name = row['registration_key'].rsplit('/', 1)[1].split('#')[0]
            if name in {'great_death_beam.lua', 'executioners_throw.lua', 'ice_burst.lua', 'terra_burst.lua'}:
                self.assertEqual(row['status'], 'BLOCKED')
                self.assertIn('grade', row['specific_blocker'])
            self.assertFalse(row['runtime_activation'])
            self.assertFalse(row['native_execution_qualified'])
            self.assertEqual(len(row['source_sha256']), 64)

    def test_wrong_cached_source_identity_refused(self):
        facts = {row['registration_key']: row for row in map(json.loads, gzip.decompress((REPO / adapter.BASE / 'source-callback-facts.jsonl.gz').read_bytes()).splitlines())}
        key = next(row['registration_key'] for row in self.candidates if row['registration_key'].startswith('canary') and '/energy_beam.lua#' in row['registration_key'])
        raw = facts[key]['source_callback_facts']
        caches = {('canary', raw['file']): {'provenance': {'sha256': '0' * 64, 'revision': facts[key]['source_revision']}}}
        with self.assertRaisesRegex(ValueError, 'source identity differs'):
            adapter.cached_base_projection(REPO, key, facts, caches, {})

    def test_vocation_none_not_silently_removed(self):
        spell = copy.deepcopy(self.candidates[0]['files']['spell.json']['spell'])
        spell['requirements']['vocations'].append('none')
        with self.assertRaisesRegex(ValueError, 'None vocation domain'):
            adapter.reader_projection(REPO, spell)


if __name__ == '__main__': unittest.main()
