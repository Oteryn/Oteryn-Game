"""Archive reviewed lane data; promote only complete ordinary candidate bundles."""
import argparse
from collections import Counter
import json
import gzip
from jsonschema import Draft202012Validator
from pathlib import Path

from import_source_spell_package import ROOT, bundle_member_path, digest, require
from import_source_program_batches import canonical, jsonl, unique_index
from import_target_projection_batches import folder, inactive
from source_spell_import_guards import read_base, write_source_only_set

BASE_SHA = '1c8ed40b00c5457ad408cbfd918147e0e1c197cd5877f7f8e8f18305e9cb7b5f'
CONFIGS = {revision: {'source': 'docs/reference/spells/r' + str(revision) + '-source-closure'} for revision in range(55, 59)}
CONFIGS[55].update({'manifest_sha': 'eeb4a4ac50cc69d1e5b5a476b53051a9c33d16c9d83fcdfcf728a6577440c80a', 'proof_sha': '4b32b2616310cb7614b40027b1f4a4350aea7b921344b58b0a746501f329ac2c', 'registration_keys_sha256': '6f409a3d4cf1a58e9b285c338a30dd08cb7912a419ae63e5139adb7f73b93a3a', 'records': 64, 'status_counts': {'CANDIDATE_SCHEMA_VALID': 21}, 'audit_status_counts': {'BLOCKED': 43, 'CANDIDATE_SCHEMA_VALID': 21}, 'counts': {'lane_records': 64, 'full_player_candidates': 21, 'native_full_player_candidates': 0, 'ordinary_full_player_candidates': 21, 'noncandidate_records': 43, 'partial_target_bundles': 0, 'proposed_native_bindings': 0, 'runtime_capability_blocked_candidates': 0, 'embedded_partial_target_records': 0}, 'optional_members': ['README.md']})

