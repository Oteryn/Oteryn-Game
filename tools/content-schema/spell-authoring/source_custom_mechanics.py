#!/usr/bin/env python3
"""Export pinned Position-based source-only mechanics descriptors; never evaluate Lua."""
import argparse
import gzip
import hashlib
import json
import pathlib
import re
import subprocess
from source_mechanics_inventory import SOURCES, scan, lex

SCHEMA = 'OTERYN_SOURCE_CUSTOM_MECHANICS/v1'
SCRIPTS = {'mazoran_fire': 'ground_transform', 'charge_vortex': 'ground_transform',
           'time_guardian': 'boss_form_swap', 'time_guardiann': 'boss_form_swap',
           'heal_brain_head': 'fixed_position_named_monster_heal',
           'generator': 'random_absolute_spawn', "gaz'haragoth_summon": 'area_bound_wild_minion_counting'}
OPS = {'transform': 'transform_item', 'setActionId': 'set_source_action_id',
       'createItem': 'create_item', 'createMonster': 'spawn_monster',
       'addHealth': 'change_health', 'teleportTo': 'teleport', 'setMaster': 'set_master',
       'setSummon': 'unavailable_summon_binding', 'remove': 'remove_subject', 'say': 'say', 'sendMagicEffect': 'visual_effect',
       'sendDistanceEffect': 'projectile_effect'}


def literal(expr):
    if expr.get('kind') == 'number_literal':
        return expr['value']
    if expr.get('kind') == 'string_literal_token':
        token = expr['token']
        if len(token) >= 2 and token[0] in ('"', "'") and token[-1] == token[0] and '\\' not in token:
            return token[1:-1]
    return None


def delay_ms(expr):
    if expr.get('kind') == 'number_literal':
        return expr['value']
    ts = expr.get('tokens', [])
    if len(ts) == 3 and ts[1] == '*' and ts[0].isdigit() and ts[2].isdigit():
        return int(ts[0]) * int(ts[2])
    return None


def descriptor(payload, source, revision, path, blob, category):
    text = payload.decode('utf-8', errors='strict'); facts = scan(text)
    # Retain offsets and strings while removing comments from auxiliary literal scans.
    clean = list(''.join('\n' if c == '\n' else ' ' for c in text))
    for token in lex(text):
        start = token['offset']; clean[start:start+len(token['value'])] = token['value']
    literal_text = ''.join(clean)
    positions = []; actions = []; schedules = []; ranges = []
    for c in facts['calls']:
        name = c['call_identity'].split(':')[-1].split('.')[-1]
        args = c['arguments']
        if c['call_identity'] == 'Position' and len(args) == 3 and all(isinstance(literal(a), (float, int)) for a in args):
            x, y, z = [literal(a) for a in args]
            positions.append({'line': c['line'], 'source_call_order': c['source_order'], 'x': x, 'y': y, 'floor': z})
        if name in OPS:
            actions.append({'operation': OPS[name], 'line': c['line'], 'source_call_order': c['source_order'],
                            'call_identity': c['call_identity'], 'arguments': args,
                            'execution_status': 'source_only_no_native_consumer'})
        if c['call_identity'] == 'math.random':
            ranges.append({'line': c['line'], 'source_call_order': c['source_order'], 'arguments': args,
                           'literal_bounds': [literal(a) for a in args] if all(isinstance(literal(a), (int, float)) for a in args) else None})
        if c['call_identity'] == 'addEvent':
            schedules.append({'line': c['line'], 'source_call_order': c['source_order'],
                              'callback': args[0] if args else None, 'delay_expression': args[1] if len(args) > 1 else None,
                              'literal_delay_ms': delay_ms(args[1]) if len(args) > 1 else None,
                              'callback_arguments': args[2:], 'execution_status': 'source_only_no_scheduler_consumer'})
    bindings = []
    pattern = re.compile(r'\{\s*itemid\s*=\s*(\d+)\s*,\s*position\s*=\s*Position\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\)\s*\}')
    for m in pattern.finditer(literal_text):
        item, x, y, z = map(int, m.groups())
        bindings.append({'line': text.count('\n', 0, m.start())+1, 'source_item_id': item, 'position': {'x': x, 'y': y, 'floor': z}})
    loops = []
    for m in re.finditer(r'\bfor\s+(\w+)\s*=\s*(\d+)\s*,\s*(\d+)\s+do\b', literal_text):
        loops.append({'line': text.count('\n', 0, m.start())+1, 'variable': m[1], 'minimum': int(m[2]), 'maximum': int(m[3])})
    tables = []
    for m in re.finditer(r'\blocal\s+(\w+)\s*=\s*\{\s*([\d\s,]+)\}', literal_text):
        tables.append({'line': text.count('\n', 0, m.start())+1, 'variable': m[1], 'numeric_values': [int(n) for n in re.findall(r'\d+', m[2])]})
    filters = []
    for m in re.finditer(r':getName\(\):lower\(\)\s*==\s*(["\'])([^"\']+)\1', literal_text):
        filters.append({'line': text.count('\n', 0, m.start())+1, 'normalized_source_name': m[2]})
    return {'source_identity': {'source': source, 'revision': revision, 'path': path, 'git_blob': blob,
                                'sha256': hashlib.sha256(payload).hexdigest(), 'bytes': len(payload)},
            'mechanic_category': category, 'runtime_activation': False,
            'identity_allocation': False, 'coordinate_domain': 'donor_absolute_coordinates_not_projected',
            'helper_sources': [], 'gaz_state_contract': None, 'unavailable_bindings': [],
            'literal_positions': positions, 'item_position_bindings': bindings,
            'literal_loop_ranges': loops, 'literal_numeric_tables': tables,
            'name_filter_literals': filters, 'random_calls': ranges,
            'candidate_actions': actions, 'candidate_schedules': schedules, 'source_evidence': facts,
            'limits': ['Source order does not assert execution order or branch reachability.',
                       'Raw item/action IDs, names and coordinates require separately accepted native bindings.',
                       'Health arithmetic, tile/world lookup, ground state and source predicate semantics remain source expressions.',
                       'Literal-only facts and typed call evidence are preserved without running or probing the source world.']}


