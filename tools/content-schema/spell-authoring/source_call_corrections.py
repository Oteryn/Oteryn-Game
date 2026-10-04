#!/usr/bin/env python3
"""Bounded R39 correction overlay; never rewrites either historical source artifact."""
import argparse
import collections
import gzip
import hashlib
import json
import os
import pathlib
import subprocess

from source_mechanics_inventory import lex

INVENTORY_PATH = 'docs/reference/spells/r28-source-closure/source-mechanics-inventory.json.gz'
AST_PATH = 'docs/reference/spells/r38-source-closure/source-syntax.jsonl.gz'
EXPECTED_INVENTORY = 'fd1b42f13ec922452646da49fb03d373b5150450720250ff9a6dcbbc4b95ec18'
EXPECTED_AST = 'e1bba945a8352595c70cdd6e62a12fced06a4109e8365399b786b5b00503be6b'
SCHEMA_PATH = 'tools/content-schema/spell-authoring/source-call-corrections.schema.json'
OUTPUT_PATH = 'docs/reference/spells/r39-source-closure/source-call-corrections.json.gz'
FLAGS = {'runtime_activation': False, 'execution_qualified': False, 'native_admission': False,
         'source_code_activation': False, 'mechanics_completion_claim': False}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode('utf-8')


def identity(row, ast=False):
    return {'source': row['source'], 'revision': row['revision'], 'path': row['path'],
            'git_blob': row['git_blob'], 'sha256': row['source_sha256' if ast else 'sha256']}


def key(value):
    return tuple(value[x] for x in ('source', 'revision', 'path', 'git_blob', 'sha256'))


