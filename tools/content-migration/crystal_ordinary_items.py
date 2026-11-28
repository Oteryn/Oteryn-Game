"""Admit the ordinary Crystal food, rope and door policies through the native Item provider."""
import json

from authored_static_item_constructors import definitions

FACTS = 'tools/content-schema/native-gameplay/crystal-ordinary-item-facts.json'
DOOR_BINDINGS = 'imports/crystalserver/bindings/ordinary-doors.json'
NOT_APPLICABLE = {'state': 'NOT_APPLICABLE'}
UNKNOWN = {'state': 'UNKNOWN'}


def known(value):
    return {'state': 'KNOWN', 'value': value}


def door_bindings(root):
    return json.loads((root / DOOR_BINDINGS).read_bytes())['bindings']


def extend_references(records, root):
    existing = {record['identity']['key']: record for record in records}
    for binding in door_bindings(root):
        identity = binding['target']
        key = identity['key']
        reference = {
            'identity': identity, 'kind': 'Item', 'client_projection': 'ClientSafe',
            'materializable': False, 'stack_class': 'Unknown',
        }
        if key not in existing:
            records.append(reference)
        elif existing[key] != reference:
            raise ValueError(f'CRYSTAL_ORDINARY_DOOR_REFERENCE_CHANGED:{key}')


def profile(definition, binding, facts):
    stackable = facts['stackable']
    pickupable = facts['pickupable']
    movable = facts['movable']
    return {
        'authoring': {'item': definition['identity']},
        'production_definition': {
            'family': 'Item', 'production_key': definition['identity']['key'],
            'revision_ref': definition['identity']['revision'],
        },
        'production_binding': binding,
        'production_binding_qualification': None,
        'admission': {
            'materializable': True,
            'legal_destinations': ['CharacterInventory', 'Ground'] if pickupable and movable else ['Ground'],
            'stack_class': 'StackCapable' if stackable else 'NonStackable',
        },
        'attributes': {
            'blocks_movement': facts['blocks_movement'],
            'blocks_projectile': facts['blocks_projectile'],
            'immovable_block_solid': facts['blocks_movement'] and not movable,
            'has_height': facts['has_height'],
            'always_on_top': facts['top_order'] != 0,
            'always_on_top_order': facts['top_order'],
        },
        'semantics': {
            'presentation': definition.get('semantics', {}).get('presentation', UNKNOWN),
            'classification': known({'item_type': NOT_APPLICABLE, 'capabilities': UNKNOWN}),
            'physical': known({
                'weight': known(facts['weight']), 'movable': known(movable), 'pickupable': known(pickupable),
            }),
            'stack': known({
                'stackable': known(stackable), 'stack_max': known(100) if stackable else NOT_APPLICABLE,
            }),
            'container': NOT_APPLICABLE,
            'temporal': NOT_APPLICABLE,
        },
    }


def source_records(root):
    source = json.loads((root / FACTS).read_bytes())
    ids = [row['id'] for row in source['records']]
    if (source['schema'] != 'OTERYN_CRYSTAL_ORDINARY_ITEM_FACTS/v1'
            or source['source_revision'] != '201189f871d23597b82caf88fd611454a2ddefb8'
            or ids != sorted(set(ids))):
        raise ValueError('CRYSTAL_ORDINARY_ITEM_SOURCE')
    return source['records']


def apply_food_policies(records, facts):
    for fact in facts:
        key = f"oteryn:item.tibia.i{fact['id']}"
        if 'food_seconds' in fact and key in records:
            records[key]['semantics']['consumption'] = known({
                'kind': 'FOOD', 'value': {'regeneration_seconds': fact['food_seconds']},
            })


def apply(root, document, order):
    canonical = definitions(root)
    bindings = json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())['bindings']
    bindings = {binding['target']['key']: binding for binding in [*bindings, *door_bindings(root)]}
    records = {row['production_definition']['production_key']: row for row in document['records']}
    captured = source_records(root)
    for facts in captured:
        key = f"oteryn:item.tibia.i{facts['id']}"
        definition = canonical[key]
        binding = bindings[key]
        if binding['target'] != definition['identity'] or binding['disposition'] != 'EXACT':
            raise ValueError(f'CRYSTAL_ORDINARY_ITEM_BINDING:{key}')
        row = records.get(key)
        if row is None:
            row = profile(definition, binding, facts)
            records[key] = row
        if facts.get('door'):
            row['semantics']['classification'] = known({
                'item_type': known('DOOR'), 'capabilities': UNKNOWN,
            })
            for flag in ('blocks_movement', 'blocks_projectile'):
                row['attributes'][flag] = facts[flag]
            row['attributes']['immovable_block_solid'] = facts['blocks_movement']
        if decay := facts.get('decay'):
            seconds, target_id = decay
            target = canonical[f'oteryn:item.tibia.i{target_id}']['identity']
            row['semantics']['temporal'] = known({
                'consumption_mode': known('DURABLE_ABSOLUTE_DEADLINE'),
                'duration': known(seconds * 1000), 'stop_duration': known(False),
                'decay_target': known({'key': target['key'], 'revision': target['revision']}),
            })
    apply_food_policies(records, captured)
    document['records'] = sorted(records.values(), key=order)
