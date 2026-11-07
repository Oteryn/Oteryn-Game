"""Lossless donor Source behavior conversion; never executes Lua or admits Native."""
import argparse
import base64
import gzip
import hashlib
import json
import tarfile
from pathlib import Path

SCHEMA = 'OTERYN_QUEST_SOURCE_OTHER_COMPONENTS/v1'
CONTROL = {'If', 'ElseIf', 'While', 'Repeat', 'Fornum', 'Forin', 'Do', 'Return', 'Break', 'Goto', 'Label'}
FUNCTION = {'Function', 'LocalFunction', 'AnonymousFunction', 'Method'}
CONSTRUCTORS = {'CreatureEvent', 'Spell', 'Combat', 'Condition', 'Action', 'MoveEvent', 'GlobalEvent', 'TalkAction'}
CONFIG = {'register', 'name', 'id', 'type', 'words', 'group', 'cooldown', 'groupCooldown', 'level', 'mana', 'range', 'needTarget', 'needLearn', 'vocation', 'isPremium', 'setParameter', 'setFormula', 'setArea', 'setCallback'}
EFFECTS = {
    'addHealth': 'health_change', 'addMana': 'mana_change', 'remove': 'removal',
    'teleportTo': 'teleport', 'setStorageValue': 'storage_write', 'set': 'keyvalue_write_candidate',
    'addItem': 'item_grant', 'removeItem': 'item_removal', 'createItem': 'item_creation',
    'createMonster': 'monster_creation', 'sendMagicEffect': 'visual_effect',
    'say': 'speech', 'addEvent': 'scheduled_call', 'execute': 'combat_execution_candidate',
    'addCondition': 'condition_application', 'registerEvent': 'event_binding',
    'addAchievement': 'achievement', 'setActionId': 'action_binding', 'setAttribute': 'attribute_write',
}

def digest(raw):
    return hashlib.sha256(raw).hexdigest()

class ASTCache:
    def __init__(self, root, wanted):
        self.root = Path(root)
        path = self.root / 'index.json'
        raw = path.read_bytes() if path.exists() else gzip.decompress((self.root / 'index.json.gz').read_bytes())
        index = json.loads(raw)
        self.expected = {r['sha256']: r['container_sha256'] for r in index['captures']}
        self.cache = {}
        if not (self.root / 'captures').is_dir():
            for archive in sorted(self.root.glob('*.tar.gz')):
                with tarfile.open(archive, 'r:gz') as tar:
                    for member in tar:
                        name = Path(member.name).name
                        source_sha = name.removesuffix('.json.gz')
                        if source_sha in wanted and member.isfile():
                            if source_sha in self.cache: raise ValueError('Duplicate AST capture')
                            self.cache[source_sha] = self.decode(tar.extractfile(member).read(), source_sha)
    def decode(self, payload, source_sha):
        if digest(payload) != self.expected[source_sha]: raise ValueError('AST container hash mismatch')
        capture = json.loads(gzip.decompress(payload))
        if capture['native_semantic_admission'] is not False: raise ValueError('AST Native admission forbidden')
        return capture
    def get(self, source_sha):
        if source_sha not in self.cache:
            payload = (self.root / 'captures' / (source_sha + '.json.gz')).read_bytes()
            self.cache[source_sha] = self.decode(payload, source_sha)
        return self.cache[source_sha]

def symbol(node):
    if not isinstance(node, dict) or 'node_type' not in node:
        return None
    f = node['fields']; kind = node['node_type']
    if kind == 'Name': return f['id']
    if kind == 'Index':
        a, b = symbol(f['value']), symbol(f['idx'])
        if a and b and f.get('notation', {}).get('name') == 'DOT': return a + '.' + b
    return None

def iter_nodes(value, pointer=''):
    if isinstance(value, dict):
        if 'node_type' in value: yield pointer, value
        for key, item in value.items():
            yield from iter_nodes(item, pointer + '/' + key)
    elif isinstance(value, list):
        for i, item in enumerate(value): yield from iter_nodes(item, pointer + '/' + str(i))

def convert(value):
    if isinstance(value, list): return [convert(v) for v in value]
    if not isinstance(value, dict): return value
    if 'node_type' in value:
        return {'op': value['node_type'], 'fields': convert(value['fields']), 'span': value['span']}
    return {k: convert(v) for k, v in value.items()}

