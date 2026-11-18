"""Materialize the two portable, nonstacking Upper Spike tools."""
import copy
import hashlib
import json

TOOLS = {
    'oteryn:item.tibia.i19203': ('spirit shovel', 2800, '19203'),
    'oteryn:item.tibia.i19205': ('ghost detector', 900, '19205'),
}


def apply_admissions(definitions):
    found = set()
    for definition in definitions:
        key = definition['identity']['key']
        if key not in TOOLS:
            continue
        name, weight, _ = TOOLS[key]
        semantics = definition['semantics']
        physical = semantics['physical']['value']
        if (key in found or definition['identity']['revision'] != 'definition-r1'
                or semantics['presentation']['value']['name'] != {'state': 'KNOWN', 'value': name}
                or physical['weight'] != {'state': 'KNOWN', 'value': weight}
                or any(physical[field] != {'state': 'KNOWN', 'value': True}
                       for field in ('movable', 'pickupable'))
                or semantics['stack']['value']['stackable'] != {'state': 'KNOWN', 'value': False}
                or definition['stack_class'] not in ('Unknown', 'NonStackable')):
            raise ValueError('SPIKE_TOOL_SOURCE_CHANGED:' + key)
        definition['materializable'] = True
        definition['stack_class'] = 'NonStackable'
        found.add(key)
    if found != TOOLS.keys():
        raise ValueError('SPIKE_TOOL_COHORT_INCOMPLETE')


def admit(root, generated, profile_path, manifest_path, order, canonical):
    definitions = [row['definition']
                   for row in json.loads((root / 'content/items/definitions/items-08000-08499.json').read_bytes())['records']
                   if row['definition']['identity']['key'] in TOOLS]
    for definition in definitions:
        if not definition['materializable'] or definition['stack_class'] != 'NonStackable':
            raise ValueError('SPIKE_TOOL_CANONICAL_ADMISSION_MISSING')
    apply_admissions(definitions)
    bindings = json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())['bindings']
    profiles = json.loads(generated[profile_path])
    for definition in definitions:
        identity = definition['identity']
        key = identity['key']
        binding, = [row for row in bindings if row['target'] == identity
                    and row['external_id'] == TOOLS[key][2]
                    and row['identity_namespace'] == 'ots/item_server_id'
                    and row['disposition'] == 'EXACT']
        if any(row['production_definition']['production_key'] == key for row in profiles['records']):
            raise ValueError('SPIKE_TOOL_NATIVE_ADMISSION_DUPLICATE:' + key)
        profiles['records'].append({
            'authoring': {'item': identity},
            'semantics': copy.deepcopy(definition['semantics']),
            'attributes': {'speed_bonus': 0},
            'admission': {'materializable': True, 'stack_class': 'NonStackable',
                          'legal_destinations': ['CharacterInventory', 'Ground']},
            'production_definition': {'family': 'Item', 'production_key': key,
                                      'revision_ref': identity['revision']},
            'production_binding': binding, 'production_binding_qualification': None,
        })
    profiles['records'].sort(key=order)
    raw = canonical(profiles)
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)
