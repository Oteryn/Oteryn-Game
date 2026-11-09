#!/usr/bin/env python3
"""Qualify exactly two pinned Crystal Soul War callback outfit literals.

This boss-only producer adds no casting flow or appearance grant. It retains
actual source/default/asset membership and preserves every unrelated record.
"""
from __future__ import annotations
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import subprocess
from build_source_world import fields

ROOT = Path(__file__).resolve().parents[3]
PIN = '201189f871d23597b82caf88fd611454a2ddefb8'
LUA = 'data-global/lib/quests/soul_war.lua'
HEADER = 'src/creatures/creatures_definitions.hpp'
PROTO = 'src/protobuf/appearances.proto'
SOURCES = {
 LUA: ('4c9d3ac502ecbd7ec32f4823a8cab13d7ad6f703', '89c0024914b0593b9b7d2bfb535a24025a552f54b158319697cdf56a66375e75'),
 HEADER: ('fc4ae0cf45a56ee7c363b0f452acfb14cbd3c63a', '8fbc70bf2416b6e76f17cce3dc1b075f8a5451f0f85b522b0a4ae34cf2cccf94'),
 PROTO: ('7069b5ead685929c0a79cb6e8b0664bbe8f19a3a', '3aaf2965b14d9c3c0252876f8d3e228f20188568a03f8840b122588dbb76b78f'),
 'data/items/appearances.dat': ('37ce9c8ee497b7edffe809f40d02c5d12ec4ccf0', '17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1'),
}
FIELDS = {'lookType', 'lookTypeEx', 'lookMount', 'lookHead', 'lookBody', 'lookLegs', 'lookFeet', 'lookAddons'}
FRAGMENT = ROOT / 'tools/content-schema/native-gameplay/srx3-soul-war-appearances.json'
BINDINGS = ROOT / 'content/presentations/bindings/spell-appearances.json'


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()


def extract_outfits(text):
    patterns = {
        286: r'waterElementalOutfit\s*=\s*\{([^{}]*)\}',
        242: r'local\s+outfitConfig\s*=\s*\{\s*outfit\s*=\s*\{([^{}]*)\}',
    }
    result = {}
    for look, pattern in patterns.items():
        matches = re.findall(pattern, text)
        if len(matches) != 1:
            raise ValueError('Soul War outfit literal is absent or duplicated')
        values = {}
        for part in matches[0].split(','):
            if not part.strip():
                continue
            match = re.fullmatch(r'\s*(\w+)\s*=\s*(\d+)\s*', part)
            if not match or match[1] not in FIELDS or match[1] in values:
                raise ValueError('Soul War outfit has dynamic/duplicate/unknown fields')
            values[match[1]] = int(match[2])
        if values.get('lookType') != look or any(v for k, v in values.items() if k != 'lookType'):
            raise ValueError('Soul War source outfit differs')
        result[look] = values
    return result


def build(source_root, appearances):
    raw = {}
    for path, (blob, sha) in SOURCES.items():
        value = appearances.read_bytes() if path == 'data/items/appearances.dat' else (source_root / path).read_bytes()
        if (hashlib.sha1(f'blob {len(value)}\0'.encode() + value).hexdigest() != blob
                or hashlib.sha256(value).hexdigest() != sha):
            raise ValueError('Pinned Soul War source differs: ' + path)
        raw[path] = value
    header = raw[HEADER].decode().split('struct Outfit_t {', 1)[1].split('};', 1)[0]
    if any(not re.search(rf'\b{field}\s*=\s*0\s*;', header) for field in FIELDS):
        raise ValueError('Unqualified Outfit_t default')
    if not re.search(rb'repeated\s+Appearance\s+outfit\s*=\s*2\s*;', raw[PROTO]):
        raise ValueError('Outfit protobuf field differs')
    outfits = {fields(member).get(1, [None])[0] for member in fields(raw['data/items/appearances.dat']).get(2, [])}
    literals = extract_outfits(raw[LUA].decode())
    if not set(literals) <= outfits:
        raise ValueError('Source outfit missing from pinned appearances.dat')
    records = []
    for look, values in sorted(literals.items()):
        record = {
            'outfit_key': f'oteryn:outfit.source.crystal.soul-war.look{look}',
            'outfit_revision': 'source-outfit-r1', 'look_type': look,
            'colours': [0, 0, 0, 0], 'addons': 0, 'mount_key': None, 'creature': None,
            'source': {'revision': PIN, 'path': LUA, 'git_blob': SOURCES[LUA][0],
                       'sha256': SOURCES[LUA][1], 'explicit_fields': values,
                       'defaulted_fields': sorted(FIELDS - values.keys()),
                       'default_source': {'revision': PIN, 'path': HEADER,
                                          'git_blob': SOURCES[HEADER][0], 'sha256': SOURCES[HEADER][1]}},
        }
        record['qualification_sha256'] = hashlib.sha256(canonical(record)).hexdigest()
        records.append(record)
    return {'schema': 'OTERYN_SOUL_WAR_CALLBACK_APPEARANCES/v1', 'source_revision': PIN,
            'source_lines': {'242': [1612, 1625], '286': [609, 618]},
            'asset_evidence': [{'path': path, 'git_blob': SOURCES[path][0], 'sha256': SOURCES[path][1]}
                               for path in [PROTO, 'data/items/appearances.dat']],
            'records': records}


def generate(source_root, appearances, check, restore_base_order=False):
    document = build(source_root, appearances)
    fragment = (json.dumps(document, ensure_ascii=False, indent=2) + '\n').encode()
    bindings = json.loads(BINDINGS.read_bytes())
    keys = {r['outfit_key'] for r in document['records']}
    retained = [r for r in bindings['records'] if r['outfit_key'] not in keys]
    if restore_base_order:
        baseline = json.loads(subprocess.check_output([
            'git', '-C', str(ROOT), 'show',
            'b2cfa798e3def61002d39a6da8b9de5df5aa2fd8:content/presentations/bindings/spell-appearances.json',
        ]))['records']
        existing = Counter(canonical(r) for r in retained)
        expected = Counter(canonical(r) for r in baseline)
        if any(existing[key] < count for key, count in expected.items()):
            raise ValueError('Refusing to reorder altered baseline appearance records')
        remaining = existing - expected
        extras = []
        for record in retained:
            key = canonical(record)
            if remaining[key]:
                extras.append(record)
                remaining[key] -= 1
        retained = baseline + extras
    bindings['records'] = retained + document['records']
    binding_raw = (json.dumps(bindings, ensure_ascii=False, indent=2) + '\n').encode()
    manifest_path = ROOT / 'content/spells.manifest.json'
    manifest = json.loads(manifest_path.read_bytes())
    manifest['spell_appearances']['sha256'] = hashlib.sha256(binding_raw).hexdigest()
    outputs = {FRAGMENT: fragment, BINDINGS: binding_raw, manifest_path: canonical(manifest) + b'\n'}
    for path, value in outputs.items():
        if check:
            if not path.exists() or path.read_bytes() != value:
                raise ValueError('Stale W11 callback appearance output: ' + str(path))
        else:
            path.write_bytes(value)
    print('W11_CALLBACK_APPEARANCES', len(document['records']), hashlib.sha256(fragment).hexdigest())


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root', type=Path, required=True)
    parser.add_argument('--appearances', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--restore-base-order', action='store_true', help='repair ordering from the explicit SRX-3 base without altering any baseline record')
    args = parser.parse_args()
    generate(args.source_root, args.appearances, args.check, args.restore_base_order)
