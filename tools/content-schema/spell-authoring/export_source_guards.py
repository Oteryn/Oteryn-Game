"""Export lexical source controls, not a Lua AST or executable gameplay rules.

Every event retains its exact byte-independent character span and source line.
Conditions remain unresolved expressions; only scalar literal call arguments
are normalized. No source callback, Lua VM, network or world API is executed.
"""
import argparse
import gzip
import hashlib
import json
import re
import subprocess
from collections import Counter
from pathlib import Path

SCHEMA = 'OTERYN_PLAYER_SOURCE_GUARDS/v1'
TOKEN = re.compile(r'--\[(=*)\[.*?\]\1\]|--[^\n]*|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'|\[(=*)\[.*?\]\2\]|[A-Za-z_]\w*|(?:\d+(?:\.\d*)?|\.\d+)|[^\s]', re.S)
DOMAINS = {
    'vocation': r'vocation', 'premium': r'premium', 'item': r'item|weapon|equipment|charge',
    'target': r'target|variant|summon|creature', 'tile': r'tile|position|protectionzone|blocking',
    'cooldown': r'cooldown|exhaust', 'soul': r'soul', 'harmony': r'harmony|serene|spiritual',
    'cancellation': r'cancel|return\s+false', 'effect': r'combat|condition|effect|sound|health|mana|event',
}


def literal(expression):
    text = expression.strip()
    value, kind = None, 'unresolved_expression'
    if text in ('true', 'false'):
        value, kind = text == 'true', 'boolean'
    elif text == 'nil':
        kind = 'nil'
    elif re.fullmatch(r'-?\d+(?:\.\d+)?', text):
        value, kind = (float(text) if '.' in text else int(text)), 'number'
    elif re.fullmatch(r'"[^"\\]*"|\'[^\'\\]*\'', text):
        value, kind = text[1:-1], 'string'
    identifier = text if re.fullmatch(r'[A-Za-z_]\w*(?:[.:][A-Za-z_]\w*)*', text) and kind == 'unresolved_expression' else None
    return {'expression': text, 'kind': kind, 'value': value, 'identifier': identifier}


def capture(text):
    if len(text.encode()) > 1048576:
        raise ValueError('source exceeds one MiB bounded lexical capture')
    tokens = [m for m in TOKEN.finditer(text) if not m.group().startswith('--')]
    if len(tokens) > 100000:
        raise ValueError('source token capture limit')
    events = []
    def add(kind, start, end, identifier, expression, arguments=()):
        masked = TOKEN.sub(lambda m: ' ' * len(m.group()) if m.group().startswith(('"', "'", '--', '[')) else m.group(), expression)
        identifiers = list(dict.fromkeys(v for v in re.findall(r'\b[A-Za-z_]\w*(?:[.:][A-Za-z_]\w*)*', masked)
                                        if v not in ('if','elseif','else','then','end','function','return','not','and','or','true','false','nil','do','until','repeat')))
        events.append({'order': len(events), 'kind': kind, 'source_line': text.count('\n', 0, start) + 1,
                       'start': start, 'end': end, 'identifier': identifier, 'expression': expression,
                       'identifiers': identifiers,
                       'expression_resolution': 'unresolved_expression' if kind in ('guard', 'return') else 'lexical_source_fact',
                       'domains': [domain for domain, pattern in DOMAINS.items() if re.search(pattern, expression, re.I)],
                       'arguments': list(arguments)})
    guards = 0
    for i, token in enumerate(tokens):
        word = token.group()
        if word in ('if', 'elseif'):
            guards += 1
            stop = next((m for m in tokens[i + 1:] if m.group() in ('then', 'end')), None)
            end = stop.start() if stop else token.end()
            add('guard', token.start(), end, word, text[token.end():end].strip())
        elif word in ('else', 'end', 'then', 'do', 'repeat', 'until'):
            add('control_marker', token.start(), token.end(), word, word)
        elif word == 'function':
            stop = next((m for m in tokens[i + 1:] if m.group() == '('), None)
            end = stop.start() if stop else token.end()
            add('function_declaration', token.start(), end, text[token.end():end].strip() or 'anonymous', text[token.start():end].strip())
        elif word == 'return':
            end = text.find('\n', token.end())
            end = len(text) if end < 0 else end
            stop = next((m for m in tokens[i + 1:] if m.start() < end and m.group() in ('end', 'else', 'elseif')), None)
            end = stop.start() if stop else end
            add('return', token.start(), end, 'return', text[token.start():end].strip(),
                [literal(text[token.end():end])] if text[token.end():end].strip() else [])
        elif re.fullmatch(r'[A-Za-z_]\w*', word):
            # Only qualified calls: avoid turning function declarations/local bare names into semantic controls.
            j = i
            while j + 2 < len(tokens) and tokens[j + 1].group() in ('.', ':') and re.fullmatch(r'[A-Za-z_]\w*', tokens[j + 2].group()):
                j += 2
            if j == i or j + 1 >= len(tokens) or tokens[j + 1].group() != '(' or (i and tokens[i - 1].group() in ('.', ':', 'function')):
                continue
            depth, arg_start, args, close = 0, tokens[j + 1].end(), [], None
            for m in tokens[j + 2:]:
                value = m.group()
                if value in ('(', '{', '['):
                    depth += 1
                elif value == ')' and depth == 0:
                    if m.start() > arg_start and text[arg_start:m.start()].strip():
                        args.append(literal(text[arg_start:m.start()]))
                    close = m
                    break
                elif value in (')', '}', ']'):
                    depth -= 1
                elif value == ',' and depth == 0:
                    args.append(literal(text[arg_start:m.start()]))
                    arg_start = m.end()
            if close:
                add('qualified_call', token.start(), close.end(), text[token.start():tokens[j].end()],
                    text[token.start():close.end()], args)
    events.sort(key=lambda row: (row['start'], row['end'], row['kind']))
    for order, event in enumerate(events):
        event['order'] = order
    assert guards == sum(event['kind'] == 'guard' for event in events)
    return {'events': events, 'guard_count': guards, 'control_structure_resolution': 'unresolved_lexical_only',
            'source_execution': False}


def export(registrars, callback_facts, source_git, out):
    import jsonschema
    schema = json.loads(Path(__file__).with_name('player-source-guards.schema.json').read_text())
    fact_rows = list(map(json.loads, gzip.decompress(callback_facts.read_bytes()).splitlines()))
    facts = {row['registration_key']: row for row in fact_rows}
    if len(facts) != len(fact_rows):
        raise ValueError('duplicate callback registration identity')
    rows, cache, source_digests = [], {}, {}
    keys = set()
    for row in map(json.loads, gzip.decompress(registrars.read_bytes()).splitlines()):
        if row['snapshot'] not in ('canary-main-current', 'crystal-summer-current'):
            continue
        source = row['snapshot'].split('-')[0]
        if row['registration_key'] in keys:
            raise ValueError('duplicate registrar identity')
        keys.add(row['registration_key'])
        fact = facts.get(row['registration_key'])
        if fact and (fact['source_revision'] != row['source_revision'] or fact['source_sha256'] != row['source_sha256']):
            raise ValueError('callback fact source identity mismatch')
        key = source, row['source_revision'], row['source_file']
        if key not in cache:
            raw = subprocess.check_output(['git', '-C', str(source_git / source), 'show', key[1] + ':' + key[2]])
            source_digests[key] = (hashlib.sha256(raw).hexdigest(), hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest())
            cache[key] = capture(raw.decode('utf-8'))
        if source_digests[key] != (row['source_sha256'], row['git_blob']):
            raise ValueError('source digest mismatch ' + row['registration_key'])
        record = {'schema': SCHEMA, 'registration_key': row['registration_key'], 'snapshot': row['snapshot'],
                  'source_revision': row['source_revision'], 'source_file': row['source_file'],
                  'source_sha256': row['source_sha256'], 'git_blob': row['git_blob'],
                  'callback_facts_present': row['registration_key'] in facts, 'runtime_activation': False,
                  'event_scope': 'file_unresolved', 'callback_binding_resolution': 'unresolved_whole_file',
                  'scope': 'whole_source_file_lexical_evidence_not_registration_control_flow', **cache[key]}
        jsonschema.validate(record, schema)
        rows.append(record)
    current_fact_keys = {key for key in facts if key.split('/')[0] in ('canary-main-current','crystal-summer-current')}
    if keys != current_fact_keys:
        raise ValueError('registrar/callback population mismatch')
    payload = b''.join((json.dumps(row, sort_keys=True, separators=(',', ':')) + '\n').encode() for row in rows)
    out.write_bytes(gzip.compress(payload, mtime=0))
    proof = {'schema': SCHEMA + '/proof', 'records': len(rows), 'unique_source_files': len(cache),
             'record_count': len(rows), 'format': 'jsonl_gzip', 'gzip_path': out.as_posix(),
             'gzip_sha256': hashlib.sha256(out.read_bytes()).hexdigest(),
             'schema_path': Path(__file__).with_name('player-source-guards.schema.json').relative_to(Path(__file__).resolve().parents[3]).as_posix(),
             'source_revisions': {snapshot: next(row['source_revision'] for row in rows if row['snapshot']==snapshot)
                                  for snapshot in sorted({row['snapshot'] for row in rows})},
             'input_registrars_sha256': hashlib.sha256(registrars.read_bytes()).hexdigest(),
             'input_callback_facts_sha256': hashlib.sha256(callback_facts.read_bytes()).hexdigest(),
             'payload_sha256': hashlib.sha256(payload).hexdigest(), 'archive_sha256': hashlib.sha256(out.read_bytes()).hexdigest(),
             'schema_sha256': hashlib.sha256(Path(__file__).with_name('player-source-guards.schema.json').read_bytes()).hexdigest(),
             'source_pins': sorted({row['source_revision'] for row in rows}),
             'events': sum(len(row['events']) for row in rows), 'guards': sum(row['guard_count'] for row in rows),
             'by_snapshot': dict(Counter(row['snapshot'] for row in rows)), 'all_records_schema_valid': True,
             'runtime_activation': False, 'external_sources_used': 0,
             'limitations': ['Lexical whole-file source evidence; not a Lua AST, CFG or normalized executable guard.',
                             'Conditions and nonliteral argument expressions remain unresolved; no callbacks executed.',
                             'Repeated registration records may share source-file events; event totals are not unique mechanics.']}
    out.with_name('player-source-guards-proof.json').write_text(json.dumps(proof, indent=2) + '\n')
    return proof


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--registrars', type=Path, required=True)
    parser.add_argument('--callback-facts', type=Path, required=True)
    parser.add_argument('--source-git', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(export(args.registrars, args.callback_facts, args.source_git, args.out)))