CONFIGS[56].update({'manifest_sha': 'd15e25e126e3e8ed1cc92b784b8c9bb882b79e20bf9fbebbd9294e465e4b8038', 'proof_sha': 'be00be53ab7e3ca6ed85762ee7d27d5ea0556ccf4ec98c7e691dc9ce9b86af57', 'registration_keys_sha256': '340bac999c48a3ff3b113ba4302523259e660659cf693c0aae6ef68aa4e2d26f', 'records': 25, 'status_counts': {'SOURCE_SCHEMA_VALID_RUNTIME_CAPABILITY_BLOCKED': 14, 'CANDIDATE_SCHEMA_VALID': 2, 'BLOCKED': 9}, 'audit_status_counts': {'SOURCE_SCHEMA_VALID_RUNTIME_CAPABILITY_BLOCKED': 14, 'CANDIDATE_SCHEMA_VALID': 2, 'BLOCKED': 9}, 'counts': {'lane_records': 25, 'full_player_candidates': 2, 'native_full_player_candidates': 2, 'ordinary_full_player_candidates': 0, 'noncandidate_records': 23, 'partial_target_bundles': 0, 'proposed_native_bindings': 16, 'runtime_capability_blocked_candidates': 2}, 'optional_members': ['partial-data.json']})
CONFIGS[57].update({'manifest_sha': '178bf8d7ab9cdb6ca0f256de5951eb61e4b2e2e3fc3bf629e682681cbb0ce515', 'proof_sha': '5c2c69a3ddc9859a2a6ede4787133b412311e9751b19f83f1767cae9f1651e6d', 'registration_keys_sha256': 'df897899f5f2278b06294e2f6ef53e48cd931153f5e6760163344b7e63f255a7', 'records': 28, 'status_counts': {'PARTIAL_NATIVE_SOURCE_TARGET': 4, 'BLOCKED': 12, 'PARTIAL_SOURCE_CONDITION_TARGET': 10, 'CANDIDATE_SCHEMA_VALID': 2}, 'audit_status_counts': {'PARTIAL_NATIVE_SOURCE_TARGET': 4, 'BLOCKED': 12, 'PARTIAL_SOURCE_CONDITION_TARGET': 10, 'CANDIDATE_SCHEMA_VALID': 2}, 'counts': {'lane_records': 28, 'full_player_candidates': 2, 'native_full_player_candidates': 2, 'ordinary_full_player_candidates': 0, 'noncandidate_records': 26, 'partial_target_bundles': 0, 'proposed_native_bindings': 0, 'runtime_capability_blocked_candidates': 2}, 'optional_members': ['README.md', 'partial-data.json'], 'custom_receipt': True, 'snapshot_layout': True, 'candidate_revision': 'source-player-r57'})
CONFIGS[58].update({'manifest_sha': 'fa72fb80f215eb4ba64d5e9c8fae42a9398a44d562bb788c70713c3573ff1744', 'proof_sha': 'ea4f2696322bcaab9d5f34c525dad0bcd19f18d1a134866dfa5a77134d321649', 'registration_keys_sha256': 'e356e5d7f9495084bc6c5a490b8af76224f5d3a725be5472d69ce0b7f44a3aa3', 'records': 45, 'status_counts': {'BLOCKED': 37, 'CANDIDATE_SCHEMA_VALID': 8}, 'audit_status_counts': {'BLOCKED': 37, 'CANDIDATE_SCHEMA_VALID': 8}, 'counts': {'lane_records': 45, 'full_player_candidates': 8, 'native_full_player_candidates': 0, 'ordinary_full_player_candidates': 8, 'noncandidate_records': 37, 'partial_target_bundles': 30, 'proposed_native_bindings': 24, 'runtime_capability_blocked_candidates': 0}, 'optional_members': ['README.md', 'canary-main-current/03110e87b566f92b/projection-receipt.json', 'canary-main-current/0ee9ef6e76a0d777/projection-receipt.json', 'canary-main-current/1a5dfeebfafbe0af/projection-receipt.json', 'canary-main-current/216c7f9ac8b3ae34/projection-receipt.json', 'canary-main-current/434de4a616823d0b/projection-receipt.json', 'canary-main-current/51e9ed27e2d4e666/projection-receipt.json', 'canary-main-current/54671ca22df640e1/projection-receipt.json', 'canary-main-current/55afdd1ea782fc7d/projection-receipt.json', 'canary-main-current/6dda208b996fb158/projection-receipt.json', 'canary-main-current/7a22a8b9160d8bad/projection-receipt.json', 'canary-main-current/7ca4a48949df9d0e/projection-receipt.json', 'canary-main-current/8ce382e80f73ead9/projection-receipt.json', 'canary-main-current/8fcf7d22d07f1781/projection-receipt.json', 'canary-main-current/97a5b8022eb1088c/projection-receipt.json', 'canary-main-current/9d301405d680dd37/projection-receipt.json', 'canary-main-current/9ff66fc4fb26632d/projection-receipt.json', 'canary-main-current/ae0c8418eb75a91f/projection-receipt.json', 'canary-main-current/e4cdd23ffba80f0f/projection-receipt.json', 'canary-main-current/e71e2133854cc1da/projection-receipt.json', 'canary-main-current/fea9b96a42d3f4d5/projection-receipt.json', 'crystal-summer-current/023fd91bb85e21d5/projection-receipt.json', 'crystal-summer-current/096d6afc567baee4/projection-receipt.json', 'crystal-summer-current/1d4b4b41b98eb602/projection-receipt.json', 'crystal-summer-current/1f2ccd5edb07dae5/projection-receipt.json', 'crystal-summer-current/2343e39acd2be1f9/projection-receipt.json', 'crystal-summer-current/396e76a9f56c77a2/projection-receipt.json', 'crystal-summer-current/4ecce57ff285c332/projection-receipt.json', 'crystal-summer-current/5273b9a067bdce97/projection-receipt.json', 'crystal-summer-current/5f953878bac1d4f4/projection-receipt.json', 'crystal-summer-current/8c643011ff5bd3a9/projection-receipt.json', 'crystal-summer-current/9764103b1ae8e3f2/projection-receipt.json', 'crystal-summer-current/e2405397903bf9e3/projection-receipt.json', 'crystal-summer-current/e8c460db8b2efe91/projection-receipt.json', 'crystal-summer-current/eaabb7654b80c2c4/projection-receipt.json', 'crystal-summer-current/eb47c14425100b6f/projection-receipt.json', 'crystal-summer-current/ec94944f69d37008/projection-receipt.json', 'crystal-summer-current/efbcae87a75ddf46/projection-receipt.json', 'crystal-summer-current/fd34675deb38d6b7/projection-receipt.json', 'partial-data.json']})

