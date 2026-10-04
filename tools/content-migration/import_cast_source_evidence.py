#!/usr/bin/env python3
"""Copy two reviewed partial cast batches; never produce an executable Spell."""
import argparse
import gzip
import json
from pathlib import Path

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

from import_source_spell_package import ROOT, bundle_member_path, digest, require
from source_spell_import_guards import read_base, write_source_only_set

BASE_SHA = '1c8ed40b00c5457ad408cbfd918147e0e1c197cd5877f7f8e8f18305e9cb7b5f'
CONFIGS = {
    'r47': {'artifact': 'source-simple-guard-evidence.jsonl.gz',
            'proof': 'source-simple-guard-proof.json', 'schema': 'source-simple-guard-evidence.schema.json',
            'proof_sha256': '35f5f1ef2c3eb12835d65878bf9ed95a16c71d8f5f09ca7f660a532c47c254b9', 'records': 14},
    'r48': {'artifact': 'source-multicombat-sequences.json.gz',
            'proof': 'source-multicombat-receipt.json', 'schema': 'source-multicombat-sequences.schema.json',
            'proof_sha256': '46fff04019cda64170591c070909aad839e97b3008c2cf01178f7c3ada212424', 'records': 3},
}


def prepare_import(root, revision, source=None):
    require(revision in CONFIGS, 'UNSUPPORTED_CAST_BATCH')
    config = CONFIGS[revision]
    require(config['proof_sha256'] is not None, 'CAST_BATCH_NOT_REVIEWED')
    source = source or root / 'docs/reference/spells' / (revision + '-source-closure')
    require(source.resolve().is_relative_to(root.resolve()), 'SOURCE_OUTSIDE_REPOSITORY')
    base, base_body, baseline, schemas, _ = read_base(root, BASE_SHA)
    proof_body = bundle_member_path(source, config['proof']).read_bytes()
    require(digest(proof_body) == config['proof_sha256'], 'CAST_REVIEW_PIN_MISMATCH')
    proof = json.loads(proof_body)
    compressed = bundle_member_path(source, config['artifact']).read_bytes()
    payload = gzip.decompress(compressed)
    schema_path = 'tools/content-schema/spell-authoring/' + config['schema']
    schema_body = bundle_member_path(root, schema_path).read_bytes()
    require(digest(compressed) == proof['gzip_sha256'] and digest(payload) == proof['payload_sha256']
            and digest(schema_body) == proof['schema_sha256'], 'CAST_PACKET_HASH_MISMATCH')
    callback_body = (base / 'player-source-bundles/source-callback-facts.jsonl.gz').read_bytes()
    require(proof['base_capture_sha256'] == digest(callback_body), 'CAST_BASE_CAPTURE_MISMATCH')
    callbacks = {r['registration_key']: r for r in map(json.loads, gzip.decompress(callback_body).splitlines())}
    registry = Registry()
    if revision == 'r48':
        syntax_path = 'tools/content-schema/spell-authoring/source-syntax.schema.json'
        syntax_body = bundle_member_path(root, syntax_path).read_bytes()
        require(digest(syntax_body) == proof['syntax_schema_sha256'], 'CAST_SYNTAX_SCHEMA_MISMATCH')
        syntax_schema = json.loads(syntax_body)
        registry = registry.with_resource(syntax_schema['$id'], Resource.from_contents(syntax_schema))
        schemas['schemas/source-syntax.schema.json'] = syntax_body
    schema = json.loads(schema_body)
    validator = Draft202012Validator(schema, registry=registry)
    if revision == 'r47':
        rows = [json.loads(line) for line in payload.splitlines()]
        for row in rows:
            validator.validate(row)
    else:
        packet = json.loads(payload)
        validator.validate(packet)
        require(packet['candidate_count'] == 0 and packet['runtime_activation'] is False
                and packet['native_admission'] is False, 'CAST_EXECUTION_CLAIM_REFUSED')
        rows = packet['records']
    require(len(rows) == config['records'] and len({r['registration_key'] for r in rows}) == len(rows),
            'CAST_SOURCE_POPULATION_MISMATCH')
    for row in rows:
        key = row['registration_key']
        require(key in callbacks and row['complete_spell_candidate'] is False
                and row['runtime_activation'] is False, 'CAST_CANDIDATE_ESCALATION_REFUSED')
        callback = callbacks[key]
        if revision == 'r47':
            require(row['source_sha256'] == callback['source_sha256']
                    and row['source_revision'] == callback['source_revision'], 'CAST_SOURCE_IDENTITY_MISMATCH')
        else:
            require(row['source_syntax']['source_sha256'] == callback['source_sha256']
                    and row['source_syntax']['revision'] == callback['source_revision'], 'CAST_SOURCE_IDENTITY_MISMATCH')
        old = base / 'player-source-bundles' / key.split('/')[0] / digest(key.encode())[:16] / 'receipt.json'
        require(json.loads(old.read_bytes())['status'] == 'BLOCKED', 'CAST_BASE_STATUS_MISMATCH')
    local_schema = 'schemas/' + config['schema']
    schemas[local_schema] = schema_body
    data = {**schemas, 'base-r28/import-manifest.json': base_body,
            'evidence/' + config['artifact']: compressed, 'evidence/' + config['proof']: proof_body}
    source_path = source.relative_to(root).as_posix()
    manifest = {'schema': 'OTERYN_PARTIAL_CAST_SOURCE_IMPORT/v1', 'revision': revision,
                'admission_status': 'source_only_not_active', 'runtime_activation': False,
                'native_identity_allocation': False, 'canonical_selection_changed': False,
                'native_execution_qualified': False, 'input_provider_equivalence': False,
                'base': {'path': 'imports/spells/r28/import-manifest.json', 'sha256': BASE_SHA},
                'source_pins': baseline['source_pins'], 'counts': {'records': len(rows), 'full_spell_candidates': 0},
                'source_keys': sorted(r['registration_key'] for r in rows),
                'source_metadata_path': 'evidence/' + config['artifact'],
                'schemaRefs': baseline['schemaRefs'] + [
                    {'uri': json.loads(body)['$id'], 'path': name, 'sha256': digest(body)}
                    for name, body in schemas.items() if name not in {s['path'] for s in baseline['schemaRefs']}],
                'artifacts': [{'path': name, 'sha256': digest(body), 'bytes': len(body),
                               'sourcePath': source_path + '/' + name.split('/', 1)[1] if name.startswith('evidence/')
                               else 'tools/content-schema/spell-authoring/' + Path(name).name if name not in {s['path'] for s in baseline['schemaRefs']} and name.startswith('schemas/')
                               else 'imports/spells/r28/' + name if name.startswith('schemas/')
                               else 'imports/spells/r28/import-manifest.json',
                               'schemaRefs': [schema['$id']] if name == 'evidence/' + config['artifact'] else []}
                              for name, body in sorted(data.items())],
                'limits': ['Partial source evidence only; full Spell receipts remain BLOCKED.',
                           'No native providers, runtime admission, canonical selection or server wiring are qualified.']}
    return data, manifest


def write_import(root, revision, destination, data, manifest):
    require(revision in CONFIGS and manifest['revision'] == revision
            and manifest['counts'] == {'records': CONFIGS[revision]['records'], 'full_spell_candidates': 0},
            'WRONG_CAST_IMPORT_FAMILY')
    return write_source_only_set(root, destination, 'imports/spells/' + revision, data, manifest)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--revision', choices=sorted(CONFIGS), required=True)
    args = parser.parse_args()
    data, manifest = prepare_import(ROOT, args.revision)
    print(json.dumps(write_import(ROOT, args.revision, ROOT / 'imports/spells' / args.revision, data, manifest)))
