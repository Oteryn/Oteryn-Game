"""Copy the two reviewed r46 movement descriptors into an inactive source set."""
import argparse
import gzip
import json
from pathlib import Path

from import_source_spell_package import ROOT, digest, require
from source_spell_import_guards import read_base, read_package, write_source_only_set

BASE_SHA = '1c8ed40b00c5457ad408cbfd918147e0e1c197cd5877f7f8e8f18305e9cb7b5f'
SOURCE = 'docs/reference/spells/r46-source-closure/player-movement-candidates'
MANIFEST_SHA = 'cc53b62c31ea58d911b40283562b67bf609ad4679c6173bbd8e8261a45061e1e'
PROOF_SHA = '78c6b468e0e19735ac5592a4c0b70e976d8b2e208c566940a9d8c184d05e3541'
REGISTRATIONS = tuple('canary-main-current/data/scripts/spells/support/' + name + '.lua#1' for name in ('levitate', 'magic_rope'))
REVISION = 'source-player-r46'


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()


def folder_for(registration):
    return registration.split('/', 1)[0] + '/' + digest(registration.encode())[:16]


def preserves_header(header, candidate):
    if isinstance(header, dict):
        return isinstance(candidate, dict) and all(k in candidate and preserves_header(v, candidate[k]) for k, v in header.items())
    return header == candidate


def validate_descriptor(spell, dependencies, catalog, expected):
    require(dependencies == {'abilities': [], 'effects': [], 'formulas': []}, 'MOVEMENT_DEPENDENCIES_REFUSED')
    require(catalog == {'definitions': []}, 'MOVEMENT_CATALOG_REFUSED')
    require(spell['spell']['execution'] == {'native_behavior': expected}, 'MOVEMENT_DESCRIPTOR_MISMATCH')


