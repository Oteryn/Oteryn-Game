"""Admit Santa's existing canonical present and snowball through normal Item policies.

Crystal santa_claus.lua:111-119 creates container6510 and includes snowball2992.
No identity, reward probability or seasonal entitlement is invented here.
"""
import argparse
import copy
import hashlib
import json
import sys
from pathlib import Path

from audit_house_decoration_sources import SOURCE_HASHES

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
import build_source_item_profiles as source
from build_spell_item_identities import BINDINGS, bind_profiles, binding_index

PATH = 'tools/content-schema/native-gameplay/seasonal-item-profiles.json'
PIN = '201189f871d23597b82caf88fd611454a2ddefb8'
CANARY = '47dfd51f45280a59a1d3e50ba7edd573d7234446'
IDS = {6510, 2992}
SANTA = 'data-global/npc/santa_claus.lua'


def encode(document):
    return (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()


def capture(crystal, canary):
    xml_raw = (crystal / 'data/items/items.xml').read_bytes()
    appearance_raw = (canary / 'data/items/appearances.dat').read_bytes()
    for name, path, raw in [('crystal', 'data/items/items.xml', xml_raw),
                            ('canary', 'data/items/appearances.dat', appearance_raw)]:
        if hashlib.sha256(raw).hexdigest() != SOURCE_HASHES[(name, path)]:
            raise ValueError('SEASONAL_ITEM_SOURCE_DRIFT:' + name + '/' + path)
    santa_raw = (crystal / SANTA).read_bytes()
    if 'Player(creature):addItem(6510)' not in santa_raw.decode() or 'reward[i] == 2992' not in santa_raw.decode():
        raise ValueError('SEASONAL_GIFT_SOURCE_DRIFT')
    xml = source.xml_index(xml_raw)
    appearances = source.engine_items.load_appearance_objects(appearance_raw)
    physical = {row['server_item_id']: row for row in source.policies(appearance_raw, xml_raw, IDS)}
    bindings = binding_index(BINDINGS.read_bytes())
    records = [source.profile(item, xml[item], appearances[item], physical[item], bindings, None)
               for item in sorted(IDS)]
    bound = bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records}, bindings, IDS,
                          independent_crystal_source=(PIN, xml_raw))
    return {'schema': 'OTERYN_SEASONAL_ITEM_PROFILES/v1', 'crystal_commit': PIN,
            'sources': [{'reference': name, 'revision': revision, 'path': path,
                         'sha256': hashlib.sha256(raw).hexdigest()}
                        for name, revision, path, raw in (
                            ('crystal', PIN, 'data/items/items.xml', xml_raw),
                            ('canary', CANARY, 'data/items/appearances.dat', appearance_raw),
                            ('crystal', PIN, SANTA, santa_raw))],
            'producer': SANTA + ':111-119', 'records': bound['records']}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    if (overlay['schema'] != 'OTERYN_SEASONAL_ITEM_PROFILES/v1'
            or {row['production_definition']['production_key'] for row in overlay['records']}
            != {f'oteryn:item.tibia.i{item}' for item in IDS}):
        raise ValueError('SEASONAL_ITEM_PROFILE_SET')
    document = json.loads(generated[profile_path])
    existing = {row['production_definition']['production_key']: row for row in document['records']}
    for row in overlay['records']:
        key = row['production_definition']['production_key']
        if key in existing:
            if existing[key]['production_definition'] != row['production_definition']:
                raise ValueError('SEASONAL_ITEM_EXISTING_IDENTITY_CONFLICT')
        else:
            document['records'].append(copy.deepcopy(row))
    document['records'].sort(key=order)
    raw = encode(document)
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--canary', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    raw = encode(capture(args.crystal, args.canary))
    destination = ROOT / PATH
    if args.check:
        if destination.read_bytes() != raw:
            raise SystemExit('SEASONAL_ITEM_PROFILE_DRIFT')
    else:
        destination.write_bytes(raw)
