import copy
import json
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

import import_source_spell_package as importer


class SourceReferenceImportTests(unittest.TestCase):
    def fixture(self):
        data = {'monster-verification-proof.json': b'{"fixture":"source_only"}\n'}
        manifest = importer.manifest_for(data, {}, {}, {'fixture': 1}, {}, {'status': 'pending'})
        return data, manifest

    def test_tampered_source_hash_refuses_before_output(self):
        data, manifest = self.fixture()
        with TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / 'imports/spells/r28'
            data['monster-verification-proof.json'] = b'tampered'
            with self.assertRaisesRegex(ValueError, 'PREWRITE_ARTIFACT_MISMATCH'):
                importer.write_import(data, {}, manifest, target, root)
            self.assertFalse(target.exists())

    def test_receipt_pin_rejects_tampered_monster_archive(self):
        with self.assertRaisesRegex(ValueError, 'SOURCE_HASH_MISMATCH'):
            importer.verify_monster_archive({'monster-source-package.tar.gz': b'tampered'},
                                            {'archive': {'sha256': importer.digest(b'original')}})

    def test_import_preserves_active_catalog_and_content_manifest(self):
        data, manifest = self.fixture()
        with TemporaryDirectory() as directory:
            root = Path(directory)
            live = [root / 'content/abilities/definitions/player-spells.json', root / 'content/spells.manifest.json']
            for path in live:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b'active input must remain identical')
            before = {p: p.read_bytes() for p in live}
            target = root / 'imports/spells/r28'
            importer.write_import(data, {}, manifest, target, root)
            self.assertEqual({p: p.read_bytes() for p in live}, before)
            self.assertEqual((target / 'monster-verification-proof.json').read_bytes(), data['monster-verification-proof.json'])
            self.assertFalse(manifest['runtime_activation'])

    def test_existing_import_set_is_not_overwritten(self):
        data, manifest = self.fixture()
        with TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / 'imports/spells/r28'
            importer.write_import(data, {}, manifest, target, root)
            before = (target / 'import-manifest.json').read_bytes()
            with self.assertRaisesRegex(ValueError, 'IMPORT_SET_ALREADY_EXISTS'):
                importer.write_import(data, {}, manifest, target, root)
            self.assertEqual((target / 'import-manifest.json').read_bytes(), before)

    def test_wrong_destination_and_active_claim_refused(self):
        data, manifest = self.fixture()
        with TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaisesRegex(ValueError, 'IMPORT_DESTINATION_OUTSIDE_R28'):
                importer.write_import(data, {}, manifest, root / 'content', root)
            active = copy.deepcopy(manifest)
            active['runtime_activation'] = True
            with self.assertRaisesRegex(ValueError, 'ACTIVE_IMPORT_REFUSED'):
                importer.write_import(data, {}, active, root / 'imports/spells/r28', root)

    def test_schema_snapshot_tampering_refused(self):
        data, manifest = self.fixture()
        schema = b'{"type":"object"}'
        manifest['schemaRefs'] = [{'path': 'schemas/fixture.schema.json', 'uri': 'urn:fixture', 'sha256': importer.digest(schema)}]
        with TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaisesRegex(ValueError, 'PREWRITE_SCHEMA_MISMATCH'):
                importer.write_import(data, {'fixture.schema.json': b'changed'}, manifest, root / 'imports/spells/r28', root)

    def test_actual_player_bundle_receipts_resolve_external_schema_uris(self):
        source = importer.ROOT / importer.SOURCE
        data = {name: (source / name).read_bytes() for name in [
            'player-source-registrars.jsonl.gz', 'player-source-authoring-projections.jsonl.gz', 'monster-verification-proof.json']}
        schemas, _, _ = importer.snapshot_schemas(importer.ROOT, source)
        result = importer.verified_player_bundles(source, data, schemas, importer.ROOT)
        self.assertEqual(result['records'], 483)
        self.assertFalse(result['execution_complete'])
        self.assertIn('player-source-bundles/source-callback-facts.jsonl.gz', data)

    def test_inside_package_file_symlink_is_refused_before_resolution(self):
        with TemporaryDirectory() as directory:
            folder = Path(directory)
            (folder / 'actual.json').write_bytes(b'{}')
            (folder / 'alias.json').symlink_to(folder / 'actual.json')
            with self.assertRaisesRegex(ValueError, 'PLAYER_BUNDLE_SYMLINK_REFUSED'):
                importer.bundle_member_path(folder, 'alias.json')

    def test_inside_package_directory_symlink_is_refused(self):
        with TemporaryDirectory() as directory:
            folder = Path(directory)
            (folder / 'actual').mkdir()
            (folder / 'actual/file.json').write_bytes(b'{}')
            (folder / 'alias').symlink_to(folder / 'actual', target_is_directory=True)
            with self.assertRaisesRegex(ValueError, 'PLAYER_BUNDLE_SYMLINK_REFUSED'):
                importer.bundle_member_path(folder, 'alias/file.json')

    def test_actual_formula_population_schema_context_and_conservation(self):
        source = importer.ROOT / importer.SOURCE
        schemas, _, _ = importer.snapshot_schemas(importer.ROOT, source)
        callback = 'player-source-bundles/source-callback-facts.jsonl.gz'
        data = {callback: (source / callback).read_bytes(),
                'monster-import-summary.json': (source / 'monster-import-summary.json').read_bytes(),
                'player-source-registrars.jsonl.gz': (source / 'player-source-registrars.jsonl.gz').read_bytes()}
        counts = importer.verified_supplements(source, data, schemas)
        self.assertEqual(counts['source-formula-evidence.jsonl.gz']['records'], 87)
        self.assertEqual(counts['player-source-guards.jsonl.gz']['guards'], 642)
        self.assertEqual(counts['source-custom-mechanics.json.gz']['records'], 21)
        self.assertIn('source-formula-population.json', data)

    def test_tampered_formula_selection_population_is_refused(self):
        source = importer.ROOT / importer.SOURCE
        schemas, _, _ = importer.snapshot_schemas(importer.ROOT, source)
        with TemporaryDirectory() as directory:
            folder = Path(directory)
            for name in ('source-formula-evidence.jsonl.gz', 'source-formula-evidence-receipt.json'):
                (folder / name).write_bytes((source / name).read_bytes())
            (folder / 'source-formula-population.json').write_bytes(b'{}')
            with self.assertRaisesRegex(ValueError, 'FORMULA_COHORT_HASH_MISMATCH'):
                importer.verified_supplements(folder, {'monster-import-summary.json': (source / 'monster-import-summary.json').read_bytes()}, schemas)

    def test_supplement_foreign_donor_revision_is_refused(self):
        source = importer.ROOT / importer.SOURCE
        schemas, _, _ = importer.snapshot_schemas(importer.ROOT, source)
        with TemporaryDirectory() as directory:
            folder = Path(directory)
            name = 'source-formula-evidence.jsonl.gz'
            proof_name = 'source-formula-evidence-receipt.json'
            (folder / name).write_bytes((source / name).read_bytes())
            proof = json.loads((source / proof_name).read_bytes())
            proof['source_revisions'] = ['0' * 40]
            (folder / proof_name).write_bytes(importer.encoded(proof))
            with self.assertRaisesRegex(ValueError, 'SUPPLEMENT_DONOR_PIN_MISMATCH'):
                importer.verified_supplements(folder, {'monster-import-summary.json': (source / 'monster-import-summary.json').read_bytes()}, schemas)


if __name__ == '__main__':
    unittest.main()
