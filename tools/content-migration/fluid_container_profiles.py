"""I10a: admit the carried fluid containers into native content.

The identities are the ones Crystal 201189f8 registers for its fluid action
(`data/scripts/actions/items/fluids.lua`, `fluid:id(...)`), minus the trough 2524 and the
large amphora 2904, which the World reference does not know as pickupable: they are map
fixtures and are filled from, not carried. A container's content is the item's durable fluid
sub-type (migration 0560), not part of its definition, so `fluid` is KNOWN with
`fluid_type` NOT_APPLICABLE. Physical facts stay the World's; fluid containers do not stack.
"""
import copy
import hashlib
import json

IDS = {2873, 2874, 2875, 2876, 2877, 2879, 2880, 2881, 2882, 2883, 2884, 2885, 2893, 2901,
       2902, 2903, 3477, 3478, 3479, 3480}


def known(value):
    return {'state': 'KNOWN', 'value': value}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    world = json.loads((root / 'content/world/definitions/reference.json').read_bytes())
    bindings = json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    document = json.loads(generated[profile_path])
    for number in sorted(IDS):
        key = f'oteryn:item.tibia.i{number}'
        source, = [r for r in world['records'] if r['identity']['key'] == key]
        binding, = [b for b in bindings['bindings'] if b['target']['key'] == key]
        if binding['disposition'] != 'EXACT' or binding['target'] != source['identity']:
            raise ValueError('FLUID_IDENTITY')
        semantics = copy.deepcopy(source['semantics'])
        physical = semantics['physical']['value']
        if physical.get('pickupable') != known(True) or physical.get('movable') != known(True) \
                or physical.get('weight', {}).get('state') != 'KNOWN':
            raise ValueError('FLUID_SOURCE_FACTS')
        semantics['fluid'] = known({'fluid_type': {'state': 'NOT_APPLICABLE'}})
        semantics['stack'] = known({'stackable': known(False), 'stack_max': {'state': 'NOT_APPLICABLE'}})
        semantics['container'] = {'state': 'NOT_APPLICABLE'}
        semantics['charges'] = {'state': 'NOT_APPLICABLE'}
        semantics['temporal'] = {'state': 'NOT_APPLICABLE'}
        if any(r['production_definition']['production_key'] == key for r in document['records']):
            raise ValueError('FLUID_DUPLICATE_PROFILE')
        document['records'].append({
            'authoring': {'item': source['identity']}, 'semantics': semantics, 'attributes': {'speed_bonus': 0},
            'admission': {'materializable': True, 'stack_class': 'NonStackable',
                          'legal_destinations': ['CharacterInventory', 'Ground']},
            'production_definition': {'family': 'Item', 'production_key': key, 'revision_ref': source['identity']['revision']},
            'production_binding': binding, 'production_binding_qualification': None,
        })
    taxonomy = {r['target']['key']: r['source_taxonomy'] for r in json.loads((root / 'content/items/taxonomy/items.json').read_bytes())['records']}
    keys = {f'oteryn:item.tibia.i{n}' for n in IDS}
    for record in document['records']:
        key = record['production_definition']['production_key']
        if key in taxonomy and key in keys:
            record['authoring']['taxonomy'] = copy.deepcopy(taxonomy[key])
    document['records'].sort(key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)
