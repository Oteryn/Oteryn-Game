#!/usr/bin/env python3
"""Import four source-only Crystal candidates against the immutable r28 import."""
import argparse
import gzip
import json
from pathlib import Path
import shutil

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

from import_source_spell_package import ROOT, bundle_member_path, digest, encoded, require

SOURCE = 'docs/reference/spells/r29-source-closure/player-companion-source-candidates'
BASE_SHA = '1c8ed40b00c5457ad408cbfd918147e0e1c197cd5877f7f8e8f18305e9cb7b5f'
SOURCE_KEYS = {'crystal-summer-current/data/scripts/spells/support/' + name + '.lua#1'
               for name in ('haste', 'strong_haste', 'charge', 'swift_foot')}


def prepare_import(root, source, expected_base_sha):
    base = root / 'imports/spells/r28'
    base_body = (base / 'import-manifest.json').read_bytes()
    require(digest(base_body) == expected_base_sha, 'BASE_MANIFEST_PIN_MISMATCH')
    baseline = json.loads(base_body)
    require(baseline['admission_status'] == 'source_only_not_active' and baseline['runtime_activation'] is False,
            'BASE_NOT_SOURCE_ONLY')
    for entry in baseline['artifacts'] + baseline['schemaRefs']:
        require(digest(bundle_member_path(base, entry['path']).read_bytes()) == entry['sha256'],
                'BASE_ARTIFACT_PIN_MISMATCH:' + entry['path'])
    schemas = {s['path']: (base / s['path']).read_bytes() for s in baseline['schemaRefs']}
    documents = {name: json.loads(body) for name, body in schemas.items()}
    registry = Registry().with_resources((d['$id'], Resource.from_contents(d)) for d in documents.values() if '$id' in d)
    validators = {name: Draft202012Validator(doc, registry=registry) for name, doc in documents.items()}
    package_body = bundle_member_path(source, 'package-manifest.json').read_bytes()
    package = json.loads(package_body)
    names = set(package['files']) | {'package-manifest.json'}
    require(names == {p.relative_to(source).as_posix() for p in source.rglob('*') if p.is_file()}, 'INCREMENT_MEMBERSHIP_MISMATCH')
    data = {}
    for name in sorted(names):
        require(name.endswith('.json') or name == 'source-callback-facts.jsonl.gz', 'ORIGINAL_ASSET_REFUSED')
        body = bundle_member_path(source, name).read_bytes()
        if name != 'package-manifest.json':
            require(digest(body) == package['files'][name], 'INCREMENT_ARTIFACT_PIN_MISMATCH:' + name)
        data['player-source-candidates/' + name] = body
    require(data['player-source-candidates/receipt.schema.json'] == schemas['schemas/player-bundle-receipt.schema.json'],
            'INCREMENT_RECEIPT_SCHEMA_CHANGED')
    summary = json.loads(data['player-source-candidates/import-summary.json'])
    require(summary['runtime_activation'] is False and summary['external_sources_used'] is False
            and summary['revision'] == 'source-player-r29' and summary['records'] == 4
            and summary['status_counts'] == {'CANDIDATE_SCHEMA_VALID': 4}, 'INCREMENT_STATUS_MISMATCH')
    crystal_revision = baseline['source_pins']['player_snapshots']['crystal-summer-current']
    require(summary['source_revision'] == crystal_revision, 'INCREMENT_DONOR_PIN_MISMATCH')
    for name, expected in summary['input_proofs'].items():
        require(name in ('r28/package-manifest.json', 'r28/source-callback-facts.jsonl.gz'), 'UNEXPECTED_BASE_INPUT')
        require(digest((base / 'player-source-bundles' / name.split('/', 1)[1]).read_bytes()) == expected,
                'INCREMENT_BASE_INPUT_MISMATCH')
    require(set(summary['input_proofs']) == {'r28/package-manifest.json', 'r28/source-callback-facts.jsonl.gz'}, 'INCREMENT_BASE_INPUT_MISSING')
    payload = gzip.decompress(data['player-source-candidates/source-callback-facts.jsonl.gz'])
    require(digest(payload) == summary['source_callback_payload_sha256'], 'INCREMENT_CALLBACK_HASH_MISMATCH')
    callbacks = [json.loads(line) for line in payload.splitlines()]
    require(len(callbacks) == 4 and {x['registration_key'] for x in callbacks} == SOURCE_KEYS, 'INCREMENT_CALLBACK_POPULATION_MISMATCH')
    old_callbacks = {x['registration_key']: x for x in map(json.loads, gzip.decompress(
        (base / 'player-source-bundles/source-callback-facts.jsonl.gz').read_bytes()).splitlines())}
    require(all(row == old_callbacks[row['registration_key']] for row in callbacks), 'INCREMENT_CALLBACK_SOURCE_CHANGED')
    callback_index = {row['registration_key']: row for row in callbacks}
    grade_schema_path = 'player-source-candidates/source-grade-actions.schema.json'
    grade_schema = json.loads(data[grade_schema_path])
    Draft202012Validator.check_schema(grade_schema)
    grade_validator = Draft202012Validator(grade_schema)
    grade_facts = [json.loads(body) for name, body in data.items() if name.endswith('/source-grade-actions.json')]
    require(len(grade_facts) == 1, 'INCREMENT_GRADE_FACTS_MISSING')
    for facts in grade_facts:
        grade_validator.validate(facts)
        require(facts['registration_key'] == 'crystal-summer-current/data/scripts/spells/support/swift_foot.lua#1'
                and all(facts[key] == callback_index[facts['registration_key']][key] for key in ('source_revision', 'source_sha256')),
                'INCREMENT_GRADE_SOURCE_IDENTITY_MISMATCH')
    receipts = []
    for name, body in data.items():
        if name.endswith('/receipt.json'):
            receipt = json.loads(body)
            validators['schemas/player-bundle-receipt.schema.json'].validate(receipt)
            require(receipt['status'] == 'CANDIDATE_SCHEMA_VALID' and not receipt['blockers']
                    and receipt['runtime_activation'] is False and receipt['native_execution_qualified'] is False
                    and receipt['source_revision'] == crystal_revision, 'INCREMENT_RECEIPT_STATUS_MISMATCH')
            folder = name.rsplit('/', 1)[0]
            callback = callback_index.get(receipt['registration_key'])
            require(callback is not None and all(receipt[key] == callback[key] for key in ('source_revision', 'source_sha256'))
                    and receipt['source_header'] == json.loads(data[folder + '/source-header.json'])['spell'],
                    'INCREMENT_RECEIPT_SOURCE_IDENTITY_MISMATCH')
            spell = json.loads(data[folder + '/spell.json'])
            validators['schemas/spell.schema.json'].validate(spell)
            validators['schemas/spell-dependencies.schema.json'].validate(json.loads(data[folder + '/dependencies.json']))
            require(spell['spell']['identity'] == {'key': receipt['candidate_key'], 'revision': 'source-player-r29'}, 'INCREMENT_IDENTITY_MISMATCH')
            base_folder = base / 'player-source-bundles' / folder.split('/', 1)[1]
            require(data[folder + '/source-header.json'] == (base_folder / 'source-header.json').read_bytes(), 'INCREMENT_HEADER_SOURCE_CHANGED')
            receipts.append(receipt)
    require(len(receipts) == 4 and {x['registration_key'] for x in receipts} == SOURCE_KEYS, 'INCREMENT_RECEIPT_POPULATION_MISMATCH')
    require({x['registration_key'] for x in summary['records_index']} == SOURCE_KEYS, 'INCREMENT_INDEX_POPULATION_MISMATCH')
    require(source.resolve().is_relative_to(root.resolve()), 'INCREMENT_SOURCE_OUTSIDE_REPOSITORY')
    source_path = source.resolve().relative_to(root.resolve()).as_posix()
    data.update(schemas)
    data['base-r28/import-manifest.json'] = base_body
    def schema_refs(name):
        if name.endswith('/source-grade-actions.json'):
            return [grade_schema_path]
        ref = {'receipt.json': 'player-bundle-receipt.schema.json', 'spell.json': 'spell.schema.json',
               'dependencies.json': 'spell-dependencies.schema.json'}.get(Path(name).name)
        return [documents['schemas/' + ref]['$id']] if ref else []
    manifest = {'schema': 'OTERYN_INCREMENTAL_SOURCE_SPELL_IMPORT/v1', 'admission_status': 'source_only_not_active',
                'runtime_activation': False, 'native_identity_allocation': False, 'canonical_selection_changed': False,
                'native_execution_qualified': False, 'base': {'path': 'imports/spells/r28/import-manifest.json',
                    'sha256': expected_base_sha, 'snapshot': 'base-r28/import-manifest.json'},
                'source_pins': baseline['source_pins'], 'schemaRefs': baseline['schemaRefs'] +
                    [{'uri': grade_schema_path, 'path': grade_schema_path, 'sha256': digest(data[grade_schema_path])}],
                'counts': {'records': 4, 'status_counts': summary['status_counts']}, 'source_keys': sorted(SOURCE_KEYS),
                'source_package': {'path': source_path + '/package-manifest.json', 'sha256': digest(package_body)},
                'artifacts': [{'path': name, 'sha256': digest(body), 'bytes': len(body),
                               'role': 'base_manifest' if name.startswith('base-r28/') else 'archived_base_schema' if name.startswith('schemas/') else 'incremental_source_candidate',
                               'sourcePath': 'imports/spells/r28/import-manifest.json' if name.startswith('base-r28/') else 'imports/spells/r28/' + name if name.startswith('schemas/') else source_path + '/' + name.split('/', 1)[1],
                               'schemaRefs': schema_refs(name)} for name, body in sorted(data.items())],
                'limits': ['Only four Crystal source candidate variants supplement the immutable r28 import.',
                           'Haste, Strong Haste, Charge and Swift Foot remain source-only speed candidates with common engine and runtime blockers.',
                           'Swift Foot GREATER is a source no-op that preserves an existing modifier; the legacy value 100 is not a source write.',
                           'No active content selection, runtime dispatch, native identity allocation or production activation.']}
    return data, manifest


