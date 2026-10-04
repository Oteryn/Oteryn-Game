"""Typed donor monster spell-slot programs; inert source data, no native promotion."""
import argparse
import copy
import gzip
import hashlib
import json
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(HERE.parent / 'spell-authoring'))

LINKS = ROOT / 'docs/reference/spells/r37-source-closure/unresolved-monster-spell-links.jsonl.gz'
SYNTAX = ROOT / 'docs/reference/spells/r38-source-closure/source-syntax.jsonl.gz'


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def symbolic(nodes, ref):
    row = nodes[ref['node_ref']]
    if row['kind'] == 'Name':
        return row['fields']['id']
    if row['kind'] == 'Index':
        left, right = symbolic(nodes, row['fields']['value']), symbolic(nodes, row['fields']['idx'])
        return left + '.' + right if left is not None and right is not None else None
    return None


def expression(nodes, ref):
    row = nodes[ref['node_ref']]
    kind = row['kind']
    return {'node_ref': row['id'], 'ast_kind': kind, 'symbol': symbolic(nodes, ref),
            'numeric_literal': row['numeric_literal_spelling'],
            'string_literal': row['string_literal_spelling'],
            'boolean_literal': True if kind == 'TrueExpr' else False if kind == 'FalseExpr' else None,
            'nil_literal': kind == 'Nil'}


ROLES = {'COMBAT_PARAM_TYPE': 'combat_type', 'COMBAT_PARAM_EFFECT': 'impact_effect',
         'COMBAT_PARAM_DISTANCEEFFECT': 'projectile_effect', 'COMBAT_PARAM_CHAIN_EFFECT': 'chain_effect',
         'COMBAT_PARAM_AGGRESSIVE': 'aggression', 'COMBAT_PARAM_DISPEL': 'condition_dispel',
         'COMBAT_PARAM_CREATEITEM': 'create_item', 'COMBAT_PARAM_BLOCKARMOR': 'armor_block',
         'COMBAT_PARAM_BLOCKSHIELD': 'shield_block', 'CONDITION_PARAM_TICKS': 'condition_duration',
         'CONDITION_PARAM_SPEED': 'condition_speed', 'CONDITION_PARAM_SUBID': 'condition_sub_id'}
KINDS = {'Combat': 'combat_constructor', 'Condition': 'condition_constructor',
         'createCombatArea': 'area_constructor', 'math.random': 'probability_draw',
         'doTargetCombatHealth': 'target_damage_or_heal', 'doAreaCombatHealth': 'area_damage_or_heal',
         'doTargetCombatCondition': 'target_condition', 'doChallengeCreature': 'taunt_target',
         'addEvent': 'scheduled_callback', 'Game.createMonster': 'monster_spawn',
         'Game.createItem': 'world_item_create', 'Position': 'position_constructor',
         'Tile': 'tile_lookup', 'Creature': 'creature_lookup', 'Monster': 'monster_lookup',
         'setCombatCallback': 'combat_callback'}
METHODS = {'setParameter': 'parameter_binding', 'setFormula': 'formula_binding',
           'setArea': 'area_binding', 'addCondition': 'condition_binding',
           'setCallback': 'callback_binding', 'addDamage': 'damage_schedule',
           'setOutfit': 'outfit_binding', 'execute': 'combat_execute',
           'sendMagicEffect': 'world_magic_effect', 'sendDistanceEffect': 'world_projectile_effect',
           'addHealth': 'creature_health_change', 'removeHealth': 'creature_health_change',
           'addMana': 'creature_mana_change', 'addCondition': 'condition_apply_or_bind',
           'removeCondition': 'condition_remove', 'teleportTo': 'world_teleport',
           'transform': 'world_transform', 'remove': 'world_remove', 'setMaster': 'summon_master_set',
           'say': 'creature_voice', 'setTarget': 'target_assignment',
           'applyZoneEffect': 'zone_combat_helper'}
CONTROL_KINDS = {'If', 'ElseIf', 'While', 'Repeat', 'Fornum', 'Forin', 'AnonymousFunction'}


