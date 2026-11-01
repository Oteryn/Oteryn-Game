#!/usr/bin/env python3
"""FIXM09: normalize the bounded Crystal gathering operands through the native importer.

Donor 201189f: skinning.lua:1,4,29,51,71,75,109,113,158-172,333-339,356;
data-crystal/scripts/actions/tools/fishing.lua:65-81; items.xml:18878-18881.
"""
import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path

import build_source_item_profiles as native

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = 'tools/content-schema/native-gameplay/gathering-item-profiles.json'
REVISION = '201189f871d23597b82caf88fd611454a2ddefb8'
IDS = {3293, 3453, 3580, 4012, 4026, 4063, 4098, 4138, 4325, 5466, 5876, 5877, 5878, 5905, 5906,
       5908, 5942, 5948, 5969, 5973, 5984, 5995, 6006, 6040, 7158, 7159, 7200,
       7236, 7237, 7441, 7442, 9596}


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
    return {'schema': 'OTERYN_NATIVE_GATHERING_ITEM_PROFILES/v1', 'source_revision': REVISION,
            'source_lines': source_lines,
            'source_files': [{'path': path, 'sha256': hashlib.sha256(raw).hexdigest()}
                             for path, raw in sources],
            'binding_table_sha256': hashlib.sha256(native.BINDINGS.read_bytes()).hexdigest(),
            'records': bound['records']}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('checkout', type=Path)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    data = (json.dumps(build(args.checkout), indent=2, ensure_ascii=False) + '\n').encode()
    if args.check:
        if (ROOT / OUTPUT).read_bytes() != data:
            raise SystemExit('GATHERING_ITEM_PROFILE_DRIFT')
    else:
        (ROOT / OUTPUT).write_bytes(data)
