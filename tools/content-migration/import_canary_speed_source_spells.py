#!/usr/bin/env python3
"""Materialize three Canary speed candidates with explicit provider inequivalence."""
import argparse
import gzip
import json
from pathlib import Path

from jsonschema import Draft202012Validator

from import_source_spell_package import ROOT, digest, require
from source_spell_import_guards import read_base, read_package, write_source_only_set

SOURCE = 'docs/reference/spells/r30-source-closure/player-canary-companion-source-candidates'
BASE_SHA = '1c8ed40b00c5457ad408cbfd918147e0e1c197cd5877f7f8e8f18305e9cb7b5f'
SOURCE_KEYS = {'canary-main-current/data/scripts/spells/support/' + name + '.lua#1'
               for name in ('haste', 'strong_haste', 'charge')}


def prepare_import(root, source, expected_base_sha=BASE_SHA):
    base, base_body, baseline, schemas, validators = read_base(root, expected_base_sha)
    packet = read_package(source)
    require(packet['receipt.schema.json'] == schemas['schemas/player-bundle-receipt.schema.json'], 'RECEIPT_SCHEMA_CHANGED')
    summary = json.loads(packet['import-summary.json'])
    require(summary['runtime_activation'] is False and summary['external_sources_used'] is False
            and summary['revision'] == 'source-player-r30' and summary['records'] == 3
            and summary['status_counts'] == {'CANDIDATE_SCHEMA_VALID': 3}, 'SOURCE_STATUS_MISMATCH')
    require(set(summary['input_proofs']) == {'r28/package-manifest.json', 'condition-speed-equivalence-r30.json'}, 'SOURCE_INPUT_PROOFS_MISMATCH')
    require(digest((base / 'player-source-bundles/package-manifest.json').read_bytes()) == summary['input_proofs']['r28/package-manifest.json'],
            'BASE_PLAYER_PACKAGE_PIN_MISMATCH')
    donor_revision = baseline['source_pins']['player_snapshots']['canary-main-current']
    input_schema_name = 'player-source-candidates/source-speed-inputs.schema.json'
    input_schema = json.loads(packet['source-speed-inputs.schema.json'])
    Draft202012Validator.check_schema(input_schema)
    provider = json.loads(packet['source-speed-inputs.json'])
    Draft202012Validator(input_schema).validate(provider)
    require(provider['source_revision'] == donor_revision and provider['input_provider_equivalence'] is False
            and provider['runtime_activation'] is False and provider['runtime_admission'] == 'blocked'
            and provider['old_staff_cap'] == 1500 and provider['current_staff_cap'] == provider['regular_cap'] == 65535,
            'INPUT_PROVIDER_SOURCE_OR_EQUIVALENCE_MISMATCH')
    helper = json.loads(packet['helper-qualification-proof.json'])
    require(helper['source_revision'] == donor_revision and helper['old_revision'] == provider['old_revision']
            and helper['runtime_activation'] is False and helper['external_sources_used'] is False
            and helper['global_input_provider_equivalent'] is False
            and helper['audit_sha256'] == summary['input_proofs']['condition-speed-equivalence-r30.json'], 'HELPER_PROOF_SCOPE_MISMATCH')
    require(helper['source_functions_and_storage'] and all(set(row) == {'path', 'symbol', 'status', 'old', 'current'}
            and row['status'] in {'byte_identical', 'changed_not_equivalent'} and row['current']
            and (row['old'] or row['status'] == 'changed_not_equivalent')
            for row in helper['source_functions_and_storage']), 'HELPER_PROOF_RECORD_INVALID')
    payload = gzip.decompress(packet['source-callback-facts.jsonl.gz'])
    require(digest(payload) == summary['source_callback_payload_sha256'], 'CALLBACK_PAYLOAD_PIN_MISMATCH')
    callbacks = [json.loads(line) for line in payload.splitlines()]
    require(len(callbacks) == 3 and {r['registration_key'] for r in callbacks} == SOURCE_KEYS, 'CALLBACK_POPULATION_MISMATCH')
    old_callbacks = {r['registration_key']: r for r in map(json.loads, gzip.decompress(
        (base / 'player-source-bundles/source-callback-facts.jsonl.gz').read_bytes()).splitlines())}
    require(all(row == old_callbacks[row['registration_key']] for row in callbacks), 'CALLBACK_SOURCE_CHANGED')
    callback_index = {r['registration_key']: r for r in callbacks}
    receipts = []
    for name, body in packet.items():
        if not name.endswith('/receipt.json'):
            continue
        receipt = json.loads(body)
        validators['schemas/player-bundle-receipt.schema.json'].validate(receipt)
        require(receipt['status'] == 'CANDIDATE_SCHEMA_VALID' and not receipt['blockers'] and receipt['runtime_activation'] is False
                and receipt['native_execution_qualified'] is False and receipt['source_revision'] == donor_revision, 'RECEIPT_STATUS_MISMATCH')
        folder = name.rsplit('/', 1)[0]
        callback = callback_index.get(receipt['registration_key'])
        header = json.loads(packet[folder + '/source-header.json'])
        require(callback is not None and all(receipt[key] == callback[key] for key in ('source_revision', 'source_sha256'))
                and receipt['source_header'] == header['spell'], 'RECEIPT_SOURCE_IDENTITY_MISMATCH')
        require(packet[folder + '/source-header.json'] == (base / 'player-source-bundles' / folder / 'source-header.json').read_bytes(),
                'HEADER_SOURCE_CHANGED')
        spell = json.loads(packet[folder + '/spell.json'])
        validators['schemas/spell.schema.json'].validate(spell)
        validators['schemas/spell-dependencies.schema.json'].validate(json.loads(packet[folder + '/dependencies.json']))
        require(spell['spell']['identity'] == {'key': receipt['candidate_key'], 'revision': 'source-player-r30'}, 'CANDIDATE_IDENTITY_MISMATCH')
        receipts.append(receipt)
    require(len(receipts) == 3 and {r['registration_key'] for r in receipts} == SOURCE_KEYS, 'RECEIPT_POPULATION_MISMATCH')
    require(len(summary['records_index']) == 3 and {r['registration_key'] for r in summary['records_index']} == SOURCE_KEYS,
            'INDEX_POPULATION_MISMATCH')
    require(source.resolve().is_relative_to(root.resolve()), 'SOURCE_OUTSIDE_REPOSITORY')
    source_path = source.resolve().relative_to(root.resolve()).as_posix()
    data = {'player-source-candidates/' + name: body for name, body in packet.items()}
    data.update(schemas)
    data['base-r28/import-manifest.json'] = base_body
    schema_refs = baseline['schemaRefs'] + [{'uri': input_schema_name, 'path': input_schema_name,
                                           'sha256': digest(packet['source-speed-inputs.schema.json'])}]
    def refs(name):
        if name.endswith('/source-speed-inputs.json'):
            return [input_schema_name]
        schema = {'receipt.json': 'player-bundle-receipt.schema.json', 'spell.json': 'spell.schema.json',
                  'dependencies.json': 'spell-dependencies.schema.json'}.get(Path(name).name)
        return [json.loads(schemas['schemas/' + schema])['$id']] if schema else []
    manifest = {'schema': 'OTERYN_INCREMENTAL_SOURCE_SPELL_IMPORT/v1', 'admission_status': 'source_only_not_active',
                'runtime_activation': False, 'native_identity_allocation': False, 'canonical_selection_changed': False,
                'native_execution_qualified': False, 'input_provider_equivalence': False,
                'base': {'path': 'imports/spells/r28/import-manifest.json', 'sha256': expected_base_sha, 'snapshot': 'base-r28/import-manifest.json'},
                'source_package': {'path': source_path + '/package-manifest.json', 'sha256': digest(packet['package-manifest.json'])},
                'source_pins': baseline['source_pins'], 'schemaRefs': schema_refs, 'source_keys': sorted(SOURCE_KEYS),
                'counts': {'records': 3, 'status_counts': summary['status_counts'], 'source_function_and_storage_proofs': len(helper['source_functions_and_storage'])},
                'artifacts': [{'path': name, 'sha256': digest(body), 'bytes': len(body), 'schemaRefs': refs(name),
                               'role': 'base_manifest' if name.startswith('base-r28/') else 'archived_base_schema' if name.startswith('schemas/') else 'incremental_source_candidate',
                               'sourcePath': 'imports/spells/r28/import-manifest.json' if name.startswith('base-r28/') else 'imports/spells/r28/' + name if name.startswith('schemas/') else source_path + '/' + name.split('/', 1)[1]}
                              for name, body in sorted(data.items())],
                'limits': ['Only current Canary Haste, Strong Haste and Charge source candidates are imported.',
                           'The speed operator is parametric; current SetMaxSpeed staff input cap is 65535, historical cap 1500.',
                           'Input-provider equivalence and runtime admission remain blocked; scoped helper facts do not prove all common engine services.',
                           'The upstream generation audit is retained by its hash; the distributed helper proof contains sanitized source metadata.',
                           'The immutable r28/r29 sets and active content selection, native IDs and runtime dispatch remain unchanged.']}
    return data, manifest


def write_import(root, destination, data, manifest):
    return write_source_only_set(root, destination, 'imports/spells/r30', data, manifest)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, default=ROOT / SOURCE)
    parser.add_argument('--base-sha256', default=BASE_SHA)
    parser.add_argument('--out', type=Path, default=ROOT / 'imports/spells/r30')
    args = parser.parse_args()
    data, manifest = prepare_import(ROOT, args.source, args.base_sha256)
    print(json.dumps(write_import(ROOT, args.out, data, manifest), indent=2))


if __name__ == '__main__':
    main()
