import copy
import json
from pathlib import Path
import shutil
from tempfile import TemporaryDirectory
import unittest

import import_cast_source_evidence as importer


class CastSourceImportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.prepared = {rev: importer.prepare_import(importer.ROOT, rev) for rev in importer.CONFIGS}

    def test_two_partial_batches_have_no_executable_candidates(self):
        for revision, (data, manifest) in self.prepared.items():
            self.assertEqual(manifest['counts'], {'records': importer.CONFIGS[revision]['records'], 'full_spell_candidates': 0})
            self.assertFalse(manifest['runtime_activation'])
            self.assertFalse(manifest['canonical_selection_changed'])
            self.assertFalse(any(name.endswith('/spell.json') for name in data))
            for name, body in data.items():
                if name.startswith('schemas/') and name not in ['schemas/' + importer.CONFIGS[revision]['schema'], 'schemas/source-syntax.schema.json']:
                    self.assertEqual(body, (importer.ROOT / 'imports/spells/r28' / name).read_bytes())

    def test_rehashed_receipt_changes_cannot_expand_reviewed_scope(self):
        for revision, config in importer.CONFIGS.items():
            with TemporaryDirectory(dir=importer.ROOT) as tmp:
                source = Path(tmp) / 'packet'
                source.mkdir()
                original = importer.ROOT / 'docs/reference/spells' / (revision + '-source-closure')
                for name in [config['proof'], config['artifact']]:
                    shutil.copyfile(original / name, source / name)
                proof = source / config['proof']
                value = json.loads(proof.read_bytes())
                value['runtime_activation'] = True
                proof.write_text(json.dumps(value))
                with self.assertRaisesRegex(ValueError, 'CAST_REVIEW_PIN_MISMATCH'):
                    importer.prepare_import(importer.ROOT, revision, source)

    def test_packet_tampering_refused_and_missing_base_refused(self):
        revision = 'r47'
        config = importer.CONFIGS[revision]
        with TemporaryDirectory(dir=importer.ROOT) as tmp:
            source = Path(tmp)
            original = importer.ROOT / 'docs/reference/spells/r47-source-closure'
            shutil.copyfile(original / config['proof'], source / config['proof'])
            import gzip
            (source / config['artifact']).write_bytes(gzip.compress(b'{}\n', mtime=0))
            with self.assertRaisesRegex(ValueError, 'CAST_PACKET_HASH_MISMATCH'):
                importer.prepare_import(importer.ROOT, revision, source)
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = root / 'imports/spells/r28/import-manifest.json'
            path.parent.mkdir(parents=True)
            path.write_bytes(b'changed')
            with self.assertRaisesRegex(ValueError, 'BASE_MANIFEST_PIN_MISMATCH'):
                importer.prepare_import(root, revision)

    def test_wrong_family_flags_destination_and_modified_bytes_refused(self):
        data, manifest = self.prepared['r47']
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            target = root / 'imports/spells/r47'
            with self.assertRaisesRegex(ValueError, 'WRONG_CAST_IMPORT_FAMILY'):
                importer.write_import(root, 'r48', target, data, manifest)
            with self.assertRaisesRegex(ValueError, 'IMPORT_DESTINATION_REFUSED'):
                importer.write_import(root, 'r47', root / 'imports/spells/r28', data, manifest)
            active = copy.deepcopy(manifest)
            active['native_execution_qualified'] = True
            with self.assertRaisesRegex(ValueError, 'ACTIVE_OR_EQUIVALENT_CLAIM_REFUSED'):
                importer.write_import(root, 'r47', target, data, active)
            altered = dict(data)
            altered[next(n for n in altered if n.startswith('evidence/'))] = b'changed'
            with self.assertRaisesRegex(ValueError, 'OUTPUT_PIN_MISMATCH'):
                importer.write_import(root, 'r47', target, altered, manifest)
            self.assertFalse(target.exists())

    def test_actual_writes_copy_exact_bytes_preserve_base_and_refuse_replay(self):
        for revision, (data, manifest) in self.prepared.items():
            with TemporaryDirectory() as tmp:
                root = Path(tmp)
                protected = root / 'imports/spells/r28/import-manifest.json'
                protected.parent.mkdir(parents=True)
                protected.write_bytes(b'immutable')
                target = root / 'imports/spells' / revision
                result = importer.write_import(root, revision, target, data, manifest)
                self.assertFalse(result['runtime_activation'])
                self.assertEqual(protected.read_bytes(), b'immutable')
                for name, body in data.items():
                    self.assertEqual((target / name).read_bytes(), body)
                self.assertEqual({p.relative_to(target).as_posix() for p in target.rglob('*') if p.is_file()}, set(data) | {'import-manifest.json', 'SHA256SUMS'})
                with self.assertRaisesRegex(ValueError, 'IMPORT_SET_ALREADY_EXISTS'):
                    importer.write_import(root, revision, target, data, manifest)


if __name__ == '__main__':
    unittest.main()
