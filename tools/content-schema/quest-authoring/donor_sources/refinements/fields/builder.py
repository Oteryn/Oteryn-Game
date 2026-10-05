"""Operational Lua field/index/named operands. No field type inference or constant folding."""
import argparse
import base64
import gzip
import hashlib
import json
import re
import sys
from pathlib import Path


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def get(value, pointer):
    for segment in pointer.split('/')[1:]:
        value = value[int(segment)] if isinstance(value, list) else value[segment]
    return value


def walk(value, pointer=''):
    if isinstance(value, dict):
        if 'node_type' in value:
            yield pointer, value
        for key, child in value.items():
            yield from walk(child, pointer + '/' + key)
    elif isinstance(value, list):
        for i, child in enumerate(value):
            yield from walk(child, pointer + '/' + str(i))


def name(node):
    return node.get('fields', {}).get('id') if node.get('node_type') == 'Name' else None


class Engine:
    """Root supplies verified context and optional resolver exposing resolve(name,pointer)."""
    def __init__(self, ast, raw, provenance, bindings=None, property_witnesses=None, global_witnesses=None):
        self.ast = ast
        self.raw = raw
        self.text = raw.decode('utf-8')
        self.provenance = provenance
        self.bindings = bindings
        self.nodes = list(walk(ast))
        self.property_witnesses = property_witnesses or []
        self.global_witnesses = global_witnesses or []

    def resolve(self, symbol, pointer):
        if self.bindings:
            return self.bindings.resolve(symbol, pointer)
        candidates = []
        for p, node in self.nodes:
            f = node['fields']
            if node['node_type'] in ('Function', 'LocalFunction', 'Method', 'AnonymousFunction'):
                scope = p + '/fields/body'
                if pointer.startswith(scope + '/'):
                    for i, arg in enumerate(f.get('args', [])):
                        if name(arg) == symbol:
                            candidates.append((len(scope), -1, {'kind': 'function_argument', 'name': symbol, 'declaration_pointer': p + '/fields/args/' + str(i), 'scope_pointer': scope}))
            if node['node_type'] == 'LocalAssign' and p.rsplit('/', 1)[-1].isdigit():
                scope, order = p.rsplit('/', 1)
                if pointer.startswith(scope + '/'):
                    use = pointer[len(scope) + 1:].split('/', 1)[0]
                    if use.isdigit() and int(use) > int(order):
                        for i, target in enumerate(f['targets']):
                            if name(target) == symbol:
                                candidates.append((len(scope), int(order), {'kind': 'local_declaration', 'name': symbol, 'declaration_pointer': p + '/fields/targets/' + str(i), 'scope_pointer': scope}))
            if node['node_type'] in ('Forin', 'Fornum'):
                scope = p + '/fields/body'
                if pointer.startswith(scope + '/'):
                    targets = f.get('targets', [f.get('target')])
                    for i, target in enumerate(targets):
                        if target and name(target) == symbol:
                            candidates.append((len(scope), -1, {'kind': 'loop_binding', 'name': symbol, 'declaration_pointer': p + '/fields/' + ('targets/' + str(i) if 'targets' in f else 'target'), 'scope_pointer': scope}))
        return max(candidates, key=lambda c: (c[0], c[1]))[2] if candidates else None

    def witness(self, node, pointer):
        span = node.get('span')
        result = {'provenance': self.provenance, 'ast_pointer': pointer, 'span': span}
        if span:
            fragment = self.text[span['start_char']:span['end_char_exclusive']]
            result.update({'raw_expression': fragment, 'slice_sha256': sha(fragment.encode()),
                           'byte_start': len(self.text[:span['start_char']].encode()),
                           'byte_end_exclusive': len(self.text[:span['end_char_exclusive']].encode())})
        else:
            result['missing_span_reason'] = 'UPSTREAM_AST_NULL_SPAN_FULL_NODE_AND_SOURCE_WITNESS_RETAINED'
        return result

    def declaration_witness(self, binding):
        if not binding or binding['kind'] != 'local_declaration':
            return None
        parent, offset = binding['declaration_pointer'].rsplit('/fields/targets/', 1)
        assignment = get(self.ast, parent)
        f = assignment['fields']
        # Never infer Lua multireturn adjustment or fold expressions.
        if len(f['targets']) != len(f['values']):
            return {'state': 'MULTIRETURN_ADJUSTMENT_NOT_FOLDED', 'assignment_witness': self.witness(assignment, parent)}
        value = f['values'][int(offset)]
        vp = parent + '/fields/values/' + offset
        result = {'state': 'INITIAL_DECLARATION_ONLY_LATER_MUTATIONS_AND_ALIASES_PRESERVED', 'assignment_witness': self.witness(assignment, parent), 'value_witness': self.witness(value, vp), 'value_kind': value['node_type']}
        if value['node_type'] == 'Table':
            result['ordered_fields'] = []
            for i, field in enumerate(value['fields']['fields']):
                fp = vp + '/fields/fields/' + str(i)
                ff = field['fields']
                result['ordered_fields'].append({'order': i, 'field_witness': self.witness(field, fp), 'key': self.witness(ff['key'], fp + '/fields/key') if ff.get('key') else None, 'key_name': name(ff['key']) if ff.get('key') else None, 'value_witness': self.witness(ff['value'], fp + '/fields/value'), 'value_kind': ff['value']['node_type']})
        elif value['node_type'] == 'Number':
            result['declared_literal'] = {'type': 'number', 'value': value['fields']['n']}
        elif value['node_type'] in ('TrueExpr', 'FalseExpr'):
            result['declared_literal'] = {'type': 'boolean', 'value': value['node_type'] == 'TrueExpr'}
        return result

    def project(self, node, pointer):
        kind, f = node['node_type'], node['fields']
        if kind not in ('Index', 'Name'):
            return None
        common = {'source_witness': self.witness(node, pointer), 'native_admission': False, 'runtime_activation': False,
                  'source_status': 'SOURCE_SPEC_COMPLETE_EXECUTION_UNPROVEN', 'value_type_not_inferred': True}
        if kind == 'Name':
            binding = self.resolve(f['id'], pointer)
            return {'kind': 'source_named_operand', 'name': f['id'],
                    'binding': binding or {'kind': 'source_global_environment', 'name': f['id'], 'environment_activation': 'NOT_PROVEN'},
                    'initial_declaration_witness': self.declaration_witness(binding),
                    'library_initial_value_candidates': [w for w in self.global_witnesses if w['symbol'] == f['id']],
                    'read_semantics': ['READ_CURRENT_LEXICAL_VALUE_OR_SOURCE_ENVIRONMENT_AT_EVALUATION_TIME', 'ABSENT_GLOBAL_WITHOUT_ENVIRONMENT_INDEX_METAMETHOD_IS_NIL', 'LATER_WRITES_UPVALUES_AND_ENVIRONMENT_METAMETHODS_NOT_FOLDED'], **common}
        dot = f['notation']['name'] == 'DOT'
        if dot and name(f['idx']) is None:
            return None
        receiver_pointer = pointer + '/fields/value'
        key_pointer = pointer + '/fields/idx'
        receiver = {'kind': 'operand_reference', 'ast_pointer': receiver_pointer, 'node_type': f['value']['node_type']}
        key = {'kind': 'literal_field_key', 'type': 'string', 'value': name(f['idx'])} if dot else {'kind': 'operand_reference', 'ast_pointer': key_pointer, 'node_type': f['idx']['node_type']}
        receiver_binding = self.resolve(name(f['value']), receiver_pointer) if name(f['value']) else None
        property_candidates = [w for w in self.property_witnesses if dot and name(f['idx']) in w['property_names']]
        return {'kind': 'source_index_read', 'notation': f['notation']['name'], 'receiver': receiver, 'key': key,
                'receiver_binding': receiver_binding, 'initial_receiver_declaration': self.declaration_witness(receiver_binding),
                'evaluation_order': ['receiver', 'key', 'lua_index_operation'],
                'operation_semantics': ['DOT_KEY_IS_LITERAL_STRING_NOT_VARIABLE_LOOKUP', 'TABLE_EXISTING_KEY_READ_OTHERWISE_INDEX_METAMETHOD', 'USERDATA_INDEX_DISPATCH_PRESERVED', 'FUNCTION_OR_TABLE_INDEX_METAMETHOD_CHAIN_PRESERVED', 'NIL_MISSING_RESULT_REMAINS_NIL', 'INVALID_RECEIVER_INDEX_ERRORS_AND_CALLBACK_SIDE_EFFECTS_PRESERVED', 'NO_ENTITY_ID_SUBTYPE_STORAGE_OR_ENUM_DOMAIN_INFERENCE', 'NO_CONSTANT_OR_TABLE_LOOKUP_FOLDING'],
                'library_initial_value_candidates': [w for w in self.global_witnesses if w['symbol'] == name(f['value'])],
                'property_dispatch_candidates': property_candidates, 'property_dispatch_activation': 'CONDITIONAL_ON_ACTUAL_RECEIVER_METATABLE_NOT_ASSUMED', **common}

    operand = project



