"""Pinned ordered cast/callback evidence; no executable spell or native admission."""
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import subprocess
from source_syntax import PINS, parse_record, versions

HERE = Path(__file__).resolve().parent
SPECS = (
    ('crystal', 'forked_glacier', 'e5506efeb1d1f45971cb8a39429856f1c6768edbefdda6bce301928f9ba41bc9', 7),
    ('crystal', 'forked_thorns', 'ed98eb45ff7916a48e18db328bb46896fbaeb45457c599293f8e771dcb9d63d0', 6),
    ('canary', 'sweeping_takedown', 'c3711e339e65eb644a29198b332b64641bc1f3c02fcbb1d3efefa08b99da6d0d', None),
)


def symbol(nodes, ref):
    row = nodes[ref['node_ref']]
    if row['kind'] == 'Name':
        return row['fields']['id']
    if row['kind'] == 'Index':
        return symbol(nodes, row['fields']['value']) + '.' + symbol(nodes, row['fields']['idx'])
    raise ValueError('unexpected function-name AST shape')


def functions(syntax):
    nodes = syntax['ast']['nodes']
    result = []
    for row in nodes:
        if row['kind'] not in ('Function', 'LocalFunction'):
            continue
        fields = row['fields']
        body = nodes[fields['body']['node_ref']]
        result.append({'name': symbol(nodes, fields['name']), 'node_ref': row['id'],
                       'arguments': [symbol(nodes, ref) for ref in fields['args']],
                       'statement_refs': [ref['node_ref'] for ref in body['fields']['body']]})
    return result


def build_record(donor, slug, digest, targets, raw=None):
    path = 'data/scripts/spells/attack/' + slug + '.lua'
    if raw is None:
        raw = subprocess.check_output(['git', '-C', '/workspace/spell-sources/' + donor,
                                       'show', PINS[donor] + ':' + path],
                                      env={**os.environ, 'GIT_NO_LAZY_FETCH': '1'})
    if hashlib.sha256(raw).hexdigest() != digest:
        raise ValueError('bounded exact pinned source bytes differ')
    item = {'source': donor, 'revision': PINS[donor], 'path': path, 'sha256': digest,
            'bytes': len(raw), 'git_blob': hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest()}
    syntax = parse_record(item, raw)
    if syntax['syntax_valid'] is not True:
        raise ValueError('complete source syntax unavailable')
    funcs = functions(syntax)
    by_name = {row['name']: row for row in funcs}
    cast = by_name['spell.onCastSpell']
    kinds = [syntax['ast']['nodes'][i]['kind'] for i in cast['statement_refs']]
    if targets is not None:
        if kinds != ['Return'] or set(by_name) != {'onGetFormulaValues', 'getChainValue', 'spell.onCastSpell'}:
            raise ValueError('chain source function shape differs')
        combats = ['combat']
        chronology = [{'operation': 'return_combat_execute', 'combat': 'combat',
                       'statement_ref': cast['statement_refs'][0]}]
        facts = {'base_target_count': targets, 'jump_distance': 5,
                 'backtracking_return_literal': False, 'wheel_target_addition': True,
                 'independent_probability_draw_count': 0, 'on_target_callbacks': [],
                 'cache': None, 'execute_returns_ignored': False,
                 'cast_return': 'combat_execute_result', 'historical_multicombat_label_current': False}
        gaps = ['Wheel additional-target input provider and chain target selection are unqualified.',
                'calculateBaseDamageHealing helper closure and Combat binding behavior are not represented by this packet.']
    else:
        expected = {'calculateSweepingDamage', 'onGetFormulaValuesInner', 'onGetFormulaValuesOuter', 'spell.onCastSpell'}
        if kinds != ['LocalAssign', 'Invoke', 'Invoke', 'Assign', 'Return'] or set(by_name) != expected:
            raise ValueError('sweeping source function shape differs')
        combats = ['combatInner', 'combatOuter']
        ops = [('capture_player_id', None), ('combat_execute', 'combatInner'),
               ('combat_execute', 'combatOuter'), ('clear_cache', None), ('return_true', None)]
        chronology = [{'operation': op, 'combat': combat, 'statement_ref': ref}
                      for (op, combat), ref in zip(ops, cast['statement_refs'])]
        facts = {'base_target_count': None, 'jump_distance': None, 'backtracking_return_literal': None,
                 'wheel_target_addition': False, 'independent_probability_draw_count': 0,
                 'on_target_callbacks': [], 'execute_returns_ignored': True, 'cast_return': 'literal_true',
                 'historical_multicombat_label_current': True,
                 'cache': {'name': 'sweepingTakedownCache', 'scope': 'file_local',
                           'key': 'player_get_id', 'producer': 'onGetFormulaValuesInner',
                           'consumer': 'onGetFormulaValuesOuter', 'cleanup': 'after_both_execute_calls',
                           'cleanup_on_exception_qualified': False, 'cache_miss_pair': [0, 0],
                           'outer_scale': '0.75', 'center_base_power': 48}}
        gaps = ['Flat damage/healing and Harmony helper input closures require their separate source evidence.',
                'Skill bands, callback cache and AREA_SWEEPING_CENTER/OUTER runtime consumers remain unqualified.',
                'Source cleanup follows both calls; exception cleanup and reentrant cache ownership are not qualified.']
    return {'registration_key': donor + ('-main-current' if donor == 'canary' else '-summer-current') + '/' + path + '#1',
            'source_syntax': syntax, 'function_programs': funcs, 'combat_identities': combats,
            'cast_chronology': chronology, 'facts': facts,
            'source_file_chronology_complete': True, 'transitive_helper_closure_complete': False,
            'native_admission': False, 'runtime_activation': False, 'complete_spell_candidate': False,
            'runtime_gaps': gaps}