def map_program(syntax):
    if not syntax['syntax_valid']:
        raise ValueError('no complete donor AST')
    nodes = syntax['ast']['nodes']
    annotations = []
    def walk(node_id, context=None, controls=()):
        row = nodes[node_id]
        fields = row['fields']
        if row['kind'] in ('Function', 'LocalFunction', 'Method'):
            context = node_id
        if row['kind'] in ('Call', 'Invoke'):
            invoke = row['kind'] == 'Invoke'
            method = symbolic(nodes, fields['func'])
            receiver = symbolic(nodes, fields['source']) if invoke else None
            identity = (receiver + ':' + method if receiver and method else method)
            category = METHODS.get(method, 'external_method_reference') if invoke else KINDS.get(method, 'external_function_reference')
            args = [expression(nodes, arg) for arg in fields['args']]
            parameter = args[0]['symbol'] if method == 'setParameter' and args else None
            annotations.append({'node_ref': node_id, 'source_preorder': len(annotations),
                                'context_function_ref': context, 'control_ancestor_refs': list(controls),
                                'category': category, 'call_identity': identity, 'receiver_symbol': receiver,
                                'receiver_binding_qualified': False,
                                'parameter_semantic_role': ROLES.get(parameter, 'other_source_parameter') if parameter else None,
                                'arguments': args})
        nested_controls = controls + (node_id,) if row['kind'] in CONTROL_KINDS else controls
        for value in fields.values():
            refs = value if isinstance(value, list) else [value]
            for ref in refs:
                if isinstance(ref, dict) and 'node_ref' in ref:
                    walk(ref['node_ref'], context, nested_controls)
    walk(syntax['ast']['root_node_ref'])
    # The upstream AST also admits Method/anonymous function forms; preserve their complete body refs.
    programs = []
    for row in nodes:
        if row['kind'] not in ('Function', 'LocalFunction', 'Method', 'AnonymousFunction'):
            continue
        fields = row['fields']
        name = symbolic(nodes, fields['name']) if 'name' in fields else None
        if row['kind'] == 'Method':
            name = (symbolic(nodes, fields['source']) or '<dynamic>') + ':' + (name or '<dynamic>')
        body = nodes[fields['body']['node_ref']]
        programs.append({'node_ref': row['id'], 'kind': row['kind'], 'name': name,
                         'arguments': [expression(nodes, a) for a in fields['args']],
                         'statement_refs': [r['node_ref'] for r in body['fields']['body']]})
    declarations = [a for a in annotations if a['category'] not in ('external_method_reference', 'external_function_reference')]
    return {'source_syntax': syntax, 'mechanic_operations': declarations,
            'external_operations': [a for a in annotations if a not in declarations],
            'function_programs': programs,
            'program_chronology': 'full_AST_control_flow_not_call_preorder_execution_order',
            'whole_source_file_preserved': True, 'transitive_helper_closure_complete': False,
            'binding_qualified': False, 'runtime_activation': False}


def load_inputs():
    links = [json.loads(line) for line in gzip.open(LINKS, 'rt')]
    identities = {(r['source'], r['registered_source']['path']) for r in links if r['registered_source']}
    syntax = {}
    with gzip.open(SYNTAX, 'rt') as stream:
        for line in stream:
            row = json.loads(line)
            key = row['source'], row['path']
            if key in identities:
                syntax[key] = row
    if len(links) != 175 or len(syntax) != 141 or len(syntax) != len(identities):
        raise ValueError('exact immutable source population differs')
    return links, syntax


def build():
    from source_monster_inline_semantics import build_inline
    links, syntax = load_inputs()
    keys = sorted(syntax)
    programs = [map_program(syntax[k]) for k in keys]
    indices = {k: i for i, k in enumerate(keys)}
    slots = []
    for source in links:
        registered = source['registered_source']
        index = indices[(source['source'], registered['path'])] if registered else None
        if registered:
            ast = programs[index]['source_syntax']
            for field, actual in [('revision', ast['revision']), ('path', ast['path']),
                                  ('sha256', ast['source_sha256']), ('git_blob', ast['git_blob'])]:
                if registered[field] != actual:
                    raise ValueError('registry exact source identity mismatch')
        slots.append({'slot_identity': source['slot_identity'], 'source': source['source'],
                      'monster': source['monster'], 'original_slot_sha256': source['original_slot_sha256'],
                      'source_parameters': source['source_parameters'], 'source_program_index': index,
                      'inline_semantics': build_inline(source) if not registered else None,
                      'custom_category': source['custom_descriptor_link']['mechanic_category'] if source['custom_descriptor_link'] else None,
                      'original_conversion_status': 'unresolved_semantics',
                      'source_mapping_status': 'typed_source_program' if registered else 'typed_inline_source_fields',
                      'source_data_complete': True, 'full_slot_projection_complete': False,
                      'projection_gaps': ([{'kind': 'native_combat_or_world_provider', 'call_identity': op['call_identity'], 'node_ref': op['node_ref']}
                                           for op in programs[index]['external_operations']]
                                          + [{'kind': 'receiver_binding_and_transitive_helper_closure', 'call_identity': None, 'node_ref': None}]
                                          if registered else [{'kind': 'undefined_combat_type_native_projection', 'call_identity': None, 'node_ref': None}]),
                      'native_provider_qualified': False, 'runtime_activation': False,
                      'engine_default_omissions': sorted({'target', 'range', 'minDamage', 'maxDamage', 'effect', 'shootEffect', 'type'} - set(source['source_parameters']))})
    return {'schema': 'OTERYN_MONSTER_SLOT_SOURCE_SEMANTICS/v1', 'slot_count': len(slots),
            'registered_slot_count': sum(s['source_program_index'] is not None for s in slots),
            'inline_slot_count': sum(s['source_program_index'] is None for s in slots),
            'custom_descriptor_slot_count': sum(s['custom_category'] is not None for s in slots),
            'source_program_count': len(programs), 'slots': slots, 'source_programs': programs,
            'candidate_count': 0, 'runtime_activation': False, 'native_admission': False,
            'external_sources_used': False}