class ASTLoader:
    """Use the existing verified physical/portable input reader, selecting only needed SHAs."""
    def __init__(self, root, digests):
        from donor_semantic_conditions import load_ast_inputs
        _, _, data = load_ast_inputs(Path(root), digests)
        self.cached = {digest: json.loads(gzip.decompress(payload)) for digest, payload in data.items()}

    def capture(self, digest, raw):
        capture = self.cached[digest]
        if capture['status'] != 'PARSED' or capture['sha256'] != digest or base64.b64decode(capture['raw_bytes_base64']) != raw:
            raise ValueError('Field AST Source raw mismatch')
        return capture['ast']


def read_source(row, manifest_path):
    path = Path(row['cache_path'])
    if not path.is_absolute():
        path = manifest_path.parent / path
    raw = path.read_bytes()
    if sha(raw) != row['sha256'] or len(raw) != row['byte_count'] or hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest() != row['git_blob_sha1']:
        raise ValueError('Field witness Source bytes differ')
    return raw


def provenance(row):
    return {k: row[k] for k in ('source', 'repository', 'revision', 'path', 'sha256', 'git_blob_sha1', 'byte_count')}


def raw_witness(row, raw, start, end, role):
    fragment = raw[start:end]
    return {'provenance': provenance(row), 'role': role, 'byte_start': start, 'byte_end_exclusive': end,
            'raw': fragment.decode(), 'slice_sha256': sha(fragment), 'runtime_activation': False}