for _revision, _embedded_count in [(55, 0), (56, 14), (57, 14), (58, 30)]:
    CONFIGS[_revision]['counts']['embedded_partial_target_records'] = _embedded_count

CONFIGS[58]['counts']['runtime_capability_blocked_candidates'] = 2

BUNDLE_NAMES = {'spell.json', 'dependencies.json', 'catalog.json', 'receipt.json', 'source-header.json'}
FULL = 'CANDIDATE_SCHEMA_VALID'


def packet_folder(reg, config):
    return 'snapshot/' + digest(reg.encode())[:16] if config.get('snapshot_layout') else folder(reg)


def validate_source_bundle(packet, path, reg, base, captures, validators, config):
    old = base / 'player-source-bundles' / folder(reg)
    receipt = json.loads(packet[path + '/receipt.json'])
    if config.get('custom_receipt'):
        Draft202012Validator(json.loads(packet['receipt.schema.json'])).validate(receipt)
    else:
        validators['schemas/player-bundle-receipt.schema.json'].validate(receipt)
    require(receipt['registration_key'] == reg and receipt['source_sha256'] == captures[reg]['source_sha256']
            and receipt['source_revision'] == captures[reg]['source_revision']
            and (receipt.get('source_header') == json.loads((old / 'source-header.json').read_bytes())['spell']
                 if not config.get('custom_receipt') else receipt['source_header_sha256'] == digest((old / 'source-header.json').read_bytes()))
            and receipt['status'] == FULL and not receipt.get('blockers', [])
            and receipt['runtime_activation'] is False and receipt['native_execution_qualified'] is False,
            'UNBLOCKING_CANDIDATE_SOURCE_RECEIPT_MISMATCH')
    spell, deps, catalog = (json.loads(packet[path + '/' + name]) for name in ['spell.json', 'dependencies.json', 'catalog.json'])
    validators['schemas/spell.schema.json'].validate(spell)
    validators['schemas/spell-dependencies.schema.json'].validate(deps)
    require(set(catalog) == {'definitions'} and all(set(ref) == {'family', 'key', 'revision'} and ref['family'] == 'Item' for ref in catalog['definitions']), 'UNBLOCKING_EXTERNAL_CATALOG_SCOPE_REFUSED')
    externals = {(ref['family'], ref['key'], ref['revision']) for ref in catalog['definitions']}
    require(receipt.get('item_owner_bindings_required', []) == catalog['definitions'], 'UNBLOCKING_ITEM_OWNER_BINDINGS_MISMATCH')
    local = {(family, row['identity']['key'], row['identity']['revision']) for plural, family in [('abilities', 'Ability'), ('effects', 'Effect'), ('formulas', 'Formula')] for row in deps[plural]}
    references = []
    def visit(value):
        if isinstance(value, dict):
            if set(value) == {'family', 'key', 'revision'}: references.append((value['family'], value['key'], value['revision']))
            for child in value.values(): visit(child)
        elif isinstance(value, list):
            for child in value: visit(child)
    visit(spell); visit(deps)
    require(set(references) == local | externals and len(local) == sum(len(rows) for rows in deps.values()), 'UNBLOCKING_DEPENDENCY_CLOSURE_MISMATCH')
    require(receipt.get('candidate_key', spell['spell']['identity']['key']) == spell['spell']['identity']['key']
            and receipt.get('dependencies', {k: len(v) for k, v in deps.items()}) == {k: len(v) for k, v in deps.items()}, 'UNBLOCKING_CANDIDATE_RECEIPT_COUNTS_MISMATCH')
    return spell, deps


