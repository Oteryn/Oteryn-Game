"""Source-qualified physical turnips for M14; the quest owner supplies the use effect."""
import argparse
import hashlib
import json
from pathlib import Path

import grizzly_shop_profiles as owner
import qengine_reward_item_profiles as shared
import quest_batch3_item_profiles as source_pin

ROOT = Path(__file__).resolve().parents[2]
PATH = 'tools/content-schema/native-gameplay/srv7-targuna-item-profiles.json'
KEY = 'oteryn:item.tibia.i53073'
SCHEMA = 'OTERYN_SRV7_TARGUNA_ITEM_PROFILES/v1'


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def definition(root):
    rows = [r['definition'] for p in (root / 'content/items/definitions').glob('items-*.json')
            for r in json.loads(p.read_bytes())['records'] if r['definition']['identity']['key'] == KEY]
    if len(rows) != 1:
        raise ValueError('SRV7_TURNIP_CANONICAL_IDENTITY')
    return rows[0]


def capture(source_root, root=ROOT):
    xml = (source_root / 'items.xml').read_bytes()
    appearances = (root / shared.CLIENT).read_bytes()
    if hashlib.sha256(xml).hexdigest() != source_pin.XML_SHA256 or hashlib.sha256(appearances).hexdigest() != shared.CLIENT_SHA:
        raise ValueError('SRV7_TURNIP_SOURCE_PIN')
    native = owner.source
    node = native.xml_index(xml)[53073]
    objects = native.engine_items.load_appearance_objects(appearances)
    physical = next(iter(native.policies(appearances, xml, {53073})))
    bindings = native.binding_index((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    canonical = definition(root)
    record = owner.qualify(53073, node, objects[53073], physical, bindings, canonical)
    record = native.bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': [record]}, bindings, {53073})['records'][0]
    return {'schema': SCHEMA, 'producer': 'tools/content-migration/srv7_targuna_item_profiles.py',
            'scope': 'Physical custody only; no custom script or generic taming activation.',
            'sources': [{'revision': source_pin.REVISION, 'path': 'data/items/items.xml',
                         'sha256': source_pin.XML_SHA256, 'line': source_pin.xml_lines(xml)[53073]},
                        {'path': shared.CLIENT, 'sha256': shared.CLIENT_SHA}],
            'definition_sha256': digest(canonical), 'binding_sha256': digest(bindings['53073']),
            'record_sha256': digest(record), 'records': [record]}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    packet = json.loads((root / PATH).read_bytes())
    bindings = owner.source.binding_index((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    records = packet['records']
    if (packet['schema'] != SCHEMA or len(records) != 1
            or records[0]['production_definition']['production_key'] != KEY
            or packet['definition_sha256'] != digest(definition(root))
            or packet['binding_sha256'] != digest(bindings['53073'])
            or packet['record_sha256'] != digest(records[0])
            or packet['sources'][0]['revision'] != source_pin.REVISION
            or packet['sources'][0]['sha256'] != source_pin.XML_SHA256
            or packet['sources'][1]['sha256'] != shared.CLIENT_SHA
            or records[0]['admission']['legal_destinations'] != ['CharacterInventory', 'Ground']
            or not records[0]['admission']['materializable']):
        raise ValueError('SRV7_TURNIP_QUALIFICATION_DRIFT')
    shared.admit_records(root, generated, profile_path, manifest_path, order, canonical, records)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root', type=Path, required=True)
    args = parser.parse_args()
    (ROOT / PATH).write_bytes(owner.encode(capture(args.source_root)))
