"""Archive reviewed private MonsterSlot2 DATA; contracts and consumers remain pending."""
import argparse
import gzip
import json
from pathlib import Path

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

from import_source_spell_package import ROOT, bundle_member_path, digest, require
from source_spell_import_guards import read_base, write_source_only_set

BASE_SHA = '1c8ed40b00c5457ad408cbfd918147e0e1c197cd5877f7f8e8f18305e9cb7b5f'
INPUT_PINS = {
    'imports/spells/r51/source-programs/source-monster-slot-semantics.json.gz': 'ff5256a1d359cfb7744be4fdadb8ece0edc2c906fd8d35a57d21f39cb5df50e8',
    'imports/spells/r54/target-projections/projected-monster-slot-candidates.json.gz': '08e7f3852bf54e1b9b0084a907d0bd949f6b7a92c785eb5902fd509cdd422187',
}
FAMILY = 'private_monster_slot_v2'
STATUS = 'SOURCE_SCHEMA_VALID'
PRODUCER_ARTIFACTS = {63: 'monster-inline-complete.json.gz', 64: 'monster-world-controllers.json.gz',
                      65: 'monster-combat-controllers.json', 66: 'monster-staged-controllers.json.gz'}
SCHEMA_ROLES = {
    'monster_slot': 'urn:oteryn:monster-slot:source-complete:candidate:2',
    'receipt': 'urn:oteryn:monster-slot-receipt:source-complete:candidate:2',
}
FALSE_FLAGS = ('runtime_activation', 'native_execution_qualified', 'native_identity_allocation',
               'canonical_selection_changed', 'input_provider_equivalence', 'source_consumer_implemented')
# Exact independently reviewed source packets and global private schemas.
CONFIGS = {revision: {'source': f'docs/reference/spells/r{revision}-monster-closure',
                     'producer_artifact': PRODUCER_ARTIFACTS[revision],
                     'records': records,
                     'counts': {'records': records, 'source_schema_valid': records,
                                'full_source_data_slots': records}}
           for revision, records in ((63, 10), (64, 21), (65, 51), (66, 83))}
REVIEWED_PACKET_PINS = {63: {'manifest_sha': '1ae02ba671b74e7ee13e41f3f154c39fe72af2a35b7d9e8a9f5cc84d99c53c80',
      'proof_sha': '7818a7efde09d2aa7d0db957fc91b1692fd3ea5b43902d72808a33c3c6809eec',
      'slot_keys_sha256': '05dd510bc965ef6ea7269910966dd29cbdfa90beb450d863cfc4dbf308077dfe'},
 64: {'manifest_sha': '23f78ad4439a182878bb20e0fab20b34b8ac734bb97de1cc8a8b9d1f1eb7c0c2',
      'proof_sha': '22dcb1a4859fb66d422a1ddaccac58f8bee615ef55c911c466206b6e394fc2d7',
      'slot_keys_sha256': '9b8d8b47fcd5193bc050095ebf5438b45f3dbef8a998c320abc2da012508e607'},
 65: {'manifest_sha': '7560463aabebaee0860442161c2219b90ac8042cb80bc56138ef658ad7d45037',
      'proof_sha': 'a8218b85aaf34b6f9b57bd4876fe8e99ba8849fa08d689d6b8846a37a1919940',
      'slot_keys_sha256': '65b142a7d2329c9f14919ddafcc323e9388227a872932807b72d86eb50a8e348'},
 66: {'manifest_sha': '5c3e21576dacaf2ad6078753c3fd896b35c22e3abb5917d27a1aab361e178784',
      'proof_sha': 'f5a25ac5b4f47e4f528f82bfc606981634ac710fefd2695be0209e4fe4d79fca',
      'slot_keys_sha256': 'bbef892f5147aea05d1b1088c4f0fd8905cbff16d1ee33dd344fce500007834b'}}
for revision, reviewed_pins in REVIEWED_PACKET_PINS.items():
    CONFIGS[revision].update(reviewed_pins)
