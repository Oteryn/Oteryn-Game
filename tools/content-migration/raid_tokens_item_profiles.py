"""Only carrying admission for Jorge's exact input and37replica output IDs."""
import copy
import hashlib
import sys

from fathers_burden_item_profiles import CLIENT
import raid_tokens_source as source

KEYS = {f'oteryn:item.tibia.i{item}' for item, _ in source.ITEMS} | {'oteryn:item.tibia.i19083'}
# Game's already accepted weights remain truth; the reference XML differs at
# these two exact identities. Neither this overlay nor generation rewrites them.
ACCEPTED_WEIGHT_DIFFERENCES = {23677: (200, 2700), 34210: (6659, 6660)}


def definitions(root, definitions, source_items):
    source.document(root)
    raw = (root / CLIENT).read_bytes()
    if hashlib.sha256(raw).hexdigest() != CLIENT.rsplit('-', 1)[1][:-4]:
        raise ValueError('RAID_TOKENS_CLIENT_PIN')
    sys.path.insert(0, str(root / 'tools/content-schema/item-authoring'))
    from engine_items import load_appearance_objects
    appearances = load_appearance_objects(raw)
    result = dict(definitions)
    for key in sorted(KEYS):
        item = int(key.rsplit('i', 1)[1])
        stackable = item == 19083
        definition = copy.deepcopy(definitions[key])
        flags = appearances.get(item, {}).get('flags', {})
        matches = [row for row in source_items
                   if int(row.get('id', row.get('fromid', '0'))) <= item
                   <= int(row.get('id', row.get('toid', '0')))]
        if (len(matches) != 1 or flags.get('flags.take') is not True
                or flags.get('flags.cumulative', False) != stackable
                or flags.get('flags.container', False) or flags.get('flags.unmove', False)):
            raise ValueError(f'RAID_TOKENS_CLIENT_FACTS:{item}')
        semantics = definition['semantics']
        weights = [row.get('value') for row in matches[0].findall("attribute[@key='weight']")]
        weight = int(weights[0]) if len(weights) == 1 else 0
        if item in ACCEPTED_WEIGHT_DIFFERENCES:
            donor_weight, accepted_weight = ACCEPTED_WEIGHT_DIFFERENCES[item]
            if weight != donor_weight:
                raise ValueError(f'RAID_TOKENS_REFERENCE_WEIGHT:{item}')
            weight = accepted_weight
        physical = {'weight': {'state': 'KNOWN', 'value': weight},
                    'movable': {'state': 'KNOWN', 'value': True},
                    'pickupable': {'state': 'KNOWN', 'value': True}}
        if (len(weights) > 1
                or semantics['presentation']['value']['name'] != {'state': 'KNOWN', 'value': matches[0].get('name')}
                or semantics['physical']['state'] != 'KNOWN'
                or any(semantics['physical']['value'].get(field, {}).get('state') != 'UNKNOWN'
                       and semantics['physical']['value'].get(field) != expected
                       for field, expected in physical.items())
                or semantics['container']['state'] not in ('UNKNOWN', 'NOT_APPLICABLE')):
            raise ValueError(f'RAID_TOKENS_CANONICAL_FACTS:{item}')
        target_class = 'StackCapable' if stackable else 'NonStackable'
        stack = {'state': 'KNOWN', 'value': {
            'stackable': {'state': 'KNOWN', 'value': stackable},
            'stack_max': {'state': 'KNOWN', 'value': 100} if stackable else {'state': 'NOT_APPLICABLE'}}}
        if (definition['stack_class'] not in ('Unknown', target_class)
                or semantics['stack']['state'] != 'UNKNOWN' and semantics['stack'] != stack):
            raise ValueError(f'RAID_TOKENS_STACK_CONFLICT:{item}')
        definition.update(materializable=True, stack_class=target_class)
        semantics['physical']['value'] = physical
        semantics['stack'] = stack
        semantics['container'] = {'state': 'NOT_APPLICABLE'}
        result[key] = definition
    return result
