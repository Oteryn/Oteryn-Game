#!/usr/bin/env python3
"""Import frozen upstream syntax data without executing or admitting source programs."""
import argparse
import base64
from collections import Counter
import copy
import csv
import gzip
import hashlib
import importlib.metadata
import io
import json
from pathlib import Path
import re

from jsonschema import Draft202012Validator

from import_source_spell_package import ROOT, bundle_member_path, digest, require
from source_spell_import_guards import read_base, write_source_only_set

BASE_SHA = '1c8ed40b00c5457ad408cbfd918147e0e1c197cd5877f7f8e8f18305e9cb7b5f'
DEPENDENCIES = {'luaparser': '4.2.0', 'antlr4-python3-runtime': '4.13.2', 'multimethod': '2.1'}
FALSE_FLAGS = ('binding_qualified', 'execution_qualified', 'runtime_activation', 'source_code_activation', 'native_admission', 'render_activation')


def row_identity(row):
    return row['source'], row['revision'], row['path'], row['git_blob'], row['source_sha256'], row['source_bytes']


def schema_validators(schema):
    # Every branch has a distinct const-kind. Validate the unchanged envelope
    # plus the exact closed branch once rather than scan 59 branches per node.
    node_schema = schema['properties']['ast']['anyOf'][1]['properties']['nodes']['items']['oneOf']
    require({r['$ref'] for r in node_schema} == {'#/$defs/' + kind for kind in schema['$defs']}, 'SYNTAX_NODE_SCHEMA_COHORT_MISMATCH')
    for kind, definition in schema['$defs'].items():
        require(definition['properties']['kind'] == {'const': kind}, 'SYNTAX_NODE_SCHEMA_DISCRIMINANT_MISMATCH')
    envelope = copy.deepcopy(schema)
    envelope['properties']['ast']['anyOf'][1]['properties']['nodes']['items'] = {'type': 'object'}
    return Draft202012Validator(envelope), {kind: Draft202012Validator(s) for kind, s in schema['$defs'].items()}


def validate_row(row, inventory, validators):
    envelope, node_validators = validators; envelope.validate(row)
    require(row_identity(row) in inventory, 'SYNTAX_FROZEN_SOURCE_IDENTITY_MISMATCH')
    require(all(row[k] is False for k in FALSE_FLAGS), 'SYNTAX_EXECUTION_CLAIM_REFUSED')
    ast = row['ast']
    if ast is None:
        require(row['node_count'] == row['maximum_depth'] == 0 and row['node_kind_counts'] == {}, 'SYNTAX_ERROR_NODE_CONSERVATION_MISMATCH')
        return
    nodes = ast['nodes']; counts = Counter()
    require(len(nodes) == row['node_count'] and ast['root_node_ref'] == 0 and nodes, 'SYNTAX_NODE_COUNT_OR_ROOT_MISMATCH')
    edges = []
    def references(value):
        if isinstance(value, dict):
            if set(value) == {'node_ref'}: yield value['node_ref']
            else:
                for child in value.values(): yield from references(child)
        elif isinstance(value, list):
            refs = [ref for child in value for ref in references(child)]
            require(refs == sorted(refs), 'SYNTAX_ORDERED_CHILD_REFERENCE_MISMATCH')
            yield from refs
    for index, node in enumerate(nodes):
        require(node['kind'] in node_validators, 'SYNTAX_UNKNOWN_NODE_KIND')
        node_validators[node['kind']].validate(node)
        require(node['id'] == index, 'SYNTAX_NODE_ID_MISMATCH')
        children = list(references(node['fields']))
        require(all(index < ref < len(nodes) for ref in children), 'SYNTAX_CHILD_REFERENCE_MISMATCH')
        edges.append(sorted(children)); counts[node['kind']] += 1
    require(dict(counts) == row['node_kind_counts'], 'SYNTAX_NODE_KIND_CONSERVATION_MISMATCH')
    visited = []; stack = [(0, 0)]; depth = 0
    while stack:
        node, level = stack.pop(); visited.append(node); depth = max(depth, level)
        stack.extend((child, level + 1) for child in reversed(edges[node]))
    require(visited == list(range(len(nodes))) and depth == row['maximum_depth'], 'SYNTAX_TREE_ORDER_OR_DEPTH_MISMATCH')


