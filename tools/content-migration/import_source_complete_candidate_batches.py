"""Import private source-complete target DATA snapshots without contract admission."""
import argparse
from collections import Counter
import json
from pathlib import Path

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

from import_source_spell_package import ROOT, bundle_member_path, digest, require
from import_source_program_batches import canonical, jsonl, unique_index
from import_candidate_unblocking_batches import audit_rows, verify_policy_proofs
from import_target_projection_batches import folder, inactive
from source_spell_import_guards import read_base, write_source_only_set

BASE_SHA = '1c8ed40b00c5457ad408cbfd918147e0e1c197cd5877f7f8e8f18305e9cb7b5f'
SPELL_URI = 'urn:oteryn:spell-authoring:source-complete:candidate:2'
SCHEMA_ROLES = {'spell': SPELL_URI, 'dependencies': 'urn:oteryn:spell-dependencies:candidate:1',
                'receipt': 'urn:oteryn:source-player-bundle-receipt:1'}
FAMILY = 'private_source_complete_v2'
FULL = 'CANDIDATE_SCHEMA_VALID'
CONFIGS = {59: {'source': 'docs/reference/spells/r59-source-closure',
      'manifest_sha': 'c42f0cb7b8bf9f66bfab322ae664e517ece9d07dff22cc2a5395235b82bc8a2b',
      'proof_sha': '367bf0a1e3f5a8d5ff32b69c3595d09ab328ee89a362d951b1061188cb998343',
      'registration_keys_sha256': 'acbd2df4c32945dd8546b36a885037ef6c22539aa76a4e6802649600223246a7',
      'records': 43,
      'counts': {'audit_records': 43,
                 'full_source_data_candidates': 41,
                 'reference_records': 2,
                 'abilities': 1,
                 'effects': 1,
                 'formulas': 1},
      'status_counts': {'CANDIDATE_SCHEMA_VALID': 41, 'REFERENCE_ONLY_REMOVED_S24': 2},
      'reference_statuses': ['REFERENCE_ONLY_REMOVED_S24']},
 60: {'source': 'docs/reference/spells/r60-source-closure',
      'manifest_sha': '84814861393473896a82ba493dd81e8cde92d978bd053f69aaf53201ad64874d',
      'proof_sha': '759dff9edb5283c087a14257b3d870d16af76e2ca995d5af55a82faa84f95973',
      'registration_keys_sha256': '21ee9935835e45f2853a083895d2401217d21edfd0dd2bbe002978b3e6ed140f',
      'records': 23,
      'counts': {'audit_records': 23,
                 'full_source_data_candidates': 23,
                 'reference_records': 0,
                 'abilities': 46,
                 'effects': 46,
                 'formulas': 46},
      'status_counts': {'CANDIDATE_SCHEMA_VALID': 23},
      'reference_statuses': []},
 61: {'source': 'docs/reference/spells/r61-source-closure',
      'manifest_sha': '1db5d87e6b2684a2e13c3e296ff0967f090746960c5f8e7ad35046dd86d746b0',
      'proof_sha': 'b675a7e70de777a44823778a03ee2f6ae0a61b5a4d000ebd3a53768c62c199b2',
      'registration_keys_sha256': 'eb4cade14e3a2c89a4d6c2ac351784bf05619ee5fa55dc0e6cae7db58b6ba635',
      'records': 26,
      'counts': {'audit_records': 26,
                 'full_source_data_candidates': 26,
                 'reference_records': 0,
                 'abilities': 3,
                 'effects': 3,
                 'formulas': 3},
      'status_counts': {'CANDIDATE_SCHEMA_VALID': 26},
      'reference_statuses': []},
 62: {'source': 'docs/reference/spells/r62-source-closure',
      'manifest_sha': 'b74144556c195a397b982889cdb0e9b6b9ee063fbdd519220f8c831a9526e9e6',
      'proof_sha': '7d4b70814f73ae5e1e012542aaf92c8015151269558baeadd9fee0c1090cc244',
      'registration_keys_sha256': 'f6e8ca0206198e3a98715574e889fb48fbdbd9c2d5e83054c479985b7a1b5af6',
      'records': 37,
      'counts': {'audit_records': 37,
                 'full_source_data_candidates': 31,
                 'reference_records': 6,
                 'abilities': 0,
                 'effects': 0,
                 'formulas': 0},
      'status_counts': {'CANDIDATE_SCHEMA_VALID': 31,
                        'REFERENCE_ONLY_DISABLED': 4,
                        'REFERENCE_ONLY_RETIRED_S24': 2},
      'reference_statuses': ['REFERENCE_ONLY_DISABLED', 'REFERENCE_ONLY_RETIRED_S24']}}