def read_packet(source, config):
    manifest_body = bundle_member_path(source, 'package-manifest.json').read_bytes()
    require(digest(manifest_body) == config['manifest_sha'], 'UNBLOCKING_MANIFEST_REVIEW_PIN_MISMATCH')
    manifest = json.loads(manifest_body)
    packet = {name: bundle_member_path(source, name).read_bytes() for name in manifest['files']}
    for name, body in packet.items():
        require(name.endswith('.json') or name in {'README.md', 'partial-data.json.gz'}, 'UNBLOCKING_ORIGINAL_ASSET_REFUSED')
        require(digest(body) == manifest['files'][name], 'UNBLOCKING_MEMBER_PIN_MISMATCH:' + name)
    packet['package-manifest.json'] = manifest_body
    require(digest(packet['projection-proof.json']) == config['proof_sha'], 'UNBLOCKING_PROOF_REVIEW_PIN_MISMATCH')
    require({'import-summary.json', 'projection-proof.json', 'receipt.schema.json', 'lane-audit.json'} <= set(packet), 'UNBLOCKING_REQUIRED_MEMBER_MISSING')
    return packet


def audit_rows(value):
    return value if isinstance(value, list) else value.get('rows', value.get('records'))


def verify_policy_proofs(root, proof):
    require('policy_proofs' in proof, 'UNBLOCKING_POLICY_PROOFS_MISSING')
    policies = proof['policy_proofs']
    rows = [{'path': path, 'sha256': sha} for path, sha in policies.items()] if isinstance(policies, dict) else policies
    for row in rows:
        path = row['path']
        require(path.startswith(('docs/architecture/', 'docs/contracts/')) and '..' not in Path(path).parts, 'UNBLOCKING_POLICY_PATH_REFUSED')
        body = bundle_member_path(root, path).read_bytes()
        require(digest(body) == row['sha256'], 'UNBLOCKING_POLICY_PIN_MISMATCH')
        if 'exact_rows' in row:
            require(row['exact_rows'] and all(text in body.decode() for text in row['exact_rows']), 'UNBLOCKING_POLICY_ROW_MISMATCH')


