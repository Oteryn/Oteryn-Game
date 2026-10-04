#!/usr/bin/env python3
"""Offline lexical Lua mechanics evidence. Never evaluates Lua or allocates runtime IDs."""
import argparse
import collections
import hashlib
import gzip
import json
import pathlib
import re
import subprocess

SCHEMA = 'OTERYN_SOURCE_MECHANICS_INVENTORY/v1'
KNOWN_METHODS = {'setParameter', 'setFormula', 'setArea', 'addDamage', 'setCallback', 'addCondition', 'execute'}
SOURCES = {'canary': '04b83b512114bfd888000d6e1433ed8ecaec7c5b',
           'crystal': '00ce02a57ca5a12e48f32a3476e37471167e4c3f'}
REGISTRAR_METHODS = set(json.loads((pathlib.Path(__file__).with_name('spell-source-registrar.schema.json')).read_text())['properties']) | {'register'}
TOKEN = re.compile(r'--\[(=*)\[.*?\]\1\]|--[^\n]*|\[(=*)\[.*?\]\2\]|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'|[A-Za-z_][A-Za-z_0-9]*|\d+(?:\.\d+)?|\.\.\.|\.\.|==|~=|<=|>=|[^\s]', re.S)


def lex(source):
    result = []
    for m in TOKEN.finditer(source):
        value = m.group()
        if value.startswith('--'):
            continue
        result.append({'value': value, 'line': source.count('\n', 0, m.start()) + 1,
                       'offset': m.start()})
    return result


def opaque(tokens, reason='expression_requires_lua_semantics'):
    values = [t['value'] for t in tokens]
    return {'kind': 'opaque_expression', 'reason': reason,
            'token_count': len(values), 'token_sha256': hashlib.sha256('\0'.join(values).encode()).hexdigest(),
            'numeric_literal_terms': [float(v) if '.' in v else int(v) for v in values if re.fullmatch(r'\d+(?:\.\d+)?', v)],
            'symbol_dependencies': sorted({v for v in values if re.fullmatch(r'[A-Za-z_]\w*', v)})}


def split_args(tokens):
    result, start, stack, blocks = [], 0, [], []
    pairs = {'(': ')', '[': ']', '{': '}'}
    pending_do = 0
    for i, token in enumerate(tokens):
        v = token['value']
        if v == 'function' or v == 'if':
            blocks.append(v)
        elif v in ('for', 'while'):
            blocks.append(v); pending_do += 1
        elif v == 'do':
            if pending_do:
                pending_do -= 1
            else:
                blocks.append(v)
        elif v == 'repeat':
            blocks.append(v)
        elif v in ('end', 'until') and blocks:
            blocks.pop()
        if v in pairs:
            stack.append(pairs[v])
        elif stack and v == stack[-1]:
            stack.pop()
        elif v == ',' and not stack and not blocks:
            result.append(tokens[start:i]); start = i + 1
    if tokens[start:]:
        result.append(tokens[start:])
    return result


def receiver_start(tokens, method_index):
    if method_index < 2 or tokens[method_index-1]['value'] not in ('.', ':'):
        return method_index

    def atom_start(end):
        j = end
        if tokens[j]['value'] in (')', ']'):
            closing = tokens[j]['value']; opening = '(' if closing == ')' else '['
            depth = 1; j -= 1
            while j >= 0 and depth:
                if tokens[j]['value'] == closing:
                    depth += 1
                elif tokens[j]['value'] == opening:
                    depth -= 1
                j -= 1
            if j >= 0 and (closing == ']' or re.fullmatch(r'[A-Za-z_]\w*', tokens[j]['value'])):
                j = atom_start(j)
            else:
                j += 1
        while j >= 2 and tokens[j-1]['value'] in ('.', ':'):
            j = atom_start(j-2)
        return max(0, j)
    return atom_start(method_index - 2)


def argument(tokens):
    if not tokens:
        return opaque(tokens, 'empty_argument')
    values = [t['value'] for t in tokens]
    if 'function' in values:
        return opaque(tokens, 'anonymous_function_body_not_exported')
    if len(values) == 1:
        v = values[0]
        if re.fullmatch(r'\d+(?:\.\d+)?', v):
            return {'kind': 'number_literal', 'value': float(v) if '.' in v else int(v)}
        if v in ('true', 'false', 'nil'):
            return {'kind': 'literal', 'value': {'true': True, 'false': False, 'nil': None}[v]}
        if v.startswith(('"', "'", '[')):
            return {'kind': 'string_literal_token', 'token': v}
        return {'kind': 'symbol', 'name': v}
    if len(values) == 2 and values[0] == '-' and re.fullmatch(r'\d+(?:\.\d+)?', values[1]):
        return {'kind': 'number_literal', 'value': -float(values[1]) if '.' in values[1] else -int(values[1])}
    if values[0] == '{' and values[-1] == '}':
        return {'kind': 'table_expression', 'entries': [argument(x) for x in split_args(tokens[1:-1])]}
    if all(v not in ('end', 'function', 'then', 'do') for v in values):
        return {'kind': 'expression_tokens', 'tokens': values, 'execution_supported': False}
    return opaque(tokens)


def scan(source):
    tokens = lex(source); calls = []; declarations = []; branches = []; expressions = []; declaration_terminals = set(); constructors = {}; spell_count = 0
    for i, t in enumerate(tokens):
        v = t['value']
        if v in ('if', 'elseif', 'for', 'while', 'repeat'):
            j = i + 1
            while j < len(tokens) and tokens[j]['value'] not in ('then', 'do', 'until', 'end') and j - i < 128:
                j += 1
            branches.append({'line': t['line'], 'kind': v, 'condition': argument(tokens[i+1:j])})
        if v == 'function':
            j = i + 1; name = []
            while j < len(tokens) and tokens[j]['value'] != '(' and j - i < 20:
                name.append(tokens[j]['value']); j += 1
            if not name and i > 1 and tokens[i-1]['value'] == '=':
                begin = i - 2
                while begin > 1 and tokens[begin-1]['value'] in ('.', ':'):
                    begin -= 2
                name = [x['value'] for x in tokens[begin:i-1]]
            if not name:
                name = [f"<anonymous:{t['line']}:{t['offset']}>"]
            if name:
                declaration_terminals.add(j - 1)
                declarations.append({'line': t['line'], 'identity': ''.join(name), 'scope_status': 'declaration_only_body_not_exported'})
        if not re.fullmatch(r'[A-Za-z_]\w*', v) or v in {'function', 'if', 'for', 'while', 'return', 'and', 'or', 'not'}:
            continue
        j = i + 1
        if j >= len(tokens) or tokens[j]['value'] != '(' or i in declaration_terminals:
            continue
        begin = receiver_start(tokens, i)
        name = [x['value'] for x in tokens[begin:i+1]]
        depth = 1; k = j + 1
        while k < len(tokens) and depth:
            if tokens[k]['value'] == '(':
                depth += 1
            elif tokens[k]['value'] == ')':
                depth -= 1
            k += 1
        full = ''.join(name) if len(name) <= 64 and 'function' not in name else 'opaque_receiver:' + v; method = v
        known = method in KNOWN_METHODS or method in REGISTRAR_METHODS or full in ('Spell', 'Combat', 'Condition', 'createCombatArea', 'addEvent', 'setCombatCallback', 'doTargetCombatHealth')
        args = [argument(a) for a in split_args(tokens[j+1:k-1])] if depth == 0 else []
        declaration = None
        if full == 'Spell':
            variable = tokens[i-2]['value'] if i > 1 and tokens[i-1]['value'] == '=' else None
            declaration = {'constructor_ordinal': spell_count, 'line': t['line'], 'variable': variable}
            spell_count += 1
            if variable:
                constructors[variable] = declaration
        registrar_shape = ('getter_candidate' if not args else 'setter_candidate') if method in REGISTRAR_METHODS else None
        calls.append({'line': t['line'], 'end_line': tokens[k-1]['line'] if k else t['line'],
                      'source_order': len(calls), 'call_identity': full,
                      'category': 'typed_known_call' if known else 'unsupported_call_reference',
                      'arguments': args, 'argument_count': len(args),
                      'spell_constructor_declaration': declaration,
                      'preceding_receiver_spell_constructor': constructors.get(name[0]) if len(name) > 1 else None,
                      'registrar_method_shape': registrar_shape,
                      'receiver_binding_limit': 'textual_preceding_assignment_not_proven_lua_object_or_scope',
                      'receiver_expression': argument(tokens[begin:i-1]) if begin < i else None,
                      'capture_status': 'lexical_not_executed' if depth == 0 else 'unbalanced_call',
                      'lexical_declaration_context': next((d['identity'] for d in reversed(declarations) if d['line'] <= t['line']), None),
                      'context_limit': 'preceding_declaration_not_proven_lexical_scope',
                      'scheduled': full == 'addEvent'})
    for line in sorted({t['line'] for t in tokens}):
        row = [t for t in tokens if t['line'] == line]
        vals = [t['value'] for t in row]
        if 'return' in vals:
            idx = vals.index('return'); tail = row[idx+1:]; stop = next((n for n, t in enumerate(tail) if t['value'] in ('end', 'else', 'elseif', ';')), len(tail)); expressions.append({'line': line, 'kind': 'return_expression', 'value': argument(tail[:stop]), 'limit': 'line_fragment_not_complete_statement'})
        if '=' in vals:
            idx = vals.index('='); expressions.append({'line': line, 'kind': 'assignment_expression', 'value': argument(row[idx+1:]), 'limit': 'line_fragment_not_complete_statement'})
    return {'calls': calls, 'expression_statements': expressions, 'function_declarations': declarations, 'branches': branches,
            'capture_limits': ['Lexical extraction does not prove registration, reachability, variable type or behavior execution.',
                               'Conditions and areas expressed as Lua tables remain source expressions, not engine-applied values.',
                               'Declaration context identifies preceding declaration only; nested lexical scope is not asserted.']}


def git(repo, *args):
    return subprocess.check_output(['git', '-C', str(repo), *args])


def build(source_root, referenced):
    files = []; populations = []
    for source, revision in SOURCES.items():
        repo = pathlib.Path(source_root) / source
        tree = git(repo, 'ls-tree', '-r', revision).decode().splitlines()
        all_lua = []
        for line in tree:
            meta, path = line.split('\t', 1); blob = meta.split()[2]
            if path.endswith('.lua'):
                all_lua.append((path, blob))
        proc = subprocess.run(['git', '-C', str(repo), 'cat-file', '--batch'],
                              input=('\n'.join(blob for _, blob in all_lua)+'\n').encode(), stdout=subprocess.PIPE, check=True)
        raw = proc.stdout; pos = 0; payloads = {}; selected = {}; helper_suffixes = set()
        for path, blob in all_lua:
            end = raw.index(b'\n', pos); header = raw[pos:end].decode().split(); size = int(header[2]); start = end + 1
            payload = raw[start:start+size]; pos = start + size + 1; payloads[path] = (blob, payload)
            text = payload.decode('utf-8', errors='replace')
            constructor = bool(re.search(r'\bSpell\s*\(', text)) and any(t['value'] == 'Spell' and n + 1 < len(ts) and ts[n+1]['value'] == '(' for ts in [lex(text)] for n, t in enumerate(ts))
            if re.search(r'/(?:spells|runes)/', path):
                selected[path] = 'spell_or_rune_directory'
            elif path in referenced.get(source, set()):
                selected[path] = 'monster_referenced_script'
            elif constructor:
                selected[path] = 'whole_tree_spell_constructor'
        # Preserve locally available helper files referenced by quoted Lua paths.
        pending = list(selected)
        while pending:
            path = pending.pop(); text = payloads[path][1].decode('utf-8', errors='replace')
            for token in lex(text):
                match = re.fullmatch(r'[\"\']([^\"\']+\.lua)[\"\']', token['value'])
                if match:
                    suffix = match[1].lstrip('/')
                    for helper in payloads:
                        if helper.endswith('/' + suffix) or helper == suffix:
                            if helper not in selected:
                                selected[helper] = 'source_helper_dependency'; pending.append(helper)
        for path in sorted(selected):
            blob, payload = payloads[path]; size = len(payload)
            facts = scan(payload.decode('utf-8', errors='replace'))
            files.append({'source': source, 'revision': revision, 'path': path, 'git_blob': blob,
                          'sha256': hashlib.sha256(payload).hexdigest(), 'bytes': size,
                          'selection': selected[path], **facts})
        populations.append({'source': source, 'revision': revision, 'tracked_selected_files': len(selected),
                            'captured_files': sum(f['source'] == source for f in files),
                            'referenced_scripts_missing_from_tree': sorted(referenced.get(source, set()) - set(selected))})
    count = collections.Counter(c['category'] for f in files for c in f['calls'])
    return {'schema': SCHEMA, 'runtime_activation': False, 'source_populations': populations,
            'counts': {'files': len(files), 'calls': sum(count.values()), **dict(count)}, 'files': files}


def main():
    p = argparse.ArgumentParser(); p.add_argument('--source-root', required=True); p.add_argument('--monster-slots', required=True); p.add_argument('--out', required=True); a = p.parse_args()
    refs = collections.defaultdict(set)
    for row in json.loads(pathlib.Path(a.monster_slots).read_text()):
        if row.get('registered_source'):
            refs[row['source']].add(row['registered_source']['path'])
    result = build(a.source_root, refs)
    result['monster_slots_sha256'] = hashlib.sha256(pathlib.Path(a.monster_slots).read_bytes()).hexdigest()
    payload = (json.dumps(result, ensure_ascii=False, indent=2)+'\n').encode()
    output = pathlib.Path(a.out)
    if output.suffix == '.gz':
        output.write_bytes(gzip.compress(payload, mtime=0))
    else:
        output.write_bytes(payload)


if __name__ == '__main__':
    main()
