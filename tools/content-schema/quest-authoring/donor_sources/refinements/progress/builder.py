"""Supplement three existing mission tracks with exact scoped-alias AST storage writes."""
import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

TRACKS = {
    ('canary', 'Storage.Quest.U11_40.ThreatenedDreams.Mission02[1]'): 'canary:quest-progress/quest/u11_40/threatened_dreams/mission02_1',
    ('canary', 'Storage.Quest.U11_40.ThreatenedDreams.Mission03[1]'): 'canary:quest-progress/quest/u11_40/threatened_dreams/mission03_1',
    ('crystalserver', 'Storage.Quest.U8_5.ShadowsOfYalahar.Mission13'): 'crystalserver:quest-progress/quest/u8_5/shadows_of_yalahar/mission13',
}
STATUS = 'SOURCE_MISSION_WRITE_SPEC_EXECUTION_UNPROVEN'


def digest(value):
    return hashlib.sha256(json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def source_path(node, pointer, engine, seen=None):
    """Source reference identity only; numeric index retained; no Native slot/constant folding."""
    seen = set() if seen is None else seen
    kind, f = node['node_type'], node['fields']
    if kind == 'Name':
        if f['id'] == 'Storage':
            if engine.resolve('Storage', pointer):
                return None
            return 'Storage', []
        binding = engine.resolve(f['id'], pointer)
        if not binding or binding['kind'] != 'local_declaration' or binding['declaration_pointer'] in seen:
            return None
        declaration, offset = binding['declaration_pointer'].rsplit('/fields/targets/', 1)
        assignment = engine_get(engine.ast, declaration)
        if len(assignment['fields']['targets']) != len(assignment['fields']['values']):
            return None
        # An alias reassignment cannot be certified as this source path.
        for ap, an in engine.nodes:
            if an['node_type'] != 'Assign':
                continue
            for target in an['fields']['targets']:
                if target.get('node_type') == 'Name' and target['fields']['id'] == f['id'] and engine.resolve(f['id'], ap) == binding:
                    return None
        vp = declaration+'/fields/values/'+offset
        resolved = source_path(assignment['fields']['values'][int(offset)], vp, engine, seen | {binding['declaration_pointer']})
        if not resolved:
            return None
        path, witnesses = resolved
        return path, witnesses + [{'binding': binding, 'assignment_witness': engine.witness(assignment, declaration), 'alias_target_witness': engine.witness(assignment['fields']['values'][int(offset)], vp), 'execution_alias_mutation_or_library_activation': 'NOT_PROVEN'}]
    if kind == 'Index':
        base = source_path(f['value'], pointer+'/fields/value', engine, seen)
        if not base:
            return None
        path, witnesses = base
        if f['notation']['name'] == 'DOT' and f['idx']['node_type'] == 'Name':
            return path+'.'+f['idx']['fields']['id'], witnesses
        if f['notation']['name'] == 'SQUARE' and f['idx']['node_type'] == 'Number' and type(f['idx']['fields']['n']) is int:
            return path+'['+str(f['idx']['fields']['n'])+']', witnesses
    return None


def engine_get(value, pointer):
    for part in pointer.split('/')[1:]:
        value = value[int(part)] if isinstance(value, list) else value[part]
    return value


def enclosing_context(engine, pointer):
    """Preserve exact full enclosing branch/loop/function statements; do not guess nearest guard."""
    result = []
    for ap, an in engine.nodes:
        if pointer.startswith(ap+'/') and an['node_type'] in ('If', 'ElseIf', 'Else', 'While', 'Repeat', 'Forin', 'Fornum', 'Function', 'LocalFunction', 'AnonymousFunction', 'Method'):
            f = an['fields']
            context = {'kind': an['node_type'], 'ast_pointer': ap, 'source_witness': engine.witness(an, ap), 'path_to_write': pointer[len(ap):], 'is_orelse_path': pointer.startswith(ap+'/fields/orelse/')}
            if f.get('test'):
                context['test_witness'] = engine.witness(f['test'], ap+'/fields/test')
            result.append(context)
    return result


def mission_bindings(root):
    by_track = {track: [] for track in TRACKS.values()}
    for file in sorted((root/'content/quests/definitions').glob('quests-*.json')):
        for row_index, row in enumerate(json.loads(file.read_text())['records']):
            q = row['definition']
            source = q.get('source_data', {}).get('quest', {})
            for i, mission in enumerate(source.get('missions', [])):
                if mission['progress'] in by_track:
                    by_track[mission['progress']].append({'quest_key': q['identity']['key'], 'source_quest_identity': source['identity'], 'mission_key': mission['key'],
                        'path': file.relative_to(root).as_posix(), 'json_pointer': '/records/'+str(row_index)+'/definition/source_data/quest/missions/'+str(i),
                        'mission_sha256': digest(mission), 'mission': mission})
    if any(not refs for refs in by_track.values()):
        raise ValueError('Required canonical Source mission declaration absent')
    return by_track


def build(root, manifest_path, ast_root):
    from donor_sources.refinements.fields import builder as fields
    root, manifest_path = Path(root), Path(manifest_path)
    files = json.loads(manifest_path.read_text())['files']
    selected = []
    raws = {}
    for row in files:
        if row['source'] not in ('canary', 'crystalserver') or not row['path'].endswith('.lua'):
            continue
        # Corpus selection is lexical only; every selected write/alias is decoded from full AST below.
        raw = fields.read_source(row, manifest_path)
        if b'setStorageValue' in raw and (b'ThreatenedDreams' in raw or b'ShadowsOfYalahar' in raw):
            selected.append(row)
            raws[row['sha256']] = raw
    loader = fields.ASTLoader(ast_root, {r['sha256'] for r in selected})
    mission_refs = mission_bindings(root)
    packet = {'schema': 'OTERYN_SOURCE_MISSION_PROGRESS_SUPPLEMENT/v1', 'records': [], 'runtime_enabled': False, 'native_admission': False,
              'original_core_unchanged': True, 'scope': 'THREE_EXISTING_SOURCE_TRACKS_NOT_COMPLETE_QUEST_MIGRATION'}
    for row in selected:
        raw = raws[row['sha256']]
        p = fields.provenance(row)
        engine = fields.Engine(loader.capture(row['sha256'], raw), raw, p)
        for pointer, node in engine.nodes:
            if node['node_type'] != 'Invoke' or fields.name(node['fields']['func']) != 'setStorageValue' or len(node['fields']['args']) != 2:
                continue
            sp = pointer+'/fields/args/0'
            resolved = source_path(node['fields']['args'][0], sp, engine)
            if not resolved or (row['source'], resolved[0]) not in TRACKS:
                continue
            target = TRACKS[row['source'], resolved[0]]
            refs = mission_refs[target]
            vp = pointer+'/fields/args/1'
            value = node['fields']['args'][1]
            value_spec = {'kind': 'source_expression_operand', 'witness': engine.witness(value, vp), 'ast_pointer': vp, 'value_kind': value['node_type'], 'runtime_type_not_inferred': True}
            if value['node_type'] == 'Number':
                value_spec.update({'kind': 'literal_number', 'value': value['fields']['n']})
            packet['records'].append({'source_component_id': ':'.join(p[k] for k in ('source','revision','path')), 'quest_keys': sorted({r['quest_key'] for r in refs}),
                'source_target': target, 'source_storage_expression': resolved[0], 'canonical_mission_refs': refs, 'status': STATUS, 'provenance': p,
                'write_ast_pointer': pointer, 'write_witness': engine.witness(node, pointer), 'scoped_alias_witnesses': resolved[1],
                'operation': {'kind': 'source_storage_write', 'evaluation_order': ['READ_RECEIVER_ONCE', 'LOOKUP_METHOD', 'EVALUATE_STORAGE_KEY', 'EVALUATE_VALUE', 'CALL_WITH_RECEIVER_SELF'],
                    'lookup_and_errors': ['METHOD_LOOKUP_CAN_RUN_INDEX_METAMETHOD_BEFORE_ARGUMENTS', 'INDEX_ERRORS_PREVENT_ARGUMENT_EVALUATION', 'NONCALLABLE_METHOD_CALL_ERROR_OCCURS_AFTER_ARGUMENTS', 'ARGUMENT_SIDE_EFFECTS_AND_METHOD_LOOKUP_MUTATION_PRESERVED'],
                    'receiver': {'ast_pointer': pointer+'/fields/source', 'witness': engine.witness(node['fields']['source'], pointer+'/fields/source'), 'actual_entity_binding': 'NOT_PROVEN'},
                    'storage_key_operand': {'kind': 'source_index_read_reference', 'ast_pointer': sp, 'witness': engine.witness(node['fields']['args'][0], sp), 'native_storage_slot': None, 'table_metatable_and_alias_runtime_uncertainty_preserved': True},
                    'value_operand': value_spec, 'exact_source_guards_and_early_returns': enclosing_context(engine, pointer), 'source_method_implementation_activation': 'NOT_PROVEN'},
                'runtime_enabled': False, 'native_admission': False, 'canonical_gap_replaced': False})
    packet['summary'] = {'selected_source_files': len(selected), 'write_records': len(packet['records']), 'source_tracks': len({r['source_target'] for r in packet['records']}), 'canonical_mission_joins': sum(len(r['canonical_mission_refs']) for r in packet['records']), 'quest_keys': sorted({q for r in packet['records'] for q in r['quest_keys']})}
    if packet['summary']['source_tracks'] != 3:
        raise ValueError('Supplement omitted a required track')
    return packet


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--repo-root', required=True)
    p.add_argument('--corpus-manifest', required=True)
    p.add_argument('--ast-root', required=True)
    p.add_argument('--out', required=True)
    args = p.parse_args()
    sys.path.insert(0, str(Path(args.repo_root)/'tools/content-schema/quest-authoring'))
    packet = build(args.repo_root, args.corpus_manifest, args.ast_root)
    Path(args.out).write_text(json.dumps(packet, ensure_ascii=False, indent=2)+'\n')
    print(json.dumps(packet['summary']))


if __name__ == '__main__':
    main()
