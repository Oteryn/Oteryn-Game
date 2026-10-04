"""Seal reviewed source DATA with one shared private schema, without admission.

This operates only on the four unsealed r59–62 producer packets. It preserves
Spell/dependency/header/standard-receipt bytes and adds explicit pending flags
to private qualification metadata. Sealed imports are never rewritten.
"""
import argparse
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
AUTHORING = ROOT / 'tools/content-schema/spell-authoring'
sys.path.insert(0, str(AUTHORING))
from compose_source_complete_schema import compose, SPELL_URI

EXTENSIONS = tuple(AUTHORING / ('source-' + name + '-native-extensions.schema.json')
                   for name in ('state', 'equipment', 'party-summon', 'world-control'))
BASE_SHA = '1c8ed40b00c5457ad408cbfd918147e0e1c197cd5877f7f8e8f18305e9cb7b5f'
FLAGS = {'target_schema_family': 'private_source_complete_v2',
         'authoring_contract_extension_pending': True,
         'source_consumer_implemented': False, 'input_provider_equivalence': False,
         'runtime_activation': False, 'native_execution_qualified': False,
         'native_identity_allocation': False, 'canonical_selection_changed': False}
ROLES = {'spell': SPELL_URI, 'dependencies': 'urn:oteryn:spell-dependencies:candidate:1',
         'receipt': 'urn:oteryn:source-player-bundle-receipt:1'}


def digest(body):
    return hashlib.sha256(body).hexdigest()


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, ensure_ascii=False) + '\n').encode()


def folder(reg):
    return reg.split('/', 1)[0] + '/' + digest(reg.encode())[:16]


def resources():
    base = ROOT / 'imports/spells/r28'
    body = (base / 'import-manifest.json').read_bytes()
    assert digest(body) == BASE_SHA, 'immutable base manifest changed'
    refs = json.loads(body)['schemaRefs']
    result = {}
    for ref in refs:
        body = (base / ref['path']).read_bytes()
        assert digest(body) == ref['sha256'], 'immutable base schema changed'
        if '$id' in json.loads(body):
            result[Path(ref['path']).name] = body
    assert json.loads(result['spell.schema.json']) == json.loads((AUTHORING / 'spell.schema.json').read_bytes()), 'composer base differs from immutable v1'
    spell, _ = compose(EXTENSIONS)
    result['source-spell-complete.schema.json'] = encoded(spell)
    for path in EXTENSIONS:
        result[path.name] = path.read_bytes()
    identities = [json.loads(body)['$id'] for body in result.values()]
    assert len(set(identities)) == len(identities), 'duplicate schema resource identity'
    return result


def seal(revision, snapshot):
    assert revision in range(59, 63)
    assert not (ROOT / f'imports/spells/r{revision}').exists(), 'sealed import already exists'
    packet = ROOT / f'docs/reference/spells/r{revision}-source-closure'
    manifest_path = packet / 'package-manifest.json'
    manifest = json.loads(manifest_path.read_bytes())
    files = {p.relative_to(packet).as_posix(): p.read_bytes()
             for p in packet.rglob('*') if p.is_file() and p != manifest_path}
    assert set(files) == set(manifest['files']), 'producer membership mismatch'
    assert all(digest(body) == manifest['files'][name] for name, body in files.items()), 'producer hashes mismatch'
    protected = {name: body for name, body in files.items()
                 if Path(name).name in ('spell.json', 'dependencies.json', 'catalog.json', 'source-header.json', 'receipt.json')}
    # Replace redundant producer-local schema copies with exact final resources;
    # a local subset sharing the global Spell URI must not survive composition.
    by_uri = {json.loads(body)['$id']: body for body in snapshot.values()}
    for name, body in list(files.items()):
        if not name.endswith('.schema.json'):
            continue
        uri = json.loads(body).get('$id')
        if uri == SPELL_URI:
            del files[name]
        elif uri in by_uri:
            files[name] = by_uri[uri]
    for name, body in snapshot.items():
        files['source-schema-snapshot/' + name] = body
    summary = json.loads(files['import-summary.json'])
    summary.update(FLAGS)
    for row in summary['records_index']:
        if row['status'] != 'CANDIDATE_SCHEMA_VALID':
            continue
        identity = json.loads(files[folder(row['registration_key']) + '/spell.json'])['spell']['identity']
        row.update(FLAGS, candidate_key=identity['key'], candidate_revision=identity['revision'])
    files['import-summary.json'] = encoded(summary)
    audit = json.loads(files['lane-audit.json'])
    audit.update(FLAGS)
    for row in audit['records']:
        if row['status'] == 'CANDIDATE_SCHEMA_VALID':
            row.update(FLAGS)
    files['lane-audit.json'] = encoded(audit)
    for row in summary['records_index']:
        if row['status'] != 'CANDIDATE_SCHEMA_VALID':
            continue
        path = folder(row['registration_key']) + '/projection-receipt.json'
        projection = json.loads(files[path])
        assert projection.get('required_operations_unrepresented', []) == [], 'producer still declares missing operations'
        assert projection.get('source_alias_to_existing_native_profile', False) is False, 'producer aliases an existing profile'
        projection.update(FLAGS, required_operations_unrepresented=[], source_alias_to_existing_native_profile=False)
        files[path] = encoded(projection)
    proof = json.loads(files['projection-proof.json'])
    proof.update(FLAGS, schema_roles=ROLES,
                 composer_sha256=digest((AUTHORING / 'compose_source_complete_schema.py').read_bytes()),
                 sealer_sha256=digest(Path(__file__).read_bytes()),
                 schema_resources=[{'path': 'source-schema-snapshot/' + name,
                                    'uri': json.loads(body)['$id'], 'sha256': digest(body)}
                                   for name, body in sorted(snapshot.items())])
    files['projection-proof.json'] = encoded(proof)
    assert all(files[name] == body for name, body in protected.items()), 'sealer altered source or target DATA'
    # All preceding assertions happen before writing. Remove only superseded
    # unsealed schema files, never source evidence or target model files.
    for name in set(manifest['files']) - set(files):
        assert name.endswith('.schema.json')
        (packet / name).unlink()
    for name, body in files.items():
        path = packet / name
        path.parent.mkdir(parents=True, exist_ok=True)
        if not path.exists() or path.read_bytes() != body:
            path.write_bytes(body)
    manifest['files'] = {name: digest(body) for name, body in sorted(files.items())}
    manifest_path.write_bytes(encoded(manifest))
    return {'revision': revision, 'manifest_sha': digest(manifest_path.read_bytes()),
            'proof_sha': digest(files['projection-proof.json']), 'files': len(files) + 1}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--seal', action='store_true', required=True)
    args = parser.parse_args()
    snapshot = resources()
    print(json.dumps([seal(revision, snapshot) for revision in range(59, 63)], indent=2))


if __name__ == '__main__':
    main()