def build():
    versions()
    records = [build_record(*spec) for spec in SPECS]
    return {'schema': 'OTERYN_ORDERED_COMBAT_SOURCE_EVIDENCE/v1', 'record_count': len(records),
            'records': records, 'runtime_activation': False, 'native_admission': False,
            'candidate_count': 0, 'external_sources_used': False,
            'ast_limits': ['Parser source spans are token anchors, not qualified complete function ranges.',
                           'Parser decoded strings are unqualified; literal spelling and original byte hashes are retained.']}


def validate(packet):
    import jsonschema
    schema_path = HERE / 'source-multicombat-sequences.schema.json'
    schema = json.loads(schema_path.read_text())
    from referencing import Registry, Resource
    syntax_schema = json.loads((HERE / 'source-syntax.schema.json').read_text())
    registry = Registry().with_resource(syntax_schema['$id'], Resource.from_contents(syntax_schema))
    jsonschema.Draft202012Validator(schema, registry=registry).validate(packet)
    for row in packet['records']:
        nodes = row['source_syntax']['ast']['nodes']
        if len({r['name'] for r in row['function_programs']}) != len(row['function_programs']):
            raise ValueError('duplicate source function identity')
        if functions(row['source_syntax']) != row['function_programs']:
            raise ValueError('function-program linkage differs from complete AST')
        cast = next(r for r in row['function_programs'] if r['name'] == 'spell.onCastSpell')
        if [r['statement_ref'] for r in row['cast_chronology']] != cast['statement_refs']:
            raise ValueError('ordered cast chronology omits or reorders source statements')
        expected_events = (['capture_player_id', 'combat_execute', 'combat_execute', 'clear_cache', 'return_true']
                           if row['facts']['cache'] is not None else ['return_combat_execute'])
        if [event['operation'] for event in row['cast_chronology']] != expected_events:
            raise ValueError('cast event semantics differ from bounded source shape')
        expected_combats = [None, 'combatInner', 'combatOuter', None, None] if row['facts']['cache'] is not None else ['combat']
        if [event['combat'] for event in row['cast_chronology']] != expected_combats:
            raise ValueError('ordered combat identity semantics differ')
        for event in row['cast_chronology']:
            if event['combat'] is not None and event['combat'] not in row['combat_identities']:
                raise ValueError('unknown combat identity')
            if not 0 <= event['statement_ref'] < len(nodes):
                raise ValueError('source statement reference out of bounds')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--out', required=True)
    args = parser.parse_args()
    packet = build()
    validate(packet)
    encoded = (json.dumps(packet, sort_keys=True, separators=(',', ':')) + '\n').encode()
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_bytes(gzip.compress(encoded, mtime=0) if out.suffix == '.gz' else encoded)


if __name__ == '__main__':
    main()