UID_CPP_BODY_SHA = {
    'canary': 'f8db8c5cbdc3227c051be99daf0fe913e41a3b5924c7b9c489dacb16d9e8d3b0',
    'crystalserver': '17a2fd7bb2445fa20a8c8f37fb4fa6029115df37a92f4cfbb502de00fc85a811',
}


def validate_item_index_model(raw):
    if sha(raw) != '1c9e1083b9deddd6210b295d385791ea2040cccfdf46e2caac05f64611878195':
        raise ValueError('Unqualified Lua ItemIndex body drift')
    text = raw.decode()
    observed = dict(re.findall(r'(?:if|elseif) key == "(\w+)" then\s*return methods\.(\w+)\(self\)', text))
    expected = {'itemid': 'getId', 'actionid': 'getActionId', 'uid': 'getUniqueId', 'type': 'getSubType'}
    if observed != expected or 'local methods = getmetatable(self)' not in text:
        raise ValueError('Unqualified Lua ItemIndex getter body drift')
    for cls in ('Item', 'Container', 'Teleport'):
        if f'rawgetmetatable("{cls}").__index = ItemIndex' not in text:
            raise ValueError('Unqualified Lua ItemIndex metatable installation drift')
    return {key: 'methods.'+getter+'(self)' for key, getter in observed.items()}


def validate_uid_cpp_model(raw, source):
    if sha(raw) != UID_CPP_BODY_SHA.get(source):
        raise ValueError('Unqualified CPP UID getter body drift')


