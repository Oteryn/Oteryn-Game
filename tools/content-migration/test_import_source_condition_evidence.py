import copy
import gzip
import json
from pathlib import Path
import shutil
from tempfile import TemporaryDirectory
import unittest

import import_source_condition_evidence as importer
from import_source_spell_package import encoded


class ConditionEvidenceImportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data, cls.manifest = importer.prepare_import(importer.ROOT, importer.ROOT / importer.SOURCE)

    def fixture(self, folder):
        folder.mkdir()
        for name in (importer.GZIP, importer.RECEIPT):
            shutil.copyfile(importer.ROOT / importer.SOURCE / name, folder / name)

    def rewrite_record(self, folder, field, value):
        rows = [json.loads(line) for line in gzip.decompress((folder / importer.GZIP).read_bytes()).splitlines()]
        if field == 'source_order':
            rows[0]['constructor_call_ref'][field] += 1
        else:
            rows[0][field] = value
        payload = b''.join(encoded(row).replace(b'\n', b'') + b'\n' for row in rows)
        body = gzip.compress(payload, mtime=0)
        (folder / importer.GZIP).write_bytes(body)
        receipt = json.loads((folder / importer.RECEIPT).read_bytes())
        receipt['gzip_sha256'] = importer.digest(body)
        receipt['payload_sha256'] = importer.digest(payload)
        (folder / importer.RECEIPT).write_bytes(encoded(receipt))

    def test_actual_59_declarations_are_partial_12_prefixes_47_gaps(self):
        self.assertEqual(self.manifest['counts'], {'source_declarations': 59, 'structural_effect_prefix_templates': 12,
                                                'explicit_payload_gaps': 47, 'full_spell_candidates': 0})
        self.assertFalse(self.manifest['template_complete'])
        self.assertFalse(self.manifest['application_binding_qualified'])
        self.assertEqual(self.manifest['base']['sha256'], importer.BASE_SHA)

    def test_changed_gzip_without_receipt_pin_is_refused(self):
        with TemporaryDirectory() as directory:
            folder = Path(directory) / 'source'
            self.fixture(folder)
            (folder / importer.GZIP).write_bytes(b'tampered')
            with self.assertRaisesRegex(ValueError, 'CONDITION_ARTIFACT_PIN_MISMATCH'):
                importer.prepare_import(importer.ROOT, folder)

    def test_rehashed_source_identity_and_call_reference_forgery_is_refused(self):
        for field, value, error in [('source_sha256', '0' * 64, 'CONDITION_FILE_IDENTITY_MISMATCH'),
                                   ('source_order', None, 'CONDITION_CALL_REFERENCE_MISMATCH')]:
            with self.subTest(field=field), TemporaryDirectory() as directory:
                folder = Path(directory) / 'source'
                self.fixture(folder)
                self.rewrite_record(folder, field, value)
                with self.assertRaisesRegex(ValueError, error):
                    importer.prepare_import(importer.ROOT, folder)

    def test_rehashed_enum_header_hash_forgery_is_refused(self):
        with TemporaryDirectory() as directory:
            folder = Path(directory) / 'source'
            self.fixture(folder)
            receipt = json.loads((folder / importer.RECEIPT).read_bytes())
            receipt['engine_enum_provenance'][0]['sha256'] = '0' * 64
            (folder / importer.RECEIPT).write_bytes(encoded(receipt))
            with self.assertRaisesRegex(ValueError, 'CONDITION_ENUM_HASH_MISMATCH'):
                importer.prepare_import(importer.ROOT, folder)

    def test_full_spell_count_or_application_qualification_claim_is_refused(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            for field in ('count', 'application'):
                forged = copy.deepcopy(self.manifest)
                if field == 'count':
                    forged['counts']['full_spell_candidates'] = 12
                    error = 'CONDITION_MANIFEST_COUNTS_INVALID'
                else:
                    forged['application_binding_qualified'] = True
                    error = 'CONDITION_COMPLETENESS_CLAIM_REFUSED'
                with self.assertRaisesRegex(ValueError, error):
                    importer.write_import(root, root / 'imports/spells/r31', self.data, forged)
            self.assertFalse((root / 'imports/spells/r31').exists())

    def test_data_only_write_preserves_previous_sets_and_active_content_and_refuses_replay(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            protected = [root / 'imports/spells' / revision / 'import-manifest.json' for revision in ('r28', 'r29', 'r30')]
            protected.append(root / 'content/spells.manifest.json')
            for path in protected:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b'unchanged immutable input')
            before = {p: p.read_bytes() for p in protected}
            target = root / 'imports/spells/r31'
            importer.write_import(root, target, self.data, self.manifest)
            self.assertEqual(before, {p: p.read_bytes() for p in protected})
            with self.assertRaisesRegex(ValueError, 'IMPORT_SET_ALREADY_EXISTS'):
                importer.write_import(root, target, self.data, self.manifest)

    def test_runtime_activation_and_old_destination_are_refused(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            forged = copy.deepcopy(self.manifest)
            forged['runtime_activation'] = True
            with self.assertRaisesRegex(ValueError, 'ACTIVE_OR_EQUIVALENT_CLAIM_REFUSED'):
                importer.write_import(root, root / 'imports/spells/r31', self.data, forged)
            with self.assertRaisesRegex(ValueError, 'IMPORT_DESTINATION_REFUSED'):
                importer.write_import(root, root / 'imports/spells/r30', self.data, self.manifest)


if __name__ == '__main__':
    unittest.main()
