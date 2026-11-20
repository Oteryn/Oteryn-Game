"""Source-backed custody for the boss and quest yields pruned by ordinary loot."""

import json

import authored_quest_item_custody as custody
import encounter_loot_item_admissions as source

ITEMS = {f'oteryn:item.tibia.i{i}' for i in (
    2863, 2950, 3019, 3022, 3023, 3024, 3304, 3331, 3334, 3335, 3339, 3432,
    3573, 5883, 5930, 7396, 8897, 8898, 9615, 10244, 10290, 12482, 12483, 12509,
)}


def admit(root, items, order):
    from build_full_spell_artifact import native_loot_yield_facts

    definitions = {}
    for path in sorted((root / 'content/items/definitions').glob('items-*.json')):
        for row in json.loads(path.read_bytes())['records']:
            definition = row['definition']
            key = definition['identity']['key']
            if key in ITEMS:
                definitions[key] = definition
    if definitions.keys() != ITEMS:
        raise ValueError('protected loot is missing a canonical Item definition')

    existing = {r['production_definition']['production_key']: r for r in items['records']}
    new = [definition for key, definition in sorted(definitions.items()) if key not in existing]
    ids = {int(definition['identity']['key'].rsplit('.i', 1)[1]) for definition in new}
    observed = source.facts(root, ids, allow_containers=True) if ids else {}
    xml = (root / 'imports/canary/items-xml/items.xml').read_bytes()
    for profile in custody.qualify_many(root, new, xml, terminal_expiry=True):
        key = profile['production_definition']['production_key']
        # The quest importer preserves canonical Unknowns outside its physical
        # scope. Loot also needs the pinned classification and fresh-instance
        # charge/expiry facts, without overwriting known Quest or combat facts.
        if observed[key]['production_binding'] != profile['production_binding']:
            raise ValueError(f'protected loot source binding differs: {key}')
        for group in ('classification', 'charges', 'temporal'):
            canonical = profile['semantics'][group]
            proven = observed[key]['semantics'][group]
            if source.accepted.known_conflict(canonical, proven):
                raise ValueError(f'protected loot canonical {group} differs: {key}')
            profile['semantics'][group] = source.accepted.accepted_first(canonical, proven)
        existing[key] = profile
        items['records'].append(profile)
    for key, definition in definitions.items():
        if native_loot_yield_facts(definition['identity'], existing[key]) is None:
            raise ValueError(f'protected loot lacks native custody: {key}')
    items['records'].sort(key=order)
