"""Native custody for the five items named by Crystal Arkulius's final exchange."""
import json

import encounter_loot_item_admissions as loot
import qengine_reward_item_profiles as rewards

# Crystal 201189f8 data-global/npc/arkulius.lua:162-165 selects these identities.
# The retained Item source supplies corroborated physical values only.
WEIGHTS = {954: 250, 8025: 15000, 8039: 2850, 8041: 2850, 8055: 12000}
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
    loot.apply_admissions(definitions, root, set(WEIGHTS), source_core=True)


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
        if not definition['materializable'] or definition['stack_class'] != record['admission']['stack_class']:
            raise ValueError('ELEMENTAL_SPHERES_CANONICAL_ADMISSION:' + key)
        record['semantics'] = rewards.accepted.accepted_first(
            definition['semantics'], record['semantics'])
    rewards.admit_records(root, generated, profile, manifest, order, canonical, records.values())