SHARED_SCHEMA_PINS = {'urn:oteryn:monster-authoring:candidate:1': '4b76f97d0beadc2c5e2367e93d35d09b3b35d1ca5ffc179231e90d1a160f64d2',
 'urn:oteryn:monster-dependencies:candidate:1': 'cdd61c0643ae5304ac4105b5af4e32f7c4ba3c80c01c1149d004c3dc507e0a5e',
 'urn:oteryn:monster-import-readiness:candidate:1': '2441939caaf7bddd988d015ad9c7a12a71b07051fe553768c9e505e2e55735ee',
 'urn:oteryn:player-source-projection:1': '64dbf9d7b9e4092ee32eaff0db745a6880f2f16c5c85d6cff628bf7938079f91',
 'urn:oteryn:player-source-registrar-import:1': '680bba3fb921fddb6c1ae78c362c052f78ad1aeacca4c7ccd6da4b45e8a5ad9a',
 'urn:oteryn:source-custom-mechanics:1': 'd84adf271c5b7fdca861070f01b6f7bcfd2d7e7e0af4367023755a4f6a74ae43',
 'urn:oteryn:source-equipment-native-extensions:1': 'b5d345127cf9aa448e8d74c2ee01e674ef0dbc91e50f21d5fa6c88751131d1e3',
 'urn:oteryn:source-formula-evidence:1': 'bd94dbc0a87caff2059bf1bd0051371431d8cce1fd408b7696ffb75c5a69caaf',
 'urn:oteryn:source-monster-profile-facts:1': '7032536e01d0a1bf4a092e1c0c4e4b3dfe478eb9e49984aa10d334964cd40384',
 'urn:oteryn:source-party-summon-native-extensions:1': '6dabaee5a3dc7db946540b04dc78358c3eaeeec7f052cec068f19fd901cdc462',
 'urn:oteryn:source-player-bundle-receipt:1': '39169c01d01be1bf5489322c3f53d09e23ce15b5a1718cb11e7ac839c620b0d6',
 'urn:oteryn:source-state-native-extensions:1': '46e2b8423c586b93922e934cfee917732d1cd31547d679187b324bf89a633ec4',
 'urn:oteryn:source-world-control-native-extensions:1': 'bd5e51b520277f36f195c832820b8e2d8089a0148a1cedcf3f4b604b8bcbe3a7',
 'urn:oteryn:spell-authoring:candidate:1': '703919824010bd0bf585d24c908666d4e04ca242c2b2080537e89e71b62703e0',
 'urn:oteryn:spell-authoring:source-complete:candidate:2': 'd16deb8ec63b7d6c1aa2284018c9d113c1777837bbcbc3ebddcf80c2962f70a1',
 'urn:oteryn:spell-dependencies:candidate:1': '4a73d4634abf657da0c9d5a9df66b52092bd728fa2f6c7ca7845a4d0ff2e111d',
 'urn:oteryn:spell-source-registrar:1': 'dee3c48c41e0091c2b6dbb0d135f7e8db9917f22dbeae86b32f7b5346ce696de'}


def pending_data(value):
    inactive(value)
    require(value['authoring_contract_extension_pending'] is True and value['source_consumer_implemented'] is False
            and value['target_schema_family'] == FAMILY and value['input_provider_equivalence'] is False,
            'SOURCE_COMPLETE_CONTRACT_OR_CONSUMER_CLAIM_REFUSED')


