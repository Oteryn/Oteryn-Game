import copy
import gzip
import json
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch

import import_source_complete_monster_batches as importer


FLAGS = {'target_schema_family': importer.FAMILY, 'authoring_contract_extension_pending': True,
         **{name: False for name in importer.FALSE_FLAGS}}


def closed(properties):
    return {'type': 'object', 'additionalProperties': False,
            'required': list(properties), 'properties': properties}


class MonsterCompleteImportTests(unittest.TestCase):
    def packet(self, root):
        source = root / 'docs/reference/spells/r63-monster-closure'
        source.mkdir(parents=True)
        originals = [{'source': 'canary', 'monster': 'Fixture',
                      'slot_identity': {'candidate_id': f'canary/fixture-{i}', 'group': 'attacks', 'source_slot_index': 0},
                      'original_slot_sha256': f'{i:064x}', 'source_parameters': {'target': False, 'amount': 0, 'effect': None}}
                     for i in range(175)]
        inputs = {}
        for path in importer.INPUT_PINS:
            body = gzip.compress(importer.canonical({'slots': originals}), mtime=0)
            target = root / path; target.parent.mkdir(parents=True); target.write_bytes(body)
            inputs[path] = importer.digest(body)
        model = {**copy.deepcopy(originals[0]), **FLAGS, 'status': importer.STATUS,
                 'full_slot_projection_complete': True, 'source_alias_to_existing_native_profile': False,
                 'required_operations_unrepresented': [], 'source_proofs': [], 'target_schedule': None,
                 'controller': {'kind': 'fixture', 'parameters': {'operation': 'source_fact'}}}
        receipt = {key: copy.deepcopy(model[key]) for key in
                   ['slot_identity', 'original_slot_sha256', 'status', 'full_slot_projection_complete', *FLAGS]}
        receipt['target_model_sha256'] = importer.digest(importer.canonical(model))
        identity = closed({'candidate_id': {'type': 'string'}, 'group': {'const': 'attacks'},
                           'source_slot_index': {'type': 'integer'}})
        properties = {key: {} for key in model}
        properties['slot_identity'] = identity
        properties['controller'] = closed({'kind': {'const': 'fixture'},
                                           'parameters': closed({'operation': {'const': 'source_fact'}})})
        model_schema = {'$schema': 'https://json-schema.org/draft/2020-12/schema',
                        '$id': importer.SCHEMA_ROLES['monster_slot'], **closed(properties)}
        receipt_schema = {'$schema': 'https://json-schema.org/draft/2020-12/schema',
                          '$id': importer.SCHEMA_ROLES['receipt'], **closed({key: {} for key in receipt})}
        schemas = {'source-schema-snapshot/monster.schema.json': model_schema,
                   'source-schema-snapshot/receipt.schema.json': receipt_schema}
        resources = [{'path': name, 'uri': value['$id'],
                      'sha256': importer.digest(importer.canonical(value))} for name, value in schemas.items()]
        proof = {**FLAGS, 'schema_roles': importer.SCHEMA_ROLES, 'schema_resources': resources,
                 'source_pins': {'canary': 'a' * 40, 'crystal': 'b' * 40}, 'input_proofs': inputs}
        summary = {**FLAGS, 'records': 1, 'records_index': [{'slot_identity': model['slot_identity'], 'status': importer.STATUS}],
                   'status_counts': {importer.STATUS: 1}}
        files = {'models.json': {'schema': 'OTERYN_PRIVATE_MONSTER_SLOT_MODELS/v2', 'records': [model]},
                 'slot-receipts.json': {'records': [receipt]}, 'lane-audit.json': {'records': [copy.deepcopy(receipt)]},
                 'projection-proof.json': proof, 'import-summary.json': summary, **schemas}
        for name, value in files.items():
            target = source / name; target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(importer.canonical(value))
        producer_artifact = importer.PRODUCER_ARTIFACTS[63]
        (source / producer_artifact).write_bytes(gzip.compress(importer.canonical({'records': [model]}), mtime=0))
        files[producer_artifact] = None
        manifest = {'files': {name: importer.digest((source / name).read_bytes()) for name in files}}
        (source / 'package-manifest.json').write_bytes(importer.canonical(manifest))
        config = {'source': source.relative_to(root).as_posix(), 'records': 1,
                  'producer_artifact': producer_artifact,
                  'manifest_sha': importer.digest((source / 'package-manifest.json').read_bytes()),
                  'proof_sha': importer.digest((source / 'projection-proof.json').read_bytes()),
                  'slot_keys_sha256': importer.digest(importer.canonical([importer.slot_key(model)])),
                  'counts': {'records': 1, 'source_schema_valid': 1, 'full_source_data_slots': 1}}
        base_schema = {'$schema': 'https://json-schema.org/draft/2020-12/schema', '$id': 'urn:test:base', 'type': 'object'}
        body = importer.canonical(base_schema)
        baseline = {'schemaRefs': [{'uri': base_schema['$id'], 'path': 'schemas/base.schema.json', 'sha256': importer.digest(body)}],
                    'source_pins': {'monster_donors': [{'source': name, 'revision': revision} for name, revision in proof['source_pins'].items()]}}
        baseline_body = importer.canonical(baseline)
        return source, config, inputs, resources, (root, baseline_body, baseline, {'schemas/base.schema.json': body}, {})

    def setup(self, root):
        source, config, inputs, resources, baseline = self.packet(root)
        patches = [patch.dict(importer.CONFIGS, {63: config}), patch.object(importer, 'INPUT_PINS', inputs),
                   patch.object(importer, 'SHARED_SCHEMA_PINS', {r['uri']: r['sha256'] for r in resources}),
                   patch.object(importer, 'read_base', return_value=baseline),
                   patch.object(importer, 'BASE_SHA', importer.digest(baseline[1]))]
        for item in patches: item.start(); self.addCleanup(item.stop)
        return source, config

    def test_review_pins_are_required_before_read_or_write(self):
        with patch.dict(importer.CONFIGS, {63: {'manifest_sha': None}}):
            with self.assertRaisesRegex(ValueError, 'MONSTER_COMPLETE_REVIEW_PENDING'):
                importer.prepare_import(Path('/missing'), 63)
        for key in importer.FALSE_FLAGS:
            altered = dict(FLAGS, **{key: True})
            with self.subTest(key=key), self.assertRaisesRegex(ValueError, 'CONTRACT_OR_CONSUMER'):
                importer.pending_data(altered)
        with self.assertRaisesRegex(ValueError, 'CONTRACT_OR_CONSUMER'):
            importer.pending_data(dict(FLAGS, authoring_contract_extension_pending=False))

    def test_exact_typed_parameters_global_schemas_and_closed_controller(self):
        with TemporaryDirectory() as tmp:
            root = Path(tmp); source, config = self.setup(root)
            data, manifest = importer.prepare_import(root, 63)
            self.assertEqual(config['counts'], manifest['counts'])
            self.assertTrue(manifest['authoring_contract_extension_pending'])
            self.assertFalse(manifest['source_consumer_implemented'])
            for artifact in manifest['artifacts']:
                if artifact['schemaRefs']:
                    self.assertEqual({'container': '/records', 'mode': 'array_items'}, artifact['schema_application'])
            packet = importer.read_packet(source, config)
            proof = json.loads(packet['projection-proof.json'])
            base = importer.read_base(root, importer.BASE_SHA)
            validators = importer.schema_validators(packet, proof, base[3], base[2])
            model = json.loads(packet['models.json'])['records'][0]
            changed = copy.deepcopy(model); changed['controller']['raw_lua'] = 'opaque'
            with self.assertRaises(Exception): validators['monster_slot'].validate(changed)
            changed_proof = copy.deepcopy(proof); changed_proof['schema_roles']['monster_slot'] = 'urn:test:base'
            with self.assertRaisesRegex(ValueError, 'SCHEMA_ROLES'):
                importer.schema_validators(packet, changed_proof, base[3], base[2])
            changed_packet = dict(packet); changed_packet[proof['schema_resources'][0]['path']] = b'{}'
            with self.assertRaisesRegex(ValueError, 'SCHEMA_BYTES_PIN'):
                importer.schema_validators(changed_packet, proof, base[3], base[2])

    def test_source_false_to_zero_missing_cohort_and_duplicate_identity_refused(self):
        with TemporaryDirectory() as tmp:
            root = Path(tmp); source, config = self.setup(root)
            packet = importer.read_packet(source, config); proof = json.loads(packet['projection-proof.json'])
            base = importer.read_base(root, importer.BASE_SHA)
            validators = importer.schema_validators(packet, proof, base[3], base[2])
            originals = importer.original_slots(root, proof)
            value = json.loads(packet['models.json'])
            for mutation in ['parameter', 'duplicate', 'alias', 'omitted_operation']:
                changed = copy.deepcopy(value)
                if mutation == 'parameter': changed['records'][0]['source_parameters']['target'] = 0
                if mutation == 'duplicate': changed['records'].append(changed['records'][0])
                if mutation == 'alias': changed['records'][0]['source_alias_to_existing_native_profile'] = True
                if mutation == 'omitted_operation': changed['records'][0]['required_operations_unrepresented'] = ['missing']
                altered = dict(packet, **{'models.json': importer.canonical(changed)})
                with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                    importer.verify_models(altered, config, originals, validators)

    def test_rehashed_packet_and_frozen_input_mutation_refused(self):
        with TemporaryDirectory() as tmp:
            root = Path(tmp); source, config = self.setup(root)
            extra = source / 'unreviewed.json'; extra.write_text('{}')
            with self.assertRaisesRegex(ValueError, 'PACKET_MEMBERSHIP'):
                importer.prepare_import(root, 63)
            extra.unlink()
            model = source / 'models.json'; model.write_text('{}')
            manifest_path = source / 'package-manifest.json'; manifest = json.loads(manifest_path.read_bytes())
            manifest['files']['models.json'] = importer.digest(model.read_bytes())
            manifest_path.write_bytes(importer.canonical(manifest))
            with self.assertRaisesRegex(ValueError, 'MANIFEST_REVIEW_PIN'):
                importer.prepare_import(root, 63)
            proof = {'input_proofs': dict(importer.INPUT_PINS)}
            (root / next(iter(importer.INPUT_PINS))).write_bytes(b'changed')
            with self.assertRaisesRegex(ValueError, 'INPUT_PIN_MISMATCH'):
                importer.original_slots(root, proof)

    def test_exact_named_gzip_evidence_parses_json_and_other_assets_are_refused(self):
        with TemporaryDirectory() as tmp:
            root = Path(tmp); source, config = self.setup(root)
            artifact = source / config['producer_artifact']
            self.assertIsInstance(json.loads(gzip.decompress(importer.read_packet(source, config)[artifact.name])), dict)
            manifest_path = source / 'package-manifest.json'
            original_manifest = json.loads(manifest_path.read_bytes())
            for name, body in [('original-script.lua.gz', gzip.compress(b'original script')),
                               (artifact.name, gzip.compress(b'not JSON'))]:
                manifest = copy.deepcopy(original_manifest)
                (source / name).write_bytes(body); manifest['files'][name] = importer.digest(body)
                manifest_path.write_bytes(importer.canonical(manifest))
                local = dict(config, manifest_sha=importer.digest(manifest_path.read_bytes()))
                with self.subTest(name=name), self.assertRaises((ValueError, json.JSONDecodeError)):
                    importer.read_packet(source, local)
                if name != artifact.name: (source / name).unlink()

    def test_only_exact_new_destination_can_copy_and_replay_is_refused(self):
        with TemporaryDirectory() as tmp:
            root = Path(tmp); _, _ = self.setup(root)
            data, manifest = importer.prepare_import(root, 63)
            destination = root / 'imports/spells/r63'
            importer.write_import(root, 63, destination, data, manifest)
            for name, body in data.items(): self.assertEqual(body, (destination / name).read_bytes())
            self.assertTrue((destination / 'SHA256SUMS').is_file())
            with self.assertRaisesRegex(ValueError, 'IMPORT_SET_ALREADY_EXISTS'):
                importer.write_import(root, 63, destination, data, manifest)
            with self.assertRaisesRegex(ValueError, 'IMPORT_DESTINATION_REFUSED'):
                importer.write_import(root, 63, root / 'content/spells', data, manifest)
            changed = dict(manifest, source_consumer_implemented=True)
            with self.assertRaisesRegex(ValueError, 'CONTRACT_OR_CONSUMER'):
                importer.write_import(root, 63, destination, data, changed)

    def test_output_reviewed_bytes_and_archive_schema_metadata_cannot_drift(self):
        with TemporaryDirectory() as tmp:
            root = Path(tmp); _, _ = self.setup(root)
            data, manifest = importer.prepare_import(root, 63)
            changed = dict(data); changed['monster-source-candidates/models.json'] = b'{}'
            with self.assertRaisesRegex(ValueError, 'OUTPUT_MEMBER_PIN'):
                importer.write_import(root, 63, root / 'imports/spells/r63', changed, manifest)
            altered = copy.deepcopy(manifest); altered['schemaRefs'] = []
            with self.assertRaisesRegex(ValueError, 'OUTPUT_SCHEMA_METADATA'):
                importer.write_import(root, 63, root / 'imports/spells/r63', data, altered)
            altered = copy.deepcopy(manifest)
            next(row for row in altered['artifacts'] if row['schemaRefs']).pop('schema_application')
            with self.assertRaisesRegex(ValueError, 'OUTPUT_ROW_SCHEMA_APPLICATION'):
                importer.write_import(root, 63, root / 'imports/spells/r63', data, altered)
            altered = copy.deepcopy(manifest); altered['source_pins'] = {}
            with self.assertRaisesRegex(ValueError, 'OUTPUT_SOURCE_METADATA'):
                importer.write_import(root, 63, root / 'imports/spells/r63', data, altered)

    def test_rehashed_extra_output_or_source_provenance_is_refused(self):
        with TemporaryDirectory() as tmp:
            root = Path(tmp); self.setup(root)
            data, manifest = importer.prepare_import(root, 63)
            changed = dict(data, **{'monster-source-candidates/unreviewed.json': b'{}'})
            altered = copy.deepcopy(manifest)
            altered['artifacts'].append({'path': 'monster-source-candidates/unreviewed.json',
                'sha256': importer.digest(b'{}'), 'bytes': 2, 'schemaRefs': [],
                'sourcePath': 'unreviewed.json', 'role': 'reviewed_private_monster_slot_data'})
            with self.assertRaisesRegex(ValueError, 'OUTPUT_REVIEWED_MEMBERSHIP'):
                importer.write_import(root, 63, root / 'imports/spells/r63', changed, altered)
            for field in ('sourcePath', 'role'):
                altered = copy.deepcopy(manifest); altered['artifacts'][0][field] = 'unreviewed'
                with self.subTest(field=field), self.assertRaisesRegex(ValueError, 'OUTPUT_SOURCE_PROVENANCE'):
                    importer.write_import(root, 63, root / 'imports/spells/r63', data, altered)
            self.assertFalse((root / 'imports/spells/r63').exists())


if __name__ == '__main__': unittest.main()
