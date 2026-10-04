#!/usr/bin/env python3
"""Import explicitly enumerated source evidence families; never create spell candidates."""
import argparse
from collections import Counter
import gzip
import hashlib
import json
import os
from pathlib import Path
import subprocess

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

from import_source_spell_package import ROOT, bundle_member_path, digest, encoded, require
from source_spell_import_guards import read_base, read_package, write_source_only_set

BASE_SHA = '1c8ed40b00c5457ad408cbfd918147e0e1c197cd5877f7f8e8f18305e9cb7b5f'
FAMILIES = {
    32: ('player-canary-swift-foot-source', None, None, None),
    33: ('', 'source-condition-semantics.jsonl.gz', 'source-condition-semantics-receipt.json', 'tools/content-schema/spell-authoring/source-condition-semantics.schema.json'),
    35: ('', 'source-monk-spender-branches.json.gz', 'source-monk-spender-branches-qualification.json', 'tools/content-schema/spell-authoring/source-monk-spender-branches.schema.json'),
    36: ('', 'source-conjure-helper.json.gz', 'source-conjure-helper-receipt.json', None),
}


def prepare_import(root, revision, source=None, repositories=Path('/workspace/spell-sources')):
    require(revision in FAMILIES, 'EVIDENCE_FAMILY_NOT_ALLOWED')
    suffix, artifact, receipt_name, schema_path = FAMILIES[revision]
    source = source or root / ('docs/reference/spells/r' + str(revision) + '-source-closure') / suffix
    base, base_body, baseline, schemas, _ = read_base(root, BASE_SHA)
    pins = {s['source']: s['revision'] for s in baseline['source_pins']['monster_donors']}
    cache = {}
    def git_bytes(donor, ref):
        require(donor in pins and ref['revision'] == pins[donor] and not Path(ref['path']).is_absolute() and '..' not in Path(ref['path']).parts,
                'EVIDENCE_SOURCE_REVISION_OR_PATH_MISMATCH')
        key = donor, ref['path']
        if key not in cache:
            cache[key] = subprocess.check_output(['git', '-C', str(repositories / donor), 'show', ref['revision'] + ':' + ref['path']],
                                                 env={**os.environ, 'GIT_NO_LAZY_FETCH': '1'})
        body = cache[key]
        require(digest(body) == ref['sha256'], 'EVIDENCE_SOURCE_HASH_MISMATCH')
        if 'git_blob' in ref:
            require(hashlib.sha1(b'blob ' + str(len(body)).encode() + b'\0' + body).hexdigest() == ref['git_blob'], 'EVIDENCE_GIT_BLOB_MISMATCH')
        return body
    callbacks = {r['registration_key']: r for r in map(json.loads, gzip.decompress(
        (base / 'player-source-bundles/source-callback-facts.jsonl.gz').read_bytes()).splitlines())}
    extra_schema_refs = []
    def add_schema(path, body):
        doc = json.loads(body); Draft202012Validator.check_schema(doc)
        schemas[path] = body
        extra_schema_refs.append({'uri': doc.get('$id', path), 'path': path, 'sha256': digest(body)})
        return doc
    if revision == 32:
        packet = read_package(source)
        source_schema = add_schema('evidence/swift-foot-source.schema.json', packet['swift-foot-source.schema.json'])
        ref_schema = add_schema('evidence/source-reference.schema.json', packet['source-reference.schema.json'])
        value, reference, receipt = [json.loads(packet[n]) for n in ('swift-foot-source.json', 'source-reference.json', 'import-summary.json')]
        Draft202012Validator(source_schema).validate(value); Draft202012Validator(ref_schema).validate(reference)
        key = 'canary-main-current/data/scripts/spells/support/swift_foot.lua#1'; callback = callbacks[key]
        require(value['registration_key'] == receipt['registration_key'] == key and receipt['records'] == 1
                and value['candidate_created'] is False and receipt['candidate_created'] is False
                and value['runtime_activation'] is False and value['native_execution_qualified'] is False
                and value['source_sha256'] == receipt['source_sha256'] == callback['source_sha256'] == reference['sha256']
                and value['source_revision'] == receipt['source_revision'] == callback['source_revision'] == reference['revision'], 'SWIFT_SOURCE_IDENTITY_MISMATCH')
        for path, expected in receipt['input_proofs'].items():
            require(path.startswith('r28/') and '..' not in Path(path).parts and digest(
                (base / 'player-source-bundles' / path.split('/', 1)[1]).read_bytes()) == expected, 'SWIFT_BASE_INPUT_PIN_MISMATCH')
        require(json.loads(packet['source-callback-facts.json']) == callback, 'SWIFT_CALLBACK_SOURCE_CHANGED')
        require(packet['source-header.json'] == (base / 'player-source-bundles/canary-main-current/61cf55fe357c2df0/source-header.json').read_bytes(),
                'SWIFT_HEADER_SOURCE_CHANGED')
        raw = git_bytes('canary', reference)
        require(len(raw) == reference['bytes'] and reference['raw_source_distributed'] is False, 'SWIFT_RAW_SOURCE_OR_SIZE_MISMATCH')
        spans = []
        def walk(node):
            if isinstance(node, dict):
                if 'source_span' in node: spans.append(node['source_span'])
                for child in node.values(): walk(child)
            elif isinstance(node, list):
                for child in node: walk(child)
        walk(value['program'])
        lines = raw.splitlines(keepends=True)
        for span in spans:
            require(digest(b''.join(lines[span['start_line'] - 1:span['end_line']])) == span['sha256'], 'SWIFT_STATEMENT_SPAN_HASH_MISMATCH')
        counts = {'source_records': 1, 'source_statement_spans': len(spans), 'full_spell_candidates': 0}
        schema_for_data = source_schema.get('$id', 'evidence/swift-foot-source.schema.json')
        main_name = 'swift-foot-source.json'
    else:
        receipt_body = bundle_member_path(source, receipt_name).read_bytes(); receipt = json.loads(receipt_body)
        compressed = bundle_member_path(source, artifact).read_bytes()
        if revision == 36: schema_path = (source / 'source-conjure-helper.schema.json').relative_to(root).as_posix()
        schema_body = (root / schema_path).read_bytes()
        require(digest(compressed) == receipt.get('gzip_sha256', receipt.get('data_sha256')) and digest(schema_body) == receipt['schema_sha256'], 'EVIDENCE_CONTAINER_OR_SCHEMA_PIN_MISMATCH')
        payload = gzip.decompress(compressed)
        require(digest(payload) == receipt['payload_sha256'] and receipt['runtime_activation'] is False, 'EVIDENCE_PAYLOAD_OR_ACTIVATION_MISMATCH')
        value = [json.loads(line) for line in payload.splitlines()] if revision == 33 else json.loads(payload)
        records = value if isinstance(value, list) else value['records']
        require(len(records) == receipt.get('record_count', receipt.get('records')), 'EVIDENCE_RECORD_COUNT_MISMATCH')
        doc = add_schema('schemas/' + Path(schema_path).name, schema_body)
        registry = Registry().with_resources((s['uri'], Resource.from_contents(json.loads(schemas[s['path']]))) for s in baseline['schemaRefs'] + extra_schema_refs)
        validator = Draft202012Validator(doc, registry=registry)
        if revision in (33, 36):
            for row in records: validator.validate(row)
        else: validator.validate(value)
        packet = {artifact: compressed, receipt_name: receipt_body}
        main_name = artifact; schema_for_data = doc.get('$id', 'schemas/' + Path(schema_path).name)
        require(all(row['runtime_activation'] is False for row in records), 'EVIDENCE_RUNTIME_ACTIVATION_REFUSED')
        if revision == 33:
            for name, expected in [('source-condition-templates.jsonl.gz', receipt['input_packet_sha256']), ('source-condition-templates-receipt.json', receipt['input_receipt_sha256'])]:
                require(digest((root / 'imports/spells/r31' / name).read_bytes()) == expected, 'CONDITION_BASE_R31_PIN_MISMATCH')
            require(digest((base / 'source-mechanics-inventory.json.gz').read_bytes()) == receipt['inventory_sha256'], 'CONDITION_INVENTORY_PIN_MISMATCH')
            old = {r['source_identity']: r for r in map(json.loads, gzip.decompress((root / 'imports/spells/r31/source-condition-templates.jsonl.gz').read_bytes()).splitlines())}
            require(len(records) == len(old) and {r['source_identity'] for r in records} == set(old), 'CONDITION_SEMANTICS_POPULATION_MISMATCH')
            for ref in receipt['source_rules_provenance']: git_bytes(ref['source'], ref)
            for row in records:
                original = old[row['source_identity']]
                require(all(row[k] == original[k] for k in ('source', 'revision', 'path', 'source_sha256', 'constructor_arguments', 'constructor_call_ref'))
                        and row['application_binding_qualified'] is False, 'CONDITION_SEMANTICS_IDENTITY_MISMATCH')
                expected = []
                for call in original['calls']:
                    args = call['arguments']
                    if call['method'] == 'setParameter' and len(args) == 2 and args[0]['kind'] == 'source_symbol':
                        parameter, argument = args[0]['value'], args[1]
                    elif call['method'] == 'setTicks' and len(args) == 1:
                        parameter, argument = 'CONDITION_PARAM_TICKS', args[0]
                    else: continue
                    expected.append((parameter, argument, call['declaration_prefix'], call['source_call_ref']))
                actual = [(a['parameter']['symbol'], a['value'], a['declaration_prefix'], a['source_call_ref']) for a in row['parameter_assignments']]
                require(actual == expected, 'CONDITION_PARAMETER_CALL_LINK_MISMATCH')
            lifetimes = dict(Counter(r['lifetime']['mode'] for r in records)); assignments = sum(len(r['parameter_assignments']) for r in records)
            require(lifetimes == receipt['lifetime_counts'] and assignments == receipt['parameter_assignment_count'], 'CONDITION_SEMANTICS_COUNT_MISMATCH')
            counts = {'source_records': len(records), 'ordered_parameter_assignments': assignments, 'lifetime_counts': lifetimes, 'full_spell_candidates': 0}
        elif revision == 35:
            require(value['runtime_activation'] is False and receipt['source_only'] is True and receipt['source_revision'] == pins['canary'], 'MONK_PROGRAM_SCOPE_MISMATCH')
            for row in records:
                proof = row['source_proof']; callback = callbacks.get(row['registration_key'])
                require(callback is not None and proof['sha256'] == callback['source_sha256'] and proof['revision'] == callback['source_revision'], 'MONK_CALLBACK_IDENTITY_MISMATCH')
                git_bytes('canary', proof)
            for helper in value['helpers'].values(): git_bytes('canary', helper['source_proof'])
            require({r['registration_key'] for r in records} == {r['registration_key'] for r in receipt['records']}, 'MONK_PROGRAM_POPULATION_MISMATCH')
            counts = {'source_records': len(records), 'full_spell_candidates': 0, 'native_formula_ready': 0}
        else:
            for path, expected in receipt['input_proofs'].items(): require(digest((base / 'player-source-bundles' / path).read_bytes()) == expected, 'CONJURE_HELPER_BASE_INPUT_MISMATCH')
            for donor, ref in receipt['helper_proofs'].items(): git_bytes(donor, ref)
            require(len({r['registration_key'] for r in records}) == len(records), 'CONJURE_HELPER_DUPLICATE_LINK')
            require({r['registration_key'] for r in records} == {key for key, cb in callbacks.items() if cb['source_callback_facts']['cast'].get('tier') == 'conjure'},
                    'CONJURE_HELPER_COHORT_MISMATCH')
            for row in records:
                callback = callbacks.get(row['registration_key'])
                require(callback is not None and digest(encoded(callback)) == row['callback_record_sha256']
                        and callback['source_sha256'] == row['source_sha256'] and callback['source_revision'] == row['revision'] == pins[row['source']], 'CONJURE_HELPER_CALLBACK_IDENTITY_MISMATCH')
                require(all(row[k] is False for k in ('result_item_type_qualified', 'effect_enum_value_qualified', 'effect_asset_binding_qualified')), 'CONJURE_HELPER_QUALIFICATION_REFUSED')
                conjure = callback['source_callback_facts']['cast']['conjure']; helper = receipt['helper_proofs'][row['source']]
                fourth = {'kind': 'source_symbol', 'symbol': conjure['effect']} if 'effect' in conjure else {'kind': 'omitted_nil'}
                require(row['source'] == callback['source_callback_facts']['source'] and all(row[k] == conjure[k] for k in ('reagent_item_id', 'result_item_id', 'count'))
                        and row['fourth_argument'] == fourth and row['helper_sha256'] == helper['sha256'] and row['helper_body_sha256'] == helper['body_sha256'],
                        'CONJURE_HELPER_PARAMETERS_MISMATCH')
            statuses = dict(Counter(r['base_status'] for r in records)); fourth = dict(Counter(r['fourth_argument']['kind'] for r in records))
            require(statuses == receipt['status_counts'] and fourth == receipt['fourth_argument_counts'], 'CONJURE_HELPER_COUNT_MISMATCH')
            counts = {'source_records': len(records), 'frozen_base_status_counts': statuses, 'fourth_argument_counts': fourth, 'full_spell_candidates': 0}
    require(source.resolve().is_relative_to(root.resolve()), 'EVIDENCE_SOURCE_OUTSIDE_REPOSITORY')
    source_path = source.resolve().relative_to(root.resolve()).as_posix()
    data = {'evidence/' + name: body for name, body in packet.items()}; data.update(schemas); data['base-r28/import-manifest.json'] = base_body
    def origin(name):
        if name.startswith('evidence/'): return source_path + '/' + name.split('/', 1)[1]
        if name == 'base-r28/import-manifest.json': return 'imports/spells/r28/import-manifest.json'
        return schema_path if name in {s['path'] for s in extra_schema_refs} else 'imports/spells/r28/' + name
    manifest = {'schema': 'OTERYN_BOUNDED_SOURCE_EVIDENCE_IMPORT/v1', 'revision': revision, 'admission_status': 'source_only_not_active',
                'runtime_activation': False, 'native_identity_allocation': False, 'canonical_selection_changed': False,
                'native_execution_qualified': False, 'input_provider_equivalence': False, 'full_spell_candidates': 0,
                'base': {'path': 'imports/spells/r28/import-manifest.json', 'sha256': BASE_SHA, 'snapshot': 'base-r28/import-manifest.json'},
                'source_pins': baseline['source_pins'], 'schemaRefs': baseline['schemaRefs'] + extra_schema_refs,
                'counts': counts, 'source_metadata_path': 'evidence/' + main_name,
                'artifacts': [{'path': name, 'sha256': digest(body), 'bytes': len(body), 'sourcePath': origin(name),
                               'role': 'source_evidence' if name.startswith('evidence/') else 'base_manifest' if name.startswith('base-r28/') else 'reference_schema',
                               'schemaRefs': [schema_for_data] if name == 'evidence/' + main_name else ['evidence/source-reference.schema.json'] if revision == 32 and name == 'evidence/source-reference.json' else []} for name, body in sorted(data.items())],
                'limits': ['Typed source evidence preserves explicit unresolved inputs and bindings; no complete new Spell candidates are admitted.',
                           'Frozen base classification counts remain scoped to their original packet, independently of later candidate overlays.',
                           'Existing imports, canonical selection, native identity allocation and runtime activation remain unchanged.']}
    return data, manifest


def write_import(root, destination, data, manifest):
    require(manifest['revision'] in FAMILIES and manifest['full_spell_candidates'] == manifest['counts']['full_spell_candidates'] == 0, 'EVIDENCE_CANDIDATE_CLAIM_REFUSED')
    return write_source_only_set(root, destination, 'imports/spells/r' + str(manifest['revision']), data, manifest)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--revision', type=int, choices=FAMILIES, required=True)
    parser.add_argument('--source', type=Path)
    parser.add_argument('--source-repositories', type=Path, default=Path('/workspace/spell-sources'))
    args = parser.parse_args()
    data, manifest = prepare_import(ROOT, args.revision, args.source, args.source_repositories)
    print(json.dumps(write_import(ROOT, ROOT / ('imports/spells/r' + str(args.revision)), data, manifest), indent=2))


if __name__ == '__main__': main()