def verify_lane(root, revision, packet, config, base, schemas, validators):
    summary = json.loads(packet['import-summary.json'])
    proof = json.loads(packet['projection-proof.json'])
    inactive(proof)
    for key in ['runtime_activation', 'native_execution_qualified', 'native_identity_allocation', 'canonical_selection_changed']:
        require(summary.get(key, proof[key]) is False, 'UNBLOCKING_SUMMARY_ACTIVATION_REFUSED')
    require(proof['input_provider_equivalence'] is False, 'UNBLOCKING_PROVIDER_EQUIVALENCE_REFUSED')
    verify_policy_proofs(root, proof)
    if not config.get('custom_receipt'):
        require(packet['receipt.schema.json'] == schemas['schemas/player-bundle-receipt.schema.json'], 'UNBLOCKING_RECEIPT_SCHEMA_CHANGED')
    audit_value = json.loads(packet['lane-audit.json'])
    audit = unique_index(audit_rows(audit_value), lambda row: row['registration_key'], 'UNBLOCKING_AUDIT_DUPLICATE')
    index = unique_index(summary['records_index'], lambda row: row['registration_key'], 'UNBLOCKING_INDEX_DUPLICATE')
    full_keys = {reg for reg, row in audit.items() if row['status'] == FULL}
    require(digest(canonical(sorted(audit))) == config['registration_keys_sha256'] and full_keys <= set(index) <= set(audit)
            and len(audit) == summary.get('audited_records', summary['records']) == config['records'], 'UNBLOCKING_LANE_POPULATION_MISMATCH')
    captures = unique_index(jsonl(base, 'player-source-bundles/source-callback-facts.jsonl.gz'), lambda row: row['registration_key'], 'UNBLOCKING_BASE_CAPTURE_DUPLICATE')
    native_profiles_body = (root / 'tools/content-schema/spell-authoring/samples/native-spell-profiles.json').read_bytes()
    profiles = json.loads(native_profiles_body)['profiles']
    require(len(profiles) == 67, 'UNBLOCKING_NATIVE_GUARD_CHANGED')
    statuses = Counter()
    counts = {'lane_records': len(audit), 'full_player_candidates': 0, 'native_full_player_candidates': 0,
              'ordinary_full_player_candidates': 0, 'noncandidate_records': 0,
              'partial_target_bundles': 0, 'proposed_native_bindings': 0, 'runtime_capability_blocked_candidates': 0, 'embedded_partial_target_records': 0}
    for reg, row in audit.items():
        require(reg in captures and row.get('source_sha256', captures[reg]['source_sha256']) == captures[reg]['source_sha256']
                and row.get('source_revision', captures[reg]['source_revision']) == captures[reg]['source_revision'], 'UNBLOCKING_AUDIT_SOURCE_IDENTITY_MISMATCH')
        require(row.get('runtime_activation', audit_value.get('runtime_activation') if isinstance(audit_value, dict) else None) is False
                and row.get('native_execution_qualified', audit_value.get('native_execution_qualified') if isinstance(audit_value, dict) else None) is False, 'UNBLOCKING_AUDIT_EXECUTION_REFUSED')
        old = base / 'player-source-bundles' / folder(reg)
        require(json.loads((old / 'receipt.json').read_bytes())['status'] == 'BLOCKED', 'UNBLOCKING_BASE_NOT_BLOCKED')
        path = packet_folder(reg, config)
        if path + '/source-header.json' in packet:
            require(packet[path + '/source-header.json'] == (old / 'source-header.json').read_bytes(), 'UNBLOCKING_SOURCE_HEADER_CHANGED')
        if reg in index:
            require(index[reg]['status'] == row['status'], 'UNBLOCKING_AUDIT_INDEX_STATUS_MISMATCH')
        statuses[row['status']] += 1
        if row['status'] != FULL:
            counts['noncandidate_records'] += 1
            if row.get('disposition', row['status']) == 'PROPOSED_NATIVE_BINDING':
                counts['proposed_native_bindings'] += 1
            if path + '/spell.json' in packet:
                counts['partial_target_bundles'] += 1
                require(row.get('full_target_projection_complete') is False or row.get('complete_spell_candidate') is False, 'UNBLOCKING_NONCANDIDATE_FULL_CLAIM_REFUSED')
            continue
        require(all(path + '/' + name in packet for name in BUNDLE_NAMES), 'UNBLOCKING_COMPLETE_BUNDLE_MISSING')
        spell, deps = validate_source_bundle(packet, path, reg, base, captures, validators, config)
        require(spell['spell']['identity'] == {'key': index[reg].get('candidate_key', 'candidate:spell/source/' + folder(reg)), 'revision': index[reg].get('candidate_revision', summary.get('candidate_revision', summary.get('revision', config.get('candidate_revision'))))}, 'UNBLOCKING_TARGET_IDENTITY_MISMATCH')
        if 'native_behavior' in spell['spell']['execution']:
            # A complete source-backed descriptor is data even when the current
            # closed native reader refuses its identity. An alias is not a model.
            require(row.get('native_data_model_complete') is True
                    and row.get('source_alias_to_existing_native_profile') is False
                    and row.get('required_operations_unrepresented') == [], 'UNBLOCKING_NATIVE_ALIAS_OR_PARTIAL_PROMOTION_REFUSED')
            matching = [profile for profile in profiles if profile['spell'] == spell['spell'] and profile['dependencies'] == deps]
            if matching:
                require(len(matching) == 1 and row.get('native_source_binding_policy_qualified') is True
                        and row.get('header_policy_qualification') == 'ESTABLISHED'
                        and proof['native_profile_file_sha256'] == digest(native_profiles_body), 'UNBLOCKING_NATIVE_PROFILE_BINDING_POLICY_MISSING')
            else:
                require(row.get('reader_acceptance_qualified') is False
                        and row.get('runtime_capability_blocked') is True, 'UNBLOCKING_NATIVE_READER_ACCEPTANCE_CLAIM_REFUSED')
            counts['native_full_player_candidates'] += 1
        else:
            counts['ordinary_full_player_candidates'] += 1
        counts['full_player_candidates'] += 1
        if row.get('runtime_capability_blocked') is True or row.get('reader_capability_status') == 'SOURCE_SCHEMA_VALID_RUNTIME_CAPABILITY_BLOCKED':
            counts['runtime_capability_blocked_candidates'] += 1
    if 'partial-data.json' in packet:
        partial = json.loads(packet['partial-data.json'])
        if isinstance(partial, dict) and isinstance(partial.get('partial_targets'), list):
            counts['embedded_partial_target_records'] = len(partial['partial_targets'])
        elif isinstance(partial, dict) and isinstance(partial.get('partial_condition_targets'), int):
            counts['embedded_partial_target_records'] = partial['partial_condition_targets'] + sum(row['status'] == 'PARTIAL_NATIVE_SOURCE_TARGET' for row in audit.values())
        elif isinstance(partial, dict) and isinstance(partial.get('records'), list):
            counts['embedded_partial_target_records'] = len(partial['records'])
        bindings = partial.get('proposed_native_bindings') if isinstance(partial, dict) else None
        if isinstance(bindings, list):
            counts['proposed_native_bindings'] = len(bindings)
    index_counts = dict(Counter(row['status'] for row in index.values()))
    require(index_counts == summary.get('status_counts', index_counts) == config['status_counts']
            and dict(statuses) == config['audit_status_counts'] and counts == config['counts'], 'UNBLOCKING_COUNTS_MISMATCH')
    standard = {packet_folder(reg, config) + '/' + name for reg in audit for name in BUNDLE_NAMES if packet_folder(reg, config) + '/' + name in packet}
    headers = {packet_folder(reg, config) + '/source-header.json' for reg in audit if packet_folder(reg, config) + '/source-header.json' in packet}
    metadata = {'package-manifest.json', 'import-summary.json', 'projection-proof.json', 'receipt.schema.json', 'lane-audit.json'}
    require(set(packet) == metadata | headers | standard | set(config.get('optional_members', [])), 'UNBLOCKING_PACKET_MEMBERSHIP_MISMATCH')
    for partial_name in ['partial-data.json', 'partial-data.json.gz']:
        if partial_name not in packet:
            continue
        def visit(value):
            if isinstance(value, dict):
                for key in ['runtime_activation', 'native_execution_qualified', 'native_identity_allocation', 'canonical_selection_changed', 'complete_spell_candidate']:
                    require(value.get(key) is not True, 'UNBLOCKING_PARTIAL_ACTIVATION_OR_FULL_CLAIM_REFUSED')
                for child in value.values(): visit(child)
            elif isinstance(value, list):
                for child in value: visit(child)
        body = packet[partial_name]
        visit(json.loads(gzip.decompress(body) if partial_name.endswith('.gz') else body))
    return summary, proof, counts, sorted(audit)


