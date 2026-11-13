"""Qualify only Rafzan elevator scenery; no portable Item admission."""
import hashlib
import json
from pathlib import Path

import qengine_reward_item_profiles as shared

ROOT = Path(__file__).resolve().parents[2]
PATH = 'tools/content-schema/native-gameplay/qm50-elevator-item-profiles.json'
IDS = {6218, 17938, 17939, 17940, 17942, 17943, 17944, 17947, 17970}
SCHEMA = 'OTERYN_QM50_ELEVATOR_ITEM_PROFILES/v1'


def capture(root=ROOT):
    source = shared.source
    xml = (root / shared.XML).read_bytes()
    appearances = (root / shared.CLIENT).read_bytes()
    if (hashlib.sha256(xml).hexdigest() != shared.XML_SHA
            or hashlib.sha256(appearances).hexdigest() != shared.CLIENT_SHA):
        raise ValueError('QM50_SOURCE_PIN')
    nodes = source.xml_index(xml)
    objects = source.engine_items.load_appearance_objects(appearances)
    physical = source.policies(appearances, xml, IDS)
    bindings = source.binding_index((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    keys = {f'oteryn:item.tibia.i{i}' for i in IDS}
    definitions = {}
    for path in (root / 'content/items/definitions').glob('items-*.json'):
        for row in json.loads(path.read_bytes())['records']:
            definition = row['definition']
            if definition['identity']['key'] in keys:
                definitions[definition['identity']['key']] = definition
    records = []
    for facts in physical:
        identifier = facts['server_item_id']
        if facts['movable'] or facts['pickupable']:
            raise ValueError('QM50_FIXED_SCENERY_REQUIRED')
        row = source.profile(identifier, nodes[identifier], objects[identifier],
                             facts, bindings, None)
        # Crystal rafzane_elevator.lua moves existing fixed scenery; it does not
        # grant portable Item mint admission to the elevator's projections.
        definition = definitions.get(f'oteryn:item.tibia.i{identifier}')
        if (definition is None or definition.get('materializable') is not False
                or row['admission'] != {'materializable': True,
                    'stack_class': 'NonStackable', 'legal_destinations': ['Ground']}):
            raise ValueError('QM50_CANONICAL_STATIONARY_ADMISSION')
        row['admission']['materializable'] = False
        records.append(row)
    bound = source.bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1',
                                 'records': records}, bindings, IDS)
    return {'schema': SCHEMA, 'producer': 'tools/content-migration/qm50_elevator_profiles.py',
            'sources': [{'path': shared.XML, 'sha256': shared.XML_SHA},
                        {'path': shared.CLIENT, 'sha256': shared.CLIENT_SHA}],
            'records': bound['records']}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    packet = capture(root)
    if json.loads((root / PATH).read_bytes()) != packet:
        raise ValueError('QM50_ELEVATOR_PROFILE_DRIFT')
    shared.admit_records(root, generated, profile_path, manifest_path, order, canonical,
                         packet['records'])


if __name__ == '__main__':
    (ROOT / PATH).write_text(json.dumps(capture(), indent=2) + '\n')
