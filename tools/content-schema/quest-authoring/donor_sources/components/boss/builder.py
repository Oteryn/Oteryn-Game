#!/usr/bin/env python3
"""Convert verified BossLever Source AST into ordered typed source components (no Lua execution)."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import tarfile


def sha(data):
    return hashlib.sha256(data).hexdigest()


def verified_raw(provenance, files, manifest_path):
    key = tuple(provenance[k] for k in ('source', 'revision', 'path'))
    witness = files.get(key)
    if witness is None:
        raise ValueError('Source absent from manifest: ' + repr(key))
    for k in ('sha256', 'git_blob_sha1', 'byte_count'):
        if k not in provenance:
            continue
        if provenance[k] != witness[k]:
            raise ValueError('Assignment witness mismatch: ' + k)
    p = Path(witness['cache_path'])
    if not p.is_absolute():
        p = manifest_path.parent / p
    raw = p.read_bytes()
    git_sha = hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest()
    if sha(raw) != witness['sha256'] or git_sha != witness['git_blob_sha1'] or len(raw) != witness['byte_count']:
        raise ValueError('Source bytes mismatch')
    return raw, {k: witness[k] for k in ('source', 'repository', 'revision', 'path', 'sha256', 'git_blob_sha1', 'byte_count')}


class ASTCache:
    def __init__(self, root, digests):
        self.root = Path(root)
        self.cache = {}
        index_path = self.root / 'index.json'
        if index_path.is_file():
            index = json.loads(index_path.read_text())
        else:
            index = json.loads(gzip.decompress((self.root / 'index.json.gz').read_bytes()))
        self.expected = {r['sha256']: r['container_sha256'] for r in index['captures']}
        if not (self.root / 'captures').is_dir():
            for archive in sorted(self.root.glob('*.tar.gz')):
                with tarfile.open(archive, 'r:gz') as tar:
                    for member in tar:
                        name = Path(member.name).name
                        digest = name.removesuffix('.json.gz').removesuffix('.json')
                        if digest in digests and member.isfile():
                            payload = tar.extractfile(member).read()
                            if sha(payload) != self.expected[digest]:
                                raise ValueError('AST container mismatch')
                            self.cache[digest] = json.loads(gzip.decompress(payload) if name.endswith('.gz') else payload)

    def get(self, digest, raw):
        if digest not in self.cache:
            p = self.root / 'captures' / (digest + '.json.gz')
            payload = p.read_bytes()
            if sha(payload) != self.expected[digest]:
                raise ValueError('AST container mismatch')
            self.cache[digest] = json.loads(gzip.decompress(payload))
        capture = self.cache[digest]
        if capture['status'] != 'PARSED' or capture['sha256'] != digest or base64.b64decode(capture['raw_bytes_base64']) != raw:
            raise ValueError('AST raw Source mismatch')
        return capture['ast']


def walk(node, pointer=''):
    if isinstance(node, dict):
        if 'node_type' in node:
            yield node, pointer
        for key, value in node.items():
            yield from walk(value, pointer + '/' + key)
    elif isinstance(node, list):
        for i, value in enumerate(node):
            yield from walk(value, pointer + '/' + str(i))


def typed(node, text, pointer=''):
    """Retain every table field, expression and callback; literals are never evaluated."""
    kind = node['node_type']
    fields = node['fields']
    result = {'kind': kind, 'ast_pointer': pointer, 'span': node.get('span')}
    if result['span']:
        span = result['span']
        result['raw_expression'] = text[span['start_char']:span['end_char_exclusive']]
        result['expression_sha256'] = sha(result['raw_expression'].encode())
    if kind == 'Table':
        result['fields'] = []
        for i, field in enumerate(fields['fields']):
            f = field['fields']
            fp = pointer + '/fields/fields/' + str(i)
            result['fields'].append({'order': i, 'key': typed(f['key'], text, fp + '/fields/key') if f.get('key') else None,
                                     'value': typed(f['value'], text, fp + '/fields/value'),
                                     'between_brackets': f.get('between_brackets', False),
                                     'field_source': typed_span(field, text), 'ast_pointer': fp})
    elif kind == 'Number':
        result['value'] = fields['n']
    elif kind == 'String':
        result['value_bytes_base64'] = fields['s']['bytes_base64']
    elif kind in ('TrueExpr', 'FalseExpr', 'Nil'):
        result['value'] = {'TrueExpr': True, 'FalseExpr': False, 'Nil': None}[kind]
    elif kind == 'Name':
        result['name'] = fields['id']
    elif kind in ('Call', 'Invoke'):
        result['callee'] = typed(fields['func'], text, pointer + '/fields/func')
        result['arguments'] = [typed(a, text, pointer + '/fields/args/' + str(i)) for i, a in enumerate(fields['args'])]
        if kind == 'Invoke':
            result['receiver'] = typed(fields['source'], text, pointer + '/fields/source')
        if kind == 'Call' and fields['func'].get('node_type') == 'Name' and fields['func']['fields']['id'] == 'Position' and len(fields['args']) == 3 and all(a['node_type'] == 'Number' for a in fields['args']):
            result['syntactic_position'] = dict(zip(('x', 'y', 'z'), [a['fields']['n'] for a in fields['args']]))
            result['builtin_identity'] = 'NOT_ASSESSED'
    else:
        result['preservation'] = 'RAW_EXPRESSION_AND_VERIFIED_FULL_AST_REFERENCE'
    return result


def typed_span(node, text):
    span = node.get('span')
    return {'span': span, 'raw': text[span['start_char']:span['end_char_exclusive']] if span else None}


def definition(ast, text):
    statements = ast['fields']['body']['fields']['body']
    out = {'ordered_statements': [], 'config_tables': [], 'boss_constructors': [], 'registrations': [], 'callback_references': []}
    for i, statement in enumerate(statements):
        pointer = '/fields/body/fields/body/' + str(i)
        kind, fields = statement['node_type'], statement['fields']
        record = {'order': i, 'kind': kind, 'ast_pointer': pointer, **typed_span(statement, text)}
        if kind in ('LocalAssign', 'Assign'):
            record['targets'] = [typed(n, text, pointer + '/fields/targets/' + str(j)) for j, n in enumerate(fields['targets'])]
            record['values'] = [typed(n, text, pointer + '/fields/values/' + str(j)) for j, n in enumerate(fields['values'])]
            for j, n in enumerate(fields['values']):
                if n['node_type'] == 'Table':
                    out['config_tables'].append({'statement_order': i, 'value_order': j, 'target': record['targets'][j] if j < len(record['targets']) else None, 'definition': record['values'][j]})
        elif kind in ('Call', 'Invoke'):
            record['definition'] = typed(statement, text, pointer)
            if kind == 'Invoke' and fields['func'].get('fields', {}).get('id') in ('position', 'uid', 'aid', 'register'):
                out['registrations'].append(record['definition'])
        out['ordered_statements'].append(record)
    for node, pointer in walk(ast):
        if node['node_type'] == 'Call' and node['fields']['func'].get('fields', {}).get('id') == 'BossLever':
            out['boss_constructors'].append(typed(node, text, pointer))
        if node['node_type'] in ('AnonymousFunction', 'Function', 'Method'):
            out['callback_references'].append({'ast_pointer': pointer, 'kind': node['node_type'], **typed_span(node, text), 'semantic_admission': False})
    return out


def build(assignment_path, manifest_path, ast_root):
    assignment = json.loads(Path(assignment_path).read_text())
    manifest_path = Path(manifest_path)
    manifest = json.loads(manifest_path.read_text())
    files = {(r['source'], r['revision'], r['path']): r for r in manifest['files']}
    records = assignment['records']
    ids = [r['source_component_id'] for r in records]
    if len(set(ids)) != len(ids):
        raise ValueError('Duplicate component assignment')
    if ast_root is None:
        ast_root = manifest_path.parent.parent / 'ast'
    controller_inputs = [r['shared_controller_source_candidate'] for r in records if r.get('shared_controller_source_candidate')]
    ast_cache = ASTCache(ast_root, {r['provenance']['sha256'] for r in records} | {r['sha256'] for r in controller_inputs})
    controllers = {}
    output = []
    for r in records:
        raw, witness = verified_raw(r['provenance'], files, manifest_path)
        text = raw.decode('utf-8')
        ast = ast_cache.get(witness['sha256'], raw)
        controller = r.get('shared_controller_source_candidate')
        source_refs = [{'role': 'COMPONENT_FULL_SOURCE_AND_AST', **witness}]
        if controller:
            c_raw, c_witness = verified_raw(controller, files, manifest_path)
            cid = ':'.join(c_witness[k] for k in ('source', 'revision', 'path'))
            source_refs.append({'role': 'SHARED_CONTROLLER_CANDIDATE_NOT_ACTIVATION_PROOF', **c_witness})
            if cid not in controllers:
                c_ast = ast_cache.get(c_witness['sha256'], c_raw)
                c_text = c_raw.decode('utf-8')
                controllers[cid] = {'source_component_id': cid, 'provenance': c_witness, 'definition': definition(c_ast, c_text),
                                    'all_tables': [typed(n, c_text, p) for n, p in walk(c_ast) if n['node_type'] == 'Table'],
                                    'semantic_admission': False}
        d = definition(ast, text)
        output.append({'source_component_id': r['source_component_id'], 'provenance': witness, 'definition': d, 'source_refs': source_refs,
                       'canonical_owner_candidates': r.get('same_source_directory_owner_candidates', []),
                       'exact_canonical_owners': r.get('exact_existing_canonical_owners', []),
                       'status': 'TYPED_SOURCE_WITH_SEMANTIC_HOLDS', 'native_admission': False,
                       'holds': ['SHARED_CONTROLLER_ACTIVATION_AND_GLOBAL_BINDINGS_NOT_PROVEN', 'CALLBACK_AND_EXPRESSION_BEHAVIOUR_NOT_LOWERED_TO_RUNTIME', 'CANONICAL_QUEST_OWNERSHIP_NOT_CERTIFIED_BY_DIRECTORY']})
    return {'schema': 'OTERYN_BOSS_COMPONENT_SOURCE/v1', 'scope': 'TYPED_DONOR_SOURCE_NOT_RUNTIME_OR_QUEST_COMPLETENESS', 'records': output,
            'controllers': list(controllers.values()), 'summary': {'records': len(output), 'typed_source_records': len(output), 'semantically_complete_records': 0,
             'config_tables': sum(len(r['definition']['config_tables']) for r in output), 'registrations': sum(len(r['definition']['registrations']) for r in output),
             'callback_references': sum(len(r['definition']['callback_references']) for r in output), 'controller_sources': len(controllers)}, 'native_admission': False}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assignment', required=True)
    p.add_argument('--corpus-manifest', required=True)
    p.add_argument('--ast-root')
    p.add_argument('--out', required=True)
    args = p.parse_args()
    result = build(args.assignment, args.corpus_manifest, args.ast_root)
    Path(args.out).write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps(result['summary'], sort_keys=True))


if __name__ == '__main__':
    main()
