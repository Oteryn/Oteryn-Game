"""Qualify the hatchet's missing equip gates from the retained pinned XML.

Canary04b83b51 data/items/items.xml:8403-8413 declares hand equipment with
no level or vocation restriction. Use the shared importer defaults, as for the
starter sword; preserve every already-qualified native fact. Crystal201189f8
data/items/items.xml:8424-8434 independently declares the same hand script.
"""
import hashlib
import json
import xml.etree.ElementTree as ET

from native_profile_composition import encode_native_profile, merge_fact

XML = 'imports/canary/items-xml/items.xml'
XML_SHA256 = '1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2'
KEY = 'oteryn:item.tibia.i3276'


def equipment(root):
    raw = (root / XML).read_bytes()
    if hashlib.sha256(raw).hexdigest() != XML_SHA256:
        raise ValueError('HATCHET_EQUIPMENT_SOURCE_PIN')
    item, = [node for node in ET.fromstring(raw) if node.get('id') == '3276']
    script, = item.findall("attribute[@key='script']")
    attributes = {node.get('key'): node.get('value') for node in script}
    if (item.get('name') != 'hatchet' or script.get('value') != 'moveevent;weapon'
            or attributes != {'weaponType': 'axe', 'slot': 'hand'}):
        raise ValueError('HATCHET_EQUIPMENT_SOURCE_RULE')
    known = lambda value: {'state': 'KNOWN', 'value': value}
    return known({'patterns': known([{
        'pattern_id': 1, 'primary_slot': known('WEAPON'),
        'additional_reserved_slots': known([]), 'mutually_exclusive_groups': known([]),
        'vocations': known([]), 'level': known(0),
        'compatibility_rule': {'state': 'NOT_APPLICABLE'},
    }])})


def admit(root, generated, profile_path, manifest_path, canonical):
    document = json.loads(generated[profile_path])
    record, = [row for row in document['records']
               if row['production_definition']['production_key'] == KEY]
    record['semantics']['equipment'] = merge_fact(
        record['semantics']['equipment'], equipment(root), KEY + '/equipment')
    raw = encode_native_profile(document)
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)
