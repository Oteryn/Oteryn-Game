"""R49: complete same-file source programs lowered from immutable R38 Lua AST.

Instructions model source control flow and evaluation operands. Expressions and
literal spelling stay in the existing AST, addressed by exact node IDs and row
hashes. No source is executed and no native API/engine contract is admitted.
"""
import argparse
import copy
import gzip
import hashlib
import json
from pathlib import Path

import jsonschema
import source_syntax

HERE = Path(__file__).resolve().parent
AST_PATH = Path('docs/reference/spells/r38-source-closure/source-syntax.jsonl.gz')
FACT_PATH = Path('docs/reference/spells/r28-source-closure/player-source-bundles/source-callback-facts.jsonl.gz')
WORKLIST = Path('docs/reference/spells/current-source-gap-worklist.json')
OUTPUT = Path('docs/reference/spells/r49-source-closure')
SCHEMA_PATH = HERE / 'source-cast-programs.schema.json'
SCHEMA_ID = 'OTERYN_SOURCE_CAST_PROGRAM/v1'
EXPECTED_AST_SHA = 'e1bba945a8352595c70cdd6e62a12fced06a4109e8365399b786b5b00503be6b'
EXPECTED_CAPTURE_SHA = '77c90d487f673b66f59c61a3831048870861ae88e5f34a8fcd46dca32a64ccd8'
OPCODES = {'Chunk': 'module', 'Block': 'ordered_block', 'Assign': 'assign', 'LocalAssign': 'local_assign',
           'While': 'while', 'Do': 'do', 'Repeat': 'repeat_until', 'ElseIf': 'else_if', 'If': 'if',
           'Label': 'label', 'Goto': 'goto', 'SemiColon': 'empty', 'Break': 'break', 'Return': 'return',
           'Fornum': 'numeric_for', 'Forin': 'generic_for', 'Call': 'call', 'Invoke': 'method_call',
           'Function': 'function', 'LocalFunction': 'local_function', 'Method': 'method',
           'AnonymousFunction': 'anonymous_function'}
FUNCTION_KINDS = {'Function', 'LocalFunction', 'Method', 'AnonymousFunction'}
EXPECTED_KEYS = '''canary-main-current/data/scripts/runes/intense_healing_rune.lua#1
canary-main-current/data/scripts/runes/ultimate_healing_rune.lua#1
canary-main-current/data/scripts/spells/attack/devastating_knockout.lua#1
canary-main-current/data/scripts/spells/attack/greater_tiger_clash.lua#1
canary-main-current/data/scripts/spells/attack/sweeping_takedown.lua#1
canary-main-current/data/scripts/spells/attack/tiger_clash.lua#1
canary-main-current/data/scripts/spells/healing/nature's_embrace.lua#1
canary-main-current/data/scripts/spells/support/blood_rage.lua#1
canary-main-current/data/scripts/spells/support/cancel_magic_shield.lua#1
canary-main-current/data/scripts/spells/support/expose_weakness.lua#1
canary-main-current/data/scripts/spells/support/find_person.lua#1
canary-main-current/data/scripts/spells/support/protector.lua#1
crystal-summer-current/data/scripts/runes/intense_healing_rune.lua#1
crystal-summer-current/data/scripts/runes/ultimate_healing_rune.lua#1
crystal-summer-current/data/scripts/spells/attack/forked_glacier.lua#1
crystal-summer-current/data/scripts/spells/attack/forked_thorns.lua#1
crystal-summer-current/data/scripts/spells/healing/heal_friend.lua#1
crystal-summer-current/data/scripts/spells/healing/nature's_embrace.lua#1
crystal-summer-current/data/scripts/spells/support/cancel_magic_shield.lua#1
crystal-summer-current/data/scripts/spells/support/expose_weakness.lua#1
crystal-summer-current/data/scripts/spells/support/find_person.lua#1'''.splitlines()


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()


def closed(properties):
    return {'type': 'object', 'additionalProperties': False, 'properties': properties, 'required': list(properties)}