def build(repo, source_root):
    repo = pathlib.Path(repo)
    inputs = {}
    for name, path, expected in [('inventory', INVENTORY_PATH, EXPECTED_INVENTORY), ('ast', AST_PATH, EXPECTED_AST)]:
        raw = (repo / path).read_bytes()
        if digest(raw) != expected:
            raise ValueError(name + ' frozen gzip SHA mismatch')
        inputs[name] = {'path': path, 'gzip_sha256': digest(raw), 'payload_sha256': digest(gzip.decompress(raw))}
    inventory = json.loads(gzip.decompress((repo / INVENTORY_PATH).read_bytes()))
    ast_counts = {}
    with gzip.open(repo / AST_PATH, 'rt') as stream:
        for line in stream:
            row = json.loads(line)
            if not row['syntax_valid'] or row['parse_status'] != 'parsed':
                raise ValueError('AST row not parsed')
            source_key = key(identity(row, ast=True))
            if source_key in ast_counts:
                raise ValueError('duplicate AST identity')
            counts = row['node_kind_counts']
            ast_counts[source_key] = counts.get('Call', 0) + counts.get('Invoke', 0)
    corrections, reconciliation = [], []
    population = set()
    for row in inventory['files']:
        source_id = identity(row); source_key = key(source_id)
        if source_key in population or source_key not in ast_counts:
            raise ValueError('source population mismatch')
        population.add(source_key)
        rejected = [(index, call) for index, call in enumerate(row['calls']) if call['call_identity'] in ('and', 'or', 'not')]
        if rejected:
            env = dict(os.environ, GIT_NO_LAZY_FETCH='1')
            payload = subprocess.check_output(['git', '-C', str(pathlib.Path(source_root) / row['source']), 'show', row['revision'] + ':' + row['path']], env=env)
            if digest(payload) != row['sha256']:
                raise ValueError('pinned source SHA mismatch')
            tokens = lex(payload.decode('utf-8'))
            sites = collections.defaultdict(list)
            for pos, token in enumerate(tokens[:-1]):
                if token['value'] in ('and', 'or', 'not') and tokens[pos+1]['value'] == '(':
                    sites[(token['line'], token['value'])].append(token['offset'])
            occurrence = collections.Counter()
            for index, call in rejected:
                operator = call['call_identity']; site = (call['line'], operator)
                ordinal = occurrence[site]; occurrence[site] += 1
                if call['category'] != 'unsupported_call_reference' or call['source_order'] != index or ordinal >= len(sites[site]):
                    raise ValueError('frozen false-call reference mismatch')
                corrections.append({'source_identity': source_id, 'frozen_call_index': index,
                                    'frozen_source_order': call['source_order'], 'line': call['line'],
                                    'operator': operator, 'source_character_offset': sites[site][ordinal],
                                    'argument_fact_sha256': digest(canonical(call['arguments'])),
                                    'frozen_call_fact_sha256': digest(canonical(call)),
                                    'reason': 'reserved_lua_operator_misclassified_as_call',
                                    'disposition': 'exclude_from_call_counts_preserve_source_operator_in_ast'})
        old = len(row['calls']); corrected = old - len(rejected); actual = ast_counts[source_key]
        if corrected != actual:
            raise ValueError('per-file call-count mismatch: ' + row['path'])
        reconciliation.append({'source_identity': source_id, 'frozen_call_count': old,
                               'excluded_operator_count': len(rejected), 'corrected_call_count': corrected,
                               'ast_call_invoke_count': actual, 'count_equal': True})
    if population != set(ast_counts):
        raise ValueError('unconserved source identities')
    categories = collections.Counter(c['category'] for row in inventory['files'] for c in row['calls'])
    operator_counts = collections.Counter(c['operator'] for c in corrections)
    counts = {'source_files': len(population), 'affected_source_files': sum(r['excluded_operator_count'] > 0 for r in reconciliation),
              'frozen_calls': sum(r['frozen_call_count'] for r in reconciliation), 'excluded_operator_calls': len(corrections),
              'corrected_calls': sum(r['corrected_call_count'] for r in reconciliation), 'ast_calls_and_invokes': sum(ast_counts.values()),
              'typed_known_calls_unchanged': categories['typed_known_call'],
              'frozen_unsupported_call_references': categories['unsupported_call_reference'],
              'corrected_unsupported_call_references': categories['unsupported_call_reference'] - len(corrections),
              'per_file_count_mismatches': 0, 'operators': dict(operator_counts)}
    if counts != {'source_files': 2346, 'affected_source_files': 23, 'frozen_calls': 50318,
                   'excluded_operator_calls': 32, 'corrected_calls': 50286, 'ast_calls_and_invokes': 50286,
                   'typed_known_calls_unchanged': 38320, 'frozen_unsupported_call_references': 11998,
                   'corrected_unsupported_call_references': 11966, 'per_file_count_mismatches': 0,
                   'operators': {'not': 8, 'or': 11, 'and': 13}}:
        raise ValueError('unexpected historical correction population')
    return {'schema': 'OTERYN_SOURCE_CALL_CORRECTIONS/v1', 'inputs': inputs,
            'fact_hash_encoding': 'utf8_json_sort_keys_compact_separators_ensure_ascii_false',
            'counts': counts, 'corrections': corrections, 'per_file_reconciliation': reconciliation,
            'qualification_limit': 'Per-file counts reconciled; no call-identity bijection, execution, binding, or mechanics qualification asserted.',
            **FLAGS}


def main():
    parser = argparse.ArgumentParser(); parser.add_argument('--repo', default='.'); parser.add_argument('--source-root', default='/workspace/spell-sources')
    args = parser.parse_args(); repo = pathlib.Path(args.repo).resolve()
    data = build(repo, args.source_root)
    import jsonschema
    jsonschema.Draft202012Validator(json.loads((repo / SCHEMA_PATH).read_text())).validate(data)
    payload = canonical(data) + b'\n'; compressed = gzip.compress(payload, mtime=0)
    destination = repo / OUTPUT_PATH; destination.parent.mkdir(parents=True, exist_ok=True); destination.write_bytes(compressed)
    proof = {'schema': 'OTERYN_SOURCE_CALL_CORRECTIONS_RECEIPT/v1', 'gzip_path': OUTPUT_PATH,
             'gzip_sha256': digest(compressed), 'payload_sha256': digest(payload), 'schema_path': SCHEMA_PATH,
             'schema_sha256': digest((repo / SCHEMA_PATH).read_bytes()), 'producer_sha256': digest(pathlib.Path(__file__).read_bytes()),
             'future_scanner_sha256': digest(pathlib.Path(__file__).with_name('source_mechanics_inventory.py').read_bytes()),
             'historical_producer_equality_required': False, 'counts': data['counts'], 'inputs': data['inputs'], **FLAGS}
    destination.with_name('source-call-corrections-receipt.json').write_text(json.dumps(proof, indent=2) + '\n')
    print(json.dumps(proof, indent=2))


if __name__ == '__main__':
    main()
