"""Archive reviewed typed source programs, without admitting executable Spells."""
import argparse
import copy
import gzip
import json
from pathlib import Path

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

from import_source_spell_package import ROOT, bundle_member_path, digest, require
from source_spell_import_guards import read_base, write_source_only_set

BASE_SHA = '1c8ed40b00c5457ad408cbfd918147e0e1c197cd5877f7f8e8f18305e9cb7b5f'
FALSE_FLAGS = ('runtime_activation', 'native_execution_qualified', 'native_identity_allocation',
               'canonical_selection_changed', 'input_provider_equivalence')
CONFIGS = {
    49: {'folder': 'docs/reference/spells/r49-source-closure', 'records': 21,
         'artifact': 'source-cast-programs.jsonl.gz', 'proof': 'source-cast-program-proof.json',
         'schema': 'tools/content-schema/spell-authoring/source-cast-programs.schema.json', 'jsonl': True,
         'receipt': 'source-cast-program-receipt.json',
         'packet_sha': '1c0a5592a0e5523d0c0479049db3931dbef0bcadf7c3c6f29dfd14cdae182298',
         'proof_sha': '3c3e746f3e09f0eaed8788c196d98a2a4f4f12461031f71123d43b2274f8b2d6',
         'receipt_sha': '49650f654997313ee302b6400dd3bc44eaa9c4feaa031ef4f059b3644813e81d'},
    50: {'folder': 'docs/reference/spells/r50-source-closure', 'records': 6,
         'artifact': 'source-monk-formula-library.json.gz',
         'proof': 'source-monk-formula-library-qualification.json',
         'schema': 'tools/content-schema/spell-authoring/source-monk-formula-library.schema.json',
         'packet_sha': '02d49418ebff9f42d1b7e546f305997616d6822f643de20fa1aa62a82157aaa4',
         'proof_sha': 'c562c1319f97ae4172095d2efb0e8f49c3ac5383c98fb6a7efca98bc60208078'},
    51: {'folder': 'docs/reference/spells/r51-source-closure', 'records': 175,
         'artifact': 'source-monster-slot-semantics.json.gz', 'proof': 'source-monster-slot-semantics-receipt.json',
         'schema': 'tools/content-schema/monster-authoring/source-monster-slot-semantics.schema.json',
         'packet_sha': 'ff5256a1d359cfb7744be4fdadb8ece0edc2c906fd8d35a57d21f39cb5df50e8',
         'proof_sha': 'c2ba511dc99e3490268a302af01b88c7659a9617583a9bbb01aed5ac08d336e7'},
}


def require_source_only(value, label):
    require(all(value[k] is False for k in FALSE_FLAGS), label + '_ACTIVATION_REFUSED')
    require(value['full_spell_candidates'] == 0, label + '_FULL_SPELL_PROMOTION_REFUSED')


def verify_input_proofs(root, proofs):
    require(proofs, 'SOURCE_INPUT_PROOFS_MISSING')
    for name, expected in proofs.items():
        require(name.startswith('imports/spells/') and not Path(name).is_absolute()
                and '..' not in Path(name).parts, 'SOURCE_INPUT_PATH_REFUSED')
        require(digest(bundle_member_path(root, name).read_bytes()) == expected, 'SOURCE_INPUT_PIN_MISMATCH:' + name)