def prepare_import(root, revision, source=None):
    require(revision in CONFIGS, 'UNSUPPORTED_UNBLOCKING_FAMILY')
    config = CONFIGS[revision]
    require(all(key in config for key in ['manifest_sha', 'proof_sha', 'registration_keys_sha256', 'records', 'status_counts', 'audit_status_counts', 'counts']), 'UNBLOCKING_REVIEW_PENDING')
    source = root / config['source'] if source is None else source
    require(source.resolve().is_relative_to(root.resolve()), 'SOURCE_OUTSIDE_REPOSITORY')
    base, base_body, baseline, schemas, validators = read_base(root, BASE_SHA)
    packet = read_packet(source, config)
    summary, proof, counts, source_keys = verify_lane(root, revision, packet, config, base, schemas, validators)
    if config.get('custom_receipt'):
        schemas['schemas/unblocking-r' + str(revision) + '-receipt.schema.json'] = packet['receipt.schema.json']
    source_path = source.resolve().relative_to(root.resolve()).as_posix()
    lane_headers = {'source-lane-headers/' + folder(reg) + '/source-header.json': (base / 'player-source-bundles' / folder(reg) / 'source-header.json').read_bytes() for reg in source_keys}
    data = {**schemas, **lane_headers, 'base-r28/import-manifest.json': base_body,
            **{'player-source-candidates/' + name: body for name, body in packet.items()}}
    def refs(name):
        schema_name = {'spell.json': 'spell.schema.json', 'dependencies.json': 'spell-dependencies.schema.json', 'receipt.json': 'player-bundle-receipt.schema.json'}.get(Path(name).name)
        if Path(name).name == 'receipt.json' and config.get('custom_receipt'):
            schema_name = 'unblocking-r' + str(revision) + '-receipt.schema.json'
        return [json.loads(schemas['schemas/' + schema_name])['$id']] if schema_name else []
    def origin(name):
        if config.get('custom_receipt') and name == 'schemas/unblocking-r' + str(revision) + '-receipt.schema.json': return source_path + '/receipt.schema.json'
        if name.startswith('source-lane-headers/'): return 'imports/spells/r28/player-source-bundles/' + name.split('/', 1)[1]
        if name.startswith('player-source-candidates/'): return source_path + '/' + name.split('/', 1)[1]
        return 'imports/spells/r28/import-manifest.json' if name.startswith('base-r28/') else 'imports/spells/r28/' + name
    manifest = {'schema': 'OTERYN_REVIEWED_CANDIDATE_UNBLOCKING_IMPORT/v1', 'revision': revision,
                'admission_status': 'source_only_not_active', 'runtime_activation': False, 'native_execution_qualified': False,
                'native_identity_allocation': False, 'canonical_selection_changed': False, 'input_provider_equivalence': False,
                'base': {'path': 'imports/spells/r28/import-manifest.json', 'sha256': BASE_SHA, 'snapshot': 'base-r28/import-manifest.json'},
                'source_pins': baseline['source_pins'], 'counts': counts, 'status_counts': config['status_counts'],
                'source_keys': source_keys, 'audit_status_counts': config['audit_status_counts'], 'qualification_proof_sha256': config['proof_sha'],
                'source_metadata_path': 'player-source-candidates/lane-audit.json', 'schemaRefs': baseline['schemaRefs'] + ([{'uri': json.loads(packet['receipt.schema.json'])['$id'], 'path': 'schemas/unblocking-r' + str(revision) + '-receipt.schema.json', 'sha256': digest(packet['receipt.schema.json'])}] if config.get('custom_receipt') else []),
                'artifacts': [{'path': name, 'sourcePath': origin(name), 'sha256': digest(body), 'bytes': len(body), 'schemaRefs': refs(name),
                               'role': 'reviewed_candidate_or_lane_data' if name.startswith('player-source-candidates/') else 'reference_schema' if name.startswith('schemas/') else 'base_manifest'} for name, body in sorted(data.items())],
                'limits': ['Only complete independently reviewed target models receive candidate status.',
                           'Proposed native bindings, typed partial models and blocked rows retain separate statuses.',
                           'Native guard equality does not itself qualify donor metadata policy, input providers or runtime admission.']}
    return data, manifest


