#!/usr/bin/env python3
"""Import the qualified R39 lexical correction overlay without rewriting historical facts."""
import argparse
from collections import Counter
import gzip
import hashlib
import io
import json
from pathlib import Path

from jsonschema import Draft202012Validator

from import_source_spell_package import ROOT, bundle_member_path, digest, require
from source_spell_import_guards import read_base, write_source_only_set

BASE_SHA = '1c8ed40b00c5457ad408cbfd918147e0e1c197cd5877f7f8e8f18305e9cb7b5f'
AST_MANIFEST_SHA = '1abb35e9c3fe2e074912d0470780b6c7722a2e641732fa300802188859082388'
SOURCE_SHA = '34300e278d5f00b8cd482324915fc2aaf3da7cdc121c36af77a9a2cab770b563'
FLAGS = ('runtime_activation', 'execution_qualified', 'native_admission', 'source_code_activation', 'mechanics_completion_claim')


def canonical(value): return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()
def identity(row, ast=False): return tuple(row[k] for k in ('source', 'revision', 'path', 'git_blob')) + (row['source_sha256' if ast else 'sha256'],)


def reconcile(value, files, ast_counts):
    inventory = {identity(row): row for row in files}
    require(len(inventory) == len(files) and set(inventory) == set(ast_counts), 'CALL_CORRECTION_SOURCE_POPULATION_MISMATCH')
    actual = {(identity(row['source_identity']), row['frozen_call_index']): row for row in value['corrections']}
    require(len(actual) == len(value['corrections']), 'CALL_CORRECTION_DUPLICATE_REFERENCE')
    expected = set(); exclusions = Counter()
    for source_id, row in inventory.items():
        for index, call in enumerate(row['calls']):
            if call['call_identity'] not in ('and', 'or', 'not'): continue
            expected.add((source_id, index)); correction = actual.get((source_id, index))
            require(correction is not None and call['category'] == 'unsupported_call_reference'
                    and correction['frozen_source_order'] == call['source_order'] == index
                    and correction['line'] == call['line'] and correction['operator'] == call['call_identity']
                    and correction['frozen_call_fact_sha256'] == digest(canonical(call))
                    and correction['argument_fact_sha256'] == digest(canonical(call['arguments']))
                    and correction['source_character_offset'] < row['bytes'], 'CALL_CORRECTION_FROZEN_REFERENCE_MISMATCH')
            exclusions[source_id] += 1
    require(set(actual) == expected, 'CALL_CORRECTION_EXCLUSION_COHORT_MISMATCH')
    reconciliation = {identity(row['source_identity']): row for row in value['per_file_reconciliation']}
    require(len(reconciliation) == len(value['per_file_reconciliation']) and set(reconciliation) == set(inventory), 'CALL_CORRECTION_RECONCILIATION_POPULATION_MISMATCH')
    for key, row in inventory.items():
        frozen = len(row['calls']); corrected = frozen - exclusions[key]
        require(reconciliation[key] == {'source_identity': {k: row[k] for k in ('source', 'revision', 'path', 'git_blob', 'sha256')},
                'frozen_call_count': frozen, 'excluded_operator_count': exclusions[key], 'corrected_call_count': corrected,
                'ast_call_invoke_count': ast_counts[key], 'count_equal': True} and corrected == ast_counts[key], 'CALL_CORRECTION_PER_FILE_COUNT_MISMATCH')
    categories = Counter(call['category'] for row in files for call in row['calls']); excluded = len(expected)
    frozen = sum(len(row['calls']) for row in files)
    counts = {'source_files': len(files), 'affected_source_files': len(exclusions), 'frozen_calls': frozen, 'excluded_operator_calls': excluded,
              'corrected_calls': frozen - excluded, 'ast_calls_and_invokes': sum(ast_counts.values()),
              'typed_known_calls_unchanged': categories['typed_known_call'], 'frozen_unsupported_call_references': categories['unsupported_call_reference'],
              'corrected_unsupported_call_references': categories['unsupported_call_reference'] - excluded, 'per_file_count_mismatches': 0,
              'operators': dict(Counter(row['operator'] for row in value['corrections']))}
    require(counts == value['counts'], 'CALL_CORRECTION_AGGREGATE_COUNT_MISMATCH')
    return counts