def schema():
    integer = {'type': 'integer', 'minimum': 0}
    string = {'type': 'string'}
    digest = {'type': 'string', 'pattern': '^[0-9a-f]{64}$'}
    reference = {'oneOf': [closed({'instruction_ref': integer}), closed({'syntax_node_ref': integer})]}
    kinds = []
    for kind, opcode in OPCODES.items():
        fields = {}
        for name, field_type in source_syntax.FIELDS[kind].items():
            if field_type == 'N': value = reference
            elif field_type == 'O': value = {'anyOf': [reference, {'type': 'null'}]}
            elif field_type == 'L': value = {'type': 'array', 'items': reference}
            elif field_type == 'D': value = {'anyOf': [reference, {'type': 'number'}]}
            elif field_type == 'B': value = {'type': 'boolean'}
            elif field_type == 'S': value = string
            elif field_type == 'call_style': value = {'enum': ['DEFAULT', 'NO_PARENTHESIS']}
            else: raise ValueError('unhandled instruction field contract ' + field_type)
            fields[name] = value
        kinds.append(closed({'source_node_ref': integer, 'source_kind': {'const': kind},
                             'opcode': {'const': opcode}, 'operands': closed(fields)}))
    function = closed({'source_node_ref': integer, 'name': string, 'body_instruction_ref': integer,
                       'parameter_syntax_node_refs': {'type': 'array', 'items': integer},
                       'is_cast_entrypoint': {'type': 'boolean'}})
    call = closed({'source_node_ref': integer, 'callee_label': string, 'call_style': string,
                   'call_kind': {'enum': ['call', 'method_call']},
                   'same_file_function_candidates': {'type': 'array', 'items': integer, 'uniqueItems': True},
                   'name_match_is_lexical_binding_qualified': {'const': False},
                   'arguments': {'type': 'array', 'items': reference},
                   'external_api_semantics_qualified': {'const': False}})
    packet = closed({'schema': {'const': SCHEMA_ID}, 'registration_key': {'enum': EXPECTED_KEYS},
                     'source_revision': {'enum': list(source_syntax.PINS.values())}, 'source_file': string,
                     'source_sha256': digest,
                     'syntax': closed({'path': string, 'archive_sha256': digest, 'record_sha256': digest,
                                       'node_count': integer, 'root_node_ref': integer}),
                     'callback_capture': closed({'path': string, 'archive_sha256': digest, 'record_sha256': digest,
                                                 'source_combats_count': integer, 'registrar_retained_in_reference': {'const': True}}),
                     'root_instruction_ref': integer, 'instructions': {'type': 'array', 'items': {'oneOf': kinds}},
                     'functions': {'type': 'array', 'items': function},
                     'cast_entrypoint_instruction_refs': {'type': 'array', 'minItems': 1, 'items': integer},
                     'dependency_calls': {'type': 'array', 'items': call},
                     'source_combat_constructors': {'type': 'array', 'items': integer},
                     'source_registrar_calls': {'type': 'array', 'items': integer},
                     'source_presentation_calls': {'type': 'array', 'items': integer},
                     'existing_evidence': {'type': 'array', 'items': closed({'import_set': string, 'metadata_path': string})},
                     'completeness': closed({'same_file_ast_nodes_reachable': integer, 'same_file_ast_nodes_total': integer,
                                            'same_file_typed_statement_program_complete': {'const': True},
                                            'same_file_functions_complete': {'const': True},
                                            'all_calls_dependency_inventory_complete': {'const': True},
                                            'callback_capture_is_complete_semantics_authority': {'const': False},
                                            'external_api_semantics_complete': {'const': False},
                                            'transitive_external_helper_source_closure_complete': {'const': False},
                                            'expression_evaluation_binding_qualified': {'const': False}}),
                     'complete_spell_candidate': {'const': False}, 'input_provider_equivalence': {'const': False}, 'native_identity_allocation': {'const': False}, 'runtime_activation': {'const': False},
                     'native_execution_qualified': {'const': False}, 'canonical_selection_changed': {'const': False}})
    return {'$schema': 'https://json-schema.org/draft/2020-12/schema', '$id': 'urn:oteryn:source-cast-program:1', **packet}


def refs(value):
    if isinstance(value, dict):
        if set(value) == {'node_ref'}:
            yield value['node_ref']
        else:
            for child in value.values(): yield from refs(child)
    elif isinstance(value, list):
        for child in value: yield from refs(child)


