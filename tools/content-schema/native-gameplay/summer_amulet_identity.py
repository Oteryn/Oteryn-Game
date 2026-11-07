"""Five explicit Magnolia loot identities, qualified without fabricated XML policy.

DAT/factory facts admit physical custody only. Wiki equipment and 180-minute
claims remain retained evidence; runtime modifiers/expiry stay UNKNOWN.
"""
import argparse
import copy
from decimal import Decimal
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[3]
PIN = '201189f871d23597b82caf88fd611454a2ddefb8'
IDS = frozenset({53197, 53199, 53201, 53203, 53205})
PACKET = 'tools/content-schema/native-gameplay/candidate-bindings/srv7-summer-amulets.json'
WIKI_SHA = 'e832defe33249751e68c8d2ab4c3e39eb79a90abf530a9d57db86283bb88c870'
MAGNOLIA_SHA = 'aae1c55aad1328c02d4220f0e06dccfb6027510d02262d997000aead96f7abae'
SOURCES = {
 'data/items/items.xml': '8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509',
 'data/items/appearances.dat': '17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1',
 'src/items/items.cpp': 'f39d8fc6c1c81b8ec6f749e1f152dd945472ffd5a36b9cf3cd9a3a64b2f19b1d',
 'src/items/items.hpp': '06d52ee83250ba852fe804c6b006b0aac150ceb26893e9079fa8c93502d209ca',
 'src/items/item.cpp': 'e27e622bf4fd9a94d7fffb3a818fd0c5f911e6fbf4b92844320154ded326950b',
}
UNKNOWN_SECTIONS = ('equipment', 'protection', 'skill_modifiers', 'temporal', 'charges')


