"""Closed source admission for the walkable, Ground-only Serpentine barrel."""
import hashlib
import json
import qengine_reward_item_profiles as shared
import native_profile_coexistence


def capture(root=shared.ROOT):
    xml = (root / shared.XML).read_bytes()
    client = (root / shared.CLIENT).read_bytes()
    if hashlib.sha256(xml).hexdigest() != shared.XML_SHA or hashlib.sha256(client).hexdigest() != shared.CLIENT_SHA:
        raise ValueError('QUESTPC5_MAP_SOURCE_PIN')
    source = shared.source
    node = source.xml_index(xml)[2523]
    if source.attrs(node) != {'primarytype': 'containers', 'containersize': '25'} or node.get('name') != 'barrel':
        raise ValueError('QUESTPC5_BARREL_SOURCE_DRIFT')
    bindings = source.binding_index((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    physical, = source.policies(client, xml, {2523})
    row = source.profile(2523, node, source.engine_items.load_appearance_objects(client)[2523], physical, bindings, None)
    if (row['attributes']['blocks_movement'] or row['attributes']['blocks_projectile']
            or not row['attributes']['has_height'] or physical['pickupable'] or not physical['movable']
            or row['admission'] != dict(materializable=True, stack_class='NonStackable', legal_destinations=['Ground'])):
        raise ValueError('QUESTPC5_BARREL_PHYSICAL_DRIFT')
    return source.bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': [row]}, bindings, {2523})['records'][0]


def admit(root, generated, profile_path, manifest_path, order, canonical):
    row = capture(root)
    document = json.loads(generated[profile_path])
    matches = [r for r in document['records'] if r['production_definition']['production_key'] == row['production_definition']['production_key']]
    if len(matches) > 1:
        raise ValueError('QUESTPC5_BARREL_DUPLICATE')
    if matches:
        merged = native_profile_coexistence.reconcile(matches[0], row, 'QUESTPC5_BARREL_CONFLICT')
        document['records'][document['records'].index(matches[0])] = merged
    else:
        document['records'].append(row)
    document['records'].sort(key=order)
    raw = canonical(document)
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)
