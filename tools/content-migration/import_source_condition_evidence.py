#!/usr/bin/env python3
"""Import partial Condition evidence without creating full spell candidates."""
import argparse
import ast
from collections import Counter
from fractions import Fraction
import gzip
import json
from pathlib import Path
import subprocess

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

from import_source_spell_package import ROOT, bundle_member_path, digest, require
from source_spell_import_guards import read_base, write_source_only_set

SOURCE = 'docs/reference/spells/r31-source-closure'
BASE_SHA = '1c8ed40b00c5457ad408cbfd918147e0e1c197cd5877f7f8e8f18305e9cb7b5f'
SCHEMA = 'tools/content-schema/spell-authoring/source-condition-template.schema.json'
NORMALIZER = 'tools/content-schema/spell-authoring/source_condition_templates.py'
GZIP = 'source-condition-templates.jsonl.gz'
RECEIPT = 'source-condition-templates-receipt.json'


def argument_normalizer(root, expected_sha):
    # Extract only the existing trusted Python normalization functions. This avoids
    # importing converter globals or changing sys.path and does not execute Lua.
    body = (root / NORMALIZER).read_bytes()
    require(digest(body) == expected_sha, 'NORMALIZER_GENERATION_PIN_MISMATCH')
    tree = ast.parse(body)
    nodes = [node for node in tree.body if isinstance(node, ast.FunctionDef) and node.name in {'scalar', 'argument_fact'}]
    require(len(nodes) == 2, 'NORMALIZER_FUNCTIONS_MISSING')
    namespace = {'ast': ast, 'Fraction': Fraction, 'json': json, 'sha': digest}
    exec(compile(ast.Module(body=nodes, type_ignores=[]), NORMALIZER, 'exec'), namespace)
    return namespace['argument_fact']


