import copy
import gzip
import json
from pathlib import Path
import shutil
from tempfile import TemporaryDirectory
import unittest

import import_source_program_batches as importer
from import_source_spell_package import encoded


class SourceProgramImportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.prepared = {revision: importer.prepare_import(importer.ROOT, revision) for revision in importer.CONFIGS}

    def test_exact_three_source_populations_without_spell_promotion(self):
        for revision, (data, manifest) in self.prepared.items():
            self.assertEqual(manifest['counts']['source_records'], importer.CONFIGS[revision]['records'])
            self.assertEqual(manifest['counts']['full_spell_candidates'], 0)
            self.assertEqual(manifest['counts']['executable_spell_count'], 0)
            for flag in importer.FALSE_FLAGS:
                self.assertIs(manifest[flag], False)
            for path, body in data.items():
                if path.startswith('schemas/') and (importer.ROOT / 'imports/spells/r28' / path).exists():
                    self.assertEqual(body, (importer.ROOT / 'imports/spells/r28' / path).read_bytes())

    def test_immutable_base_tamper_and_unknown_family_rejected(self):
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            base = root / 'imports/spells/r28/import-manifest.json'
            base.parent.mkdir(parents=True)
            base.write_bytes(b'changed')
            with self.assertRaisesRegex(ValueError, 'BASE_MANIFEST_PIN_MISMATCH'):
                importer.prepare_import(root, 49)
            with self.assertRaisesRegex(ValueError, 'UNSUPPORTED_SOURCE_PROGRAM_FAMILY'):
                importer.prepare_import(root, 48)

    def test_changed_program_and_rehashed_activation_proof_rejected(self):
        for revision, config in importer.CONFIGS.items():
            with TemporaryDirectory(dir=importer.ROOT) as tmp:
                source = Path(tmp) / 'packet'
                source.mkdir()
                for name in [config['artifact'], config['proof']] + ([config['receipt']] if 'receipt' in config else []):
                    shutil.copyfile(importer.ROOT / config['folder'] / name, source / name)
                (source / config['artifact']).write_bytes(b'changed')
                with self.assertRaisesRegex(ValueError, 'SOURCE_PROGRAM_REVIEW_PIN_MISMATCH'):
                    importer.prepare_import(importer.ROOT, revision, source)
            with TemporaryDirectory(dir=importer.ROOT) as tmp:
                source = Path(tmp) / 'packet'
                source.mkdir()
                shutil.copyfile(importer.ROOT / config['folder'] / config['artifact'], source / config['artifact'])
                proof = json.loads((importer.ROOT / config['folder'] / config['proof']).read_bytes())
                proof['runtime_activation'] = True
                (source / config['proof']).write_bytes(encoded(proof))
                with self.assertRaisesRegex(ValueError, 'SOURCE_PROGRAM_PROOF_REVIEW_PIN_MISMATCH'):
                    importer.prepare_import(importer.ROOT, revision, source)

    def test_typed_program_operand_formula_and_monster_parameter_source_drift_rejected(self):
        base, _, baseline, _, _ = importer.read_base(importer.ROOT, importer.BASE_SHA)
        for revision, (data, _) in self.prepared.items():
            config = importer.CONFIGS[revision]
            payload = gzip.decompress(data['source-programs/' + config['artifact']])
            document = [json.loads(line) for line in payload.splitlines()] if config.get('jsonl') else json.loads(payload)
            proof = json.loads(data['source-programs/' + config['proof']])
            if revision == 49:
                document[0]['instructions'][0]['operands'] = {}
                expected = 'CAST_PROGRAM_AST_OPERAND_MISMATCH'
            elif revision == 50:
                document['formula_definitions'][0]['expressions']['minimum'] = {'const': '999'}
                expected = 'MONK_FORMULA_CAPTURE_OR_LINK_MISMATCH'
            else:
                document['slots'][0]['source_parameters']['range'] = 999
                expected = 'MONSTER_SLOT_SOURCE_OR_PROJECTION_MISMATCH'
            with self.assertRaisesRegex(ValueError, expected):
                importer.verify_population(importer.ROOT, revision, document, proof, base, baseline)

    def test_input_proof_path_escape_and_changed_source_rejected(self):
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            with self.assertRaisesRegex(ValueError, 'SOURCE_INPUT_PATH_REFUSED'):
                importer.verify_input_proofs(root, {'imports/spells/../../private': 'bad'})
            with self.assertRaisesRegex(ValueError, 'SOURCE_INPUT_PROOFS_MISSING'):
                importer.verify_input_proofs(root, {})
            path = root / 'imports/spells/r28/example.json'
            path.parent.mkdir(parents=True)
            path.write_bytes(b'changed')
            with self.assertRaisesRegex(ValueError, 'SOURCE_INPUT_PIN_MISMATCH'):
                importer.verify_input_proofs(root, {'imports/spells/r28/example.json': importer.digest(b'old')})

    def test_wrong_family_destination_flags_and_bytes_rejected_before_write(self):
        for revision, (data, manifest) in self.prepared.items():
            with TemporaryDirectory() as tmp:
                root = Path(tmp)
                destination = root / 'imports/spells' / ('r' + str(revision))
                with self.assertRaisesRegex(ValueError, 'IMPORT_DESTINATION_REFUSED'):
                    importer.write_import(root, revision, root / 'imports/spells/r28', data, manifest)
                with self.assertRaisesRegex(ValueError, 'WRONG_SOURCE_PROGRAM_FAMILY'):
                    importer.write_import(root, 48, destination, data, manifest)
                for flag in importer.FALSE_FLAGS:
                    active = copy.deepcopy(manifest)
                    active[flag] = True
                    with self.assertRaisesRegex(ValueError, 'ACTIVATION_REFUSED'):
                        importer.write_import(root, revision, destination, data, active)
                promoted = copy.deepcopy(manifest)
                promoted['full_spell_candidates'] = 1
                with self.assertRaisesRegex(ValueError, 'FULL_SPELL_PROMOTION_REFUSED'):
                    importer.write_import(root, revision, destination, data, promoted)
                altered = dict(data)
                altered[manifest['source_metadata_path']] = b'changed'
                with self.assertRaisesRegex(ValueError, 'SOURCE_PROGRAM_IMPORT_PROOF_PIN_MISMATCH'):
                    importer.write_import(root, revision, destination, altered, manifest)
                executable = copy.deepcopy(manifest)
                executable['executable_spell_count'] = 1
                with self.assertRaisesRegex(ValueError, 'SOURCE_PROGRAM_IMPORT_COUNT_MISMATCH'):
                    importer.write_import(root, revision, destination, data, executable)
                self.assertFalse(destination.exists())

    def test_actual_new_copy_checksum_closure_preserves_prior_sets_and_refuses_replay(self):
        for revision, (data, manifest) in self.prepared.items():
            with TemporaryDirectory() as tmp:
                root = Path(tmp)
                old = root / 'imports/spells/r28/import-manifest.json'
                old.parent.mkdir(parents=True)
                old.write_bytes(b'immutable')
                target = root / 'imports/spells' / ('r' + str(revision))
                result = importer.write_import(root, revision, target, data, manifest)
                self.assertFalse(result['runtime_activation'])
                self.assertEqual(old.read_bytes(), b'immutable')
                self.assertEqual({p.relative_to(target).as_posix() for p in target.rglob('*') if p.is_file()}, set(data) | {'import-manifest.json', 'SHA256SUMS'})
                for name, body in data.items():
                    self.assertEqual((target / name).read_bytes(), body)
                sums = (target / 'SHA256SUMS').read_text().splitlines()
                for line in sums:
                    expected, name = line.split('  ', 1)
                    self.assertEqual(importer.digest((target / name).read_bytes()), expected)
                with self.assertRaisesRegex(ValueError, 'IMPORT_SET_ALREADY_EXISTS'):
                    importer.write_import(root, revision, target, data, manifest)


if __name__ == '__main__':
    unittest.main()
