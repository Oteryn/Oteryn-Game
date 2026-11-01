"""Exact Cake bridge source evidence without canonical Item admission.

PR 2781 ANSWER 6549960114 keeps this known source item unavailable until the
canonical identity/map owner resolves it. Physical evidence grants no reference.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import xml.etree.ElementTree as ET
from pathlib import Path

from canary_generic_item_identity import _fields

CRYSTAL = '201189f871d23597b82caf88fd611454a2ddefb8'
CANARY = '47dfd51f45280a59a1d3e50ba7edd573d7234446'
DECISION = 'https://github.com/Oteryn/Oteryn-Game/pull/2781#issuecomment-6549960114'
SOURCES = (
    ('crystal', 'src/items/item.cpp', 'e27e622bf4fd9a94d7fffb3a818fd0c5f911e6fbf4b92844320154ded326950b'),
    ('crystal', 'src/items/items.hpp', '06d52ee83250ba852fe804c6b006b0aac150ceb26893e9079fa8c93502d209ca'),
    ('crystal', 'src/items/items.cpp', 'f39d8fc6c1c81b8ec6f749e1f152dd945472ffd5a36b9cf3cd9a3a64b2f19b1d'),
    ('crystal', 'src/protobuf/appearances.proto', '3aaf2965b14d9c3c0252876f8d3e228f20188568a03f8840b122588dbb76b78f'),
    ('crystal', 'data-global/scripts/lib/a_piece_of_cake_config.lua', '044d68d0ed8a6b73b860365f6c7e917de62e249c7e176aa575788a0de51f4da9'),
    ('crystal', 'data/items/items.xml', '8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509'),
    ('canary', 'data/items/appearances.dat', 'aa44a154f30c7ed59acc25f246286396e4043851ef0b54ef3cf3951e46d1ce50'),
    ('canary', 'data/items/items.xml', '1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2'),
)


def evidence():
    return {
        'schema': 'OTERYN_UNAVAILABLE_ITEM_SOURCE_EVIDENCE/v1',
        'item_id': 48227,
        'reason': 'EVENT_ITEM_CANONICAL_IDENTITY_UNAVAILABLE',
        'decision': DECISION,
        'source_revision': CRYSTAL,
        'sources': [{'source_key': 'oteryn:source.crystalserver' if engine == 'crystal' else 'oteryn:source.canary',
                     'source_revision': CRYSTAL if engine == 'crystal' else CANARY,
                     'path': path, 'sha256': digest} for engine, path, digest in SOURCES],
        'method': 'Pinned protobuf flags + exact Crystal Items loader + ItemType defaults + Item factory + explicit Cake bridge producer',
        'producer': 'data-global/scripts/lib/a_piece_of_cake_config.lua:249-292',
        'items_xml_present': False,
        'appearance_flags': {'bank': True, 'bank_waypoints': 150, 'unmove': True,
                             'automap': True, 'automap_color': 121, 'fullbank': True},
        'status': 'Unavailable until the canonical Item identity/map owner supplies an admitted definition; no source suffix',
    }


def _contains_item(raw, item_id):
    return any(int(row.get('id', row.get('fromid', '0'))) <= item_id
               <= int(row.get('id', row.get('toid', '0'))) for row in ET.fromstring(raw))


def source_evidence(canary, crystal):
    roots = {'canary': canary, 'crystal': crystal}
    data = {}
    for engine, path, expected in SOURCES:
        raw = (roots[engine] / path).read_bytes()
        if hashlib.sha256(raw).hexdigest() != expected:
            raise ValueError('EVENT_ITEM_IDENTITY_SOURCE_DRIFT:' + engine + '/' + path)
        data[(engine, path)] = raw
    if any(_contains_item(data[(engine, 'data/items/items.xml')], 48227) for engine in roots):
        raise ValueError('EVENT_ITEM_IDENTITY_XML_DRIFT')
    appearances = _fields(data[('canary', 'data/items/appearances.dat')])
    matches = [_fields(raw) for raw in appearances.get(1, []) if _fields(raw).get(1) == [48227]]
    if len(matches) != 1 or len(matches[0].get(3, [])) != 1:
        raise ValueError('EVENT_ITEM_IDENTITY_APPEARANCE_AMBIGUOUS')
    flags = _fields(matches[0][3][0])
    if flags != {1: [bytes.fromhex('089601')], 14: [1], 30: [bytes.fromhex('0879')], 32: [1]}:
        raise ValueError('EVENT_ITEM_IDENTITY_APPEARANCE_DRIFT')
    assertions = {
        'src/items/items.hpp': ['group = ITEM_GROUP_NONE;', 'type = ITEM_TYPE_NONE;',
                               'uint32_t decayTime = 0;', 'uint32_t charges = 0;', 'int32_t weight = 0;'],
        'src/items/items.cpp': ['ItemType &iType = items[object.id()];', 'iType.group = ITEM_GROUP_GROUND;',
                               'iType.movable = object.flags().unmove() == false;',
                               'iType.pickupable = object.flags().take();', 'iType.stackable = object.flags().cumulative();'],
        'src/items/item.cpp': ['if (it.id != 0)', 'newItem = std::make_shared<Item>(type, count);'],
        'data-global/scripts/lib/a_piece_of_cake_config.lua': ['FloorId = 48227,',
                               'Game.createItem(itemToPlace, 1, pos)', 'bridge.FloorId'],
    }
    for path, snippets in assertions.items():
        text = data[('crystal', path)].decode()
        if any(snippet not in text for snippet in snippets):
            raise ValueError('EVENT_ITEM_IDENTITY_LOADER_DRIFT:' + path)
    return evidence()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--canary', type=Path, required=True)
    parser.add_argument('--crystal', type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(source_evidence(args.canary, args.crystal), indent=2))
