#!/usr/bin/env python3
"""Reproduce the S10 source fact projection without protobuf tooling.

Usage: python3 content/npcs/s10_walk_facts.py /directory/with/pinned/assets
The directory contains {canary,crystal}-{appearances.dat,items.xml}.
No fetching or tool installation is performed. Hashes in the adjacent JSON bind
both inputs before parsing; the check compares the complete semantic projection.
"""
import hashlib
import json
from pathlib import Path
import sys
import xml.etree.ElementTree as ET


def varint(data, offset):
    value = 0
    shift = 0
    while offset < len(data) and shift < 70:
        byte = data[offset]
        offset += 1
        value |= (byte & 127) << shift
        if byte < 128:
            return value, offset
        shift += 7
    raise ValueError('invalid protobuf varint')


def fields(data):
    offset = 0
    while offset < len(data):
        tag, offset = varint(data, offset)
        number, wire = tag >> 3, tag & 7
        if wire == 0:
            value, offset = varint(data, offset)
        elif wire == 2:
            size, offset = varint(data, offset)
            value = data[offset:offset + size]
            offset += size
        elif wire in (1, 5):
            size = 8 if wire == 1 else 4
            value = data[offset:offset + size]
            offset += size
        else:
            raise ValueError('unsupported protobuf wire type')
        if not number or offset > len(data):
            raise ValueError('invalid protobuf field')
        yield number, value


def appearances(data):
    result = {}
    for number, value in fields(data):
        if number != 1:
            continue
        row = dict(fields(value))
        identity = row[1]
        # Appearances.object -> Appearance.flags -> AppearanceFlags.height.
        result[identity] = any(key == 27 for key, _ in fields(row.get(3, b'')))
    return result


def ranges(values):
    result = []
    for value in sorted(values):
        if result and result[-1][1] + 1 == value:
            result[-1][1] = value
        else:
            result.append([value, value])
    return result


def check(directory):
    recorded = json.loads(Path(__file__).with_name('s10-walk-heights.json').read_text())
    heights = {}
    damage = set()
    for label, source in zip(('canary', 'crystal'), recorded['sources']):
        appearance = (directory / (label + '-appearances.dat')).read_bytes()
        items = (directory / (label + '-items.xml')).read_bytes()
        assert hashlib.sha256(appearance).hexdigest() == source['sha256']
        assert hashlib.sha256(items).hexdigest() == source['items_xml_sha256']
        current = appearances(appearance)
        assert all(heights[key] == value for key, value in current.items() if key in heights)
        heights.update(current)
        for item in ET.fromstring(items):
            damaging = any(
                attr.get('key') == 'field' and any(
                    child.get('key') in ('damage', 'start') and int(child.get('value', 0)) != 0
                    for child in attr
                ) for attr in item
            )
            if damaging:
                low = int(item.get('id', item.get('fromid', '0')))
                high = int(item.get('toid', low))
                damage.update(range(low, high + 1))
    assert recorded['known_ranges'] == ranges(heights)
    assert recorded['height_ranges'] == ranges(key for key, value in heights.items() if value)
    assert recorded['damaging_field_ids'] == sorted(damage)
    print(f'S10 walk facts verified: {len(heights)} identities, {len(damage)} damaging fields')


if __name__ == '__main__':
    check(Path(sys.argv[1]))