def label(nodes, node_ref):
    node = nodes[node_ref]; fields = node['fields']; kind = node['kind']
    if kind == 'Name': return fields['id']
    if kind == 'Index':
        value = label(nodes, fields['value']['node_ref']); index = label(nodes, fields['idx']['node_ref'])
        return value + '.' + index
    if kind == 'String': return 'string@' + str(node_ref)
    return kind + '@' + str(node_ref)


def lower(ast_record):
    if not ast_record['syntax_valid'] or ast_record['parse_status'] != 'parsed':
        raise ValueError('complete parsed R38 AST required')
    nodes = ast_record['ast']['nodes']
    if ast_record['node_count'] != len(nodes) or any(node['id'] != i for i, node in enumerate(nodes)):
        raise ValueError('source AST IDs/count invalid')
    for node in nodes:
        if node['kind'] not in source_syntax.FIELDS or set(node['fields']) != set(source_syntax.FIELDS[node['kind']]):
            raise ValueError('source node differs from exact upstream field contract')
        if any(type(ref) is not int or ref < 0 or ref >= len(nodes) for ref in refs(node['fields'])):
            raise ValueError('dangling syntax dependency')
    instruction_ids = {node['id'] for node in nodes if node['kind'] in OPCODES}
    def transform(value):
        if isinstance(value, dict):
            if set(value) == {'node_ref'}:
                ref = value['node_ref']; return {'instruction_ref' if ref in instruction_ids else 'syntax_node_ref': ref}
            return {key: transform(child) for key, child in value.items()}
        if isinstance(value, list): return [transform(child) for child in value]
        return copy.deepcopy(value)
    instructions = [{'source_node_ref': node['id'], 'source_kind': node['kind'],
                     'opcode': OPCODES[node['kind']], 'operands': transform(node['fields'])}
                    for node in nodes if node['id'] in instruction_ids]
    functions = []
    for node in nodes:
        if node['kind'] not in FUNCTION_KINDS: continue
        fields = node['fields']
        name = label(nodes, fields['name']['node_ref']) if 'name' in fields else 'anonymous@' + str(node['id'])
        if node['kind'] == 'Method': name = label(nodes, fields['source']['node_ref']) + ':' + name
        functions.append({'source_node_ref': node['id'], 'name': name,
                          'body_instruction_ref': fields['body']['node_ref'],
                          'parameter_syntax_node_refs': [ref['node_ref'] for ref in fields['args']],
                          'is_cast_entrypoint': name.endswith('.onCastSpell') or name.endswith(':onCastSpell')})
    entries = [value['source_node_ref'] for value in functions if value['is_cast_entrypoint']]
    if not entries: raise ValueError('cast function entrypoint missing')
    calls = []
    for node in nodes:
        if node['kind'] not in ('Call', 'Invoke'): continue
        fields = node['fields']; name = label(nodes, fields['func']['node_ref'])
        if node['kind'] == 'Invoke': name = label(nodes, fields['source']['node_ref']) + ':' + name
        calls.append({'source_node_ref': node['id'], 'callee_label': name, 'call_style': fields['style'],
                      'call_kind': OPCODES[node['kind']],
                      'same_file_function_candidates': [value['source_node_ref'] for value in functions if value['name'] == name],
                      'name_match_is_lexical_binding_qualified': False,
                      'arguments': transform(fields['args']), 'external_api_semantics_qualified': False})
    root = ast_record['ast']['root_node_ref']
    seen, pending = set(), [root]
    while pending:
        ref = pending.pop()
        if ref not in seen:
            seen.add(ref); pending.extend(refs(nodes[ref]['fields']))
    if len(seen) != len(nodes): raise ValueError('unreachable AST nodes; no loss allowed')
    return {'root_instruction_ref': root, 'instructions': instructions, 'functions': functions,
            'cast_entrypoint_instruction_refs': entries, 'dependency_calls': calls,
            'source_combat_constructors': [call['source_node_ref'] for call in calls if call['callee_label'] == 'Combat'],
            'source_registrar_calls': [call['source_node_ref'] for call in calls if call['callee_label'].endswith(':register')],
            'source_presentation_calls': [call['source_node_ref'] for call in calls if call['callee_label'].endswith(
                (':sendMagicEffect', ':sendCancelMessage', ':sendTextMessage', ':castSound', ':impactSound', ':setParameter'))],
            'completeness': {'same_file_ast_nodes_reachable': len(seen), 'same_file_ast_nodes_total': len(nodes),
                             'same_file_typed_statement_program_complete': True, 'same_file_functions_complete': True,
                             'all_calls_dependency_inventory_complete': True,
                             'callback_capture_is_complete_semantics_authority': False,
                             'external_api_semantics_complete': False,
                             'transitive_external_helper_source_closure_complete': False,
                             'expression_evaluation_binding_qualified': False}}


