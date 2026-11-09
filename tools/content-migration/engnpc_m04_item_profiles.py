"""Source-qualified physical Items needed by the M04 terminal NPC commands.

The canonical i901 definition and its D289 hold remain byte-for-byte intact.
Only its already accepted NonStackable native core wins over the donor's stack
flag; no source stackability, use effect or definition enrichment is promoted.
"""
import argparse
import copy
import hashlib
import json
import xml.etree.ElementTree as ET
from pathlib import Path

import grizzly_shop_profiles as shared
import qengine_reward_item_profiles as merger
import quest_reward_item_semantics as rewards

ROOT = shared.ROOT
PATH = 'tools/content-schema/native-gameplay/engnpc-m04-item-profiles.json'
SCHEMA = 'OTERYN_ENGNPC_M04_ITEM_PROFILES/v1'
REVISION = shared.REVISION
XML_SHA = shared.SOURCE_HASHES['items.xml']
XML_ITEMS = {
    901: '37cfc2ad1e9b536cd92969e64fbd633f386b04db1dcb1f12b3c2775abf95da83',
    902: '7005d77bd6735fcc4fee38eaabb75abfe39d8f3b6a331b551abec69af528b5f9',
    3212: 'e5a3e44d3ca228dff49461376688b0e89d552a26fa85387cd653b9e7e74d4962',
    3214: '5fe7c232fc3c1366cfab8fb2ece11dfe18490ded60f0fa4f6455041e775f9c08',
}
MARLIN_CORE_SHA = '3e1a277020e06a7467dd81e1fb058429f45807234cd7a055fb5b04cb87cc7389'
DIARY = 'oteryn:item.tibia.i3212'
DIARY_TEXT = shared.source.K({
    'readable': shared.source.K(True), 'max_text_length': shared.source.K(1023),
    'writeable': shared.source.U, 'distance_read': shared.source.U,
    'write_once_target': shared.source.U,
})


def definitions(root):
    keys = {f'oteryn:item.tibia.i{i}' for i in XML_ITEMS}
    rows = [row['definition']
            for path in sorted((root / 'content/items/definitions').glob('items-*.json'))
            for row in json.loads(path.read_bytes())['records']
            if row['definition']['identity']['key'] in keys]
    result = {row['identity']['key']: row for row in rows}
    if result.keys() != keys or len(rows) != len(keys):
        raise ValueError('M04_CANONICAL_ITEM_SET')
    return result


def qualify(item_id, node, appearance, physical, bindings, definition):
    accepted = definition
    if item_id == 3212:
        # The existing text owner retains the source writeable/maxtextlen
        # interpretation. Qualify physical custody without adding WRITE power.
        if shared.source.attrs(node) != {'writeable': '1', 'maxtextlen': '1023', 'weight': '1300'}:
            raise ValueError('M04_DIARY_SOURCE_DRIFT')
        node = copy.deepcopy(node)
        for attribute in list(node):
            if attribute.get('key') in {'writeable', 'maxtextlen'}:
                node.remove(attribute)
    if item_id == 901:
        if (hashlib.sha256(rewards.canonical(definition)).hexdigest() != MARLIN_CORE_SHA
                or definition['stack_class'] != 'NonStackable'
                or definition['semantics']['stack'] != shared.source.U
                or not appearance['flags'].get('flags.cumulative')):
            raise ValueError('M04_ACCEPTED_MARLIN_CORE_DRIFT')
        # Compare all donor facts normally. This copy changes only the class
        # used for comparison; neither the canonical definition nor D289 moves.
        accepted = copy.deepcopy(definition)
        accepted['stack_class'] = 'StackCapable'
    record = shared.qualify(item_id, node, appearance, physical, bindings, accepted)
    if item_id == 3212:
        # Physical admission does not qualify the old text carrier's abilities.
        record['attributes']['speed_bonus'] = None
        record['semantics']['skill_modifiers'] = copy.deepcopy(definition['semantics']['skill_modifiers'])
    if item_id == 901:
        record['admission']['stack_class'] = definition['stack_class']
        record['semantics']['stack'] = copy.deepcopy(definition['semantics']['stack'])
    return record