def read_packet(source, config):
    manifest_body = bundle_member_path(source, 'package-manifest.json').read_bytes()
    require(digest(manifest_body) == config['manifest_sha'], 'SOURCE_COMPLETE_MANIFEST_REVIEW_PIN_MISMATCH')
    members = json.loads(manifest_body)['files']
    require(set(members) | {'package-manifest.json'} == {path.relative_to(source).as_posix()
            for path in source.rglob('*') if path.is_file()}, 'SOURCE_COMPLETE_PACKET_MEMBERSHIP_MISMATCH')
    packet = {'package-manifest.json': manifest_body}
    for name, expected in members.items():
        require(name.endswith('.json') or name == 'README.md', 'SOURCE_COMPLETE_ORIGINAL_ASSET_REFUSED')
        body = bundle_member_path(source, name).read_bytes()
        require(digest(body) == expected, 'SOURCE_COMPLETE_MEMBER_PIN_MISMATCH:' + name)
        packet[name] = body
    require(digest(packet['projection-proof.json']) == config['proof_sha'], 'SOURCE_COMPLETE_PROOF_REVIEW_PIN_MISMATCH')
    return packet


def verify_declared_inputs(root, proof):
    for name, expected in proof.get('input_proofs', {}).items():
        require(name.startswith(('imports/spells/', 'docs/reference/spells/'))
                and not Path(name).is_absolute() and '..' not in Path(name).parts,
                'SOURCE_COMPLETE_INPUT_PATH_REFUSED')
        require(digest(bundle_member_path(root, name).read_bytes()) == expected,
                'SOURCE_COMPLETE_INPUT_PIN_MISMATCH:' + name)


def schema_validators(packet, proof, base_schemas, baseline):
    require(SHARED_SCHEMA_PINS is not None, 'SOURCE_COMPLETE_GLOBAL_SCHEMA_REVIEW_PENDING')
    declared = unique_index(proof['schema_resources'], lambda row: row['uri'], 'SOURCE_COMPLETE_SCHEMA_URI_DUPLICATE')
    require({uri: row['sha256'] for uri, row in declared.items()} == SHARED_SCHEMA_PINS, 'SOURCE_COMPLETE_GLOBAL_SCHEMA_PINS_MISMATCH')
    documents = {ref['uri']: json.loads(base_schemas[ref['path']]) for ref in baseline['schemaRefs']}
    snapshots = {}
    for uri, row in declared.items():
        path = row['path']
        require(path.startswith('source-schema-snapshot/') and '..' not in Path(path).parts, 'SOURCE_COMPLETE_SCHEMA_PATH_REFUSED')
        body = packet[path]
        require(digest(body) == row['sha256'], 'SOURCE_COMPLETE_SCHEMA_BYTES_PIN_MISMATCH')
        schema = json.loads(body)
        require(schema['$id'] == uri, 'SOURCE_COMPLETE_SCHEMA_ID_MISMATCH')
        Draft202012Validator.check_schema(schema)
        require(uri not in documents or documents[uri] == schema, 'SOURCE_COMPLETE_EXISTING_SCHEMA_MUTATION_REFUSED')
        documents[uri] = schema
        snapshots[path] = body
    roles = proof['schema_roles']
    require(roles == SCHEMA_ROLES
            and all(uri in documents for uri in roles.values()), 'SOURCE_COMPLETE_SCHEMA_ROLES_MISMATCH')
    registry = Registry().with_resources((uri, Resource.from_contents(schema)) for uri, schema in documents.items())
    return {role: Draft202012Validator(documents[uri], registry=registry) for role, uri in roles.items()}, snapshots