def verify_lowering(program, ast_record):
    expected = lower(ast_record)
    for key, value in expected.items():
        if program[key] != value: raise ValueError('typed source lowering differs from exact AST: ' + key)
    return True


def build(repo):
    repo = Path(repo)
    worklist = json.loads((repo / WORKLIST).read_text())
    selected = {row['registration_key']: row for row in worklist['records'] if row['baseline_lane'] == 'other_cast'}
    if len(selected) != 21 or set(selected) != set(EXPECTED_KEYS):
        raise ValueError('expected exact 21 current other_cast registrations')
    fact_bytes = (repo / FACT_PATH).read_bytes()
    if sha(fact_bytes) != EXPECTED_CAPTURE_SHA: raise ValueError('immutable R28 capture bytes changed')
    fact_rows = [json.loads(line) for line in gzip.decompress(fact_bytes).splitlines()]
    facts = {row['registration_key']: row for row in fact_rows}
    if len(fact_rows) != len(facts): raise ValueError('duplicate source callback facts')
    ast_bytes = (repo / AST_PATH).read_bytes()
    if sha(ast_bytes) != EXPECTED_AST_SHA: raise ValueError('immutable R38 syntax bytes changed')
    targets = {(key.split('-')[0], key.split('/', 1)[1].rsplit('#', 1)[0]) for key in EXPECTED_KEYS}
    syntax = {}
    with gzip.open(repo / AST_PATH) as source:
        for line in source:
            row = json.loads(line); key = (row['source'], row['path'])
            if key in targets:
                if key in syntax: raise ValueError('duplicate immutable syntax source')
                syntax[key] = row
    if set(syntax) != targets: raise ValueError('source syntax population incomplete')
    records = []
    # Validate instruction nodes once by their closed type instead of repeated oneOf scans.
    schema_value = schema()
    node_schemas = schema_value['properties']['instructions']['items']['oneOf']
    validators = {value['properties']['source_kind']['const']: jsonschema.Draft202012Validator(value) for value in node_schemas}
    header_schema = copy.deepcopy(schema_value); header_schema['properties']['instructions']['items'] = {'type': 'object'}
    header_validator = jsonschema.Draft202012Validator(header_schema)
    for key in EXPECTED_KEYS:
        if selected[key]['structural_status'] != 'BLOCKED': raise ValueError('only blocked other_cast source programs')
        fact = facts[key]; donor = key.split('-')[0]; path = key.split('/', 1)[1].rsplit('#', 1)[0]
        ast_record = syntax[(donor, path)]
        if (fact['source_revision'] != ast_record['revision'] or fact['source_sha256'] != ast_record['source_sha256']
                or fact['source_revision'] != source_syntax.PINS[donor]):
            raise ValueError('AST/capture/pin identity mismatch')
        result = lower(ast_record)
        record = {'schema': SCHEMA_ID, 'registration_key': key, 'source_revision': fact['source_revision'],
                  'source_file': path, 'source_sha256': fact['source_sha256'],
                  'syntax': {'path': AST_PATH.as_posix(), 'archive_sha256': sha(ast_bytes), 'record_sha256': sha(canonical(ast_record)),
                             'node_count': ast_record['node_count'], 'root_node_ref': ast_record['ast']['root_node_ref']},
                  'callback_capture': {'path': FACT_PATH.as_posix(), 'archive_sha256': sha(fact_bytes),
                                       'record_sha256': sha(canonical(fact)),
                                       'source_combats_count': len(fact['source_callback_facts'].get('combats', [])),
                                       'registrar_retained_in_reference': True},
                  **result, 'existing_evidence': selected[key]['source_evidence'],
                  'complete_spell_candidate': False, 'input_provider_equivalence': False, 'native_identity_allocation': False, 'runtime_activation': False,
                  'native_execution_qualified': False, 'canonical_selection_changed': False}
        header_validator.validate(record)
        for instruction in result['instructions']: validators[instruction['source_kind']].validate(instruction)
        verify_lowering(record, ast_record)
        records.append(record)
    return records