SHARED_SCHEMA_PINS = {'urn:oteryn:monster-combat-controllers:1': '3b4c629b6f9aabeb21cfdc481cb91f4013a5e3991a898df59b01c789508268f3',
 'urn:oteryn:monster-inline-controller:1': 'ccaaa61c23318a3091d369219a89b5d962dfd6527b0d3e0d5b000f412524fd6d',
 'urn:oteryn:monster-slot-receipt:source-complete:candidate:2': '5943a7f97c4058f591a46d95a925e3245915a5b53f42c0389fb9eeb6ccaf363a',
 'urn:oteryn:monster-slot:source-complete:candidate:2': '6103cdfea9020beafdc6de5f7a319fe0f4541cd406144fa0eb7004e3fdd32b07',
 'urn:oteryn:monster-staged-controllers:1': '971b4c71d018fe5992d667b699e62054f84cb84193edab8a64f09f302f68235a',
 'urn:oteryn:monster-world-controllers:1': '804a5f473002ed65157dfbf2eab41d63461698ba5e134878549dc3e133daa941'}
REQUIRED_MEMBERS = {'models.json', 'slot-receipts.json', 'lane-audit.json',
                    'import-summary.json', 'projection-proof.json', 'package-manifest.json'}


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()


def slot_key(row):
    return json.dumps(row['slot_identity'], sort_keys=True)


def index(rows, label):
    result = {slot_key(row): row for row in rows}
    require(len(result) == len(rows), label + '_DUPLICATE_IDENTITY')
    return result


def pending_data(value):
    require(all(value[k] is False for k in FALSE_FLAGS)
            and value['authoring_contract_extension_pending'] is True
            and value['target_schema_family'] == FAMILY,
            'MONSTER_COMPLETE_CONTRACT_OR_CONSUMER_CLAIM_REFUSED')


def reviewed_config(revision):
    require(revision in CONFIGS, 'UNSUPPORTED_MONSTER_COMPLETE_FAMILY')
    config = CONFIGS[revision]
    require(all(isinstance(config.get(name), str) and len(config[name]) == 64
                and all(c in '0123456789abcdef' for c in config[name])
                for name in ('manifest_sha', 'proof_sha', 'slot_keys_sha256')),
            'MONSTER_COMPLETE_REVIEW_PENDING')
    require(config.get('producer_artifact') == PRODUCER_ARTIFACTS[revision],
            'MONSTER_COMPLETE_PRODUCER_ARTIFACT_REFUSED')
    require(SHARED_SCHEMA_PINS is not None, 'MONSTER_COMPLETE_GLOBAL_SCHEMA_REVIEW_PENDING')
    return config


def read_packet(source, config):
    body = bundle_member_path(source, 'package-manifest.json').read_bytes()
    require(digest(body) == config['manifest_sha'], 'MONSTER_COMPLETE_MANIFEST_REVIEW_PIN_MISMATCH')
    names = json.loads(body)['files']
    require(set(names) | {'package-manifest.json'} == {
        path.relative_to(source).as_posix() for path in source.rglob('*') if path.is_file()},
        'MONSTER_COMPLETE_PACKET_MEMBERSHIP_MISMATCH')
    require(REQUIRED_MEMBERS | {config['producer_artifact']} <= set(names) | {'package-manifest.json'},
            'MONSTER_COMPLETE_REQUIRED_MEMBER_MISSING')
    packet = {'package-manifest.json': body}
    for name, expected in names.items():
        require(name.endswith('.json') or name == 'README.md' or name == config['producer_artifact'],
                'MONSTER_COMPLETE_ORIGINAL_ASSET_REFUSED')
        packet[name] = bundle_member_path(source, name).read_bytes()
        require(digest(packet[name]) == expected, 'MONSTER_COMPLETE_MEMBER_PIN_MISMATCH:' + name)
        if name == config['producer_artifact']:
            require(name in PRODUCER_ARTIFACTS.values(), 'MONSTER_COMPLETE_PRODUCER_ARTIFACT_REFUSED')
            producer_body = gzip.decompress(packet[name]) if name.endswith('.gz') else packet[name]
            require(isinstance(json.loads(producer_body), dict),
                    'MONSTER_COMPLETE_PRODUCER_JSON_OBJECT_REQUIRED')
    require(digest(packet['projection-proof.json']) == config['proof_sha'], 'MONSTER_COMPLETE_PROOF_REVIEW_PIN_MISMATCH')
    return packet


