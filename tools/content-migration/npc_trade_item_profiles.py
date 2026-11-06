"""NPCFIX: compose Albinius's already admitted Tome into the active Item provider.

The canonical Game definition owns every fact. This does not infer an Item action,
quest effect or new identity, and does not relax the runtime inventory gate.
"""
import copy
import hashlib
import json

KEY = 'oteryn:item.tibia.i23986'


def _known(value):
    return {'state': 'KNOWN', 'value': value}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    definitions = [row['definition']
                   for path in sorted((root / 'content/items/definitions').glob('items-*.json'))
                   for row in json.loads(path.read_bytes())['records']
                   if row['definition']['identity']['key'] == KEY]
    if len(definitions) != 1:
        raise ValueError('NPCFIX_TOME_DEFINITION')
    definition = definitions[0]
    if (definition['materializable'] is not True
            or definition['stack_class'] != 'StackCapable'
            or definition['semantics']['stack'] != _known({'stackable': _known(True), 'stack_max': _known(100)})
            or definition['semantics']['physical'] != _known({
                'movable': _known(True), 'pickupable': _known(True), 'weight': _known(1000)})):
        raise ValueError('NPCFIX_TOME_CANONICAL_ADMISSION')
    bindings = json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    matches = [binding for binding in bindings['bindings'] if binding['target']['key'] == KEY]
    if len(matches) != 1 or matches[0]['disposition'] != 'EXACT' or matches[0]['target'] != definition['identity']:
        raise ValueError('NPCFIX_TOME_IDENTITY')
    document = json.loads(generated[profile_path])
    if any(row['production_definition']['production_key'] == KEY for row in document['records']):
        raise ValueError('NPCFIX_TOME_DUPLICATE')
    document['records'].append({
        'authoring': {'item': definition['identity']},
        'semantics': copy.deepcopy(definition['semantics']),
        'attributes': {'speed_bonus': 0},
        'admission': {'materializable': True, 'stack_class': 'StackCapable',
                      'legal_destinations': ['CharacterInventory', 'Ground']},
        'production_definition': {'family': 'Item', 'production_key': KEY,
                                  'revision_ref': definition['identity']['revision']},
        'production_binding': matches[0],
        'production_binding_qualification': None,
    })
    document['records'].sort(key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)
