"""Project already-qualified native Item custody into scoped ordinary quest-creature loot.

The active Item provider is authoritative for materialization, not a stale canonical
reference-only flag. This bridge reuses exact admitted profiles and existing loot
owners; it never changes ordinary loot entries, probabilities, quantities or holds.
"""
from __future__ import annotations

import copy
import json
from pathlib import Path

import quest_corpse_profiles as corpse

ROOT = Path(__file__).resolve().parents[2]
PROFILE = 'content/items/definitions/spell-native-profiles.json'
MAX_ITEMS, MAX_DEPTH = 512, 8


def required_items(root, creatures, tables):
    # Reuse the validated active rule/objective/native-Creature join.
    corpse.required(root, creatures)
    objectives = {row['key']: row for row in json.loads((root / corpse.OBJECTIVES).read_bytes())['objectives']}
    rules = json.loads((root / corpse.RULES).read_bytes())['rules']
    rule_creatures = {objectives[rule['objective']]['creature'] for rule in rules}
    ordinary = json.loads((root / corpse.REQUIREMENTS).read_bytes())['objectives']
    selected = rule_creatures | {objectives[key]['creature'] for key in ordinary}
    native = {row['profile']['target']['key']: row['profile']['target'] for row in creatures['records']}
    definitions = {}
    for path in sorted((root / 'content/creatures/definitions').glob('creatures-*.json')):
        for row in json.loads(path.read_bytes())['records']:
            definition = row['definition']
            if definition['identity']['key'] in selected:
                if definition['identity']['key'] in definitions:
                    raise ValueError('QUEST_LOOT_DUPLICATE_CREATURE_DEFINITION')
                definitions[definition['identity']['key']] = definition
    found = {}

    def entries(values, depth=0):
        if depth > MAX_DEPTH or len(values) > MAX_ITEMS:
            raise ValueError('QUEST_LOOT_ENTRY_BOUNDS')
        for entry in values:
            reference = entry['item']
            corpse.item_id(reference)
            key = reference['key']
            if key in found and found[key] != reference:
                raise ValueError('QUEST_LOOT_ITEM_REVISION_CONFLICT')
            found[key] = reference
            if len(found) > MAX_ITEMS:
                raise ValueError('QUEST_LOOT_ITEM_BOUNDS')
            entries(entry.get('children', []), depth + 1)

    for creature in sorted(selected):
        definition = definitions.get(creature)
        # The real loot generator binds a null table when a native-only Creature
        # has no canonical definition. A quest-loot rule still requires one.
        if definition is None and creature not in rule_creatures:
            continue
        if definition is None or definition['identity'] != native[creature]:
            raise ValueError('QUEST_LOOT_CREATURE_DEFINITION_BINDING')
        reference = definition['loot']
        if reference is None:
            continue
        table = tables.get(reference['key'])
        if table is None or table['identity'] != reference:
            raise ValueError('QUEST_LOOT_EXACT_TABLE_BINDING')
        entries(table['entries'])
    return found


def admit(root, generated, profile_path, manifest_path, order, canonical):
    """Admit the exact scoped ordinary loot cohort through existing custody owners."""
    import hashlib
    import authored_quest_item_custody as custody
    creatures = json.loads(generated.get(corpse.CREATURES, (root / corpse.CREATURES).read_bytes()))
    tables = {}
    for path in sorted((root / 'content/loot').glob('loot-*.json')):
        for row in json.loads(path.read_bytes())['records']:
            definition = row['definition']
            if definition['identity']['key'] in tables:
                raise ValueError('QUEST_LOOT_DUPLICATE_TABLE')
            tables[definition['identity']['key']] = definition
    wanted = required_items(root, creatures, tables)
    document = json.loads(generated[profile_path])
    existing = {row['production_definition']['production_key']: row for row in document['records']}
    definitions = {}
    for path in sorted((root / 'content/items/definitions').glob('items-*.json')):
        for row in json.loads(path.read_bytes())['records']:
            definition = row['definition']
            key = definition['identity']['key']
            if key in wanted and key not in existing:
                if key in definitions:
                    raise ValueError('QUEST_LOOT_DUPLICATE_CANONICAL_ITEM')
                definitions[key] = definition
    if set(definitions) != set(wanted) - set(existing):
        raise ValueError('QUEST_LOOT_CANONICAL_ITEM_MISSING')
    raw = (root / corpse.XML).read_bytes()
    records = custody.qualify_many(root, [definition for _, definition in sorted(definitions.items())],
                                   raw, terminal_expiry=True)
    document['records'].extend(records)
    document['records'].sort(key=order)
    generated[profile_path] = canonical(document)
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(generated[profile_path]).hexdigest()
    generated[manifest_path] = canonical(manifest)