def schema_validators(packet, proof, base_schemas, baseline):
    require(SHARED_SCHEMA_PINS is not None, 'MONSTER_COMPLETE_GLOBAL_SCHEMA_REVIEW_PENDING')
    resources = proof['schema_resources']
    declared = {row['uri']: row for row in resources}
    require(len(declared) == len(resources)
            and {uri: row['sha256'] for uri, row in declared.items()} == SHARED_SCHEMA_PINS,
            'MONSTER_COMPLETE_GLOBAL_SCHEMA_PINS_MISMATCH')
    documents = {ref['uri']: json.loads(base_schemas[ref['path']]) for ref in baseline['schemaRefs']}
    paths = set()
    for uri, row in declared.items():
        name = row['path']
        require(name.startswith('source-schema-snapshot/') and name.endswith('.json')
                and '..' not in Path(name).parts and name not in paths,
                'MONSTER_COMPLETE_SCHEMA_PATH_REFUSED')
        paths.add(name)
        body = packet[name]
        require(digest(body) == row['sha256'], 'MONSTER_COMPLETE_SCHEMA_BYTES_PIN_MISMATCH')
        schema = json.loads(body)
        require(schema['$id'] == uri, 'MONSTER_COMPLETE_SCHEMA_ID_MISMATCH')
        Draft202012Validator.check_schema(schema)
        require(uri not in documents or documents[uri] == schema, 'MONSTER_COMPLETE_BASE_SCHEMA_MUTATION_REFUSED')
        documents[uri] = schema
    require(proof['schema_roles'] == SCHEMA_ROLES
            and all(uri in documents for uri in SCHEMA_ROLES.values()), 'MONSTER_COMPLETE_SCHEMA_ROLES_MISMATCH')
    registry = Registry().with_resources((uri, Resource.from_contents(schema)) for uri, schema in documents.items())
    return {role: Draft202012Validator(documents[uri], registry=registry) for role, uri in SCHEMA_ROLES.items()}


def original_slots(root, proof):
    require(INPUT_PINS.items() <= proof['input_proofs'].items(), 'MONSTER_COMPLETE_FROZEN_INPUT_PINS_MISMATCH')
    for name, expected in proof['input_proofs'].items():
        require(name.startswith(('imports/spells/', 'docs/reference/spells/'))
                and not Path(name).is_absolute() and '..' not in Path(name).parts,
                'MONSTER_COMPLETE_INPUT_PATH_REFUSED')
        require(digest(bundle_member_path(root, name).read_bytes()) == expected,
                'MONSTER_COMPLETE_INPUT_PIN_MISMATCH:' + name)
    values = [json.loads(gzip.decompress(bundle_member_path(root, path).read_bytes()))['slots'] for path in INPUT_PINS]
    first, second = (index(rows, 'MONSTER_COMPLETE_ORIGINAL') for rows in values)
    require(len(first) == 175 and set(first) == set(second), 'MONSTER_COMPLETE_ORIGINAL_POPULATION_MISMATCH')
    for key, row in first.items():
        other = second[key]
        require(row['original_slot_sha256'] == other['original_slot_sha256']
                and row['source'] == other['source'] and row['monster'] == other['monster']
                and canonical(row['source_parameters']) == canonical(other['source_parameters']),
                'MONSTER_COMPLETE_ORIGINAL_SOURCE_DISAGREEMENT')
    return first


def verify_models(packet, config, originals, validators):
    model_body = json.loads(packet['models.json'])
    require(model_body['schema'] == 'OTERYN_PRIVATE_MONSTER_SLOT_MODELS/v2', 'MONSTER_COMPLETE_MODEL_WRAPPER_MISMATCH')
    models = index(model_body['records'], 'MONSTER_COMPLETE_MODELS')
    receipts = index(json.loads(packet['slot-receipts.json'])['records'], 'MONSTER_COMPLETE_RECEIPTS')
    audit = index(json.loads(packet['lane-audit.json'])['records'], 'MONSTER_COMPLETE_AUDIT')
    summary = json.loads(packet['import-summary.json'])
    pending_data(summary)
    records_index = index(summary['records_index'], 'MONSTER_COMPLETE_INDEX')
    keys = sorted(models)
    require(set(models) == set(receipts) == set(audit) == set(records_index)
            and len(keys) == summary['records'] == config['records']
            and digest(canonical(keys)) == config['slot_keys_sha256'], 'MONSTER_COMPLETE_LANE_POPULATION_MISMATCH')
    for key, model in models.items():
        require(key in originals, 'MONSTER_COMPLETE_UNKNOWN_SOURCE_SLOT')
        old = originals[key]
        validators['monster_slot'].validate(model)
        receipt = receipts[key]; validators['receipt'].validate(receipt)
        for row in (model, receipt, audit[key]):
            pending_data(row)
            require(row['status'] == STATUS and row['full_slot_projection_complete'] is True
                    and row['original_slot_sha256'] == old['original_slot_sha256'],
                    'MONSTER_COMPLETE_SOURCE_STATUS_OR_HASH_MISMATCH')
        require(records_index[key]['status'] == STATUS and model['source'] == old['source']
                and model['monster'] == old['monster']
                and canonical(model['source_parameters']) == canonical(old['source_parameters']),
                'MONSTER_COMPLETE_SOURCE_PARAMETERS_CHANGED')
        require(model['source_alias_to_existing_native_profile'] is False
                and model['required_operations_unrepresented'] == [], 'MONSTER_COMPLETE_ALIAS_OR_MISSING_OPERATION_REFUSED')
        require(receipt['target_model_sha256'] == digest(canonical(model)), 'MONSTER_COMPLETE_MODEL_RECEIPT_HASH_MISMATCH')
    counts = {'records': len(keys), 'source_schema_valid': len(keys), 'full_source_data_slots': len(keys)}
    require(counts == config['counts'] and summary['status_counts'] == {STATUS: len(keys)}, 'MONSTER_COMPLETE_COUNTS_MISMATCH')
    return counts, keys


