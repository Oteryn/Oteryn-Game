"""Pinned, Ground-only physical policies for First Dragon's four exit gems."""
import hashlib
import json

import qengine_reward_item_profiles as owner

IDS = {8646, 8647, 8648, 8649}


def capture(root):
    xml = (root / owner.XML).read_bytes()
    client = (root / owner.CLIENT).read_bytes()
    if (hashlib.sha256(xml).hexdigest() != owner.XML_SHA
            or hashlib.sha256(client).hexdigest() != owner.CLIENT_SHA):
        raise ValueError('FIRST_DRAGON_EXIT_SOURCE_PIN')
    source = owner.source
    nodes = source.xml_index(xml)
    objects = source.engine_items.load_appearance_objects(client)
    bindings = source.binding_index((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    records = []
    for physical in source.policies(client, xml, IDS):
        ident = physical['server_item_id']
        record = source.profile(ident, nodes[ident], objects[ident], physical, bindings, None)
        if (source.attrs(nodes[ident]) or physical['block_solid']
                or physical['movable'] or physical['pickupable']
                or record['admission']['legal_destinations'] != ['Ground']):
            raise ValueError('FIRST_DRAGON_EXIT_PHYSICAL_DRIFT')
        records.append(record)
    return source.bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1',
                                'records': records}, bindings, IDS)['records']


def admit(root, generated, profile_path, manifest_path, order, canonical):
    records = capture(root)
    proof = {'schema': 'OTERYN_FIRST_DRAGON_EXIT_PROFILES/v1',
             'sources': [{'path': owner.XML, 'sha256': owner.XML_SHA},
                         {'path': owner.CLIENT, 'sha256': owner.CLIENT_SHA}],
             'records': records}
    generated['tools/content-schema/native-gameplay/questpc3-exit-profiles.json'] = (
        json.dumps(proof, indent=2, ensure_ascii=False) + '\n').encode()
    owner.admit_records(root, generated, profile_path, manifest_path, order, canonical, records)