def build(source_root):
    records = []
    for source, revision in SOURCES.items():
        prefixes = ['data-otservbr-global'] if source == 'canary' else ['data-global', 'data-crystal']
        repo = pathlib.Path(source_root) / source
        for prefix in prefixes:
            for slug, category in SCRIPTS.items():
                path = f'{prefix}/scripts/spells/monster/{slug}.lua'
                payload = subprocess.check_output(['git', '-C', str(repo), 'show', revision+':'+path])
                blob = subprocess.check_output(['git', '-C', str(repo), 'rev-parse', revision+':'+path], text=True).strip()
                record = descriptor(payload, source, revision, path, blob, category)
                if category == 'area_bound_wild_minion_counting':
                    helper_path = f'{prefix}/scripts/spells/monster/gaz_functions.lua'
                    helper = subprocess.check_output(['git', '-C', str(repo), 'show', revision+':'+helper_path])
                    helper_blob = subprocess.check_output(['git', '-C', str(repo), 'rev-parse', revision+':'+helper_path], text=True).strip()
                    # Read exact literal declarations; never seed or execute mutable Lua state.
                    helper_text = helper.decode()
                    initial = {key: int(value) for key, value in re.findall(r'(MinionsNow|MaxSummons)\s*=\s*(\d+)', helper_text)}
                    if set(initial) != {'MinionsNow', 'MaxSummons'}:
                        raise ValueError('Gaz helper literal contract incomplete')
                    record['helper_sources'] = [{'source_identity': {'source': source, 'revision': revision,
                        'path': helper_path, 'git_blob': helper_blob, 'sha256': hashlib.sha256(helper).hexdigest(),
                        'bytes': len(helper)}, 'source_evidence': scan(helper_text)}]
                    source_text = payload.decode()
                    normalized = re.sub(r'\s+', '', source_text)
                    required_markers = ['ifcheck>=GazVariables.MaxSummonsthen', 'ifcheck<GazVariables.MinionsNowthen',
                        'fori=1,(GazVariables.MinionsNow-check)do', 'math.random(0,100)<25',
                        'GazVariables.MinionsNow=GazVariables.MinionsNow+1', 'monster:setSummon(sum)']
                    if not all(marker in normalized for marker in required_markers):
                        raise ValueError('Pinned Gaz control-flow contract differs; retain as unsupported rather than invent facts')
                    spectator_calls = [c for c in record['source_evidence']['calls'] if c['call_identity'] == 'Game.getSpectators']
                    search_args = [literal(a) for a in spectator_calls[0]['arguments'][-4:]]
                    names = re.findall(r':getName\(\)\s*==\s*"([^"]+)"', source_text)
                    if search_args != [25,25,25,25] or names != ["Minion of Gaz'haragoth"]:
                        raise ValueError('Pinned Gaz spectator/name filter contract differs')
                    record['gaz_state_contract'] = {'initial_literal_state': initial,
                        'counting_name_exact': "Minion of Gaz'haragoth", 'spectator_range_arguments': [25,25,25,25],
                        'owned_summon_filter_present': False, 'counting_scope': 'name_matched_area_spectators',
                        'chance_rng_bounds': [0,100], 'chance_predicate': 'roll_less_than_25',
                        'mutable_state_key': 'GazVariables.MinionsNow',
                        'state_increment_branch': 'successful_random_branch_source_increment_not_conditioned_on_spawn_success',
                        'limit_comparison': 'check_greater_than_or_equal_MaxSummons_return_false',
                        'fill_comparison': 'check_less_than_MinionsNow',
                        'fill_loop_bound_expression': 'GazVariables.MinionsNow - check',
                        'ownership_status': 'unavailable_binding_do_not_infer_owned_summons'}
                    record['unavailable_bindings'] = [{'source_method': 'setSummon', 'argument_symbol': 'sum',
                        'argument_declaration_status': 'not_declared_in_spell_or_literal_helper',
                        'source_binding_status': 'not_registered_in_pinned_lua_or_creature_sources',
                        'execution_status': 'source_only_unsupported_binding'}]
                records.append(record)
    return {'schema': SCHEMA, 'runtime_activation': False, 'record_count': len(records), 'records': records}


def main():
    p = argparse.ArgumentParser(); p.add_argument('--source-root', required=True); p.add_argument('--out', required=True); a = p.parse_args()
    result = build(a.source_root)
    import jsonschema
    schema = json.loads(pathlib.Path(__file__).with_name('source-custom-mechanics.schema.json').read_text())
    jsonschema.Draft202012Validator(schema).validate(result)
    payload = (json.dumps(result, ensure_ascii=False, indent=2)+'\n').encode()
    out = pathlib.Path(a.out)
    out.write_bytes(gzip.compress(payload, mtime=0) if out.suffix == '.gz' else payload)


if __name__ == '__main__':
    main()
