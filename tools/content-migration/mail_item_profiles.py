"""MAIL-1: parcel, letter and label become carriable native Items.

Canary 52550d5e `data/items/items.xml` rows 3503-3507 carry no `type`/`stackable`
that would bar pickup; the World already holds their exact weights, nonstackable
class and parcel capacity. This layer adds only inventory admission; text and
other semantics keep whatever earlier layers proved.
"""
import copy
import hashlib
import json

FACTS = 'tools/content-schema/native-gameplay/mail-item-facts.json'
REVISION = '52550d5e3c0ffe565f4bd1014f4e5575abf1059b'
ITEMS_SHA256 = '1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2'
ROWS = [
    {'id': 3503, 'line': 10963, 'weight': 1800, 'container_capacity': 10},
    {'id': 3504, 'line': 10969, 'weight': 1800, 'container_capacity': 10},
    {'id': 3505, 'line': 10974, 'weight': 50, 'container_capacity': None},
    {'id': 3506, 'line': 10980, 'weight': 50, 'container_capacity': None},
    {'id': 3507, 'line': 10985, 'weight': 10, 'container_capacity': None},
]
KNOWN_TRUE = {'state': 'KNOWN', 'value': True}
KNOWN_FALSE = {'state': 'KNOWN', 'value': False}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    facts = json.loads((root / FACTS).read_bytes())
    if (facts['schema'] != 'OTERYN_MAIL_ITEM_SOURCE_FACTS/v1'
            or facts['source']['revision'] != REVISION
            or facts['source']['sha256'] != ITEMS_SHA256
            or facts['rows'] != ROWS):
        raise ValueError('MAIL_REFERENCE_FACTS')
    world = json.loads((root / 'content/world/definitions/reference.json').read_bytes())
    bindings = json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    document = json.loads(generated[profile_path])
    for fact in facts['rows']:
        key = f"oteryn:item.tibia.i{fact['id']}"
        sources = [r for r in world['records'] if r['identity']['key'] == key]
        identities = [b for b in bindings['bindings'] if b['target']['key'] == key]
        if (len(sources) != 1 or len(identities) != 1
                or identities[0]['disposition'] != 'EXACT'
                or sources[0]['identity'] != identities[0]['target']):
            raise ValueError('MAIL_PROFILE_IDENTITY')
        source, binding = sources[0], identities[0]
        physical = source['semantics']['physical']['value']
        stack = source['semantics']['stack']['value']
        container = source['semantics']['container']
        capacity = fact['container_capacity']
        if (physical['weight'] != {'state': 'KNOWN', 'value': fact['weight']}
                or any(physical[field] != KNOWN_TRUE for field in ('movable', 'pickupable'))
                or stack['stackable'] != KNOWN_FALSE
                or (capacity is None) != (container['state'] == 'UNKNOWN')
                or (capacity is not None
                    and container['value']['capacity'] != {'state': 'KNOWN', 'value': capacity})):
            raise ValueError('MAIL_PHYSICAL_FACTS')
        records = [r for r in document['records']
                   if r['production_definition']['production_key'] == key]
        if len(records) > 1:
            raise ValueError('MAIL_PROFILE_IDENTITY')
        if records:
            # The writable layer's text-only carrier: same identity, now admitted.
            record = records[0]
            if (record['admission'] != {'materializable': False, 'stack_class': 'NonStackable',
                                        'legal_destinations': ['Ground']}
                    or record['production_binding'] != binding
                    or record['semantics']['physical'] != source['semantics']['physical']
                    or record['semantics']['stack'] != source['semantics']['stack']):
                raise ValueError('MAIL_PROFILE_CONFLICT')
        else:
            record = {
                'authoring': {'item': source['identity']},
                'semantics': copy.deepcopy(source['semantics']), 'attributes': {},
                'production_definition': {'family': 'Item', 'production_key': key,
                                          'revision_ref': source['identity']['revision']},
                'production_binding': binding, 'production_binding_qualification': None,
            }
            document['records'].append(record)
        record['admission'] = {'materializable': True, 'stack_class': 'NonStackable',
                               'legal_destinations': ['CharacterInventory', 'Ground']}
    document['records'].sort(key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)