def write_import(root, revision, destination, data, manifest):
    require(revision in CONFIGS and manifest['revision'] == revision, 'WRONG_UNBLOCKING_FAMILY')
    config = CONFIGS[revision]; inactive(manifest)
    require(manifest['counts'] == config['counts'] and manifest['status_counts'] == config['status_counts'], 'UNBLOCKING_OUTPUT_COUNTS_MISMATCH')
    require(digest(canonical(manifest['source_keys'])) == config['registration_keys_sha256'], 'UNBLOCKING_OUTPUT_SOURCE_POPULATION_MISMATCH')
    body = data['player-source-candidates/package-manifest.json']
    require(digest(body) == config['manifest_sha'], 'UNBLOCKING_OUTPUT_PACKAGE_PIN_MISMATCH')
    require(digest(data['player-source-candidates/projection-proof.json']) == config['proof_sha'] == manifest['qualification_proof_sha256'], 'UNBLOCKING_OUTPUT_PROOF_PIN_MISMATCH')
    for name, expected in json.loads(body)['files'].items():
        require(digest(data['player-source-candidates/' + name]) == expected, 'UNBLOCKING_OUTPUT_MEMBER_PIN_MISMATCH')
    return write_source_only_set(root, destination, 'imports/spells/r' + str(revision), data, manifest)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__); parser.add_argument('--revision', type=int, choices=sorted(CONFIGS), required=True)
    parser.add_argument('--source', type=Path); parser.add_argument('--prepare-only', action='store_true'); args = parser.parse_args()
    data, manifest = prepare_import(ROOT, args.revision, args.source)
    print(json.dumps({'artifacts': len(data), 'counts': manifest['counts']} if args.prepare_only else write_import(ROOT, args.revision, ROOT / 'imports/spells' / ('r' + str(args.revision)), data, manifest), sort_keys=True))