def validate_dependency_closure(spell, dependencies, catalog, receipt):
    require(set(dependencies) == {'abilities', 'effects', 'formulas'}, 'SOURCE_COMPLETE_DEPENDENCY_FAMILIES_MISMATCH')
    local = {(family, row['identity']['key'], row['identity']['revision'])
             for plural, family in [('abilities', 'Ability'), ('effects', 'Effect'), ('formulas', 'Formula')]
             for row in dependencies[plural]}
    require(len(local) == sum(len(rows) for rows in dependencies.values()), 'SOURCE_COMPLETE_DEPENDENCY_DUPLICATE')
    require(set(catalog) == {'definitions'} and all(set(ref) == {'family', 'key', 'revision'}
            and ref['family'] in {'Item', 'Creature'} for ref in catalog['definitions']), 'SOURCE_COMPLETE_EXTERNAL_CATALOG_SCOPE_REFUSED')
    external = {(row['family'], row['key'], row['revision']) for row in catalog['definitions']}
    require(len(external) == len(catalog['definitions']), 'SOURCE_COMPLETE_EXTERNAL_REFERENCE_DUPLICATE')
    references = []
    def visit(value):
        if isinstance(value, dict):
            if set(value) == {'family', 'key', 'revision'}:
                references.append((value['family'], value['key'], value['revision']))
            for child in value.values(): visit(child)
        elif isinstance(value, list):
            for child in value: visit(child)
    visit(spell); visit(dependencies)
    require(set(references) == local | external, 'SOURCE_COMPLETE_DEPENDENCY_CLOSURE_MISMATCH')
    require(receipt['dependencies'] == {key: len(rows) for key, rows in dependencies.items()}, 'SOURCE_COMPLETE_RECEIPT_DEPENDENCY_COUNTS_MISMATCH')
    items = [row for row in catalog['definitions'] if row['family'] == 'Item']
    require(receipt['item_owner_bindings_required'] == items, 'SOURCE_COMPLETE_ITEM_OWNER_BINDINGS_MISMATCH')


def verify_models(packet, proof, summary, audit, base, validators, config):
    captures = unique_index(jsonl(base, 'player-source-bundles/source-callback-facts.jsonl.gz'), lambda row: row['registration_key'], 'SOURCE_COMPLETE_BASE_CAPTURE_DUPLICATE')
    index = unique_index(summary['records_index'], lambda row: row['registration_key'], 'SOURCE_COMPLETE_INDEX_DUPLICATE')
    require(set(audit) == set(index) and digest(canonical(sorted(audit))) == config['registration_keys_sha256']
            and len(audit) == summary['records'] == config['records'], 'SOURCE_COMPLETE_LANE_POPULATION_MISMATCH')
    counts = {'audit_records': len(audit), 'full_source_data_candidates': 0, 'reference_records': 0,
              'abilities': 0, 'effects': 0, 'formulas': 0}
    for reg, row in audit.items():
        require(reg in captures and row['status'] == index[reg]['status'], 'SOURCE_COMPLETE_UNKNOWN_REGISTRATION_OR_STATUS')
        raw = captures[reg]
        path = folder(reg)
        original = base / 'player-source-bundles' / path
        header_body = (original / 'source-header.json').read_bytes()
        require(packet[path + '/source-header.json'] == header_body, 'SOURCE_COMPLETE_SOURCE_HEADER_CHANGED')
        old_receipt = json.loads((original / 'receipt.json').read_bytes())
        require(old_receipt['status'] == 'BLOCKED' and row.get('source_sha256', raw['source_sha256']) == raw['source_sha256']
                and row.get('source_revision', raw['source_revision']) == raw['source_revision'], 'SOURCE_COMPLETE_SOURCE_IDENTITY_MISMATCH')
        if row['status'] != FULL:
            require(row['status'] in config['reference_statuses'], 'SOURCE_COMPLETE_UNQUALIFIED_STATUS_REFUSED')
            require(not any(path + '/' + name in packet for name in ['spell.json', 'dependencies.json', 'catalog.json']), 'SOURCE_COMPLETE_REFERENCE_HAS_TARGET_MODEL')
            counts['reference_records'] += 1
            continue
        projection = json.loads(packet[path + '/projection-receipt.json'])
        pending_data(projection)
        require(projection['required_operations_unrepresented'] == []
                and projection['source_alias_to_existing_native_profile'] is False,
                'SOURCE_COMPLETE_ALIAS_OR_MISSING_OPERATION_REFUSED')
        receipt = json.loads(packet[path + '/receipt.json'])
        validators['receipt'].validate(receipt)
        require(receipt['registration_key'] == reg and receipt['status'] == FULL and not receipt['blockers']
                and receipt['source_revision'] == raw['source_revision'] and receipt['source_sha256'] == raw['source_sha256']
                and receipt['source_header'] == json.loads(header_body)['spell']
                and receipt['runtime_activation'] is False and receipt['native_execution_qualified'] is False,
                'SOURCE_COMPLETE_RECEIPT_SOURCE_OR_STATUS_MISMATCH')
        spell, dependencies, catalog = (json.loads(packet[path + '/' + name]) for name in ['spell.json', 'dependencies.json', 'catalog.json'])
        validators['spell'].validate(spell); validators['dependencies'].validate(dependencies)
        require(spell['spell']['identity'] == {'key': index[reg]['candidate_key'], 'revision': index[reg]['candidate_revision']}
                and receipt['candidate_key'] == index[reg]['candidate_key'], 'SOURCE_COMPLETE_TARGET_IDENTITY_MISMATCH')
        validate_dependency_closure(spell, dependencies, catalog, receipt)
        counts['full_source_data_candidates'] += 1
        for plural in ['abilities', 'effects', 'formulas']:
            counts[plural] += len(dependencies[plural])
    require(counts == config['counts'] and dict(Counter(row['status'] for row in index.values())) == summary['status_counts'] == config['status_counts'], 'SOURCE_COMPLETE_COUNTS_MISMATCH')
    return counts, sorted(audit)


