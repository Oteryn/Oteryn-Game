"""Admit the physical item used by the existing Pegasus Feather action."""

import hashlib
import json

import qengine_reward_item_profiles as items

PROOF = 'tools/content-schema/native-gameplay/pegasus-feather-item-profile.json'


def admit(root, generated, profile_path, manifest_path, order, canonical):
    item_id = 48424
    source = items.source
    xml = (root / items.XML).read_bytes()
    appearances = (root / items.CLIENT).read_bytes()
    if (hashlib.sha256(xml).hexdigest() != items.XML_SHA
            or hashlib.sha256(appearances).hexdigest() != items.CLIENT_SHA):
        raise ValueError('Pegasus Feather source pins drifted')
    node = source.xml_index(xml)[item_id]
    attrs = source.attrs(node)
    # Crystal 201189f data/items/items.xml:81676-81679 matches the retained source row.
    if attrs != {'primarytype': 'taming items', 'weight': '100'}:
        raise ValueError('Pegasus Feather source attributes drifted')

    shard = json.loads((root / 'content/items/definitions/items-26500-26999.json').read_bytes())
    key = f'oteryn:item.tibia.i{item_id}'
    definition = next(row['definition'] for row in shard['records']
                      if row['definition']['identity']['key'] == key)
    physical = definition['semantics']['physical']['value']
    if physical['weight'] != source.K(10):
        raise ValueError('Pegasus Feather reference weight drifted')
    # The newly admitted custody profile uses Crystal's weight over the wiki value.
    physical['weight'] = source.K(int(attrs['weight']))
    objects = source.engine_items.load_appearance_objects(appearances)
    observed, = source.policies(appearances, xml, {item_id})
    bindings = source.binding_index((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    record = items.accepted.qualify(item_id, node, objects[item_id], observed, bindings, definition)
    packet = {'schema': 'OTERYN_PEGASUS_FEATHER_ITEM_PROFILE/v1',
              'sources': [{'path': items.XML, 'sha256': items.XML_SHA, 'line': 80788},
                          {'path': items.CLIENT, 'sha256': items.CLIENT_SHA}],
              'records': [record]}
    generated[PROOF] = (json.dumps(packet, ensure_ascii=False, indent=2) + '\n').encode()
    items.admit_records(root, generated, profile_path, manifest_path, order, canonical, packet['records'])
