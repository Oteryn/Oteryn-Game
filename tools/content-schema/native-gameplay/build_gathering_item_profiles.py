#!/usr/bin/env python3
"""FIXM09: normalize the bounded Crystal gathering operands through the native importer.

Donor 201189f: skinning.lua:1,4,29,51,71,75,109,113,158-172,333-339,356;
data-crystal/scripts/actions/tools/fishing.lua:65-81; items.xml:18878-18881.
P11 mechanical rod: data-global/scripts/actions/other/mechanical_fishing.lua:334-378;
items.xml:25156-25159. Its already qualified Global 920 weight is retained over XML 919.
"""
import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

import build_source_item_profiles as native

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools/content-migration'))
from gathering_item_profiles import mechanical_rod_policy
OUTPUT = 'tools/content-schema/native-gameplay/gathering-item-profiles.json'
REVISION = '201189f871d23597b82caf88fd611454a2ddefb8'
IDS = {3580, 4012, 4026, 4063, 4098, 4138, 4325, 5876, 5877, 5878, 5905, 5906,
       5908, 5942, 5948, 5969, 5973, 5984, 5995, 6006, 6040, 7158, 7159, 7200,
       7236, 7237, 7441, 7442, 9306}


def build(checkout):
    if subprocess.check_output(['git', '-C', str(checkout), 'rev-parse', 'HEAD'], text=True).strip() != REVISION:
        raise ValueError('GATHERING_DONOR_REVISION')
    if subprocess.check_output(['git', '-C', str(checkout), 'status', '--porcelain', '--',
                                'data/items/items.xml', 'data/items/appearances.dat'], text=True).strip():
        raise ValueError('GATHERING_DONOR_DIRTY')
    sources = [(path, (checkout / path).read_bytes()) for path in
               ('data/items/items.xml', 'data/items/appearances.dat')]
    xml, appearances = [raw for _, raw in sources]
    source_lines = {}
    for line_number, line in enumerate(xml.decode().splitlines(), 1):
        if '<item ' not in line:
            continue
        low = re.search(r'\b(?:id|fromid)="(\d+)"', line)
        high = re.search(r'\btoid="(\d+)"', line)
        if low:
            first = int(low[1])
            last = int(high[1]) if high else first
            for item in IDS:
                if first <= item <= last:
                    source_lines[str(item)] = f'data/items/items.xml:{line_number}'
    if set(map(int, source_lines)) != IDS:
        raise ValueError('GATHERING_DONOR_ITEM_LINES')
    bindings = native.binding_index(native.BINDINGS.read_bytes())
    index = native.xml_index(xml)
    objects = native.engine_items.load_appearance_objects(appearances)
    physical = {row['server_item_id']: row for row in native.policies(appearances, xml, IDS)}
    records = [native.profile(item, index[item], objects[item], physical[item], bindings, None)
               for item in sorted(IDS)]
    bound = native.bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records},
                                 bindings, set(index))
    binding, qualified_physical, proof = mechanical_rod_policy(ROOT)
    mechanical, = [r for r in bound['records'] if r['production_definition']['production_key'] == binding['target']['key']]
    donor_physical = json.loads(json.dumps(qualified_physical))
    donor_physical['value']['weight']['value'] = 919
    if (index[9306].get('name') != 'mechanical fishing rod'
            or mechanical['semantics']['physical'] != donor_physical
            or mechanical['production_binding'] != binding):
        raise ValueError('GATHERING_MECHANICAL_DONOR_POLICY')
    mechanical['semantics']['physical'] = qualified_physical
    return {'schema': 'OTERYN_NATIVE_GATHERING_ITEM_PROFILES/v1', 'source_revision': REVISION,
            'source_lines': source_lines,
            'source_files': [{'path': path, 'sha256': hashlib.sha256(raw).hexdigest()}
                             for path, raw in sources],
            'binding_table_sha256': hashlib.sha256(native.BINDINGS.read_bytes()).hexdigest(),
            'qualified_overrides': [proof],
            'records': bound['records']}


def refresh_rekey_proof():
    """Re-pin only unchanged rod facts after an approved offline snapshot re-key."""
    from source_relocations import read_pinned
    packet = json.loads((ROOT / OUTPUT).read_bytes())
    previous, = packet['qualified_overrides']
    current = mechanical_rod_policy(ROOT)[2]
    if previous.keys() != current.keys():
        raise ValueError('GATHERING_REKEY_PROOF_SHAPE')
    if {k: v for k, v in previous.items() if k != 'inputs'} != {
            k: v for k, v in current.items() if k != 'inputs'}:
        raise ValueError('GATHERING_REKEY_CHANGED_FACTS')
    if len(previous['inputs']) != len(current['inputs']):
        raise ValueError('GATHERING_REKEY_INPUT_SET')
    for old, new in zip(previous['inputs'], current['inputs']):
        if old['path'] != new['path'] or read_pinned(ROOT, old['path'], old['sha256']) != (ROOT / new['path']).read_bytes():
            raise ValueError('GATHERING_REKEY_UNREVIEWED_INPUT')
    packet['qualified_overrides'] = [current]
    (ROOT / OUTPUT).write_text(json.dumps(packet, indent=2, ensure_ascii=False) + '\n')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('checkout', type=Path, nargs='?')
    parser.add_argument('--refresh-rekey-proof', action='store_true')
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    if args.refresh_rekey_proof:
        if args.checkout is not None or args.check:
            parser.error('--refresh-rekey-proof cannot capture or check a donor')
        refresh_rekey_proof()
        raise SystemExit(0)
    if args.checkout is None:
        parser.error('checkout is required for donor capture')
    data = (json.dumps(build(args.checkout), indent=2, ensure_ascii=False) + '\n').encode()
    if args.check:
        if (ROOT / OUTPUT).read_bytes() != data:
            raise SystemExit('GATHERING_ITEM_PROFILE_DRIFT')
    else:
        (ROOT / OUTPUT).write_bytes(data)
