"""SRV9B: qualify Lynda's two previously nonmaterializable wedding items.

Crystal 201189f8 items.xml:25795-25804 and Canary 47dfd51f agree on
9585/9586, weights40/2500 and nonstacking. Physical/stack facts and exact
identity bindings remain the existing source-qualified World records.
"""
import copy
import hashlib
import json

from native_profile_composition import encode_native_profile

WEIGHTS = {9585: 40, 9586: 2500}


def known(value):
    return {'state': 'KNOWN', 'value': value}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    world = json.loads((root / 'content/world/definitions/reference.json').read_bytes())
    bindings = json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    document = json.loads(generated[profile_path])
    for number, weight in sorted(WEIGHTS.items()):
        key = f'oteryn:item.tibia.i{number}'
        source, = [r for r in world['records'] if r['identity']['key'] == key]
        binding, = [b for b in bindings['bindings'] if b['target']['key'] == key]
        semantics = copy.deepcopy(source['semantics'])
        physical = semantics.get('physical', {}).get('value', {})
        if (binding['disposition'] != 'EXACT' or binding['target'] != source['identity']
                or physical.get('pickupable') != known(True)
                or physical.get('movable') != known(True)
                or physical.get('weight') != known(weight)
                or semantics.get('stack', {}).get('value', {}).get('stackable') != known(False)):
            raise ValueError('MARRIAGE_ITEM_SOURCE_DRIFT')
        if any(r['production_definition']['production_key'] == key for r in document['records']):
            raise ValueError('MARRIAGE_ITEM_DUPLICATE')
        document['records'].append({
            'authoring': {'item': source['identity']}, 'semantics': semantics,
            'attributes': {'speed_bonus': 0},
            'admission': {'materializable': True, 'stack_class': 'NonStackable',
                          'legal_destinations': ['CharacterInventory', 'Ground']},
            'production_definition': {'family': 'Item', 'production_key': key,
                                      'revision_ref': source['identity']['revision']},
            'production_binding': binding, 'production_binding_qualification': None,
        })
    document['records'].sort(key=order)
    raw = encode_native_profile(document)
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)