def prepare_import(root, revision, source=None):
    require(revision in CONFIGS, 'UNSUPPORTED_SOURCE_PROGRAM_FAMILY')
    config = CONFIGS[revision]
    require(all(key in config for key in ['artifact', 'proof', 'schema', 'packet_sha', 'proof_sha']), 'SOURCE_PROGRAM_REVIEW_PENDING')
    source = root / config['folder'] if source is None else source
    require(source.resolve().is_relative_to(root.resolve()), 'SOURCE_OUTSIDE_REPOSITORY')
    base, base_body, baseline, schemas, _ = read_base(root, BASE_SHA)
    artifact, proof_name = config['artifact'], config['proof']
    packet = bundle_member_path(source, artifact).read_bytes()
    proof_body = bundle_member_path(source, proof_name).read_bytes()
    require(digest(packet) == config['packet_sha'], 'SOURCE_PROGRAM_REVIEW_PIN_MISMATCH')
    require(digest(proof_body) == config['proof_sha'], 'SOURCE_PROGRAM_PROOF_REVIEW_PIN_MISMATCH')
    proof = json.loads(proof_body)
    payload = gzip.decompress(packet)
    schema_body = (root / config['schema']).read_bytes()
    require(digest(packet) == proof['gzip_sha256'] and digest(payload) == proof['payload_sha256']
            and digest(schema_body) == proof['schema_sha256'], 'SOURCE_PROGRAM_PACKET_OR_SCHEMA_PIN_MISMATCH')
    verify_input_proofs(root, proof['input_proofs'])
    require_source_only(proof, 'SOURCE_PROGRAM_PROOF')
    schema = json.loads(schema_body)
    resources = [(ref['uri'], Resource.from_contents(json.loads(schemas[ref['path']]))) for ref in baseline['schemaRefs']]
    extra_schemas = {}
    for path, expected in proof.get('schema_proofs', {}).items():
        require(path.startswith('tools/content-schema/') and '..' not in Path(path).parts, 'SOURCE_SCHEMA_PATH_REFUSED')
        body = bundle_member_path(root, path).read_bytes()
        require(digest(body) == expected, 'SOURCE_PROGRAM_EXTRA_SCHEMA_PIN_MISMATCH')
        definition = json.loads(body)
        extra_schemas['schemas/' + Path(path).name] = (path, body)
        if revision == 51 and definition['$id'] == 'urn:oteryn:source-syntax:1':
            definition = copy.deepcopy(definition)
            definition['properties']['ast']['anyOf'][1]['properties']['nodes']['items'] = {'type': 'object'}
        resources.append((definition['$id'], Resource.from_contents(definition)))
    resources.append((schema['$id'], Resource.from_contents(schema)))
    registry = Registry().with_resources(resources)
    document = [json.loads(line) for line in payload.splitlines()] if config.get('jsonl') else json.loads(payload)
    if revision == 49:
        envelope = copy.deepcopy(schema)
        envelope['properties']['instructions']['items'] = {'type': 'object'}
        validator = Draft202012Validator(envelope, registry=registry)
        instruction_validators = {item['properties']['source_kind']['const']: Draft202012Validator(item, registry=registry)
                                  for item in schema['properties']['instructions']['items']['oneOf']}
        for row in document:
            validator.validate(row)
            for instruction in row['instructions']:
                require(instruction.get('source_kind') in instruction_validators, 'CAST_PROGRAM_UNKNOWN_INSTRUCTION_KIND')
                instruction_validators[instruction['source_kind']].validate(instruction)
    else:
        Draft202012Validator(schema, registry=registry).validate(document)
    if revision == 51:
        syntax_schema = json.loads(extra_schemas['schemas/source-syntax.schema.json'][1])
        node_validators = {kind: Draft202012Validator(definition, registry=registry) for kind, definition in syntax_schema['$defs'].items()}
        for program in document['source_programs']:
            for node in program['source_syntax']['ast']['nodes']:
                require(node.get('kind') in node_validators, 'MONSTER_PROGRAM_UNKNOWN_SYNTAX_KIND')
                node_validators[node['kind']].validate(node)
    rows = verify_population(root, revision, document, proof, base, baseline)
    proof_count = proof['formula_count'] if revision == 50 else proof['records']
    require(len(rows) == config['records'] == proof_count, 'SOURCE_PROGRAM_POPULATION_MISMATCH')
    local_schema = 'schemas/' + Path(config['schema']).name
    schemas[local_schema] = schema_body
    schemas.update({name: body for name, (_, body) in extra_schemas.items()})
    source_path = source.resolve().relative_to(root.resolve()).as_posix()
    data = {**schemas, 'base-r28/import-manifest.json': base_body,
            'source-programs/' + artifact: packet, 'source-programs/' + proof_name: proof_body}
    if 'receipt' in config:
        receipt_body = bundle_member_path(source, config['receipt']).read_bytes()
        require(digest(receipt_body) == config['receipt_sha'], 'SOURCE_PROGRAM_RECEIPT_REVIEW_PIN_MISMATCH')
        receipt = json.loads(receipt_body)
        require(receipt['records'] == len(rows) and receipt['complete_spell_candidates'] == 0
                and all(receipt[k] is False for k in ['runtime_activation', 'native_execution_qualified', 'canonical_selection_changed'])
                and receipt['artifacts'] == {artifact: digest(packet), proof_name: digest(proof_body)}, 'CAST_PROGRAM_RECEIPT_MISMATCH')
        data['source-programs/' + config['receipt']] = receipt_body
    def origin(name):
        if name.startswith('source-programs/'):
            return source_path + '/' + name.split('/', 1)[1]
        if name in extra_schemas:
            return extra_schemas[name][0]
        if name == local_schema:
            return config['schema']
        return 'imports/spells/r28/import-manifest.json' if name.startswith('base-r28/') else 'imports/spells/r28/' + name
    counts = {'source_records': len(rows), 'full_spell_candidates': 0, 'executable_spell_count': 0}
    manifest = {'schema': 'OTERYN_TYPED_SOURCE_PROGRAM_IMPORT/v1', 'revision': revision,
                'admission_status': 'source_only_not_active', **{k: False for k in FALSE_FLAGS},
                'full_spell_candidates': 0, 'executable_spell_count': 0,
                'base': {'path': 'imports/spells/r28/import-manifest.json', 'sha256': BASE_SHA, 'snapshot': 'base-r28/import-manifest.json'},
                'source_pins': baseline['source_pins'], 'counts': counts,
                'source_metadata_path': 'source-programs/' + artifact,
                'qualification_proof_sha256': config['proof_sha'],
                'schemaRefs': baseline['schemaRefs'] + [{'uri': json.loads(body)['$id'], 'path': name, 'sha256': digest(body)}
                    for name, body in sorted(schemas.items()) if name not in {ref['path'] for ref in baseline['schemaRefs']}],
                'artifacts': [{'path': name, 'sourcePath': origin(name), 'sha256': digest(body), 'bytes': len(body),
                               'schemaRefs': [schema['$id']] if name == 'source-programs/' + artifact else [],
                               'role': 'source_program' if name.startswith('source-programs/') else 'reference_schema' if name.startswith('schemas/') else 'base_manifest'}
                              for name, body in sorted(data.items())],
                'limits': ['Typed source program data only; no complete Spell, active identity or native execution is admitted.',
                           'Historical source statuses and schemas remain immutable; provider and runtime equivalence are unqualified.']}
    return data, manifest


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()


