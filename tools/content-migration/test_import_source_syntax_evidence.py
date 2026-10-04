import base64
import copy
import hashlib
import gzip
import json
from pathlib import Path
from tempfile import TemporaryDirectory
from types import SimpleNamespace
import unittest

import import_source_syntax_evidence as importer


class SourceSyntaxImportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.source = importer.ROOT / 'docs/reference/spells/r38-source-closure'
        schema = json.loads((importer.ROOT / 'tools/content-schema/spell-authoring/source-syntax.schema.json').read_bytes())
        cls.validators = importer.schema_validators(schema)
        with gzip.open(cls.source / 'source-syntax.jsonl.gz', 'rb') as stream: cls.row = json.loads(stream.readline())
        cls.inventory = {importer.row_identity(cls.row)}

    def test_actual_first_source_tree_closed_schema_and_conservation(self):
        importer.validate_row(self.row, self.inventory, self.validators)
        self.assertGreater(self.row['node_count'], 100)

    def test_source_sha_or_path_forgery_is_refused(self):
        for field, value in [('source_sha256', '0' * 64), ('path', 'data/scripts/false-source.lua')]:
            bad = copy.deepcopy(self.row); bad[field] = value
            with self.assertRaisesRegex(ValueError, 'SYNTAX_FROZEN_SOURCE_IDENTITY_MISMATCH'):
                importer.validate_row(bad, self.inventory, self.validators)

    def test_node_kind_count_or_id_forgery_is_refused(self):
        bad = copy.deepcopy(self.row); bad['node_kind_counts']['Name'] += 1
        with self.assertRaisesRegex(ValueError, 'SYNTAX_NODE_KIND_CONSERVATION_MISMATCH'):
            importer.validate_row(bad, self.inventory, self.validators)
        bad = copy.deepcopy(self.row); bad['ast']['nodes'][1]['id'] = 2
        with self.assertRaisesRegex(ValueError, 'SYNTAX_NODE_ID_MISMATCH'):
            importer.validate_row(bad, self.inventory, self.validators)

    def test_cycle_and_changed_array_order_are_refused(self):
        bad = copy.deepcopy(self.row); bad['ast']['nodes'][0]['fields']['body']['node_ref'] = 0
        with self.assertRaisesRegex(ValueError, 'SYNTAX_CHILD_REFERENCE_MISMATCH'):
            importer.validate_row(bad, self.inventory, self.validators)
        bad = copy.deepcopy(self.row); bad['ast']['nodes'][1]['fields']['body'].reverse()
        with self.assertRaisesRegex(ValueError, 'SYNTAX_ORDERED_CHILD_REFERENCE_MISMATCH'):
            importer.validate_row(bad, self.inventory, self.validators)

    def test_unknown_node_fields_remain_rejected_by_exact_branch_schema(self):
        from jsonschema.exceptions import ValidationError
        bad = copy.deepcopy(self.row); bad['ast']['nodes'][0]['fields']['unexpected'] = 1
        with self.assertRaises(ValidationError): importer.validate_row(bad, self.inventory, self.validators)

    def test_dependency_pin_forgery_is_refused(self):
        proof = json.loads((self.source / 'source-syntax-receipt.json').read_bytes()); proof['dependencies']['luaparser'] = '4.2.1'
        with self.assertRaisesRegex(ValueError, 'SYNTAX_DEPENDENCY_VERSION_PIN_MISMATCH'):
            importer.dependency_provenance(proof)

    def test_actual_metadata_hashes_and_invalid_generation_record_digest(self):
        proof = json.loads((self.source / 'source-syntax-receipt.json').read_bytes())
        importer.dependency_provenance(proof)
        proof['dependency_provenance']['multimethod']['record_sha256'] = 'invalid-generation-hash'
        with self.assertRaisesRegex(ValueError, 'SYNTAX_DEPENDENCY_PROVENANCE_HASH_SHAPE_MISMATCH'):
            importer.dependency_provenance(proof)

    def test_historical_record_is_preserved_without_current_wrapper_equality(self):
        proof = json.loads((self.source / 'source-syntax-receipt.json').read_bytes())
        proof['dependency_provenance']['luaparser']['record_sha256'] = 'a' * 64
        importer.dependency_provenance(proof)
        self.assertEqual(proof['dependency_provenance']['luaparser']['record_sha256'], 'a' * 64)

    def test_current_python_module_integrity_with_relocated_cli_wrapper(self):
        with TemporaryDirectory() as directory:
            root = Path(directory); metadata = root / 'example-1.dist-info'; metadata.mkdir()
            body = b'constant = 1\n'; module = root / 'example.py'; module.write_bytes(body)
            wrapper = root / 'bin/example'; wrapper.parent.mkdir(); wrapper.write_bytes(b'#!/new/absolute/venv/python\n')
            record = metadata / 'RECORD'
            module_hash = base64.urlsafe_b64encode(hashlib.sha256(body).digest()).rstrip(b'=').decode()
            record.write_text('example.py,sha256=' + module_hash + ',' + str(len(body)) + '\n'
                              + 'bin/example,sha256=old-wrapper-hash,1\nexample-1.dist-info/RECORD,,\n')
            distribution = SimpleNamespace(files=[Path('example.py'), Path('bin/example'), Path('example-1.dist-info/RECORD')], locate_file=lambda path: root / path)
            importer.verify_current_python_record(distribution)
            module.write_bytes(b'constant = 2\n')
            with self.assertRaisesRegex(ValueError, 'SYNTAX_DEPENDENCY_PYTHON_MODULE_INTEGRITY_MISMATCH'):
                importer.verify_current_python_record(distribution)

    def test_output_hash_activation_and_previous_imports_guards(self):
        # Exercise the real shared writer with a small candidate set; full packet preflight is independently recorded.
        data = {'evidence/source-syntax.jsonl.gz': b'evidence'}
        manifest = {'admission_status': 'source_only_not_active', 'revision': 38, 'full_spell_candidates': 0, 'native_identity_allocation': False, 'canonical_selection_changed': False,
                    'native_execution_qualified': False, 'input_provider_equivalence': False, **{k: False for k in importer.FALSE_FLAGS},
                    'artifacts': [{'path': p, 'sha256': importer.digest(b), 'bytes': len(b)} for p, b in data.items()]}
        with TemporaryDirectory() as directory:
            root = Path(directory); destination = root / 'imports/spells/r38'
            active = root / 'content/spells.manifest.json'; active.parent.mkdir(); active.write_bytes(b'active')
            old = root / 'imports/spells/r28/import-manifest.json'; old.parent.mkdir(parents=True); old.write_bytes(b'old')
            bad = copy.deepcopy(manifest); bad['execution_qualified'] = True
            with self.assertRaisesRegex(ValueError, 'SYNTAX_ACTIVATION_CLAIM_REFUSED'): importer.write_import(root, destination, data, bad)
            with self.assertRaisesRegex(ValueError, 'OUTPUT_PIN_MISMATCH'): importer.write_import(root, destination, {'evidence/source-syntax.jsonl.gz': b'forgery'}, manifest)
            importer.write_import(root, destination, data, manifest)
            self.assertEqual(active.read_bytes(), b'active'); self.assertEqual(old.read_bytes(), b'old')
            with self.assertRaisesRegex(ValueError, 'IMPORT_SET_ALREADY_EXISTS'): importer.write_import(root, destination, data, manifest)


if __name__ == '__main__': unittest.main()
