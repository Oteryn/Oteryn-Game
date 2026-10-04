import copy
import json
from pathlib import Path
import shutil
from tempfile import TemporaryDirectory
import unittest

import import_candidate_unblocking_batches as importer
from import_source_spell_package import encoded


class CandidateUnblockingImportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.reviewed = [revision for revision, config in importer.CONFIGS.items() if 'manifest_sha' in config]
        cls.prepared = {revision: importer.prepare_import(importer.ROOT, revision) for revision in cls.reviewed}

    def test_unreviewed_families_fail_closed_and_unknown_family_refused(self):
        for revision, config in importer.CONFIGS.items():
            if 'manifest_sha' not in config:
                with self.assertRaisesRegex(ValueError, 'UNBLOCKING_REVIEW_PENDING'):
                    importer.prepare_import(importer.ROOT, revision)
        with self.assertRaisesRegex(ValueError, 'UNSUPPORTED_UNBLOCKING_FAMILY'):
            importer.prepare_import(importer.ROOT, 59)

    def test_full_models_are_distinct_from_native_bindings_and_all_headers_preserved(self):
        self.assertTrue(self.reviewed)
        for revision, (data, manifest) in self.prepared.items():
            config = importer.CONFIGS[revision]
            counts = manifest['counts']
            self.assertEqual(counts['full_player_candidates'] + counts['noncandidate_records'], counts['lane_records'])
            self.assertEqual(counts['ordinary_full_player_candidates'] + counts['native_full_player_candidates'], counts['full_player_candidates'])
            self.assertEqual(len(manifest['source_keys']), counts['lane_records'])
            self.assertEqual(counts, config['counts'])
            for reg in manifest['source_keys']:
                path = importer.folder(reg) + '/source-header.json'
                self.assertEqual(data['source-lane-headers/' + path], (importer.ROOT / 'imports/spells/r28/player-source-bundles' / path).read_bytes())
            for field in ['runtime_activation', 'native_identity_allocation', 'native_execution_qualified', 'canonical_selection_changed', 'input_provider_equivalence']:
                self.assertIs(manifest[field], False)

    def test_explicit_vocation_reader_gap_is_counted_without_losing_source_none(self):
        if 58 in self.prepared:
            data, manifest = self.prepared[58]
            self.assertEqual(manifest['counts']['runtime_capability_blocked_candidates'], 2)
            audit = importer.audit_rows(json.loads(data['player-source-candidates/lane-audit.json']))
            blocked = [row for row in audit if row.get('reader_capability_status') == 'SOURCE_SCHEMA_VALID_RUNTIME_CAPABILITY_BLOCKED']
            self.assertEqual(len(blocked), 2)
            for row in blocked:
                path = importer.packet_folder(row['registration_key'], importer.CONFIGS[58])
                spell = json.loads(data['player-source-candidates/' + path + '/spell.json'])
                self.assertIn('none', spell['spell']['requirements']['vocations'])

    def test_rehashed_source_mutation_refused_by_independent_review_pin(self):
        for revision in self.reviewed:
            config = importer.CONFIGS[revision]
            with TemporaryDirectory(dir=importer.ROOT) as tmp:
                source = Path(tmp) / 'packet'
                shutil.copytree(importer.ROOT / config['source'], source)
                path = source / 'lane-audit.json'
                document = json.loads(path.read_bytes())
                rows = importer.audit_rows(document)
                rows[0]['runtime_activation'] = True
                path.write_bytes(encoded(document))
                member = source / 'package-manifest.json'
                manifest = json.loads(member.read_bytes())
                manifest['files']['lane-audit.json'] = importer.digest(path.read_bytes())
                member.write_bytes(encoded(manifest))
                with self.assertRaisesRegex(ValueError, 'UNBLOCKING_MANIFEST_REVIEW_PIN_MISMATCH'):
                    importer.prepare_import(importer.ROOT, revision, source)

    def test_source_headers_and_missing_complete_models_cannot_be_promoted(self):
        base, _, _, schemas, validators = importer.read_base(importer.ROOT, importer.BASE_SHA)
        for revision, (data, _) in self.prepared.items():
            config = importer.CONFIGS[revision]
            packet = {name.split('/', 1)[1]: body for name, body in data.items() if name.startswith('player-source-candidates/')}
            audit = importer.audit_rows(json.loads(packet['lane-audit.json']))
            full = next((row for row in audit if row['status'] == importer.FULL), None)
            if full is None:
                continue
            path = importer.packet_folder(full['registration_key'], config)
            changed = dict(packet)
            header = json.loads(changed[path + '/source-header.json'])
            header['spell']['name'] += ' changed'
            changed[path + '/source-header.json'] = encoded(header)
            with self.assertRaisesRegex(ValueError, 'UNBLOCKING_SOURCE_HEADER_CHANGED'):
                importer.verify_lane(importer.ROOT, revision, changed, config, base, schemas, validators)
            changed = dict(packet)
            del changed[path + '/dependencies.json']
            with self.assertRaisesRegex(ValueError, 'UNBLOCKING_COMPLETE_BUNDLE_MISSING'):
                importer.verify_lane(importer.ROOT, revision, changed, config, base, schemas, validators)

    def test_native_alias_cannot_be_promoted_and_reader_acceptance_cannot_be_faked(self):
        base, _, _, schemas, validators = importer.read_base(importer.ROOT, importer.BASE_SHA)
        for revision, (data, _) in self.prepared.items():
            packet = {name.split('/', 1)[1]: body for name, body in data.items() if name.startswith('player-source-candidates/')}
            document = json.loads(packet['lane-audit.json'])
            native = next((row for row in importer.audit_rows(document) if row['status'] == importer.FULL and row.get('native_data_model_complete')), None)
            if native is None:
                continue
            for field, value, expected in [('source_alias_to_existing_native_profile', True, 'UNBLOCKING_NATIVE_ALIAS_OR_PARTIAL_PROMOTION_REFUSED'), ('required_operations_unrepresented', ['cast_guard'], 'UNBLOCKING_NATIVE_ALIAS_OR_PARTIAL_PROMOTION_REFUSED'), ('reader_acceptance_qualified', True, 'UNBLOCKING_NATIVE_READER_ACCEPTANCE_CLAIM_REFUSED')]:
                changed = copy.deepcopy(document)
                row = next(row for row in importer.audit_rows(changed) if row['registration_key'] == native['registration_key'])
                row[field] = value
                altered = dict(packet); altered['lane-audit.json'] = encoded(changed)
                with self.assertRaisesRegex(ValueError, expected):
                    importer.verify_lane(importer.ROOT, revision, altered, importer.CONFIGS[revision], base, schemas, validators)

    def test_destination_counts_activation_rehashed_bytes_copy_and_replay(self):
        for revision, (data, manifest) in self.prepared.items():
            with TemporaryDirectory() as tmp:
                root = Path(tmp); destination = root / 'imports/spells' / ('r' + str(revision))
                with self.assertRaisesRegex(ValueError, 'IMPORT_DESTINATION_REFUSED'):
                    importer.write_import(root, revision, root / 'imports/spells/r28', data, manifest)
                active = copy.deepcopy(manifest); active['native_execution_qualified'] = True
                with self.assertRaisesRegex(ValueError, 'TARGET_PROJECTION_ACTIVATION_REFUSED'):
                    importer.write_import(root, revision, destination, data, active)
                false_counts = copy.deepcopy(manifest); false_counts['counts']['full_player_candidates'] += 1
                with self.assertRaisesRegex(ValueError, 'UNBLOCKING_OUTPUT_COUNTS_MISMATCH'):
                    importer.write_import(root, revision, destination, data, false_counts)
                altered = dict(data); member = 'player-source-candidates/lane-audit.json'; altered[member] = b'changed'
                rehashed = copy.deepcopy(manifest); entry = next(row for row in rehashed['artifacts'] if row['path'] == member)
                entry.update(sha256=importer.digest(altered[member]), bytes=len(altered[member]))
                with self.assertRaisesRegex(ValueError, 'UNBLOCKING_OUTPUT_MEMBER_PIN_MISMATCH'):
                    importer.write_import(root, revision, destination, altered, rehashed)
                self.assertFalse(destination.exists())
                old = root / 'imports/spells/r28/import-manifest.json'; old.parent.mkdir(parents=True); old.write_bytes(b'immutable')
                result = importer.write_import(root, revision, destination, data, manifest)
                self.assertFalse(result['runtime_activation']); self.assertEqual(old.read_bytes(), b'immutable')
                for name, body in data.items(): self.assertEqual((destination / name).read_bytes(), body)
                for line in (destination / 'SHA256SUMS').read_text().splitlines():
                    expected, name = line.split('  ', 1); self.assertEqual(importer.digest((destination / name).read_bytes()), expected)
                with self.assertRaisesRegex(ValueError, 'IMPORT_SET_ALREADY_EXISTS'):
                    importer.write_import(root, revision, destination, data, manifest)


if __name__ == '__main__':
    unittest.main()