def project(fact, record, binding, *, source_charges_absent=False):
    """One exact, previously source-qualified native profile; no new source inference."""
    identity = fact['item']
    typed = {'family': identity['family'], 'production_key': identity['key'],
             'revision_ref': identity['revision']}
    if (record['production_definition'] != typed or binding.get('disposition') != 'EXACT'
            or binding.get('target') != identity or record['production_binding'] != binding):
        raise ValueError('QUEST_LOOT_NATIVE_IDENTITY_BINDING')
    admission, semantics = record['admission'], record['semantics']
    if (admission['materializable'] is not True
            or admission['legal_destinations'] != ['CharacterInventory', 'Ground']
            or admission['stack_class'] not in ('NonStackable', 'StackCapable')):
        raise ValueError('QUEST_LOOT_NATIVE_CUSTODY_NOT_QUALIFIED')
    physical = semantics['physical']
    value = physical.get('value', {})
    if (physical.get('state') != 'KNOWN'
            or value.get('movable') != corpse.grizzly.source.K(True)
            or value.get('pickupable') != corpse.grizzly.source.K(True)
            or value.get('weight', {}).get('state') != 'KNOWN'
            or type(value['weight'].get('value')) is not int
            or not 0 <= value['weight']['value'] <= 0xffffffff):
        raise ValueError('QUEST_LOOT_NATIVE_PHYSICAL_NOT_QUALIFIED')
    stackable = admission['stack_class'] == 'StackCapable'
    stack = semantics['stack']
    if (stack.get('state') != 'KNOWN'
            or stack['value'].get('stackable') != corpse.grizzly.source.K(stackable)
            or (stackable and (stack['value'].get('stack_max', {}).get('state') != 'KNOWN'
                              or type(stack['value']['stack_max'].get('value')) is not int
                              or not 1 <= stack['value']['stack_max']['value'] <= 100))):
        raise ValueError('QUEST_LOOT_NATIVE_STACK_NOT_QUALIFIED')
    container = semantics['container']
    if container.get('state') == 'KNOWN':
        capacity = container['value']['capacity']
        if capacity.get('state') != 'KNOWN' or type(capacity.get('value')) is not int or not 1 <= capacity['value'] <= 255:
            raise ValueError('QUEST_LOOT_NATIVE_CAPACITY_NOT_QUALIFIED')
        capacity = capacity['value']
    elif container == corpse.grizzly.source.NA:
        capacity = None
    else:
        raise ValueError('QUEST_LOOT_NATIVE_CONTAINER_NOT_QUALIFIED')
    if fact['container_capacity'] is not None and fact['container_capacity'] != capacity:
        raise ValueError('QUEST_LOOT_ACCEPTED_CAPACITY_CONFLICT')
    result = copy.deepcopy(fact)
    result['materializable'] = True
    result['container_capacity'] = capacity
    charges = semantics['charges']
    if charges.get('state') == 'KNOWN':
        count = charges['value']['count']
        if count.get('state') != 'KNOWN' or type(count.get('value')) is not int or not 1 <= count['value'] <= 65535:
            raise ValueError('QUEST_LOOT_NATIVE_CHARGES_NOT_QUALIFIED')
        if fact.get('charges', count['value']) != count['value']:
            raise ValueError('QUEST_LOOT_ACCEPTED_CHARGE_CONFLICT')
        result['charges'] = count['value']
        if stackable:
            result['stackable'] = True
    elif (charges != corpse.grizzly.source.NA
          and not (source_charges_absent and charges == corpse.grizzly.source.U)) or 'charges' in fact:
        raise ValueError('QUEST_LOOT_NATIVE_CHARGE_POLICY_CONFLICT')
    return result


def apply_loot_facts(facts, root, creatures, tables, profiles=None):
    import hashlib
    required = required_items(root, creatures, tables)
    profiles = profiles or json.loads((root / PROFILE).read_bytes())
    native = {row['production_definition']['production_key']: row for row in profiles['records']}
    if len(native) != len(profiles['records']):
        raise ValueError('QUEST_LOOT_DUPLICATE_NATIVE_ITEM')
    bindings = json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())['bindings']
    raw = (root / corpse.XML).read_bytes()
    if hashlib.sha256(raw).hexdigest() != corpse.XML_SHA:
        raise ValueError('QUEST_LOOT_SOURCE_XML_PIN')
    xml = corpse.grizzly.source.xml_index(raw)
    updates = {}
    for key, reference in required.items():
        fact = facts.get(key)
        if fact is None or fact['item'] != reference:
            raise ValueError('QUEST_LOOT_CANONICAL_FACT_IDENTITY')
        if fact['materializable']:
            continue
        matches = [binding for binding in bindings if binding['target'] == reference]
        if key not in native or not matches:
            raise ValueError('QUEST_LOOT_NATIVE_ADMISSION_MISSING:' + key)
        if len(matches) != 1:
            raise ValueError('QUEST_LOOT_AMBIGUOUS_EXACT_BINDING:' + key)
        number = corpse.item_id(reference)
        if number not in xml:
            raise ValueError('QUEST_LOOT_SOURCE_ITEM_ABSENT')
        updates[key] = project(fact, native[key], matches[0],
                               source_charges_absent='charges' not in corpse.grizzly.source.attrs(xml[number]))
    # Publish only after the complete scoped cohort passes; no partial input mutation.
    facts.update(updates)
