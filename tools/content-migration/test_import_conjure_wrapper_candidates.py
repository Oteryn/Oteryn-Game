import copy
import json
from pathlib import Path
import shutil
from tempfile import TemporaryDirectory
import unittest

import import_conjure_wrapper_candidates as importer
from import_source_spell_package import encoded


class ConjureWrapperImportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data, cls.manifest = importer.prepare_import(importer.ROOT, importer.ROOT / importer.SOURCE)

    def test_actual_four_wrappers_retain_source_r28_item_provider(self):
        self.assertEqual(set(self.manifest['source_keys']), importer.SOURCE_KEYS)
        self.assertEqual(self.manifest['counts']['records'], 4)
        self.assertEqual(self.manifest['counts']['result_item_type_qualified'], 0)
        for name, body in self.data.items():
            if name.endswith('/spell.json'):
                conjure = json.loads(body)['spell']['execution']['conjure']
                self.assertEqual(conjure['result']['revision'], 'source-player-r28')

    def rewrite_member(self, folder, path, value):
        body = encoded(value); path.write_bytes(body)
        manifest_path = folder / 'package-manifest.json'
        package = json.loads(manifest_path.read_bytes())
        package['files'][path.relative_to(folder).as_posix()] = importer.digest(body)
        manifest_path.write_bytes(encoded(package))

    def test_rehashed_receipt_provenance_forgery_is_refused(self):
        with TemporaryDirectory() as directory:
            folder = Path(directory) / 'source'; shutil.copytree(importer.ROOT / importer.SOURCE, folder)
            path = next(folder.glob('*/*/receipt.json')); receipt = json.loads(path.read_bytes())
            receipt['source_sha256'] = '0' * 64; self.rewrite_member(folder, path, receipt)
            with self.assertRaisesRegex(ValueError, 'WRAPPER_RECEIPT_SOURCE_IDENTITY_MISMATCH'):
                importer.prepare_import(importer.ROOT, folder)

    def test_rehashed_result_type_qualification_forgery_is_refused(self):
        with TemporaryDirectory() as directory:
            folder = Path(directory) / 'source'; shutil.copytree(importer.ROOT / importer.SOURCE, folder)
            path = folder / 'wrapper-source-proof.json'; proof = json.loads(path.read_bytes())
            proof['records'][0]['result_item_type_qualified'] = True; self.rewrite_member(folder, path, proof)
            with self.assertRaisesRegex(ValueError, 'WRAPPER_SOURCE_CONJURE_MISMATCH'):
                importer.prepare_import(importer.ROOT, folder)

    def test_tampered_candidate_bytes_are_refused_before_output(self):
        with TemporaryDirectory() as directory:
            root = Path(directory); data = dict(self.data)
            data[next(n for n in data if n.endswith('/spell.json'))] = b'tampered'
            with self.assertRaisesRegex(ValueError, 'OUTPUT_PIN_MISMATCH'):
                importer.write_import(root, root / 'imports/spells/r34', data, self.manifest)
            self.assertFalse((root / 'imports/spells/r34').exists())

    def test_new_data_set_preserves_earlier_sets_and_active_content_and_refuses_replay(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            protected = [root / 'imports/spells' / ('r' + str(n)) / 'import-manifest.json' for n in range(28, 34)] + [root / 'content/spells.manifest.json']
            for path in protected:
                path.parent.mkdir(parents=True, exist_ok=True); path.write_bytes(b'original input')
            before = {p: p.read_bytes() for p in protected}; target = root / 'imports/spells/r34'
            importer.write_import(root, target, self.data, self.manifest)
            self.assertEqual(before, {p: p.read_bytes() for p in protected})
            with self.assertRaisesRegex(ValueError, 'IMPORT_SET_ALREADY_EXISTS'):
                importer.write_import(root, target, self.data, self.manifest)

    def test_activation_claim_and_base_destination_are_refused(self):
        with TemporaryDirectory() as directory:
            root = Path(directory); forged = copy.deepcopy(self.manifest); forged['runtime_activation'] = True
            with self.assertRaisesRegex(ValueError, 'ACTIVE_OR_EQUIVALENT_CLAIM_REFUSED'):
                importer.write_import(root, root / 'imports/spells/r34', self.data, forged)
            with self.assertRaisesRegex(ValueError, 'IMPORT_DESTINATION_REFUSED'):
                importer.write_import(root, root / 'imports/spells/r28', self.data, self.manifest)


if __name__ == '__main__': unittest.main()
