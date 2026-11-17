"""Closed physical/temporal qualification for M07's held trial inputs and Dreamer entrance tools.

Crystal 201189f8 items.xml and the retained Canary XML have byte-identical
21389/21466/5467/3595 records (including their serialized XML tails). Native publication
must still select these records and regenerate the provider; this module does
not change admission or invent decay rules from item descriptions.
"""
import hashlib
import xml.etree.ElementTree as ET

import grizzly_shop_profiles as owner

# Crystal 201189f871d23597b82caf88fd611454a2ddefb8 data/items/items.xml,
# complete file SHA256 8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509.
SOURCE_RECORDS = {
    3595: '6c756e98369225f43afdda2044254cd3004d32c841ce83cebd78897e36704180',
    5467: '6e788b59b3e1c3b5b68d0decc30a210007db94c63d4346f5352036562d94d3a4',
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
            or (definition['materializable'] is not True and item_id != 5467)):
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
    stack_class = 'StackCapable' if item_id == 3595 else 'NonStackable'
    if (record['admission'] != {'materializable': True, 'stack_class': stack_class,
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
    # Fire Bug's accepted definition still has incomplete admission. Its exact
    # pinned source/appearance pair supplies native custody, as for QENGINE inputs.
    if definition['stack_class'] not in ((stack_class, 'Unknown') if item_id == 5467
                                          else (stack_class,)):
        raise ValueError('LION_TRIAL_ITEM_STACK')
    record['semantics'] = owner.accepted_first(definition['semantics'], record['semantics'])
    return source.bind_profiles(
        {'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': [record]},
        bindings, {item_id},
    )['records'][0]


def records(root, xml, definitions, *, top_order_keys=()):
    """Resolve the exact M07 native records from the retained pinned providers."""
    import qengine_reward_item_profiles as shared

    top_order_keys = set(top_order_keys)
    if not top_order_keys <= {f'oteryn:item.tibia.i{item}' for item in SOURCE_RECORDS}:
        raise ValueError('LION_TRIAL_ITEM_TOP_ORDER_COHORT')
    source = owner.source
    raw = (root / shared.CLIENT).read_bytes()
    if hashlib.sha256(raw).hexdigest() != shared.CLIENT_SHA:
        raise ValueError('LION_TRIAL_CLIENT_SOURCE')
    appearances = source.engine_items.load_appearance_objects(raw)
    physical = {row['server_item_id']: row for row in source.policies(
        raw, xml, set(SOURCE_RECORDS))}
    nodes = source.xml_index(xml)
    bindings = source.binding_index(
        (root / 'imports/crystalserver/bindings/items.json').read_bytes())
    qualified = {f'oteryn:item.tibia.i{item_id}': qualify(
        item_id, nodes[item_id], appearances[item_id], physical[item_id], bindings,
        definitions[f'oteryn:item.tibia.i{item_id}']) for item_id in SOURCE_RECORDS}
    # House's retained appearance pin and this verified pin agree on these
    # four ordering facts. Reproduce only its optional pair, from source flags,
    # using the same clip/top/bottom precedence; never copy native attributes.
    for item_id in SOURCE_RECORDS:
        key = f'oteryn:item.tibia.i{item_id}'
        if key in top_order_keys:
            flags = appearances[item_id]['flags']
            order = next((value for flag, value in (('clip', 1), ('top', 3), ('bottom', 2))
                          if flags.get('flags.' + flag, False)), 0)
            qualified[key]['attributes'].update(always_on_top=order != 0,
                                                 always_on_top_order=order)
    return qualified