def write_import(root, destination, data, manifest):
    require(destination.resolve() == (root / 'imports/spells/r29').resolve(), 'INCREMENT_DESTINATION_REFUSED')
    require(not destination.exists(), 'INCREMENT_ALREADY_EXISTS')
    require(all(manifest[key] is False for key in ('runtime_activation', 'native_identity_allocation', 'canonical_selection_changed', 'native_execution_qualified')),
            'INCREMENT_ACTIVE_CLAIM_REFUSED')
    require(len(manifest['artifacts']) == len(data) and {x['path'] for x in manifest['artifacts']} == set(data), 'INCREMENT_OUTPUT_MEMBERSHIP_MISMATCH')
    for entry in manifest['artifacts']:
        require(not Path(entry['path']).is_absolute() and '..' not in Path(entry['path']).parts
                and digest(data[entry['path']]) == entry['sha256'] and len(data[entry['path']]) == entry['bytes'], 'INCREMENT_OUTPUT_PIN_MISMATCH')
    outputs = dict(data, **{'import-manifest.json': encoded(manifest)})
    destination.mkdir(parents=True, exist_ok=False)
    try:
        for name, body in outputs.items():
            path = destination / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(body)
        (destination / 'SHA256SUMS').write_text(''.join(f'{digest(body)}  {name}\n' for name, body in sorted(outputs.items())))
    except BaseException:
        shutil.rmtree(destination)
        raise
    return {'manifest_sha256': digest(outputs['import-manifest.json']), 'files': len(outputs) + 1, 'runtime_activation': False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, default=ROOT / SOURCE)
    parser.add_argument('--base-sha256', default=BASE_SHA)
    parser.add_argument('--out', type=Path, default=ROOT / 'imports/spells/r29')
    args = parser.parse_args()
    data, manifest = prepare_import(ROOT, args.source, args.base_sha256)
    print(json.dumps(write_import(ROOT, args.out, data, manifest), indent=2))


if __name__ == '__main__':
    main()