def prepare_import(root, source=None):
    source = source or root / 'docs/reference/spells/r39-source-closure'
    base, base_body, baseline, schemas, _ = read_base(root, BASE_SHA)
    proof_name, artifact = 'source-call-corrections-receipt.json', 'source-call-corrections.json.gz'
    proof_body = bundle_member_path(source, proof_name).read_bytes(); proof = json.loads(proof_body)
    compressed = bundle_member_path(source, artifact).read_bytes(); payload = gzip.decompress(compressed); value = json.loads(payload)
    schema_path = 'tools/content-schema/spell-authoring/source-call-corrections.schema.json'
    schema_body = (root / schema_path).read_bytes(); schema = json.loads(schema_body)
    require(digest(compressed) == proof['gzip_sha256'] == SOURCE_SHA and digest(payload) == proof['payload_sha256']
            and digest(schema_body) == proof['schema_sha256'], 'CALL_CORRECTION_QUALIFIED_PACKET_HASH_MISMATCH')
    Draft202012Validator(schema).validate(value)
    require(all(value[k] is False and proof[k] is False for k in FLAGS) and value['inputs'] == proof['inputs'] and value['counts'] == proof['counts'], 'CALL_CORRECTION_PACKET_SCOPE_MISMATCH')
    inventory_body = (base / 'source-mechanics-inventory.json.gz').read_bytes(); inventory_payload = gzip.decompress(inventory_body)
    require(digest(inventory_body) == value['inputs']['inventory']['gzip_sha256'] and digest(inventory_payload) == value['inputs']['inventory']['payload_sha256'], 'CALL_CORRECTION_INVENTORY_PIN_MISMATCH')
    ast_folder = root / 'imports/spells/r38'; ast_manifest_body = (ast_folder / 'import-manifest.json').read_bytes()
    require(digest(ast_manifest_body) == AST_MANIFEST_SHA, 'CALL_CORRECTION_AST_MANIFEST_PIN_MISMATCH')
    ast_manifest = json.loads(ast_manifest_body); ast_body = (ast_folder / 'evidence/source-syntax.jsonl.gz').read_bytes()
    ast_entry = next(e for e in ast_manifest['artifacts'] if e['path'] == 'evidence/source-syntax.jsonl.gz')
    require(digest(ast_body) == ast_entry['sha256'] == value['inputs']['ast']['gzip_sha256'] and len(ast_body) == ast_entry['bytes'], 'CALL_CORRECTION_AST_DATA_PIN_MISMATCH')
    ast_counts = {}; ast_payload_hash = hashlib.sha256()
    with gzip.GzipFile(fileobj=io.BytesIO(ast_body)) as stream:
        for line in stream:
            ast_payload_hash.update(line); row = json.loads(line); key = identity(row, ast=True)
            require(key not in ast_counts and row['parse_status'] == 'parsed', 'CALL_CORRECTION_AST_IDENTITY_OR_STATUS_MISMATCH')
            ast_counts[key] = row['node_kind_counts'].get('Call', 0) + row['node_kind_counts'].get('Invoke', 0)
    require(ast_payload_hash.hexdigest() == value['inputs']['ast']['payload_sha256'], 'CALL_CORRECTION_AST_PAYLOAD_PIN_MISMATCH')
    counts = reconcile(value, json.loads(inventory_payload)['files'], ast_counts)
    local_schema = 'schemas/source-call-corrections.schema.json'; schemas[local_schema] = schema_body
    source_path = source.resolve().relative_to(root.resolve()).as_posix()
    data = {**schemas, 'base-r28/import-manifest.json': base_body, 'base-r38/import-manifest.json': ast_manifest_body,
            'evidence/' + artifact: compressed, 'evidence/' + proof_name: proof_body}
    def origin(path):
        if path.startswith('evidence/'): return source_path + '/' + path.split('/', 1)[1]
        if path == local_schema: return schema_path
        if path.startswith('base-r38/'): return 'imports/spells/r38/import-manifest.json'
        return 'imports/spells/r28/import-manifest.json' if path.startswith('base-r28/') else 'imports/spells/r28/' + path
    manifest = {'schema': 'OTERYN_SOURCE_CALL_CORRECTION_IMPORT/v1', 'revision': 39, 'admission_status': 'source_only_not_active',
                **{k: False for k in FLAGS}, 'native_identity_allocation': False, 'canonical_selection_changed': False, 'native_execution_qualified': False,
                'input_provider_equivalence': False, 'full_spell_candidates': 0,
                'base': {'path': 'imports/spells/r28/import-manifest.json', 'sha256': BASE_SHA, 'snapshot': 'base-r28/import-manifest.json'},
                'source_inputs': {'r38': {'path': 'imports/spells/r38/import-manifest.json', 'sha256': AST_MANIFEST_SHA, 'snapshot': 'base-r38/import-manifest.json'}, **value['inputs']},
                'source_pins': baseline['source_pins'], 'counts': {**counts, 'full_spell_candidates': 0}, 'source_metadata_path': 'evidence/' + artifact,
                'schemaRefs': baseline['schemaRefs'] + [{'uri': schema.get('$id', local_schema), 'path': local_schema, 'sha256': digest(schema_body)}],
                'artifacts': [{'path': p, 'sourcePath': origin(p), 'sha256': digest(b), 'bytes': len(b), 'role': 'source_correction_evidence' if p.startswith('evidence/') else 'reference_schema' if p.startswith('schemas/') else 'base_manifest',
                               'schemaRefs': [schema.get('$id', local_schema)] if p == 'evidence/' + artifact else []} for p, b in sorted(data.items())],
                'limits': ['Only 32 frozen lexical false-call references are excluded; all 38320 typed known calls remain unchanged.',
                           'The overlay reconciles per-file counts against source syntax; it does not establish a call-identity bijection or executable mechanics.',
                           'Earlier facts, Spell candidates, canonical selection and runtime activation remain unchanged.']}
    return data, manifest


def write_import(root, destination, data, manifest):
    require(manifest['revision'] == 39 and manifest['full_spell_candidates'] == 0 and all(manifest[k] is False for k in FLAGS), 'CALL_CORRECTION_ACTIVATION_CLAIM_REFUSED')
    return write_source_only_set(root, destination, 'imports/spells/r39', data, manifest)


def main():
    parser = argparse.ArgumentParser(description=__doc__); parser.add_argument('--source', type=Path); args = parser.parse_args()
    data, manifest = prepare_import(ROOT, args.source)
    print(json.dumps(write_import(ROOT, ROOT / 'imports/spells/r39', data, manifest), indent=2))


if __name__ == '__main__': main()
