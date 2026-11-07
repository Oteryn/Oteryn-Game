"""MINING-1: three native policies from admitted identities and pinned source facts.

The World already knows their weights and physical properties. Only the exact
Reference stack facts are added; unknown unrelated semantics remain unknown.
"""
import copy
import hashlib
import json

FACTS = 'tools/content-schema/native-gameplay/mining-item-facts.json'
IDS = {3028, 3456, 10310}
APPEARANCE_SHA256 = 'aa44a154f30c7ed59acc25f246286396e4043851ef0b54ef3cf3951e46d1ce50'


def admit(root, generated, profile_path, manifest_path, order, canonical):
    facts = json.loads((root / FACTS).read_bytes())
    if (facts['schema'] != 'OTERYN_PICK_ITEM_SOURCE_FACTS/v1'
            or facts['source']['revision'] != '52550d5e3c0ffe565f4bd1014f4e5575abf1059b'
            or facts['source']['sha256'] != APPEARANCE_SHA256
            or facts['rows'] != [
                {'id': 3028, 'stackable': True, 'pickupable': True},
                {'id': 3456, 'stackable': False, 'pickupable': True},
                {'id': 10310, 'stackable': True, 'pickupable': True}]):
        raise ValueError('MINING_REFERENCE_FACTS')
    world = json.loads((root / 'content/world/definitions/reference.json').read_bytes())
    bindings = json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    document = json.loads(generated[profile_path])
    for fact in facts['rows']:
        key = f"oteryn:item.tibia.i{fact['id']}"
        sources = [r for r in world['records'] if r['identity']['key'] == key]
        identities = [b for b in bindings['bindings'] if b['target']['key'] == key]
        if (len(sources) != 1 or len(identities) != 1
                or identities[0]['disposition'] != 'EXACT'
                or sources[0]['identity'] != identities[0]['target']
                or any(r['production_definition']['production_key'] == key for r in document['records'])):
            raise ValueError('MINING_PROFILE_IDENTITY')
        source, binding = sources[0], identities[0]
        semantics = copy.deepcopy(source['semantics'])
        physical = semantics['physical']['value']
        if (physical['weight'] != {'state': 'KNOWN', 'value': {3028: 10, 3456: 4500, 10310: 156}[fact['id']]}
                or any(physical[field] != {'state': 'KNOWN', 'value': True} for field in ('movable', 'pickupable'))):
            raise ValueError('MINING_PHYSICAL_FACTS')
        semantics['stack'] = {'state': 'KNOWN', 'value': {
            'stackable': {'state': 'KNOWN', 'value': fact['stackable']},
            'stack_max': {'state': 'KNOWN', 'value': 100} if fact['stackable'] else {'state': 'NOT_APPLICABLE'}}}
        document['records'].append({
            'authoring': {'item': source['identity']}, 'semantics': semantics, 'attributes': {},
            'admission': {'materializable': True, 'stack_class': 'StackCapable' if fact['stackable'] else 'NonStackable',
                          'legal_destinations': ['CharacterInventory', 'Ground']},
            'production_definition': {'family': 'Item', 'production_key': key, 'revision_ref': source['identity']['revision']},
            'production_binding': binding, 'production_binding_qualification': None,
        })
    document['records'].sort(key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)
