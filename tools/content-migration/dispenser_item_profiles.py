"""SRX-1: source-bound profiles for Crystal's 22 casks and kegs.

Capture uses the pinned current Crystal XML and the retained official 15.30
appearance bank. Regeneration replays the captured source rows through the
existing Item compiler; it does not infer missing canonical attributes.
"""
import argparse
import hashlib
import json
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
import build_source_item_profiles as source

PATH = 'tools/content-schema/native-gameplay/dispenser-item-profiles.json'
REVISION = '201189f871d23597b82caf88fd611454a2ddefb8'
XML_SHA = '8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509'
IDS = (25879, 25880, 25881, 25882, 25883, 25889, 25890, 25891, 25892, 25899, 25900,
       25903, 25904, 25905, 25906, 25907, 25908, 25909, 25910, 25911, 25913, 25914)
APPEARANCES = 'content/assets/files/appearances-2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.dat'


def records(rows, root=ROOT, ids=IDS):
    bindings = source.binding_index((root / source.BINDINGS.relative_to(ROOT)).read_bytes())
    if [row['id'] for row in rows] != list(ids):
        raise ValueError('DISPENSER_SOURCE_SET')
    profiles = [source.profile(row['id'], ET.fromstring(row['xml']),
                               {'flags': row['flags']}, row['physical'], bindings, None)
                for row in rows]
    return source.bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': profiles},
                                bindings, set(ids))['records']


def capture(xml_path, ids=IDS):
    raw = xml_path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != XML_SHA:
        raise ValueError('DISPENSER_CURRENT_XML_DIGEST')
    bank = (ROOT / APPEARANCES).read_bytes()
    if hashlib.sha256(bank).hexdigest() != '2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2':
        raise ValueError('DISPENSER_OFFICIAL_APPEARANCE_DIGEST')
    xml = source.xml_index(raw)
    objects = source.engine_items.load_appearance_objects(bank)
    physical = {row['server_item_id']: row for row in source.policies(bank, raw, set(ids))}
    rows = [{'id': item, 'xml': ET.tostring(xml[item], encoding='unicode'),
             'flags': objects[item]['flags'], 'physical': physical[item]} for item in ids]
    return {'schema': 'OTERYN_NATIVE_DISPENSER_PROFILES/v1', 'source_revision': REVISION,
            'source_xml_sha256': XML_SHA, 'source_rows': rows, 'records': records(rows, ids=ids)}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    if (overlay['schema'] != 'OTERYN_NATIVE_DISPENSER_PROFILES/v1'
            or overlay['source_revision'] != REVISION or overlay['source_xml_sha256'] != XML_SHA
            or overlay['records'] != records(overlay['source_rows'], root)):
        raise ValueError('DISPENSER_PROFILE_DRIFT')
    document = json.loads(generated[profile_path])
    existing = {row['production_definition']['production_key']: row for row in document['records']}
    for row in overlay['records']:
        key = row['production_definition']['production_key']
        if key in existing and existing[key] != row:
            raise ValueError('DISPENSER_EXISTING_OWNER_CONFLICT')
        if key not in existing:
            document['records'].append(row)
    document['records'].sort(key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)


def capture_json(packet):
    """Stable keys and one source/profile per line keep ordinary diffs local."""
    encode = lambda value: json.dumps(value, sort_keys=True, ensure_ascii=False,
                                      separators=(',', ':'))
    fields = []
    for key in sorted(packet):
        value = packet[key]
        rendered = ('[\n' + ',\n'.join('    ' + encode(row) for row in value)
                    + '\n  ]') if isinstance(value, list) else encode(value)
        fields.append('  ' + encode(key) + ': ' + rendered)
    return '{\n' + ',\n'.join(fields) + '\n}\n'


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--xml', type=Path, required=True)
    args = parser.parse_args()
    (ROOT / PATH).write_text(capture_json(capture(args.xml)))
