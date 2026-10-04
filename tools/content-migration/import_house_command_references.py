#!/usr/bin/env python3
"""Import qualified secondary house-command notes without claiming fresh external verification."""
import argparse
from collections import Counter
import gzip
import hashlib
import json
import os
from pathlib import Path
import subprocess

from jsonschema import Draft202012Validator

from import_source_spell_package import ROOT, bundle_member_path, digest, require
from source_spell_import_guards import read_base, write_source_only_set

BASE_SHA = '1c8ed40b00c5457ad408cbfd918147e0e1c197cd5877f7f8e8f18305e9cb7b5f'
PROOF_SHA = 'efbc57a45bdabe4c35b98f66cd5e4dcd02f96aed79127d86b7c4345d8313c99f'
OPERATIONS = {'aleta grav': 'door_access_list', 'aleta sio': 'guest_access_list', 'aleta som': 'subowner_access_list', 'alana sio': 'kick_character'}


def verify_rows(rows, callbacks, cache, cache_sha):
    selected = {key: row for key, row in callbacks.items() if '/spells/house/' in key}
    actual = {}
    for row in rows:
        source = row['source']; snapshot = 'canary-main-current' if source['source'] == 'canary' else 'crystal-summer-current'
        key = snapshot + '/' + source['path'] + '#1'; callback = selected.get(key)
        require(key not in actual and callback is not None and source['sha256'] == callback['source_sha256'] and source['revision'] == callback['source_revision']
                and source['git_blob'] == callback['source_callback_facts']['blob'] and row['name'] == callback['source_callback_facts']['name']
                and row['words'] == callback['source_callback_facts']['registrar']['words'] and row['source_identity'] == source['source'] + '/' + source['path']
                and row['operation'] == OPERATIONS[row['words']], 'HOUSE_REFERENCE_SOURCE_JOIN_MISMATCH')
        reference = row['external_reference']
        require(reference['cache_path'] == 'docs/reference/tibia-manual/houses.md' and reference['cache_sha256'] == reference['content_sha256'] == cache_sha
                and reference['quote'] in cache and row['words'].casefold() in reference['quote'].casefold(), 'HOUSE_REFERENCE_SECONDARY_QUOTE_MISMATCH')
        require(reference['original_manual_text_verified'] is False and row['fresh_wiki_verified'] is False and row['binding_qualified'] is False and row['runtime_activation'] is False,
                'HOUSE_REFERENCE_VERIFICATION_ESCALATION_REFUSED')
        actual[key] = row
    require(set(actual) == set(selected), 'HOUSE_REFERENCE_SOURCE_POPULATION_MISMATCH')


