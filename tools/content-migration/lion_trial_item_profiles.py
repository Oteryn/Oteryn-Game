"""Closed physical/temporal qualification for M07's two held trial inputs.

Crystal 201189f8 items.xml and the retained Canary XML have byte-identical
21389/21466 records (including their serialized XML tails). Native publication
must still select these records and regenerate the provider; this module does
not change admission or invent decay rules from item descriptions.
"""
import hashlib
import xml.etree.ElementTree as ET

import grizzly_shop_profiles as owner

# Crystal 201189f871d23597b82caf88fd611454a2ddefb8 data/items/items.xml,
# complete file SHA256 8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509.
SOURCE_RECORDS = {
    21389: '593dda554a3c255633ad5c3278098f566447472a45c760561bf70ff5171408b6',
    21466: 'f5c7ba16e4b6941ff450ade217b079c4e42da9985e771fa10a86c3d1b978ee01',
}


def qualify(item_id, node, appearance, physical, bindings, definition):
    """Produce one exact native record, keeping accepted canonical facts first."""
    source = owner.source
    key = f'oteryn:item.tibia.i{item_id}'
    if (item_id not in SOURCE_RECORDS or node.get('id') != str(item_id)
            or hashlib.sha256(ET.tostring(node, encoding='utf-8')).hexdigest()
            != SOURCE_RECORDS[item_id]
            or definition['identity'] != {'family': 'Item', 'key': key,
                                          'revision': 'definition-r1'}
            or definition['materializable'] is not True):
        raise ValueError('LION_TRIAL_ITEM_SOURCE')
    binding = bindings.get(str(item_id))
    if (binding is None or binding['target'] != definition['identity']
            or binding['disposition'] != 'EXACT'):
        raise ValueError('LION_TRIAL_ITEM_BINDING')
    flags = appearance['flags']
    if flags.get('flags.liquidcontainer') or flags.get('flags.liquidpool'):
        raise ValueError('LION_TRIAL_ITEM_FLUID')
    record = source.profile(item_id, node, appearance, physical, bindings, None)
    expected_temporal = (source.K({
        'consumption_mode': source.K('DURABLE_ABSOLUTE_DEADLINE'),
        'duration': source.K(3_600_000), 'stop_duration': source.K(False),
        'decay_target': source.NA,
    }) if item_id == 21389 else source.NA)
    if (record['admission'] != {'materializable': True, 'stack_class': 'NonStackable',
                                'legal_destinations': ['CharacterInventory', 'Ground']}
            or record['semantics']['charges'] != source.NA
            or record['semantics']['container'] != source.NA
            or record['semantics']['temporal'] != expected_temporal
            or any(record['attributes'][field] is not False for field in
                   ('blocks_movement', 'blocks_projectile', 'immovable_block_solid'))):
        raise ValueError('LION_TRIAL_ITEM_CUSTODY')
    for group in ('physical', 'stack', 'container', 'temporal'):
        if owner.known_conflict(definition['semantics'][group], record['semantics'][group]):
            raise ValueError('LION_TRIAL_ITEM_CANONICAL_CONFLICT:' + group)
    if definition['stack_class'] != 'NonStackable':
        raise ValueError('LION_TRIAL_ITEM_STACK')
    record['semantics'] = owner.accepted_first(definition['semantics'], record['semantics'])
    return source.bind_profiles(
        {'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': [record]},
        bindings, {item_id},
    )['records'][0]
