import copy
import gzip
import json
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

import import_target_projection_batches as importer
from import_source_spell_package import encoded


class TargetProjectionImportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.prepared = {revision: importer.prepare_import(importer.ROOT, revision) for revision in importer.CONFIGS}

    def test_exact_full_models_partial_models_and_native_bindings_distinct(self):
        self.assertEqual(self.prepared[52][1]['counts']['player_candidates'], 3)
        self.assertEqual(self.prepared[52][1]['counts']['proposed_native_source_bindings'], 4)
        self.assertEqual(self.prepared[53][1]['counts']['player_candidates'], 6)
        self.assertEqual(self.prepared[54][1]['counts']['full_monster_slot_candidates'], 10)
        self.assertEqual(self.prepared[54][1]['counts']['partial_monster_slots'], 62)
        for revision, (data, manifest) in self.prepared.items():
            for flag in importer.FALSE_FLAGS:
                self.assertIs(manifest[flag], False)
            self.assertEqual(manifest['executable_spell_count'], 0)
            self.assertFalse(manifest['source_numeric_equivalence'])
            for path, body in data.items():
                old = importer.ROOT / 'imports/spells/r28' / path
                if path.startswith('schemas/') and old.exists():
                    self.assertEqual(body, old.read_bytes())
            for entry in manifest['schemaRefs']:
                self.assertEqual(importer.digest(data[entry['path']]), entry['sha256'])

    def test_source_receipt_and_dependency_reference_tamper_rejected(self):
        data, _ = self.prepared[53]
        packet = {name.split('/', 1)[1]: body for name, body in data.items() if name.startswith('target-projections/')}
        base, _, _, _, validators = importer.read_base(importer.ROOT, importer.BASE_SHA)
        captures = {row['registration_key']: row for row in importer.jsonl(base, 'player-source-bundles/source-callback-facts.jsonl.gz')}
        proof = json.loads(packet['projection-qualification.json']); row = proof['records'][0]
        path = importer.folder(row['registration_key'])
        mutated = dict(packet)
        receipt = json.loads(mutated[path + '/receipt.json']); receipt['source_sha256'] = '0' * 64
        mutated[path + '/receipt.json'] = encoded(receipt)
        with self.assertRaisesRegex(ValueError, 'TARGET_SOURCE_RECEIPT_MISMATCH'):
            importer.verify_source_bundle(mutated, path, row['registration_key'], base, captures, validators)
        mutated = dict(packet)
        spell = json.loads(mutated[path + '/spell.json']); spell['spell']['execution']['ability']['key'] += '/missing'
        mutated[path + '/spell.json'] = encoded(spell)
        with self.assertRaisesRegex(ValueError, 'TARGET_DEPENDENCY_CLOSURE_MISMATCH'):
            importer.bundle(mutated, path, validators)

    def test_partial_monster_cannot_be_counted_as_full_or_mutate_source_fields(self):
        data, _ = self.prepared[54]
        config = importer.CONFIGS[54]
        packet = {name.split('/', 1)[1]: body for name, body in data.items() if name.startswith('target-projections/')}
        _, _, _, _, validators = importer.read_base(importer.ROOT, importer.BASE_SHA)
        document = json.loads(gzip.decompress(packet[config['artifact']]))
        for change, expected in [('promotion', 'MONSTER_PARTIAL_COUNTED_FULL'), ('source', 'MONSTER_PROJECTION_SOURCE_OR_SCOPE_MISMATCH'), ('schedule', 'MONSTER_FULL_SLOT_SCHEDULE_MISMATCH')]:
            altered = copy.deepcopy(document)
            if change == 'promotion':
                row = next(row for row in altered['slots'] if row['status'] == 'PARTIAL_TARGET_DEFINITIONS')
                row['full_slot_projection_complete'] = True
            elif change == 'source':
                altered['slots'][0]['source_parameters']['range'] = 999
            else:
                row = next(row for row in altered['slots'] if row['full_slot_projection_complete'])
                row['target_schedule']['chance_percent'] = 99
            payload = encoded(altered)
            mutated = dict(packet); mutated[config['artifact']] = gzip.compress(payload, mtime=0)
            proof = json.loads(packet[config['proof']]); proof['payload_sha256'] = importer.digest(payload)
            with self.assertRaisesRegex(ValueError, expected):
                importer.verify_monster_projection(importer.ROOT, mutated, proof, validators)

    def test_changed_packet_or_proof_refused_before_materialization(self):
        for revision, config in importer.CONFIGS.items():
            with TemporaryDirectory(dir=importer.ROOT) as tmp:
                source = Path(tmp)
                proof = json.loads((importer.ROOT / config['source'] / config['proof']).read_bytes())
                proof['runtime_activation'] = True
                (source / config['proof']).write_bytes(encoded(proof))
                with self.assertRaisesRegex(ValueError, 'TARGET_PROOF_REVIEW_PIN_MISMATCH'):
                    importer.prepare_import(importer.ROOT, revision, source)

    def test_destination_replay_activation_rehashed_members_and_false_counts_refused(self):
        for revision, (data, manifest) in self.prepared.items():
            with TemporaryDirectory() as tmp:
                root = Path(tmp); destination = root / 'imports/spells' / ('r' + str(revision))
                with self.assertRaisesRegex(ValueError, 'IMPORT_DESTINATION_REFUSED'):
                    importer.write_import(root, revision, root / 'imports/spells/r28', data, manifest)
                for flag in importer.FALSE_FLAGS:
                    active = copy.deepcopy(manifest); active[flag] = True
                    with self.assertRaisesRegex(ValueError, 'TARGET_PROJECTION_ACTIVATION_REFUSED'):
                        importer.write_import(root, revision, destination, data, active)
                counts = copy.deepcopy(manifest); counts['counts']['player_candidates'] += 1
                with self.assertRaisesRegex(ValueError, 'TARGET_OUTPUT_COUNTS_MISMATCH'):
                    importer.write_import(root, revision, destination, data, counts)
                mutated = dict(data)
                member = next(name for name in data if name.startswith('target-projections/') and (name.endswith('/spell.json') or name.endswith('.json.gz')))
                mutated[member] = b'changed'
                rehashed = copy.deepcopy(manifest)
                entry = next(row for row in rehashed['artifacts'] if row['path'] == member)
                entry.update(sha256=importer.digest(mutated[member]), bytes=len(mutated[member]))
                with self.assertRaisesRegex(ValueError, 'TARGET_OUTPUT_(MEMBER|PACKET)_PIN_MISMATCH'):
                    importer.write_import(root, revision, destination, mutated, rehashed)
                self.assertFalse(destination.exists())
                old = root / 'imports/spells/r28/import-manifest.json'; old.parent.mkdir(parents=True); old.write_bytes(b'immutable')
                result = importer.write_import(root, revision, destination, data, manifest)
                self.assertFalse(result['runtime_activation']); self.assertEqual(old.read_bytes(), b'immutable')
                for name, body in data.items(): self.assertEqual((destination / name).read_bytes(), body)
                self.assertEqual({p.relative_to(destination).as_posix() for p in destination.rglob('*') if p.is_file()}, set(data) | {'import-manifest.json', 'SHA256SUMS'})
                for line in (destination / 'SHA256SUMS').read_text().splitlines():
                    expected, name = line.split('  ', 1); self.assertEqual(importer.digest((destination / name).read_bytes()), expected)
                with self.assertRaisesRegex(ValueError, 'IMPORT_SET_ALREADY_EXISTS'):
                    importer.write_import(root, revision, destination, data, manifest)


if __name__ == '__main__':
    unittest.main()
