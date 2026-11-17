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


def complete(profiles, facts):
    if facts.get('schema') != SCHEMA:
        raise ValueError('WEAPON_EQUIP_FACTS_SCHEMA')
    result = copy.deepcopy(profiles)
    by_key = {r['production_definition']['production_key']: r for r in result['records']}
    for fact in facts['records']:
        record = by_key.get(fact['key'])
        if record is None:
            raise ValueError('WEAPON_EQUIP_UNKNOWN_ITEM:' + fact['key'])
        equipment = record['semantics']['equipment']
        if equipment['state'] != 'KNOWN' or equipment['value']['patterns']['state'] != 'KNOWN':
            raise ValueError('WEAPON_EQUIP_PATTERN_STATE:' + fact['key'])
        patterns = equipment['value']['patterns']['value']
        if len(patterns) != 1 or patterns[0]['primary_slot'] != {'state': 'KNOWN', 'value': 'WEAPON'}:
            raise ValueError('WEAPON_EQUIP_PATTERN_SHAPE:' + fact['key'])
        for field, value in (('level', fact['level']), ('vocations', fact['vocations'])):
            wanted = {'state': 'KNOWN', 'value': value}
            current = patterns[0][field]
            if current['state'] == 'UNKNOWN':
                patterns[0][field] = wanted
            elif current != wanted:
                raise ValueError(f'WEAPON_EQUIP_KNOWN_CONFLICT:{fact["key"]}:{field}')
    return result


def apply_native(root, generated, canonical):
    facts = json.loads((root / FACTS).read_bytes())
    raw = canonical(complete(json.loads(generated[PROFILE]), facts))
    generated[PROFILE] = raw
    manifest = json.loads(generated[MANIFEST])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[MANIFEST] = canonical(manifest)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.parse_args()
    facts = json.loads((ROOT / FACTS).read_bytes())
    done = complete(json.loads((ROOT / PROFILE).read_bytes()), facts)
    print(f"weapon equip requirements: {len(facts['records'])} facts, "
          f"{'applied' if done == json.loads((ROOT / PROFILE).read_bytes()) else 'pending'}")