def verify_current_python_record(distribution):
    entries = list(distribution.files or [])
    record = [e for e in entries if Path(e).name == 'RECORD' and '.dist-info' in Path(e).parent.name]
    require(len(record) == 1, 'SYNTAX_DEPENDENCY_RECORD_ENTRY_MISMATCH')
    rows = list(csv.reader(io.StringIO(Path(distribution.locate_file(record[0])).read_bytes().decode())))
    require(all(len(row) == 3 for row in rows) and len({row[0] for row in rows}) == len(rows), 'SYNTAX_DEPENDENCY_RECORD_SHAPE_MISMATCH')
    module_rows = [row for row in rows if Path(row[0]).suffix == '.py']
    require(module_rows, 'SYNTAX_DEPENDENCY_RECORD_MODULES_MISSING')
    for path, expected_hash, expected_size in module_rows:
        body = Path(distribution.locate_file(path)).read_bytes()
        actual_hash = 'sha256=' + base64.urlsafe_b64encode(hashlib.sha256(body).digest()).rstrip(b'=').decode()
        require(expected_hash == actual_hash and expected_size == str(len(body)), 'SYNTAX_DEPENDENCY_PYTHON_MODULE_INTEGRITY_MISMATCH')
    # Generated bin/luaparser and bin/pygrun contain environment-specific shebangs.
    # The historical full RECORD digest remains generation provenance; current
    # Python module entries are verified against the installation's own RECORD.


def dependency_provenance(receipt):
    require(receipt['dependencies'] == DEPENDENCIES, 'SYNTAX_DEPENDENCY_VERSION_PIN_MISMATCH')
    for name, version in DEPENDENCIES.items():
        require(importlib.metadata.version(name) == version, 'SYNTAX_INSTALLED_DEPENDENCY_VERSION_MISMATCH')
    package = importlib.metadata.distribution('luaparser')
    for key, path in [('upstream_astnodes_sha256', 'luaparser/astnodes.py'), ('upstream_parser_api_sha256', 'luaparser/ast.py'), ('upstream_builder_sha256', 'luaparser/builder.py')]:
        require(digest(Path(package.locate_file(path)).read_bytes()) == receipt[key], 'SYNTAX_UPSTREAM_CODE_HASH_MISMATCH')
    require(set(receipt['dependency_provenance']) == set(DEPENDENCIES), 'SYNTAX_DEPENDENCY_PROVENANCE_COHORT_MISMATCH')
    for name, provenance in receipt['dependency_provenance'].items():
        require(set(provenance) == {'metadata_sha256', 'record_sha256'} and all(isinstance(value, str) and re.fullmatch('[0-9a-f]{64}', value) for value in provenance.values()),
                'SYNTAX_DEPENDENCY_PROVENANCE_HASH_SHAPE_MISMATCH')
        distribution = importlib.metadata.distribution(name)
        metadata = [entry for entry in distribution.files or [] if Path(entry).name == 'METADATA' and '.dist-info' in Path(entry).parent.name]
        require(len(metadata) == 1 and digest(Path(distribution.locate_file(metadata[0])).read_bytes()) == provenance['metadata_sha256'], 'SYNTAX_DEPENDENCY_METADATA_HASH_MISMATCH')
        verify_current_python_record(distribution)


