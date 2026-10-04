import copy
import json
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest
import shutil

import import_incremental_source_spells as importer


class IncrementalSourceImportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data, cls.manifest = importer.prepare_import(importer.ROOT, importer.ROOT / importer.SOURCE, importer.BASE_SHA)

    def test_actual_four_candidates_bind_frozen_base_schemas_and_source_keys(self):
        self.assertEqual(self.manifest['counts']['records'], 4)
        self.assertEqual(set(self.manifest['source_keys']), importer.SOURCE_KEYS)
        self.assertEqual(self.manifest['base']['sha256'], importer.BASE_SHA)
        self.assertFalse(self.manifest['native_execution_qualified'])
        for schema in self.manifest['schemaRefs']:
            if schema['path'].startswith('schemas/'):
                self.assertEqual(self.data[schema['path']], (importer.ROOT / 'imports/spells/r28' / schema['path']).read_bytes())
        self.assertTrue(any(name.endswith('/source-grade-actions.json') for name in self.data))

    def test_tampered_base_is_rejected_before_source_read_or_output(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            base = root / 'imports/spells/r28/import-manifest.json'
            base.parent.mkdir(parents=True)
            base.write_bytes(b'tampered')
            with self.assertRaisesRegex(ValueError, 'BASE_MANIFEST_PIN_MISMATCH'):
                importer.prepare_import(root, root / 'missing-source', importer.BASE_SHA)
            self.assertFalse((root / 'imports/spells/r29').exists())

    def test_tampered_candidate_is_rejected_before_output(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            data = dict(self.data)
            name = next(n for n in data if n.endswith('/spell.json'))
            data[name] = b'tampered'
            with self.assertRaisesRegex(ValueError, 'INCREMENT_OUTPUT_PIN_MISMATCH'):
                importer.write_import(root, root / 'imports/spells/r29', data, self.manifest)
            self.assertFalse((root / 'imports/spells/r29').exists())

    def test_incremental_write_preserves_base_and_active_content_and_rejects_replay(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            protected = [root / 'imports/spells/r28/import-manifest.json', root / 'content/spells.manifest.json']
            for path in protected:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b'existing immutable input')
            before = {p: p.read_bytes() for p in protected}
            target = root / 'imports/spells/r29'
            importer.write_import(root, target, self.data, self.manifest)
            self.assertEqual(before, {p: p.read_bytes() for p in protected})
            with self.assertRaisesRegex(ValueError, 'INCREMENT_ALREADY_EXISTS'):
                importer.write_import(root, target, self.data, self.manifest)

    def test_activation_claim_and_base_destination_are_refused(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            active = copy.deepcopy(self.manifest)
            active['canonical_selection_changed'] = True
            with self.assertRaisesRegex(ValueError, 'INCREMENT_ACTIVE_CLAIM_REFUSED'):
                importer.write_import(root, root / 'imports/spells/r29', self.data, active)
            with self.assertRaisesRegex(ValueError, 'INCREMENT_DESTINATION_REFUSED'):
                importer.write_import(root, root / 'imports/spells/r28', self.data, self.manifest)

    def test_rehashed_receipt_source_sha_or_header_forgery_is_refused(self):
        source = importer.ROOT / importer.SOURCE
        for field in ('source_sha256', 'source_header'):
            with self.subTest(field=field), TemporaryDirectory() as directory:
                folder = Path(directory) / 'source'
                shutil.copytree(source, folder)
                path = next(folder.glob('*/*/receipt.json'))
                receipt = json.loads(path.read_bytes())
                if field == 'source_sha256':
                    receipt[field] = '0' * 64
                else:
                    receipt[field]['name'] = 'False source header'
                body = importer.encoded(receipt)
                path.write_bytes(body)
                package_path = folder / 'package-manifest.json'
                package = json.loads(package_path.read_bytes())
                package['files'][path.relative_to(folder).as_posix()] = importer.digest(body)
                package_path.write_bytes(importer.encoded(package))
                with self.assertRaisesRegex(ValueError, 'INCREMENT_RECEIPT_SOURCE_IDENTITY_MISMATCH'):
                    importer.prepare_import(importer.ROOT, folder, importer.BASE_SHA)


if __name__ == '__main__':
    unittest.main()
