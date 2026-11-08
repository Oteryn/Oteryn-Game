"""Admit existing seasonal reward/corpse identities through normal Item policies.

Crystal santa_claus.lua:111-119 creates container6510 and includes snowball2992.
No identity, reward probability or seasonal entitlement is invented here.
"""
import argparse
import copy
import hashlib
import json
import sys
from pathlib import Path
from compact_record_json import encode

from audit_house_decoration_sources import SOURCE_HASHES

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
import build_source_item_profiles as source
from build_spell_item_identities import BINDINGS, bind_profiles, binding_index

PATH = 'tools/content-schema/native-gameplay/seasonal-item-profiles.json'
PIN = '201189f871d23597b82caf88fd611454a2ddefb8'
CANARY = '47dfd51f45280a59a1d3e50ba7edd573d7234446'
# Crystal Cake config.lua:500-516 supplies all fixed map and reward identities.
CAKE_REWARDS = {14756, 20347, 20350, 28484, 28485, 28486}
CAKE_MAP = {12085, 12086, 12147, 12148, 12152, 12153, 12154, 12155,
            12090, 12091, 12092, 12093, 12094, 12095}
# Crystal201189f data-global/npc/ruprecht.lua:54-77 names22 offers;
# :109-126 burns6496 for6526, or6526 tokens for one exact listed output.
RUPRECHT_ITEMS = {6500, 20310, 174, 7527, 6531, 10346, 20315, 6511,
                  10339, 20311, 20308, 20309, 20313, 10338, 21952, 6496,
                  22879, 29947, 24394, 35692, 27591, 32746, 6526}
RUPRECHT = 'data-global/npc/ruprecht.lua'
RUPRECHT_BLOB = '6718df2ef51872c95a8fe2036add77d2623141bb'
IDS = {6510, 2992, 12141, 2886, 2887} | CAKE_REWARDS | CAKE_MAP | RUPRECHT_ITEMS
BRUTUS = 'data-global/npc/brutus_the_baker.lua'
CAKE_CONFIG = 'data-global/scripts/lib/a_piece_of_cake_config.lua'
CAKE_BITE = 'data-global/scripts/quests/a_piece_of_cake/actions_cake_bite.lua'
CAKE_POLICY_SOURCES = {
    BRUTUS: 'a105a573c305f901c95af4b9c7e254c8b97ce9be',
    CAKE_CONFIG: 'b8291626b473b91916cdac2fe5727323c1367c6d',
    CAKE_BITE: 'a662d386a90d295978c73ddff79a8d46c987d0d4',
}
SANTA = 'data-global/npc/santa_claus.lua'
CAKE = 'data-global/monster/quests/a_piece_of_cake/cake_golem.lua'


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
    cake_raw = (crystal / CAKE).read_bytes()
    if (hashlib.sha1(b'blob ' + str(len(cake_raw)).encode() + b'\0' + cake_raw).hexdigest()
            != 'd95b96f4ed3c5e47d9219a3112c6065ea5de1e2d'
            or 'monster.corpse = 12141' not in cake_raw.decode()
            or 'monster.race = "venom"' not in cake_raw.decode()):
        raise ValueError('SEASONAL_CAKE_ITEM_SOURCE_DRIFT')
    cake_policy_raw = []
    for path, expected in CAKE_POLICY_SOURCES.items():
        raw = (crystal / path).read_bytes()
        if hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest() != expected:
            raise ValueError('SEASONAL_CAKE_POLICY_SOURCE_DRIFT:' + path)
        cake_policy_raw.append(('crystal', PIN, path, raw))
    ruprecht_raw = (crystal / RUPRECHT).read_bytes()
    if (hashlib.sha1(b'blob ' + str(len(ruprecht_raw)).encode() + b'\0' + ruprecht_raw).hexdigest()
            != RUPRECHT_BLOB):
        raise ValueError('SEASONAL_RUPRECHT_SOURCE_DRIFT')
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
                            ('crystal', PIN, SANTA, santa_raw),
                            ('crystal', PIN, CAKE, cake_raw),
                            ('crystal', PIN, RUPRECHT, ruprecht_raw), *cake_policy_raw)],
            'producer': SANTA + ':111-119',
            'cake_producer': CAKE + ':25-26',
            'cake_reward_producer': BRUTUS + ':99-158',
            'cake_map_producer': CAKE_CONFIG + ':508-514',
            'ruprecht_producer': RUPRECHT + ':54-77,109-126', 'records': bound['records']}


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
