"""Carry already admitted quest offering definitions into the native Item provider."""
import copy
import hashlib
import json


def admit(root, generated, profile_path, manifest_path, order, canonical):
    callbacks = json.loads((root / 'content/quests/world-callbacks.json').read_bytes())
    keys = {offering['item'] for callback in callbacks['callbacks']
            for offering in callback['offerings']}
    document = json.loads(generated[profile_path])
    present = {row['production_definition']['production_key'] for row in document['records']}
    missing = keys - present
    definitions = {}
    for path in sorted((root / 'content/items/definitions').glob('items-*.json')):
        for row in json.loads(path.read_bytes())['records']:
            definition = row['definition']
            key = definition['identity']['key']
            if key in missing:
                if key in definitions:
                    raise ValueError('QUEST_CALLBACK_DUPLICATE_ITEM')
                definitions[key] = definition
    if set(definitions) != missing:
        raise ValueError('QUEST_CALLBACK_ITEM_MISSING')
    bindings = json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())['bindings']
    for key, definition in sorted(definitions.items()):
        identity = definition['identity']
        matches = [binding for binding in bindings if binding['target'] == identity]
        if (not definition['materializable']
                or definition['stack_class'] not in ('StackCapable', 'NonStackable')
                or len(matches) != 1 or matches[0]['disposition'] != 'EXACT'):
            raise ValueError('QUEST_CALLBACK_ITEM_NOT_ADMITTED')
        physical = definition['semantics']['physical']
        if (physical['state'] != 'KNOWN'
                or any(physical['value'][field] != {'state': 'KNOWN', 'value': True}
                       for field in ('movable', 'pickupable'))):
            raise ValueError('QUEST_CALLBACK_ITEM_NOT_PORTABLE')
        document['records'].append({
            'authoring': {'item': identity},
            'semantics': copy.deepcopy(definition['semantics']), 'attributes': {},
            'admission': {'materializable': True, 'stack_class': definition['stack_class'],
                          'legal_destinations': ['CharacterInventory', 'Ground']},
            'production_definition': {'family': identity['family'], 'production_key': key,
                                      'revision_ref': identity['revision']},
            'production_binding': matches[0], 'production_binding_qualification': None,
        })
    document['records'].sort(key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)