def jsonl(root, path, selected=None):
    with gzip.open(bundle_member_path(root, path), 'rt') as stream:
        rows = (json.loads(line) for line in stream)
        return [row for row in rows if selected is None or (row['source'], row['path']) in selected]


def unique_index(rows, key, code):
    mapping = {key(row): row for row in rows}
    require(len(mapping) == len(rows), code)
    return mapping


def verify_player_rows(rows, base, baseline):
    capture = unique_index(jsonl(base, 'player-source-bundles/source-callback-facts.jsonl.gz'),
                           lambda row: row['registration_key'], 'BASE_CAPTURE_DUPLICATE')
    keys = unique_index(rows, lambda row: row['registration_key'], 'SOURCE_PROGRAM_DUPLICATE_REGISTRATION')
    for reg, row in keys.items():
        require(reg in capture, 'SOURCE_PROGRAM_REGISTRATION_UNKNOWN')
        fact = capture[reg]
        folder = base / 'player-source-bundles' / reg.split('/', 1)[0] / digest(reg.encode())[:16]
        require(json.loads((folder / 'receipt.json').read_bytes())['status'] == 'BLOCKED', 'SOURCE_PROGRAM_BASE_NOT_BLOCKED')
        require(row['source_sha256'] == fact['source_sha256'] and row['source_revision'] == fact['source_revision']
                == baseline['source_pins']['player_snapshots'][reg.split('/', 1)[0]], 'SOURCE_PROGRAM_SOURCE_IDENTITY_MISMATCH')
        require(row['native_execution_qualified'] is False, 'SOURCE_PROGRAM_NATIVE_QUALIFICATION_REFUSED')
    return capture


