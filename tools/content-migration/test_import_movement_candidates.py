import copy
import json
from pathlib import Path
import shutil
from tempfile import TemporaryDirectory
import unittest

import import_movement_candidates as importer
from import_source_spell_package import encoded


class MovementImportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data, cls.manifest = importer.prepare_import(importer.ROOT)

    def test_reviewed_population_and_archived_schema_bytes(self):
        self.assertEqual(self.manifest['source_keys'], list(importer.REGISTRATIONS))
        self.assertEqual(self.manifest['counts'], {'records': 2, 'status_counts': {'CANDIDATE_SCHEMA_VALID': 2}, 'native_descriptors': 2, 'abilities': 0, 'effects': 0, 'formulas': 0})
        for name, body in self.data.items():
            if name.startswith('schemas/'):
                self.assertEqual(body, (importer.ROOT / 'imports/spells/r28' / name).read_bytes())
        for field in ['runtime_activation', 'native_identity_allocation', 'canonical_selection_changed', 'native_execution_qualified', 'input_provider_equivalence']:
            self.assertIs(self.manifest[field], False)

    def test_rehashed_active_source_mutation_refused_by_review_pin(self):
        with TemporaryDirectory(dir=importer.ROOT) as tmp:
            source = Path(tmp) / 'packet'
            shutil.copytree(importer.ROOT / importer.SOURCE, source)
            member = next(source.glob('*/*/receipt.json'))
            receipt = json.loads(member.read_bytes())
            receipt['runtime_activation'] = True
            member.write_bytes(encoded(receipt))
            path = source / 'package-manifest.json'
            manifest = json.loads(path.read_bytes())
            manifest['files'][member.relative_to(source).as_posix()] = importer.digest(member.read_bytes())
            path.write_bytes(encoded(manifest))
            with self.assertRaisesRegex(ValueError, 'SOURCE_REVIEW_PIN_MISMATCH'):
                importer.prepare_import(importer.ROOT, source)

    def test_base_tamper_and_extra_original_asset_rejected(self):
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            base = root / 'imports/spells/r28/import-manifest.json'
            base.parent.mkdir(parents=True)
            base.write_bytes(b'changed')
            with self.assertRaisesRegex(ValueError, 'BASE_MANIFEST_PIN_MISMATCH'):
                importer.prepare_import(root)
        with TemporaryDirectory(dir=importer.ROOT) as tmp:
            source = Path(tmp) / 'packet'
            shutil.copytree(importer.ROOT / importer.SOURCE, source)
            (source / 'asset.png').write_bytes(b'original')
            with self.assertRaisesRegex(ValueError, 'SOURCE_PACKAGE_MEMBERSHIP_MISMATCH'):
                importer.prepare_import(importer.ROOT, source)

    def test_nested_source_header_cannot_be_changed_but_engine_defaults_are_allowed(self):
        header = {'costs': {'mana': 20}, 'targeting': {'self_target': True}, 'groups': [{'group': 'support'}]}
        filled = copy.deepcopy(header)
        filled['costs']['soul'] = 0
        self.assertTrue(importer.preserves_header(header, filled))
        filled['costs']['mana'] = 21
        self.assertFalse(importer.preserves_header(header, filled))
        self.assertFalse(importer.preserves_header(header, {'costs': {'mana': 20}}))

    def test_changed_descriptor_and_synthetic_dependencies_refused(self):
        spell = json.loads(next(body for name, body in self.data.items() if name.endswith('/spell.json')))
        deps = {'abilities': [], 'effects': [], 'formulas': []}
        expected = copy.deepcopy(spell['spell']['execution']['native_behavior'])
        changed = copy.deepcopy(spell)
        changed['spell']['execution']['native_behavior']['parameters']['success_effect'] = 'poff'
        with self.assertRaisesRegex(ValueError, 'MOVEMENT_DESCRIPTOR_MISMATCH'):
            importer.validate_descriptor(changed, deps, {'definitions': []}, expected)
        with self.assertRaisesRegex(ValueError, 'MOVEMENT_DEPENDENCIES_REFUSED'):
            importer.validate_descriptor(spell, dict(deps, abilities=[{}]), {'definitions': []}, expected)
        with self.assertRaisesRegex(ValueError, 'MOVEMENT_CATALOG_REFUSED'):
            importer.validate_descriptor(spell, deps, {'definitions': [{}]}, expected)

    def test_wrong_destination_family_flags_and_changed_bytes_before_write(self):
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            destination = root / 'imports/spells/r46'
            with self.assertRaisesRegex(ValueError, 'IMPORT_DESTINATION_REFUSED'):
                importer.write_import(root, root / 'imports/spells/r28', self.data, self.manifest)
            wrong = copy.deepcopy(self.manifest)
            wrong['source_keys'].pop()
            with self.assertRaisesRegex(ValueError, 'WRONG_MOVEMENT_IMPORT_FAMILY'):
                importer.write_import(root, destination, self.data, wrong)
            for field in ['runtime_activation', 'native_identity_allocation', 'canonical_selection_changed', 'native_execution_qualified', 'input_provider_equivalence']:
                active = copy.deepcopy(self.manifest)
                active[field] = True
                with self.assertRaisesRegex(ValueError, 'ACTIVE_OR_EQUIVALENT_CLAIM_REFUSED'):
                    importer.write_import(root, destination, self.data, active)
            data = dict(self.data)
            data[next(name for name in data if name.endswith('/spell.json'))] = b'changed'
            with self.assertRaisesRegex(ValueError, 'OUTPUT_PIN_MISMATCH'):
                importer.write_import(root, destination, data, self.manifest)
            self.assertFalse(destination.exists())

    def test_new_copy_preserves_prior_set_and_refuses_replay(self):
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            old = root / 'imports/spells/r28/import-manifest.json'
            old.parent.mkdir(parents=True)
            old.write_bytes(b'immutable')
            target = root / 'imports/spells/r46'
            result = importer.write_import(root, target, self.data, self.manifest)
            self.assertFalse(result['runtime_activation'])
            self.assertEqual(old.read_bytes(), b'immutable')
            self.assertEqual({p.relative_to(target).as_posix() for p in target.rglob('*') if p.is_file()}, set(self.data) | {'import-manifest.json', 'SHA256SUMS'})
            for name, body in self.data.items():
                self.assertEqual((target / name).read_bytes(), body)
            with self.assertRaisesRegex(ValueError, 'IMPORT_SET_ALREADY_EXISTS'):
                importer.write_import(root, target, self.data, self.manifest)


if __name__ == '__main__':
    unittest.main()
