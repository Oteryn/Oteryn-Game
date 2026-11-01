"""PAR-M10: admit the captured wolf 12369 into the native provider so the magic wolf trap can mint it.

Crystal 201189f8 `data/scripts/movements/trap.lua:39-42` removes a Starving Wolf that steps on
the magic wolf trap 12368 and creates one 12369 on the trap's tile. The World reference already
knows its facts from the digest-pinned Canary XML (`items.xml`, item 12369: weight 5000, no
speed attribute): movable, pickupable and not stackable. Only those known facts are carried;
the other semantics stay as the reference has them. A captured wolf is a quest item a player
picks up, so it is admitted for the ground (where the trap creates it) and a character
inventory.
"""
import copy
import hashlib
import json
import xml.etree.ElementTree as ET

ITEM = 12369
KEY = f'oteryn:item.tibia.i{ITEM}'
SOURCE = 'imports/canary/items-xml/items.xml'
SOURCE_SHA256 = '1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2'
KNOWN_TRUE = {'state': 'KNOWN', 'value': True}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    document = json.loads(generated[profile_path])
    if any(r['production_definition']['production_key'] == KEY for r in document['records']):
        return
    raw = (root / SOURCE).read_bytes()
    if hashlib.sha256(raw).hexdigest() != SOURCE_SHA256:
        raise ValueError('CAPTURED_WOLF_SOURCE_DIGEST')
    nodes = [n for n in ET.fromstring(raw).findall('item') if n.get('id') == str(ITEM)]
    if len(nodes) != 1 or nodes[0].get('name') != 'captured wolf':
        raise ValueError('CAPTURED_WOLF_SOURCE_ITEM')
    attrs = {a.get('key').lower(): a.get('value') for a in nodes[0].findall('attribute')}
    if attrs.get('weight') != '5000' or 'speed' in attrs:
        raise ValueError('CAPTURED_WOLF_SOURCE_FACTS')
    world = json.loads((root / 'content/world/definitions/reference.json').read_bytes())
    bindings = json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    sources = [r for r in world['records'] if r['identity']['key'] == KEY]
    matches = [b for b in bindings['bindings'] if b['target']['key'] == KEY]
    if (len(sources) != 1 or len(matches) != 1 or matches[0]['disposition'] != 'EXACT'
            or int(matches[0]['external_id']) != ITEM
            or matches[0]['target'] != sources[0]['identity']):
        raise ValueError('CAPTURED_WOLF_IDENTITY')
    source, binding = sources[0], matches[0]
    semantics = copy.deepcopy(source['semantics'])
    physical, stack = semantics['physical'], semantics['stack']
    if (physical['state'] != 'KNOWN'
            or physical['value']['weight'] != {'state': 'KNOWN', 'value': 5000}
            or any(physical['value'][field] != KNOWN_TRUE for field in ('movable', 'pickupable'))
            or stack['state'] != 'KNOWN'
            or stack['value']['stackable'] != {'state': 'KNOWN', 'value': False}):
        raise ValueError('CAPTURED_WOLF_PHYSICAL_FACTS')
    document['records'].append({
        'authoring': {'item': source['identity']}, 'semantics': semantics,
        # Canary's Abilities constructor defaults speed to zero when XML omits it.
        'attributes': {'speed_bonus': 0},
        'admission': {'materializable': True, 'stack_class': 'NonStackable',
                      'legal_destinations': ['CharacterInventory', 'Ground']},
        'production_definition': {'family': 'Item', 'production_key': KEY,
                                  'revision_ref': source['identity']['revision']},
        'production_binding': binding, 'production_binding_qualification': None,
    })
    document['records'].sort(key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)
