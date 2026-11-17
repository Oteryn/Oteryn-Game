#!/usr/bin/env python3
"""C19b: derive the five missing Newhaven kit profiles through the pinned Item importer.

Kit membership: CrystalServer9f5a72c6 data-global/lib/others/newhaven.lua:3-95.
Physical flags: retained official15.30 appearance bank; XML: retained Canary04b83b51.
No source fetches or original asset redistribution. Existing native rules qualify output.
"""
import argparse
import hashlib
import json
from pathlib import Path

import build_source_item_profiles as native

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = Path(__file__).with_name('newhaven-starter-item-profiles.json')
IDS = {34017, 21348, 21400, 21350, 50166}
XML = ROOT / 'imports/canary/items-xml/items.xml'
APPEARANCES = ROOT / ('content/assets/files/appearances-'
    '2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.dat')


def build():
    xml = XML.read_bytes()
    appearances = APPEARANCES.read_bytes()
    manifest = json.loads((XML.parent / 'manifest.json').read_bytes())
    expected_xml = next(row['sha256'] for row in manifest['files'] if row['path'] == 'items.xml')
    if hashlib.sha256(xml).hexdigest() != expected_xml:
        raise ValueError('NEWHAVEN_STARTER_XML_PIN')
    if hashlib.sha256(appearances).hexdigest() != '2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2':
        raise ValueError('NEWHAVEN_STARTER_APPEARANCE_PIN')
    bindings = native.binding_index(native.BINDINGS.read_bytes())
    index = native.xml_index(xml)
    objects = native.engine_items.load_appearance_objects(appearances)
    physical = {row['server_item_id']: row for row in native.policies(appearances, xml, IDS)}
    records = [native.profile(item, index[item], objects[item], physical[item], bindings, None)
               for item in sorted(IDS)]
    # The source kit explicitly equips the lit torch in CONST_SLOT_AMMO
    # (Newhaven.starterItems, newhaven.lua:10,29,48,66,84). Its XML omits an
    # equip script; retain that audited primary slot instead of guessing one.
    torch = next(row for row in records if row['authoring']['item']['key'] == 'candidate:item/34017')
    torch['semantics']['equipment'] = native.K({'patterns': native.K([{
        'pattern_id': 1, 'primary_slot': native.K('EXTRA'),
        'additional_reserved_slots': native.K([]), 'mutually_exclusive_groups': native.K([]),
        'vocations': native.K([]), 'level': native.K(0),
        'compatibility_rule': dict(native.NA),
    }])})
    bound = native.bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records},
                                 bindings, set(index))
    return {
        'schema': 'OTERYN_NATIVE_NEWHAVEN_STARTER_ITEM_PROFILES/v1',
        'producer': 'tools/content-schema/native-gameplay/build_newhaven_starter_profiles.py',
        'kit_source': {'repository': 'zimbadev/crystalserver',
                       'revision': '9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d',
                       'path': 'data-global/lib/others/newhaven.lua', 'lines': '3-95'},
        'xml_source': manifest['source'],
        'sources': [{'path': str(path.relative_to(ROOT)), 'sha256': hashlib.sha256(raw).hexdigest()}
                    for path, raw in [(XML, xml), (APPEARANCES, appearances)]],
        'binding_table_sha256': hashlib.sha256(native.BINDINGS.read_bytes()).hexdigest(),
        'records': bound['records'],
    }


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    data = (json.dumps(build(), indent=2, ensure_ascii=False) + '\n').encode()
    if args.check:
        if OUTPUT.read_bytes() != data:
            raise SystemExit('NEWHAVEN_STARTER_PROFILE_DRIFT')
    else:
        OUTPUT.write_bytes(data)