def restore(value):
    if isinstance(value, list): return [restore(v) for v in value]
    if not isinstance(value, dict): return value
    if 'op' in value:
        return {'node_type': value['op'], 'fields': restore(value['fields']), 'span': value['span']}
    return {k: restore(v) for k, v in value.items()}

def witness(node, pointer, raw, text):
    span = node['span']
    if span is None:
        return {'ast_pointer': pointer, 'span': None, 'span_status': 'UPSTREAM_NODE_HAS_NO_TOKEN_BOUNDS', 'source_sha256': digest(raw)}
    start, end = span['start_char'], span['end_char_exclusive']
    if not 0 <= start <= end <= len(text): raise ValueError('AST span outside source')
    byte_start, byte_end = len(text[:start].encode('utf-8')), len(text[:end].encode('utf-8'))
    return {'ast_pointer': pointer, 'span': span, 'byte_start': byte_start, 'byte_end_exclusive': byte_end,
            'source_slice_sha256': digest(raw[byte_start:byte_end]), 'source_sha256': digest(raw), 'span_status': 'EXACT'}

def component(record, entry, capture, raw):
    p = record['provenance']
    if capture['status'] != 'PARSED' or capture['sha256'] != p['sha256']: raise ValueError('AST not parsed or wrong source')
    if base64.b64decode(capture['raw_bytes_base64']) != raw: raise ValueError('AST raw source mismatch')
    text = raw.decode('utf-8')
    ast = capture['ast']; behavior = convert(ast)
    if restore(behavior) != ast: raise ValueError('Lossless behavior conversion failed')
    functions, calls, control, declarations = [], [], [], []
    counts = {}
    for ptr, node in iter_nodes(ast):
        kind = node['node_type']; f = node['fields']; counts[kind] = counts.get(kind, 0) + 1
        ref = witness(node, ptr, raw, text)
        if kind in FUNCTION:
            functions.append({'kind': kind, 'symbol': symbol(f.get('name')), 'arguments': [symbol(a) for a in f.get('args', [])], 'body_pointer': ptr + '/fields/body', 'source_ref': ref})
        if kind in CONTROL:
            control.append({'kind': kind, 'behavior_pointer': ptr.replace('/fields/', '/fields/'), 'source_ref': ref})
        if kind in {'Assign', 'LocalAssign'}:
            declarations.append({'kind': kind, 'targets': [symbol(n) for n in f['targets']], 'values_pointer': ptr + '/fields/values', 'source_ref': ref})
        if kind in {'Call', 'Invoke'}:
            name = symbol(f.get('func')); receiver = symbol(f.get('source')) if kind == 'Invoke' else None
            category = 'unresolved_call'
            if name in CONSTRUCTORS: category = 'source_constructor'
            elif name in CONFIG: category = 'source_registration' if name == 'register' else 'source_configuration'
            elif name in EFFECTS: category = EFFECTS[name]
            elif name in {'require', 'dofile', 'loadfile'}: category = 'source_include_candidate'
            calls.append({'call_kind': kind, 'callee': name, 'receiver': receiver, 'category': category,
                          'arguments_pointer': ptr + '/fields/args', 'source_ref': ref,
                          'identity_resolution': 'SOURCE_SYNTAX_ONLY_BUILTIN_OR_CALLEE_IDENTITY_NOT_PROVEN'})
    for call in calls:
        call_pointer = call['source_ref']['ast_pointer']
        contexts = [f for f in functions if call_pointer.startswith(f['body_pointer'] + '/')]
        context = max(contexts, key=lambda f: len(f['body_pointer'])) if contexts else None
        call['caller_function_pointer'] = context['source_ref']['ast_pointer'] if context else None
        call['caller_function_symbol'] = context['symbol'] if context else None
        target = ((call['receiver'] + '.') if call['receiver'] else '') + (call['callee'] or '')
        call['same_file_definition_candidates'] = [f['source_ref']['ast_pointer'] for f in functions if f['symbol'] and f['symbol'] == target]
        call['binding_status'] = 'EXACT_SOURCE_SYMBOL_CANDIDATES_ONLY_SCOPE_AND_RUNTIME_BINDING_UNPROVEN'
    unresolved = [c for c in calls if c['category'] in {'unresolved_call', 'source_include_candidate'}]
    return {'source_component_id': record['source_component_id'], 'provenance': dict(p),
            'definition': {'representation': 'LOSSLESS_SOURCE_BEHAVIOR_TREE', 'behavior': behavior,
                           'functions': functions, 'control_flow': control, 'declarations': declarations,
                           'calls': calls, 'configuration_call_pointers': [c['source_ref']['ast_pointer'] for c in calls if c['category'] in {'source_constructor', 'source_configuration', 'source_registration'}],
                           'effect_candidate_pointers': [c['source_ref']['ast_pointer'] for c in calls if c['category'] in set(EFFECTS.values())],
                           'node_counts': counts, 'total_nodes': sum(counts.values())},
            'source_refs': [{'source_sha256': p['sha256'], 'byte_count': len(raw), 'scope': 'WHOLE_SOURCE_FILE'},
                            {'ast_capture_sha256': digest(json.dumps(capture, sort_keys=True, separators=(',', ':')).encode()), 'scope': 'PARSED_AST_OBJECT_CANONICAL_JSON'}],
            'existing_canonical_owners': record.get('exact_existing_canonical_owners', []),
            'holds': [{'code': 'TYPED_OTERYN_QUEST_SEMANTICS_NOT_ESTABLISHED'},
                      {'code': 'EXTERNAL_CALLER_AND_HELPER_BINDINGS_NOT_ESTABLISHED', 'call_candidate_count': len(unresolved)}],
            'native_admission': False, 'runtime_activation': False,
            'coverage': {'source_bytes_verified': True, 'all_ast_nodes_preserved': True, 'all_control_flow_preserved': True,
                         'all_returns_preserved': True, 'oteryn_semantic_equivalence': False}}

