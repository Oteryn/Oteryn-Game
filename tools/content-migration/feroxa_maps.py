"""M52 closed full-moon scenery projection from hash-verified source maps.

Capture is an explicit offline authoring operation. Normal generation consumes only
normalized facts; original third-party map bytes are never published.
"""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import sys

from feroxa_raid import REVISION, LIB, EVENT, sources

CAPTURE = 'imports/crystalserver/grimvale-full-moon-maps.json'
CAPTURE_SHA256 = 'f7196fe4796c6845fd3abdb06ec8b485d36d8953e2a82de6eb90c2be294097a5'
RECEIPT = 'tools/content-schema/quest-authoring/donor_sources/maps/structural-receipt.json'
SCHEMA = 'OTERYN_FEROXA_SOURCE_MAPS/v1'
NAMES = ('middle', 'final')


def encode(value):
    return (json.dumps(value, sort_keys=True, separators=(',', ':')) + '\n').encode()


def normalize(root, name, raw):
    if name not in NAMES or len(raw) > 128 * 1024:
        raise ValueError('FEROXA_MAP_SOURCE_BOUND')
    path = f'data-global/world/world_changes/full_moon/{name}.otbm'
    records = json.loads((root / RECEIPT).read_bytes())['records']
    matches = [(record, ref) for record in records for ref in record['source_refs']
               if ref['source'] == 'crystalserver' and ref['revision'] == REVISION
               and ref['source_path'] == path]
    if len(matches) != 1:
        raise ValueError('FEROXA_MAP_SOURCE_RECEIPT')
    record, ref = matches[0]
    sha = hashlib.sha256(raw).hexdigest()
    blob = hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest()
    if sha != record['artifact_sha256'] or blob != ref['git_blob_sha1'] or len(raw) != record['decoded_bytes']:
        raise ValueError('FEROXA_MAP_SOURCE_DIGEST')
    spec = importlib.util.spec_from_file_location('feroxa_otbm_reader', root / 'tools/content-schema/world-authoring/otbm_reader.py')
    reader = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = reader
    spec.loader.exec_module(reader)
    tiles, teleports = {}, []

    def tile(x, y, z, flags, house, zones, items):
        if not (33413 <= x <= 33447 and 31520 <= y <= 31554 and z == 11) or house or zones or flags not in (None, 0, 8):
            raise ValueError('FEROXA_MAP_TILE')
        if (x, y, z) in tiles or not 1 <= len(items) <= 64:
            raise ValueError('FEROXA_MAP_STACK')
        stack = []
        for ordinal, (number, depth, attrs) in enumerate(items):
            if depth != 0 or attrs.keys() - {'dest'}:
                raise ValueError('FEROXA_MAP_ITEM_ATTRIBUTES')
            if attrs:
                destination = tuple(attrs['dest'])
                if (x, y, z, ordinal, number, destination) != (33430, 31537, 11, 1, 1949, (33457, 31531, 11)):
                    raise ValueError('FEROXA_MAP_TELEPORT')
                teleports.append([x, y, z, ordinal, *destination])
            stack.append(number)
        tiles[x, y, z] = [x, y, z, flags or 0, stack]

    facts = reader.read_tiles(raw, tile)
    if len(tiles) != 1225 or facts.unknown_item_attrs or facts.unknown_tile_attrs or len(teleports) != 1:
        raise ValueError('FEROXA_MAP_COVERAGE')
    return dict(name=name, source={**ref, 'sha256': sha, 'byte_count': len(raw),
                                  'evidence': 'OTS_HYPOTHESIS_ONLY'},
                tiles=[tiles[key] for key in sorted(tiles, key=lambda p: (p[2], p[1], p[0]))],
                teleports=teleports)


def verify_capture(raw):
    if len(raw) > 256 * 1024 or hashlib.sha256(raw).hexdigest() != CAPTURE_SHA256:
        raise ValueError('FEROXA_MAP_CAPTURE_DIGEST')
    capture = json.loads(raw)
    if capture['schema'] != SCHEMA or [p['name'] for p in capture['phases']] != list(NAMES):
        raise ValueError('FEROXA_MAP_CAPTURE_SCHEMA')
    return capture


def load(root):
    return verify_capture((root / CAPTURE).read_bytes())


def capture(root, directory):
    captured = sources(root)

    def minutes(path, function):
        match = re.search(r"addEvent\(" + re.escape(function) + r",\s*(\d+)\s*\*\s*60\s*\*\s*1000", captured[path]['text'])
        if not match:
            raise ValueError('FEROXA_MAP_TIMER')
        return int(match[1]) * 60_000

    middle = minutes(EVENT, 'Game.loadMap')
    final = middle + minutes(LIB, 'grimvaleSpectators') + minutes(LIB, 'loadMap')
    phases = [dict(normalize(root, name, (directory / f'{name}.otbm').read_bytes()),
                   phase=index + 1, after_ms=delay)
              for index, (name, delay) in enumerate(zip(NAMES, (middle, final)))]
    # The existing item loader owns XML/appearance interpretation. Fail closed for
    # custody-bearing or unimplemented floor-change entries before publishing facts.
    sys.path.insert(0, str(root / 'tools/content-schema/native-gameplay'))
    import build_world_change_item_profiles as source_items
    import build_source_world
    xml = source_items.read_xml(root)
    appearance = (root / source_items.APPEARANCES).read_bytes()
    source_items.verify_sources(xml, appearance)
    ids = {number for phase in phases for tile in phase['tiles'] for number in tile[4]}
    policies = build_source_world.policies(appearance, xml, ids)
    for policy in policies:
        if policy['pickupable'] or policy['container'] or policy['floor_change'] or policy['dynamic_kind'] not in (None, 'teleport'):
            raise ValueError('FEROXA_MAP_UNSUPPORTED_PHYSICAL_ITEM')
        del policy['xml_attributes']
    return dict(schema=SCHEMA, phases=phases, palette=policies,
                physical_sources=dict(xml_sha256=source_items.XML_SHA256,
                                      appearances_sha256=source_items.APPEARANCES_SHA256),
                timer_sources=[dict(path=path, revision=REVISION, sha256=captured[path]['sha256']) for path in (EVENT, LIB)])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--capture-from', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    raw = encode(capture(root, args.capture_from))
    (root / CAPTURE).write_bytes(raw)
    print(f'FEROXA_MAP_CAPTURE_SHA256 {hashlib.sha256(raw).hexdigest()}')


if __name__ == '__main__':
    main()
