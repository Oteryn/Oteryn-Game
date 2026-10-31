#!/usr/bin/env python3
"""I19: normalize Crystal enchanting operands through the existing native importer.

Donor 201189f: enchanting.lua:13-41,76-102,126-133 and data/items/items.xml.
"""
import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path

import build_source_item_profiles as native

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = 'tools/content-schema/native-gameplay/enchanting-item-profiles.json'
REVISION = '201189f871d23597b82caf88fd611454a2ddefb8'
IDS = (set(range(146, 162)) | {3030, 3029, 3032, 3033, 675, 676, 677, 678,
       3271, 7383, 7384, 7406, 7402, 3317, 3318, 7389, 7380, 3342, 3311, 3333,
       7415, 7392, 3279, 3447, 8077} | set(range(660, 675)) | set(range(679, 694))
       | set(range(779, 799)) | set(range(801, 811)) | {761, 762, 763, 774, 8078, 8079, 8080, 8081})



def build(checkout):
    if subprocess.check_output(['git', '-C', str(checkout), 'rev-parse', 'HEAD'], text=True).strip() != REVISION:
        raise ValueError('ENCHANTING_DONOR_REVISION')
    if subprocess.check_output(['git', '-C', str(checkout), 'status', '--porcelain', '--',
                                'data/items/items.xml', 'data/items/appearances.dat',
                                'data-global/scripts/actions/other/enchanting.lua'], text=True).strip():
        raise ValueError('ENCHANTING_DONOR_DIRTY')
    sources = [(path, (checkout / path).read_bytes()) for path in
               ('data/items/items.xml', 'data/items/appearances.dat',
                'data-global/scripts/actions/other/enchanting.lua')]
    xml, appearances = [raw for _, raw in sources[:2]]
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
        raise ValueError('ENCHANTING_DONOR_ITEM_LINES')
    bindings = native.binding_index(native.BINDINGS.read_bytes())
    index = native.xml_index(xml)
    objects = native.engine_items.load_appearance_objects(appearances)
    physical = {row['server_item_id']: row for row in native.policies(appearances, xml, IDS)}
    records = [native.profile(item, index[item], objects[item], physical[item], bindings, None)
               for item in sorted(IDS)]
    # The action installs 1000 charges on every non-ammunition result (enchanting.lua:126-133).
    weapons = set(range(660, 675)) | set(range(679, 694)) | set(range(779, 799)) | set(range(801, 811))
    for item, record in zip(sorted(IDS), records):
        if item in weapons:
            record['semantics']['charges'] = native.K({'count': native.K(1000)})
        if item in weapons | {761, 762, 763, 774, 8078, 8079, 8080, 8081}:
            attrs = native.attrs(index[item])
            elements = [{'element': element.upper(), 'points': native.K(int(attrs['element' + element]))}
                        for element in ('earth', 'energy', 'fire', 'ice') if 'element' + element in attrs]
            record['semantics']['weapon']['value']['elemental'] = native.K(elements)
    bound = native.bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records},
                                 bindings, set(index))
    action = sources[2][1].decode()
    def array(name):
        return [int(n) for n in re.findall(r'\d+', re.search(r'local ' + name + r' = \{([^}]+)\}', action)[1])]
    altars = action.split('local enchantingAltars = {', 1)[1].split('local enchantedGems', 1)[0]
    matrix = action.split('local enchantedItems = {', 1)[1].split('local enchanting =', 1)[0]
    rules = {'gems': array('enchantableGems'), 'enchanted': array('enchantedGems'),
             'altars': [[int(n) for n in re.findall(r'\d+', row)] for row in re.findall(r'\{([^}]+)\}', altars)],
             'weapons': [[int(item), [int(n) for n in re.findall(r'\d+', row)]]
                         for item, row in re.findall(r'\[(\d+)\] = \{([^}]+)\}', matrix)],
             'mana': int(re.search(r'manaCost = (\d+)', action)[1]),
             'soul': int(re.search(r'soulCost = (\d+)', action)[1])}
    if len(rules['weapons']) != 17 or len(rules['altars']) != 4:
        raise ValueError('ENCHANTING_ACTION_TABLES')
    return {'rules': rules, 'schema': 'OTERYN_NATIVE_ENCHANTING_ITEM_PROFILES/v1', 'source_revision': REVISION,
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
            raise SystemExit('ENCHANTING_ITEM_PROFILE_DRIFT')
    else:
        (ROOT / OUTPUT).write_bytes(data)