def property_evidence(source, revision, rows, manifest_path):
    by_path = {r['path']: r for r in rows if r['source'] == source and r['revision'] == revision}
    lua_path = 'data/libs/functions/revscriptsys.lua'
    cpp_path = 'src/lua/functions/items/item_functions.cpp'
    if lua_path not in by_path:
        return []
    lr = by_path[lua_path]
    raw = read_source(lr, manifest_path)
    proofs = []
    for cls, classes in [('Item', ['Item', 'Container', 'Teleport']), ('Creature', ['Player', 'Monster', 'Npc'])]:
        marker = ('-- '+cls+' index').encode()
        if marker not in raw:
            continue
        start = raw.index(marker)
        end = raw.find(b'\n-- ', start + len(marker))
        if end < 0:
            end = len(raw)
        if cls == 'Creature' and sha(raw[start:end]) != '22dda0579f985bb1134cca4b6db892f9f8b254b880fa25c958b62e16d9cda9bf':
            raise ValueError('Unqualified Lua CreatureIndex body drift')
        item_operations = validate_item_index_model(raw[start:end]) if cls == 'Item' else None
        proof = {'property_names': ['uid', 'type', 'itemid', 'actionid'], 'conditional_receiver_metatables': classes,
                 'lua_metatable_witness': raw_witness(lr, raw, start, end, cls+'Index'),
                 'getter_cpp_witnesses': [], 'activation': 'CONDITIONAL_NOT_INFERRED_FROM_VARIABLE_NAME',
                 'property_operations': (item_operations if cls == 'Item' else {'itemid': 'literal1', 'uid': 'methods.getId(self)', 'type': 'branch_player_monster_npc_thingtype_or0', 'actionid': 'literal0'})}
        if cls == 'Item' and cpp_path in by_path:
            cr = by_path[cpp_path]
            cpp = read_source(cr, manifest_path)
            for method in ['Id', 'UniqueId', 'ActionId', 'SubType']:
                marker = ('int ItemFunctions::luaItemGet'+method+'(').encode()
                if marker in cpp:
                    start = cpp.index(marker)
                    end = cpp.index(b'\n}', start)+2
                    if method == 'UniqueId':
                        validate_uid_cpp_model(cpp[start:end], source)
                    proof['getter_cpp_witnesses'].append(raw_witness(cr, cpp, start, end, 'Item.get'+method))
            proof['uid_semantics'] = 'SOURCE_UNIQUEID_ATTRIBUTE_OR_SCRIPTENV_TEMPORARY_ADDTHING_HANDLE_NOT_ALWAYS_PERSISTENT_UID'
        proofs.append(proof)
    # Populated table fields require their constructor/return Source, not userdata guessing.
    for path, markers, properties, origin in [
        ('src/lua/functions/creatures/creature_functions.cpp', ['int CreatureFunctions::luaCreatureGetIcon('], ['category', 'icon', 'count'], 'CREATURE_GETICON_RETURN_TABLE_OR_NIL_OR_FALSE'),
        ('src/lua/functions/lua_functions_loader.cpp', ['void Lua::pushOutfit('], ['lookBody'], 'LUA_PUSHOUTFIT_TABLE'),
    ]:
        if path not in by_path:
            continue
        row = by_path[path]
        raw = read_source(row, manifest_path)
        body_witnesses = []
        for marker in markers:
            if marker.encode() in raw:
                start = raw.index(marker.encode())
                end = raw.index(b'\n}', start)+2
                body_witnesses.append(raw_witness(row, raw, start, end, origin))
        if body_witnesses:
            proofs.append({'property_names': properties, 'conditional_receiver_metatables': ['SOURCE_RETURN_TABLE_'+origin],
                           'getter_cpp_witnesses': body_witnesses, 'activation': 'CONDITIONAL_ON_ACTUAL_CALL_RESULT_NOT_INFERRED_FROM_VARIABLE_NAME',
                           'property_operations': {prop: 'SOURCE_TABLE_FIELD_POPULATED_IN_EXACT_CPP_BODY' for prop in properties}})
    return proofs