def prepare_import(root, source=None):
    source = root / SOURCE if source is None else source
    require(source.resolve().is_relative_to(root.resolve()), 'SOURCE_OUTSIDE_REPOSITORY')
    base, base_body, baseline, schemas, validators = read_base(root, BASE_SHA)
    packet = read_package(source)
    require(digest(packet['package-manifest.json']) == MANIFEST_SHA, 'SOURCE_REVIEW_PIN_MISMATCH')
    require(digest(packet['source-qualification-proof.json']) == PROOF_SHA, 'PROOF_REVIEW_PIN_MISMATCH')
    require(packet['receipt.schema.json'] == schemas['schemas/player-bundle-receipt.schema.json'], 'RECEIPT_SCHEMA_CHANGED')
    names = {'package-manifest.json', 'import-summary.json', 'source-callback-facts.jsonl.gz', 'source-qualification-proof.json', 'receipt.schema.json'}
    for reg in REGISTRATIONS:
        names.update(folder_for(reg) + '/' + name for name in ['source-header.json', 'receipt.json', 'spell.json', 'dependencies.json', 'catalog.json'])
    require(set(packet) == names, 'MOVEMENT_PACKET_MEMBERSHIP_MISMATCH')
    summary = json.loads(packet['import-summary.json'])
    proof = json.loads(packet['source-qualification-proof.json'])
    require(summary['records'] == 2 and summary['revision'] == REVISION and summary['status_counts'] == {'CANDIDATE_SCHEMA_VALID': 2}
            and summary['runtime_activation'] is False and summary['external_sources_used'] is False
            and summary['native_descriptor_count'] == 2 and summary['full_source_mechanics_1_to_1_complete'] is False, 'SOURCE_STATUS_MISMATCH')
    expected_index = [{'blockers': [], 'candidate_key': 'candidate:spell/source/' + folder_for(reg), 'candidate_revision': REVISION,
                       'registration_key': reg, 'status': 'CANDIDATE_SCHEMA_VALID'} for reg in REGISTRATIONS]
    require(sorted(summary['records_index'], key=lambda r: r['registration_key']) == sorted(expected_index, key=lambda r: r['registration_key']), 'SOURCE_INDEX_MISMATCH')
    require(set(summary['input_proofs']) == {'r28/package-manifest.json', 'r28/import-summary.json'}, 'SOURCE_INPUT_PROOFS_MISMATCH')
    for name in ['package-manifest.json', 'import-summary.json']:
        require(summary['input_proofs']['r28/' + name] == digest((base / 'player-source-bundles' / name).read_bytes()), 'BASE_PLAYER_INPUT_PIN_MISMATCH')
    require(proof['schema'] == 'OTERYN_SOURCE_MOVEMENT_QUALIFICATION/v1'
            and proof['qualification_scope'] == 'existing_authoring_movement_descriptor_transfer_only', 'PROOF_SCOPE_MISMATCH')
    for name in ['spell.schema.json', 'spell-dependencies.schema.json']:
        require(proof['schema_proofs'][name] == digest(schemas['schemas/' + name]), 'PROOF_SCHEMA_PIN_MISMATCH')
    require(all(proof[k] is False for k in ['runtime_activation', 'native_execution_qualified', 'native_identity_allocation', 'canonical_selection_changed', 'external_sources_used', 'legacy_matcher_modified']), 'PROOF_QUALIFICATION_MISMATCH')
    base_capture = (base / 'player-source-bundles/source-callback-facts.jsonl.gz').read_bytes()
    require(packet['source-callback-facts.jsonl.gz'] == base_capture, 'CALLBACK_SOURCE_CHANGED')
    capture = {r['registration_key']: r for r in map(json.loads, gzip.decompress(base_capture).splitlines())}
    require(proof['source_capture_gzip_sha256'] == digest(base_capture) and proof['source_capture_records'] == len(capture) == 483
            and summary['captured_records'] == 483 and summary['source_callback_payload_sha256'] == digest(gzip.decompress(base_capture)), 'PROOF_CAPTURE_MISMATCH')
    records = {r['registration_key']: r for r in proof['records']}
    require(len(proof['records']) == len(records) == 2 and set(records) == set(REGISTRATIONS), 'PROOF_RECORD_POPULATION_MISMATCH')
    for reg in REGISTRATIONS:
        folder = folder_for(reg)
        old_folder = base / 'player-source-bundles' / folder
        old_receipt = (old_folder / 'receipt.json').read_bytes()
        old_header = (old_folder / 'source-header.json').read_bytes()
        require(json.loads(old_receipt)['status'] == 'BLOCKED', 'BASE_REGISTRATION_NOT_BLOCKED')
        require(packet[folder + '/source-header.json'] == old_header, 'HEADER_SOURCE_CHANGED')
        fact = capture[reg]
        record = records[reg]
        require(record['historical_receipt_sha256'] == digest(old_receipt) and record['historical_header_sha256'] == digest(old_header)
                and record['selected_capture_fact_sha256'] == digest(canonical(fact)), 'PROOF_HISTORICAL_INPUT_MISMATCH')
        pin = baseline['source_pins']['player_snapshots']['canary-main-current']
        key = 'candidate:spell/source/' + folder
        require(record['source_revision'] == pin and record['source_sha256'] == fact['source_sha256']
                and record['candidate_key'] == key and record['candidate_revision'] == REVISION
                and record['source_path'] == reg.split('/', 1)[1].rsplit('#', 1)[0]
                and record['source_git_blob'] == fact['source_callback_facts']['blob']
                and record['native_execution_qualified'] is False, 'PROOF_SOURCE_IDENTITY_MISMATCH')
        receipt = json.loads(packet[folder + '/receipt.json'])
        header = json.loads(old_header)['spell']
        validators['schemas/player-bundle-receipt.schema.json'].validate(receipt)
        require(receipt['status'] == 'CANDIDATE_SCHEMA_VALID' and not receipt['blockers'] and receipt['runtime_activation'] is False
                and receipt['native_execution_qualified'] is False and receipt['external_sources_used'] is False
                and receipt['source_revision'] == pin and receipt['source_sha256'] == fact['source_sha256']
                and receipt['source_header'] == header and receipt['registration_key'] == reg and receipt['candidate_key'] == key
                and receipt['remaining_mechanics'], 'RECEIPT_SOURCE_OR_STATUS_MISMATCH')
        spell = json.loads(packet[folder + '/spell.json'])
        deps = json.loads(packet[folder + '/dependencies.json'])
        catalog = json.loads(packet[folder + '/catalog.json'])
        validators['schemas/spell.schema.json'].validate(spell)
        validators['schemas/spell-dependencies.schema.json'].validate(deps)
        require(spell['spell']['identity'] == {'key': key, 'revision': REVISION}, 'SPELL_IDENTITY_MISMATCH')
        require(preserves_header(header, spell['spell']), 'SPELL_HEADER_CHANGED')
        require(receipt['dependencies'] == {k: 0 for k in ['abilities', 'effects', 'formulas']}, 'RECEIPT_DEPENDENCY_COUNT_MISMATCH')
        crystal_reg = reg.replace('canary-main-current/', 'crystal-summer-current/', 1)
        counterpart = json.loads((base / 'player-source-bundles' / folder_for(crystal_reg) / 'spell.json').read_bytes())
        require({k: v for k, v in spell['spell'].items() if k != 'identity'} ==
                {k: v for k, v in counterpart['spell'].items() if k != 'identity'}, 'MOVEMENT_ARCHIVED_DEFAULTS_OR_FIELDS_MISMATCH')
        expected = counterpart['spell']['execution']['native_behavior']
        validate_descriptor(spell, deps, catalog, expected)
        require(record['native_behavior_sha256'] == digest(canonical(expected)), 'PROOF_DESCRIPTOR_PIN_MISMATCH')
    source_path = source.resolve().relative_to(root.resolve()).as_posix()
    data = {'player-source-candidates/' + name: body for name, body in packet.items()}
    data.update(schemas)
    data['base-r28/import-manifest.json'] = base_body
    def refs(name):
        schema = {'spell.json': 'spell.schema.json', 'dependencies.json': 'spell-dependencies.schema.json', 'receipt.json': 'player-bundle-receipt.schema.json'}.get(Path(name).name)
        return [json.loads(schemas['schemas/' + schema])['$id']] if schema else []
    manifest = {'schema': 'OTERYN_INCREMENTAL_SOURCE_SPELL_IMPORT/v1', 'admission_status': 'source_only_not_active',
        'runtime_activation': False, 'native_identity_allocation': False, 'canonical_selection_changed': False,
        'native_execution_qualified': False, 'input_provider_equivalence': False,
        'base': {'path': 'imports/spells/r28/import-manifest.json', 'sha256': BASE_SHA, 'snapshot': 'base-r28/import-manifest.json'},
        'source_package': {'path': source_path + '/package-manifest.json', 'sha256': MANIFEST_SHA}, 'qualification_proof_sha256': PROOF_SHA,
        'source_pins': baseline['source_pins'], 'schemaRefs': baseline['schemaRefs'], 'source_keys': list(REGISTRATIONS),
        'counts': {'records': 2, 'status_counts': {'CANDIDATE_SCHEMA_VALID': 2}, 'native_descriptors': 2, 'abilities': 0, 'effects': 0, 'formulas': 0},
        'artifacts': [{'path': name, 'sha256': digest(body), 'bytes': len(body), 'schemaRefs': refs(name),
            'role': 'base_manifest' if name.startswith('base-r28/') else 'archived_base_schema' if name.startswith('schemas/') else 'reviewed_movement_source_candidate',
            'sourcePath': 'imports/spells/r28/import-manifest.json' if name.startswith('base-r28/') else 'imports/spells/r28/' + name if name.startswith('schemas/') else source_path + '/' + name.split('/', 1)[1]}
            for name, body in sorted(data.items())],
        'limits': ['Only the two reviewed Canary Levitate/Magic Rope source descriptors are copied.',
                   'Historical BLOCKED receipts, headers and captures remain immutable in r28.',
                   'Source descriptor qualification does not qualify native execution, provider equivalence, assets or activation.']}
    return data, manifest


def write_import(root, destination, data, manifest):
    require(manifest['source_keys'] == list(REGISTRATIONS) and manifest['source_package']['sha256'] == MANIFEST_SHA, 'WRONG_MOVEMENT_IMPORT_FAMILY')
    return write_source_only_set(root, destination, 'imports/spells/r46', data, manifest)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path)
    parser.add_argument('--prepare-only', action='store_true')
    args = parser.parse_args()
    data, manifest = prepare_import(ROOT, args.source)
    print(json.dumps({'artifacts': len(data), 'source_keys': manifest['source_keys']} if args.prepare_only else write_import(ROOT, ROOT / 'imports/spells/r46', data, manifest), sort_keys=True))
