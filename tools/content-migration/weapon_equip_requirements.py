"""Complete unknown native weapon equip level and vocations from pinned source facts.

The server projects an equip pattern only when its level and vocations are known
(`projected_equipment_claims`), but the Wiki lowering keeps an absent or zero requirement
UNKNOWN. Crystal gives these weapons no level and no vocations (level 0, any vocation); where
Crystal gives a level or the Knight vocation, that value is used, and the Wiki agrees with every
row. Only an UNKNOWN field is filled; a known field that differs refuses.
"""
from __future__ import annotations
import argparse
import copy
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PROFILE = 'content/items/definitions/spell-native-profiles.json'
MANIFEST = 'content/spells.manifest.json'
FACTS = 'tools/content-schema/native-gameplay/weapon-equip-requirement-facts.json'
SCHEMA = 'OTERYN_NATIVE_WEAPON_EQUIP_REQUIREMENT_FACTS/v1'


def complete(profiles, facts, packet=False):
    if facts.get('schema') != SCHEMA:
        raise ValueError('WEAPON_EQUIP_FACTS_SCHEMA')
    result = copy.deepcopy(profiles)
    by_key = {r['production_definition']['production_key']: r for r in result['records']}
    for fact in facts['records']:
        record = by_key.get(fact['key'])
        if record is None and packet:
            continue
        if record is None:
            raise ValueError('WEAPON_EQUIP_UNKNOWN_ITEM:' + fact['key'])
        equipment = record['semantics']['equipment']
        if packet and (equipment['state'] != 'KNOWN' or equipment['value']['patterns']['state'] != 'KNOWN'):
            continue
        if equipment['state'] != 'KNOWN' or equipment['value']['patterns']['state'] != 'KNOWN':
            raise ValueError('WEAPON_EQUIP_PATTERN_STATE:' + fact['key'])
        patterns = equipment['value']['patterns']['value']
        if packet and len(patterns) != 1:
            continue
        if len(patterns) != 1 or patterns[0]['primary_slot'] != {'state': 'KNOWN', 'value': 'WEAPON'}:
            raise ValueError('WEAPON_EQUIP_PATTERN_SHAPE:' + fact['key'])
        for field, value in (('level', fact['level']), ('vocations', fact['vocations'])):
            wanted = {'state': 'KNOWN', 'value': value}
            current = patterns[0][field]
            if current['state'] == 'UNKNOWN':
                patterns[0][field] = wanted
            elif current != wanted and not packet:
                raise ValueError(f'WEAPON_EQUIP_KNOWN_CONFLICT:{fact["key"]}:{field}')
    return result


def complete_semantics(root, key, semantics):
    """The canonical definition's semantics as the native owner completes them (MP01)."""
    facts = json.loads((root / FACTS).read_bytes())
    facts['records'] = [f for f in facts['records'] if f['key'] == key]
    if not facts['records']:
        return semantics
    row = {'production_definition': {'production_key': key}, 'semantics': semantics}
    return complete({'records': [row]}, facts, packet=True)['records'][0]['semantics']


# Source packets that carry their own expected copy of a native profile for some of these Items.
# Their rows must show the same completed requirement, or the owners' conflict guards refuse.
PACKETS = (
    'tools/content-schema/native-gameplay/boss-reward-yield-profiles.json',
    'tools/content-schema/native-gameplay/enchanting-item-profiles.json',
    'tools/content-schema/native-gameplay/event-item-profiles.json',
    'tools/content-schema/native-gameplay/hireling-item-profiles.json',
    'tools/content-schema/native-gameplay/qready1-reward-item-profiles.json',
    'tools/content-schema/native-gameplay/seasonal-item-profiles.json',
    'tools/content-schema/native-gameplay/task-board-item-profiles.json',
)


def _compact(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':'))


def _line_per_record(document):
    def render(value):
        if isinstance(value, list) and value:
            return '[\n' + ',\n'.join(_compact(item) for item in value) + '\n]'
        return _compact(value)
    return '{\n' + ',\n'.join(json.dumps(k) + ':' + render(v) for k, v in document.items()) + '\n}\n'


FORMATS = (
    lambda d: json.dumps(d, ensure_ascii=False, separators=(',', ':')) + '\n',
    lambda d: json.dumps(d, ensure_ascii=False, indent=2) + '\n',
    _line_per_record,
)


def sync_packet(raw, facts):
    """Complete the packet's rows for these Items, keeping the file's own serialization."""
    document = json.loads(raw)
    layout = next((f for f in FORMATS if f(document).encode() == raw), None)
    if layout is None:
        raise ValueError('WEAPON_EQUIP_PACKET_FORMAT')
    keys = {f['key'] for f in facts['records']}

    def key_of(row):
        return row.get('production_definition', {}).get('production_key')
    rows = [r for r in document['records'] if key_of(r) in keys]
    done = complete({'records': rows}, facts, packet=True)
    by_key = {key_of(r): r for r in done['records']}
    document['records'] = [by_key.get(key_of(r), r) for r in document['records']]
    return layout(document).encode()


def apply_native(root, generated, canonical):
    facts = json.loads((root / FACTS).read_bytes())
    raw = canonical(complete(json.loads(generated[PROFILE]), facts))
    generated[PROFILE] = raw
    manifest = json.loads(generated[MANIFEST])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[MANIFEST] = canonical(manifest)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--sync-packets', action='store_true')
    args = parser.parse_args()
    if args.sync_packets:
        known = json.loads((ROOT / FACTS).read_bytes())
        for name in PACKETS:
            (ROOT / name).write_bytes(sync_packet((ROOT / name).read_bytes(), known))
        raise SystemExit(0)
    facts = json.loads((ROOT / FACTS).read_bytes())
    done = complete(json.loads((ROOT / PROFILE).read_bytes()), facts)
    print(f"weapon equip requirements: {len(facts['records'])} facts, "
          f"{'applied' if done == json.loads((ROOT / PROFILE).read_bytes()) else 'pending'}")
