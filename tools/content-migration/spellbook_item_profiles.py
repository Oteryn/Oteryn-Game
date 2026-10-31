"""I03: carry the Crystal spellbooks that canonical content can materialize into the native provider.

Crystal 201189f8 `data/scripts/actions/items/spellbook.lua:52` binds the spell list to these 19
item ids. Only an id whose canonical Item definition is already materializable, with an EXACT
Crystal binding and portable physical facts, can exist in a character inventory; the others stay
out of the provider rather than gaining a manufactured admission. 6120 is admitted earlier by the
quest-callback overlay, so it is skipped here.
"""
import copy
import hashlib
import json
import xml.etree.ElementTree as ET

CRYSTAL_SPELLBOOKS = (3059, 6120, 8072, 8073, 8074, 8075, 8076, 8090, 11691, 14769, 16107,
                      20088, 21400, 22755, 25699, 29431, 20089, 20090, 34153)
SOURCE = 'imports/canary/items-xml/items.xml'
SOURCE_SHA256 = '1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2'
KNOWN_TRUE = {'state': 'KNOWN', 'value': True}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    keys = {f'oteryn:item.tibia.i{number}': number for number in CRYSTAL_SPELLBOOKS}
    document = json.loads(generated[profile_path])
    present = {row['production_definition']['production_key'] for row in document['records']}
    definitions = {}
    for path in sorted((root / 'content/items/definitions').glob('items-*.json')):
        for row in json.loads(path.read_bytes())['records']:
            definition = row['definition']
            key = definition['identity']['key']
            if key in keys:
                if key in definitions:
                    raise ValueError('SPELLBOOK_DUPLICATE_ITEM')
                definitions[key] = definition
    if set(definitions) != set(keys):
        raise ValueError('SPELLBOOK_ITEM_MISSING')
    bindings = json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())['bindings']
    raw = (root / SOURCE).read_bytes()
    if hashlib.sha256(raw).hexdigest() != SOURCE_SHA256:
        raise ValueError('SPELLBOOK_SOURCE_DIGEST')
    source_items = {int(item.get('id')): item for item in ET.fromstring(raw).findall('item')
                    if item.get('id')}
    admitted = []
    for key, definition in sorted(definitions.items()):
        if key in present or not definition['materializable']:
            continue
        identity = definition['identity']
        matches = [binding for binding in bindings if binding['target'] == identity]
        physical = definition['semantics']['physical']
        if (definition['stack_class'] != 'NonStackable'
                or len(matches) != 1 or matches[0]['disposition'] != 'EXACT'
                or int(matches[0]['external_id']) != keys[key]
                or physical['state'] != 'KNOWN'
                or any(physical['value'][field] != KNOWN_TRUE for field in ('movable', 'pickupable'))):
            raise ValueError('SPELLBOOK_ITEM_NOT_ADMITTED')
        source = source_items.get(keys[key])
        if source is None or 'spellbook' not in source.get('name', ''):
            raise ValueError('SPELLBOOK_SOURCE_ITEM_MISSING')
        speeds = [attr for attr in source.findall('attribute')
                  if attr.get('key', '').lower() == 'speed']
        if speeds:
            raise ValueError('SPELLBOOK_UNEXPECTED_SPEED')
        document['records'].append({
            'authoring': {'item': identity},
            'semantics': copy.deepcopy(definition['semantics']),
            # Canary's Abilities constructor defaults speed to zero when XML omits it.
            'attributes': {'speed_bonus': 0},
            'admission': {'materializable': True, 'stack_class': 'NonStackable',
                          'legal_destinations': ['CharacterInventory', 'Ground']},
            'production_definition': {'family': identity['family'], 'production_key': key,
                                      'revision_ref': identity['revision']},
            'production_binding': matches[0], 'production_binding_qualification': None,
        })
        admitted.append(keys[key])
    if not admitted:
        return
    document['records'].sort(key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)
