"""Complete exact native Item custody from the qualified Crystal source cohorts."""

import hashlib
import json

from grizzly_shop_profiles import accepted_first

FACTS = 'content/items/exact-custody-facts.json'
SCHEMA = 'OTERYN_NATIVE_EXACT_ITEM_CUSTODY_FACTS/v1'
FACTS_SHA256 = 'b69c31cd133434098f77a8c1b8829e20c7a22465ed3f548c31e8230c1eb71f1e'


def admit(root, items, order):
    raw = (root / FACTS).read_bytes()
    if hashlib.sha256(raw).hexdigest() != FACTS_SHA256:
        raise ValueError('ITEM_CUSTODY_SOURCE_DIGEST')
    packet = json.loads(raw)
    if (packet['schema'] != SCHEMA
            or packet['source']['revision'] != '201189f871d23597b82caf88fd611454a2ddefb8'):
        raise ValueError('ITEM_CUSTODY_SOURCE_PACKET')
    exact = {
        row['target']['key']: row
        for row in json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())['bindings']
        if row['disposition'] == 'EXACT' and row['target']['family'] == 'Item'
    }
    canonical = {
        row['definition']['identity']['key']: row['definition']
        for path in (root / 'content/items/definitions').glob('items-*.json')
        for row in json.loads(path.read_bytes())['records']
    }
    active = {row['production_definition']['production_key'] for row in items['records']}
    reviewed = set()
    added = []
    for profile in packet['records']:
        identity = profile['production_definition']
        key = identity['production_key']
        reference = canonical[key]['identity']
        if (key in reviewed or profile['production_binding'] != exact.get(key)
                or reference != profile['production_binding']['target']
                or identity['family'] != 'Item'
                or identity['revision_ref'] != reference['revision']):
            raise ValueError(f'ITEM_CUSTODY_EXACT_IDENTITY:{key}')
        reviewed.add(key)
        if key in active:
            continue
        # Existing canonical semantics retain their authored values. Source appearance
        # custody and the closed set of exposed destinations stay coupled.
        profile['semantics'] = accepted_first(canonical[key].get('semantics', {}), profile['semantics'])
        physical = profile['semantics']['physical']['value']
        if 'CharacterInventory' in profile['admission']['legal_destinations'] and any(
                physical[field] != {'state': 'KNOWN', 'value': True}
                for field in ('movable', 'pickupable')):
            raise ValueError(f'ITEM_CUSTODY_PORTABLE_SOURCE_CONFLICT:{key}')
        added.append(profile)
    items['records'].extend(added)
    items['records'].sort(key=order)
    return added