def prepare_import(root, revision, source=None):
    require(revision in CONFIGS, 'UNSUPPORTED_SOURCE_COMPLETE_FAMILY')
    config = CONFIGS[revision]
    require(all(key in config for key in ['manifest_sha', 'proof_sha', 'registration_keys_sha256', 'records', 'counts', 'status_counts', 'reference_statuses']), 'SOURCE_COMPLETE_REVIEW_PENDING')
    source = root / config['source'] if source is None else source
    require(source.resolve().is_relative_to(root.resolve()), 'SOURCE_OUTSIDE_REPOSITORY')
    base, base_body, baseline, base_schemas, _ = read_base(root, BASE_SHA)
    packet = read_packet(source, config)
    proof = json.loads(packet['projection-proof.json']); summary = json.loads(packet['import-summary.json'])
    pending_data(proof); pending_data(summary); verify_policy_proofs(root, proof); verify_declared_inputs(root, proof)
    validators, snapshots = schema_validators(packet, proof, base_schemas, baseline)
    audit = unique_index(audit_rows(json.loads(packet['lane-audit.json'])), lambda row: row['registration_key'], 'SOURCE_COMPLETE_AUDIT_DUPLICATE')
    counts, keys = verify_models(packet, proof, summary, audit, base, validators, config)
    source_path = source.resolve().relative_to(root.resolve()).as_posix()
    data = {**base_schemas, 'base-r28/import-manifest.json': base_body, **{'player-source-candidates/' + name: body for name, body in packet.items()}}
    def refs(name):
        role = {'spell.json': 'spell', 'dependencies.json': 'dependencies', 'receipt.json': 'receipt'}.get(Path(name).name)
        return [proof['schema_roles'][role]] if role else []
    manifest = {'schema': 'OTERYN_PRIVATE_SOURCE_COMPLETE_DATA_IMPORT/v2', 'revision': revision,
                'admission_status': 'source_only_not_active', 'target_schema_family': FAMILY,
                'authoring_contract_extension_pending': True, 'source_consumer_implemented': False,
                'runtime_activation': False, 'native_execution_qualified': False, 'native_identity_allocation': False,
                'canonical_selection_changed': False, 'input_provider_equivalence': False,
                'base': {'path': 'imports/spells/r28/import-manifest.json', 'sha256': BASE_SHA, 'snapshot': 'base-r28/import-manifest.json'},
                'source_pins': baseline['source_pins'], 'source_keys': keys, 'counts': counts, 'status_counts': summary['status_counts'],
                'qualification_proof_sha256': config['proof_sha'], 'schema_roles': proof['schema_roles'],
                'source_metadata_path': 'player-source-candidates/lane-audit.json',
                'schemaRefs': baseline['schemaRefs'] + [{'uri': json.loads(body)['$id'], 'path': 'player-source-candidates/' + name, 'sha256': digest(body)} for name, body in snapshots.items()],
                'artifacts': [{'path': name, 'sha256': digest(body), 'bytes': len(body), 'schemaRefs': refs(name),
                               'sourcePath': source_path + '/' + name.split('/', 1)[1] if name.startswith('player-source-candidates/') else 'imports/spells/r28/import-manifest.json' if name.startswith('base-r28/') else 'imports/spells/r28/' + name,
                               'role': 'reviewed_private_source_complete_data' if name.startswith('player-source-candidates/') else 'archived_base_schema' if name.startswith('schemas/') else 'base_manifest'} for name, body in sorted(data.items())],
                'limits': ['Models validate against the private source-complete v2 schema family; accepted v1 is unchanged.',
                           'Contract extensions and source consumers remain pending; no runtime/native admission is claimed.',
                           'Reference-only source registrations have no target model and are counted separately.']}
    return data, manifest


