"""Qualify the exact portable Gerimor finale gifts for the existing NPC MINT owner."""
from xml.etree import ElementTree as ET

import qengine_reward_item_profiles as profiles


# Crystal 201189f871d23597b82caf88fd611454a2ddefb8, data/items/items.xml:85996-86002.
# Source SHA-256: 8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509.
# The retained Canary XML has no record for this crown. The 15.30 appearance
# supplies the physical flags.
MONK_CROWN = 50313
MONK_WEIGHT = 1000
MYSTERY_BOX = 26186


def monk_profile(root):
    source = profiles.source
    client = (root / profiles.CLIENT).read_bytes()
    objects = source.engine_items.load_appearance_objects(client)
    bindings = source.binding_index((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    xml = ET.Element('items')
    node = ET.SubElement(xml, 'item', id=str(MONK_CROWN))
    ET.SubElement(node, 'attribute', key='weight', value=str(MONK_WEIGHT))
    [physical] = source.policies(client, ET.tostring(xml), {MONK_CROWN})
    appearance = objects[MONK_CROWN]
    if not physical['movable'] or not physical['pickupable'] or physical['container']:
        raise ValueError('GERIMOR_MONK_CROWN_APPEARANCE')
    record = source.profile(MONK_CROWN, node, appearance, physical, bindings, None)
    if record['admission']['stack_class'] != 'NonStackable':
        raise ValueError('GERIMOR_MONK_CROWN_STACK')
    # This pass qualifies custody only. Equip and use rules stay with their owners.
    record['semantics']['equipment'] = {'state': 'UNKNOWN'}
    return source.bind_profiles(
        {'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': [record]},
        bindings, {MONK_CROWN},
    )['records'][0]


def admit(root, generated, profile_path, manifest_path, order, canonical):
    records = profiles.capture(root, ids={MYSTERY_BOX})['records']
    profiles.admit_records(
        root, generated, profile_path, manifest_path, order, canonical,
        [*records, monk_profile(root)],
    )
