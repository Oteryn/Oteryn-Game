#!/usr/bin/env python3
"""Refresh the reviewed Rust key-33 inventory; the Rust test remains the gate.

Use --reader path::function only after reviewing that lock-only reader. The
refresh refuses a reader containing a direct key-33 table mutation. It never
changes lane proofs, delegated callers, SQL pins or test assertions.
"""
import argparse
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
PIN = ROOT / 'apps/game-server/tests/integration_rest/spell_lane_key33_pin.rs'
TABLE = r'(?:game_item_ground_locations|game_item_corpse_container_entries|game_native_map_scope_adoptions)\b'
WRITE = re.compile(r'(?:INSERT INTO|DELETE FROM|UPDATE) ' + TABLE)
LOCK = re.compile(r"'hex'\), ?33\)")
ROW = re.compile(r'\("([^"]+)", "([^"]+)"\)')


def block(text, name):
    return re.search(r'const ' + name + r':[^=]+?= &\[(.*?)\n\];', text, re.S)


def refresh(readers):
    original = text = PIN.read_text()
    registered = block(text, 'NON_LANE_KEY33_READS')
    non_lane = set(ROW.findall(registered[1]))
    for reader in readers:
        path, name = reader.rsplit('::', 1)
        non_lane.add((path, name))
    sites, writes = set(), set()
    roots = [ROOT / 'apps/game-server/src', *(ROOT / 'crates/gameplay').glob('*/src')]
    for root in roots:
        for path in sorted(root.rglob('*.rs')):
            relative = path.relative_to(ROOT)
            if any(p == 'tests' or p.endswith('_tests') for p in relative.parts) or path.name.endswith('_tests.rs'):
                continue
            source = re.split(r'#\[cfg\(test\)\]\s*(?:#\[allow\([^]]*\)\]\s*)?(?:pub\(crate\) )?mod ', path.read_text())[0]
            source = ' '.join(source.split())
            for pattern, destination in [(LOCK, sites), (WRITE, writes)]:
                for match in pattern.finditer(source):
                    functions = list(re.finditer(r'\bfn (\w+)', source[:match.start()]))
                    if not functions:
                        raise ValueError(f'{relative}: site outside function')
                    destination.add((relative.as_posix(), functions[-1][1]))
    sites |= writes
    if not non_lane <= sites or non_lane & writes:
        raise ValueError('non-lane readers must be present and contain no key-33 table writes')
    for name, values in [('RUST_KEY33_SITES', sites), ('NON_LANE_KEY33_READS', non_lane)]:
        match = block(text, name)
        rows = ''.join(f'    ("{path}", "{function}"),\n' for path, function in sorted(values))
        text = text[:match.start(1)] + '\n' + rows.rstrip('\n') + text[match.end(1):]
    if text != original:
        PIN.write_text(text)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--reader', action='append', default=[], metavar='PATH::FUNCTION')
    refresh(parser.parse_args().reader)
