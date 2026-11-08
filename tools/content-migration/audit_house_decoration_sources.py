"""Check furniture transform target closure without inventing Item identities.

Source inputs are read-only donor snapshots. Output contains hashes and extracted
facts, never donor assets. This does not admit a rule or a replacement target.
"""
import argparse
import hashlib
import json
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/item-authoring'))
import engine_items

PINS = {'crystal': '201189f871d23597b82caf88fd611454a2ddefb8',
        'canary': '47dfd51f45280a59a1d3e50ba7edd573d7234446'}
SOURCE_HASHES = {
    ('crystal', 'data/items/items.xml'): '8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509',
    ('crystal', 'data/items/appearances.dat'): '17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1',
    ('canary', 'data/items/items.xml'): '1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2',
    ('canary', 'data/items/appearances.dat'): 'aa44a154f30c7ed59acc25f246286396e4043851ef0b54ef3cf3951e46d1ce50',
}


def unavailable_targets(xml_raw, appearance_ids, canonical_ids):
    """Crystal items.xml:65373-65378,70951-70955 contain the observed bad edges."""
    xml_ids = set()
    rows = []
    for node in ET.fromstring(xml_raw):
        low = int(node.get('id', node.get('fromid', '0')))
        high = int(node.get('id', node.get('toid', '0')))
        ids = range(low, high + 1)
        xml_ids.update(ids)
        attrs = {a.get('key', '').lower(): a.get('value') for a in node}
        if 'wrapableto' not in attrs:
            continue
        for kind in ('rotateto', 'transformonuse'):
            if kind in attrs:
                rows.extend((item, kind, int(attrs[kind])) for item in ids)
    return [{'source': item, 'operation': kind, 'target': target,
             'target_in_xml': target in xml_ids,
             'target_in_appearances': target in appearance_ids,
             'target_in_canonical_references': target in canonical_ids}
            for item, kind, target in sorted(rows)
            if target not in appearance_ids or target not in canonical_ids]


def capture(roots):
    bindings_path = 'imports/crystalserver/bindings/items.json'
    reference_path = 'content/world/definitions/reference.json'
    raw_bindings = (ROOT / bindings_path).read_bytes()
    raw_reference = (ROOT / reference_path).read_bytes()
    number = lambda key: int(key.removeprefix('oteryn:item.tibia.i'))
    bindings = {number(row['target']['key']) for row in json.loads(raw_bindings)['bindings']
                if row['disposition'] == 'EXACT' and row['target']['key'].startswith('oteryn:item.tibia.i')}
    references = {number(row['identity']['key']) for row in json.loads(raw_reference)['records']
                  if row['identity']['key'].startswith('oteryn:item.tibia.i')}
    sources, unavailable = [], []
    for name, root in roots.items():
        xml_path, appearance_path = 'data/items/items.xml', 'data/items/appearances.dat'
        xml_raw, appearance_raw = (root / xml_path).read_bytes(), (root / appearance_path).read_bytes()
        for path, raw in ((xml_path, xml_raw), (appearance_path, appearance_raw)):
            digest = hashlib.sha256(raw).hexdigest()
            if digest != SOURCE_HASHES[(name, path)]:
                raise ValueError('HOUSE_DECORATION_DONOR_SOURCE_DRIFT:' + name + '/' + path)
            sources.append({'donor': name, 'revision': PINS[name], 'path': path,
                            'sha256': digest})
        appearances = engine_items.load_appearance_objects(appearance_raw)
        lines = xml_raw.decode().splitlines()
        for row in unavailable_targets(xml_raw, set(appearances), bindings & references):
            target_line = next(i for i, line in enumerate(lines, 1)
                               if re.search(r'key="' + row['operation'] + r'" value="' + str(row['target']) + '"', line, re.IGNORECASE))
            unavailable.append({'donor': name, 'donor_locator': f'{xml_path}:{target_line}', **row})
    return {'schema': 'OTERYN_HOUSE_DECORATION_SOURCE_CLOSURE/v1',
            'sources': sources,
            'canonical_inputs': [{'path': path, 'sha256': hashlib.sha256(raw).hexdigest()}
                                 for path, raw in ((bindings_path, raw_bindings), (reference_path, raw_reference))],
            'unavailable_transforms': unavailable}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--canary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = capture({'crystal': args.crystal, 'canary': args.canary})
    from compact_record_json import encode
    args.output.write_bytes(encode(result))