def verify_population(root, revision, document, proof, base, baseline):
    if revision == 49:
        rows = document
        captures = verify_player_rows(rows, base, baseline)
        selected = {(row['registration_key'].split('-')[0], row['source_file']) for row in rows}
        syntax = unique_index(jsonl(root, 'imports/spells/r38/evidence/source-syntax.jsonl.gz', selected),
                              lambda row: (row['source'], row['path']), 'SOURCE_SYNTAX_DUPLICATE')
        require(set(proof['registration_keys']) == {row['registration_key'] for row in rows}, 'SOURCE_PROGRAM_PROOF_REGISTRATION_MISMATCH')
        for row in rows:
            reg = row['registration_key']
            fact = captures[reg]
            ast = syntax[(reg.split('-')[0], row['source_file'])]
            require(row['source_file'] == reg.split('/', 1)[1].rsplit('#', 1)[0]
                    and ast['revision'] == row['source_revision'] and ast['source_sha256'] == row['source_sha256']
                    and row['syntax']['record_sha256'] == digest(canonical(ast))
                    and row['syntax']['node_count'] == ast['node_count']
                    and row['callback_capture']['record_sha256'] == digest(canonical(fact)), 'CAST_PROGRAM_SOURCE_JOIN_MISMATCH')
            nodes = ast['ast']['nodes']
            instructions = unique_index(row['instructions'], lambda i: i['source_node_ref'], 'CAST_PROGRAM_DUPLICATE_INSTRUCTION')
            def transform(value):
                if isinstance(value, dict):
                    if set(value) == {'node_ref'}:
                        ref = value['node_ref']
                        return {'instruction_ref' if ref in instructions else 'syntax_node_ref': ref}
                    return {k: transform(v) for k, v in value.items()}
                return [transform(v) for v in value] if isinstance(value, list) else value
            for ref, instruction in instructions.items():
                require(0 <= ref < len(nodes) and instruction['source_kind'] == nodes[ref]['kind']
                        and instruction['operands'] == transform(nodes[ref]['fields']), 'CAST_PROGRAM_AST_OPERAND_MISMATCH')
            require(row['complete_spell_candidate'] is False and row['runtime_activation'] is False
                    and row['canonical_selection_changed'] is False, 'CAST_PROGRAM_PROMOTION_REFUSED')
        return rows
    if revision == 50:
        rows = document['formula_definitions']
        captures = verify_player_rows(rows, base, baseline)
        require(len(document['helpers']) == 1 and document['complete_spell_candidates'] == 0, 'MONK_LIBRARY_HELPER_POPULATION_MISMATCH')
        helper = document['helpers'][0]
        reference = helper['program_reference']
        require(reference['path'] == 'imports/spells/r28/source-formula-evidence.jsonl.gz' and reference['field'] == 'tier_program'
                and digest((root / reference['path']).read_bytes()) == reference['gzip_sha256'], 'MONK_HELPER_ARCHIVE_MISMATCH')
        evidence = unique_index(jsonl(root, reference['path']), lambda row: row['registration_key'], 'MONK_FORMULA_EVIDENCE_DUPLICATE')
        require(digest(canonical(evidence[reference['registration_key']]['tier_program'])) == reference['program_sha256'], 'MONK_HELPER_PROGRAM_MISMATCH')
        def link(tree):
            if set(tree) == {'fn', 'args'}:
                require(tree['fn'] == 'flat_damage_healing' and tree['args'] == [{'var': 'level'}], 'MONK_FORMULA_UNKNOWN_HELPER')
                return {'helper_ref': helper['key'], 'args': [{'var': 'level'}]}
            if set(tree) == {'op', 'args'}:
                return {'op': tree['op'], 'args': [link(arg) for arg in tree['args']]}
            return tree
        for row in rows:
            fact = captures[row['registration_key']]
            formula = fact['source_callback_facts']['combats'][0]['callbacks'][0]['formula']
            require(row['capture_fact_sha256'] == digest(canonical(fact)) and row['helper_refs'] == [helper['key']]
                    and row['expressions'] == {bound: link(formula[bound]) for bound in ['minimum', 'maximum']}
                    and row['complete_spell_candidate'] is False, 'MONK_FORMULA_CAPTURE_OR_LINK_MISMATCH')
        require({row['registration_key'] for row in proof['records']} == {row['registration_key'] for row in rows}
                and all(row['spell_status'] == 'BLOCKED' for row in proof['records']), 'MONK_FORMULA_PROOF_POPULATION_MISMATCH')
        return rows
    rows = document['slots']
    links = unique_index(jsonl(root, 'imports/spells/r37/evidence/unresolved-monster-spell-links.jsonl.gz'),
                         lambda row: canonical(row['slot_identity']), 'MONSTER_LINK_DUPLICATE')
    keys = unique_index(rows, lambda row: canonical(row['slot_identity']), 'MONSTER_PROGRAM_DUPLICATE_SLOT')
    require(set(keys) == set(links) and len(document['source_programs']) == 141, 'MONSTER_PROGRAM_SOURCE_POPULATION_MISMATCH')
    selected = {(p['source_syntax']['source'], p['source_syntax']['path']) for p in document['source_programs']}
    syntax = unique_index(jsonl(root, 'imports/spells/r38/evidence/source-syntax.jsonl.gz', selected),
                          lambda row: (row['source'], row['path']), 'SOURCE_SYNTAX_DUPLICATE')
    for program in document['source_programs']:
        ast = program['source_syntax']
        require(ast == syntax[(ast['source'], ast['path'])] and program['runtime_activation'] is False
                and program['binding_qualified'] is False, 'MONSTER_PROGRAM_AST_SOURCE_MISMATCH')
    for key, row in keys.items():
        old = links[key]
        require(all(row[field] == old[field] for field in ['source', 'monster', 'original_slot_sha256', 'source_parameters'])
                and row['original_conversion_status'] == 'unresolved_semantics'
                and row['runtime_activation'] is False and row['native_provider_qualified'] is False
                and row['full_slot_projection_complete'] is False, 'MONSTER_SLOT_SOURCE_OR_PROJECTION_MISMATCH')
        index = row['source_program_index']
        registered = old['registered_source']
        if registered:
            require(type(index) is int and 0 <= index < len(document['source_programs']), 'MONSTER_PROGRAM_REFERENCE_INVALID')
            ast = document['source_programs'][index]['source_syntax']
            require(ast['source'] == old['source'] and all(registered[k] == ast[a] for k, a in [('revision', 'revision'), ('path', 'path'), ('sha256', 'source_sha256'), ('git_blob', 'git_blob')]), 'MONSTER_SLOT_PROGRAM_BINDING_MISMATCH')
        else:
            require(index is None and row['inline_semantics'] is not None, 'MONSTER_SLOT_INLINE_BINDING_MISMATCH')
    return rows


