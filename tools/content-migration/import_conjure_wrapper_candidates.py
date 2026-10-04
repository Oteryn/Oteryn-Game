#!/usr/bin/env python3
"""Import four exact instant Conjure wrappers against immutable r28 source inputs."""
import argparse
import gzip
import json
from pathlib import Path
import subprocess

from import_source_spell_package import ROOT, digest, require
from source_spell_import_guards import read_base, read_package, write_source_only_set

SOURCE = 'docs/reference/spells/r34-source-closure/player-conjure-wrappers'
BASE_SHA = '1c8ed40b00c5457ad408cbfd918147e0e1c197cd5877f7f8e8f18305e9cb7b5f'
SOURCE_KEYS = {snapshot + '/data/scripts/spells/conjuring/' + name + '.lua#1'
               for snapshot in ('canary-main-current', 'crystal-summer-current')
               for name in ('chameleon_rune', 'destroy_field_rune')}


def prepare_import(root, source, expected_base_sha=BASE_SHA, repositories=Path('/workspace/spell-sources')):
    base, base_body, baseline, schemas, validators = read_base(root, expected_base_sha)
    packet = read_package(source)
    require(packet['receipt.schema.json'] == schemas['schemas/player-bundle-receipt.schema.json'], 'WRAPPER_RECEIPT_SCHEMA_CHANGED')
    summary = json.loads(packet['import-summary.json'])
    require(summary['runtime_activation'] is False and summary['external_sources_used'] is False
            and summary['revision'] == 'source-player-r34' and summary['records'] == 4
            and summary['status_counts'] == {'CANDIDATE_SCHEMA_VALID': 4}, 'WRAPPER_STATUS_MISMATCH')
    require(set(summary['input_proofs']) == {'r28/package-manifest.json'} and digest(
        (base / 'player-source-bundles/package-manifest.json').read_bytes()) == summary['input_proofs']['r28/package-manifest.json'], 'WRAPPER_BASE_INPUT_PIN_MISMATCH')
    payload = gzip.decompress(packet['source-callback-facts.jsonl.gz'])
    require(digest(payload) == summary['source_callback_payload_sha256'], 'WRAPPER_CALLBACK_PAYLOAD_MISMATCH')
    callbacks = [json.loads(line) for line in payload.splitlines()]
    require(len(callbacks) == 4 and {r['registration_key'] for r in callbacks} == SOURCE_KEYS, 'WRAPPER_CALLBACK_POPULATION_MISMATCH')
    old_callbacks = {r['registration_key']: r for r in map(json.loads, gzip.decompress(
        (base / 'player-source-bundles/source-callback-facts.jsonl.gz').read_bytes()).splitlines())}
    require(all(row == old_callbacks[row['registration_key']] for row in callbacks), 'WRAPPER_CALLBACK_SOURCE_CHANGED')
    callback_index = {r['registration_key']: r for r in callbacks}
    pins = {s: baseline['source_pins']['player_snapshots'][s] for s in ('canary-main-current', 'crystal-summer-current')}
    proof = json.loads(packet['wrapper-source-proof.json'])
    require(proof['runtime_activation'] is False and proof['external_sources_used'] is False and proof['source_pins'] == pins
            and proof['item_provider_revision'] == 'source-player-r28' and len(proof['records']) == 4
            and {r['registration_key'] for r in proof['records']} == SOURCE_KEYS, 'WRAPPER_PROOF_SCOPE_MISMATCH')
    proof_index = {r['registration_key']: r for r in proof['records']}
    helper_cache = {}
    for row in proof['records']:
        callback = callback_index[row['registration_key']]
        facts = callback['source_callback_facts']
        require(facts['spell_type'] == 'instant' and facts['cast']['tier'] == 'conjure'
                and all(row[k] == facts['cast']['conjure'][k] for k in ('reagent_item_id', 'result_item_id', 'count'))
                and row['argument_count'] == 3 and row['missing_fourth_argument'] == 'nil'
                and row['runtime_activation'] is False and row['result_item_type_qualified'] is False, 'WRAPPER_SOURCE_CONJURE_MISMATCH')
        donor = row['registration_key'].split('/', 1)[0]
        repo = repositories / donor.split('-', 1)[0]
        script = subprocess.check_output(['git', '-C', str(repo), 'show', pins[donor] + ':' + facts['file']])
        require(digest(script) == callback['source_sha256'], 'WRAPPER_GIT_SOURCE_HASH_MISMATCH')
        lines = script.splitlines()
        scope = b'\n'.join(lines[row['start_line'] - 1:row['end_line']]).strip()
        require(digest(scope) == row['callback_sha256'], 'WRAPPER_CAST_SCOPE_HASH_MISMATCH')
        helper = row['helper_effect_branch_source']
        require(helper['source_revision'] == pins[donor] and helper['path'] == 'data/scripts/lib/register_spells.lua', 'WRAPPER_HELPER_SOURCE_PIN_MISMATCH')
        if donor not in helper_cache:
            helper_cache[donor] = subprocess.check_output(['git', '-C', str(repo), 'show', pins[donor] + ':' + helper['path']])
        helper_body = helper_cache[donor]
        require(digest(helper_body) == helper['file_sha256'], 'WRAPPER_HELPER_HASH_MISMATCH')
        require(digest(helper_body.splitlines()[helper['branch_line'] - 1].strip()) == helper['branch_sha256'], 'WRAPPER_HELPER_BRANCH_HASH_MISMATCH')
    receipts = []
    for name, body in packet.items():
        if not name.endswith('/receipt.json'): continue
        receipt = json.loads(body)
        validators['schemas/player-bundle-receipt.schema.json'].validate(receipt)
        callback = callback_index.get(receipt['registration_key'])
        folder = name.rsplit('/', 1)[0]
        require(callback is not None and receipt['source_revision'] == callback['source_revision'] and receipt['source_sha256'] == callback['source_sha256']
                and receipt['source_header'] == json.loads(packet[folder + '/source-header.json'])['spell'], 'WRAPPER_RECEIPT_SOURCE_IDENTITY_MISMATCH')
        require(receipt['runtime_activation'] is False and receipt['native_execution_qualified'] is False and receipt['status'] == 'CANDIDATE_SCHEMA_VALID'
                and not receipt['blockers'], 'WRAPPER_RECEIPT_STATUS_MISMATCH')
        require(packet[folder + '/source-header.json'] == (base / 'player-source-bundles' / folder / 'source-header.json').read_bytes(), 'WRAPPER_HEADER_SOURCE_CHANGED')
        spell = json.loads(packet[folder + '/spell.json'])
        validators['schemas/spell.schema.json'].validate(spell)
        validators['schemas/spell-dependencies.schema.json'].validate(json.loads(packet[folder + '/dependencies.json']))
        require(spell['spell']['carrier'] == 'instant' and spell['spell']['identity'] == {'key': receipt['candidate_key'], 'revision': 'source-player-r34'}, 'WRAPPER_SPELL_IDENTITY_MISMATCH')
        conjure = spell['spell']['execution']['conjure']
        row = proof_index[receipt['registration_key']]
        snapshot = receipt['registration_key'].split('/', 1)[0]
        expected_refs = [{'family': 'Item', 'key': 'candidate:item/source/' + snapshot + '/' + str(row[key]), 'revision': 'source-player-r28'}
                         for key in ('reagent_item_id', 'result_item_id')]
        require(conjure['count'] == row['count'] and [conjure['reagent'], conjure['result']] == expected_refs
                and receipt['item_owner_bindings_required'] == expected_refs, 'WRAPPER_ITEM_PROVIDER_IDENTITY_MISMATCH')
        receipts.append(receipt)
    require(len(receipts) == 4 and {r['registration_key'] for r in receipts} == SOURCE_KEYS
            and len(summary['records_index']) == 4 and {r['registration_key'] for r in summary['records_index']} == SOURCE_KEYS, 'WRAPPER_RECEIPT_POPULATION_MISMATCH')
    require(source.resolve().is_relative_to(root.resolve()), 'WRAPPER_SOURCE_OUTSIDE_REPOSITORY')
    source_path = source.resolve().relative_to(root.resolve()).as_posix()
    data = {'player-source-candidates/' + name: body for name, body in packet.items()}
    data.update(schemas); data['base-r28/import-manifest.json'] = base_body
    def refs(name):
        schema = {'receipt.json': 'player-bundle-receipt.schema.json', 'spell.json': 'spell.schema.json', 'dependencies.json': 'spell-dependencies.schema.json'}.get(Path(name).name)
        return [json.loads(schemas['schemas/' + schema])['$id']] if schema else []
    manifest = {'schema': 'OTERYN_INCREMENTAL_SOURCE_SPELL_IMPORT/v1', 'admission_status': 'source_only_not_active',
                'runtime_activation': False, 'native_identity_allocation': False, 'canonical_selection_changed': False,
                'native_execution_qualified': False, 'input_provider_equivalence': False,
                'base': {'path': 'imports/spells/r28/import-manifest.json', 'sha256': expected_base_sha, 'snapshot': 'base-r28/import-manifest.json'},
                'source_package': {'path': source_path + '/package-manifest.json', 'sha256': digest(packet['package-manifest.json'])},
                'source_pins': baseline['source_pins'], 'schemaRefs': baseline['schemaRefs'], 'source_keys': sorted(SOURCE_KEYS),
                'counts': {'records': 4, 'status_counts': summary['status_counts'], 'result_item_type_qualified': 0},
                'artifacts': [{'path': name, 'sha256': digest(body), 'bytes': len(body), 'schemaRefs': refs(name),
                               'role': 'base_manifest' if name.startswith('base-r28/') else 'archived_base_schema' if name.startswith('schemas/') else 'incremental_source_candidate',
                               'sourcePath': 'imports/spells/r28/import-manifest.json' if name.startswith('base-r28/') else 'imports/spells/r28/' + name if name.startswith('schemas/') else source_path + '/' + name.split('/', 1)[1]}
                              for name, body in sorted(data.items())],
                'limits': ['Only four source instant Conjure wrappers are imported; Item identities retain their source-player-r28 provider revision.',
                           'Result Item type and the conditional rune RED versus missing fourth argument visual branch remain unqualified.',
                           'Existing inactive import sets and canonical selection, native identities, runtime dispatch and visual owners remain unchanged.']}
    return data, manifest


def write_import(root, destination, data, manifest):
    return write_source_only_set(root, destination, 'imports/spells/r34', data, manifest)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, default=ROOT / SOURCE)
    parser.add_argument('--source-repositories', type=Path, default=Path('/workspace/spell-sources'))
    parser.add_argument('--base-sha256', default=BASE_SHA)
    parser.add_argument('--out', type=Path, default=ROOT / 'imports/spells/r34')
    args = parser.parse_args()
    data, manifest = prepare_import(ROOT, args.source, args.base_sha256, args.source_repositories)
    print(json.dumps(write_import(ROOT, args.out, data, manifest), indent=2))


if __name__ == '__main__': main()