def write_json(path, value):
    path.write_text(json.dumps(value, sort_keys=True, indent=2, ensure_ascii=False) + '\n')


def main():
    parser = argparse.ArgumentParser(); parser.add_argument('--repo', default='.'); args = parser.parse_args()
    repo = Path(args.repo).resolve(); records = build(repo); out = repo / OUTPUT; out.mkdir(parents=True, exist_ok=True)
    write_json(SCHEMA_PATH, schema())
    payload = b''.join(canonical(row) + b'\n' for row in records)
    path = out / 'source-cast-programs.jsonl.gz'; path.write_bytes(gzip.compress(payload, mtime=0))
    proof = {'schema': SCHEMA_ID + '/proof', 'records': len(records), 'complete_same_file_source_programs': len(records),
             'complete_spell_candidates': 0, 'full_spell_candidates': 0, 'instructions': sum(len(row['instructions']) for row in records),
             'functions': sum(len(row['functions']) for row in records), 'calls': sum(len(row['dependency_calls']) for row in records),
             'source_ast_nodes_reachable': sum(row['syntax']['node_count'] for row in records),
             'registration_keys': EXPECTED_KEYS, 'source_pins': source_syntax.PINS,
             'payload_sha256': sha(payload), 'gzip_sha256': sha(path.read_bytes()), 'schema_sha256': sha(SCHEMA_PATH.read_bytes()),
             'base_ast_sha256': sha((repo / AST_PATH).read_bytes()), 'base_capture_sha256': sha((repo / FACT_PATH).read_bytes()),
             'worklist_sha256': sha((repo / WORKLIST).read_bytes()), 'producer_sha256': sha(Path(__file__).read_bytes()),
             'runtime_activation': False, 'native_execution_qualified': False, 'canonical_selection_changed': False,
             'native_identity_allocation': False, 'input_provider_equivalence': False,
             'input_proofs': {'imports/spells/r38/evidence/source-syntax.jsonl.gz': sha((repo / AST_PATH).read_bytes()),
                              'imports/spells/r28/player-source-bundles/source-callback-facts.jsonl.gz': sha((repo / FACT_PATH).read_bytes())},
             'all_same_file_nodes_and_functions_linked': True, 'transitive_external_helper_source_closure_complete': False,
             'limits': ['Typed source statements and same-file functions are complete; referenced expressions preserve R38 syntax, raw literals and ordering.',
                       'Names matching same-file functions are candidates, not lexical scope binding proof.',
                       'External APIs, condition state, game services, helper source closure and Lua/native evaluation remain unqualified.',
                       'Existing truncated callback capture is retained only as dependency provenance; full AST is program authority.',
                       'No executable Lua payload, native admission, runtime contract change or full Spell promotion.']}
    write_json(out / 'source-cast-program-proof.json', proof)
    receipt = {'schema': SCHEMA_ID + '/receipt', 'status': 'COMPLETE_SAME_FILE_SOURCE_PROGRAMS_FULL_SPELLS_BLOCKED',
               'records': len(records), 'complete_same_file_source_programs': len(records), 'complete_spell_candidates': 0,
               'gzip_sha256': proof['gzip_sha256'], 'payload_sha256': proof['payload_sha256'],
               'input_proofs': proof['input_proofs'], 'native_identity_allocation': False, 'input_provider_equivalence': False,
               'registration_keys': EXPECTED_KEYS, 'schema_path': SCHEMA_PATH.relative_to(repo).as_posix(),
               'schema_sha256': proof['schema_sha256'], 'base_ast_sha256': proof['base_ast_sha256'],
               'base_capture_sha256': proof['base_capture_sha256'],
               'runtime_activation': False, 'native_execution_qualified': False, 'canonical_selection_changed': False,
               'artifacts': {value.name: sha(value.read_bytes()) for value in [path, out / 'source-cast-program-proof.json']}}
    write_json(out / 'source-cast-program-receipt.json', receipt)
    print(json.dumps({key: proof[key] for key in ['records', 'instructions', 'functions', 'calls', 'source_ast_nodes_reachable']}))


if __name__ == '__main__':
    main()
