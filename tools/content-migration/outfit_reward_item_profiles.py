"""Qualify the exact portable outfit materials with existing loot custody."""
import hashlib
import json
import xml.etree.ElementTree as ET

import encounter_loot_item_admissions as custody

# Serialized item records from Crystal 201189f data/items/items.xml. These
# five records are byte-identical in the already retained summer XML capture.
SOURCE_ROWS = {
    44753: '5ae1e5bca66b18404ac4cc824842023db368b14f89adf9cdc2dc5616b11702ab',
    44754: 'ba1ecb4f3e137bcb2964c0b9ed7c6fcb9e3297f4718a5ed5b0ad2cb03ca796cc',
    48514: '3a9ac16b4a0c09f54a92e8939f527199dfbe5644582f89160f645eef128ea141',
    48515: 'bd917f553d04cae315d56c5d9e729a7511d8d96a4b08eee206d8b2475cdd76d8',
    54640: '53278c8bde9d9d822b188122af218fc34cc3d87e92e8e8374cbc3008ae35c560',
}
RETAINED = 'imports/ots-source-evidence/item-fx-audio295/retained-source-files.json'


def source_xml(root):
    source = next(
        row for row in json.loads((root / RETAINED).read_bytes())['source_files']
        if row['repository'] == 'zimbadev/crystalserver'
        and row['repository_path'] == 'data/items/items.xml'
        and row['revision'] == '00ce02a57ca5a12e48f32a3476e37471167e4c3f'
    )
    raw = source['raw_utf8'].encode()
    records = custody.source.xml_index(raw)
    for item, digest in SOURCE_ROWS.items():
        if hashlib.sha256(ET.tostring(records[item])).hexdigest() != digest:
            raise ValueError(f'outfit item source drift: {item}')
    return raw


def apply_admissions(definitions, root):
    custody.apply_admissions(
        definitions, root, set(SOURCE_ROWS), source_core=True, xml_override=source_xml(root),
    )


def admit_native(root, generated, profile_path, manifest_path, order, canonical):
    import native_goal_loot_items
    profiles = custody.facts(root, set(SOURCE_ROWS), xml_override=source_xml(root))
    native_goal_loot_items.admit_native(
        root, generated, profile_path, manifest_path, order, canonical,
        item_ids=SOURCE_ROWS, qualified_profiles=profiles,
    )