def prepare_import(root, revision, source=None):
    config = reviewed_config(revision)
    source = root / config['source'] if source is None else source
    require(source.resolve().is_relative_to(root.resolve()), 'SOURCE_OUTSIDE_REPOSITORY')
    _, base_body, baseline, schemas, _ = read_base(root, BASE_SHA)
    packet = read_packet(source, config)
    proof = json.loads(packet['projection-proof.json']); pending_data(proof)
    pins = {row['source']: row['revision'] for row in baseline['source_pins']['monster_donors']}
    require(proof['source_pins'] == pins, 'MONSTER_COMPLETE_SOURCE_PINS_MISMATCH')
    validators = schema_validators(packet, proof, schemas, baseline)
    counts, keys = verify_models(packet, config, original_slots(root, proof), validators)
    source_path = source.resolve().relative_to(root.resolve()).as_posix()
    data = {**schemas, 'base-r28/import-manifest.json': base_body,
            **{'monster-source-candidates/' + name: body for name, body in packet.items()}}
    def refs(name):
        if name == 'monster-source-candidates/models.json':
            return [SCHEMA_ROLES['monster_slot']]
        if name == 'monster-source-candidates/slot-receipts.json':
            return [SCHEMA_ROLES['receipt']]
        return []
    manifest = {'schema': 'OTERYN_PRIVATE_MONSTER_SLOT_DATA_IMPORT/v2', 'revision': revision,
                'admission_status': 'source_only_not_active', 'target_schema_family': FAMILY,
                'authoring_contract_extension_pending': True, **{key: False for key in FALSE_FLAGS},
                'base': {'path': 'imports/spells/r28/import-manifest.json', 'sha256': BASE_SHA,
                         'snapshot': 'base-r28/import-manifest.json'},
                'source_pins': baseline['source_pins'], 'source_slot_keys': keys, 'counts': counts,
                'status_counts': {STATUS: len(keys)}, 'qualification_proof_sha256': config['proof_sha'],
                'schema_roles': SCHEMA_ROLES, 'source_metadata_path': 'monster-source-candidates/lane-audit.json',
                'schemaRefs': baseline['schemaRefs'] + [{'uri': row['uri'], 'path': 'monster-source-candidates/' + row['path'],
                                                       'sha256': row['sha256']} for row in proof['schema_resources']],
                'artifacts': [{'path': name, 'sha256': digest(body), 'bytes': len(body), 'schemaRefs': refs(name),
                    **({'schema_application': {'container': '/records', 'mode': 'array_items'}} if refs(name) else {}),
                    'sourcePath': source_path + '/' + name.split('/', 1)[1] if name.startswith('monster-source-candidates/')
                    else 'imports/spells/r28/import-manifest.json' if name.startswith('base-r28/') else 'imports/spells/r28/' + name,
                    'role': 'reviewed_private_monster_slot_data' if name.startswith('monster-source-candidates/')
                    else 'archived_base_schema' if name.startswith('schemas/') else 'base_manifest'} for name, body in sorted(data.items())],
                'limits': ['Private MonsterSlot2 source DATA; accepted v1 schemas remain unchanged.',
                           'Contract extensions and source consumers remain pending; no native/runtime admission.',
                           'Exact original monster slot identities and typed source parameters are preserved.']}
    return data, manifest


