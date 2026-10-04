import copy
import gzip
import json
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

import import_paralyze_partial_templates as importer


class ParalyzePartialImportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data, cls.manifest = importer.prepare_import(importer.ROOT)
        cls.rows = list(map(json.loads, gzip.decompress(cls.data[cls.manifest['source_metadata_path']]).splitlines()))
        base = importer.ROOT / 'imports/spells/r28/player-source-bundles'
        cls.callbacks = {r['registration_key']: r for r in map(json.loads, gzip.decompress((base / 'source-callback-facts.jsonl.gz').read_bytes()).splitlines())}
        cls.joined = {}
        for row in cls.rows:
            key = row['registration_key']; folder = base / key.split('/')[0] / importer.digest(key.encode())[:16]
            cls.joined[key] = (cls.callbacks[key], (folder / 'source-header.json').read_bytes(), (folder / 'receipt.json').read_bytes())

    def check(self, row):
        importer.verify_row(row, *self.joined[self.rows[0]['registration_key']])

    def test_only_two_use_runes_partial_templates_not_conjure_or_complete_spells(self):
        self.assertEqual({r['registration_key'] for r in self.rows}, importer.KEYS)
        self.assertTrue(all('/runes/' in r['registration_key'] and '/conjuring/' not in r['registration_key'] for r in self.rows))
        self.assertEqual(self.manifest['counts']['full_spell_status_counts'], {'BLOCKED': 2})
        self.assertEqual(self.manifest['counts']['source_backed_speed_formulas'], 2)
        self.assertEqual(self.manifest['full_spell_candidates'], 0)

    def test_forged_source_or_historical_status_refused(self):
        for field in ('sha256', 'git_blob'):
            row = copy.deepcopy(self.rows[0]); row['source_identity'][field] = '0' * len(row['source_identity'][field])
            with self.assertRaisesRegex(ValueError, 'SOURCE_JOIN_MISMATCH'): self.check(row)
        row = copy.deepcopy(self.rows[0]); row['full_spell_status'] = 'CANDIDATE_SCHEMA_VALID'
        with self.assertRaisesRegex(ValueError, 'SOURCE_JOIN_MISMATCH'): self.check(row)

    def test_changed_condition_or_formula_refused(self):
        for path in ('duration', 'offset', 'raw'):
            row = copy.deepcopy(self.rows[0])
            if path == 'duration': row['dependency_templates']['effects'][0]['duration_ms'] = 5000
            elif path == 'offset': row['dependency_templates']['formulas'][0]['speed']['minimum_offset'] = 0
            else: row['cpp_control_proof']['speed_normalization']['source_coefficients'][0] = 0
            with self.assertRaisesRegex(ValueError, 'SOURCE_(CONDITION|SPEED_PROJECTION)_MISMATCH'): self.check(row)

    def test_execute_true_not_health_success_and_correct_donor(self):
        row = copy.deepcopy(self.rows[0]); row['cpp_control_proof']['number_variant_result'] = 'doCombat_health_success'
        with self.assertRaisesRegex(ValueError, 'CPP_RETURN_CONTRACT_MISMATCH'): self.check(row)
        crystal = next(r for r in self.rows if r['source_identity']['source'] == 'crystal')
        row = copy.deepcopy(crystal); row['dependency_templates']['effects'][0]['presentation']['caster_effect_asset_binding'] = 'canary.appearance:effect/magic_green'
        with self.assertRaisesRegex(ValueError, 'DONOR_PRESENTATION_MISMATCH'): importer.verify_row(row, *self.joined[crystal['registration_key']])

    def test_rehashed_unqualified_receipt_refused(self):
        with TemporaryDirectory() as directory:
            source = Path(directory); proof = json.loads(self.data['evidence/source-paralyze-partial-receipt.json']); proof['candidate_count'] = 2
            (source / 'source-paralyze-partial-receipt.json').write_text(json.dumps(proof))
            with self.assertRaisesRegex(ValueError, 'QUALIFIED_RECEIPT_HASH_MISMATCH'): importer.prepare_import(importer.ROOT, source)

    def test_complete_claim_or_output_tamper_refused_and_prior_content_unchanged(self):
        with TemporaryDirectory() as directory:
            root = Path(directory); target = root / 'imports/spells/r45'
            active = root / 'content/spells.manifest.json'; active.parent.mkdir(); active.write_bytes(b'active')
            old = root / 'imports/spells/r44/import-manifest.json'; old.parent.mkdir(parents=True); old.write_bytes(b'old')
            bad = copy.deepcopy(self.manifest); bad['full_spell_candidates'] = 2
            with self.assertRaisesRegex(ValueError, 'COMPLETE_SPELL_CLAIM_REFUSED'): importer.write_import(root, target, self.data, bad)
            changed = dict(self.data); changed[self.manifest['source_metadata_path']] = b'forged'
            with self.assertRaisesRegex(ValueError, 'OUTPUT_PIN_MISMATCH'): importer.write_import(root, target, changed, self.manifest)
            importer.write_import(root, target, self.data, self.manifest)
            self.assertEqual(active.read_bytes(), b'active'); self.assertEqual(old.read_bytes(), b'old')
            with self.assertRaisesRegex(ValueError, 'IMPORT_SET_ALREADY_EXISTS'): importer.write_import(root, target, self.data, self.manifest)


if __name__ == '__main__': unittest.main()