def validate(packet):
    import jsonschema
    from referencing import Registry, Resource
    schema = json.loads((HERE / 'source-monster-slot-semantics.schema.json').read_text())
    syntax_schema = json.loads((HERE.parent / 'spell-authoring/source-syntax.schema.json').read_text())
    inline_schema = json.loads((HERE / 'source-monster-inline-semantics.schema.json').read_text())
    registry = Registry().with_resources([(s['$id'], Resource.from_contents(s)) for s in (syntax_schema, inline_schema)])
    # R38's per-kind dispatch preserves strict node validation without quadratic 59-way scans.
    envelope = copy.deepcopy(schema)
    envelope['properties']['source_programs']['items']['properties']['source_syntax'] = {'type': 'object'}
    jsonschema.Draft202012Validator(envelope, registry=registry).validate(packet)
    header = copy.deepcopy(syntax_schema)
    header['properties']['ast']['anyOf'][1]['properties']['nodes']['items'] = {'type': 'object'}
    header_validator = jsonschema.Draft202012Validator(header)
    node_validators = {kind: jsonschema.Draft202012Validator(definition)
                       for kind, definition in syntax_schema['$defs'].items()}
    for program in packet['source_programs']:
        header_validator.validate(program['source_syntax'])
        for node in program['source_syntax']['ast']['nodes']:
            node_validators[node['kind']].validate(node)

    identities = [tuple(s['slot_identity'].values()) for s in packet['slots']]
    if len(set(identities)) != 175:
        raise ValueError('duplicate or missing source slots')
    for program in packet['source_programs']:
        expected = map_program(program['source_syntax'])
        if program != expected:
            raise ValueError('typed mechanic mapping differs from complete source AST')
    originals = {tuple(r['slot_identity'].values()): r for r in (json.loads(line) for line in gzip.open(LINKS, 'rt'))}
    for slot in packet['slots']:
        original = originals.get(tuple(slot['slot_identity'].values()))
        if original is None or slot['source_parameters'] != original['source_parameters'] or slot['original_slot_sha256'] != original['original_slot_sha256']:
            raise ValueError('source slot changed from immutable R37')
        index = slot['source_program_index']
        if (index is None) != (original['registered_source'] is None):
            raise ValueError('registered/inline source selection changed')
        if index is not None and 0 <= index < len(packet['source_programs']):
            ast = packet['source_programs'][index]['source_syntax']
            if ast['source'] != original['source'] or ast['source_sha256'] != original['registered_source']['sha256'] or ast['path'] != original['registered_source']['path']:
                raise ValueError('slot linked to wrong exact source program')
        if index is not None and not 0 <= index < len(packet['source_programs']):
            raise ValueError('slot source program out of bounds')
    if (sum(s['source_program_index'] is not None for s in packet['slots']) != 156
            or sum(s['custom_category'] is not None for s in packet['slots']) != 21):
        raise ValueError('registry/custom membership differs')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    packet = build()
    validate(packet)
    payload = (json.dumps(packet, sort_keys=True, separators=(',', ':')) + '\n').encode()
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_bytes(gzip.compress(payload, mtime=0))


if __name__ == '__main__':
    main()