def write_import(root, revision, destination, data, manifest):
    require(revision in CONFIGS and manifest['revision'] == revision, 'WRONG_SOURCE_PROGRAM_FAMILY')
    require_source_only(manifest, 'SOURCE_PROGRAM_IMPORT')
    config = CONFIGS[revision]
    require(manifest['executable_spell_count'] == 0 and manifest['counts'] == {'source_records': config['records'], 'full_spell_candidates': 0, 'executable_spell_count': 0}, 'SOURCE_PROGRAM_IMPORT_COUNT_MISMATCH')
    require(manifest['qualification_proof_sha256'] == config['proof_sha']
            and digest(data['source-programs/' + config['proof']]) == config['proof_sha']
            and digest(data['source-programs/' + config['artifact']]) == config['packet_sha'], 'SOURCE_PROGRAM_IMPORT_PROOF_PIN_MISMATCH')
    return write_source_only_set(root, destination, 'imports/spells/r' + str(revision), data, manifest)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--revision', type=int, choices=sorted(CONFIGS), required=True)
    parser.add_argument('--source', type=Path)
    parser.add_argument('--prepare-only', action='store_true')
    args = parser.parse_args()
    data, manifest = prepare_import(ROOT, args.revision, args.source)
    print(json.dumps({'artifacts': len(data), 'counts': manifest['counts']} if args.prepare_only else
                     write_import(ROOT, args.revision, ROOT / 'imports/spells' / ('r' + str(args.revision)), data, manifest), sort_keys=True))