def prepare_import(root, source, expected_base_sha=BASE_SHA, repositories=Path('/workspace/spell-sources')):
    base, base_body, baseline, schemas, _ = read_base(root, expected_base_sha)
    receipt_body = bundle_member_path(source, RECEIPT).read_bytes()
    receipt = json.loads(receipt_body)
    compressed = bundle_member_path(source, GZIP).read_bytes()
    schema_body = (root / SCHEMA).read_bytes()
    require(digest(compressed) == receipt['gzip_sha256'] and digest(schema_body) == receipt['schema_sha256'], 'CONDITION_ARTIFACT_PIN_MISMATCH')
    require(receipt['runtime_activation'] is False and receipt['external_sources_used'] is False, 'CONDITION_SOURCE_ONLY_REQUIRED')
    payload = gzip.decompress(compressed)
    require(digest(payload) == receipt['payload_sha256'], 'CONDITION_PAYLOAD_PIN_MISMATCH')
    inventory_path = 'imports/spells/r28/source-mechanics-inventory.json.gz'
    inventory_bytes = (root / inventory_path).read_bytes()
    require(digest(inventory_bytes) == receipt['inventory_sha256'], 'CONDITION_INVENTORY_PIN_MISMATCH')
    inventory = json.loads(gzip.decompress(inventory_bytes))
    inventory_index = {(f['source'], f['path']): f for f in inventory['files']}
    donors = {s['source']: s for s in baseline['source_pins']['monster_donors']}
    require(set(receipt['source_revisions']) == {s['revision'] for s in donors.values()}, 'CONDITION_DONOR_PINS_MISMATCH')
    require(len(receipt['engine_enum_provenance']) == len(donors) and {e['source'] for e in receipt['engine_enum_provenance']} == set(donors),
            'CONDITION_ENUM_POPULATION_MISMATCH')
    for enum in receipt['engine_enum_provenance']:
        require(enum['revision'] == donors[enum['source']]['revision'] and enum['path'] == 'src/creatures/creatures_definitions.hpp', 'CONDITION_ENUM_SOURCE_PIN_MISMATCH')
        header = subprocess.check_output(['git', '-C', str(repositories / enum['source']), 'show', enum['revision'] + ':' + enum['path']])
        require(digest(header) == enum['sha256'], 'CONDITION_ENUM_HASH_MISMATCH')
    schema = json.loads(schema_body)
    Draft202012Validator.check_schema(schema)
    schema_path = 'schemas/source-condition-template.schema.json'
    schema_refs = baseline['schemaRefs'] + [{'uri': schema['$id'], 'path': schema_path, 'sha256': digest(schema_body)}]
    schemas[schema_path] = schema_body
    registry = Registry().with_resources((s['uri'], Resource.from_contents(json.loads(schemas[s['path']]))) for s in schema_refs)
    validator = Draft202012Validator(schema, registry=registry)
    argument_fact = argument_normalizer(root, receipt['exporter_sha256'])
    records = [json.loads(line) for line in payload.splitlines()]
    require(len(records) == receipt['record_count'] == len({r['source_identity'] for r in records}), 'CONDITION_POPULATION_MISMATCH')
    for record in records:
        validator.validate(record)
        file = inventory_index.get((record['source'], record['path']))
        require(file is not None and file['revision'] == record['revision'] == donors[record['source']]['revision']
                and file['sha256'] == record['source_sha256'], 'CONDITION_FILE_IDENTITY_MISMATCH')
        require(record['source_identity'] == record['source'] + '/' + record['path'] + ':' + str(record['declaration_line']), 'CONDITION_DECLARATION_IDENTITY_MISMATCH')
        def lookup(ref):
            require(0 <= ref['inventory_call_index'] < len(file['calls']), 'CONDITION_CALL_INDEX_INVALID')
            call = file['calls'][ref['inventory_call_index']]
            require(call['line'] == ref['line'] and call['source_order'] == ref['source_order'], 'CONDITION_CALL_REFERENCE_MISMATCH')
            return call
        constructor = lookup(record['constructor_call_ref'])
        require(constructor['call_identity'] == 'Condition' and constructor['line'] == record['declaration_line']
                and record['constructor_arguments'] == [argument_fact(a) for a in constructor['arguments']]
                and constructor['arguments'][0].get('name') == record['source_type_constant'], 'CONDITION_CONSTRUCTOR_MISMATCH')
        for fact in record['calls']:
            call = lookup(fact['source_call_ref'])
            require(call['call_identity'] == record['source_variable'] + ':' + fact['method'] and call['line'] == fact['line']
                    and fact['arguments'] == [argument_fact(a) for a in call['arguments']], 'CONDITION_CALL_ARGUMENT_OR_RECEIVER_MISMATCH')
        require(record['appearance_binding_unresolved'] == (record['source_type_constant'] == 'CONDITION_OUTFIT'), 'CONDITION_APPEARANCE_QUALIFICATION_MISMATCH')
        mapped = record['typed_effect'] is not None
        require(mapped == (record['status'] == 'existing_effect_schema_valid_source_only'), 'CONDITION_TEMPLATE_STATUS_MISMATCH')
    status_counts = dict(Counter(r['status'] for r in records))
    require(status_counts == receipt['status_counts'], 'CONDITION_STATUS_CONSERVATION_MISMATCH')
    require(source.resolve().is_relative_to(root.resolve()), 'CONDITION_SOURCE_OUTSIDE_REPOSITORY')
    source_path = source.resolve().relative_to(root.resolve()).as_posix()
    data = {GZIP: compressed, RECEIPT: receipt_body, 'base-r28/import-manifest.json': base_body, **schemas}
    def origin(name):
        if name in (GZIP, RECEIPT): return source_path + '/' + name
        if name == 'base-r28/import-manifest.json': return 'imports/spells/r28/import-manifest.json'
        return SCHEMA if name == schema_path else 'imports/spells/r28/' + name
    manifest = {'schema': 'OTERYN_SOURCE_CONDITION_EVIDENCE_IMPORT/v1', 'admission_status': 'source_only_not_active',
                'runtime_activation': False, 'native_identity_allocation': False, 'canonical_selection_changed': False,
                'native_execution_qualified': False, 'input_provider_equivalence': False,
                'application_binding_qualified': False, 'template_complete': False,
                'base': {'path': 'imports/spells/r28/import-manifest.json', 'sha256': expected_base_sha, 'snapshot': 'base-r28/import-manifest.json'},
                'inventory': {'path': inventory_path, 'sha256': receipt['inventory_sha256']},
                'source_pins': baseline['source_pins'], 'enum_source_provenance': receipt['engine_enum_provenance'], 'schemaRefs': schema_refs,
                'counts': {'source_declarations': len(records), 'structural_effect_prefix_templates': status_counts.get('existing_effect_schema_valid_source_only', 0),
                           'explicit_payload_gaps': status_counts.get('source_condition_payload_gap', 0), 'full_spell_candidates': 0},
                'artifacts': [{'path': name, 'sha256': digest(body), 'bytes': len(body), 'sourcePath': origin(name),
                               'role': 'partial_condition_source_evidence' if name == GZIP else 'condition_source_receipt' if name == RECEIPT else 'base_manifest' if name.startswith('base-r28/') else 'reference_schema',
                               'schemaRefs': [schema['$id']] if name == GZIP else []} for name, body in sorted(data.items())],
                'limits': ['Effect payloads are partial declaration-prefix templates, not complete spells or qualified application bindings.',
                           'Later/conditional mutations, receiver scope and source application hooks remain unqualified.',
                           'Outfit appearance bindings remain unresolved; all explicit gaps and call references are preserved.',
                           'The immutable r28/r29/r30 sets and active content selection, native IDs and runtime remain unchanged.']}
    return data, manifest


def write_import(root, destination, data, manifest):
    require(manifest['template_complete'] is False and manifest['application_binding_qualified'] is False, 'CONDITION_COMPLETENESS_CLAIM_REFUSED')
    records = [json.loads(line) for line in gzip.decompress(data[GZIP]).splitlines()]
    mapped = sum(r['typed_effect'] is not None for r in records)
    require(manifest['schema'] == 'OTERYN_SOURCE_CONDITION_EVIDENCE_IMPORT/v1' and manifest['counts'] == {
        'source_declarations': len(records), 'structural_effect_prefix_templates': mapped,
        'explicit_payload_gaps': len(records) - mapped, 'full_spell_candidates': 0}, 'CONDITION_MANIFEST_COUNTS_INVALID')
    return write_source_only_set(root, destination, 'imports/spells/r31', data, manifest)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, default=ROOT / SOURCE)
    parser.add_argument('--source-repositories', type=Path, default=Path('/workspace/spell-sources'))
    parser.add_argument('--base-sha256', default=BASE_SHA)
    parser.add_argument('--out', type=Path, default=ROOT / 'imports/spells/r31')
    args = parser.parse_args()
    data, manifest = prepare_import(ROOT, args.source, args.base_sha256, args.source_repositories)
    print(json.dumps(write_import(ROOT, args.out, data, manifest), indent=2))


if __name__ == '__main__':
    main()