def build(assignment_path, corpus_manifest_path, ast_root):
    assigned = json.loads(Path(assignment_path).read_text())['records']
    ids = [r['source_component_id'] for r in assigned]
    if len(set(ids)) != len(ids): raise ValueError('Duplicate assigned Source ID')
    manifest_path = Path(corpus_manifest_path)
    corpus = json.loads(manifest_path.read_text())
    entries = {(e['source'], e['revision'], e['path']): e for e in corpus['files']}
    out = []
    ast_cache = ASTCache(ast_root, {r['provenance']['sha256'] for r in assigned})
    for record in assigned:
        p = record['provenance']; e = entries[(p['source'], p['revision'], p['path'])]
        for key in ('git_blob_sha1', 'sha256', 'byte_count'):
            if e[key] != p[key]: raise ValueError('Assignment Source pin mismatch: ' + key)
        path = Path(e['cache_path']); path = path if path.is_absolute() else manifest_path.parent / path
        raw = path.read_bytes()
        if digest(raw) != p['sha256'] or len(raw) != p['byte_count']: raise ValueError('Source byte mismatch')
        git_sha = hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest()
        if git_sha != p['git_blob_sha1']: raise ValueError('Git blob mismatch')
        capture = ast_cache.get(p['sha256'])
        out.append(component(record, e, capture, raw))
    return {'schema': SCHEMA, 'scope': 'SOURCE_BEHAVIOR_ONLY_NOT_NATIVE_QUEST_MIGRATION',
            'assignment_sha256': digest(Path(assignment_path).read_bytes()),
            'records': out, 'summary': {'components': len(out), 'functions': sum(len(r['definition']['functions']) for r in out),
                'control_flow_nodes': sum(len(r['definition']['control_flow']) for r in out),
                'calls': sum(len(r['definition']['calls']) for r in out),
                'ast_nodes': sum(r['definition']['total_nodes'] for r in out)},
            'native_admission': False, 'runtime_activation': False}

def main():
    ap = argparse.ArgumentParser(); ap.add_argument('--assignment', required=True); ap.add_argument('--corpus-manifest', required=True)
    ap.add_argument('--ast-root', required=True); ap.add_argument('--out', required=True); args = ap.parse_args()
    data = build(args.assignment, args.corpus_manifest, args.ast_root)
    encoded = (json.dumps(data, ensure_ascii=False, sort_keys=True, separators=(',', ':')) + '\n').encode()
    path = Path(args.out); path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(gzip.compress(encoded, mtime=0) if path.suffix == '.gz' else encoded)
    print(json.dumps(data['summary'], sort_keys=True))

if __name__ == '__main__': main()
