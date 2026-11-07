"""Append explicitly reviewed capabilities to compatibility snapshots, preserving legacy entries.

This does not regenerate the previous route table from the current implementation.
Only requested authored fragments can add rows; existing rows may never change here.
"""
import argparse
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

def fragment(name):
    if Path(name).name != name or not name.endswith('.json'):
        raise ValueError('expected capability fragment filename')
    return json.loads((ROOT / 'crates/protocol-oteryn/capabilities' / name).read_text())

def routes(text, entries):
    function = text.index('fn derived_gated_routes_match_previous_table()')
    start = text.index('    let expected: &[(u32, &[u32], &[u32])] = &[\n', function)
    end = text.index('    ];', start)
    body = text[start:end]
    for entry in entries:
        ident = entry['id']
        rows = re.findall(r'\(' + str(ident) + r',\s*&\[([^]]*)\],\s*&\[([^]]*)\]\)', body)
        expected = (entry['commands'], entry['domains'])
        if rows:
            if any(tuple([int(x) for x in re.findall(r'\d+', cell)] for cell in row) != expected for row in rows):
                raise ValueError('existing route row differs; review ABI instead of rewriting it')
        else:
            commands = ', '.join(map(str, expected[0]))
            domains = ', '.join(map(str, expected[1]))
            body += f'        ({ident}, &[{commands}], &[{domains}]),\n'
    return text[:start] + body + text[end:]

def supported(text, entries):
    marker = text.index('        CLIENT_SUPPORTED_CAPABILITIES,')
    start = text.index('        &[\n', marker)
    end = text.index('\n        ]', start)
    body = text[start:end]
    for entry in entries:
        ident = entry['id']
        existing = [int(x) for x in re.findall(r'\d+', body)]
        if ident not in existing:
            if existing and ident <= max(existing):
                raise ValueError('new supported capability must append above the previous ABI')
            last = list(re.finditer(r'\d+', body))[-1].end()
            body = body[:last] + f', {ident}' + body[last:]
    return text[:start] + body + text[end:]

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--route', action='append', default=[])
    parser.add_argument('--support', action='append', default=[])
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for relative, transform, names in [
        ('crates/session/src/session_tests_capabilities.rs', routes, args.route),
        ('crates/session/src/session_tests_chat.rs', supported, args.support),
    ]:
        path = ROOT / relative
        before = path.read_text()
        after = transform(before, [fragment(name) for name in names])
        if args.check:
            if before != after:
                raise ValueError(f'session compatibility snapshot drift: {relative}')
        elif after != before:
            path.write_text(after)
    print('explicit session compatibility additions ' + ('verified' if args.check else 'generated'))

if __name__ == '__main__':
    main()