def write_import(root, revision, destination, data, manifest):
    config = reviewed_config(revision); pending_data(manifest)
    require(manifest['revision'] == revision and manifest['counts'] == config['counts']
            and digest(canonical(manifest['source_slot_keys'])) == config['slot_keys_sha256'],
            'MONSTER_COMPLETE_OUTPUT_FAMILY_OR_COHORT_MISMATCH')
    require(manifest['base'] == {'path': 'imports/spells/r28/import-manifest.json', 'sha256': BASE_SHA,
                                'snapshot': 'base-r28/import-manifest.json'}
            and digest(data['base-r28/import-manifest.json']) == BASE_SHA,
            'MONSTER_COMPLETE_OUTPUT_BASE_PIN_MISMATCH')
    baseline = json.loads(data['base-r28/import-manifest.json'])
    require(manifest['source_pins'] == baseline['source_pins']
            and manifest['status_counts'] == {STATUS: config['records']},
            'MONSTER_COMPLETE_OUTPUT_SOURCE_METADATA_MISMATCH')
    for ref in baseline['schemaRefs']:
        require(digest(data[ref['path']]) == ref['sha256'], 'MONSTER_COMPLETE_OUTPUT_BASE_SCHEMA_PIN_MISMATCH')
    prefix = 'monster-source-candidates/'
    require(digest(data[prefix + 'package-manifest.json']) == config['manifest_sha']
            and digest(data[prefix + 'projection-proof.json']) == config['proof_sha'] == manifest['qualification_proof_sha256'],
            'MONSTER_COMPLETE_OUTPUT_REVIEW_PIN_MISMATCH')
    proof = json.loads(data[prefix + 'projection-proof.json'])
    require(manifest['schema_roles'] == SCHEMA_ROLES and manifest['schemaRefs'] == baseline['schemaRefs'] + [
        {'uri': row['uri'], 'path': prefix + row['path'], 'sha256': row['sha256']} for row in proof['schema_resources']],
        'MONSTER_COMPLETE_OUTPUT_SCHEMA_METADATA_MISMATCH')
    package_files = json.loads(data[prefix + 'package-manifest.json'])['files']
    expected_members = {'base-r28/import-manifest.json'} | {ref['path'] for ref in baseline['schemaRefs']} | {
        prefix + name for name in set(package_files) | {'package-manifest.json'}}
    require(set(data) == expected_members, 'MONSTER_COMPLETE_OUTPUT_REVIEWED_MEMBERSHIP_MISMATCH')
    for name, expected in package_files.items():
        require(digest(data[prefix + name]) == expected, 'MONSTER_COMPLETE_OUTPUT_MEMBER_PIN_MISMATCH')
    for artifact in manifest['artifacts']:
        name = artifact['path']
        role = {'models.json': 'monster_slot', 'slot-receipts.json': 'receipt'}.get(Path(name).name)
        expected_refs = [SCHEMA_ROLES[role]] if role and name.startswith(prefix) else []
        expected_source = config['source'] + '/' + name[len(prefix):] if name.startswith(prefix) else (
            'imports/spells/r28/import-manifest.json' if name == 'base-r28/import-manifest.json'
            else 'imports/spells/r28/' + name)
        expected_role = 'reviewed_private_monster_slot_data' if name.startswith(prefix) else (
            'base_manifest' if name == 'base-r28/import-manifest.json' else 'archived_base_schema')
        require(artifact['sourcePath'] == expected_source and artifact['role'] == expected_role,
                'MONSTER_COMPLETE_OUTPUT_SOURCE_PROVENANCE_MISMATCH')
        require(artifact['schemaRefs'] == expected_refs
                and artifact.get('schema_application') == ({'container': '/records', 'mode': 'array_items'} if expected_refs else None),
                'MONSTER_COMPLETE_OUTPUT_ROW_SCHEMA_APPLICATION_MISMATCH')
    return write_source_only_set(root, destination, f'imports/spells/r{revision}', data, manifest)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--revision', type=int, choices=sorted(CONFIGS), required=True)
    parser.add_argument('--source', type=Path); parser.add_argument('--prepare-only', action='store_true')
    args = parser.parse_args(); data, manifest = prepare_import(ROOT, args.revision, args.source)
    print(json.dumps({'artifacts': len(data), 'counts': manifest['counts']} if args.prepare_only
                     else write_import(ROOT, args.revision, ROOT / f'imports/spells/r{args.revision}', data, manifest), sort_keys=True))
