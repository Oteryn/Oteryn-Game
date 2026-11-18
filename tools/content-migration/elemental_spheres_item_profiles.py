"""Native custody for Crystal chamber admission and Arkulius's final exchange."""
import json

import encounter_loot_item_admissions as loot
import qengine_reward_item_profiles as rewards

ICE_CRYSTAL = 942

# Crystal 201189f8 actions_lever.lua:1-25 and
# data-global/npc/arkulius.lua:162-165 select these identities.
# The retained Item source supplies corroborated physical values only.
WEIGHTS = {
    ICE_CRYSTAL: 250,
    946: 250,
    947: 250,
    948: 250,
    954: 250,
    8025: 15000,
    8039: 2850,
    8041: 2850,
    8055: 12000,
}
KEYS = {f'oteryn:item.tibia.i{item}': item for item in WEIGHTS}


def qualified(root):
    records = loot.facts(root, set(WEIGHTS))
    if records.keys() != KEYS.keys():
        raise ValueError('ELEMENTAL_SPHERES_ITEM_COHORT')
    for key, record in records.items():
        weight = record['semantics']['physical']['value']['weight']
        if weight != {'state': 'KNOWN', 'value': WEIGHTS[KEYS[key]]}:
            raise ValueError('ELEMENTAL_SPHERES_ITEM_WEIGHT:' + key)
    return records


def apply_admissions(definitions, root):
    qualified(root)
    # The Flawless Ice Crystal has an existing canonical admission.
    loot.apply_admissions(definitions, root, set(WEIGHTS) - {ICE_CRYSTAL}, source_core=True)


def admit(root, generated, profile, manifest, order, canonical):
    records = qualified(root)
    definitions = {}
    for path in (root / 'content/items/definitions').glob('items-*.json'):
        for row in json.loads(path.read_bytes())['records']:
            definition = row['definition']
            key = definition['identity']['key']
            if key in KEYS:
                if key in definitions:
                    raise ValueError('ELEMENTAL_SPHERES_DUPLICATE_ITEM:' + key)
                definitions[key] = definition
    for key, record in records.items():
        definition = definitions[key]
        if not definition['materializable']:
            raise ValueError('ELEMENTAL_SPHERES_CANONICAL_ADMISSION:' + key)
        canonical_stack = definition['stack_class']
        source_stack = record['admission']['stack_class']
        record['semantics'] = rewards.accepted.accepted_first(
            definition['semantics'], record['semantics'])
        if KEYS[key] == ICE_CRYSTAL and (canonical_stack, source_stack) == ('NonStackable', 'StackCapable'):
            # Project native custody with the accepted canonical stack class.
            record['admission']['stack_class'] = canonical_stack
            record['semantics']['stack'] = definition['semantics']['stack']
        elif canonical_stack != source_stack:
            raise ValueError('ELEMENTAL_SPHERES_CANONICAL_ADMISSION:' + key)
    rewards.admit_records(root, generated, profile, manifest, order, canonical, records.values())