def prepare_import(root, source=None):
    source = source or root / 'docs/reference/spells/r38-source-closure'
    base, base_body, baseline, schemas, _ = read_base(root, BASE_SHA)
    proof_name, artifact = 'source-syntax-receipt.json', 'source-syntax.jsonl.gz'
    proof_body = bundle_member_path(source, proof_name).read_bytes(); proof = json.loads(proof_body)
    compressed = bundle_member_path(source, artifact).read_bytes()
    schema_path = 'tools/content-schema/spell-authoring/source-syntax.schema.json'
    schema_body = (root / schema_path).read_bytes(); schema = json.loads(schema_body)
    require(digest(compressed) == proof['gzip_sha256'] and digest(schema_body) == proof['schema_sha256'], 'SYNTAX_PACKET_OR_SCHEMA_HASH_MISMATCH')
    dependency_provenance(proof)
    pins = {r['source']: r['revision'] for r in baseline['source_pins']['monster_donors']}
    require(proof['source_revisions'] == sorted(pins.values()) and all(proof[k] is False for k in FALSE_FLAGS if k != 'binding_qualified'), 'SYNTAX_PACKET_SOURCE_SCOPE_MISMATCH')
    inventory_body = (base / 'source-mechanics-inventory.json.gz').read_bytes()
    require(digest(inventory_body) == proof['inventory_sha256'], 'SYNTAX_FROZEN_INVENTORY_HASH_MISMATCH')
    files = json.loads(gzip.decompress(inventory_body))['files']
    inventory = {(r['source'], r['revision'], r['path'], r['git_blob'], r['sha256'], r['bytes']) for r in files}
    require(len(inventory) == len(files), 'SYNTAX_FROZEN_SOURCE_IDENTITY_COLLAPSE')
    validators = schema_validators(schema); seen = set(); counts = Counter(); kinds = Counter(); payload_hash = hashlib.sha256()
    maxima = {'depth': 0, 'nodes_per_file': 0, 'source_bytes': 0, 'serialized_row_bytes': 0}
    with gzip.GzipFile(fileobj=io.BytesIO(compressed)) as stream:
        for line in stream:
            payload_hash.update(line); row = json.loads(line); validate_row(row, inventory, validators)
            identity = row_identity(row); require(identity not in seen, 'SYNTAX_DUPLICATE_SOURCE_IDENTITY'); seen.add(identity)
            counts['records'] += 1; counts[row['parse_status']] += 1; counts['nodes'] += row['node_count']; kinds.update(row['node_kind_counts'])
            if counts['records'] % 400 == 0: print(json.dumps({'validated_source_records': counts['records'], 'validated_nodes': counts['nodes']}), flush=True)
            for key, value in [('depth', row['maximum_depth']), ('nodes_per_file', row['node_count']), ('source_bytes', row['source_bytes']), ('serialized_row_bytes', len(line))]: maxima[key] = max(maxima[key], value)
    require(payload_hash.hexdigest() == proof['payload_sha256'] and seen == inventory and len(seen) == proof['record_count'], 'SYNTAX_PAYLOAD_OR_SOURCE_POPULATION_MISMATCH')
    require(dict(counts) == proof['counts'] and dict(kinds) == proof['node_kind_counts'] and maxima == proof['measured_maxima'], 'SYNTAX_AGGREGATE_CONSERVATION_MISMATCH')
    source_path = source.resolve().relative_to(root.resolve()).as_posix(); local_schema = 'schemas/source-syntax.schema.json'; schemas[local_schema] = schema_body
    data = {**schemas, 'base-r28/import-manifest.json': base_body, 'evidence/' + artifact: compressed, 'evidence/' + proof_name: proof_body}
    def origin(path):
        if path.startswith('evidence/'): return source_path + '/' + path.split('/', 1)[1]
        if path == local_schema: return schema_path
        return 'imports/spells/r28/import-manifest.json' if path.startswith('base-r28/') else 'imports/spells/r28/' + path
    manifest = {'schema': 'OTERYN_SOURCE_SYNTAX_IMPORT/v1', 'revision': 38, 'admission_status': 'source_only_not_active',
                'runtime_activation': False, 'native_identity_allocation': False, 'canonical_selection_changed': False, 'native_execution_qualified': False,
                'input_provider_equivalence': False, 'binding_qualified': False, 'execution_qualified': False, 'source_code_activation': False, 'native_admission': False, 'render_activation': False,
                'full_spell_candidates': 0, 'base': {'path': 'imports/spells/r28/import-manifest.json', 'sha256': BASE_SHA, 'snapshot': 'base-r28/import-manifest.json'},
                'source_pins': baseline['source_pins'], 'parser_dependencies': proof['dependencies'], 'dependency_provenance': proof['dependency_provenance'],
                'counts': {**dict(counts), 'source_counts': dict(Counter(r[0] for r in seen)), 'distinct_source_sha256': len({r[4] for r in seen}), 'node_kinds': len(kinds), 'parse_error_counts': {s: counts.get(s, 0) for s in ('syntax_error', 'upstream_builder_error')}, 'full_spell_candidates': 0},
                'source_metadata_path': 'evidence/' + artifact, 'schemaRefs': baseline['schemaRefs'] + [{'uri': schema['$id'], 'path': local_schema, 'sha256': digest(schema_body)}],
                'artifacts': [{'path': p, 'sourcePath': origin(p), 'sha256': digest(b), 'bytes': len(b), 'role': 'source_syntax_evidence' if p.startswith('evidence/') else 'reference_schema' if p.startswith('schemas/') else 'base_manifest',
                               'schemaRefs': [schema['$id']] if p == 'evidence/' + artifact else []} for p, b in sorted(data.items())],
                'limits': ['Full upstream syntax structure is retained as source evidence; decoded strings and source anchors remain explicitly unqualified.', 'Syntax validity does not establish runtime API binding, execution, native admission or rendering.', 'No third-party source files or dependency code are redistributed. Existing imports and canonical selection remain unchanged.']}
    return data, manifest


def write_import(root, destination, data, manifest):
    require(manifest['revision'] == 38 and manifest['full_spell_candidates'] == 0 and all(manifest[k] is False for k in FALSE_FLAGS), 'SYNTAX_ACTIVATION_CLAIM_REFUSED')
    return write_source_only_set(root, destination, 'imports/spells/r38', data, manifest)


def main():
    parser = argparse.ArgumentParser(description=__doc__); parser.add_argument('--source', type=Path); args = parser.parse_args()
    data, manifest = prepare_import(ROOT, args.source)
    print(json.dumps(write_import(ROOT, ROOT / 'imports/spells/r38', data, manifest), indent=2))


if __name__ == '__main__': main()
