#!/usr/bin/env python3
"""Refresh the exact Type2 outbox insert inventory; runtime tests verify each tuple."""
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
TARGET = ROOT / 'crates/gameplay/durability/src/durability/item_mint_audit.rs'
roots = ['apps/game-server/src'] + [
    f'crates/gameplay/{package}/src'
    for package in ['foundation', 'content', 'durability']
]
paths = sorted((p for root in roots for p in (ROOT / root).rglob('*.rs')),
               key=lambda p: p.parts)
sites = []
for path in paths:
    text = path.read_text()
    for marker in re.finditer(r'\n#\[cfg\(test\)\]\n', text):
        lines = [line.strip() for line in text[marker.end():].splitlines()
                 if not line.strip().startswith('#')]
        if lines and 'mod ' in lines[0]:
            text = text[:marker.start()]
            break
    if 'INSERT INTO game_item_audit_outbox(' in text:
        sites.append(path.relative_to(ROOT).as_posix())
if not sites:
    raise SystemExit('refusing an empty insert inventory')
text = TARGET.read_text()
start = text.index('    fn every_type2_insert_takes_its_tuple_from_a_type2_transaction()')
pattern = r'(assert_eq!\(\s*sites,\s*\[).*?(\n            \]\n        \);)'
prefix, body = text[:start], text[start:]
rows = ''.join(f'\n                "{site}",' for site in sites)
body, count = re.subn(pattern, lambda m: m[1] + rows + m[2], body,
                      count=1, flags=re.S)
if count != 1:
    raise SystemExit('expected exactly one strict insert inventory')
updated = prefix + body
if TARGET.read_text() != updated:
    TARGET.write_text(updated)
print(f'Refreshed {len(sites)} exact Type2 insert sites')
