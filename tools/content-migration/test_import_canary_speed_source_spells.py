import copy
import json
from pathlib import Path
import shutil
from tempfile import TemporaryDirectory
import unittest

from jsonschema import ValidationError

import import_canary_speed_source_spells as importer
from import_source_spell_package import encoded


class CanarySpeedSourceImportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data, cls.manifest = importer.prepare_import(importer.ROOT, importer.ROOT / importer.SOURCE)

    def test_actual_three_canary_keys_with_inequivalent_provider_and_archived_schemas(self):
        self.assertEqual(set(self.manifest['source_keys']), importer.SOURCE_KEYS)
        self.assertEqual(self.manifest['counts']['records'], 3)
        self.assertEqual(self.manifest['counts']['source_function_and_storage_proofs'], 69)
        self.assertFalse(self.manifest['input_provider_equivalence'])
        for name, body in self.data.items():
            if name.startswith('schemas/'):
                self.assertEqual(body, (importer.ROOT / 'imports/spells/r28' / name).read_bytes())

    def rewrite_member(self, folder, path, value):
        body = encoded(value)
        path.write_bytes(body)
        package_path = folder / 'package-manifest.json'
        package = json.loads(package_path.read_bytes())
        package['files'][path.relative_to(folder).as_posix()] = importer.digest(body)
        package_path.write_bytes(encoded(package))

    def test_rehashed_false_source_receipt_sha_is_refused(self):
        with TemporaryDirectory() as directory:
            folder = Path(directory) / 'source'
            shutil.copytree(importer.ROOT / importer.SOURCE, folder)
            path = next(folder.glob('*/*/receipt.json'))
            receipt = json.loads(path.read_bytes())
            receipt['source_sha256'] = '0' * 64
            self.rewrite_member(folder, path, receipt)
            with self.assertRaisesRegex(ValueError, 'RECEIPT_SOURCE_IDENTITY_MISMATCH'):
                importer.prepare_import(importer.ROOT, folder)

    def test_rehashed_staff_cap_or_equivalence_forgery_is_refused_by_schema(self):
        for key, value in [('input_provider_equivalence', True), ('current_staff_cap', 1500)]:
            with self.subTest(key=key), TemporaryDirectory() as directory:
                folder = Path(directory) / 'source'
                shutil.copytree(importer.ROOT / importer.SOURCE, folder)
                path = folder / 'source-speed-inputs.json'
                provider = json.loads(path.read_bytes())
                provider[key] = value
                self.rewrite_member(folder, path, provider)
                with self.assertRaises(ValidationError):
                    importer.prepare_import(importer.ROOT, folder)

    def test_tampered_base_is_refused_before_packet_read(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / 'imports/spells/r28/import-manifest.json'
            path.parent.mkdir(parents=True)
            path.write_bytes(b'tampered')
            with self.assertRaisesRegex(ValueError, 'BASE_MANIFEST_PIN_MISMATCH'):
                importer.prepare_import(root, root / 'absent')

    def test_tampered_candidate_is_refused_before_write(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            data = dict(self.data)
            data[next(n for n in data if n.endswith('/spell.json'))] = b'tampered'
            with self.assertRaisesRegex(ValueError, 'OUTPUT_PIN_MISMATCH'):
                importer.write_import(root, root / 'imports/spells/r30', data, self.manifest)
            self.assertFalse((root / 'imports/spells/r30').exists())

    def test_new_write_preserves_r28_r29_and_active_content_and_refuses_replay(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            protected = [root / 'imports/spells/r28/import-manifest.json', root / 'imports/spells/r29/import-manifest.json', root / 'content/spells.manifest.json']
            for path in protected:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b'immutable existing artifact')
            before = {p: p.read_bytes() for p in protected}
            target = root / 'imports/spells/r30'
            importer.write_import(root, target, self.data, self.manifest)
            self.assertEqual(before, {p: p.read_bytes() for p in protected})
            with self.assertRaisesRegex(ValueError, 'IMPORT_SET_ALREADY_EXISTS'):
                importer.write_import(root, target, self.data, self.manifest)

    def test_provider_equivalence_claim_and_old_destination_are_refused(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            active = copy.deepcopy(self.manifest)
            active['input_provider_equivalence'] = True
            with self.assertRaisesRegex(ValueError, 'ACTIVE_OR_EQUIVALENT_CLAIM_REFUSED'):
                importer.write_import(root, root / 'imports/spells/r30', self.data, active)
            with self.assertRaisesRegex(ValueError, 'IMPORT_DESTINATION_REFUSED'):
                importer.write_import(root, root / 'imports/spells/r29', self.data, self.manifest)


if __name__ == '__main__':
    unittest.main()