def library_evidence(source, revision, rows, manifest_path, loader):
    selected = [r for r in rows if r['source'] == source and r['revision'] == revision and r['path'] in {
        'data-global/scripts/lib/a_piece_of_cake_config.lua',
        'data-otservbr-global/lib/others/soulpit.lua',
        'data-global/lib/others/soulpit.lua'}]
    results = []
    for row in selected:
        raw = read_source(row, manifest_path)
        ast = loader.capture(row['sha256'], raw)
        engine = Engine(ast, raw, provenance(row))
        for pointer, node in walk(ast):
            if node['node_type'] != 'Assign':
                continue
            for i, target in enumerate(node['fields']['targets']):
                symbol = name(target)
                if symbol in ('CakeQuest', 'SoulPit'):
                    results.append({'symbol': symbol, 'state': 'DECLARATION_CANDIDATE_RUNTIME_LOAD_ORDER_AND_MUTATIONS_NOT_FOLDED',
                                    'source_witness': engine.witness(node, pointer)})
                elif target['node_type'] == 'Index' and name(target['fields']['value']) == 'CakeQuest' and name(target['fields']['idx']) == 'MonsterName':
                    results.append({'symbol': 'CakeQuest', 'field': 'MonsterName', 'state': 'DECLARATION_CANDIDATE_RUNTIME_LOAD_ORDER_AND_MUTATIONS_NOT_FOLDED',
                                    'source_witness': engine.witness(node, pointer)})
    return results

def build(partials, corpus_manifest, ast_root):
    manifest_path = Path(corpus_manifest)
    files = json.loads(manifest_path.read_text())['files']
    by_id = {':'.join(r[k] for k in ('source', 'revision', 'path')): r for r in files}
    packet = {'schema': 'OTERYN_SOURCE_FIELD_OPERANDS/v1', 'records': [], 'native_admission': False, 'runtime_activation': False}
    library_paths = {'data-global/scripts/lib/a_piece_of_cake_config.lua', 'data-otservbr-global/lib/others/soulpit.lua', 'data-global/lib/others/soulpit.lua'}
    relevant = {(r['provenance']['source'], r['provenance']['revision']) for r in partials}
    digests = {r['provenance']['sha256'] for r in partials} | {r['sha256'] for r in files if (r['source'], r['revision']) in relevant and r['path'] in library_paths}
    loader = ASTLoader(ast_root, digests)
    properties = {key: property_evidence(*key, files, manifest_path) for key in relevant}
    libraries = {key: library_evidence(*key, files, manifest_path, loader) for key in relevant}
    cache = {}
    for condition in partials:
        sid = condition['source_component_id']
        p = condition['provenance']
        if any(p[k] != by_id[sid][k] for k in p):
            raise ValueError('Field Source manifest differs')
        if sid not in cache:
            raw_path = Path(by_id[sid]['cache_path'])
            if not raw_path.is_absolute():
                raw_path = manifest_path.parent / raw_path
            raw = raw_path.read_bytes()
            if sha(raw) != p['sha256'] or len(raw) != p['byte_count'] or hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest() != p['git_blob_sha1']:
                raise ValueError('Field Source bytes differ')
            ast = loader.capture(p['sha256'], raw)
            key = p['source'], p['revision']
            cache[sid] = Engine(ast, raw, p, property_witnesses=properties[key], global_witnesses=libraries[key])
        engine = cache[sid]
        cp = condition['source_ref']['condition_ast_pointer']
        node = get(engine.ast, cp)
        # Every field/index anywhere inside the guard includes call argument/storage indices.
        for rel, atom in walk(node):
            if atom['node_type'] != 'Index' and not (atom['node_type'] == 'Name' and (atom['fields']['id'].upper() == atom['fields']['id'] or atom['fields']['id'] in {'CreatureIconCategory_Quests', 'CreatureIconQuests_RedShield'})):
                continue
            ap = cp + rel
            spec = engine.project(atom, ap)
            if spec:
                packet['records'].append({'source_component_id': sid, 'condition_ast_pointer': cp, 'atom_ast_pointer': ap, 'provenance': p, 'specification': spec})
    packet['summary'] = {'conditions': len(partials), 'field_operand_records': len(packet['records']), 'source_components': len(cache), 'type_or_constant_folds': 0}
    return packet


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--partials', required=True)
    p.add_argument('--authoring', default=str(Path(__file__).resolve().parents[3]))
    p.add_argument('--corpus-manifest', required=True)
    p.add_argument('--ast-root', required=True)
    p.add_argument('--out', required=True)
    args = p.parse_args()
    sys.path.insert(0, args.authoring)
    packet = build(json.loads(Path(args.partials).read_text()), args.corpus_manifest, args.ast_root)
    Path(args.out).write_text(json.dumps(packet, ensure_ascii=False, indent=2)+'\n')
    print(json.dumps(packet['summary']))


if __name__ == '__main__':
    main()