def write_import(root, revision, destination, data, manifest):
    require(revision in CONFIGS and manifest['revision'] == revision, 'WRONG_SOURCE_COMPLETE_FAMILY')
    config = CONFIGS[revision]; pending_data(manifest)
    require(manifest['base'] == {'path': 'imports/spells/r28/import-manifest.json', 'sha256': BASE_SHA,
                                'snapshot': 'base-r28/import-manifest.json'}
            and digest(data['base-r28/import-manifest.json']) == BASE_SHA, 'SOURCE_COMPLETE_OUTPUT_BASE_PIN_MISMATCH')
    baseline = json.loads(data['base-r28/import-manifest.json'])
    for ref in baseline['schemaRefs']:
        require(digest(data[ref['path']]) == ref['sha256'], 'SOURCE_COMPLETE_OUTPUT_BASE_SCHEMA_PIN_MISMATCH')
    require(manifest['counts'] == config['counts'] and manifest['status_counts'] == config['status_counts']
            and digest(canonical(manifest['source_keys'])) == config['registration_keys_sha256'], 'SOURCE_COMPLETE_OUTPUT_COUNTS_OR_KEYS_MISMATCH')
    body = data['player-source-candidates/package-manifest.json']
    require(digest(body) == config['manifest_sha'] and digest(data['player-source-candidates/projection-proof.json']) == config['proof_sha'] == manifest['qualification_proof_sha256'], 'SOURCE_COMPLETE_OUTPUT_REVIEW_PIN_MISMATCH')
    proof = json.loads(data['player-source-candidates/projection-proof.json'])
    expected_schemas = baseline['schemaRefs'] + [{'uri': row['uri'], 'path': 'player-source-candidates/' + row['path'],
                                               'sha256': row['sha256']} for row in proof['schema_resources']]
    require(manifest['schema_roles'] == SCHEMA_ROLES and manifest['schemaRefs'] == expected_schemas,
            'SOURCE_COMPLETE_OUTPUT_SCHEMA_METADATA_MISMATCH')
    for name, expected in json.loads(body)['files'].items():
        require(digest(data['player-source-candidates/' + name]) == expected, 'SOURCE_COMPLETE_OUTPUT_MEMBER_PIN_MISMATCH')
    return write_source_only_set(root, destination, 'imports/spells/r' + str(revision), data, manifest)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__); parser.add_argument('--revision', type=int, choices=sorted(CONFIGS), required=True)
    parser.add_argument('--source', type=Path); parser.add_argument('--prepare-only', action='store_true'); args = parser.parse_args()
    data, manifest = prepare_import(ROOT, args.revision, args.source)
    print(json.dumps({'artifacts': len(data), 'counts': manifest['counts']} if args.prepare_only else write_import(ROOT, args.revision, ROOT / 'imports/spells' / ('r' + str(args.revision)), data, manifest), sort_keys=True))