def prepare_import(root, source=None, repositories=Path('/workspace/spell-sources')):
    source = source or root / 'docs/reference/spells/r43-source-enrichment'
    base, base_body, baseline, schemas, _ = read_base(root, BASE_SHA)
    proof_name, artifact = 'house-command-reference-receipt.json', 'house-command-references.jsonl.gz'
    proof_body = bundle_member_path(source, proof_name).read_bytes(); proof = json.loads(proof_body)
    require(digest(proof_body) == PROOF_SHA, 'HOUSE_REFERENCE_QUALIFIED_PROOF_MISMATCH')
    compressed = bundle_member_path(source, artifact).read_bytes(); payload = gzip.decompress(compressed)
    schema_path = 'tools/content-schema/spell-authoring/house-command-reference.schema.json'; schema_body = (root / schema_path).read_bytes(); schema = json.loads(schema_body)
    require(digest(compressed) == proof['gzip_sha256'] and digest(payload) == proof['payload_sha256'] and digest(schema_body) == proof['schema_sha256'], 'HOUSE_REFERENCE_PACKET_HASH_MISMATCH')
    pins = {r['source']: r['revision'] for r in baseline['source_pins']['monster_donors']}
    require(proof['source_revisions'] == sorted(pins.values()) and proof['runtime_activation'] is False and proof['fresh_wiki_verified'] is False and proof['original_manual_provenance_verified'] is False,
            'HOUSE_REFERENCE_PACKET_VERIFICATION_REFUSED')
    cache_path = 'docs/reference/tibia-manual/houses.md'; cache_body = bundle_member_path(root, cache_path).read_bytes(); cache_sha = digest(cache_body)
    require(cache_path == proof['cache_path'] and cache_sha == proof['cache_sha256'], 'HOUSE_REFERENCE_CACHE_PIN_MISMATCH')
    rows = [json.loads(line) for line in payload.splitlines()]; validator = Draft202012Validator(schema)
    for row in rows: validator.validate(row)
    callbacks = {r['registration_key']: r for r in map(json.loads, gzip.decompress((base / 'player-source-bundles/source-callback-facts.jsonl.gz').read_bytes()).splitlines())}
    verify_rows(rows, callbacks, cache_body.decode(), cache_sha)
    verified = set()
    for row in rows:
        for ref in (row['source'], row['upstream_comparison']['house_cpp_provenance']):
            donor = ref['source']; require(donor in pins and ref['revision'] == pins[donor] and not Path(ref['path']).is_absolute() and '..' not in Path(ref['path']).parts, 'HOUSE_REFERENCE_GIT_SOURCE_SCOPE_MISMATCH')
            key = donor, ref['path']
            if key in verified: continue
            body = subprocess.check_output(['git', '-C', str(repositories / donor), 'show', ref['revision'] + ':' + ref['path']], env={**os.environ, 'GIT_NO_LAZY_FETCH': '1'})
            require(len(body) == ref['bytes'] and digest(body) == ref['sha256'] and hashlib.sha1(b'blob ' + str(len(body)).encode() + b'\0' + body).hexdigest() == ref['git_blob'], 'HOUSE_REFERENCE_GIT_SOURCE_HASH_MISMATCH')
            verified.add(key)
    counts = {'source_records': len(rows), 'command_count': len({r['words'] for r in rows}), 'source_counts': dict(Counter(r['source']['source'] for r in rows)),
              'fresh_wiki_verified_records': 0, 'original_manual_verified_records': 0, 'full_spell_candidates': 0}
    require(counts['source_records'] == proof['record_count'] and counts['command_count'] == proof['command_count'], 'HOUSE_REFERENCE_COUNT_CONSERVATION_MISMATCH')
    local_schema = 'schemas/house-command-reference.schema.json'; schemas[local_schema] = schema_body; source_path = source.relative_to(root).as_posix()
    data = {**schemas, 'base-r28/import-manifest.json': base_body, 'evidence/' + artifact: compressed, 'evidence/' + proof_name: proof_body}
    def origin(path):
        if path.startswith('evidence/'): return source_path + '/' + path.split('/', 1)[1]
        if path == local_schema: return schema_path
        return 'imports/spells/r28/import-manifest.json' if path.startswith('base-r28/') else 'imports/spells/r28/' + path
    manifest = {'schema': 'OTERYN_HOUSE_COMMAND_REFERENCE_IMPORT/v1', 'revision': 43, 'admission_status': 'source_only_not_active',
                'runtime_activation': False, 'fresh_wiki_verified': False, 'original_manual_provenance_verified': False, 'binding_qualified': False,
                'source_override': False, 'native_identity_allocation': False, 'canonical_selection_changed': False, 'native_execution_qualified': False,
                'input_provider_equivalence': False, 'execution_qualified': False, 'full_spell_candidates': 0,
                'base': {'path': 'imports/spells/r28/import-manifest.json', 'sha256': BASE_SHA, 'snapshot': 'base-r28/import-manifest.json'},
                'source_pins': baseline['source_pins'], 'reference_inputs': [{'path': cache_path, 'sha256': cache_sha, 'scope': 'oteryn_written_secondary_notes_original_manual_unverified'}],
                'counts': counts, 'source_metadata_path': 'evidence/' + artifact,
                'schemaRefs': baseline['schemaRefs'] + [{'uri': schema['$id'], 'path': local_schema, 'sha256': digest(schema_body)}],
                'artifacts': [{'path': p, 'sourcePath': origin(p), 'sha256': digest(b), 'bytes': len(b), 'role': 'supplemental_secondary_reference' if p.startswith('evidence/') else 'reference_schema' if p.startswith('schemas/') else 'base_manifest',
                               'schemaRefs': [schema['$id']] if p == 'evidence/' + artifact else []} for p, b in sorted(data.items())],
                'limits': ['Quotes are cached Oteryn-written secondary notes; no fresh Wiki or original official manual verification is claimed.',
                           'Premium alignment, full callback guards, door geometry and runtime behavior remain unqualified.',
                           'Source headers, Spell candidates, canonical selection and runtime activation remain unchanged.']}
    return data, manifest


def write_import(root, destination, data, manifest):
    require(manifest['revision'] == 43 and manifest['full_spell_candidates'] == 0 and all(manifest[k] is False for k in ('fresh_wiki_verified', 'original_manual_provenance_verified', 'binding_qualified', 'source_override')), 'HOUSE_REFERENCE_VERIFICATION_CLAIM_REFUSED')
    return write_source_only_set(root, destination, 'imports/spells/r43', data, manifest)


def main():
    parser = argparse.ArgumentParser(description=__doc__); parser.add_argument('--source', type=Path); parser.add_argument('--source-repositories', type=Path, default=Path('/workspace/spell-sources')); args = parser.parse_args()
    data, manifest = prepare_import(ROOT, args.source, args.source_repositories); print(json.dumps(write_import(ROOT, ROOT / 'imports/spells/r43', data, manifest), indent=2))


if __name__ == '__main__': main()
