"""Lossless, ordered Source event IR. No execution or native admission."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import re
import tarfile


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def read_source(entry, manifest_dir):
    p = Path(entry['cache_path'])
    raw = (p if p.is_absolute() else manifest_dir / p).read_bytes()
    if digest(raw) != entry['sha256'] or len(raw) != entry['byte_count']:
        raise ValueError('source bytes mismatch')
    git = hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest()
    if git != entry['git_blob_sha1']:
        raise ValueError('git blob mismatch')
    return raw


def load_captures(root, shas):
    index_path = root / 'index.json'
    if index_path.exists():
        index = json.loads(index_path.read_bytes())
    else:
        index = json.loads(gzip.decompress((root / 'index.json.gz').read_bytes()))
    entries = {c['sha256']: c for c in index['captures']}
    results = {}
    def qualify(sha, packed):
        if digest(packed) != entries[sha]['container_sha256']:
            raise ValueError('AST container index mismatch')
        decoded = gzip.decompress(packed)
        if digest(decoded) != entries[sha]['capture_sha256']:
            raise ValueError('AST content index mismatch')
        results[sha] = json.loads(decoded)
    for sha in shas:
        path = root / 'captures' / (sha + '.json.gz')
        if path.exists(): qualify(sha, path.read_bytes())
    missing = set(shas) - results.keys()
    if missing:
        for archive in sorted(root.glob('*.tar.gz')):
            with tarfile.open(archive, 'r:gz') as tar:
                for member in tar:
                    sha = Path(member.name).name.removesuffix('.json.gz')
                    if sha in missing:
                        qualify(sha, tar.extractfile(member).read())
                        missing.remove(sha)
            if not missing: break
    if missing: raise ValueError('AST captures missing')
    return results


def symbol(n):
    if not isinstance(n, dict) or 'node_type' not in n:
        return None
    f, k = n['fields'], n['node_type']
    if k == 'Name':
        return f['id']
    if k == 'Index':
        left, right = symbol(f['value']), symbol(f['idx'])
        return left + '.' + right if left and right else None
    if k in ('Call', 'Invoke'):
        fun = symbol(f['func'])
        if k == 'Invoke':
            receiver = symbol(f['source'])
            return receiver + ':' + fun if receiver and fun else None
        return fun
    return None


METHODS = {
    'setStorageValue': 'SOURCE_STORAGE_WRITE', 'getStorageValue': 'SOURCE_STORAGE_READ',
    'addItem': 'SOURCE_ITEM_CREATE', 'removeItem': 'SOURCE_ITEM_REMOVE',
    'createItem': 'SOURCE_ITEM_CREATE', 'createMonster': 'SOURCE_MONSTER_CREATE',
    'teleportTo': 'SOURCE_TELEPORT', 'sendMagicEffect': 'SOURCE_EFFECT',
    'transform': 'SOURCE_TRANSFORM', 'remove': 'SOURCE_REMOVE',
    'addHealth': 'SOURCE_HEALTH_CHANGE', 'getHealth': 'SOURCE_HEALTH_READ',
    'addEvent': 'SOURCE_DEFERRED_CALL', 'stopEvent': 'SOURCE_DEFERRED_CANCEL',
    'register': 'SOURCE_EVENT_REGISTER', 'registerEvent': 'SOURCE_CREATURE_EVENT_ATTACH',
    'interval': 'SOURCE_INTERVAL_CONFIG', 'time': 'SOURCE_TIME_CONFIG',
    'CreatureEvent': 'SOURCE_CREATURE_EVENT_CONSTRUCTOR', 'GlobalEvent': 'SOURCE_GLOBAL_EVENT_CONSTRUCTOR',
}
BRANCH = {'If', 'ElseIf'}
LOOP = {'Forin', 'Fornum', 'While', 'Repeat'}


def convert(capture, raw):
    text = raw.decode('utf-8')
    refs, callbacks, helpers, calls, controls, assignments = [], [], [], [], [], []
    names = set()
    event_receivers = set()
    def discover(node):
        if isinstance(node, dict):
            if node.get('node_type') in ('Assign', 'LocalAssign'):
                f = node['fields']
                for target, value in zip(f.get('targets', []), f.get('values', [])):
                    if value.get('node_type') == 'Call' and symbol(value) in ('CreatureEvent', 'GlobalEvent'):
                        event_receivers.add(symbol(target))
            for v in node.values(): discover(v)
        elif isinstance(node, list):
            for v in node: discover(v)
    discover(capture['ast'])
    def visit(value, path):
        if isinstance(value, list):
            return [visit(v, path + '/' + str(i)) for i, v in enumerate(value)]
        if not isinstance(value, dict):
            return value
        if 'node_type' not in value:
            return {k: visit(v, path + '/' + k) for k, v in value.items()}
        kind, fields = value['node_type'], value['fields']
        span = value.get('span')
        proof = {'ast_pointer': path, 'char_span': span}
        if span is not None:
            start, end = span['start_char'], span['end_char_exclusive']
            if not 0 <= start <= end <= len(text):
                raise ValueError('AST source span bounds')
            part = text[start:end].encode('utf-8')
            proof.update(byte_start=len(text[:start].encode('utf-8')), byte_end_exclusive=len(text[:end].encode('utf-8')), span_sha256=digest(part))
        refs.append(proof)
        irkind = ('BRANCH' if kind in BRANCH else 'LOOP' if kind in LOOP else
                  'RETURN' if kind == 'Return' else 'ASSIGNMENT' if kind in ('Assign', 'LocalAssign') else
                  'CALL' if kind in ('Call', 'Invoke') else 'FUNCTION' if kind in ('Function', 'LocalFunction', 'Method', 'AnonymousFunction') else
                  'ORDERED_BLOCK' if kind in ('Chunk', 'Block') else 'SOURCE_EXPRESSION')
        result = {'kind': irkind, 'lua_node_type': kind, 'source_ref': len(refs) - 1,
                  'fields': {k: visit(v, path + '/fields/' + k) for k, v in fields.items()}}
        if kind == 'Name':
            names.add(fields['id'])
        if kind in ('Function', 'LocalFunction', 'Method'):
            name = symbol(fields.get('name'))
            item = {'name': name, 'tree_pointer': path, 'source_ref': result['source_ref'], 'parameters': [symbol(x) for x in fields.get('args', [])]}
            (callbacks if name and re.search(r'(?:^|[.:])on[A-Z]', name) else helpers).append(item)
        if kind in ('Call', 'Invoke'):
            name = symbol(value)
            last = re.split('[.:]', name)[-1] if name else None
            operation = METHODS.get(last, 'SOURCE_CALL_UNRESOLVED')
            if last in ('register', 'interval', 'time'):
                receiver = symbol(fields.get('source')) if kind == 'Invoke' else None
                if receiver not in event_receivers:
                    operation = 'SOURCE_CALL_UNRESOLVED'
            if last in ('CreatureEvent', 'GlobalEvent') and name != last:
                operation = 'SOURCE_CALL_UNRESOLVED'
            result['source_operation'] = operation
            result['dispatch_binding'] = 'NOT_PROVEN'
            calls.append({'callee': name, 'operation': operation, 'tree_pointer': path, 'source_ref': result['source_ref']})
        if kind in BRANCH | LOOP or kind == 'Return':
            controls.append({'lua_node_type': kind, 'tree_pointer': path, 'source_ref': result['source_ref']})
        if kind in ('Assign', 'LocalAssign'):
            assignments.append({'targets': [symbol(x) for x in fields.get('targets', [])], 'tree_pointer': path, 'source_ref': result['source_ref']})
        return result
    program = visit(capture['ast'], '')
    return {'language': 'LUA_SOURCE_EVENT_IR/v1', 'program': program, 'callbacks': callbacks,
            'helpers': helpers, 'calls': calls, 'control_flow': controls, 'assignments': assignments,
            'symbol_references': sorted(names), 'event_receiver_candidates': sorted(x for x in event_receivers if x),
            'source_refs': refs}


def validate_capture(capture, raw):
    if capture.get('status') != 'PARSED' or capture.get('native_semantic_admission') is not False:
        raise ValueError('capture not qualified Source only')
    if base64.b64decode(capture['raw_bytes_base64']) != raw or capture['sha256'] != digest(raw):
        raise ValueError('AST/source mismatch')


def build(assignment, manifest_path, ast_root):
    manifest = json.loads(manifest_path.read_text())
    sources = {(x['source'], x['revision'], x['path']): x for x in manifest['files']}
    records = []
    captures = load_captures(ast_root, {e['provenance']['sha256'] for e in assignment['records']})
    for entry in assignment['records']:
        p = entry['provenance']
        source = sources[(p['source'], p['revision'], p['path'])]
        for key in ('sha256', 'git_blob_sha1', 'byte_count'):
            if p[key] != source[key]:
                raise ValueError('assignment provenance mismatch')
        raw = read_source(source, manifest_path.parent)
        capture = captures[p['sha256']]
        validate_capture(capture, raw)
        definition = convert(capture, raw)
        refs = definition.pop('source_refs')
        records.append({'source_component_id': entry['source_component_id'], 'provenance': p,
                        'definition': definition, 'source_refs': refs,
                        'existing_canonical_owners': entry['exact_existing_canonical_owners'],
                        'caller_witnesses': [],
                        'holds': ['SOURCE_API_DISPATCH_NOT_BOUND_TO_OTERYN', 'RUNTIME_ACTIVATION_NOT_ASSESSED',
                                  'GLOBAL_SYMBOL_AND_DYNAMIC_HELPER_BINDINGS_REQUIRE_JOIN'],
                        'conversion_status': 'MATERIALIZED_SOURCE_IR_PARTIAL_SEMANTIC_BINDING',
                        'native_semantic_admission': False, 'runtime_admission': False})
    if len({r['source_component_id'] for r in records}) != len(records):
        raise ValueError('duplicate component')
    # Exact quoted event-name attach expressions are witnesses only, never ownership.
    event_names = set()
    for r in records:
        def walk(x):
            if isinstance(x, dict):
                if x.get('source_operation') in ('SOURCE_CREATURE_EVENT_CONSTRUCTOR', 'SOURCE_GLOBAL_EVENT_CONSTRUCTOR'):
                    for arg in x['fields'].get('args', []):
                        if arg.get('lua_node_type') == 'String':
                            s = arg['fields']['s']
                            if isinstance(s, dict) and 'bytes_base64' in s:
                                event_names.add((r['provenance']['source'], r['provenance']['revision'], base64.b64decode(s['bytes_base64']).decode('utf-8')))
                for v in x.values(): walk(v)
            elif isinstance(x, list):
                for v in x: walk(v)
        walk(r['definition']['program'])
    witnesses = []
    attach = re.compile(rb'\bregisterEvent\s*\(\s*([\"\x27])([^\"\x27\r\n]+)\1\s*\)')
    for f in manifest['files']:
        if f['source'] not in ('canary', 'crystalserver') or not f['path'].endswith('.lua'):
            continue
        raw = read_source(f, manifest_path.parent)
        for match in attach.finditer(raw):
            event = match.group(2).decode('utf-8')
            if (f['source'], f['revision'], event) in event_names:
                witnesses.append({'event_name': event, 'provenance': {k: f[k] for k in ('source','revision','path','sha256','git_blob_sha1','byte_count')},
                                  'byte_start': match.start(), 'byte_end_exclusive': match.end(), 'span_sha256': digest(match.group()),
                                  'qualification': 'LEXICAL_ATTACH_CANDIDATE_NOT_EXECUTION_OR_OWNER_PROOF'})
    for r in records:
        events_for_record = set()
        def names_in_record(x):
            if isinstance(x, dict):
                if x.get('source_operation') in ('SOURCE_CREATURE_EVENT_CONSTRUCTOR', 'SOURCE_GLOBAL_EVENT_CONSTRUCTOR'):
                    for arg in x['fields'].get('args', []):
                        if arg.get('lua_node_type') == 'String':
                            value = arg['fields'].get('s', {})
                            if isinstance(value, dict) and 'bytes_base64' in value:
                                events_for_record.add(base64.b64decode(value['bytes_base64']).decode('utf-8'))
                for v in x.values(): names_in_record(v)
            elif isinstance(x, list):
                for v in x: names_in_record(v)
        names_in_record(r['definition']['program'])
        r['caller_witnesses'] = [w for w in witnesses if w['event_name'] in events_for_record and
                                w['provenance']['source'] == r['provenance']['source'] and
                                w['provenance']['revision'] == r['provenance']['revision']]
    summary = {'components': len(records), 'materialized_source_ir': len(records), 'fully_semantically_bound': 0,
               'callbacks': sum(len(r['definition']['callbacks']) for r in records),
               'helper_functions': sum(len(r['definition']['helpers']) for r in records),
               'ordered_control_nodes': sum(len(r['definition']['control_flow']) for r in records),
               'source_api_operations': sum(sum(c['operation'] != 'SOURCE_CALL_UNRESOLVED' for c in r['definition']['calls']) for r in records),
               'unresolved_dispatch_calls': sum(sum(c['operation'] == 'SOURCE_CALL_UNRESOLVED' for c in r['definition']['calls']) for r in records),
               'lexical_caller_witnesses': len(witnesses)}
    return {'schema': 'OTERYN_EVENT_COMPONENT_SOURCE_IR/v1', 'records': records, 'caller_witnesses': witnesses,
            'summary': summary, 'native_semantic_admission': False, 'runtime_admission': False}


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--assignment', type=Path, required=True)
    p.add_argument('--corpus-manifest', type=Path, required=True)
    p.add_argument('--ast-root', type=Path, required=True)
    p.add_argument('--out', type=Path, required=True)
    args = p.parse_args()
    packet = build(json.loads(args.assignment.read_text()), args.corpus_manifest, args.ast_root)
    encoded = (json.dumps(packet, sort_keys=True, ensure_ascii=False, separators=(',', ':')) + '\n').encode()
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_bytes(gzip.compress(encoded, mtime=0) if args.out.suffix == '.gz' else encoded)
    print(json.dumps(packet['summary'], sort_keys=True))


if __name__ == '__main__':
    main()
