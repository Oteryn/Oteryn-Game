import copy
import json
from pathlib import Path
import shutil
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch

import import_source_complete_candidate_batches as importer
from import_source_spell_package import encoded


class SourceCompleteCandidateImportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.reviewed = [revision for revision, config in importer.CONFIGS.items() if 'manifest_sha' in config]
        cls.prepared = {revision: importer.prepare_import(importer.ROOT, revision) for revision in cls.reviewed}

    def test_review_is_required_and_private_contract_flags_cannot_be_promoted(self):
        with patch.dict(importer.CONFIGS, {59: {'source': 'missing'}}, clear=False):
            with self.assertRaisesRegex(ValueError, 'SOURCE_COMPLETE_REVIEW_PENDING'):
                importer.prepare_import(importer.ROOT, 59)
        source_only = {'runtime_activation': False, 'native_execution_qualified': False, 'native_identity_allocation': False,
                       'canonical_selection_changed': False, 'authoring_contract_extension_pending': True,
                       'source_consumer_implemented': False, 'target_schema_family': importer.FAMILY,
                       'input_provider_equivalence': False}
        for field, value in [('authoring_contract_extension_pending', False), ('source_consumer_implemented', True),
                             ('target_schema_family', 'accepted_v1'), ('input_provider_equivalence', True)]:
            altered = dict(source_only, **{field: value})
            with self.assertRaisesRegex(ValueError, 'SOURCE_COMPLETE_CONTRACT_OR_CONSUMER_CLAIM_REFUSED'):
                importer.pending_data(altered)
        with TemporaryDirectory() as tmp:
            root = Path(tmp); name = 'imports/spells/r28/evidence.json'
            path = root / name; path.parent.mkdir(parents=True); path.write_bytes(b'original')
            proof = {'input_proofs': {name: importer.digest(path.read_bytes())}}
            importer.verify_declared_inputs(root, proof)
            path.write_bytes(b'changed')
            with self.assertRaisesRegex(ValueError, 'SOURCE_COMPLETE_INPUT_PIN_MISMATCH'):
                importer.verify_declared_inputs(root, proof)
            with self.assertRaisesRegex(ValueError, 'SOURCE_COMPLETE_INPUT_PATH_REFUSED'):
                importer.verify_declared_inputs(root, {'input_proofs': {'../outside.json': 'x'}})

    def test_every_reviewed_packet_uses_identical_global_schema_and_exact_original_headers(self):
        self.assertTrue(self.reviewed, 'No reviewed source-complete packet is configured')
        primary = []
        for revision, (data, manifest) in self.prepared.items():
            self.assertTrue(manifest['authoring_contract_extension_pending'])
            self.assertFalse(manifest['source_consumer_implemented'])
            self.assertEqual(manifest['target_schema_family'], importer.FAMILY)
            self.assertEqual(manifest['counts']['full_source_data_candidates'] + manifest['counts']['reference_records'], manifest['counts']['audit_records'])
            for ref in manifest['schemaRefs']:
                self.assertEqual(importer.digest(data[ref['path']]), ref['sha256'])
                if ref['uri'] == importer.SPELL_URI:
                    primary.append(data[ref['path']])
            for reg in manifest['source_keys']:
                path = importer.folder(reg) + '/source-header.json'
                self.assertEqual(data['player-source-candidates/' + path], (importer.ROOT / 'imports/spells/r28/player-source-bundles' / path).read_bytes())
        self.assertTrue(primary)
        self.assertTrue(all(body == primary[0] for body in primary))

    def test_rehashed_source_consumer_admission_mutation_refused(self):
        for revision in self.reviewed:
            config = importer.CONFIGS[revision]
            with TemporaryDirectory(dir=importer.ROOT) as tmp:
                source = Path(tmp) / 'packet'
                shutil.copytree(importer.ROOT / config['source'], source)
                extra = source / 'unreviewed.json'
                extra.write_bytes(b'{}')
                with self.assertRaisesRegex(ValueError, 'SOURCE_COMPLETE_PACKET_MEMBERSHIP_MISMATCH'):
                    importer.prepare_import(importer.ROOT, revision, source)
                extra.unlink()
                proof_path = source / 'projection-proof.json'
                proof = json.loads(proof_path.read_bytes()); proof['source_consumer_implemented'] = True
                proof_path.write_bytes(encoded(proof))
                manifest_path = source / 'package-manifest.json'
                manifest = json.loads(manifest_path.read_bytes()); manifest['files']['projection-proof.json'] = importer.digest(proof_path.read_bytes())
                manifest_path.write_bytes(encoded(manifest))
                with self.assertRaisesRegex(ValueError, 'SOURCE_COMPLETE_MANIFEST_REVIEW_PIN_MISMATCH'):
                    importer.prepare_import(importer.ROOT, revision, source)

    def test_schema_uri_and_closed_global_resource_bytes_cannot_be_replaced(self):
        base, _, baseline, schemas, _ = importer.read_base(importer.ROOT, importer.BASE_SHA)
        for revision, (data, _) in self.prepared.items():
            packet = {name.split('/', 1)[1]: body for name, body in data.items() if name.startswith('player-source-candidates/')}
            proof = json.loads(packet['projection-proof.json'])
            for role in ['spell', 'dependencies', 'receipt']:
                changed = copy.deepcopy(proof)
                changed['schema_roles'][role] = 'urn:oteryn:spell-authoring:candidate:1'
                with self.assertRaisesRegex(ValueError, 'SOURCE_COMPLETE_SCHEMA_ROLES_MISMATCH'):
                    importer.schema_validators(packet, changed, schemas, baseline)
            primary = next(ref for ref in proof['schema_resources'] if ref['uri'] == importer.SPELL_URI)
            changed_packet = dict(packet); changed_packet[primary['path']] = b'changed'
            with self.assertRaisesRegex(ValueError, 'SOURCE_COMPLETE_SCHEMA_BYTES_PIN_MISMATCH'):
                importer.schema_validators(changed_packet, proof, schemas, baseline)

    def test_source_headers_dependency_closure_and_missing_operations_refused(self):
        base, _, baseline, schemas, _ = importer.read_base(importer.ROOT, importer.BASE_SHA)
        for revision, (data, _) in self.prepared.items():
            packet = {name.split('/', 1)[1]: body for name, body in data.items() if name.startswith('player-source-candidates/')}
            proof = json.loads(packet['projection-proof.json']); summary = json.loads(packet['import-summary.json'])
            audit = {row['registration_key']: row for row in importer.audit_rows(json.loads(packet['lane-audit.json']))}
            validators, _ = importer.schema_validators(packet, proof, schemas, baseline)
            row = next(row for row in audit.values() if row['status'] == importer.FULL)
            path = importer.folder(row['registration_key'])
            altered = dict(packet); header = json.loads(packet[path + '/source-header.json']); header['spell']['name'] += ' changed'
            altered[path + '/source-header.json'] = encoded(header)
            with self.assertRaisesRegex(ValueError, 'SOURCE_COMPLETE_SOURCE_HEADER_CHANGED'):
                importer.verify_models(altered, proof, summary, audit, base, validators, importer.CONFIGS[revision])
            altered = dict(packet); projection = json.loads(packet[path + '/projection-receipt.json']); projection['required_operations_unrepresented'] = ['missing_cast_guard']
            altered[path + '/projection-receipt.json'] = encoded(projection)
            with self.assertRaisesRegex(ValueError, 'SOURCE_COMPLETE_ALIAS_OR_MISSING_OPERATION_REFUSED'):
                importer.verify_models(altered, proof, summary, audit, base, validators, importer.CONFIGS[revision])
            spell = json.loads(packet[path + '/spell.json']); deps = json.loads(packet[path + '/dependencies.json'])
            catalog = json.loads(packet[path + '/catalog.json']); receipt = json.loads(packet[path + '/receipt.json'])
            catalog['definitions'].append({'family': 'Item', 'key': 'candidate:item/unreferenced', 'revision': 'fixture'})
            with self.assertRaisesRegex(ValueError, 'SOURCE_COMPLETE_DEPENDENCY_CLOSURE_MISMATCH'):
                importer.validate_dependency_closure(spell, deps, catalog, receipt)

    def test_write_guards_preserve_old_sets_refuse_rehashed_members_counts_and_replay(self):
        for revision, (data, manifest) in self.prepared.items():
            with TemporaryDirectory() as tmp:
                root = Path(tmp); destination = root / 'imports/spells' / ('r' + str(revision))
                with self.assertRaisesRegex(ValueError, 'IMPORT_DESTINATION_REFUSED'):
                    importer.write_import(root, revision, root / 'imports/spells/r28', data, manifest)
                false_counts = copy.deepcopy(manifest); false_counts['counts']['full_source_data_candidates'] += 1
                with self.assertRaisesRegex(ValueError, 'SOURCE_COMPLETE_OUTPUT_COUNTS_OR_KEYS_MISMATCH'):
                    importer.write_import(root, revision, destination, data, false_counts)
                altered = dict(data); name = 'player-source-candidates/lane-audit.json'; altered[name] = b'changed'
                rehashed = copy.deepcopy(manifest); entry = next(row for row in rehashed['artifacts'] if row['path'] == name)
                entry.update(sha256=importer.digest(altered[name]), bytes=len(altered[name]))
                with self.assertRaisesRegex(ValueError, 'SOURCE_COMPLETE_OUTPUT_MEMBER_PIN_MISMATCH'):
                    importer.write_import(root, revision, destination, altered, rehashed)
                schema_name = json.loads(data['base-r28/import-manifest.json'])['schemaRefs'][0]['path']
                altered = dict(data); altered[schema_name] = b'changed archived schema'
                rehashed = copy.deepcopy(manifest)
                entry = next(row for row in rehashed['artifacts'] if row['path'] == schema_name)
                entry.update(sha256=importer.digest(altered[schema_name]), bytes=len(altered[schema_name]))
                with self.assertRaisesRegex(ValueError, 'SOURCE_COMPLETE_OUTPUT_BASE_SCHEMA_PIN_MISMATCH'):
                    importer.write_import(root, revision, destination, altered, rehashed)
                changed_roles = copy.deepcopy(manifest)
                changed_roles['schema_roles']['receipt'] = importer.SPELL_URI
                with self.assertRaisesRegex(ValueError, 'SOURCE_COMPLETE_OUTPUT_SCHEMA_METADATA_MISMATCH'):
                    importer.write_import(root, revision, destination, data, changed_roles)
                self.assertFalse(destination.exists())
                old = root / 'imports/spells/r28/import-manifest.json'; old.parent.mkdir(parents=True); old.write_bytes(b'immutable')
                result = importer.write_import(root, revision, destination, data, manifest)
                self.assertFalse(result['runtime_activation']); self.assertEqual(old.read_bytes(), b'immutable')
                self.assertEqual({p.relative_to(destination).as_posix() for p in destination.rglob('*') if p.is_file()}, set(data) | {'import-manifest.json', 'SHA256SUMS'})
                for name, body in data.items(): self.assertEqual((destination / name).read_bytes(), body)
                with self.assertRaisesRegex(ValueError, 'IMPORT_SET_ALREADY_EXISTS'):
                    importer.write_import(root, revision, destination, data, manifest)


if __name__ == '__main__':
    unittest.main()
