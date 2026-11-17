"""Admit only the owner-selected shopping bag through existing canonical/native producers.

NPC-BUY-BACKPACKS-1 adopts Crystal 201189f's ITEM_SHOPPING_BAG (21411),
twenty slots. The established canonical physical/container/stack facts remain authoritative.
"""
import copy
import hashlib
import json

KEY = 'oteryn:item.tibia.i21411'


def apply_admissions(definitions):
    for definition in definitions:
        if definition['identity']['key'] != KEY:
            continue
        semantics = definition['semantics']
        if (semantics['container'] != {'state': 'KNOWN', 'value': {'capacity': {'state': 'KNOWN', 'value': 20}}}
                or semantics['stack']['value']['stackable'] != {'state': 'KNOWN', 'value': False}
                or semantics['physical']['value']['movable'] != {'state': 'KNOWN', 'value': True}
                or semantics['physical']['value']['pickupable'] != {'state': 'KNOWN', 'value': True}):
            raise ValueError('NPC_SHOPPING_BAG_FACT_CONFLICT')
        definition['materializable'] = True
        definition['stack_class'] = 'NonStackable'


def admit(root, generated, profile_path, manifest_path, order, canonical):
    source = next(row['definition'] for row in json.loads((root / 'content/items/definitions/items-10500-10999.json').read_bytes())['records']
                  if row['definition']['identity']['key'] == KEY)
    apply_admissions([source])
    bindings = json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())['bindings']
    binding = next(row for row in bindings if row['external_id'] == '21411')
    if binding['target'] != source['identity'] or binding['disposition'] != 'EXACT':
        raise ValueError('NPC_SHOPPING_BAG_IDENTITY_CONFLICT')
    document = json.loads(generated[profile_path])
    existing = next((row for row in document['records'] if row['production_definition']['production_key'] == KEY), None)
    admission = {'materializable': True, 'stack_class': 'NonStackable', 'legal_destinations': ['CharacterInventory', 'Ground']}
    if existing is None:
        document['records'].append({'authoring': {'item': source['identity']}, 'semantics': copy.deepcopy(source['semantics']),
                                   'attributes': {}, 'admission': admission,
                                   'production_definition': {'family': 'Item', 'production_key': KEY, 'revision_ref': source['identity']['revision']},
                                   'production_binding': binding, 'production_binding_qualification': None})
    else:
        if existing['production_binding'] != binding:
            raise ValueError('NPC_SHOPPING_BAG_NATIVE_IDENTITY_CONFLICT')
        existing['admission'] = admission
    document['records'].sort(key=order)
    data = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = data
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(data).hexdigest()
    generated[manifest_path] = canonical(manifest)
