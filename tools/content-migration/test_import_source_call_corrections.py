import copy
import gzip
import json
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

import import_source_call_corrections as importer


class SourceCallCorrectionImportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data, cls.manifest = importer.prepare_import(importer.ROOT)
        cls.value = json.loads(gzip.decompress(cls.data[cls.manifest['source_metadata_path']]))
        cls.files = json.loads(gzip.decompress((importer.ROOT / 'imports/spells/r28/source-mechanics-inventory.json.gz').read_bytes()))['files']
        cls.ast_counts = {importer.identity(row['source_identity']): row['ast_call_invoke_count'] for row in cls.value['per_file_reconciliation']}

    def test_actual_32_corrections_2346_files_and_typed_known_calls_unchanged(self):
        counts = self.manifest['counts']
        self.assertEqual((counts['source_files'], counts['excluded_operator_calls']), (2346, 32))
        self.assertEqual((counts['frozen_calls'], counts['corrected_calls']), (50318, 50286))
        self.assertEqual((counts['frozen_unsupported_call_references'], counts['corrected_unsupported_call_references']), (11998, 11966))
        self.assertEqual(counts['typed_known_calls_unchanged'], 38320)
        self.assertEqual(counts['full_spell_candidates'], 0)
        self.assertEqual(self.manifest['source_inputs']['r38']['sha256'], importer.AST_MANIFEST_SHA)

    def test_tampered_frozen_call_hash_or_source_identity_is_refused(self):
        for field, value in [('frozen_call_fact_sha256', '0' * 64), ('frozen_call_index', 0)]:
            bad = copy.deepcopy(self.value); bad['corrections'][0][field] = value
            with self.assertRaisesRegex(ValueError, 'CALL_CORRECTION_FROZEN_REFERENCE_MISMATCH'):
                importer.reconcile(bad, self.files, self.ast_counts)

    def test_dropped_correction_or_changed_per_file_count_is_refused(self):
        bad = copy.deepcopy(self.value); bad['corrections'].pop()
        with self.assertRaisesRegex(ValueError, 'CALL_CORRECTION_FROZEN_REFERENCE_MISMATCH'):
            importer.reconcile(bad, self.files, self.ast_counts)
        bad = copy.deepcopy(self.value); bad['per_file_reconciliation'][0]['corrected_call_count'] += 1
        with self.assertRaisesRegex(ValueError, 'CALL_CORRECTION_PER_FILE_COUNT_MISMATCH'):
            importer.reconcile(bad, self.files, self.ast_counts)

    def test_rehashed_unqualified_source_packet_is_refused(self):
        with TemporaryDirectory() as directory:
            folder = Path(directory); proof_path = 'source-call-corrections-receipt.json'
            proof = json.loads(self.data['evidence/' + proof_path]); value = copy.deepcopy(self.value)
            value['corrections'][0]['source_character_offset'] += 1
            payload = importer.canonical(value); compressed = gzip.compress(payload, mtime=0)
            proof['gzip_sha256'] = importer.digest(compressed); proof['payload_sha256'] = importer.digest(payload)
            (folder / proof_path).write_text(json.dumps(proof)); (folder / 'source-call-corrections.json.gz').write_bytes(compressed)
            with self.assertRaisesRegex(ValueError, 'CALL_CORRECTION_QUALIFIED_PACKET_HASH_MISMATCH'):
                importer.prepare_import(importer.ROOT, folder)

    def test_exact_new_copy_does_not_mutate_base_or_active_catalog(self):
        with TemporaryDirectory() as directory:
            root = Path(directory); target = root / 'imports/spells/r39'
            active = root / 'content/spells.manifest.json'; active.parent.mkdir(); active.write_bytes(b'active')
            old = root / 'imports/spells/r38/import-manifest.json'; old.parent.mkdir(parents=True); old.write_bytes(b'old')
            importer.write_import(root, target, self.data, self.manifest)
            self.assertEqual(active.read_bytes(), b'active'); self.assertEqual(old.read_bytes(), b'old')
            with self.assertRaisesRegex(ValueError, 'IMPORT_SET_ALREADY_EXISTS'): importer.write_import(root, target, self.data, self.manifest)

    def test_output_byte_forgery_and_execution_claim_refused(self):
        with TemporaryDirectory() as directory:
            root = Path(directory); target = root / 'imports/spells/r39'
            bad = copy.deepcopy(self.manifest); bad['mechanics_completion_claim'] = True
            with self.assertRaisesRegex(ValueError, 'CALL_CORRECTION_ACTIVATION_CLAIM_REFUSED'): importer.write_import(root, target, self.data, bad)
            data = dict(self.data); data[self.manifest['source_metadata_path']] = b'forged'
            with self.assertRaisesRegex(ValueError, 'OUTPUT_PIN_MISMATCH'): importer.write_import(root, target, data, self.manifest)
            self.assertFalse(target.exists())


if __name__ == '__main__': unittest.main()