def rebuild(root, witnesses):
    if set(witnesses) != {str(i) for i in XML_ITEMS}:
        raise ValueError('M04_SOURCE_ITEM_SET')
    nodes = {}
    for item_id, digest in XML_ITEMS.items():
        raw = witnesses[str(item_id)].encode()
        if hashlib.sha256(raw).hexdigest() != digest:
            raise ValueError('M04_CRYSTAL_XML_WITNESS_DRIFT')
        node = ET.fromstring(raw)
        if node.tag != 'item' or node.get('id') != str(item_id):
            raise ValueError('M04_CRYSTAL_ITEM_ID')
        nodes[item_id] = node
    client = (root / rewards.CLIENT).read_bytes()
    if (len(client) != rewards.CLIENT_BYTES
            or hashlib.sha256(client).hexdigest() != rewards.CLIENT_SHA):
        raise ValueError('M04_CLIENT_SOURCE_PIN')
    source = shared.source
    appearances = source.engine_items.load_appearance_objects(client)
    xml = b'<items>' + b''.join(ET.tostring(node) for node in nodes.values()) + b'</items>'
    physical = {row['server_item_id']: row
                for row in source.policies(client, xml, set(XML_ITEMS))}
    bindings = source.binding_index((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    canonical = definitions(root)
    records = [qualify(i, nodes[i], appearances[i], physical[i], bindings,
                       canonical[f'oteryn:item.tibia.i{i}']) for i in sorted(XML_ITEMS)]
    return {
        'schema': SCHEMA, 'producer': 'tools/content-migration/engnpc_m04_item_profiles.py',
        'runtime_enabled': True, 'source_revision': REVISION,
        'purpose': 'M04 carried inputs and terminal outputs; accepted i901 native core retained.',
        'sources': [
            {'repository': 'zimbadev/crystalserver', 'revision': REVISION,
             'path': 'data/items/items.xml', 'sha256': XML_SHA},
            {'path': rewards.CLIENT, 'sha256': rewards.CLIENT_SHA},
        ],
        'xml_witnesses': witnesses,
        'canonical_sha256': {key: hashlib.sha256(rewards.canonical(value)).hexdigest()
                             for key, value in sorted(canonical.items())},
        'records': records,
    }


def capture(xml_path, root=ROOT):
    raw = xml_path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != XML_SHA:
        raise ValueError('M04_CRYSTAL_SOURCE_PIN')
    nodes = shared.source.xml_index(raw)
    return rebuild(root, {str(i): ET.tostring(nodes[i], encoding='unicode')
                          for i in XML_ITEMS})


def admit(root, generated, profile_path, manifest_path, order, canonical):
    packet = json.loads((root / PATH).read_bytes())
    if packet != rebuild(root, packet['xml_witnesses']):
        raise ValueError('M04_ITEM_PROFILE_DRIFT')
    document = json.loads(generated[profile_path])
    carriers = [row for row in document['records']
                if row['production_definition']['production_key'] == DIARY]
    if (len(carriers) != 1 or carriers[0]['admission'] != {
            'materializable': False, 'stack_class': 'NonStackable',
            'legal_destinations': ['Ground']}
            or carriers[0]['semantics']['readable_writeable'] != DIARY_TEXT):
        raise ValueError('M04_DIARY_CARRIER_DRIFT')
    # This exact fixed-text carrier gains physical custody. The strict merger
    # still checks identity, bindings and all known text/physical/attribute facts.
    diary = next(row for row in packet['records']
                 if row['production_definition']['production_key'] == DIARY)
    carriers[0]['admission'] = copy.deepcopy(diary['admission'])
    working = dict(generated)
    working[profile_path] = shared.encode(document)
    merger.admit_records(root, working, profile_path, manifest_path, order,
                         canonical, packet['records'])
    generated.update(working)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-xml', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    raw = shared.encode(capture(args.source_xml))
    target = ROOT / PATH
    if args.check:
        if target.read_bytes() != raw:
            raise SystemExit('M04_ITEM_PROFILE_DRIFT')
    else:
        target.write_bytes(raw)