def candidate(donor, wiki_path, magnolia_path):
    sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
    import build_source_item_profiles as source
    from build_spell_item_identities import BINDINGS, binding_index
    raw = {path: (donor / path).read_bytes() for path in SOURCES}
    if any(hashlib.sha256(raw[path]).hexdigest() != pin for path, pin in SOURCES.items()):
        raise ValueError('SUMMER_AMULET_FACTORY_SOURCE_DRIFT')
    wiki_raw = wiki_path.read_bytes()
    magnolia = magnolia_path.read_bytes()
    if hashlib.sha256(wiki_raw).hexdigest() != WIKI_SHA or hashlib.sha256(magnolia).hexdigest() != MAGNOLIA_SHA:
        raise ValueError('SUMMER_AMULET_WIKI_OR_EXPLICIT_LOOT_DRIFT')
    wiki = json.loads(wiki_raw)
    if (wiki['repository'], wiki['revision'], wiki['path'], wiki['source_blob_git_sha1']) != ('tibia-sh/tibiawiki-data', '31748fd6abc1a18378ecade240c805a7aab6bf20', 'index.db', 'a8298e4951d2bc3427f89c407e08da67d41b8a1a'):
        raise ValueError('SUMMER_AMULET_WIKI_PIN')
    rows = {row['appearance_id']: row for row in wiki['rows']}
    if len(wiki['rows']) != 5 or set(rows) != IDS:
        raise ValueError('SUMMER_AMULET_EXACT_FIVE')
    xml = source.xml_index(raw['data/items/items.xml'])
    bindings = binding_index(BINDINGS.read_bytes())
    objects = source.engine_items.load_appearance_objects(raw['data/items/appearances.dat'])
    physics = {row['server_item_id']: row for row in source.policies(raw['data/items/appearances.dat'], raw['data/items/items.xml'], IDS)}
    checks = {
      'src/items/items.cpp': ['iType.id = static_cast<uint16_t>(object.id());', 'iType.movable = object.flags().unmove() == false;', 'iType.stackable = object.flags().cumulative();', 'iType.pickupable = object.flags().take();', 'iType.name = object.name();'],
      'src/items/item.cpp': ['if (it.id != 0)', 'newItem = std::make_shared<Item>(type, count);'],
    }
    if any(any(text not in raw[path].decode() for text in texts) for path, texts in checks.items()):
        raise ValueError('SUMMER_AMULET_FACTORY_CLOSURE')
    profiles = []
    appearance_facts = []
    for item in sorted(IDS):
        facts, obj, physical = rows[item], objects[item], physics[item]
        if item in xml or str(item) in bindings:
            raise ValueError('SUMMER_AMULET_NO_HISTORICAL_IDENTITY_OVERRIDE')
        if (obj['name'] != facts['actual_name'] or facts['client_id'] != item or facts['slot'] != 'neck'
                or facts['is_stackable'] != 0 or facts['is_pickupable'] != 1
                or not physical['movable'] or not physical['pickupable']
                or obj['flags'].get('flags.cumulative', False) or obj['flags'].get('clothes.slot') != 2
                or obj['flags'].get('market.trade_as_object_id') != item):
            raise ValueError('SUMMER_AMULET_DAT_WIKI_IDENTITY_JOIN')
        source_line = '{ id = ' + str(item) + ', chance = 300 }'
        if source_line not in magnolia.decode():
            raise ValueError('SUMMER_AMULET_EXPLICIT_PRODUCER')
        record = source.profile(item, None, obj, physical, bindings, None)
        record['semantics']['physical']['value']['weight'] = source.K(int(Decimal(str(facts['weight'])) * 100))
        record['semantics']['presentation']['value']['name'] = source.K(facts['actual_name'])
        record['semantics']['presentation']['value']['description'] = {'state': 'UNKNOWN'}
        for section in UNKNOWN_SECTIONS:
            record['semantics'][section] = {'state': 'UNKNOWN'}
        if record['attributes'].get('spell_augments') == []:
            del record['attributes']['spell_augments']
        binding = {'source_key': 'oteryn:source.crystalserver', 'source_revision': PIN,
          'identity_namespace': 'ots/item_server_id', 'external_id': str(item),
          'target': {'family': 'Item', 'key': f'oteryn:item.tibia.i{item}', 'revision': 'definition-r1'}, 'disposition': 'EXACT'}
        record['production_binding'] = binding
        record['production_definition'] = {'family': 'Item', 'production_key': binding['target']['key'], 'revision_ref': 'definition-r1'}
        profiles.append(record)
        appearance_facts.append({'id': item, 'name': obj['name'], 'flags': obj['flags']})
    return {'schema': 'OTERYN_SRV7_SUMMER_AMULET_IDENTITIES/v1', 'family': 'Item', 'source_revision': PIN,
      'bindings': [record['production_binding'] for record in profiles], 'profiles': profiles,
      'sources': [{'path': path, 'sha256': pin} for path, pin in SOURCES.items()],
      'loot_source': {'repository': 'zimbadev/crystalserver', 'revision': PIN, 'path': 'data-global/monster/summer_update_2026/bosses/the_moonsnow_magnolia.lua', 'sha256': MAGNOLIA_SHA, 'lines': [119, 123]},
      'wiki_source_sha256': WIKI_SHA, 'wiki_source': wiki, 'appearance_facts': appearance_facts,
      'scope': 'Exactly five literal Magnolia loot IDs; materializable nonstackable inventory/Ground custody only. No equipment, resistance, modifier, charge or timer activation; those fields remain UNKNOWN.'}


def profiles(packet):
    result = copy.deepcopy(packet['profiles'])
    for row in result:
        row['production_binding_qualification'] = copy.deepcopy(packet)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--donor', type=Path, required=True)
    parser.add_argument('--wiki', type=Path, required=True)
    parser.add_argument('--magnolia', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    raw = (json.dumps(candidate(args.donor, args.wiki, args.magnolia), indent=2, ensure_ascii=False) + '\n').encode()
    path = ROOT / PACKET
    sha = (hashlib.sha256(raw).hexdigest() + '\n').encode()
    if args.check:
        if path.read_bytes() != raw or path.with_suffix('.sha256').read_bytes() != sha:
            raise ValueError('SUMMER_AMULET_PACKET_DRIFT')
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(raw)
        path.with_suffix('.sha256').write_bytes(sha)

if __name__ == '__main__':
    main()
